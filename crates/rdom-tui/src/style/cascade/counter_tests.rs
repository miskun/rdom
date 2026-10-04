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
    restyle_vars(&mut dom, &sheets, registry, &[first, div, last]);
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
    restyle_vars(&mut dom, &sheets, registry, &[div, last]);
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
    restyle_vars(&mut dom, &sheets, registry, &[div, last]);
    assert_eq!(before(&dom, last).as_deref(), Some("1. "));
}

/// `C2G-RESTYLE-WALK` — the UA sheet uses counters (`ol` / `ul` reset
/// `list-item`, `li` increments it), so every sheet set does. A restyle
/// of a leaf that creates, increments and reads no counter cannot change
/// any counter's value (CSS Lists 3 §3.1), so it must not walk the tree
/// before it to rebuild counter state: 50 lists of 100 items, the leaf a
/// `span` in the last item — the restyle visits the leaf, not 10 000
/// elements.
#[test]
fn restyle_of_a_counter_free_leaf_visits_its_subtree_only() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let mut leaf = root;
    for _ in 0..50 {
        let ol = dom.create_element("ol");
        dom.append_child(root, ol).unwrap();
        for _ in 0..100 {
            let li = dom.create_element("li");
            dom.append_child(ol, li).unwrap();
            leaf = dom.create_element("span");
            dom.append_child(li, leaf).unwrap();
        }
    }
    let sheet = sheet(".x { color: red }");
    dom.cascade(&sheet);
    dom.node_mut(leaf)
        .ext_mut()
        .unwrap()
        .set_inline_style(TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    let sheets = [&sheet];
    let registry = Rc::new(PropertyRegistry::new(&sheets));
    walk::probe::take();
    restyle_vars(&mut dom, &sheets, registry, &[leaf]);
    let visits = walk::probe::take();
    assert_eq!(computed_of(&dom, leaf).fg, Color::Rgb(255, 0, 0));
    assert!(visits <= 4, "visited {visits} nodes for one leaf");
}

/// `C2G-RESTYLE-WALK` — a subtree root inside another root is cascaded
/// with it (DOM §4.2.1 tree order: the outer root comes first), not
/// again on its own — whatever order the roots are given in. With and
/// without the UA sheet (whose counters take a different path).
#[test]
fn nested_subtree_roots_are_cascaded_once() {
    for ua in [false, true] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        // The inner element is created first: its id sorts first.
        let inner = dom.create_element("p");
        let outer = dom.create_element("div");
        dom.append_child(root, outer).unwrap();
        dom.append_child(outer, inner).unwrap();
        let mut sheet = if ua {
            Stylesheet::new()
        } else {
            Stylesheet::bare()
        };
        sheet.add_rule("p", TuiStyle::new().bold(true)).unwrap();
        dom.cascade(&sheet);
        match_probe::take();
        dom.cascade_subtrees(&sheet, &[outer]);
        let once = match_probe::take();
        dom.cascade_subtrees(&sheet, &[inner, outer]);
        let both = match_probe::take();
        assert_eq!(both, once, "ua: {ua}");
    }
}

/// `C2G-RESTYLE-WALK` — CSS Lists 3 §3.1: a subtree root whose counter
/// ops change moves the counters of everything after it. A class change
/// makes the first `h2::before` count 10; the third `h2` — not a root —
/// reads 12 after the partial cascade, as after a full one.
#[test]
fn changed_counter_ops_renumber_later_elements() {
    let css = format!("{SECTIONS} .big::before {{ counter-increment: sec 10 }}");
    let mut dom = TuiDom::new();
    let root = main_el(&mut dom);
    let ids: Vec<NodeId> = (0..3).map(|_| h2(&mut dom, root)).collect();
    let sheet = sheet(&css);
    dom.cascade(&sheet);
    assert_eq!(before(&dom, ids[2]).as_deref(), Some("3. "));
    dom.set_attribute(ids[0], "class", "big").unwrap();
    dom.cascade_subtrees(&sheet, &[ids[0]]);
    assert_eq!(before(&dom, ids[0]).as_deref(), Some("10. "));
    assert_eq!(before(&dom, ids[2]).as_deref(), Some("12. "));
}

/// CSS Lists 3 §3.1 and CSS Style Attributes §3: counter ops in `style`
/// attributes take part in counters like the sheet's. With a bare sheet
/// (no counter rule at all), a partial cascade of the second element
/// still continues the counter its preceding sibling and parent set —
/// the partial walk used to look only at the sheets, cascade the root
/// alone and number it from scratch.
#[test]
fn inline_counter_ops_take_part_in_partial_cascades() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let list = dom.create_element("div");
    dom.append_child(root, list).unwrap();
    dom.set_attribute(list, "style", "counter-reset: c 5")
        .unwrap();
    let items: Vec<NodeId> = (0..2)
        .map(|_| {
            let id = dom.create_element("div");
            dom.append_child(list, id).unwrap();
            dom.set_attribute(id, "style", "counter-increment: c; content: counter(c)")
                .unwrap();
            id
        })
        .collect();
    assert!(crate::seed_inline_styles(&mut dom).is_empty());
    let content = |dom: &TuiDom, id: NodeId| {
        dom.node(id)
            .ext()
            .and_then(|e| e.computed.as_ref())
            .and_then(|c| c.content.clone())
    };
    let sheet = Stylesheet::bare();
    dom.cascade(&sheet);
    assert_eq!(content(&dom, items[1]).as_deref(), Some("7"));
    dom.cascade_subtrees(&sheet, &[items[1]]);
    assert_eq!(content(&dom, items[1]).as_deref(), Some("7"));
}
