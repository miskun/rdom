//! Whitespace modes — four side-by-side columns demonstrating
//! `white-space: normal | pre | pre-wrap | nowrap`.
//!
//! Each column holds the same source text — a sentence with
//! collapsible runs of spaces and a literal newline — but with a
//! different `white-space` value. The CSS spec says:
//!
//! - `normal`   — collapse runs to one space, drop newlines, wrap.
//! - `pre`      — preserve runs + newlines, do NOT wrap.
//! - `pre-wrap` — preserve runs + newlines, DO wrap.
//! - `nowrap`   — collapse like normal, but do NOT wrap.
//!
//! The packer (`crates/rdom-tui/src/render/inline/packer.rs`)
//! reads `white_space` from the IFC block's `ComputedStyle` and
//! switches between the four modes.

use std::io;

use rdom_tui::{App, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

const SAMPLE: &str = "first    line with    runs of    spaces\nsecond line after a literal newline";

pub const MARKUP: &str = r#"<div class="ws-demo">
  <h1>white-space modes</h1>
  <div class="row">
    <div class="col">
      <h2>normal</h2>
      <p class="ws-normal">first    line with    runs of    spaces
second line after a literal newline<span></span></p>
    </div>
    <div class="col">
      <h2>pre</h2>
      <p class="ws-pre">first    line with    runs of    spaces
second line after a literal newline<span></span></p>
    </div>
    <div class="col">
      <h2>pre-wrap</h2>
      <p class="ws-pre-wrap">first    line with    runs of    spaces
second line after a literal newline<span></span></p>
    </div>
    <div class="col">
      <h2>nowrap</h2>
      <p class="ws-nowrap">first    line with    runs of    spaces
second line after a literal newline<span></span></p>
    </div>
  </div>
</div>"#;

pub const CSS: &str = r#"
.ws-demo {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 1 2;
  gap: 1;
}
.ws-demo h1 {
  color: rgb(180, 220, 255);
}
.ws-demo .row {
  flex: 1;
  display: flex;
  flex-direction: row;
  gap: 2;
}
.ws-demo .col {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 1;
  overflow-x: hidden;
}
.ws-demo .col h2 {
  color: rgb(180, 200, 220);
}
.ws-demo .ws-normal {
  white-space: normal;
}
.ws-demo .ws-pre {
  white-space: pre;
}
.ws-demo .ws-pre-wrap {
  white-space: pre-wrap;
}
.ws-demo .ws-nowrap {
  white-space: nowrap;
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "ws-demo").unwrap();

    let h1 = dom.create_element("h1");
    let h1_t = dom.create_text_node("white-space modes");
    dom.append_child(h1, h1_t).unwrap();
    dom.append_child(root, h1).unwrap();

    let row = dom.create_element("div");
    dom.set_attribute(row, "class", "row").unwrap();

    for (label, ws_class) in [
        ("normal", "ws-normal"),
        ("pre", "ws-pre"),
        ("pre-wrap", "ws-pre-wrap"),
        ("nowrap", "ws-nowrap"),
    ] {
        let col = dom.create_element("div");
        dom.set_attribute(col, "class", "col").unwrap();

        let h2 = dom.create_element("h2");
        let h2_t = dom.create_text_node(label);
        dom.append_child(h2, h2_t).unwrap();
        dom.append_child(col, h2).unwrap();

        let p = dom.create_element("p");
        dom.set_attribute(p, "class", ws_class).unwrap();
        let t = dom.create_text_node(SAMPLE);
        dom.append_child(p, t).unwrap();
        // SUB-2 workaround so `<p>` establishes an IFC; the
        // packer reads `white_space` from the IFC block's
        // computed style, so the per-class override only takes
        // effect when the `<p>` is IFC-laid.
        let tail = dom.create_element("span");
        dom.append_child(p, tail).unwrap();
        dom.append_child(col, p).unwrap();

        dom.append_child(row, col).unwrap();
    }

    dom.append_child(root, row).unwrap();
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

pub struct WhitespaceModes;

impl Demo for WhitespaceModes {
    fn slug(&self) -> &'static str {
        "text/whitespace"
    }

    fn title(&self) -> &'static str {
        "white-space modes"
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
