//! Scrollable list — 50 rows in a `overflow-y: auto` container,
//! with `:hover` highlight following the cursor.
//!
//! Exercises wheel scrolling (runtime's default action), the
//! scrollbar gutter (stable, always reserved), and the hover
//! cascade running on every mouse-move.

use std::io;

use rdom_tui::{App, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="scroll-list-demo">
  <h1>Scrollable list — wheel to scroll, hover highlights rows</h1>
  <div class="list">
    <div class="row">Row 01 — a scrollable entry</div>
    <div class="row">Row 02 — a scrollable entry</div>
    …
    <div class="row">Row 50 — a scrollable entry</div>
  </div>
</div>"#;

pub const CSS: &str = r#"
.scroll-list-demo {
  flex: 1;
  flex-direction: column;
  padding: 1 2;
  gap: 1;
}
.scroll-list-demo h1 {
  height: 1;
  color: rgb(180, 220, 255);
  font-weight: bold;
}
.scroll-list-demo .list {
  flex: 1;
  flex-direction: column;
  overflow-y: auto;
}
.scroll-list-demo .row {
  height: 1;
}
.scroll-list-demo .row:hover {
  background: rgb(169, 169, 169);
  color: rgb(255, 255, 255);
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "scroll-list-demo")
        .unwrap();

    let h1 = dom.create_element("h1");
    let h1_text = dom.create_text_node("Scrollable list — wheel to scroll, hover highlights rows");
    dom.append_child(h1, h1_text).unwrap();
    dom.append_child(root, h1).unwrap();

    let list = dom.create_element("div");
    dom.set_attribute(list, "class", "list").unwrap();
    for i in 1..=50 {
        let row = dom.create_element("div");
        dom.set_attribute(row, "class", "row").unwrap();
        let text = dom.create_text_node(&format!("  Row {i:02}  —  a scrollable entry"));
        dom.append_child(row, text).unwrap();
        dom.append_child(list, row).unwrap();
    }
    dom.append_child(root, list).unwrap();

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

pub struct ScrollableList;

impl Demo for ScrollableList {
    fn slug(&self) -> &'static str {
        "layout/scrollable-list"
    }

    fn title(&self) -> &'static str {
        "Scrollable list"
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
