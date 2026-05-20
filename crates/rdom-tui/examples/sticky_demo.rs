//! M5.4 — `position: sticky` demo.
//!
//! A scrollable list with a sticky header. As the user scrolls the
//! list down, the header stays pinned at the top of the scrollport
//! instead of scrolling off-screen.
//!
//! ```text
//! ┌── Sticky header ────────────────┐
//! │ Item 0                          │
//! │ Item 1                          │
//! │ ...                             │   ← scroll the list
//! └─────────────────────────────────┘
//! ```
//!
//! After scrolling, the header stays at y=0 while items move up:
//!
//! ```text
//! ┌── Sticky header ────────────────┐   ← pinned
//! │ Item 7                          │
//! │ Item 8                          │
//! │ ...                             │
//! └─────────────────────────────────┘
//! ```
//!
//! Press the down-arrow / page-down keys to scroll. The header
//! never leaves its sticky pin point.

use rdom_style::layout::{Length, Position};
use rdom_tui::App;
use rdom_tui::prelude::*;

fn main() -> std::io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    // Scroll container (overflow: hidden gives it the scrollport
    // semantics sticky uses).
    let scroller = dom.create_element("scroller");
    dom.append_child(root, scroller).unwrap();

    // Sticky header — stays at the top of the scrollport.
    let header = dom.create_element("header");
    let header_text = dom.create_text_node("Sticky header");
    dom.append_child(header, header_text).unwrap();
    dom.append_child(scroller, header).unwrap();

    // 20 list items so the content overflows the viewport and the
    // user has something to scroll past.
    for i in 0..20 {
        let item = dom.create_element("item");
        let label = format!("Item {i}");
        let text = dom.create_text_node(&label);
        dom.append_child(item, text).unwrap();
        dom.append_child(scroller, item).unwrap();
    }

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "scroller",
            TuiStyle::new()
                .width(Size::Fixed(40))
                .height(Size::Fixed(15))
                .overflow(Overflow::Auto),
        )
        .rule_unchecked(
            "header",
            TuiStyle::new()
                .height(Size::Fixed(1))
                .position(Position::Sticky)
                .top(Length::Cells(0))
                .bg(rdom_style::Color::Rgb(40, 40, 60))
                .fg(rdom_style::Color::Rgb(230, 230, 240))
                .bold(true),
        )
        .rule_unchecked("item", TuiStyle::new().height(Size::Fixed(1)));

    App::new(dom, sheet)?.run()
}
