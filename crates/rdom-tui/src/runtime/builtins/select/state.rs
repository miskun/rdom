//! Selection-state mutation shared by the click, keyboard, and
//! type-ahead paths: the `selected` attribute writes (single pick,
//! toggle, anchor-to-target range extend), the `data-rdom-highlight`
//! / `data-rdom-anchor` marker accessors, and the `input` + `change`
//! pair fired after every user-driven change.

use rdom_core::NodeId;

use super::model::{option_disabled, options, selected_options};
use crate::tui_event::TuiDispatchExt;
use crate::{TuiDom, TuiEvent};

/// Marker: the currently-focused option within a multi-select.
const HIGHLIGHT_ATTR: &str = "data-rdom-highlight";
/// Marker: anchor for shift-extend range selection (multi-select).
const ANCHOR_ATTR: &str = "data-rdom-anchor";

// ── Defaults (defaultSelected) ─────────────────────────────────────

/// Record every option's current `selected` attribute as its
/// `defaultSelected`, where not yet known. The attribute is the live
/// selectedness (as `checked` is for toggles), so it is captured before
/// the select's first change; options added later are captured before
/// the next one. Every runtime write below calls this first.
pub(crate) fn note_default_selected(dom: &mut TuiDom, select: NodeId) {
    for opt in options(dom, select) {
        let selected = dom.node(opt).has_attribute("selected");
        if let Some(ext) = dom.node_mut(opt).ext_mut()
            && ext.default_selected.is_none()
        {
            ext.default_selected = Some(selected);
        }
    }
}

/// HTML §4.10.7 reset algorithm for `<select>`: every option goes back
/// to its `defaultSelected` (an option never captured still holds its
/// authored attribute and is left alone), then the selectedness setting
/// algorithm runs. No events fire.
pub(crate) fn reset_to_default(dom: &mut TuiDom, select: NodeId) {
    for opt in options(dom, select) {
        match dom.node(opt).ext().and_then(|e| e.default_selected) {
            Some(true) => {
                let _ = dom.set_attribute(opt, "selected", "");
            }
            Some(false) => {
                let _ = dom.remove_attribute(opt, "selected");
            }
            None => {}
        }
    }
    super::selectedness::run(dom, select);
}

// ── Selection writes ───────────────────────────────────────────────

pub(super) fn select_single(dom: &mut TuiDom, select: NodeId, option: NodeId) {
    let current = selected_options(dom, select);
    let already = current.len() == 1 && current[0] == option;
    if already {
        return;
    }
    note_default_selected(dom, select);
    for opt in current {
        let _ = dom.remove_attribute(opt, "selected");
    }
    let _ = dom.set_attribute(option, "selected", "");
}

pub(super) fn toggle_option(dom: &mut TuiDom, select: NodeId, option: NodeId) {
    note_default_selected(dom, select);
    if dom.node(option).has_attribute("selected") {
        let _ = dom.remove_attribute(option, "selected");
    } else {
        let _ = dom.set_attribute(option, "selected", "");
    }
}

/// Shift-click / shift-arrow: extend selection from the anchor
/// option to `target`. Sets the anchor on first shift-action if
/// absent. All options between anchor and target (inclusive)
/// become selected; others outside the range are deselected.
pub(super) fn extend_selection_to(dom: &mut TuiDom, select: NodeId, target: NodeId) {
    let anchor = anchor(dom, select).unwrap_or_else(|| {
        // First shift action — anchor snaps to the current highlight
        // (or the target itself when no highlight exists yet).
        let a = highlight(dom, select).unwrap_or(target);
        set_anchor(dom, select, Some(a));
        a
    });
    let all = options(dom, select);
    let a_idx = all.iter().position(|&o| o == anchor);
    let t_idx = all.iter().position(|&o| o == target);
    let (Some(a_idx), Some(t_idx)) = (a_idx, t_idx) else {
        return;
    };
    note_default_selected(dom, select);
    let (lo, hi) = if a_idx <= t_idx {
        (a_idx, t_idx)
    } else {
        (t_idx, a_idx)
    };
    for (i, &opt) in all.iter().enumerate() {
        if option_disabled(dom, opt) {
            continue;
        }
        if i >= lo && i <= hi {
            let _ = dom.set_attribute(opt, "selected", "");
        } else {
            let _ = dom.remove_attribute(opt, "selected");
        }
    }
}

// ── Event firing ───────────────────────────────────────────────────

/// Fire `input` then `change` on the select (matches the pattern
/// from C.4b toggle). Both non-cancelable; apps observe.
pub(super) fn fire_input_and_change(dom: &mut TuiDom, select: NodeId) {
    let mut input_ev = TuiEvent::new("input");
    let _ = dom.dispatch_tui_event(select, &mut input_ev);
    let mut change_ev = TuiEvent::new("change");
    let _ = dom.dispatch_tui_event(select, &mut change_ev);
}

// ── Highlight / anchor attribute helpers ───────────────────────────

pub(super) fn highlight(dom: &TuiDom, select: NodeId) -> Option<NodeId> {
    options(dom, select)
        .into_iter()
        .find(|&o| dom.node(o).has_attribute(HIGHLIGHT_ATTR))
}

pub(super) fn set_highlight(dom: &mut TuiDom, select: NodeId, target: Option<NodeId>) {
    for opt in options(dom, select) {
        let want = target == Some(opt);
        let has = dom.node(opt).has_attribute(HIGHLIGHT_ATTR);
        if want && !has {
            let _ = dom.set_attribute(opt, HIGHLIGHT_ATTR, "");
        } else if !want && has {
            let _ = dom.remove_attribute(opt, HIGHLIGHT_ATTR);
        }
    }
}

pub(super) fn anchor(dom: &TuiDom, select: NodeId) -> Option<NodeId> {
    options(dom, select)
        .into_iter()
        .find(|&o| dom.node(o).has_attribute(ANCHOR_ATTR))
}

pub(super) fn set_anchor(dom: &mut TuiDom, select: NodeId, target: Option<NodeId>) {
    for opt in options(dom, select) {
        let want = target == Some(opt);
        let has = dom.node(opt).has_attribute(ANCHOR_ATTR);
        if want && !has {
            let _ = dom.set_attribute(opt, ANCHOR_ATTR, "");
        } else if !want && has {
            let _ = dom.remove_attribute(opt, ANCHOR_ATTR);
        }
    }
}
