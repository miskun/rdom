//! DOM §4.2.6, the `ParentNode` / `ChildNode` convenience methods, step by
//! step (C13G-DOM-CONVENIENCE): each converts its nodes into one node
//! ("converting nodes into a node": strings become `Text`, several nodes a
//! `DocumentFragment` they are moved into) and inserts that node once —
//! so an invalid node fails before anything moves, and a node in the list
//! that is the reference (or the receiver) is handled as the spec's
//! viable-sibling steps say.

use crate::{Dom, DomError, NodeId, NodeOrString};

fn kids(dom: &Dom, parent: NodeId) -> Vec<NodeId> {
    dom.node(parent).child_nodes().map(|n| n.id()).collect()
}

/// `<div>` under the root with `n` `<span>` children: the div and them.
fn row(dom: &mut Dom, n: usize) -> (NodeId, Vec<NodeId>) {
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let spans = (0..n)
        .map(|_| {
            let s = dom.create_element("span");
            dom.append_child(div, s).unwrap();
            s
        })
        .collect();
    (div, spans)
}

// ── append() ───────────────────────────────────────────────────────

/// §4.2.6 `append()`: one insertion of the converted node — a node that
/// cannot go there (the parent itself, an inclusive ancestor) fails it
/// before any other node moves.
#[test]
fn append_inserts_nothing_when_a_node_is_invalid() {
    let mut dom: Dom = Dom::new();
    let (div, spans) = row(&mut dom, 1);
    let x = dom.create_element("i");
    let err = dom
        .node_mut(div)
        .append([x.into(), div.into()])
        .unwrap_err();
    assert_eq!(err, DomError::HierarchyRequest);
    assert_eq!(kids(&dom, div), spans);
    assert!(dom.node(x).parent_node().is_none());
}

/// §4.2.6: a `DocumentFragment` in the list gives its children, in place.
#[test]
fn append_takes_a_fragments_children_in_order() {
    let mut dom: Dom = Dom::new();
    let (div, _) = row(&mut dom, 0);
    let frag = dom.create_document_fragment();
    let (a, b, c) = (
        dom.create_element("a"),
        dom.create_element("b"),
        dom.create_element("c"),
    );
    dom.append_child(frag, a).unwrap();
    dom.append_child(frag, b).unwrap();
    dom.node_mut(div).append([frag.into(), c.into()]).unwrap();
    assert_eq!(kids(&dom, div), [a, b, c]);
}

// ── prepend() ──────────────────────────────────────────────────────

/// §4.2.6 `prepend()`: the nodes are converted first, then inserted before
/// the parent's first child *as it is then* — prepending the first child
/// with another node keeps it first.
#[test]
fn prepend_with_the_first_child_keeps_the_list_order() {
    let mut dom: Dom = Dom::new();
    let (div, spans) = row(&mut dom, 2);
    let x = dom.create_element("i");
    dom.node_mut(div)
        .prepend([spans[0].into(), x.into()])
        .unwrap();
    assert_eq!(kids(&dom, div), [spans[0], x, spans[1]]);
}

/// §4.2.6: an invalid node fails `prepend()` before any node moves.
#[test]
fn prepend_inserts_nothing_when_a_node_is_invalid() {
    let mut dom: Dom = Dom::new();
    let (div, spans) = row(&mut dom, 1);
    let x = dom.create_element("i");
    let root = dom.root();
    let err = dom
        .node_mut(div)
        .prepend([x.into(), root.into()])
        .unwrap_err();
    assert_eq!(err, DomError::HierarchyRequest);
    assert_eq!(kids(&dom, div), spans);
}

// ── before() ───────────────────────────────────────────────────────

/// §4.2.6 `before()`: an invalid node (an ancestor of the parent) fails it
/// before the valid nodes before it are inserted.
#[test]
fn before_inserts_nothing_when_a_node_is_invalid() {
    let mut dom: Dom = Dom::new();
    let (div, spans) = row(&mut dom, 2);
    let x = dom.create_element("i");
    let err = dom
        .node_mut(spans[1])
        .before([x.into(), div.into()])
        .unwrap_err();
    assert_eq!(err, DomError::HierarchyRequest);
    assert_eq!(kids(&dom, div), spans);
}

/// §4.2.6 `before()` steps 3–5: the viable previous sibling is the first
/// preceding sibling not in the list — the receiver's previous sibling in
/// the list moves with the rest, in list order.
#[test]
fn before_with_its_previous_sibling_in_the_list() {
    let mut dom: Dom = Dom::new();
    let (div, spans) = row(&mut dom, 3);
    let x = dom.create_element("i");
    dom.node_mut(spans[2])
        .before([x.into(), spans[1].into()])
        .unwrap();
    assert_eq!(kids(&dom, div), [spans[0], x, spans[1], spans[2]]);
}

// ── after() ────────────────────────────────────────────────────────

