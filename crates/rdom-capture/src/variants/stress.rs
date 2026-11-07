//! `stress` (`specs/CAPTURE.md` §Variants): an element's text rewritten —
//! doubled, four times as long, empty, one long word, CJK, emoji,
//! right-to-left, a large number — each rewrite in a fork of its own, and
//! what it did to the layout: text cut off, a box that wrapped, one that
//! overflows its parent, the elements it shifted, the ones it now
//! overlaps. Rewrites change the fork's text nodes (`Session::rewrite`),
//! never the app's data; a mutation observer in the fork sees them.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::app::CaptureApp;
use crate::capture::identity::identities;
use crate::capture::{Layer, node_key, nodes};
use crate::session::{CaptureRequest, Session, SessionError, Step};

/// The most elements one `stress` rewrites (each rewrite of each one is
/// a fork).
pub const MAX_TARGETS: usize = 8;

/// How a text is rewritten.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Rewrite {
    /// Twice the text.
    Double,
    /// Four times the text.
    Quadruple,
    /// No text.
    Empty,
    /// One word too long to break.
    LongWord,
    /// CJK, two cells a character.
    Cjk,
    /// Emoji.
    Emoji,
    /// Right-to-left (Hebrew).
    Rtl,
    /// A large number.
    Number,
}

impl Rewrite {
    /// Every rewrite, in order.
    pub const ALL: [Rewrite; 8] = [
        Rewrite::Double,
        Rewrite::Quadruple,
        Rewrite::Empty,
        Rewrite::LongWord,
        Rewrite::Cjk,
        Rewrite::Emoji,
        Rewrite::Rtl,
        Rewrite::Number,
    ];

    /// Whether it is a function of the text (doubled, emptied) rather than
    /// a sample that replaces it.
    pub fn relative(self) -> bool {
        matches!(self, Rewrite::Double | Rewrite::Quadruple | Rewrite::Empty)
    }

    /// `text` rewritten (a whitespace-only text is left alone: it is
    /// layout, not content).
    pub fn apply(self, text: &str) -> String {
        if text.trim().is_empty() {
            return text.to_string();
        }
        match self {
            Rewrite::Double => format!("{text} {text}"),
            Rewrite::Quadruple => [text; 4].join(" "),
            Rewrite::Empty => String::new(),
            Rewrite::LongWord => "Supercalifragilisticexpialidocious-internationalization".into(),
            Rewrite::Cjk => "国際化対応のとても長いテキスト".into(),
            Rewrite::Emoji => "🎉 Done ✅ 🚀 ✨".into(),
            Rewrite::Rtl => "שלום עולם, זהו טקסט ארוך".into(),
            Rewrite::Number => "1,234,567,890.12".into(),
        }
    }
}

/// What one rewrite of one element did.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct Stressed {
    /// The element, by identity.
    pub key: String,
    pub rewrite: Rewrite,
    /// Its border box before and after (`None`: not rendered).
    pub before: Option<[i32; 4]>,
    pub after: Option<[i32; 4]>,
    /// What broke; empty when the layout took it.
    pub findings: Vec<Finding>,
    /// Why it could not be tried.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// What a rewrite did to the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Kind {
    /// Fewer of the text's cells show than before.
    Truncated,
    /// The box got taller.
    Wrapped,
    /// The box reaches further past its parent's than before.
    Overflow,
    /// Other elements moved.
    Shifted,
    /// It now overlaps elements it did not.
    Overlap,
}

/// One thing a rewrite did to the layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Finding {
    pub kind: Kind,
    pub message: String,
    /// The elements involved, by identity.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<String>,
}

/// What a fork shows of the stressed element and the rest, settled.
struct Laid {
    /// Border boxes by identity.
    boxes: HashMap<String, [i32; 4]>,
    /// Ancestors and descendants of the target, by identity (they move
    /// with it; that is no shift).
    family: HashSet<String>,
    parent: Option<String>,
    /// Text cells the target's subtree paints, and the cells its text needs.
    shown: usize,
    needed: usize,
}

fn intersects(a: [i32; 4], b: [i32; 4]) -> bool {
    a[2] > 0
        && a[3] > 0
        && b[2] > 0
        && b[3] > 0
        && a[0] < b[0] + b[2]
        && b[0] < a[0] + a[2]
        && a[1] < b[1] + b[3]
        && b[1] < a[1] + a[3]
}

