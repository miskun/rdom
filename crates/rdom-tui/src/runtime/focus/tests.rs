//! Focus management tests — tabindex ordering, Tab/Shift-Tab
//! navigation, programmatic focus_node, event order, click-to-focus.

use rdom_core::{ListenerOptions, NodeId};
use std::cell::RefCell;
use std::rc::Rc;

use crate::TuiDom;
use crate::runtime::focus::{
    focus_node, nearest_focusable_ancestor,
    tabindex::{
        focus_next, focus_prev, focusable_elements, is_focusable, is_tab_focusable, tab_index,
    },
};

// ── Tabindex parsing / focusability ─────────────────────────────────

#[test]
fn tabindex_absent_is_none() {
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    assert_eq!(tab_index(&dom, div), None);
    assert!(!is_focusable(&dom, div));
    assert!(!is_tab_focusable(&dom, div));
}

#[test]
fn tabindex_zero_is_tab_focusable() {
    let mut dom: TuiDom = TuiDom::new();
    let el = dom.create_element("el");
    dom.set_attribute(el, "tabindex", "0").unwrap();
    assert_eq!(tab_index(&dom, el), Some(0));
    assert!(is_focusable(&dom, el));
    assert!(is_tab_focusable(&dom, el));
}

#[test]
fn tabindex_negative_focusable_but_not_tab_reachable() {
    let mut dom: TuiDom = TuiDom::new();
    let el = dom.create_element("el");
    dom.set_attribute(el, "tabindex", "-1").unwrap();
    assert!(is_focusable(&dom, el));
    assert!(!is_tab_focusable(&dom, el));
}

#[test]
fn tabindex_garbage_is_none() {
    let mut dom: TuiDom = TuiDom::new();
    let el = dom.create_element("el");
    dom.set_attribute(el, "tabindex", "not-a-number").unwrap();
    assert_eq!(tab_index(&dom, el), None);
    assert!(!is_focusable(&dom, el));
}

// ── focusable_elements ordering ─────────────────────────────────────

#[test]
fn focusable_elements_orders_positive_then_zero() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    let c = dom.create_element("c");
    let d = dom.create_element("d");
    dom.set_attribute(a, "tabindex", "0").unwrap(); // in DOM order among zeros
    dom.set_attribute(b, "tabindex", "2").unwrap(); // second in positives
    dom.set_attribute(c, "tabindex", "1").unwrap(); // first in positives
    dom.set_attribute(d, "tabindex", "0").unwrap(); // second in zeros
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    dom.append_child(root, c).unwrap();
    dom.append_child(root, d).unwrap();

    let order = focusable_elements(&dom);
    // c (ti=1), b (ti=2), a (ti=0), d (ti=0)
    assert_eq!(order, vec![c, b, a, d]);
}

#[test]
fn focusable_elements_skips_negative_tabindex() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    let c = dom.create_element("c");
    dom.set_attribute(a, "tabindex", "0").unwrap();
    dom.set_attribute(b, "tabindex", "-1").unwrap();
    dom.set_attribute(c, "tabindex", "0").unwrap();
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    dom.append_child(root, c).unwrap();

    assert_eq!(focusable_elements(&dom), vec![a, c]);
}

#[test]
fn focusable_elements_same_positive_tabindex_uses_doc_order() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.set_attribute(a, "tabindex", "1").unwrap();
    dom.set_attribute(b, "tabindex", "1").unwrap();
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    assert_eq!(focusable_elements(&dom), vec![a, b]);
}

// ── focus_next / focus_prev navigation ──────────────────────────────

#[test]
fn focus_next_from_none_goes_to_first() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.set_attribute(a, "tabindex", "0").unwrap();
    dom.set_attribute(b, "tabindex", "0").unwrap();
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();

    focus_next(&mut dom);
    assert_eq!(dom.focused(), Some(a));
}

#[test]
fn focus_next_advances_and_wraps() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    let c = dom.create_element("c");
    for e in [a, b, c] {
        dom.set_attribute(e, "tabindex", "0").unwrap();
        dom.append_child(root, e).unwrap();
    }

    focus_next(&mut dom); // None → a
    assert_eq!(dom.focused(), Some(a));
    focus_next(&mut dom);
    assert_eq!(dom.focused(), Some(b));
    focus_next(&mut dom);
    assert_eq!(dom.focused(), Some(c));
    focus_next(&mut dom); // wrap
    assert_eq!(dom.focused(), Some(a));
}

#[test]
fn focus_prev_from_none_goes_to_last() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.set_attribute(a, "tabindex", "0").unwrap();
    dom.set_attribute(b, "tabindex", "0").unwrap();
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();

    focus_prev(&mut dom);
    assert_eq!(dom.focused(), Some(b));
}

