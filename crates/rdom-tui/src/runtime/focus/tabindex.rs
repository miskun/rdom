//! `tabindex` attribute handling + Tab / Shift-Tab navigation.
//!
//! ## Spec
//!
//! Per HTML:
//!
//! - **tabindex > 0**: element is reachable via Tab; visited in
//!   ascending numeric order. Multiple elements with the same
//!   positive `tabindex` visit in document order.
//! - **tabindex = 0**: reachable via Tab in document order, after
//!   every positive-tabindex element.
//! - **tabindex < 0**: programmatically focusable (via
//!   `focus_node`) but **not** reachable via Tab.
//! - **tabindex absent**: not focusable at all.
//!
//! ## Navigation
//!
//! [`focus_next`] / [`focus_prev`] move focus through the
//! focusable-in-tab-order set, wrapping at the ends. If nothing is
//! currently focused, `focus_next` starts at the first element;
//! `focus_prev` starts at the last.

use rdom_core::NodeId;

use crate::TuiDom;
use crate::layout::Overflow;
use crate::node::TuiNodeExt;

/// Effective tabindex for focus ordering. Reflects the full HTML
/// "focusable area" rules — not just the literal `tabindex`
/// attribute:
///
/// 1. `disabled` elements are NEVER focusable (returns `None`).
/// 2. Explicit `tabindex` attribute wins when present.
/// 3. **Implicit focusability**: certain tags are tab-reachable
///    without needing `tabindex="0"` — `<button>`, `<input>`
///    (except `type="hidden"`), `<textarea>`, `<details>`,
///    `<select>`, and `<a[href]>` / `<area[href]>`. Matches
///    the HTML living standard's focusable-area list.
///
/// Callers that want the raw attribute value should read the
/// attribute directly; `tab_index` is semantic.
pub fn tab_index(dom: &TuiDom, id: NodeId) -> Option<i32> {
    let node = dom.node(id);
    // Disabled elements never focus.
    if node.has_attribute("disabled") {
        return None;
    }
    // Explicit tabindex wins.
    if let Some(t) = node
        .get_attribute("tabindex")
        .and_then(|s| s.parse::<i32>().ok())
    {
        return Some(t);
    }
    // Implicit focusability — treat as `tabindex="0"`.
    if is_implicit_focusable(dom, id) {
        return Some(0);
    }
    None
}

/// HTML living-standard "focusable area" rules for elements
/// without an explicit `tabindex`. Called from `tab_index`.
///
/// Two sources of implicit focusability:
/// 1. **Intrinsic tags** — `<input>`, `<button>`, etc. ([`intrinsic_tag_focusable`]).
/// 2. **Scroll containers** — a clipping element whose content overflows is
///    keyboard-focusable so it can be scrolled (matching modern browsers),
///    *unless* it already contains a focus stop of its own (then a tab stop on
///    the container would be redundant — the web's rule). Its focus affordance
///    is the accent scrollbar thumb (`:focus::scrollbar-thumb`), not a fill.
fn is_implicit_focusable(dom: &TuiDom, id: NodeId) -> bool {
    if intrinsic_tag_focusable(dom, id) {
        return true;
    }
    is_scroll_container(dom, id) && !has_focusable_descendant(dom, id)
}

/// The tag-based focusable-area list (no scroll-container rule — that's handled
/// in [`is_implicit_focusable`] and would otherwise recurse).
fn intrinsic_tag_focusable(dom: &TuiDom, id: NodeId) -> bool {
    let node = dom.node(id);
    let Some(tag) = node.tag_name() else {
        return false;
    };
    match tag {
        // `<input type="hidden">` is NOT focusable. Every other input type is.
        "input" => !matches!(node.get_attribute("type"), Some("hidden")),
        "button" | "textarea" | "select" => true,
        // `<summary>` is the focus target of a `<details>` disclosure widget.
        "summary" => true,
        // Anchors + image map areas need `href` to be focusable.
        "a" | "area" => node.has_attribute("href"),
        // ARIA tree container: `<ul role=tree>` holds focus on behalf of its
        // active descendant (the cursor row). See `runtime::builtins::tree`.
        _ => node.get_attribute("role") == Some("tree"),
    }
}

/// True when `id` shows a scrollbar — it clips on an axis (`overflow: scroll`
/// or `auto`) and its content overflows the scrollport, so there's somewhere
/// to scroll. The TUI's keyboard-focusable scroll region.
fn is_scroll_container(dom: &TuiDom, id: NodeId) -> bool {
    let node = dom.node(id);
    let (Some(ext), Some(c)) = (node.tui_ext(), node.computed()) else {
        return false;
    };
    let pb = rdom_style::layout::compute_padding_box(ext.layout, c.border);
    let scrolls_y = matches!(c.overflow_y, Overflow::Scroll | Overflow::Auto)
        && ext.scroll_content_height > pb.height as usize;
    let scrolls_x = matches!(c.overflow_x, Overflow::Scroll | Overflow::Auto)
        && ext.scroll_content_width > pb.width as usize;
    scrolls_x || scrolls_y
}

