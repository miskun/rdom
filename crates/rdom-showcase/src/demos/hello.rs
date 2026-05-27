//! Hello World — the simplest possible demo. A `<div>` with text
//! inside. Acts as the M2 scaffold's "is the shell actually
//! working" canary.
//!
//! Authoring pattern (followed by every demo): one `const MARKUP`,
//! one `const CSS`. `build` constructs the DOM matching MARKUP;
//! `stylesheet` parses CSS via `rdom_css::from_css`; `source`
//! exposes both strings to the Source view (M7). Single source of
//! truth per demo — no drift between the runtime tree and what the
//! Source tab shows.

use rdom_tui::{NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

const MARKUP: &str = r#"<div class="hello">
  <h1>Hello, rdom!</h1>
  <p>If you can read this in a terminal, the showcase shell is mounted.</p>
</div>"#;

const CSS: &str = r#"
.hello {
  padding: 1 2;
}
.hello h1 {
  color: rgb(180, 220, 255);
  font-weight: bold;
}
"#;

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
        rdom_css::from_css(CSS)
    }

    fn source(&self) -> Source {
        Source {
            markup: MARKUP,
            css: CSS,
        }
    }
}
