//! Private helpers used across the `TuiAccessors` / `TuiAccessorsMut`
//! impl blocks. Visibility is `pub(super)` so siblings (`read_ref`,
//! `read_mut`, `write`) can call in; nothing in this file is part of
//! the public API.

use rdom_core::NodeId;

use crate::{Result, TuiDom, TuiExt};

/// Parse a numeric f64 attribute. Used by `<progress>` /
/// `<meter>` accessors — mirrors the parsing in
/// `runtime::builtins::gauge::parse_attr` but is kept local to
/// avoid bumping that helper to `pub`.
pub(super) fn parse_numeric_attribute(
    node: &rdom_core::NodeRef<'_, TuiExt>,
    name: &str,
) -> Option<f64> {
    node.get_attribute(name).and_then(|s| s.parse().ok())
}

pub(super) fn write_boolean_attribute(
    node: &mut rdom_core::NodeMut<'_, TuiExt>,
    name: &str,
    value: bool,
) -> Result<()> {
    if value {
        node.set_attribute(name, "")
    } else {
        node.remove_attribute(name).map(|_| ())
    }
}

/// Mark the first descendant `<option>` whose effective value matches
/// `target` as `selected`; clear `selected` from every other option.
/// No match → every option ends up unselected. Matches
/// `HTMLSelectElement.value` setter.
pub(super) fn set_select_value(dom: &mut TuiDom, select: NodeId, target: &str) -> Result<()> {
    crate::runtime::builtins::select::note_default_selected(dom, select);
    let options: Vec<NodeId> = collect_options(dom, select);
    // Find the first match in document order.
    let first_match: Option<NodeId> = options
        .iter()
        .copied()
        .find(|&id| crate::runtime::builtins::select::option_value(dom, id) == target);
    for opt in options {
        let should_select = Some(opt) == first_match;
        let is_selected = dom.has_attribute(opt, "selected");
        if should_select && !is_selected {
            dom.set_attribute(opt, "selected", "")?;
        } else if !should_select && is_selected {
            dom.remove_attribute(opt, "selected")?;
        }
    }
    Ok(())
}

fn collect_options(dom: &TuiDom, root: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk_options(dom, root, &mut out);
    out
}

fn walk_options(dom: &TuiDom, id: NodeId, out: &mut Vec<NodeId>) {
    if dom.node(id).tag_name() == Some("option") {
        out.push(id);
    }
    for child in dom.node(id).child_nodes() {
        walk_options(dom, child.id(), out);
    }
}

pub(super) fn read_scroll_x(dom: &TuiDom, id: NodeId) -> i32 {
    use crate::node::TuiNodeExt;
    dom.node(id).tui_ext().map(|e| e.scroll_x).unwrap_or(0)
}

pub(super) fn read_scroll_y(dom: &TuiDom, id: NodeId) -> i32 {
    use crate::node::TuiNodeExt;
    dom.node(id)
        .tui_ext()
        .map(|e| e.scroll_y as i32)
        .unwrap_or(0)
}

/// `HTMLElement.isContentEditable` (HTML §6.8.1): walk ancestors
/// through the inherit state (`Dom::content_editable_state` — keywords
/// ASCII case-insensitive, an invalid value inherits):
///
/// - true (`true` / `""`) or plaintext-only → `true`;
/// - false → `false` (overrides any inherited true from a higher
///   ancestor);
/// - inherit (absent / invalid) → continue walking.
///
/// Falls off the root as `false`.
pub(super) fn effective_content_editable(dom: &TuiDom, id: NodeId) -> bool {
    let mut cur = Some(id);
    while let Some(node_id) = cur {
        if let Some(state) = dom.content_editable_state(node_id) {
            return state.is_editing_host();
        }
        cur = dom.node(node_id).parent_node().map(|p| p.id());
    }
    false
}
