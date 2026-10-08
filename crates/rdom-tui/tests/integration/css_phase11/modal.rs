//! C11-MODAL-POPOVER — `:modal` and the top layer a modal `<dialog>`
//! lives in (HTML §4.11.4, CSS Position 4 "top layer"): rendered above
//! every stacking context, out of every ancestor's clip, against the
//! viewport, its `::backdrop` between it and the page, which is inert
//! to the pointer.

use rdom_tui::layout::Position;
use rdom_tui::render::Buffer;
use rdom_tui::runtime::builtins::dialog;
use rdom_tui::{HitTestExt, LayoutRect, NodeId, TuiDom, TuiNodeExt};

use crate::css_phase5::{lay_out, paint, rect, rows};

/// An element with attributes and optional text, appended to `parent`.
fn node(dom: &mut TuiDom, parent: NodeId, tag: &str, attrs: &[(&str, &str)], text: &str) -> NodeId {
    let e = dom.create_element(tag);
    for (k, v) in attrs {
        dom.set_attribute(e, k, v).unwrap();
    }
    if !text.is_empty() {
        let t = dom.create_text_node(text);
        dom.append_child(e, t).unwrap();
    }
    dom.append_child(parent, e).unwrap();
    e
}

/// HTML §4.11.4: `showModal()` adds the dialog to the top layer, where
/// `:modal` matches it; `close()` removes it; `show()` opens it out of
/// the top layer. No marker attribute stands in for the state.
#[test]
fn show_modal_and_close_move_the_dialog_in_and_out_of_the_top_layer() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = node(&mut dom, root, "dialog", &[], "hi");
    dialog::show_modal(&mut dom, d);
    assert_eq!(dom.top_layer(), [d]);
    assert!(dom.matches(d, "dialog:modal").unwrap());
    assert!(dialog::is_modal(&dom, d));
    assert_eq!(dom.node(d).attributes().count(), 1, "only `open`");
    dialog::close(&mut dom, d, "");
    assert!(dom.top_layer().is_empty());
    assert!(!dom.matches(d, ":modal").unwrap());
    dialog::show(&mut dom, d);
    assert!(dom.top_layer().is_empty());
    assert!(!dom.matches(d, ":modal").unwrap());
}

/// HTML's rendering section (the UA's `dialog:modal` rules): a modal
/// dialog is fixed,
/// inset 0 with auto margins and a fit-content size — centred in the
/// viewport, wherever it sits in the tree.
#[test]
fn a_modal_dialog_is_centred_in_the_viewport() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    node(&mut dom, root, "p", &[], "page");
    let d = node(&mut dom, root, "dialog", &[], "hi");
    dialog::show_modal(&mut dom, d);
    lay_out(&mut dom, "dialog { border: none; padding: 0 }", 20, 9);
    assert_eq!(rect(&dom, d), LayoutRect::new(9, 4, 2, 1));
}

/// CSS Position 4: an element in the top layer whose `position` is not
/// `absolute` or `fixed` computes to `absolute`, and its containing block
/// is the initial containing block — not a positioned ancestor.
#[test]
fn position_computes_to_absolute_against_the_viewport() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let frame = node(&mut dom, root, "div", &[("class", "frame")], "");
    let d = node(&mut dom, frame, "dialog", &[], "hi");
    dialog::show_modal(&mut dom, d);
    lay_out(
        &mut dom,
        ".frame { position: relative; margin-left: 5; width: 4; height: 2 } \
         dialog { position: static; border: none; padding: 0 }",
        20,
        9,
    );
    assert_eq!(dom.node(d).computed().unwrap().position, Position::Absolute);
    assert_eq!(rect(&dom, d), LayoutRect::new(9, 4, 2, 1));
}

