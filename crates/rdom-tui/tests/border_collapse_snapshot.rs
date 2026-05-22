//! Paint snapshot for the `border_collapse_demo` example /
//! showcase demo. Pins the joiner output at a fixed viewport so
//! a regression in `paint_pass::border_join` flags immediately.

mod common;

use rdom_showcase::demos::border_collapse;
use rdom_tui::prelude::*;

use common::{assert_snapshot, buffer_to_snapshot, render};

#[test]
fn border_collapse_paint_matches_golden() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = border_collapse::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = border_collapse::stylesheet();
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 40, 9));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "border_collapse.snap");
}
