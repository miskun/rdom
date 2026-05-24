//! Initial-paint snapshots for the M8 Text & inline demos.
//!
//! These pin the static initial render — text layout, inline
//! formatting, UA defaults for inline tags (`<strong>`, `<em>`,
//! `<code>`, `<mark>`), and IFC wrap at the chosen viewport
//! width. Style drift (color, modifiers) isn't part of the
//! snapshot — the harness records visible glyphs, and style
//! regressions are caught by the paint-pass unit tests in
//! `rdom-tui`.

mod common;

use rdom_showcase::demos::{headings, inline_formatting, whitespace_modes};
use rdom_tui::prelude::*;

use common::{assert_snapshot, buffer_to_snapshot, render};

#[test]
fn headings_initial_paint() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = headings::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = headings::stylesheet();
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 60, 14));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "headings.snap");
}

#[test]
fn whitespace_modes_initial_paint() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = whitespace_modes::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = whitespace_modes::stylesheet();
    // Wide viewport so the four columns each get ~20 cells —
    // enough to demonstrate wrap vs. no-wrap clearly without
    // crowding the longest sample line.
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 100, 14));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "whitespace_modes.snap");
}

#[test]
fn inline_formatting_initial_paint() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = inline_formatting::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = inline_formatting::stylesheet();
    // Width chosen so the first paragraph wraps to 3–4 lines —
    // enough to demonstrate IFC wrap with mixed inline children.
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 60, 14));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "inline_formatting.snap");
}
