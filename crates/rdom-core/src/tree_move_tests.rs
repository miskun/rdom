//! C12G-MOVE-RECORD — inserting a node that already has a parent removes
//! it first, with its own `childList` record (DOM §4.2.3 "insert" step
//! 7.1 "adopt", whose step 2 runs "remove" — not suppressed — when the
//! node's parent is non-null). A `MutationObserver` sees the old parent
//! lose the child, then the new parent gain it.

use std::cell::RefCell;
use std::rc::Rc;

use crate::{Dom, Mutation, MutationObserver, NodeId};

type Records = Rc<RefCell<Vec<(NodeId, Vec<NodeId>, Vec<NodeId>)>>>;

/// The `ChildListChanged` records only: `(parent, added, removed)`.
struct ChildList(Records);

impl MutationObserver<()> for ChildList {
    fn observe(&mut self, _dom: &mut Dom<()>, record: &Mutation) {
        if let Mutation::ChildListChanged {
            parent,
            added,
            removed,
        } = record
        {
            self.0
                .borrow_mut()
                .push((*parent, added.clone(), removed.clone()));
        }
    }
}

fn observe(dom: &mut Dom) -> Records {
    let records = Records::default();
    dom.add_mutation_observer(Box::new(ChildList(records.clone())));
    records
}

fn el(dom: &mut Dom, parent: NodeId) -> NodeId {
    let e = dom.create_element("div");
    dom.append_child(parent, e).unwrap();
    e
}

/// §4.2.3 insert → adopt → remove: `append_child` of an attached node
/// queues the old parent's removal record, then the new parent's
/// insertion record.
#[test]
fn appending_an_attached_node_records_its_removal_first() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let (p1, p2) = (el(&mut dom, root), el(&mut dom, root));
    let a = el(&mut dom, p1);
    let records = observe(&mut dom);
    dom.append_child(p2, a).unwrap();
    assert_eq!(
        *records.borrow(),
        vec![(p1, vec![], vec![a]), (p2, vec![a], vec![])]
    );
}

/// The same for `insert_before`, and for a move inside one parent (a
/// reorder is a removal and an insertion, both on that parent).
#[test]
fn reordering_within_a_parent_records_a_removal_and_an_insertion() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let p = el(&mut dom, root);
    let (a, b) = (el(&mut dom, p), el(&mut dom, p));
    let records = observe(&mut dom);
    dom.insert_before(p, b, Some(a)).unwrap();
    assert_eq!(
        *records.borrow(),
        vec![(p, vec![], vec![b]), (p, vec![b], vec![])]
    );
    assert_eq!(dom.node(p).first_child().map(|n| n.id()), Some(b));
}

/// A node with no parent has nothing to leave: one record.
#[test]
fn inserting_an_orphan_records_only_its_insertion() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let a = dom.create_element("div");
    let records = observe(&mut dom);
    dom.append_child(root, a).unwrap();
    assert_eq!(*records.borrow(), vec![(root, vec![a], vec![])]);
}

/// §4.2.3 insert step 4: a fragment's children are removed from it with
/// observers suppressed, and one record for the fragment lists them all.
#[test]
fn inserting_a_fragment_records_its_emptying_once() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let frag = dom.create_document_fragment();
    let (a, b) = (el(&mut dom, frag), el(&mut dom, frag));
    let records = observe(&mut dom);
    dom.append_child(root, frag).unwrap();
    assert_eq!(
        *records.borrow(),
        vec![
            (frag, vec![], vec![a, b]),
            (root, vec![a], vec![]),
            (root, vec![b], vec![]),
        ]
    );
}

/// §4.2.3 "replace": the replaced child leaves in the same record the
/// new one arrives in (`removedNodes` « child », `addedNodes` « node »),
/// after the new node's own removal from its old parent.
#[test]
fn replace_child_records_the_replaced_child() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let (p1, p2) = (el(&mut dom, root), el(&mut dom, root));
    let old = el(&mut dom, p1);
    let new = el(&mut dom, p2);
    let records = observe(&mut dom);
    dom.replace_child(p1, old, new).unwrap();
    assert_eq!(
        *records.borrow(),
        vec![(p2, vec![], vec![new]), (p1, vec![new], vec![old])]
    );
    assert!(dom.node(old).parent_node().is_none());
}

/// §4.2.3 "replace" step 7: replacing a child with its own next sibling
/// leaves that sibling where the child was.
#[test]
fn replacing_a_child_with_its_next_sibling() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let p = el(&mut dom, root);
    let (a, b, c) = (el(&mut dom, p), el(&mut dom, p), el(&mut dom, p));
    dom.replace_child(p, a, b).unwrap();
    let kids: Vec<NodeId> = dom.node(p).children().map(|n| n.id()).collect();
    assert_eq!(kids, vec![b, c]);
}

/// §4.2.3 "pre-insert" step 3: inserting a node before itself inserts it
/// before its next sibling — it stays where it is.
#[test]
fn inserting_a_node_before_itself_keeps_its_place() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let p = el(&mut dom, root);
    let (a, b) = (el(&mut dom, p), el(&mut dom, p));
    dom.insert_before(p, a, Some(a)).unwrap();
    let kids: Vec<NodeId> = dom.node(p).children().map(|n| n.id()).collect();
    assert_eq!(kids, vec![a, b]);
}
