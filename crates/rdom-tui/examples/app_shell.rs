//! M5.6 — TUI app-shell demo. Showcases `border-collapse: collapse`
//! in the headline grid layout the M5 planning conversation called
//! out: a single bordered shell with nested rows and columns that
//! share their borders into clean junction glyphs.
//!
//! ```text
//! ┌────────────────────────────────────┐
//! ├─────┬───────────────────────┬──────┤
//! │     │                       │      │
//! │     ├───────────────────────┤      │
//! │     │                       │      │
//! ├─────┴───────────────────────┴──────┤
//! │ status                             │
//! └────────────────────────────────────┘
//! ```
//!
//! Run with `cargo run -p rdom-tui --example app_shell`. The
//! snapshot in `tests/snapshots/app_shell.snap` pins the exact
//! painted output. Every internal border is shared with the outer
//! shell's frame under `border-collapse: collapse`; the M5.5c paint
//! joiner rewrites the cells where lines meet into `├ ┤ ┬ ┴ ┼`.

use rdom_style::layout::Border;
use rdom_tui::App;
use rdom_tui::prelude::*;

fn build_dom() -> (TuiDom, Stylesheet) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    // ── DOM ──────────────────────────────────────────────────────
    let shell = dom.create_element("shell");
    dom.append_child(root, shell).unwrap();

    // Header row (just a horizontal divider — no children).
    let header = dom.create_element("header");
    dom.append_child(shell, header).unwrap();

    // Three-column body.
    let body = dom.create_element("body");
    let sidebar = dom.create_element("sidebar");
    let middle = dom.create_element("middle");
    let right_panel = dom.create_element("right-panel");
    dom.append_child(body, sidebar).unwrap();
    dom.append_child(body, middle).unwrap();
    dom.append_child(body, right_panel).unwrap();
    dom.append_child(shell, body).unwrap();

    // Footer row (status bar).
    let footer = dom.create_element("footer");
    dom.append_child(shell, footer).unwrap();

    // ── Stylesheet ───────────────────────────────────────────────
    let sheet = Stylesheet::bare()
        // Outer shell. Border + collapse turns every inner border
        // into shared cells; the joiner rewrites junction glyphs.
        // Flex(1) on both axes fills the viewport so the demo grows
        // to whatever terminal the user runs it in.
        .rule_unchecked(
            "shell",
            TuiStyle::new()
                .width(Size::Flex(1))
                .height(Size::Flex(1))
                .direction(Direction::Column)
                .border(Border::Single)
                .collapse_borders(),
        )
        // Header — 3 rows tall with a border. Under collapse, its
        // bottom border merges with the body's top border into a
        // single shared row of T-junctions.
        .rule_unchecked(
            "header",
            TuiStyle::new()
                .height(Size::Fixed(3))
                .border(Border::Single),
        )
        // Body — three columns, vertical-flex to fill remaining
        // height. Direction Row stacks its children horizontally.
        // Its own border provides the meeting row with the header
        // and footer.
        .rule_unchecked(
            "body",
            TuiStyle::new()
                .height(Size::Flex(1))
                .direction(Direction::Row)
                .border(Border::Single),
        )
        // Sidebar — fixed-ish width with a min-width floor.
        .rule_unchecked(
            "sidebar",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .min_width(12)
                .border(Border::Single),
        )
        // Middle — flex-grow eats the remaining horizontal space.
        .rule_unchecked(
            "middle",
            TuiStyle::new().width(Size::Flex(1)).border(Border::Single),
        )
        // Right panel — fixed-ish, max-width cap.
        .rule_unchecked(
            "right-panel",
            TuiStyle::new()
                .width(Size::Fixed(24))
                .max_width(32)
                .border(Border::Single),
        )
        // Footer — 3 rows tall.
        .rule_unchecked(
            "footer",
            TuiStyle::new()
                .height(Size::Fixed(3))
                .border(Border::Single),
        );

    (dom, sheet)
}

fn main() -> std::io::Result<()> {
    let (dom, sheet) = build_dom();
    App::new(dom, sheet)?.run()
}
