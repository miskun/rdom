//! Paint snapshot for the `parse_and_render` example / showcase
//! demo. Pins the three-crate end-to-end pipeline: parser → CSS →
//! cascade → layout → paint.
use rdom_showcase::demos::parse_and_render;
use rdom_tui::prelude::*;

use crate::common::{assert_snapshot, buffer_to_snapshot, render};

#[test]
fn parse_and_render_paint_matches_golden() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = parse_and_render::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = parse_and_render::stylesheet();
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 70, 16));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "parse_and_render.snap");
}
