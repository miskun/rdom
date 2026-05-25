//! Counter button — minimal button + click handler.
//!
//! Shared logic for the showcase demo and the standalone example:
//!
//! - `build(&mut TuiDom) -> NodeId` constructs the subtree.
//! - `stylesheet() -> Stylesheet` returns the demo's CSS.
//! - `source()` exposes the strings for the M7 Source tab.
//! - `run_standalone()` is the standalone-example entry point —
//!   `crates/rdom-tui/examples/counter_button.rs` is a thin shim
//!   that calls it.
//!
//! Exercises the click event + a `Rc<Cell<u32>>`-backed counter
//! that mutates a text node. The text-node mutation flows through
//! `MutationObserver` → `DirtyTracker`, and the next paint
//! re-cascades + repaints automatically.

use std::cell::Cell;
use std::io;
use std::rc::Rc;

use rdom_tui::{App, ListenerOptions, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="counter-demo">
  <h1>Counter button</h1>
  <p>Click the button or press Ctrl-C to exit.</p>
  <button>Clicks: 0</button>
</div>"#;

pub const CSS: &str = r#"
.counter-demo {
  flex: 1;
  flex-direction: column;
  padding: 2 4;
  gap: 1;
}
.counter-demo h1 {
  color: rgb(180, 220, 255);
  font-weight: bold;
  height: 1;
}
.counter-demo p {
  height: 1;
}
"#;

/// Build the demo subtree under `dom`. Returns the root `<div>` —
/// caller appends it to wherever the demo should mount.
pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "counter-demo").unwrap();

    let h1 = dom.create_element("h1");
    let h1_text = dom.create_text_node("Counter button");
    dom.append_child(h1, h1_text).unwrap();
    dom.append_child(root, h1).unwrap();

    let p = dom.create_element("p");
    let p_text = dom.create_text_node("Click the button or press Ctrl-C to exit.");
    dom.append_child(p, p_text).unwrap();
    dom.append_child(root, p).unwrap();

    let button = dom.create_element("button");
    // Label is just the text — UA `::before { content: "[ " }` and
    // `::after { content: " ]" }` provide the bracket chrome around it.
    let label = dom.create_text_node("Clicks: 0");
    dom.append_child(button, label).unwrap();
    dom.append_child(root, button).unwrap();

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

    root
}

/// The demo's stylesheet. Re-parsed on every call; cheap because
/// the CSS is tiny.
pub fn stylesheet() -> Stylesheet {
    rdom_css::from_css(CSS)
}

/// Standalone-example entry point. Builds a one-off `App` with the
/// demo subtree mounted directly under the root. Used by the
/// `crates/rdom-tui/examples/counter_button.rs` shim.
pub fn run_standalone() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = build(&mut dom);
    dom.append_child(root, demo_root).unwrap();
    App::new(dom, stylesheet())?.run()
}

pub struct CounterButton;

impl Demo for CounterButton {
    fn slug(&self) -> &'static str {
        "events/counter-button"
    }

    fn title(&self) -> &'static str {
        "Counter Button"
    }

    fn category(&self) -> Category {
        Category::Events
    }

    fn build(&self, dom: &mut TuiDom) -> NodeId {
        build(dom)
    }

    fn stylesheet(&self) -> Stylesheet {
        stylesheet()
    }

    fn source(&self) -> Source {
        Source {
            markup: MARKUP,
            css: CSS,
        }
    }
}
