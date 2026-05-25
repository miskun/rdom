//! Headings — `<h1>` through `<h6>` rendered with UA defaults
//! (bold accent), plus a per-demo override that overrides the
//! cascade for `<h1>` only.
//!
//! Exercises the UA stylesheet's heading rules + selector
//! specificity: `h1`–`h6` all bold via UA; the demo's
//! `.headings h1 { color: ... }` adds a color without disturbing
//! the bold modifier from UA — demonstrating that cascade layers
//! merge rather than overwrite.

use std::io;

use rdom_tui::{App, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="headings">
  <h1>Heading 1 — primary</h1>
  <h2>Heading 2 — secondary</h2>
  <h3>Heading 3 — tertiary</h3>
  <h4>Heading 4</h4>
  <h5>Heading 5</h5>
  <h6>Heading 6</h6>
  <p>Body text under the headings shows the contrast: paragraphs use the default fg, headings are bold (UA) with the demo-level color override on h1.</p>
</div>"#;

pub const CSS: &str = r#"
.headings {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 1 2;
  gap: 0;
}
.headings h1 {
  color: rgb(180, 220, 255);
}
.headings h2 {
  color: rgb(160, 200, 240);
}
.headings h3 {
  color: rgb(140, 180, 220);
}
.headings p {
  padding-top: 1;
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "headings").unwrap();

    for (tag, label) in [
        ("h1", "Heading 1 — primary"),
        ("h2", "Heading 2 — secondary"),
        ("h3", "Heading 3 — tertiary"),
        ("h4", "Heading 4"),
        ("h5", "Heading 5"),
        ("h6", "Heading 6"),
    ] {
        let h = dom.create_element(tag);
        let t = dom.create_text_node(label);
        dom.append_child(h, t).unwrap();
        dom.append_child(root, h).unwrap();
    }

    let p = dom.create_element("p");
    let pt = dom.create_text_node(
        "Body text under the headings shows the contrast: paragraphs use the default fg, headings are bold (UA) with the demo-level color override on h1.",
    );
    dom.append_child(p, pt).unwrap();
    // SUB-2 workaround: trailing empty <span> so the <p> is IFC.
    let tail = dom.create_element("span");
    dom.append_child(p, tail).unwrap();
    dom.append_child(root, p).unwrap();

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

pub struct Headings;

impl Demo for Headings {
    fn slug(&self) -> &'static str {
        "text/headings"
    }

    fn title(&self) -> &'static str {
        "Headings"
    }

    fn category(&self) -> Category {
        Category::Text
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
