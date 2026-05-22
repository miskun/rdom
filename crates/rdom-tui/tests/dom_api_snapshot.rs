//! Paint snapshot for the `dom_api` example / showcase demo.
//! Non-visual example originally — translated to a showcase demo
//! that renders the API walkthrough's textual report into a
//! `<pre>` block. Snapshot pins the report content + structure.

mod common;

use rdom_showcase::demos::dom_api;
use rdom_tui::prelude::*;

use common::{assert_snapshot, buffer_to_snapshot, render};

#[test]
fn dom_api_paint_matches_golden() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = dom_api::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = dom_api::stylesheet();
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 70, 50));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "dom_api.snap");
}