/// Does any descendant of `id` qualify as its own focus stop? If so, a tab stop
/// on the enclosing scroll container would be redundant. A descendant counts
/// when it has an explicit `tabindex >= 0`, is intrinsically focusable
/// (control / tree), or is itself a scroll container.
fn has_focusable_descendant(dom: &TuiDom, id: NodeId) -> bool {
    let mut stack: Vec<NodeId> = dom.node(id).children().map(|c| c.id()).collect();
    while let Some(d) = stack.pop() {
        let node = dom.node(d);
        let explicit_nonneg = node
            .get_attribute("tabindex")
            .and_then(|s| s.parse::<i32>().ok())
            .is_some_and(|t| t >= 0);
        let enabled = !node.has_attribute("disabled");
        if (enabled && (explicit_nonneg || intrinsic_tag_focusable(dom, d)))
            || is_scroll_container(dom, d)
        {
            return true;
        }
        stack.extend(dom.node(d).children().map(|c| c.id()));
    }
    false
}

/// True iff the element is focusable at all — either via Tab
/// (tabindex >= 0) or programmatically (tabindex < 0).
pub fn is_focusable(dom: &TuiDom, id: NodeId) -> bool {
    tab_index(dom, id).is_some()
}

/// True iff the element participates in Tab navigation.
/// Excludes `tabindex < 0` (programmatic-only).
pub fn is_tab_focusable(dom: &TuiDom, id: NodeId) -> bool {
    tab_index(dom, id).is_some_and(|t| t >= 0)
}

/// Collect all tab-focusable elements in tab-navigation order.
///
/// Order:
/// 1. Elements with `tabindex > 0`, sorted by `tabindex` ascending,
///    ties broken by document order.
/// 2. Elements with `tabindex == 0`, in document order.
///
/// `tabindex < 0` elements are excluded (not Tab-reachable).
///
/// **Radio groups are a single tab stop.** Per HTML, a `name`-keyed
/// `<input type=radio>` group represents itself in sequential focus
/// navigation by exactly ONE radio: the currently-`checked` member,
/// or — if none is checked — the first member in document order.
/// The other group members are reachable only via the in-group
/// arrow-key navigation provided by the toggle builtin. This pass
/// runs after tabindex ordering: collect everything tabindex-wise
/// first, then collapse each radio group down to its representative.
pub fn focusable_elements(dom: &TuiDom) -> Vec<NodeId> {
    let mut positive: Vec<(i32, usize, NodeId)> = Vec::new();
    let mut zero: Vec<(usize, NodeId)> = Vec::new();
    let mut order: usize = 0;
    collect(dom, dom.root(), &mut positive, &mut zero, &mut order);

    positive.sort_by_key(|(ti, ord, _)| (*ti, *ord));
    zero.sort_by_key(|(ord, _)| *ord);

    let raw: Vec<NodeId> = positive
        .into_iter()
        .map(|(_, _, id)| id)
        .chain(zero.into_iter().map(|(_, id)| id))
        .collect();

    dedupe_radio_groups(dom, raw)
}

/// Collapse each `name`-keyed radio group in `list` to a single
/// tab stop: the checked member if any, otherwise the first member
/// in `list`'s order. Non-radio elements and radios without a
/// `name` attribute pass through unchanged.
fn dedupe_radio_groups(dom: &TuiDom, list: Vec<NodeId>) -> Vec<NodeId> {
    use std::collections::HashMap;

    // First pass: for each named radio group, pick the
    // representative (checked > first).
    let mut rep_for: HashMap<String, NodeId> = HashMap::new();
    for &id in &list {
        let Some(name) = named_radio_name(dom, id) else {
            continue;
        };
        let is_checked = dom.node(id).has_attribute("checked");
        match rep_for.get(&name) {
            None => {
                rep_for.insert(name, id);
            }
            Some(&existing) if is_checked && !dom.node(existing).has_attribute("checked") => {
                // Existing rep is not checked, current one is —
                // promote current.
                rep_for.insert(name, id);
            }
            _ => {}
        }
    }

    // Second pass: filter — keep non-radios and named-radio
    // representatives; drop other group members.
    list.into_iter()
        .filter(|&id| match named_radio_name(dom, id) {
            Some(name) => rep_for.get(&name) == Some(&id),
            None => true,
        })
        .collect()
}