#[test]
fn focus_prev_retreats_and_wraps() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.set_attribute(a, "tabindex", "0").unwrap();
    dom.set_attribute(b, "tabindex", "0").unwrap();
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();

    dom.set_focused(Some(a));
    focus_prev(&mut dom); // wrap to b
    assert_eq!(dom.focused(), Some(b));
    focus_prev(&mut dom);
    assert_eq!(dom.focused(), Some(a));
}

#[test]
fn focus_next_empty_is_noop() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div"); // no tabindex
    dom.append_child(root, div).unwrap();
    focus_next(&mut dom);
    assert_eq!(dom.focused(), None);
}

// ── focus_node event ordering ───────────────────────────────────────

fn record_events(
    dom: &mut TuiDom,
    node: NodeId,
    types: &[&'static str],
) -> Rc<RefCell<Vec<(NodeId, String)>>> {
    let log = Rc::new(RefCell::new(Vec::new()));
    for &t in types {
        let log = log.clone();
        let ty = t.to_string();
        dom.add_event_listener(node, t, ListenerOptions::default(), move |ctx| {
            log.borrow_mut()
                .push((ctx.event.current_target.unwrap_or(node), ty.clone()));
        })
        .unwrap();
    }
    log
}

#[test]
fn focus_node_fires_blur_focusout_then_focus_focusin() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();

    let la = record_events(&mut dom, a, &["blur", "focus", "focusin", "focusout"]);
    let lb = record_events(&mut dom, b, &["blur", "focus", "focusin", "focusout"]);

    dom.set_focused(Some(a));
    la.borrow_mut().clear(); // ignore noise from the initial state

    focus_node(&mut dom, Some(b));

    let ev_a: Vec<String> = la.borrow().iter().map(|(_, s)| s.clone()).collect();
    let ev_b: Vec<String> = lb.borrow().iter().map(|(_, s)| s.clone()).collect();
    assert_eq!(ev_a, vec!["blur", "focusout"]);
    assert_eq!(ev_b, vec!["focus", "focusin"]);
    assert_eq!(dom.focused(), Some(b));
}

#[test]
fn focus_node_idempotent_same_target() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    dom.append_child(root, a).unwrap();
    dom.set_focused(Some(a));

    let log = record_events(&mut dom, a, &["blur", "focus", "focusin", "focusout"]);
    focus_node(&mut dom, Some(a));
    assert!(log.borrow().is_empty(), "same-target focus fires no events");
}

#[test]
fn focus_node_none_fires_blur_and_focusout_only() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    dom.append_child(root, a).unwrap();
    dom.set_focused(Some(a));

    let log = record_events(&mut dom, a, &["blur", "focus", "focusin", "focusout"]);
    focus_node(&mut dom, None);
    let ev: Vec<String> = log.borrow().iter().map(|(_, s)| s.clone()).collect();
    assert_eq!(ev, vec!["blur", "focusout"]);
    assert_eq!(dom.focused(), None);
}

#[test]
fn focus_events_correct_bubbling() {
    // focus / blur must NOT bubble; focusin / focusout MUST.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent");
    let child = dom.create_element("child");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let parent_log = record_events(&mut dom, parent, &["blur", "focus", "focusin", "focusout"]);

    focus_node(&mut dom, Some(child));
    let events: Vec<String> = parent_log.borrow().iter().map(|(_, s)| s.clone()).collect();
    // Parent should see focusin (bubbles) but NOT focus (non-bubbling).
    assert!(events.contains(&"focusin".to_string()));
    assert!(!events.contains(&"focus".to_string()));
}

// ── nearest_focusable_ancestor ──────────────────────────────────────

#[test]
fn nearest_focusable_ancestor_finds_closest() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let middle = dom.create_element("middle");
    let inner = dom.create_element("inner");
    dom.set_attribute(outer, "tabindex", "0").unwrap();
    dom.set_attribute(middle, "tabindex", "0").unwrap();
    dom.append_child(middle, inner).unwrap();
    dom.append_child(outer, middle).unwrap();
    dom.append_child(root, outer).unwrap();

    // inner is not focusable; walk up → middle (closer than outer).
    assert_eq!(nearest_focusable_ancestor(&dom, inner), Some(middle));
}

#[test]
fn nearest_focusable_ancestor_returns_self_when_focusable() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("el");
    dom.set_attribute(el, "tabindex", "0").unwrap();
    dom.append_child(root, el).unwrap();

    assert_eq!(nearest_focusable_ancestor(&dom, el), Some(el));
}

#[test]
fn nearest_focusable_ancestor_none_when_no_focusable_chain() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("el");
    dom.append_child(root, el).unwrap();
    assert_eq!(nearest_focusable_ancestor(&dom, el), None);
}

