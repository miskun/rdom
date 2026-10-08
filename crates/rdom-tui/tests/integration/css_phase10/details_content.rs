//! C10-DETAILS-CONTENT — HTML §4.11.1 / §15.5.20 and CSS Pseudo-Elements
//! 4: a `<details>` element's content — everything but its first
//! `<summary>` child — is slotted into `::details-content`, which hides it
//! while the element is closed (`content-visibility: hidden` in the UA
//! sheet) and which its content inherits from.

use rdom_tui::prelude::*;

use super::{el, paint_tree, text_el};

/// `<details [open]><summary>S</summary>loose<p>para</p></details>`.
fn details(dom: &mut TuiDom, root: NodeId, open: bool) -> NodeId {
    let d = el(dom, root, "details", "");
    if open {
        dom.set_attribute(d, "open", "").unwrap();
    }
    text_el(dom, d, "summary", "", "S");
    let t = dom.create_text_node("loose");
    dom.append_child(d, t).unwrap();
    text_el(dom, d, "p", "", "para");
    d
}

/// HTML §15.5.20: a closed `<details>` renders its summary alone — its
/// whole content slot is hidden, text directly inside it included (it
/// used to show: only element children were hidden).
#[test]
fn a_closed_details_hides_its_whole_content() {
    let rows = paint_tree("", 8, 3, |dom, root| {
        details(dom, root, false);
    });
    assert_eq!(rows, ["▸ S     ", "        ", "        "]);
    let rows = paint_tree("", 8, 3, |dom, root| {
        details(dom, root, true);
    });
    assert_eq!(rows, ["▾ S     ", "loose   ", "para    "]);
}

/// The content slot is the content's parent for inheritance: a color set
/// on `::details-content` reaches the content, not the summary.
#[test]
fn the_content_inherits_from_details_content() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = details(&mut dom, root, true);
    let buf = super::paint(&mut dom, "details::details-content { color: red }", 8, 3);
    let red = rdom_tui::Color::Rgb(255, 0, 0);
    assert_eq!(buf.cell(0, 2).unwrap().fg, red, "the <p> in the slot");
    assert_ne!(
        buf.cell(2, 0).unwrap().fg,
        red,
        "the summary is not slotted"
    );
    assert!(
        dom.node(d)
            .ext()
            .unwrap()
            .computed_details_content()
            .is_some()
    );
}

/// `display: none` on the slot hides the content of an open `<details>`
/// too; a second `<summary>` is content, hidden while closed.
#[test]
fn the_slot_decides_what_shows() {
    let rows = paint_tree(
        "details::details-content { display: none }",
        8,
        3,
        |dom, root| {
            details(dom, root, true);
        },
    );
    assert_eq!(rows, ["▾ S     ", "        ", "        "]);
    let rows = paint_tree("", 8, 3, |dom, root| {
        let d = details(dom, root, false);
        text_el(dom, d, "summary", "", "T");
    });
    assert_eq!(rows, ["▸ S     ", "        ", "        "]);
}
