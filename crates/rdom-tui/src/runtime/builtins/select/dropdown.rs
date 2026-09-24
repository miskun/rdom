//! Dropdown open / close state (C.7b): the `data-rdom-open` marker
//! on a single-select with display size 1, and the dropdown-vs-
//! listbox test that gates it. Listboxes (`multiple` or `size > 1`)
//! have no open/closed state — their option list is always visible.

use rdom_core::NodeId;

use super::model::display_size;
use crate::TuiDom;

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