impl<A: CaptureApp> Session<A> {
    /// What each of `rewrites` does to each element `selector` matches
    /// (at most [`MAX_TARGETS`]), each in its own fork, settled
    /// `settle_ms`.
    pub fn stress(
        &self,
        selector: &str,
        rewrites: &[Rewrite],
        settle_ms: u64,
    ) -> Result<Vec<Stressed>, SessionError> {
        let invalid = SessionError::InvalidInput;
        let dom = self.app()?.dom();
        let targets: Vec<String> = dom
            .query_selector_all_in(dom.root(), selector)
            .map_err(|e| invalid(format!("invalid selector {selector:?}: {e:?}")))?
            .into_iter()
            .map(node_key)
            .collect();
        if targets.is_empty() {
            return Err(invalid(format!("no element matches {selector:?}")));
        }
        if targets.len() > MAX_TARGETS {
            return Err(invalid(format!(
                "{} elements match {selector:?}: at most {MAX_TARGETS} are stressed at once, narrow the selector",
                targets.len()
            )));
        }
        let mut rewrites = rewrites.to_vec();
        rewrites.dedup();
        let mut base = self.fork()?;
        base.advance(settle_ms)?;
        let mut out = Vec::new();
        for target in &targets {
            let before = laid(&base, target)?;
            for &rewrite in &rewrites {
                let key = before.0.clone();
                let tried = (|| -> Result<_, SessionError> {
                    let mut fork = self.fork()?;
                    fork.rewrite(target, rewrite)?;
                    fork.advance(settle_ms)?;
                    laid(&fork, target)
                })();
                out.push(match tried {
                    Ok((_, after)) => findings_of(key, rewrite, &before.1, &after),
                    Err(e) => Stressed {
                        key,
                        rewrite,
                        before: before.1.boxes.get(&before.0).copied(),
                        after: None,
                        findings: Vec::new(),
                        error: Some(e.to_string()),
                    },
                });
            }
        }
        Ok(out)
    }

    /// Rewrite the text nodes of the element named `node` (a fork's step,
    /// logged): app code's mutation observers see it, as they would a
    /// translation or a user's data.
    pub fn rewrite(&mut self, node: &str, rewrite: Rewrite) -> Result<(), SessionError> {
        let id = crate::session::node_by_key(self.app()?, node)?;
        self.step_inspected(|app| {
            let dom = app.dom_mut();
            // A relative rewrite (doubled, emptied) applies to every text;
            // a sample replaces the first and empties the rest, so the
            // element shows it once.
            let mut first = true;
            for t in rendered_texts(dom, id) {
                let old = dom.node(t).node_value().unwrap_or_default().to_string();
                if old.trim().is_empty() {
                    continue;
                }
                let new = if rewrite.relative() || first {
                    rewrite.apply(&old)
                } else {
                    String::new()
                };
                first = false;
                dom.node_mut(t)
                    .set_node_value(&new)
                    .map_err(|e| SessionError::InvalidInput(format!("{e:?}")))?;
            }
            Ok(())
        })?;
        let identity = crate::session::identity_of(self.app()?, id);
        self.record(Step::Rewrite {
            node: identity,
            rewrite,
        });
        Ok(())
    }
}

/// The layout around `target` in `s`: its identity and what it shows.
fn laid<A: CaptureApp>(s: &Session<A>, target: &str) -> Result<(String, Laid), SessionError> {
    let app = s.app()?;
    let dom = app.dom();
    let id = crate::session::node_by_key(app, target)?;
    let ids = identities(dom);
    let key_of = |n| ids.get(&n).map(|i| i.key.clone());
    let boxes = nodes::boxes(dom)
        .into_iter()
        .filter_map(|(n, b)| Some((key_of(n)?, b.border)))
        .collect();
    let mut family: HashSet<String> = dom.descendants(id).filter_map(key_of).collect();
    let mut cur = dom.node(id).parent_node().map(|p| p.id());
    let mut parent = None;
    while let Some(p) = cur {
        if let Some(k) = key_of(p) {
            parent.get_or_insert_with(|| k.clone());
            family.insert(k);
        }
        cur = dom.node(p).parent_node().map(|n| n.id());
    }
    let capture = s.capture(&CaptureRequest::with_layers([Layer::Grid, Layer::Owners]))?;
    let (grid, owners) = (
        capture.grid.expect("asked for"),
        capture.owners.expect("asked for"),
    );
    let subtree: HashSet<String> = std::iter::once(id)
        .chain(dom.descendants(id))
        .map(node_key)
        .collect();
    // Non-blank glyphs only, by width: a space where a line wraps is never
    // painted.
    let shown = grid
        .rows
        .iter()
        .zip(&owners.rows)
        .map(|(runs, owner_runs)| {
            let cells: Vec<&str> = runs.iter().flat_map(|r| r.cells()).collect();
            owner_runs
                .iter()
                .filter(|r| {
                    r.part == "text" && r.node.as_ref().is_some_and(|n| subtree.contains(n))
                })
                .flat_map(|r| usize::from(r.x)..usize::from(r.x + r.w))
                .filter_map(|x| cells.get(x).filter(|g| !g.trim().is_empty()))
                // A wide glyph's width: its trailing cell holds `""`.
                .map(|g| unicode_width::UnicodeWidthStr::width(*g))
                .sum::<usize>()
        })
        .sum();
    let needed = rendered_texts(dom, id)
        .into_iter()
        .flat_map(|t| {
            dom.node(t)
                .node_value()
                .unwrap_or_default()
                .chars()
                .collect::<Vec<_>>()
        })
        .filter(|c| !c.is_whitespace())
        .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0))
        .sum();
    Ok((
        key_of(id).unwrap_or_else(|| target.to_string()),
        Laid {
            boxes,
            family,
            parent,
            shown,
            needed,
        },
    ))
}

