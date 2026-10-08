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

/// Open a dropdown — set the open marker and put its picker in the top
/// layer (`TopLayerKind::Picker`, HTML's `::picker(select)`): the option
/// list overlays the page from the select's row, outside every clip, while
/// the select keeps its one row in flow. No-op for listbox-mode selects
/// (their option list is always visible) and for a select not in the
/// document.
pub fn open(dom: &mut TuiDom, select: NodeId) {
    if !is_dropdown(dom, select) || !dom.node(select).is_connected() {
        return;
    }
    let _ = dom.set_attribute(select, OPEN_ATTR, "");
    if !dom.is_in_top_layer(select) {
        dom.add_to_top_layer(select, rdom_core::TopLayerKind::Picker)
            .expect("a connected element joins the top layer");
    }
}

/// Close a dropdown — clear the open marker and take its picker out of
/// the top layer. No-op for listbox-mode selects.
pub fn close(dom: &mut TuiDom, select: NodeId) {
    if !is_dropdown(dom, select) {
        return;
    }
    let _ = dom.remove_attribute(select, OPEN_ATTR);
    if dom.top_layer_kind(select) == Some(rdom_core::TopLayerKind::Picker) {
        dom.remove_from_top_layer(select);
    }
}

/// The open pickers (`TopLayerKind::Picker`), bottom to top.
fn open_pickers(dom: &TuiDom) -> Vec<NodeId> {
    dom.top_layer()
        .iter()
        .copied()
        .filter(|&id| dom.top_layer_kind(id) == Some(rdom_core::TopLayerKind::Picker))
        .collect()
}

/// The open pickers a press at `target` began inside (document data for
/// the release, [`light_dismiss_up`]).
#[derive(Debug, Default)]
struct PressedPickers(Vec<NodeId>);

/// Light dismiss, as a popover's (HTML §6.12.2): note which open pickers
/// a press at `target` (`None`: on nothing) began inside.
pub(crate) fn light_dismiss_down(dom: &mut TuiDom, target: Option<NodeId>) {
    let pickers = open_pickers(dom);
    if pickers.is_empty() {
        return;
    }
    let inside = pickers
        .into_iter()
        .filter(|&p| target.is_some_and(|t| crate::node::is_descendant_or_self(dom, t, p)))
        .collect();
    dom.set_document_data(PressedPickers(inside));
}

/// Light dismiss: a release at `target` closes every open picker that
/// neither the press nor the release was inside. Whether one closed.
pub(crate) fn light_dismiss_up(dom: &mut TuiDom, target: Option<NodeId>) -> bool {
    let pressed = dom
        .remove_document_data::<PressedPickers>()
        .map_or_else(Vec::new, |p| p.0);
    let mut closed = false;
    for picker in open_pickers(dom) {
        let released_in =
            target.is_some_and(|t| crate::node::is_descendant_or_self(dom, t, picker));
        if !released_in && !pressed.contains(&picker) {
            close(dom, picker);
            closed = true;
        }
    }
    closed
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
