//! Option-list model and read API for `<select>`: which `<option>`s
//! belong to a select (descending `<optgroup>`), which are selected,
//! how a select / option reports its `value` and label, whether the
//! select is `multiple`, its HTML "display size", and the ancestor
//! walks the click path uses to find the option / select under a
//! target. Everything here is read-only over the DOM.

use rdom_core::NodeId;

use crate::TuiDom;

// ── Public read API ────────────────────────────────────────────────

/// Collect every currently-selected `<option>` under `select`, in
/// document order. Used by `form::collect` and by apps that want
/// to read the current selection.
pub fn selected_options(dom: &TuiDom, select: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    for opt in options(dom, select) {
        if dom.node(opt).has_attribute("selected") {
            out.push(opt);
        }
    }
    out
}

/// The current value of a `<select>`. Single-select: value of the
/// selected option (or empty if none). Multi-select: space-
/// separated values of all selected options (matches the legacy
/// rdom serialization).
pub fn value(dom: &TuiDom, select: NodeId) -> String {
    let selected = selected_options(dom, select);
    if selected.is_empty() {
        return String::new();
    }
    if is_multi(dom, select) {
        selected
            .iter()
            .map(|&id| option_value(dom, id))
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        option_value(dom, selected[0])
    }
}

/// Read an `<option>`'s submit value: the `value` attribute, or
/// (per HTML) the text content when `value` is absent.
pub fn option_value(dom: &TuiDom, option: NodeId) -> String {
    if let Some(v) = dom.node(option).get_attribute("value") {
        return v.to_string();
    }
    option_label(dom, option)
}

/// Read an `<option>`'s display label: the `label` attribute if
/// set, otherwise the text content.
pub fn option_label(dom: &TuiDom, option: NodeId) -> String {
    if let Some(v) = dom.node(option).get_attribute("label") {
        return v.to_string();
    }
    let mut text = String::new();
    for child in dom.node(option).child_nodes() {
        if child.node_type() == rdom_core::NodeType::Text
            && let Some(s) = child.node_value()
        {
            text.push_str(s);
        }
    }
    text
}

// ── Tree traversal helpers ─────────────────────────────────────────

/// Is this select a multi-select? Only the `multiple` attribute makes
/// one (HTML §4.10.7); a `size > 1` list box without `multiple` is
/// still single-select.
pub(super) fn is_multi(dom: &TuiDom, select: NodeId) -> bool {
    dom.node(select).has_attribute("multiple")
}

/// HTML §4.10.7 "display size": the `size` attribute parsed as a
/// non-negative integer greater than zero, else the default of 1
/// (4 when `multiple`, which does not matter for the dropdown test).
pub(super) fn display_size(dom: &TuiDom, select: NodeId) -> u32 {
    dom.node(select)
        .get_attribute("size")
        .and_then(|s| s.trim().parse::<u32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(1)
}

/// Collect every `<option>` descendant of `select`, in document
/// order. Descends into `<optgroup>` children (options nest under
/// groups per HTML).
pub fn options(dom: &TuiDom, select: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk_options(dom, select, &mut out);
    out
}

fn walk_options(dom: &TuiDom, id: NodeId, out: &mut Vec<NodeId>) {
    for child in dom.node(id).child_nodes() {
        match child.tag_name() {
            Some("option") => out.push(child.id()),
            Some("optgroup") => walk_options(dom, child.id(), out),
            _ => {}
        }
    }
}

pub(super) fn closest_option(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if dom.node(n).tag_name() == Some("option") {
            return Some(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
}

/// Walk up from `id` (inclusive) to the nearest `<select>`.
/// Same logic as `enclosing_select`; separate name reflects its
/// use from the click path where the target might be the select
/// chrome itself, not an option descendant.
pub(super) fn closest_select(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    enclosing_select(dom, id)
}

pub(super) fn enclosing_select(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if dom.node(n).tag_name() == Some("select") {
            return Some(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
}
