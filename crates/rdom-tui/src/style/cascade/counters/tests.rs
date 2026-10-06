//! `CounterState` unit tests: scoping (CSS Lists 3 §4.4–§4.5), the
//! reset / increment / set order, the implicit `list-item` increment
//! (§4.6) and the reversed counter's initial value (§4.2).

use super::*;
use crate::TuiDom;

fn op(name: &str, value: i32) -> CounterOp {
    CounterOp::new(name, value)
}

/// A box style holding `reset`, `increment` and `set`.
fn ops(reset: &[CounterOp], increment: &[CounterOp], set: &[CounterOp]) -> ComputedStyle {
    let mut style = ComputedStyle::initial();
    style.counter_reset = reset.to_vec();
    style.counter_increment = increment.to_vec();
    style.counter_set = set.to_vec();
    style
}

/// `ol > li` numbering with a nested list: the inner reset is
/// scoped to the inner `<ol>`, the outer count resumes after it,
/// and a following sibling `<ol>` starts over.
#[test]
fn nested_lists_scope_and_resume() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let ol = dom.create_element("ol");
    let li1 = dom.create_element("li");
    let inner = dom.create_element("ol");
    let inner_li = dom.create_element("li");
    let li2 = dom.create_element("li");
    let ol2 = dom.create_element("ol");
    dom.append_child(root, ol).unwrap();
    dom.append_child(ol, li1).unwrap();
    dom.append_child(li1, inner).unwrap();
    dom.append_child(inner, inner_li).unwrap();
    dom.append_child(ol, li2).unwrap();
    dom.append_child(root, ol2).unwrap();

    let reset = ops(&[op("list-item", 0)], &[], &[]);
    let inc = ops(&[], &[op("list-item", 1)], &[]);
    let mut st = CounterState::default();
    st.enter(Some(root), Owner::element(ol), &reset);
    st.enter(Some(ol), Owner::element(li1), &inc);
    assert_eq!(st.value("list-item"), 1);
    st.enter(Some(li1), Owner::element(inner), &reset);
    st.enter(Some(inner), Owner::element(inner_li), &inc);
    assert_eq!(st.value("list-item"), 1);
    assert_eq!(st.values("list-item"), vec![1, 1], "nested, not replaced");
    st.exit(inner_li);
    st.exit(inner); // inner <ol> closes: its own instance survives until li1 exits
    assert_eq!(
        st.value("list-item"),
        1,
        "still in the inner <ol>'s scope (a following sibling would see it)"
    );
    st.exit(li1); // leaving <li> 1 drops the inner <ol>'s instance
    assert_eq!(st.value("list-item"), 1, "outer count resumes");
    st.enter(Some(ol), Owner::element(li2), &inc);
    assert_eq!(st.value("list-item"), 2);
    st.exit(li2);
    st.exit(ol);
    // <ol> 2 is a following sibling of <ol> 1: its reset replaces the
    // first one's counter (§4.5), it does not nest in it.
    st.enter(Some(root), Owner::element(ol2), &reset);
    assert_eq!(st.values("list-item"), vec![0]);
}

#[test]
fn increment_without_reset_creates_the_counter_and_missing_reads_zero() {
    let mut dom: Dom = Dom::new();
    let a = dom.create_element("a");
    let mut st = CounterState::default();
    assert_eq!(st.value("x"), 0);
    assert_eq!(st.values("x"), vec![0]);
    st.enter(None, Owner::element(a), &ops(&[], &[op("x", 5)], &[]));
    assert_eq!(st.value("x"), 5);
    st.enter(None, Owner::element(a), &ops(&[], &[op("x", -2)], &[]));
    assert_eq!(st.value("x"), 3);
}

/// §4.4: reset, then increment, then set — on one box.
#[test]
fn a_box_resets_then_increments_then_sets() {
    let mut dom: Dom = Dom::new();
    let a = dom.create_element("a");
    let mut st = CounterState::default();
    st.enter(
        None,
        Owner::element(a),
        &ops(&[op("c", 3)], &[op("c", 1)], &[]),
    );
    assert_eq!(st.value("c"), 4);
    st.enter(
        None,
        Owner::element(a),
        &ops(&[], &[op("c", 1)], &[op("c", 9)]),
    );
    assert_eq!(st.value("c"), 9);
    st.enter(None, Owner::element(a), &ops(&[], &[], &[op("d", 2)]));
    assert_eq!(
        st.value("d"),
        2,
        "setting a counter not in scope instantiates it"
    );
}

/// §4.6: a list item increments `list-item` by one — by minus one on a
/// reversed counter — unless its `counter-increment` names `list-item`.
#[test]
fn list_items_increment_implicitly() {
    let mut dom: Dom = Dom::new();
    let a = dom.create_element("li");
    let mut item = ComputedStyle::initial();
    item.list_item = true;
    let mut st = CounterState::default();
    st.enter(None, Owner::element(a), &item);
    st.enter(None, Owner::element(a), &item);
    assert_eq!(st.value("list-item"), 2);
    let mut zero = item.clone();
    zero.counter_increment = vec![op("list-item", 0)];
    st.enter(None, Owner::element(a), &zero);
    assert_eq!(st.value("list-item"), 2);
    let reversed = ops(&[CounterOp::reversed("list-item", Some(5))], &[], &[]);
    st.enter(None, Owner::element(a), &reversed);
    st.enter(None, Owner::element(a), &item);
    assert_eq!(st.value("list-item"), 4);
}

/// §4.2's algorithm: the negated increments, plus the last non-zero one;
/// a set stops it, adding its value.
#[test]
fn the_reversed_initial_value_follows_the_algorithm() {
    let t = |boxes: &[(i64, Option<i32>)]| {
        let trace = reversed::Trace::for_test(boxes);
        trace.initial_value_for_test()
    };
    assert_eq!(t(&[(-1, None), (-1, None), (-1, None)]), 4);
    assert_eq!(t(&[(-1, None), (-1, Some(5)), (-1, None)]), 7);
    assert_eq!(t(&[(-2, None), (0, None)]), 4);
    assert_eq!(t(&[]), 0);
    assert_eq!(t(&[(i64::from(i32::MIN), None); 3]), i32::MAX);
}

/// The scan reads the boxes in scope as last cascaded: an `<ol>`'s
/// reversed counter counts its cascaded items.
#[test]
fn the_scan_counts_the_cascaded_items_in_scope() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let ol = dom.create_element("ol");
    dom.append_child(root, ol).unwrap();
    for _ in 0..3 {
        let li = dom.create_element("li");
        dom.append_child(ol, li).unwrap();
    }
    let sheet = rdom_css::from_css_strict("").unwrap();
    use crate::CascadeExt;
    dom.cascade(&sheet);
    let reset = ops(&[CounterOp::reversed("list-item", None)], &[], &[]);
    assert_eq!(
        reversed::initial_value(&dom, Owner::element(ol), &reset, "list-item"),
        4
    );
}
