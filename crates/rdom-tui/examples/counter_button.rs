//! Counter button — the minimal App-based demo.
//!
//! A single `<button>` that increments a counter on click. The
//! text-node mutation flows through `MutationObserver` →
//! `DirtyTracker`, and the next frame re-cascades + repaints
//! automatically — no manual redraw call in the listener.
//!
//! Controls: click to increment. Ctrl-C to quit.
//!
//! Run: `cargo run --example counter_button -p rdom-tui`
//!
//! Exercises Phase 3 (App event loop), Phase 2 (mouse routing +
//! click synthesis), Phase 5 (focus on click → `:focus` cascade).

use std::cell::Cell;
use std::io;
use std::rc::Rc;

use rdom_tui::prelude::*;

fn main() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let screen = dom.create_element("screen");
    let title = dom.create_element("title");
    let title_text = dom.create_text_node("Counter button demo");
    dom.append_child(title, title_text).unwrap();

    let hint = dom.create_element("hint");
    let hint_text = dom.create_text_node("Click the button or press Ctrl-C to exit.");
    dom.append_child(hint, hint_text).unwrap();

    let button = dom.create_element("button");
    // Label is just the text — UA `::before { content: "[ " }` and
    // `::after { content: " ]" }` provide the bracket chrome around it.
    let label = dom.create_text_node("Clicks: 0");
    dom.append_child(button, label).unwrap();

    dom.append_child(screen, title).unwrap();
    dom.append_child(screen, hint).unwrap();
    dom.append_child(screen, button).unwrap();
    dom.append_child(root, screen).unwrap();

    // Author CSS is layout-only — `<screen>`, `<title>`, `<hint>` are
    // custom tags rdom doesn't ship UA defaults for, so they need a
    // minimum of structural styling to render at all. The `<button>`
    // intentionally has NO author rules: this example exists to show
    // what a button looks like out-of-the-box.
    let sheet = Stylesheet::new()
        .rule(
            "screen",
            TuiStyle::new()
                .direction(Direction::Column)
                .width(Size::Flex(1))
                .height(Size::Flex(1))
                .padding(Padding::symmetric(4, 2))
                .gap(1),
        )
        .unwrap()
        .rule("title", TuiStyle::new().height(Size::Fixed(1)))
        .unwrap()
        .rule("hint", TuiStyle::new().height(Size::Fixed(1)))
        .unwrap();

    let count = Rc::new(Cell::new(0u32));
    let c = count.clone();
    dom.add_event_listener(button, "click", ListenerOptions::default(), move |ctx| {
        let n = c.get() + 1;
        c.set(n);
        let _ = ctx
            .dom
            .node_mut(label)
            .set_node_value(&format!("Clicks: {n}"));
    })
    .unwrap();

    App::new(dom, sheet)?.run()
}
