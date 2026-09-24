//! Paint snapshot for the `mutation_observer` showcase demo.
//! Pins the initial paint (empty list, empty log) so visual
//! regressions to the demo chrome flag immediately.
use rdom_showcase::demos::mutation_observer;
use rdom_tui::prelude::*;

use crate::common::{assert_snapshot, buffer_to_snapshot, render};

#[test]
fn mutation_observer_initial_paint_matches_golden() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = mutation_observer::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = mutation_observer::stylesheet();
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 60, 22));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "mutation_observer.snap");
}
