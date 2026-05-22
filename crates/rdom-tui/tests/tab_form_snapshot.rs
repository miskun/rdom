//! Paint snapshot for the `tab_form` example / showcase demo.
//! Pins the initial paint (no focus, empty inputs, "(not submitted)"
//! status). Catches regressions to `<input>`, `<textarea>`, and
//! `<button>` UA chrome.

mod common;

use rdom_showcase::demos::tab_form;
use rdom_tui::prelude::*;

use common::{assert_snapshot, buffer_to_snapshot, render};

#[test]
fn tab_form_initial_paint_matches_golden() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = tab_form::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = tab_form::stylesheet();
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 90, 16));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "tab_form.snap");
}
