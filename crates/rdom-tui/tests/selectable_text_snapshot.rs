//! Paint snapshot for the `selectable_text` example / showcase demo.
//! Pins the initial paint (no selection, no hover) so wrapping +
//! CJK width + structural padding regressions flag.

mod common;

use rdom_showcase::demos::selectable_text;
use rdom_tui::prelude::*;

use common::{assert_snapshot, buffer_to_snapshot, render};

#[test]
fn selectable_text_initial_paint_matches_golden() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = selectable_text::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = selectable_text::stylesheet();
    // Wide enough to fit the prose without wrapping every word; tall
    // enough that all three paragraphs land.
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 70, 16));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "selectable_text.snap");
}
