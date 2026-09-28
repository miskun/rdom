//! Lists and generated content — list markers on block-content items
//! and inline `::before` / `::after`.
//!
//! An `<ol>` item whose content is a `<p>` still gets its marker on the
//! paragraph's first line, as in a browser, numbered by the `list-item`
//! counter; a second paragraph in the same item gets none. rdom's marker
//! is the item's `::before`, placed inside the line rather than hung in
//! the list's padding (DIVERGENCES §Layout). Inline
//! `::before` / `::after` put generated text around a phrase and a
//! badge without touching the tree.

use std::io;

use rdom_parser::parse_into;
use rdom_tui::{App, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="lists-gen">
  <ol class="steps">
    <li><p>Paragraph inside the first item.</p></li>
    <li><p>First paragraph of the second item.</p><p>Its second paragraph has no marker.</p></li>
    <li>A plain third item.</li>
  </ol>
  <ul class="bullets">
    <li><p>A bulleted paragraph.</p></li>
  </ul>
  <p>Quoted: <span class="quote">generated quotes</span></p>
  <p>Build <span class="badge">ok</span> <span class="badge warn">2 warnings</span></p>
</div>"#;

pub const CSS: &str = r#"
.lists-gen {
  padding: 1 2;
}
.lists-gen .quote::before {
  content: "“";
  color: rgb(120, 170, 255);
}
.lists-gen .quote::after {
  content: "”";
  color: rgb(120, 170, 255);
}
.lists-gen .badge::before {
  content: "[";
}
.lists-gen .badge::after {
  content: "]";
}
.lists-gen .badge {
  color: rgb(140, 220, 140);
}
.lists-gen .badge.warn {
  color: rgb(255, 200, 80);
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let host = dom.create_element("div");
    parse_into(dom, MARKUP, host).expect("template parses");
    host
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

pub struct ListsGenerated;

impl Demo for ListsGenerated {
    fn slug(&self) -> &'static str {
        "pseudo-elements/lists-generated"
    }

    fn title(&self) -> &'static str {
        "Lists + generated content"
    }

    fn category(&self) -> Category {
        Category::PseudoElements
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
