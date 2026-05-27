//! Flex Row — three colored boxes that share a row via `flex: 1`.
//! Demonstrates how flex distributes remaining space across
//! siblings of equal flex weight.
//!
//! Class scoping: all selectors are descendants of `.flex-row-demo`
//! so this demo's CSS only applies inside its own subtree. The
//! showcase preloads every demo's stylesheet at startup; without
//! scoping, demos' rules would cross-contaminate.

use rdom_tui::{NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

const MARKUP: &str = r#"<div class="flex-row-demo">
  <div class="box a">A</div>
  <div class="box b">B</div>
  <div class="box c">C</div>
</div>"#;

const CSS: &str = r#"
.flex-row-demo {
  display: flex;
  flex-direction: row;
  gap: 1;
  padding: 1 2;
}
.flex-row-demo .box {
  flex: 1;
  height: 3;
  border: solid;
  padding: 0 1;
}
.flex-row-demo .a { border-color: rgb(220, 160, 160); color: rgb(220, 160, 160); }
.flex-row-demo .b { border-color: rgb(160, 200, 220); color: rgb(160, 200, 220); }
.flex-row-demo .c { border-color: rgb(160, 220, 180); color: rgb(160, 220, 180); }
"#;

pub struct FlexRow;

impl Demo for FlexRow {
    fn slug(&self) -> &'static str {
        "layout/flex-row"
    }

    fn title(&self) -> &'static str {
        "Flex Row"
    }

    fn category(&self) -> Category {
        Category::Layout
    }

    fn build(&self, dom: &mut TuiDom) -> NodeId {
        let root = dom.create_element("div");
        dom.set_attribute(root, "class", "flex-row-demo").unwrap();
        for (letter, cls) in [("A", "a"), ("B", "b"), ("C", "c")] {
            let box_el = dom.create_element("div");
            dom.set_attribute(box_el, "class", &format!("box {cls}"))
                .unwrap();
            let text = dom.create_text_node(letter);
            dom.append_child(box_el, text).unwrap();
            dom.append_child(root, box_el).unwrap();
        }
        root
    }

    fn stylesheet(&self) -> Stylesheet {
        rdom_css::from_css(CSS)
    }

    fn source(&self) -> Source {
        Source {
            markup: MARKUP,
            css: CSS,
        }
    }
}
