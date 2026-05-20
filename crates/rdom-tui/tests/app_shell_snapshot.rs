//! Paint snapshot for the M5.6 `app_shell` example. Builds the same
//! DOM the example builds, renders it through the full
//! cascade → layout → paint pipeline at a fixed 80×20 viewport, and
//! compares the painted buffer against the golden at
//! `tests/snapshots/app_shell.snap`.
//!
//! `border-collapse: collapse` makes every internal border in the
//! shell render as a shared cell with the correct junction glyph
//! (`┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼`). Coincident-corner cells (e.g. shell's
//! top-left + header's top-left at (0,0)) render correctly via the
//! per-cell `Buffer::border_mask` — each ring's `paint_border` ORs
//! its directional bits in additively, and the M5.5c joiner
//! derives the glyph from the resulting mask. Closes the
//! `M5-COLLAPSE-2` limitation noted in the original M5 review gate.
//!
//! The DOM construction below mirrors `examples/app_shell.rs` — if
//! either side changes, the other must follow.
//!
//! To regenerate the golden after an intentional shell change:
//!
//! ```sh
//! UPDATE_SNAPSHOTS=1 cargo test -p rdom-tui --test app_shell_snapshot
//! ```

mod common;

use rdom_style::layout::Border;
use rdom_tui::prelude::*;

use common::{assert_snapshot, buffer_to_snapshot, render};

#[test]
fn app_shell_renders_with_collapsed_borders() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let shell = dom.create_element("shell");
    dom.append_child(root, shell).unwrap();

    let header = dom.create_element("header");
    dom.append_child(shell, header).unwrap();

    let body = dom.create_element("body");
    let sidebar = dom.create_element("sidebar");
    let middle = dom.create_element("middle");
    let right_panel = dom.create_element("right-panel");
    dom.append_child(body, sidebar).unwrap();
    dom.append_child(body, middle).unwrap();
    dom.append_child(body, right_panel).unwrap();
    dom.append_child(shell, body).unwrap();

    let footer = dom.create_element("footer");
    dom.append_child(shell, footer).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "shell",
            TuiStyle::new()
                .width(Size::Flex(1))
                .height(Size::Flex(1))
                .direction(Direction::Column)
                .border(Border::Single)
                .collapse_borders(),
        )
        .rule_unchecked(
            "header",
            TuiStyle::new()
                .height(Size::Fixed(3))
                .border(Border::Single),
        )
        .rule_unchecked(
            "body",
            TuiStyle::new()
                .height(Size::Flex(1))
                .direction(Direction::Row)
                .border(Border::Single),
        )
        .rule_unchecked(
            "sidebar",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .min_width(12)
                .border(Border::Single),
        )
        .rule_unchecked(
            "middle",
            TuiStyle::new().width(Size::Flex(1)).border(Border::Single),
        )
        .rule_unchecked(
            "right-panel",
            TuiStyle::new()
                .width(Size::Fixed(24))
                .max_width(32)
                .border(Border::Single),
        )
        .rule_unchecked(
            "footer",
            TuiStyle::new()
                .height(Size::Fixed(3))
                .border(Border::Single),
        );

    // Render at a fixed viewport so the snapshot is deterministic.
    // The Flex(1) shell expands to fill it — same code path the
    // runtime example uses, just with a controlled size.
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 80, 20));
    let snapshot = buffer_to_snapshot(&buf);
    assert_snapshot(&snapshot, "app_shell.snap");
}
