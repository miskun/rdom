//! Translucency — group `opacity` over text and borders.
//!
//! Two absolutely positioned cards sit over a paragraph and a bordered
//! box. The outer card is `opacity: 0.7`; the inner card, nested inside
//! it, is `opacity: 0.5` of its own, so it shows at 0.35 overall. Each
//! card composes as a group (CSS Color 4 §3.2: the subtree renders
//! opaque, then blends once), so the inner card's text never shows the
//! outer card's through it. The cards' padding keeps their text off the
//! backdrop's text: what shows through each card is the paragraph and the
//! bordered box, tinted toward the card's background. Where glyphs would
//! overlap, a terminal cell can show only one (DIVERGENCES §opacity).

use std::io;

use rdom_parser::parse_into;
use rdom_tui::{App, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="translucency">
  <p class="backdrop">The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog.</p>
  <div class="framed">A bordered box under the cards</div>
  <div class="card outer">
    <p>Outer card, opacity 0.7</p>
    <div class="card inner"><p>Inner card, 0.5 of 0.7</p></div>
  </div>
</div>"#;

pub const CSS: &str = r#"
.translucency {
  position: relative;
  padding: 1 2;
  height: 17;
}
.translucency .backdrop {
  color: rgb(200, 200, 200);
  width: 44;
}
.translucency .framed {
  border: rounded;
  border-color: rgb(255, 200, 80);
  width: 40;
  height: 3;
  margin-top: 1;
}
.translucency .card {
  position: absolute;
  border: rounded;
  padding: 1;
}
.translucency .outer {
  top: 2;
  left: 10;
  width: 34;
  height: 14;
  opacity: 0.7;
  background: rgb(30, 60, 120);
  border-color: rgb(120, 170, 255);
  color: rgb(230, 240, 255);
}
.translucency .inner {
  top: 6;
  left: 6;
  width: 26;
  height: 5;
  opacity: 0.5;
  background: rgb(120, 30, 90);
  border-color: rgb(255, 140, 200);
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

pub struct Translucency;

impl Demo for Translucency {
    fn slug(&self) -> &'static str {
        "cascade/translucency"
    }

    fn title(&self) -> &'static str {
        "Translucency"
    }

    fn category(&self) -> Category {
        Category::Cascade
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
