//! Counter button — the minimal `App`-based program.
//!
//! One `<button>` that increments a counter on click. The listener
//! rewrites a text node; that mutation flows through
//! `MutationObserver` → the dirty tracker, and the next frame
//! re-cascades and repaints on its own — no manual redraw call.
//!
//! Controls: click to increment. Ctrl-C to quit.
//!
//! Run: `cargo run -p rdom-tui --example counter_button`
//!
//! Self-contained on purpose: this file is the whole program. The
//! browsable version with a source tab lives in `rdom-showcase`
//! ("Events → Counter Button").

use std::cell::Cell;
use std::io;
use std::rc::Rc;

use rdom_tui::{App, ListenerOptions, TuiDom};

const CSS: &str = r#"
.counter-demo {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 1 2;
  gap: 1;
}
.counter-demo h1 {
  color: rgb(180, 220, 255);
  font-weight: bold;
}
.counter-demo p {
  height: 1;
}
"#;

fn main() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let host = dom.create_element("div");
    dom.set_attribute(host, "class", "counter-demo").unwrap();
    dom.append_child(root, host).unwrap();

    let h1 = dom.create_element("h1");
    let h1_text = dom.create_text_node("Counter button");
    dom.append_child(h1, h1_text).unwrap();
    dom.append_child(host, h1).unwrap();

    let p = dom.create_element("p");
    let p_text = dom.create_text_node("Click the button or press Ctrl-C to exit.");
    dom.append_child(p, p_text).unwrap();
    dom.append_child(host, p).unwrap();

    // The label is plain text; the UA stylesheet's `::before` /
    // `::after` supply the `[ … ]` bracket chrome.
    let button = dom.create_element("button");
    let label = dom.create_text_node("Clicks: 0");
    dom.append_child(button, label).unwrap();
    dom.append_child(host, button).unwrap();

    let count = Rc::new(Cell::new(0u32));
    let c = count.clone();
    dom.add_event_listener(button, "click", ListenerOptions::default(), move |ctx| {
        let n = c.get() + 1;
        c.set(n);
        ctx.dom
            .node_mut(label)
            .set_node_value(&format!("Clicks: {n}"))
            .expect("label text node is live");
    })
    .unwrap();

    App::new(dom, rdom_css::from_css(CSS))?.run()
}
