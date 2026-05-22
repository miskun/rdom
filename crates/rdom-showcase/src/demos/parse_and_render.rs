//! All three crates composing: `rdom-parser` builds the tree from
//! an HTML-ish template, `rdom-css` parses the stylesheet, and
//! `rdom-tui` cascades + lays out + paints.
//!
//! Useful as a "does the whole pipeline still work end-to-end?"
//! sanity check.

use std::io;

use rdom_parser::parse_into;
use rdom_tui::{App, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"
<app class="par-demo">
  <title>rdom: parse → cascade → render</title>
  <body>
    <section class="card">
      <h>Three crates, one pipeline</h>
      <p>Template parsed by <code>rdom-parser</code>.</p>
      <p>Cascaded + laid out + painted by <code>rdom-tui</code>.</p>
      <p>Using <code>rdom-core</code> underneath.</p>
    </section>
    <section class="card accent">
      <h>Features shown</h>
      <p>• HTML-ish templates, entity decoding</p>
      <p>• CSS cascade with <code>var()</code>, pseudo-elements</p>
      <p>• Flexbox <em>and</em> <b>inline</b> layout</p>
      <p>• Unicode: 中文 🦀 👨‍👩‍👧</p>
    </section>
  </body>
  <footer>Ctrl-C to exit</footer>
</app>
"#;

pub const CSS: &str = r#"
.par-demo {
  --accent: #3d90ce;
  --ink: #d0d0d0;
  --muted: #808080;
  flex: 1;
  flex-direction: column;
}
.par-demo title {
  color: var(--accent);
  font-weight: bold;
  padding: 0 1;
  height: 2;
  border-bottom: solid;
  border-color: var(--accent);
}
.par-demo body {
  flex-direction: row;
  gap: 2;
  padding: 1;
  flex: 1;
}
.par-demo .card {
  flex: 1;
  border: rounded;
  padding: 1;
  color: var(--ink);
  border-color: var(--muted);
}
.par-demo .card.accent {
  border-color: var(--accent);
}
.par-demo h {
  color: var(--accent);
  font-weight: bold;
  height: 1;
}
.par-demo footer {
  color: var(--muted);
  height: 1;
  padding: 0 1;
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    // Wrap the parsed tree in a class-scoped container so the
    // demo's CSS doesn't bleed onto other demos via the bare
    // `app` / `body` / `section` selectors. The class lives on
    // the parsed `<app>` element directly via the `class`
    // attribute in MARKUP.
    let host = dom.create_element("div");
    dom.set_attribute(host, "class", "par-demo-host").unwrap();
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

pub struct ParseAndRender;

impl Demo for ParseAndRender {
    fn slug(&self) -> &'static str {
        "builtins/parse-and-render"
    }

    fn title(&self) -> &'static str {
        "Parse + render"
    }

    fn category(&self) -> Category {
        Category::BuiltIns
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
