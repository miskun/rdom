//! Paint snapshot for the `scrollable_list` example / showcase
//! demo. Pins the initial paint at scrollTop=0 — first ~10 rows
//! visible, scrollbar gutter reserved on the right.
use rdom_showcase::demos::scrollable_list;
use rdom_tui::prelude::*;

use crate::common::{assert_snapshot, buffer_to_snapshot, render};

#[test]
fn scrollable_list_initial_paint_matches_golden() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = scrollable_list::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = scrollable_list::stylesheet();
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 60, 14));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "scrollable_list.snap");
}
