//! Dropdown open / close state (C.7b): the `data-rdom-open` marker
//! on a single-select with display size 1, and the dropdown-vs-
//! listbox test that gates it. Listboxes (`multiple` or `size > 1`)
//! have no open/closed state — their option list is always visible.

use rdom_core::NodeId;

use super::model::{display_size, option_label, selected_options};
use crate::TuiDom;
use crate::render::paint_pass::ChromeText;

/// Marker: dropdown is open (options expanded below chrome). Only
/// meaningful for single-select dropdowns — listboxes (`multiple`
/// or `size`) ignore this attribute and are always "open."
const OPEN_ATTR: &str = "data-rdom-open";

/// True when this select is a drop-down box: no `multiple` and a
/// display size of 1 (HTML §4.10.7 — `size="1"`, `size="0"`, or a
/// non-numeric size all mean 1). List boxes (`multiple` or `size > 1`)
/// always show their option list and have no open/closed state.
pub fn is_dropdown(dom: &TuiDom, select: NodeId) -> bool {
    !dom.node(select).has_attribute("multiple") && display_size(dom, select) <= 1
}

/// True when this dropdown is currently open. Always `false`
/// for listbox-mode selects.
pub fn is_open(dom: &TuiDom, select: NodeId) -> bool {
    dom.node(select).has_attribute(OPEN_ATTR)
}

/// Open a dropdown — set the open marker. No-op for listbox-
/// mode selects (their option list is always visible).
pub fn open(dom: &mut TuiDom, select: NodeId) {
    if !is_dropdown(dom, select) {
        return;
    }
    let _ = dom.set_attribute(select, OPEN_ATTR, "");
}

/// Close a dropdown — clear the open marker. No-op for listbox-
/// mode selects.
pub fn close(dom: &mut TuiDom, select: NodeId) {
    if !is_dropdown(dom, select) {
        return;
    }
    let _ = dom.remove_attribute(select, OPEN_ATTR);
}

/// The paint pass's chrome for a closed dropdown (see
/// `runtime::builtins::inline_chrome`): the selected-option label to
/// paint as the `<select>`'s own text — the option elements
/// themselves are hidden by UA `display: none`, so the runtime
/// echoes the selected label into the select's content area
/// instead. The dropdown affordance (`▾` chevron at the right
/// edge) is supplied separately by the UA
/// `select:not([multiple]):not([size]):not([data-rdom-open])::after`
/// rule, NOT prepended here.
///
/// `None` for any non-select, for listbox-mode selects (which
/// paint their options directly), and for open dropdowns
/// (where the option children paint themselves via the normal
/// traversal).
pub(crate) fn inline_chrome(dom: &TuiDom, id: NodeId, _width: u16) -> Option<ChromeText> {
    if dom.node(id).tag_name() != Some("select") {
        return None;
    }
    if !is_dropdown(dom, id) {
        return None;
    }
    if is_open(dom, id) {
        return None;
    }
    let selected = selected_options(dom, id);
    let label = selected
        .first()
        .map(|&opt| option_label(dom, opt))
        .unwrap_or_default();
    Some(ChromeText {
        text: label,
        fg: None,
    })
}
