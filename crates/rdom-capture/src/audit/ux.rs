//! The UX checks (`specs/CAPTURE.md` §Audits, UX). The ones that act —
//! focusing, pressing Tab — do so in a fork, so the session is left as
//! it was.

use rdom_tui::core_api::NodeId;
use rdom_tui::runtime::focus::tabindex::focusable_elements;
use serde_json::json;

use super::frame::Frame;
use super::{Check, Checked, DesignSystem, Finding, Ran, Severity};
use crate::app::CaptureApp;
use crate::capture::Layer;
use crate::input::Input;
use crate::query::outline::{name, role};
use crate::session::{CaptureRequest, Session};

/// The most elements the Tab walk measures.
const MAX_ELEMENTS: usize = 48;

/// How long each Tab step settles before it is measured: a focus style's
/// transition ends.
const SETTLE_MS: u64 = 300;

/// A Tab walk through the page in a fork — real focus, keyboard modality —
/// for `focus-visible` and `reachability`.
pub(super) struct Walk(Result<Walked, String>);

struct Walked {
    /// The focusable elements (at most [`MAX_ELEMENTS`]), and how many
    /// there are.
    focusable: Vec<NodeId>,
    total: usize,
    /// Each element Tab focused, in order, with the cells its focus
    /// changed in its own region (its border box and a cell around it),
    /// and that region.
    visited: Vec<(NodeId, u32, [i32; 4])>,
    presses: usize,
}

/// Press Tab through a fork of `s`, measuring each focus.
pub(super) fn walk<A: CaptureApp>(s: &Session<A>, f: &Frame<'_>) -> Walk {
    let all: Vec<NodeId> = focusable_elements(f.app.dom())
        .into_iter()
        .filter(|id| f.boxes.contains_key(id))
        .collect();
    let total = all.len();
    let focusable: Vec<NodeId> = all.into_iter().take(MAX_ELEMENTS).collect();
    if focusable.is_empty() {
        return Walk(Ok(Walked {
            focusable,
            total,
            visited: Vec::new(),
            presses: 0,
        }));
    }
    Walk((|| -> Result<Walked, String> {
        let mut fork = s
            .fork()
            .map_err(|e| format!("cannot fork the session: {e}"))?;
        let grid = |fork: &Session<A>| -> Result<Vec<Vec<String>>, String> {
            let c = fork
                .capture(&CaptureRequest::with_layers([Layer::Grid]))
                .map_err(|e| e.to_string())?;
            Ok(cells(&c.grid.expect("asked for")))
        };
        let mut before = grid(&fork)?;
        let mut visited: Vec<(NodeId, u32, [i32; 4])> = Vec::new();
        let presses = focusable.len() * 2 + 2;
        for _ in 0..presses {
            fork.input(&[Input::Key { key: "tab".into() }])
                .map_err(|e| e.to_string())?;
            fork.advance(SETTLE_MS).map_err(|e| e.to_string())?;
            let after = grid(&fork)?;
            let app = fork.app().map_err(|e| e.to_string())?;
            let active = app.dom().active_element().map(|n| n.id());
            if let Some(id) = active {
                if visited.first().is_some_and(|v| v.0 == id) && visited.len() > 1 {
                    break;
                }
                if !visited.iter().any(|v| v.0 == id) {
                    let region = rdom_tui::capture::box_model(app.dom(), id)
                        .map(|b| b.border)
                        .map(|b| {
                            [
                                b.x - 1,
                                b.y - 1,
                                i32::from(b.width) + 2,
                                i32::from(b.height) + 2,
                            ]
                        })
                        .unwrap_or([0, 0, 0, 0]);
                    let changed = differ(&before, &after, region);
                    visited.push((id, changed, region));
                }
            }
            before = after;
        }
        Ok(Walked {
            focusable,
            total,
            visited,
            presses,
        })
    })())
}

/// Each cell's glyph and colors, as a comparable string.
fn cells(grid: &crate::capture::Grid) -> Vec<Vec<String>> {
    crate::motion::cells_of(grid)
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|(g, fg, bg)| format!("{g}{fg}{bg}"))
                .collect()
        })
        .collect()
}

/// How many cells of `region` differ between `a` and `b`.
fn differ(a: &[Vec<String>], b: &[Vec<String>], [x, y, w, h]: [i32; 4]) -> u32 {
    let mut n = 0;
    for yy in y.max(0)..y + h {
        for xx in x.max(0)..x + w {
            let at =
                |g: &[Vec<String>]| g.get(yy as usize).and_then(|r| r.get(xx as usize)).cloned();
            if at(a) != at(b) {
                n += 1;
            }
        }
    }
    n
}

fn capped(w: &Walked) -> Option<String> {
    (w.total > w.focusable.len()).then(|| {
        format!(
            "{} focusable elements; the first {} were walked",
            w.total,
            w.focusable.len()
        )
    })
}

/// Elements whose real focus (Tab, keyboard modality) changes no cell in
/// their own region once settled: a keyboard user cannot see where they
/// are.
pub(super) fn focus_visible(walk: &Walk, f: &Frame<'_>) -> Result<Ran, String> {
    let w = walk.0.as_ref().map_err(Clone::clone)?;
    let findings = w
        .visited
        .iter()
        .filter(|(_, changed, _)| *changed == 0)
        .map(|(id, _, region)| Finding {
            check: Check::FocusVisible,
            severity: Severity::Error,
            message: "focusing it with Tab changes no cell around it: no visible focus indicator"
                .into(),
            nodes: vec![f.key(*id)],
            cells: Vec::new(),
            inputs: json!({"cells_changed": 0, "region": region, "settled_ms": SETTLE_MS}),
            threshold: json!({"cells_changed": 1}),
        })
        .collect();
    Ok(Ran {
        findings,
        partial: capped(w),
    })
}

