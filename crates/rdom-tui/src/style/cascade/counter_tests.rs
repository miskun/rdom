//! Counters across partial cascades: the elements a partial walk does
//! not recompute — those between the roots of `cascade_subtrees` and
//! the subtrees `restyle_vars` keeps — must contribute every counter op
//! they hold, their `::before` / `::after` boxes' included, in tree
//! order: element, `::before`, children, `::after` (CSS Lists 3 §3.1;
//! CSS Pseudo-Elements 4 §4: `::before` is the element's first child,
//! `::after` its last).

use super::*;
use crate::TuiDom;
use crate::style::{Color, TuiStyle};

const SECTIONS: &str = "main { counter-reset: sec }
     h2::before { counter-increment: sec; content: counter(sec) \". \" }
     .x { color: red }";

fn sheet(css: &str) -> Stylesheet {
    rdom_css::from_css_strict(css).expect("sheet parses without warnings")
}

/// A `<main>` (which resets `sec`) under the document root.
fn main_el(dom: &mut TuiDom) -> NodeId {
    let root = dom.root();
    let id = dom.create_element("main");
    dom.append_child(root, id).unwrap();
    id
}

fn h2(dom: &mut TuiDom, parent: NodeId) -> NodeId {
    let id = dom.create_element("h2");
    dom.append_child(parent, id).unwrap();
    id
}

fn before(dom: &TuiDom, id: NodeId) -> Option<String> {
    dom.node(id)
        .ext()
        .and_then(|e| e.computed_before.as_ref())
        .and_then(|b| b.content.clone())
}

/// Three `h2`s numbered by their `::before`; a class change on the
/// third re-cascades only it, and the two before it — replayed, not
/// recomputed — still count: "3. ", not "1. ".
#[test]
fn subtree_cascade_replays_pseudo_element_counter_ops_before_it() {
    let mut dom = TuiDom::new();
    let root = main_el(&mut dom);
    let ids: Vec<NodeId> = (0..3).map(|_| h2(&mut dom, root)).collect();
    let sheet = sheet(SECTIONS);
    dom.cascade(&sheet);
    assert_eq!(before(&dom, ids[2]).as_deref(), Some("3. "));
    dom.set_attribute(ids[2], "class", "x").unwrap();
    dom.cascade_subtrees(&sheet, &[ids[2]]);
    assert_eq!(before(&dom, ids[2]).as_deref(), Some("3. "));
}

/// `restyle_vars` keeps an element whose style comes out unchanged,
/// with its subtree; its own `::before` and its children's must still
/// count for a later root. Here the first `h2` is a kept root and the
/// `div` a kept root holding two more; the last `h2` changed (its
/// inline style) and reads 4.
#[test]
fn restyle_keeps_pseudo_element_counter_ops_of_kept_subtrees() {
    let mut dom = TuiDom::new();
    let root = main_el(&mut dom);
    let first = h2(&mut dom, root);
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let _inner = [h2(&mut dom, div), h2(&mut dom, div)];
    let last = h2(&mut dom, root);
    let sheet = sheet(SECTIONS);
    dom.cascade(&sheet);
    assert_eq!(before(&dom, last).as_deref(), Some("4. "));

    dom.node_mut(last)
        .ext_mut()
        .unwrap()
        .set_inline_style(TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    let sheets = [&sheet];
    let registry = Rc::new(PropertyRegistry::new(&sheets));
    restyle_vars(
        &mut dom,
        &sheets,
        registry,
        &[first, div, last],
        Viewport::default(),
    );
    assert_eq!(
        computed_of(&dom, last).fg,
        Color::Rgb(255, 0, 0),
        "the last h2 was restyled"
    );
    assert_eq!(before(&dom, last).as_deref(), Some("4. "));
}

/// A kept element's `::after` comes after its children: with
/// `div::after { counter-increment: sec }` the `div`'s own `::after`
/// counts once its children have, so the `h2` after the `div` reads 4
/// in a full cascade and in a restyle that keeps the `div`.
#[test]
fn restyle_replays_after_ops_after_the_children() {
    let css = format!("{SECTIONS} div::after {{ counter-increment: sec; content: '' }}");
    let mut dom = TuiDom::new();
    let root = main_el(&mut dom);
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let inner = [h2(&mut dom, div), h2(&mut dom, div)];
    let last = h2(&mut dom, root);
    let sheet = sheet(&css);
    dom.cascade(&sheet);
    assert_eq!(before(&dom, inner[1]).as_deref(), Some("2. "));
    assert_eq!(before(&dom, last).as_deref(), Some("4. "));

    dom.node_mut(last)
        .ext_mut()
        .unwrap()
        .set_inline_style(TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    let sheets = [&sheet];
    let registry = Rc::new(PropertyRegistry::new(&sheets));
    restyle_vars(
        &mut dom,
        &sheets,
        registry,
        &[div, last],
        Viewport::default(),
    );
    assert_eq!(before(&dom, last).as_deref(), Some("4. "));
}

/// Order within a kept element: its `::before` comes before its
/// children. `div::before { counter-reset: sec }` gives the `div`'s
/// children their own `sec` (scoped to the `div`), so the `h2` after the
/// `div` reads the outer `sec`: 1, in a full cascade and in a restyle
/// that keeps the `div` (a replay of `::before` after the children would
/// count them in the outer `sec` and read 3).
#[test]
fn restyle_replays_before_ops_before_the_children() {
    let css = format!("{SECTIONS} div::before {{ counter-reset: sec; content: '' }}");
    let mut dom = TuiDom::new();
    let root = main_el(&mut dom);
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let inner = [h2(&mut dom, div), h2(&mut dom, div)];
    let last = h2(&mut dom, root);
    let sheet = sheet(&css);
    dom.cascade(&sheet);
    assert_eq!(before(&dom, inner[1]).as_deref(), Some("2. "));
    assert_eq!(before(&dom, last).as_deref(), Some("1. "));

    dom.node_mut(last)
        .ext_mut()
        .unwrap()
        .set_inline_style(TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    let sheets = [&sheet];
    let registry = Rc::new(PropertyRegistry::new(&sheets));
    restyle_vars(
        &mut dom,
        &sheets,
        registry,
        &[div, last],
        Viewport::default(),
    );
    assert_eq!(before(&dom, last).as_deref(), Some("1. "));
}
