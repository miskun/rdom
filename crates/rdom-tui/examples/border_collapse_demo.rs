//! Focused single-purpose demo of `border-collapse: collapse`.
//!
//! Renders the headline ASCII art from the M5 design conversation:
//!
//! ```text
//! ┌─────────────────────┐
//! ├─────┬─────────┬─────┤
//! │     │         │     │
//! │     ├─────────┤     │
//! │     │         │     │
//! ├─────┴─────────┴─────┤
//! └─────────────────────┘
//! ```
//!
//! Five bordered boxes (outer shell + left column + middle column
//! split into two rows + right column) share their borders under
//! `border-collapse: collapse`. The M5.5c joiner derives the right
//! `┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼` glyph at every junction.
//!
//! Run with `cargo run -p rdom-tui --example border_collapse_demo`.

use rdom_style::layout::Border;
use rdom_tui::App;
use rdom_tui::prelude::*;

fn main() -> std::io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let shell = dom.create_element("shell");
    dom.append_child(root, shell).unwrap();

    let left = dom.create_element("col");
    dom.append_child(shell, left).unwrap();

    let middle = dom.create_element("middle-col");
    let top = dom.create_element("middle-row");
    let bottom = dom.create_element("middle-row");
    dom.append_child(middle, top).unwrap();
    dom.append_child(middle, bottom).unwrap();
    dom.append_child(shell, middle).unwrap();

    let right = dom.create_element("col");
    dom.append_child(shell, right).unwrap();

    let sheet = Stylesheet::bare()
        // Outer shell — bordered, direction row, collapse on.
        .rule_unchecked(
            "shell",
            TuiStyle::new()
                .width(Size::Flex(1))
                .height(Size::Flex(1))
                .direction(Direction::Row)
                .border(Border::Single)
                .collapse_borders(),
        )
        // Fixed-width side columns.
        .rule_unchecked(
            "col",
            TuiStyle::new()
                .width(Size::Fixed(12))
                .border(Border::Single),
        )
        // Middle column — flex-grow eats remaining width; direction
        // column inside so its children stack vertically.
        .rule_unchecked(
            "middle-col",
            TuiStyle::new()
                .width(Size::Flex(1))
                .direction(Direction::Column)
                .border(Border::Single),
        )
        // Two stacked rows inside the middle column.
        .rule_unchecked(
            "middle-row",
            TuiStyle::new().height(Size::Flex(1)).border(Border::Single),
        );

    App::new(dom, sheet)?.run()
}
