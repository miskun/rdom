//! C11G-MODAL-INERT — inertness (HTML §6.3 "Inert subtrees": the `inert`
//! attribute and "blocked by a modal dialog").

use crate::{Dom, NodeId, TopLayerKind};

fn el(dom: &mut Dom, parent: NodeId, tag: &str) -> NodeId {
    let e = dom.create_element(tag);
    dom.append_child(parent, e).unwrap();
    e
}

/// With no `inert` attribute and no modal dialog nothing is inert.
#[test]
fn nothing_is_inert_by_default() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let div = el(&mut dom, root, "div");
    let t = dom.create_text_node("x");
    dom.append_child(div, t).unwrap();
    assert!(!dom.is_inert(div));
    assert!(!dom.is_inert(t));
    assert_eq!(dom.blocking_modal(), None);
}

/// HTML §6.3.1: the `inert` attribute makes the element and all its
/// flat tree descendants inert; its siblings are untouched.
#[test]
fn the_inert_attribute_makes_the_subtree_inert() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "div");
    let inner = el(&mut dom, a, "button");
    let t = dom.create_text_node("x");
    dom.append_child(inner, t).unwrap();
    let b = el(&mut dom, root, "button");
    dom.set_attribute(a, "inert", "").unwrap();
    assert!(dom.is_inert(a));
    assert!(dom.is_inert(inner));
    assert!(dom.is_inert(t));
    assert!(!dom.is_inert(b));
    dom.remove_attribute(a, "inert").unwrap();
    assert!(!dom.is_inert(inner));
}

/// HTML §6.3 "blocked by a modal dialog": while the topmost dialog in
/// the top layer is modal, every connected node except the dialog and
/// its descendants is inert — the root, the dialog's ancestors and its
/// siblings included.
#[test]
fn a_modal_dialog_makes_everything_outside_it_inert() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let main = el(&mut dom, root, "main");
    let outside = el(&mut dom, main, "button");
    let dlg = el(&mut dom, main, "dialog");
    let inside = el(&mut dom, dlg, "button");
    dom.add_to_top_layer(dlg, TopLayerKind::ModalDialog)
        .unwrap();
    assert_eq!(dom.blocking_modal(), Some(dlg));
    assert!(dom.is_inert(outside));
    assert!(dom.is_inert(main), "the dialog's ancestors are inert too");
    assert!(!dom.is_inert(dlg));
    assert!(!dom.is_inert(inside));
    dom.remove_from_top_layer(dlg);
    assert_eq!(dom.blocking_modal(), None);
    assert!(!dom.is_inert(outside));
}

/// Only the topmost modal dialog is the subject: a modal dialog beneath
/// it is inert, and a popover above it outside its subtree is too (HTML
/// excepts the subject's descendants only).
#[test]
fn only_the_topmost_modal_dialog_escapes() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let lower = el(&mut dom, root, "dialog");
    let upper = el(&mut dom, root, "dialog");
    let pop = el(&mut dom, root, "div");
    let nested_pop = el(&mut dom, upper, "div");
    dom.add_to_top_layer(lower, TopLayerKind::ModalDialog)
        .unwrap();
    dom.add_to_top_layer(upper, TopLayerKind::ModalDialog)
        .unwrap();
    dom.add_to_top_layer(pop, TopLayerKind::Popover).unwrap();
    dom.add_to_top_layer(nested_pop, TopLayerKind::Popover)
        .unwrap();
    assert_eq!(dom.blocking_modal(), Some(upper), "a popover is no dialog");
    assert!(dom.is_inert(lower));
    assert!(dom.is_inert(pop));
    assert!(!dom.is_inert(nested_pop), "inside the subject");
}

/// HTML §6.3.1: the descendants the `inert` attribute makes inert are
/// those "which don't otherwise escape inertness (such as modal
/// dialogs)" — a modal dialog inside an inert subtree is interactive,
/// while an `inert` element inside the dialog is still inert.
#[test]
fn a_modal_dialog_escapes_an_inert_ancestor() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div");
    let dlg = el(&mut dom, wrap, "dialog");
    let ok = el(&mut dom, dlg, "button");
    let off = el(&mut dom, dlg, "div");
    let off_btn = el(&mut dom, off, "button");
    dom.set_attribute(wrap, "inert", "").unwrap();
    dom.set_attribute(off, "inert", "").unwrap();
    dom.add_to_top_layer(dlg, TopLayerKind::ModalDialog)
        .unwrap();
    assert!(!dom.is_inert(dlg));
    assert!(!dom.is_inert(ok));
    assert!(dom.is_inert(off_btn));
    assert!(dom.is_inert(wrap));
}
