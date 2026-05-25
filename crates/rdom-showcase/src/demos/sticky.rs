//! `position: sticky` — pinned header inside a scrollable list.
//!
//! Renders a scrollable container with 20 items + a sticky header
//! at the top. As the user scrolls (down-arrow / page-down), the
//! header stays pinned at y=0 while items move past it.
//!
//! Shared with the standalone example at
//! `crates/rdom-tui/examples/sticky_demo.rs` via `run_standalone()`.

use std::io;

use rdom_tui::{App, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="sticky-demo">
  <div class="header">Sticky header</div>
  <div class="item">Item 0</div>
  <div class="item">Item 1</div>
  …
  <div class="item">Item 19</div>
</div>"#;

pub const CSS: &str = r#"
.sticky-demo {
  width: 40;
  height: 15;
  overflow: auto;
  flex-direction: column;
}
.sticky-demo .header {
  height: 1;
  position: sticky;
  top: 0;
  background: rgb(40, 40, 60);
  color: rgb(230, 230, 240);
  font-weight: bold;
}
.sticky-demo .item {
  height: 1;
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "sticky-demo").unwrap();

    let header = dom.create_element("div");
    dom.set_attribute(header, "class", "header").unwrap();
    let header_text = dom.create_text_node("Sticky header");
    dom.append_child(header, header_text).unwrap();
    dom.append_child(root, header).unwrap();

    for i in 0..20 {
        let item = dom.create_element("div");
        dom.set_attribute(item, "class", "item").unwrap();
        let text = dom.create_text_node(&format!("Item {i}"));
        dom.append_child(item, text).unwrap();
        dom.append_child(root, item).unwrap();
    }

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

pub struct Sticky;

impl Demo for Sticky {
    fn slug(&self) -> &'static str {
        "positioning/sticky"
    }

    fn title(&self) -> &'static str {
        "Sticky header"
    }

    fn category(&self) -> Category {
        Category::Positioning
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
