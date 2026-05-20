//! Scrollable list — scroll a long list with the mouse wheel, watch
//! `:hover` follow the cursor.
//!
//! 50 rows inside a viewport that only fits ~10. `overflow: Scroll`
//! on the container engages the runtime's wheel-scroll default
//! action (Phase 4). Moving the cursor over a row flips its
//! `:hover` pseudo-class (Phase 2 auto-hover + Phase 3 cascade
//! re-run).
//!
//! Controls:
//!   Mouse wheel up/down  — scroll.
//!   Mouse move           — highlight the row under the cursor.
//!   Ctrl-C               — quit.
//!
//! Run: `cargo run --example scrollable_list -p rdom-tui`

use std::io;

use rdom_tui::prelude::*;

fn main() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let screen = dom.create_element("screen");
    let title = dom.create_element("title");
    let title_text =
        dom.create_text_node("Scrollable list — wheel to scroll, hover highlights rows");
    dom.append_child(title, title_text).unwrap();

    let list = dom.create_element("list");
    for i in 1..=50 {
        let row = dom.create_element("row");
        let t = dom.create_text_node(&format!("  Row {i:02}  —  a scrollable entry"));
        dom.append_child(row, t).unwrap();
        dom.append_child(list, row).unwrap();
    }

    dom.append_child(screen, title).unwrap();
    dom.append_child(screen, list).unwrap();
    dom.append_child(root, screen).unwrap();

    // Author CSS — structural for the custom tags (`<screen>`,
    // `<title>`, `<list>`, `<row>` have no UA defaults) plus one
    // `row:hover` rule (`:hover` highlight is the point of this
    // example; without an author rule the cascade has nothing to
    // do on hover). Scroll is `overflow-y: Auto` — track + thumb
    // paint only when the row list actually overflows the
    // viewport (50 rows × 1 cell tall vs. whatever the user's
    // terminal gives us). On a tall terminal where all 50 rows
    // fit, no scrollbar paints. The 1-cell gutter is still
    // reserved either way (`scrollbar-gutter: stable` semantics)
    // so rows don't reflow when the scrollbar appears.
    let sheet = Stylesheet::new()
        .rule(
            "screen",
            TuiStyle::new()
                .direction(Direction::Column)
                .width(Size::Flex(1))
                .height(Size::Flex(1))
                .padding(Padding::symmetric(2, 1))
                .gap(1),
        )
        .unwrap()
        .rule("title", TuiStyle::new().height(Size::Fixed(1)))
        .unwrap()
        .rule(
            "list",
            TuiStyle::new()
                .direction(Direction::Column)
                .height(Size::Flex(1))
                .overflow_y(Overflow::Auto),
        )
        .unwrap()
        .rule("row", TuiStyle::new().height(Size::Fixed(1)))
        .unwrap()
        .rule(
            "row:hover",
            TuiStyle::new()
                .bg(Color::Rgb(169, 169, 169))
                .fg(Color::Rgb(255, 255, 255)),
        )
        .unwrap();

    App::new(dom, sheet)?.run()
}
