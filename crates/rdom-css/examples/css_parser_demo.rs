//! All three string-CSS surfaces composed: a `<style>` block in
//! the template, inline `style="…"` attributes on individual
//! elements, and a `:root { --var: … }` block referenced via
//! `var(--name)` in author rules.
//!
//! The demo parses an HTML-ish template via `rdom-parser`, then
//! calls `rdom_css::extend_from_style_tags` and
//! `rdom_css::seed_inline_styles` before handing the populated
//! `Dom<TuiExt>` to `App::run`. From there the cascade resolves
//! var references, applies `!important` precedence, and paints.
//!
//! Press Ctrl-C to exit.
//!
//! Run with: `cargo run --example css_parser_demo -p rdom-css`

use std::io;

use rdom_parser::parse_into;
use rdom_tui::prelude::*;
use rdom_tui::{extend_from_style_tags, seed_inline_styles};

const TEMPLATE: &str = r#"
<screen>
  <style>
    :root {
      --accent: #3d90ce;
      --ink: #d0d0d0;
      --muted: #707070;
    }

    title {
      color: var(--accent);
      font-weight: bold;
      height: 1;
    }

    hint {
      color: var(--muted);
      height: 1;
    }

    card {
      display: block;
      border: rounded;
      border-color: var(--accent);
      padding: 1 2;
      gap: 1;
    }

    label {
      color: var(--ink);
      font-weight: bold;
    }

    note {
      color: var(--muted);
    }
  </style>

  <title>CSS parser demo — &lt;style&gt; blocks, inline style, var()</title>
  <hint>Author CSS comes from the &lt;style&gt; block above. Inline style overrides where used. Ctrl-C to exit.</hint>

  <card>
    <label>Author CSS</label>
    <note>This whole card is styled by the &lt;style&gt; block — color, padding, border.</note>
  </card>

  <card style="border-color: red">
    <label>Inline override</label>
    <note>The card's border-color is set inline; it beats the author rule for var(--accent).</note>
  </card>

  <card>
    <label style="color: red !important">Inline !important</label>
    <note>The label's color is forced red even though the author rule said --ink. !important wins.</note>
  </card>
</screen>
"#;

fn main() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    parse_into(&mut dom, TEMPLATE, root).expect("template parses");

    let mut sheet = Stylesheet::new();
    let style_warnings = extend_from_style_tags(&dom, &mut sheet);
    let inline_warnings = seed_inline_styles(&mut dom);

    if !style_warnings.is_empty() {
        eprintln!("warnings from <style> blocks: {style_warnings:?}");
    }
    if !inline_warnings.is_empty() {
        eprintln!("warnings from inline style: {inline_warnings:?}");
    }

    // Author-side touch-ups for the demo's `<screen>` host.
    let sheet = sheet
        .rule(
            "screen",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Column)
                .width(Size::Flex(1))
                .height(Size::Flex(1))
                .padding(Padding::symmetric(2, 1))
                .gap(1),
        )
        .unwrap();

    App::new(dom, sheet)?.run()
}