// ── Implicit focusability (Phase C.1) ───────────────────────────────

#[test]
fn implicit_focusable_button_without_tabindex() {
    let mut dom: TuiDom = TuiDom::new();
    let b = dom.create_element("button");
    assert_eq!(tab_index(&dom, b), Some(0));
    assert!(is_focusable(&dom, b));
    assert!(is_tab_focusable(&dom, b));
}

#[test]
fn implicit_focusable_textarea_summary_select() {
    // `<summary>` is the focus target of a `<details>` widget,
    // not `<details>` itself — per HTML living standard.
    for tag in ["textarea", "summary", "select"] {
        let mut dom: TuiDom = TuiDom::new();
        let el = dom.create_element(tag);
        assert_eq!(
            tab_index(&dom, el),
            Some(0),
            "<{tag}> should be implicit-focusable"
        );
    }
}

#[test]
fn details_itself_is_not_focusable() {
    // Regression guard: `<details>` does NOT take focus; its
    // child `<summary>` does.
    let mut dom: TuiDom = TuiDom::new();
    let d = dom.create_element("details");
    assert_eq!(tab_index(&dom, d), None);
}

#[test]
fn implicit_focusable_input_except_hidden() {
    // <input> without type — implicit "text" — focusable.
    let mut dom: TuiDom = TuiDom::new();
    let plain = dom.create_element("input");
    assert_eq!(tab_index(&dom, plain), Some(0));

    // <input type="text"> — focusable.
    let mut dom2: TuiDom = TuiDom::new();
    let text = dom2.create_element("input");
    dom2.set_attribute(text, "type", "text").unwrap();
    assert_eq!(tab_index(&dom2, text), Some(0));

    // <input type="checkbox"> — focusable.
    let mut dom3: TuiDom = TuiDom::new();
    let cb = dom3.create_element("input");
    dom3.set_attribute(cb, "type", "checkbox").unwrap();
    assert_eq!(tab_index(&dom3, cb), Some(0));

    // <input type="hidden"> — NOT focusable.
    let mut dom4: TuiDom = TuiDom::new();
    let hidden = dom4.create_element("input");
    dom4.set_attribute(hidden, "type", "hidden").unwrap();
    assert_eq!(tab_index(&dom4, hidden), None);
    assert!(!is_focusable(&dom4, hidden));
}

#[test]
fn implicit_focusable_anchor_requires_href() {
    // <a> without href — NOT focusable.
    let mut dom: TuiDom = TuiDom::new();
    let a = dom.create_element("a");
    assert_eq!(tab_index(&dom, a), None);

    // <a href="..."> — focusable.
    let mut dom2: TuiDom = TuiDom::new();
    let a2 = dom2.create_element("a");
    dom2.set_attribute(a2, "href", "/x").unwrap();
    assert_eq!(tab_index(&dom2, a2), Some(0));
}

#[test]
fn disabled_overrides_focusability() {
    // `disabled` is a blanket override — button with disabled
    // is NOT focusable even though `<button>` is implicit.
    let mut dom: TuiDom = TuiDom::new();
    let b = dom.create_element("button");
    dom.set_attribute(b, "disabled", "").unwrap();
    assert_eq!(tab_index(&dom, b), None);
    assert!(!is_focusable(&dom, b));
}

#[test]
fn disabled_overrides_explicit_tabindex() {
    // Explicit tabindex=0 + disabled — still not focusable.
    let mut dom: TuiDom = TuiDom::new();
    let b = dom.create_element("div");
    dom.set_attribute(b, "tabindex", "0").unwrap();
    dom.set_attribute(b, "disabled", "").unwrap();
    assert_eq!(tab_index(&dom, b), None);
}

#[test]
fn explicit_tabindex_wins_over_implicit() {
    // <button tabindex="-1"> — programmatic only. is_focusable
    // still true but is_tab_focusable should be false.
    let mut dom: TuiDom = TuiDom::new();
    let b = dom.create_element("button");
    dom.set_attribute(b, "tabindex", "-1").unwrap();
    assert_eq!(tab_index(&dom, b), Some(-1));
    assert!(is_focusable(&dom, b));
    assert!(!is_tab_focusable(&dom, b));
}

#[test]
fn tab_navigation_visits_implicit_focusable_elements() {
    // Realistic HTML: form with <button>, <input>, <a href>.
    // Tab should visit all three without any `tabindex` attrs.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let b = dom.create_element("button");
    let i = dom.create_element("input");
    let a = dom.create_element("a");
    dom.set_attribute(a, "href", "/x").unwrap();
    dom.append_child(root, b).unwrap();
    dom.append_child(root, i).unwrap();
    dom.append_child(root, a).unwrap();

    let order = focusable_elements(&dom);
    assert_eq!(order, vec![b, i, a]);
}

