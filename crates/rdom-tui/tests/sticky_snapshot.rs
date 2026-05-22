//! Paint snapshot for the `sticky_demo` example / showcase demo.
//! Pins the initial paint (header at y=0, items below). Scrolling
//! is interactive and not part of the static snapshot.

mod common;

use rdom_showcase::demos::sticky;
use rdom_tui::prelude::*;

use common::{assert_snapshot, buffer_to_snapshot, render};

#[test]
fn sticky_initial_paint_matches_golden() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = sticky::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = sticky::stylesheet();
    // Demo defines its own 40×15 container; render at the same.
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 40, 15));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "sticky.snap");
}