/// §4.2.6 `after()`: a `DocumentFragment` then a node — one converted
/// fragment inserted before the viable next sibling; the old cursor
/// walked onto the emptied fragment and failed after a partial insert.
#[test]
fn after_with_a_fragment_then_a_node() {
    let mut dom: Dom = Dom::new();
    let (div, spans) = row(&mut dom, 2);
    let frag = dom.create_document_fragment();
    let (a, b, c) = (
        dom.create_element("a"),
        dom.create_element("b"),
        dom.create_element("c"),
    );
    dom.append_child(frag, a).unwrap();
    dom.append_child(frag, b).unwrap();
    dom.node_mut(spans[0])
        .after([frag.into(), c.into()])
        .unwrap();
    assert_eq!(kids(&dom, div), [spans[0], a, b, c, spans[1]]);
}

/// §4.2.6 `after()` steps 3–4: the viable next sibling is the first
/// following sibling not in the list.
#[test]
fn after_with_its_next_sibling_in_the_list() {
    let mut dom: Dom = Dom::new();
    let (div, spans) = row(&mut dom, 3);
    let x = dom.create_element("i");
    dom.node_mut(spans[0])
        .after([x.into(), spans[1].into()])
        .unwrap();
    assert_eq!(kids(&dom, div), [spans[0], x, spans[1], spans[2]]);
}

/// §4.2.6: an invalid node fails `after()` before any node moves.
#[test]
fn after_inserts_nothing_when_a_node_is_invalid() {
    let mut dom: Dom = Dom::new();
    let (div, spans) = row(&mut dom, 1);
    let x = dom.create_element("i");
    let err = dom
        .node_mut(spans[0])
        .after([x.into(), div.into()])
        .unwrap_err();
    assert_eq!(err, DomError::HierarchyRequest);
    assert_eq!(kids(&dom, div), spans);
}

// ── replaceWith() ──────────────────────────────────────────────────

/// §4.2.6 `replaceWith()` step 5: replacing the receiver with itself —
/// "this could have been inserted into node" — leaves it in place.
#[test]
fn replace_with_itself_keeps_the_node() {
    let mut dom: Dom = Dom::new();
    let (div, spans) = row(&mut dom, 2);
    dom.node_mut(spans[0])
        .replace_with([spans[0].into()])
        .unwrap();
    assert_eq!(kids(&dom, div), spans);
}

/// §4.2.6 `replaceWith()` steps 3–6: with itself and another node, both
/// stand where it stood, in list order, before the viable next sibling.
#[test]
fn replace_with_itself_and_another() {
    let mut dom: Dom = Dom::new();
    let (div, spans) = row(&mut dom, 2);
    let x = dom.create_element("i");
    dom.node_mut(spans[0])
        .replace_with([spans[0].into(), x.into()])
        .unwrap();
    assert_eq!(kids(&dom, div), [spans[0], x, spans[1]]);
}

/// §4.2.6: an invalid node fails `replaceWith()` before the receiver goes.
#[test]
fn replace_with_inserts_nothing_when_a_node_is_invalid() {
    let mut dom: Dom = Dom::new();
    let (div, spans) = row(&mut dom, 1);
    let x = dom.create_element("i");
    let err = dom
        .node_mut(spans[0])
        .replace_with([x.into(), div.into()])
        .unwrap_err();
    assert_eq!(err, DomError::HierarchyRequest);
    assert_eq!(kids(&dom, div), spans);
}

/// §4.2.6: `replaceWith()` with no nodes removes the receiver.
#[test]
fn replace_with_nothing_removes_the_node() {
    let mut dom: Dom = Dom::new();
    let (div, spans) = row(&mut dom, 2);
    dom.node_mut(spans[0])
        .replace_with(Vec::<NodeOrString>::new())
        .unwrap();
    assert_eq!(kids(&dom, div), [spans[1]]);
}

// ── replaceChildren() ──────────────────────────────────────────────

/// §4.2.6 `replaceChildren()` step 2: "ensure pre-insertion validity"
/// before "replace all" — an invalid node leaves the children as they
/// were, not cleared.
#[test]
fn replace_children_keeps_the_children_when_a_node_is_invalid() {
    let mut dom: Dom = Dom::new();
    let (div, spans) = row(&mut dom, 2);
    let x = dom.create_element("i");
    let err = dom
        .node_mut(div)
        .replace_children([x.into(), div.into()])
        .unwrap_err();
    assert_eq!(err, DomError::HierarchyRequest);
    assert_eq!(kids(&dom, div), spans);
}

/// §4.2.6: a current child in the list stays, in list order.
#[test]
fn replace_children_with_a_current_child() {
    let mut dom: Dom = Dom::new();
    let (div, spans) = row(&mut dom, 2);
    let x = dom.create_element("i");
    dom.node_mut(div)
        .replace_children([x.into(), spans[1].into()])
        .unwrap();
    assert_eq!(kids(&dom, div), [x, spans[1]]);
    assert!(dom.node(spans[0]).parent_node().is_none());
}