/// The text nodes of `id`'s subtree that render: not in a `<style>`,
/// `<script>` or `<template>`, not under an element with no box.
fn rendered_texts(
    dom: &rdom_tui::TuiDom,
    id: rdom_tui::core_api::NodeId,
) -> Vec<rdom_tui::core_api::NodeId> {
    dom.descendants(id)
        .filter(|&n| dom.node(n).node_type() == rdom_tui::core_api::NodeType::Text)
        .filter(|&n| {
            let mut cur = dom.node(n).parent_node().map(|p| p.id());
            while let Some(p) = cur {
                let tag = dom
                    .node(p)
                    .tag_name()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if matches!(tag.as_str(), "style" | "script" | "template")
                    || rdom_tui::capture::box_model(dom, p).is_none()
                {
                    return false;
                }
                if p == id {
                    return true;
                }
                cur = dom.node(p).parent_node().map(|x| x.id());
            }
            true
        })
        .collect()
}

fn findings_of(key: String, rewrite: Rewrite, a: &Laid, b: &Laid) -> Stressed {
    let (before, after) = (a.boxes.get(&key).copied(), b.boxes.get(&key).copied());
    let mut findings = Vec::new();
    // Judged against the layout before the rewrite: what was cut already
    // is not the rewrite's doing.
    let cut = |l: &Laid| l.needed.saturating_sub(l.shown);
    if cut(b) > cut(a) {
        findings.push(Finding {
            kind: Kind::Truncated,
            message: format!("{} of {} text cells show", b.shown, b.needed),
            nodes: vec![key.clone()],
        });
    }
    if let (Some(x), Some(y)) = (before, after)
        && y[3] > x[3]
    {
        findings.push(Finding {
            kind: Kind::Wrapped,
            message: format!("{} rows tall, was {}", y[3], x[3]),
            nodes: vec![key.clone()],
        });
    }
    // How far past its parent's border box, on any side.
    let over = |l: &Laid, me: Option<[i32; 4]>| -> i32 {
        let (Some(me), Some(p)) = (me, l.parent.as_ref().and_then(|p| l.boxes.get(p))) else {
            return 0;
        };
        [
            (me[0] + me[2]) - (p[0] + p[2]),
            p[0] - me[0],
            (me[1] + me[3]) - (p[1] + p[3]),
            p[1] - me[1],
        ]
        .into_iter()
        .max()
        .unwrap_or(0)
        .max(0)
    };
    let (was, now) = (over(a, before), over(b, after));
    if now > was {
        findings.push(Finding {
            kind: Kind::Overflow,
            message: format!("{now} cells past its parent's box"),
            nodes: vec![key.clone(), b.parent.clone().unwrap_or_default()],
        });
    }
    let others = |l: &Laid| -> Vec<String> {
        let mut v: Vec<String> = l
            .boxes
            .keys()
            .filter(|k| !l.family.contains(*k) && **k != key)
            .cloned()
            .collect();
        v.sort();
        v
    };
    let shifted: Vec<String> = others(b)
        .into_iter()
        .filter(|k| {
            a.boxes
                .get(k)
                .is_some_and(|x| b.boxes.get(k).is_some_and(|y| x[..2] != y[..2]))
        })
        .collect();
    // Only the topmost of what moved: a moved box's descendants move with it.
    let shifted: Vec<String> = shifted
        .iter()
        .filter(|k| !shifted.iter().any(|s| k.starts_with(&format!("{s}/"))))
        .cloned()
        .collect();
    if !shifted.is_empty() {
        findings.push(Finding {
            kind: Kind::Shifted,
            message: format!("{} other elements moved", shifted.len()),
            nodes: shifted,
        });
    }
    if let Some(me) = after {
        let was = before.unwrap_or([0; 4]);
        let overlaps: Vec<String> = others(b)
            .into_iter()
            .filter(|k| {
                let now = b.boxes[k];
                intersects(me, now) && !a.boxes.get(k).is_some_and(|old| intersects(was, *old))
            })
            .collect();
        if !overlaps.is_empty() {
            findings.push(Finding {
                kind: Kind::Overlap,
                message: format!("now overlaps {} elements it did not", overlaps.len()),
                nodes: overlaps,
            });
        }
    }
    Stressed {
        key,
        rewrite,
        before,
        after,
        findings,
        error: None,
    }
}
