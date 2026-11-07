//! The UX checks (`specs/CAPTURE.md` §Audits, UX). The ones that act —
//! focusing, pressing Tab — do so in a fork, so the session is left as
//! it was.

use rdom_tui::core_api::{ForcedState, NodeId};
use rdom_tui::runtime::focus::tabindex::focusable_elements;
use serde_json::json;

use super::frame::Frame;
use super::{Checked, DesignSystem, Finding, Severity};
use crate::app::CaptureApp;
use crate::capture::node_key;
use crate::input::Input;
use crate::query::outline::{name, role};
use crate::session::Session;

/// The most elements the acting checks try.
const MAX_ELEMENTS: usize = 48;

/// Focusable elements whose `:focus-visible` (and `:focus`) changes no
/// cell: a keyboard user cannot see where they are.
pub(super) fn focus_visible<A: CaptureApp>(s: &Session<A>, f: &Frame<'_>) -> Checked {
    let targets: Vec<NodeId> = focusable_elements(f.app.dom())
        .into_iter()
        .filter(|id| f.boxes.contains_key(id))
        .take(MAX_ELEMENTS)
        .collect();
    if targets.is_empty() {
        return Ok(Vec::new());
    }
    let mut fork = s
        .fork()
        .map_err(|e| format!("cannot fork the session: {e}"))?;
    let mut out = Vec::new();
    for id in targets {
        let key = node_key(id);
        let mut changed = 0;
        for state in [ForcedState::Focus, ForcedState::FocusVisible] {
            changed += fork
                .force(&key, state, true)
                .map_err(|e| e.to_string())?
                .cells_changed;
        }
        for state in [ForcedState::FocusVisible, ForcedState::Focus] {
            fork.force(&key, state, false).map_err(|e| e.to_string())?;
        }
        if changed == 0 {
            out.push(Finding {
                check: "focus-visible",
                severity: Severity::Error,
                message: "focusing it changes no cell: no visible focus indicator".into(),
                nodes: vec![f.key(id)],
                cells: Vec::new(),
                inputs: json!({"cells_changed": 0}),
                threshold: json!({"cells_changed": 1}),
            });
        }
    }
    Ok(out)
}

/// Focusable elements Tab never reaches, and Tab steps that go against
/// the reading order (up, or left on the same row).
pub(super) fn reachability<A: CaptureApp>(s: &Session<A>, f: &Frame<'_>) -> Checked {
    let focusable: Vec<NodeId> = focusable_elements(f.app.dom())
        .into_iter()
        .filter(|id| f.boxes.contains_key(id))
        .take(MAX_ELEMENTS)
        .collect();
    if focusable.is_empty() {
        return Ok(Vec::new());
    }
    let mut fork = s
        .fork()
        .map_err(|e| format!("cannot fork the session: {e}"))?;
    let mut visited: Vec<NodeId> = Vec::new();
    for _ in 0..focusable.len() * 2 + 2 {
        fork.input(&[Input::Key { key: "tab".into() }])
            .map_err(|e| e.to_string())?;
        let Some(active) = fork
            .app()
            .map_err(|e| e.to_string())?
            .dom()
            .active_element()
            .map(|n| n.id())
        else {
            continue;
        };
        if visited.first() == Some(&active) && visited.len() > 1 {
            break;
        }
        if visited.last() != Some(&active) {
            visited.push(active);
        }
    }
    let mut out = Vec::new();
    for id in &focusable {
        if !visited.contains(id) {
            out.push(Finding {
                check: "reachability",
                severity: Severity::Error,
                message: "focusable, but Tab never reaches it".into(),
                nodes: vec![f.key(*id)],
                cells: Vec::new(),
                inputs: json!({"tab_presses": focusable.len() * 2 + 2}),
                threshold: serde_json::Value::Null,
            });
        }
    }
    for pair in visited.windows(2) {
        let (Some(a), Some(b)) = (f.boxes.get(&pair[0]), f.boxes.get(&pair[1])) else {
            continue;
        };
        let (a, b) = (a.border, b.border);
        let back = b[1] + b[3] <= a[1] || (b[1] == a[1] && b[0] < a[0]);
        if back {
            out.push(Finding {
                check: "reachability",
                severity: Severity::Info,
                message: "Tab goes from one to the other against the reading order".into(),
                nodes: vec![f.key(pair[0]), f.key(pair[1])],
                cells: Vec::new(),
                inputs: json!({"from": a, "to": b}),
                threshold: serde_json::Value::Null,
            });
        }
    }
    Ok(out)
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
    ids.sort();
    let [w, h] = d.min_target.map(i32::from);
    Ok(ids
        .into_iter()
        .filter_map(|id| Some((id, f.boxes.get(&id)?.border)))
        .filter(|(_, b)| b[2] > 0 && b[3] > 0 && (b[2] < w || b[3] < h))
        .map(|(id, b)| Finding {
            check: "targets",
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
        if let Some(level) = tag
            .strip_prefix('h')
            .and_then(|n| n.parse::<u8>().ok())
            .filter(|n| (1..=6).contains(n))
        {
            if let Some((prev, prev_id)) = last
                && level > prev + 1
            {
                out.push(Finding {
                    check: "landmarks",
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
                check: "landmarks",
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