/// Return the radio's `name` attribute iff `id` is an
/// `<input type=radio>` with a non-empty `name`. Returns `None`
/// for non-radios and for nameless radios (which don't form a
/// group and so don't get deduped).
fn named_radio_name(dom: &TuiDom, id: NodeId) -> Option<String> {
    let node = dom.node(id);
    if node.tag_name() != Some("input") {
        return None;
    }
    if node.get_attribute("type") != Some("radio") {
        return None;
    }
    let name = node.get_attribute("name")?;
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

fn collect(
    dom: &TuiDom,
    id: NodeId,
    positive: &mut Vec<(i32, usize, NodeId)>,
    zero: &mut Vec<(usize, NodeId)>,
    order: &mut usize,
) {
    *order += 1;
    let current_order = *order;
    if let Some(t) = tab_index(dom, id) {
        if t > 0 {
            positive.push((t, current_order, id));
        } else if t == 0 {
            zero.push((current_order, id));
        }
        // t < 0: skip — not tab-reachable.
    }
    for child in dom.node(id).child_nodes() {
        collect(dom, child.id(), positive, zero, order);
    }
}

/// Tab: move focus to the next tab-focusable element. Wraps
/// around at the end of the list.
///
/// If no element is currently focused, focus moves to the first.
/// If the list is empty, this is a no-op.
pub fn focus_next(dom: &mut TuiDom) {
    step_focus(dom, 1);
}

/// Shift+Tab: move focus to the previous tab-focusable element.
/// Wraps. Empty list → no-op.
pub fn focus_prev(dom: &mut TuiDom) {
    step_focus(dom, -1);
}

fn step_focus(dom: &mut TuiDom, direction: i32) {
    let list = focusable_elements(dom);
    if list.is_empty() {
        return;
    }

    let target = match dom
        .focused()
        .and_then(|cur| list.iter().position(|&e| e == cur))
    {
        Some(i) => {
            // Modular arithmetic on i32 to handle the wrap cleanly
            // in both directions.
            let len = list.len() as i32;
            let next = (i as i32 + direction).rem_euclid(len);
            list[next as usize]
        }
        None => {
            if direction > 0 {
                list[0]
            } else {
                list[list.len() - 1]
            }
        }
    };

    super::focus_node(dom, Some(target));
}

#[cfg(test)]
mod focusable_tests {
    use super::*;
    use crate::layout::{Overflow, Size};
    use crate::render::{LayoutExt, Rect};
    use crate::style::{CascadeExt, Stylesheet, TuiStyle};
    use crate::{TuiDom, TuiNodeMutExt};

    /// A `<div>` 4 rows tall with `overflow-y: scroll`. `tall` controls whether
    /// its content overflows the box (→ a scroll container); `build` adds
    /// children before cascade.
    fn scroll_div(tall: bool, build: impl FnOnce(&mut TuiDom, NodeId)) -> (TuiDom, NodeId) {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let d = dom.create_element("div");
        dom.node_mut(d).set_inline_style(
            TuiStyle::new()
                .height(Size::Fixed(4))
                .overflow_y(Overflow::Scroll),
        );
        build(&mut dom, d);
        dom.append_child(root, d).unwrap();
        dom.cascade(&Stylesheet::new());
        dom.layout_dom(Rect::new(0, 0, 20, 10));
        if let Some(ext) = dom.node_mut(d).ext_mut() {
            ext.scroll_content_height = if tall { 50 } else { 2 };
        }
        (dom, d)
    }

    #[test]
    fn scrollable_div_is_focusable() {
        let (dom, d) = scroll_div(true, |_, _| {});
        assert!(
            is_focusable(&dom, d),
            "a scrollable overflow div is Tab-focusable"
        );
    }

    #[test]
    fn non_overflowing_overflow_div_is_not_focusable() {
        let (dom, d) = scroll_div(false, |_, _| {});
        assert!(
            !is_focusable(&dom, d),
            "content fits → no scrollbar → not focusable"
        );
    }

    #[test]
    fn plain_div_is_not_focusable() {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let d = dom.create_element("div");
        dom.append_child(root, d).unwrap();
        dom.cascade(&Stylesheet::new());
        dom.layout_dom(Rect::new(0, 0, 20, 10));
        assert!(
            !is_focusable(&dom, d),
            "a non-scrolling div is not focusable"
        );
    }

    #[test]
    fn scroll_container_with_focusable_child_is_not_a_redundant_stop() {
        let (dom, d) = scroll_div(true, |dom, parent| {
            let b = dom.create_element("button");
            dom.append_child(parent, b).unwrap();
        });
        assert!(
            !is_focusable(&dom, d),
            "the button is the stop; the enclosing scroller must not also be one"
        );
    }
}
