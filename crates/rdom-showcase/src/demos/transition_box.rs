//! CSS `transition` demo — click a box, watch it animate between
//! two states.
//!
//! The `.box` has a base style; the `.box.active` class adds
//! different `width`/`color`. `transition: width 500ms ease-in-out,
//! color 500ms ease-in-out` interpolates between the two when the
//! `active` class toggles. The substrate's animation engine
//! (`rdom-tui::runtime::animation`) drives the per-tick value
//! interpolation; the paint pass sees the in-between values.
//!
//! Note on timing-function support: rdom-style only parses the
//! named keywords (`ease`, `ease-in`, `ease-out`, `ease-in-out`).
//! All four expand to their canonical cubic-bezier values
//! internally per the substrate. `cubic-bezier(a, b, c, d)`
//! literal syntax is deferred (`D-M3-2` in TECH_DEBT.md).

use std::io;

use rdom_tui::{App, ListenerOptions, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="transition-demo">
  <h1>CSS transition</h1>
  <p>Click the box to toggle its `active` class. Width + color animate over 500ms.</p>
  <div class="box">Click me</div>
</div>"#;

pub const CSS: &str = r#"
.transition-demo {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 1 2;
  gap: 1;
}
.transition-demo h1 {
  color: rgb(180, 220, 255);
  font-weight: bold;
}
.transition-demo .box {
  height: 3;
  width: 20;
  border: solid;
  border-color: rgb(120, 130, 150);
  color: rgb(180, 190, 210);
  padding: 0 2;
  transition: width 500ms ease-in-out, color 500ms ease-in-out, border-color 500ms ease-in-out;
}
.transition-demo .box.active {
  width: 40;
  color: rgb(255, 230, 150);
  border-color: rgb(220, 200, 120);
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "transition-demo").unwrap();

    let h1 = dom.create_element("h1");
    let h1_t = dom.create_text_node("CSS transition");
    dom.append_child(h1, h1_t).unwrap();
    dom.append_child(root, h1).unwrap();

    let p = dom.create_element("p");
    let p_t = dom.create_text_node(
        "Click the box to toggle its `active` class. Width + color animate over 500ms.",
    );
    dom.append_child(p, p_t).unwrap();
    dom.append_child(root, p).unwrap();

    let bx = dom.create_element("div");
    dom.set_attribute(bx, "class", "box").unwrap();
    let bx_text = dom.create_text_node("Click me");
    dom.append_child(bx, bx_text).unwrap();
    dom.append_child(root, bx).unwrap();

    // Click handler toggles the `active` class. The cascade picks
    // up the new computed values; the animation engine kicks off
    // transitions for the changed animatable properties (width,
    // color, border-color in this demo).
    dom.add_event_listener(bx, "click", ListenerOptions::default(), move |ctx| {
        if ctx.dom.node(bx).has_class("active") {
            let _ = ctx.dom.remove_class(bx, "active");
        } else {
            let _ = ctx.dom.add_class(bx, "active");
        }
    })
    .unwrap();

    root
}

pub fn stylesheet() -> Stylesheet {
    rdom_css::from_css(CSS)
}

pub fn run_standalone() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = build(&mut dom);
    dom.append_child(root, demo_root).unwrap();
    App::new(dom, stylesheet())?.run()
}

pub struct TransitionBox;

impl Demo for TransitionBox {
    fn slug(&self) -> &'static str {
        "animations/transition"
    }

    fn title(&self) -> &'static str {
        "CSS transition"
    }

    fn category(&self) -> Category {
        Category::Animations
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
