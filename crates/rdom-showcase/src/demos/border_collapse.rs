//! `border-collapse: collapse` — five bordered boxes sharing edges.
//!
//! The headline ASCII art from the M5 design conversation:
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
//! Exercises the joiner: every junction picks the right
//! `┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼` glyph.

use std::io;

use rdom_tui::{App, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="border-collapse-demo">
  <div class="col left"></div>
  <div class="middle">
    <div class="row top"></div>
    <div class="row bottom"></div>
  </div>
  <div class="col right"></div>
</div>"#;

pub const CSS: &str = r#"
.border-collapse-demo {
  flex: 1;
  display: flex;
  flex-direction: row;
  border: solid;
  border-collapse: collapse;
}
.border-collapse-demo .col {
  width: 12;
  border: solid;
}
.border-collapse-demo .middle {
  flex: 1;
  display: flex;
  flex-direction: column;
  border: solid;
  /* BORDER-MODEL-1: collapse is non-inheriting; the middle column
   * declares it so its `.row` children share borders with each
   * other AND with `.middle`'s own ring. */
  border-collapse: collapse;
}
.border-collapse-demo .middle .row {
  flex: 1;
  border: solid;
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "border-collapse-demo")
        .unwrap();

    let left = dom.create_element("div");
    dom.set_attribute(left, "class", "col left").unwrap();
    dom.append_child(root, left).unwrap();

    let middle = dom.create_element("div");
    dom.set_attribute(middle, "class", "middle").unwrap();
    let top = dom.create_element("div");
    dom.set_attribute(top, "class", "row top").unwrap();
    let bottom = dom.create_element("div");
    dom.set_attribute(bottom, "class", "row bottom").unwrap();
    dom.append_child(middle, top).unwrap();
    dom.append_child(middle, bottom).unwrap();
    dom.append_child(root, middle).unwrap();

    let right = dom.create_element("div");
    dom.set_attribute(right, "class", "col right").unwrap();
    dom.append_child(root, right).unwrap();

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

pub struct BorderCollapse;

impl Demo for BorderCollapse {
    fn slug(&self) -> &'static str {
        "layout/border-collapse"
    }

    fn title(&self) -> &'static str {
        "Border collapse"
    }

    fn category(&self) -> Category {
        Category::Layout
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