/// CSS Position 4: the top layer paints after the whole document, above
/// a `z-index: 99` box, and no ancestor's `overflow` clips it.
#[test]
fn the_top_layer_paints_above_every_context_and_outside_every_clip() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let clip = node(&mut dom, root, "div", &[("class", "clip")], "");
    let d = node(&mut dom, clip, "dialog", &[], "hi");
    node(&mut dom, root, "div", &[("class", "cover")], "");
    dialog::show_modal(&mut dom, d);
    let buf = paint(
        &mut dom,
        ".clip { overflow: hidden; width: 1; height: 1 } \
         .cover { position: fixed; inset: 0; z-index: 99; background-color: red } \
         dialog { border: none; padding: 0 }",
        7,
        3,
    );
    assert_eq!(rows(&buf, 7, 3)[1], "  hi   ");
}

/// Pseudo-Elements 4 `::backdrop`: a top-layer element's backdrop covers
/// the viewport between the page and the element; a dialog not rendered
/// (`display: none`) has none.
#[test]
fn the_backdrop_paints_between_the_page_and_the_dialog() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    node(&mut dom, root, "p", &[], "page");
    let d = node(&mut dom, root, "dialog", &[], "hi");
    dialog::show_modal(&mut dom, d);
    let css = "dialog { border: none; padding: 0; background-color: blue } \
               dialog::backdrop { background-color: rgb(0 128 0) }";
    let buf: Buffer = paint(&mut dom, css, 6, 3);
    let green = rdom_tui::Color::Rgb(0, 128, 0);
    assert_eq!(buf.cell(0, 0).unwrap().bg, green, "over the page");
    assert_eq!(
        buf.cell(0, 0).unwrap().symbol(),
        "p",
        "the page shows through"
    );
    assert_eq!(buf.cell(2, 1).unwrap().bg, rdom_tui::Color::Rgb(0, 0, 255));
    let hidden = paint(&mut dom, &format!("{css} dialog {{ display: none }}"), 6, 3);
    assert_eq!(hidden.cell(0, 0).unwrap().bg, rdom_tui::Color::Reset);
}

/// HTML §6.3 (inert subtrees): with a modal dialog open, the rest of the
/// document is inert — a point outside the dialog hits its `::backdrop`,
/// whose events go to the dialog; a point inside hits the dialog's
/// content.
#[test]
fn outside_a_modal_dialog_the_pointer_hits_its_backdrop() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let page = node(&mut dom, root, "button", &[], "page");
    let d = node(&mut dom, root, "dialog", &[], "");
    let inner = node(&mut dom, d, "span", &[], "hi");
    dialog::show_modal(&mut dom, d);
    lay_out(&mut dom, "dialog { border: none; padding: 0 }", 6, 3);
    assert_eq!(rect(&dom, d), LayoutRect::new(2, 1, 2, 1));
    assert_eq!(dom.hit_test(0, 0), Some(d), "not the page's button");
    assert_eq!(dom.hit_test(2, 1), Some(inner));
    assert_ne!(dom.hit_test(0, 0), Some(page));
    dialog::close(&mut dom, d, "");
    lay_out(&mut dom, "dialog { border: none; padding: 0 }", 6, 3);
    assert_eq!(dom.hit_test(0, 0), Some(page));
}

/// The top layer is the element's only paint: the stacking walk leaves
/// it out, so a translucent dialog composites once over the page — not
/// once in place and again on top.
#[test]
fn a_top_layer_element_paints_once() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = node(&mut dom, root, "dialog", &[], "");
    dialog::show_modal(&mut dom, d);
    let buf = paint(
        &mut dom,
        "dialog { border: none; padding: 0; width: 3; height: 1; \
                  background-color: rgb(255 0 0 / 50%) }",
        7,
        3,
    );
    // Centred: columns 2..5 of row 1. Half red over the dark scheme's
    // black canvas, once: 127 or 128.
    let bg = buf.cell(3, 1).unwrap().bg;
    let rdom_tui::Color::Rgb(r, g, b) = bg else {
        panic!("an RGB fill: {bg:?}");
    };
    assert!(
        (127..=128).contains(&r) && g == 0 && b == 0,
        "({r}, {g}, {b})"
    );
}