/// Focusable elements Tab never reaches, and Tab steps that go against
/// the reading order (up, or left on the same row).
pub(super) fn reachability(walk: &Walk, f: &Frame<'_>) -> Result<Ran, String> {
    let w = walk.0.as_ref().map_err(Clone::clone)?;
    let order: Vec<String> = w.visited.iter().map(|v| f.key(v.0)).collect();
    let mut out = Vec::new();
    for id in &w.focusable {
        if !w.visited.iter().any(|v| v.0 == *id) {
            out.push(Finding {
                check: Check::Reachability,
                severity: Severity::Error,
                message: "focusable, but Tab never reaches it".into(),
                nodes: vec![f.key(*id)],
                cells: Vec::new(),
                inputs: json!({"tab_presses": w.presses, "reached": order, "stuck_at": order.last()}),
                threshold: serde_json::Value::Null,
            });
        }
    }
    for pair in w.visited.windows(2) {
        let (Some(a), Some(b)) = (f.boxes.get(&pair[0].0), f.boxes.get(&pair[1].0)) else {
            continue;
        };
        let (a, b) = (a.border, b.border);
        let back = b[1] + b[3] <= a[1] || (b[1] == a[1] && b[0] < a[0]);
        if back {
            out.push(Finding {
                check: Check::Reachability,
                severity: Severity::Info,
                message: "Tab goes from one to the other against the reading order".into(),
                nodes: vec![f.key(pair[0].0), f.key(pair[1].0)],
                cells: Vec::new(),
                inputs: json!({"from": a, "to": b}),
                threshold: serde_json::Value::Null,
            });
        }
    }
    Ok(Ran {
        findings: out,
        partial: capped(w),
    })
}

/// Interactive elements smaller than the least target.
pub(super) fn targets(f: &Frame<'_>, d: &DesignSystem) -> Checked {
    let dom = f.app.dom();
    let mut ids = focusable_elements(dom);
    for id in dom.descendants(dom.root()) {
        let tag = dom
            .node(id)
            .tag_name()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(tag.as_str(), "button" | "summary" | "select") && !ids.contains(&id) {
            ids.push(id);
        }
    }
    // An inactive control is no target.
    ids.retain(|&id| !dom.matches(id, ":disabled").unwrap_or(false));
    ids.sort();
    let [w, h] = d.min_target.map(i32::from);
    Ok(ids
        .into_iter()
        .filter_map(|id| Some((id, f.boxes.get(&id)?.border)))
        .filter(|(_, b)| b[2] > 0 && b[3] > 0 && (b[2] < w || b[3] < h))
        .map(|(id, b)| Finding {
            check: Check::Targets,
            severity: Severity::Warning,
            message: format!("a {} × {} target, under {w} × {h}", b[2], b[3]),
            nodes: vec![f.key(id)],
            cells: vec![[
                b[0].max(0) as u16,
                b[1].max(0) as u16,
                b[2] as u16,
                b[3] as u16,
            ]],
            inputs: json!({"size": [b[2], b[3]]}),
            threshold: json!({"size": [w, h]}),
        })
        .collect())
}

/// Headings that skip a level, and landmarks of one role, several of them,
/// some without a name to tell them apart.
pub(super) fn landmarks(f: &Frame<'_>) -> Checked {
    let dom = f.app.dom();
    let mut out = Vec::new();
    let mut last: Option<(u8, NodeId)> = None;
    let mut by_role: std::collections::BTreeMap<String, Vec<(NodeId, Option<String>)>> =
        Default::default();
    for id in dom.descendants(dom.root()) {
        let tag = dom
            .node(id)
            .tag_name()
            .unwrap_or_default()
            .to_ascii_lowercase();
        // A heading that does not render is no heading of the page.
        let rendered = f.boxes.contains_key(&id);
        if let Some(level) = tag
            .strip_prefix('h')
            .and_then(|n| n.parse::<u8>().ok())
            .filter(|n| (1..=6).contains(n) && rendered)
        {
            if let Some((prev, prev_id)) = last
                && level > prev + 1
            {
                out.push(Finding {
                    check: Check::Landmarks,
                    severity: Severity::Warning,
                    message: format!("an h{level} follows an h{prev}: a heading level is skipped"),
                    nodes: vec![f.key(prev_id), f.key(id)],
                    cells: Vec::new(),
                    inputs: json!({"from": prev, "to": level}),
                    threshold: serde_json::Value::Null,
                });
            }
            last = Some((level, id));
        }
        if let Some(r) = role(dom, id)
            && matches!(
                r.as_str(),
                "main"
                    | "navigation"
                    | "complementary"
                    | "banner"
                    | "contentinfo"
                    | "search"
                    | "form"
                    | "region"
            )
        {
            by_role
                .entry(r.clone())
                .or_default()
                .push((id, name(dom, id, &r)));
        }
    }
    for (r, list) in by_role {
        if list.len() < 2 {
            continue;
        }
        let unnamed: Vec<NodeId> = list
            .iter()
            .filter(|(_, n)| n.is_none())
            .map(|(id, _)| *id)
            .collect();
        if !unnamed.is_empty() {
            out.push(Finding {
                check: Check::Landmarks,
                severity: Severity::Warning,
                message: format!(
                    "{} {r} landmarks, {} without a name to tell them apart",
                    list.len(),
                    unnamed.len()
                ),
                nodes: unnamed.iter().map(|n| f.key(*n)).collect(),
                cells: Vec::new(),
                inputs: json!({"role": r, "count": list.len()}),
                threshold: serde_json::Value::Null,
            });
        }
    }
    Ok(out)
}
