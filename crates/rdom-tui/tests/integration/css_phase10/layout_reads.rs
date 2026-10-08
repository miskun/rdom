//! C10G-API-SMALL — what a consumer reads of generated boxes after
//! layout: where a generated fragment is drawn (a relatively positioned
//! pseudo-element's shift, CSS 2.1 §9.4.3), whether it is an outside list
//! marker (CSS Lists 3 §3.5), and which pseudo-element a positioned box
//! is (CSS Pseudo-Elements 4 §2).

use rdom_tui::ext::PseudoSlot;
use rdom_tui::prelude::*;
use rdom_tui::render::GeneratedFragment;

use super::{el, lay_out, text_el};

/// The generated fragments on `id`'s lines.
fn generated(dom: &TuiDom, id: NodeId) -> Vec<GeneratedFragment> {
    dom.node(id)
        .ext()
        .and_then(|e| e.inline_layout.as_ref())
        .map(|l| l.lines.iter().flat_map(|l| l.generated.clone()).collect())
        .unwrap_or_default()
}

/// CSS 2.1 §9.4.3: a relatively positioned `::before` is packed in its
/// host's line, then drawn shifted by its offsets — `offset()` is the
/// shift and `drawn_at()` where paint and hit-testing put it.
#[test]
fn a_fragment_tells_where_it_is_drawn() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let h = text_el(&mut dom, root, "div", "h", "x");
    lay_out(
        &mut dom,
        ".h::before { position: relative; top: 1; left: 2; content: 'ab' }",
        8,
        3,
    );
    let before = generated(&dom, h)
        .into_iter()
        .find(|g| g.slot == PseudoSlot::Before)
        .expect("in the line");
    assert_eq!((before.x, before.y), (0, 0), "packed where it is in flow");
    assert_eq!(before.offset(), (2, 1));
    assert_eq!(before.drawn_at(), (2, 1));
    assert!(!before.is_outside_marker());
}

/// CSS Lists 3 §3.5: an outside marker is a generated fragment beside the
/// line, taking no room in it; an inside one is in the line.
#[test]
fn a_fragment_tells_an_outside_marker() {
    for (position, outside) in [("outside", true), ("inside", false)] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let ul = el(&mut dom, root, "ul", "");
        let li = text_el(&mut dom, ul, "li", "", "a");
        lay_out(
            &mut dom,
            &format!("li {{ list-style-position: {position} }}"),
            8,
            1,
        );
        let marker = generated(&dom, li)
            .into_iter()
            .find(|g| g.slot == PseudoSlot::Marker)
            .expect("a marker");
        assert_eq!(marker.is_outside_marker(), outside, "{position}");
    }
}

/// CSS Pseudo-Elements 4 §2: an absolutely positioned `::before` /
/// `::after` is a box of its own; `positioned_pseudos()` names each one's
/// slot and gives its border box and lines.
#[test]
fn positioned_pseudos_name_their_slot() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let h = text_el(&mut dom, root, "div", "h", "x");
    lay_out(
        &mut dom,
        ".h { position: relative; height: 2 } \
         .h::before { position: absolute; top: 1; left: 3; content: 'b' } \
         .h::after { position: absolute; top: 0; right: 0; content: 'aa' }",
        8,
        2,
    );
    let ext = dom.node(h).ext().unwrap();
    let boxes: Vec<_> = ext
        .positioned_pseudos()
        .map(|p| (p.slot, p.border_box.x, p.border_box.y, p.lines.lines.len()))
        .collect();
    assert_eq!(
        boxes,
        [(PseudoSlot::Before, 3, 1, 1), (PseudoSlot::After, 6, 0, 1)]
    );
}
