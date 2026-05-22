//! Hello World — the simplest possible demo. A `<div>` with text
//! inside. Acts as the M2 scaffold's "is the shell actually
//! working" canary.

use rdom_core::NodeId;
use rdom_style::Color;
use rdom_tui::layout::Padding;
use rdom_tui::{Stylesheet, TuiDom, TuiStyle};

use crate::{Category, Demo, Source};

pub struct HelloWorld;

impl Demo for HelloWorld {
    fn slug(&self) -> &'static str {
        "layout/hello-world"
    }

    fn title(&self) -> &'static str {
        "Hello World"
    }

    fn category(&self) -> Category {
        Category::Layout
    }

    fn build(&self, dom: &mut TuiDom) -> NodeId {
        let root = dom.create_element("div");
        dom.set_attribute(root, "class", "hello").unwrap();
        let h1 = dom.create_element("h1");
        let text = dom.create_text_node("Hello, rdom!");
        dom.append_child(h1, text).unwrap();
        dom.append_child(root, h1).unwrap();
        let p = dom.create_element("p");
        let pt = dom
            .create_text_node("If you can read this in a terminal, the showcase shell is mounted.");
        dom.append_child(p, pt).unwrap();
        dom.append_child(root, p).unwrap();
        root
    }

    fn stylesheet(&self) -> Stylesheet {
        Stylesheet::bare()
            .rule_unchecked(".hello", TuiStyle::new().padding(Padding::all(1)))
            .rule_unchecked(
                "h1",
                TuiStyle::new().fg(Color::Rgb(180, 220, 255)).bold(true),
            )
    }

    fn source(&self) -> Source {
        Source {
            markup: r#"<div class="hello">
  <h1>Hello, rdom!</h1>
  <p>If you can read this in a terminal, the showcase shell is mounted.</p>
</div>"#,
            css: r#".hello { padding: 1; }
h1 { color: rgb(180, 220, 255); font-weight: bold; }"#,
        }
    }
}
