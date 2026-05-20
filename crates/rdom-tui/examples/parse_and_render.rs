//! All three crates composing: parse an HTML-ish template with
//! `rdom-parser`, hand the populated `Dom<TuiExt>` to `App::run`,
//! which does cascade + layout + paint on every dirty frame.
//!
//! Useful as a "does the whole pipeline still work end-to-end?"
//! sanity check. Press Ctrl-C to exit.
//!
//! Run with: `cargo run --example parse_and_render -p rdom-tui`

use std::io;

use rdom_parser::parse_into;
use rdom_tui::prelude::*;

const TEMPLATE: &str = r#"
<app>
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

fn main() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    parse_into(&mut dom, TEMPLATE, root).expect("template parses");

    let sheet = Stylesheet::new()
        .define_var("accent", "#3d90ce")
        .define_var("ink", "#d0d0d0")
        .define_var("muted", "#808080")
        .rule(
            "app",
            TuiStyle::new()
                .direction(Direction::Column)
                .width(Size::Flex(1))
                .height(Size::Flex(1)),
        )
        .unwrap()
        .rule(
            "title",
            TuiStyle::new()
                .fg_var("accent")
                .bold(true)
                .padding(Padding::symmetric(1, 0))
                .height(Size::Fixed(2))
                .border(Border::Bottom)
                .border_fg_var("accent"),
        )
        .unwrap()
        .rule(
            "body",
            TuiStyle::new()
                .direction(Direction::Row)
                .gap(2)
                .padding(Padding::all(1))
                .height(Size::Flex(1)),
        )
        .unwrap()
        .rule(
            ".card",
            TuiStyle::new()
                .width(Size::Flex(1))
                .border(Border::Rounded)
                .padding(Padding::all(1))
                .fg_var("ink")
                .border_fg_var("muted"),
        )
        .unwrap()
        .rule(".card.accent", TuiStyle::new().border_fg_var("accent"))
        .unwrap()
        .rule(
            "h",
            TuiStyle::new()
                .fg_var("accent")
                .bold(true)
                .height(Size::Fixed(1)),
        )
        .unwrap()
        // `p` is Auto height — grows to fit wrapped inline content.
        .rule("p", TuiStyle::new())
        .unwrap()
        .rule(
            "footer",
            TuiStyle::new()
                .fg_var("muted")
                .height(Size::Fixed(1))
                .padding(Padding::symmetric(1, 0)),
        )
        .unwrap();

    App::new(dom, sheet)?.run()
}
