//! C11-MODAL-POPOVER — the document's top layer (CSS Position 4, HTML
//! §6.12 / §4.11.4) and `:modal` (Selectors 4 §11, HTML §4.16.3).

use std::cell::RefCell;
use std::rc::Rc;

use crate::{Dom, InteractionKind, Mutation, MutationObserver, NodeId, TopLayerKind};

fn el(dom: &mut Dom, parent: NodeId, tag: &str) -> NodeId {
    let e = dom.create_element(tag);
    dom.append_child(parent, e).unwrap();
    e
}

struct Log(Rc<RefCell<Vec<NodeId>>>);

impl MutationObserver<()> for Log {
    fn observe(&mut self, _: &mut Dom, record: &Mutation) {
        if let Mutation::InteractionChanged {
            next: Some(id),
            kind: InteractionKind::TopLayer,
            ..
        } = record
        {
            self.0.borrow_mut().push(*id);
        }
    }
}

/// The top layer is an ordered set: adding appends, adding an element
/// already in it moves it to the end ("add an element to the top layer"
/// removes it first), removing takes it out. Each change is reported.
#[test]
fn the_top_layer_is_an_ordered_set() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "dialog");
    let b = el(&mut dom, root, "div");
    let log = Rc::new(RefCell::new(Vec::new()));
    dom.add_mutation_observer(Box::new(Log(log.clone())));
    dom.add_to_top_layer(a, TopLayerKind::ModalDialog).unwrap();
    dom.add_to_top_layer(b, TopLayerKind::Popover).unwrap();
    assert_eq!(dom.top_layer(), [a, b]);
    dom.add_to_top_layer(a, TopLayerKind::ModalDialog).unwrap();
    assert_eq!(dom.top_layer(), [b, a]);
    assert_eq!(dom.top_layer_kind(a), Some(TopLayerKind::ModalDialog));
    assert_eq!(dom.top_layer_kind(b), Some(TopLayerKind::Popover));
    assert!(dom.remove_from_top_layer(b));
    assert!(!dom.remove_from_top_layer(b), "already out");
    assert_eq!(dom.top_layer(), [a]);
    assert_eq!(dom.top_layer_kind(b), None);
    assert_eq!(*log.borrow(), [a, b, a, b]);
}

/// A node that is not a connected element cannot enter the top layer.
#[test]
fn only_connected_elements_enter_the_top_layer() {
    let mut dom: Dom = Dom::new();
    let loose = dom.create_element("dialog");
    assert!(
        dom.add_to_top_layer(loose, TopLayerKind::ModalDialog)
            .is_err()
    );
    let text = dom.create_text_node("x");
    let root = dom.root();
    dom.append_child(root, text).unwrap();
    assert!(dom.add_to_top_layer(text, TopLayerKind::Popover).is_err());
    assert!(dom.top_layer().is_empty());
}

/// HTML's removing steps (dialog §4.11.4, popover §6.12): an element
/// removed from the document leaves the top layer — itself or as part of
/// a removed subtree — and the change is reported while it is connected.
#[test]
fn removal_from_the_document_leaves_the_top_layer() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div");
    let inner = el(&mut dom, wrap, "dialog");
    let other = el(&mut dom, root, "div");
    dom.add_to_top_layer(inner, TopLayerKind::ModalDialog)
        .unwrap();
    dom.add_to_top_layer(other, TopLayerKind::Popover).unwrap();
    let log = Rc::new(RefCell::new(Vec::new()));
    dom.add_mutation_observer(Box::new(Log(log.clone())));
    dom.remove_child(root, wrap).unwrap();
    assert_eq!(dom.top_layer(), [other]);
    assert_eq!(*log.borrow(), [inner]);
}

/// Selectors 4 `:modal`, HTML §4.16.3: a dialog shown modally — in the
/// top layer as a modal dialog — not a popover, not a plain open dialog.
#[test]
fn modal_matches_modal_dialogs_in_the_top_layer() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let modal = el(&mut dom, root, "dialog");
    let open = el(&mut dom, root, "dialog");
    dom.set_attribute(open, "open", "").unwrap();
    let pop = el(&mut dom, root, "div");
    dom.add_to_top_layer(modal, TopLayerKind::ModalDialog)
        .unwrap();
    dom.add_to_top_layer(pop, TopLayerKind::Popover).unwrap();
    assert!(dom.matches(modal, ":modal").unwrap());
    assert!(!dom.matches(open, ":modal").unwrap());
    assert!(!dom.matches(pop, ":modal").unwrap());
    assert!(dom.matches(modal, ":MODAL").unwrap());
    dom.remove_from_top_layer(modal);
    assert!(!dom.matches(modal, ":modal").unwrap());
}
