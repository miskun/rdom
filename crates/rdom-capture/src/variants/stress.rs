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

/// One thing a rewrite did to the layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Finding {
    /// `truncated`, `wrapped`, `overflow`, `shifted`, `overlap`.
    pub kind: &'static str,
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
    /// (at most 8 elements), each in its own fork, settled `settle_ms`.
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
            .take(8)
            .collect();
        if targets.is_empty() {
            return Err(invalid(format!("no element matches {selector:?}")));
        }
        let mut base = self.fork()?;
        base.advance(settle_ms)?;
        let mut out = Vec::new();
        for target in &targets {
            let before = laid(&base, target)?;
            for &rewrite in rewrites {
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
            let texts: Vec<_> = dom
                .descendants(id)
                .filter(|&n| dom.node(n).node_type() == rdom_tui::core_api::NodeType::Text)
                .collect();
            for t in texts {
                let new = rewrite.apply(dom.node(t).node_value().unwrap_or_default());
                dom.node_mut(t)
                    .set_node_value(&new)
                    .map_err(|e| SessionError::InvalidInput(format!("{e:?}")))?;
            }
            Ok(())
        })?;
        self.record(Step::Rewrite {
            node: node.to_string(),
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
    let owners = s
        .capture(&CaptureRequest::with_layers([Layer::Owners]))?
        .owners
        .expect("asked for");
    let subtree: HashSet<String> = std::iter::once(id)
        .chain(dom.descendants(id))
        .map(node_key)
        .collect();
    let shown = owners
        .rows
        .iter()
        .flatten()
        .filter(|r| r.part == "text" && r.node.as_ref().is_some_and(|n| subtree.contains(n)))
        .map(|r| usize::from(r.w))
        .sum();
    let text = dom.text_content(id);
    let needed = unicode_width::UnicodeWidthStr::width(
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .as_str(),
    );
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

fn findings_of(key: String, rewrite: Rewrite, a: &Laid, b: &Laid) -> Stressed {
    let (before, after) = (a.boxes.get(&key).copied(), b.boxes.get(&key).copied());
    let mut findings = Vec::new();
    if b.shown < b.needed {
        findings.push(Finding {
            kind: "truncated",
            message: format!("{} of {} text cells show", b.shown, b.needed),
            nodes: vec![key.clone()],
        });
    }
    if let (Some(x), Some(y)) = (before, after)
        && y[3] > x[3]
    {
        findings.push(Finding {
            kind: "wrapped",
            message: format!("{} rows tall, was {}", y[3], x[3]),
            nodes: vec![key.clone()],
        });
    }
    if let (Some(me), Some(p)) = (after, b.parent.as_ref().and_then(|p| b.boxes.get(p))) {
        let past = (me[0] + me[2]) - (p[0] + p[2]);
        if past > 0 || me[0] < p[0] {
            findings.push(Finding {
                kind: "overflow",
                message: format!("{} cells past its parent's box", past.max(p[0] - me[0])),
                nodes: vec![key.clone(), b.parent.clone().unwrap_or_default()],
            });
        }
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
    if !shifted.is_empty() {
        findings.push(Finding {
            kind: "shifted",
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
                kind: "overlap",
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
