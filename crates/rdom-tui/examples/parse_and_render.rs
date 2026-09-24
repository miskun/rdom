//! All three crates composing: `rdom-parser` builds the tree from an
//! HTML-ish template, `rdom-css` parses the stylesheet, `rdom-tui`
//! cascades, lays out and paints.
//!
//! Run: `cargo run -p rdom-tui --example parse_and_render`
//!
//! Self-contained on purpose: this file is the whole program. The
//! browsable version lives in `rdom-showcase` ("Built-ins → Parse +
//! render").

use std::io;

use rdom_parser::parse_into;
use rdom_tui::{App, TuiDom};

const MARKUP: &str = r#"
<div class="par-demo">
  <header>rdom: parse → cascade → render</header>
  <main>
    <section class="card">
      <h2>Three crates, one pipeline</h2>
      <p>Template parsed by <code>rdom-parser</code>.</p>
      <p>Cascaded + laid out + painted by <code>rdom-tui</code>.</p>
      <p>Using <code>rdom-core</code> underneath.</p>
    </section>
    <section class="card accent">
      <h2>Features shown</h2>
      <p>• HTML-ish templates, character references (&amp; &copy;)</p>
      <p>• CSS cascade with <code>var()</code>, pseudo-elements</p>
      <p>• Flexbox <em>and</em> <b>inline</b> layout</p>
      <p>• Unicode: 中文 🦀 👨‍👩‍👧</p>
    </section>
  </main>
  <footer>Ctrl-C to exit</footer>
</div>
"#;

const CSS: &str = r#"
.par-demo {
  --accent: #3d90ce;
  --ink: #d0d0d0;
  --muted: #808080;
  flex: 1;
  display: flex;
  flex-direction: column;
}
.par-demo header {
  color: var(--accent);
  font-weight: bold;
  padding: 0 1;
  height: 2;
  border-bottom: solid;
  border-color: var(--accent);
}
.par-demo main {
  display: flex;
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
  overflow-y: hidden;
}
.par-demo .card.accent {
  border-color: var(--accent);
}
.par-demo h2 {
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

fn main() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    parse_into(&mut dom, MARKUP, root).expect("template parses");
    App::new(dom, rdom_css::from_css(CSS))?.run()
}
