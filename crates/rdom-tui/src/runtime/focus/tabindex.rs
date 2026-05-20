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
fn is_implicit_focusable(dom: &TuiDom, id: NodeId) -> bool {
    let node = dom.node(id);
    let Some(tag) = node.tag_name() else {
        return false;
    };
    match tag {
        // `<input type="hidden">` is NOT focusable. Every other
        // input type is.
        "input" => !matches!(node.get_attribute("type"), Some("hidden")),
        "button" | "textarea" | "select" => true,
        // `<summary>` is the focus target of a `<details>`
        // disclosure widget, not `<details>` itself (per the
        // HTML living standard). A `<summary>` without a parent
        // `<details>` has no defined behavior — we still treat
        // it as focusable.
        "summary" => true,
        // Anchors + image map areas need `href` to be focusable.
        "a" | "area" => node.has_attribute("href"),
        _ => false,
    }
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
pub fn focusable_elements(dom: &TuiDom) -> Vec<NodeId> {
    let mut positive: Vec<(i32, usize, NodeId)> = Vec::new();
    let mut zero: Vec<(usize, NodeId)> = Vec::new();
    let mut order: usize = 0;
    collect(dom, dom.root(), &mut positive, &mut zero, &mut order);

    positive.sort_by_key(|(ti, ord, _)| (*ti, *ord));
    zero.sort_by_key(|(ord, _)| *ord);

    positive
        .into_iter()
        .map(|(_, _, id)| id)
        .chain(zero.into_iter().map(|(_, id)| id))
        .collect()
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