#[test]
fn non_focusable_tags_stay_non_focusable() {
    // Just making sure we didn't accidentally make every element
    // focusable.
    for tag in ["div", "span", "p", "section", "h1", "code"] {
        let mut dom: TuiDom = TuiDom::new();
        let el = dom.create_element(tag);
        assert_eq!(
            tab_index(&dom, el),
            None,
            "<{tag}> should not be focusable without tabindex"
        );
    }
}

// ── Radio-group single tab stop ──────────────────────────────────────
// Per HTML, a `name`-keyed `<input type=radio>` group is one tab stop.
// Tab moves focus IN/OUT of the group as a unit; only the checked
// radio (or the first if none is checked) is in the sequential focus
// chain. Arrow-key navigation among the other group members is handled
// by the toggle builtin; those non-checked radios must NOT show up in
// `focusable_elements`.

fn make_radio(dom: &mut TuiDom, name: &str, id: &str, checked: bool) -> NodeId {
    let r = dom.create_element("input");
    dom.set_attribute(r, "type", "radio").unwrap();
    dom.set_attribute(r, "name", name).unwrap();
    dom.set_attribute(r, "id", id).unwrap();
    if checked {
        dom.set_attribute(r, "checked", "").unwrap();
    }
    r
}

#[test]
fn radio_group_with_checked_member_excludes_unchecked_from_tab_list() {
    // Three radios with `name="g"`, second is checked. Tab list
    // should include ONLY the second radio (one tab stop for the
    // whole group).
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let r1 = make_radio(&mut dom, "g", "r1", false);
    let r2 = make_radio(&mut dom, "g", "r2", true);
    let r3 = make_radio(&mut dom, "g", "r3", false);
    for r in [r1, r2, r3] {
        dom.append_child(root, r).unwrap();
    }
    let list = focusable_elements(&dom);
    assert_eq!(
        list,
        vec![r2],
        "tab list should contain only the checked radio"
    );
}

#[test]
fn radio_group_without_checked_member_keeps_first_as_tab_stop() {
    // No radio checked → the FIRST in document order represents
    // the group in the tab list.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let r1 = make_radio(&mut dom, "g", "r1", false);
    let r2 = make_radio(&mut dom, "g", "r2", false);
    for r in [r1, r2] {
        dom.append_child(root, r).unwrap();
    }
    let list = focusable_elements(&dom);
    assert_eq!(list, vec![r1], "tab list should contain only first radio");
}

#[test]
fn radio_without_name_is_not_in_a_group() {
    // A radio with no `name` (or empty name) forms its own
    // single-element group — it's still tab-reachable on its own
    // because there's nothing for it to dedupe against.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let r1 = make_radio(&mut dom, "", "r1", false);
    let r2 = dom.create_element("input");
    dom.set_attribute(r2, "type", "radio").unwrap();
    dom.set_attribute(r2, "id", "r2").unwrap();
    // r2 has no `name` attribute at all
    dom.append_child(root, r1).unwrap();
    dom.append_child(root, r2).unwrap();
    let list = focusable_elements(&dom);
    assert_eq!(list, vec![r1, r2], "nameless radios stand on their own");
}

#[test]
fn distinct_radio_groups_each_contribute_one_tab_stop() {
    // Two groups with different `name`s — each contributes its own
    // single tab stop, so the list is 2 long.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let g1a = make_radio(&mut dom, "g1", "g1a", true);
    let g1b = make_radio(&mut dom, "g1", "g1b", false);
    let g2a = make_radio(&mut dom, "g2", "g2a", false);
    let g2b = make_radio(&mut dom, "g2", "g2b", true);
    for r in [g1a, g1b, g2a, g2b] {
        dom.append_child(root, r).unwrap();
    }
    let list = focusable_elements(&dom);
    assert_eq!(list, vec![g1a, g2b], "one tab stop per distinct group");
}

#[test]
fn radio_group_dedupe_does_not_affect_other_focusables() {
    // A button before, a radio group, a button after — the buttons
    // remain in the tab list unchanged.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let b1 = dom.create_element("button");
    let r1 = make_radio(&mut dom, "g", "r1", false);
    let r2 = make_radio(&mut dom, "g", "r2", true);
    let b2 = dom.create_element("button");
    dom.append_child(root, b1).unwrap();
    dom.append_child(root, r1).unwrap();
    dom.append_child(root, r2).unwrap();
    dom.append_child(root, b2).unwrap();
    let list = focusable_elements(&dom);
    assert_eq!(list, vec![b1, r2, b2]);
}
