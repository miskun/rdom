# rdom-css

CSS string parser for [rdom](https://github.com/miskun/rdom). Turns real
CSS source — standalone stylesheets, `<style>` blocks in templates, or
inline `style="…"` attributes — into [`Stylesheet`] and [`TuiStyle`]
values consumed by [`rdom-tui`](../rdom-tui/)'s cascade.

Hand-rolled. Zero external parser dependencies (no `cssparser`, no
`lightningcss`). Depends only on `rdom-core` and `rdom-style`.

## Quick start

```rust
use rdom_css::from_css;
use rdom_tui::prelude::*;

// One-shot: parse a CSS string into a Stylesheet that already has
// the UA defaults baked in. Unknown properties become silent warnings.
let sheet = from_css(r#"
    :root {
        --accent: #3d90ce;
    }

    .hero {
        color: var(--accent);
        font-weight: bold;
        padding: 1 2;
        border: solid;
    }

    button:hover {
        background-color: lightgray;
    }
"#);

// Build a tree, attach the sheet, render in a terminal.
let mut dom: TuiDom = TuiDom::new();
// ... build the tree ...
App::new(dom, sheet)?.run()
```

For warnings-aware parsing, call `parse` (lenient) or `parse_strict`
(first warning is returned as `ParseError`):

```rust
use rdom_css::parse;

let result = parse(".hero { font-weight: ultraviolet }");
assert_eq!(result.warnings.len(), 1);
// WarningKind::InvalidValue { property: "font-weight", value: "ultraviolet" }
```

## Three CSS surfaces, one parser

All three call the same tokenizer + property dispatch — there is no
parallel grammar.

| Surface | Entry point | Notes |
|---|---|---|
| Standalone stylesheet string | `from_css(s)` / `parse(s)` / `parse_strict(s)` | Full rule list, custom-property declarations under `:root { --name: value }`, `<color>` `var()` references. |
| `<style>…</style>` in a template | `rdom_tui::cssom::apply::extend_from_style_tags(&mut sheet, &dom)` | Walks the parsed `Dom`, finds every `<style>` element, feeds its text content through `parse`, appends to the sheet. |
| Inline `style="…"` attribute | `parse_inline(s)` / `parse_inline_strict(s)` | Declaration list (no selectors, no braces). Returns a `TuiStyle` and any warnings. Drives `style="…"` attribute writes via `rdom-tui`'s `StyleDeclaration` and the `InlineStyleObserver`. |

## Supported grammar

```text
stylesheet  := (comment | at-rule | rule)*
rule        := selector-list '{' decl-list '}'
selector-list := selector (',' selector)*
decl-list   := (decl ';')* decl?
decl        := identifier ':' value ('!' 'important')?
value       := token+
```

- **Selectors** — full coverage of `rdom-core`'s selector engine. Type
  (`div`, `h1`), universal (`*`), id (`#app`), class (`.hero`), attribute
  (`[lang]`, `[lang="en"]`, `~=`, `|=`, `^=`, `$=`, `*=`), pseudo-classes
  (`:hover`, `:focus`, `:not(...)`, `:first-child`, `:last-child`,
  `:only-child`, `:empty`, `:root`, `:checked`, `:indeterminate`,
  `:open`, …), pseudo-elements (`::before`, `::after`, `::selection`,
  `::backdrop`), descendant / child / next-sibling / subsequent-sibling
  combinators, comma-separated lists.
- **Properties** — the 32-name `rdom-style::property_dispatch` table:
  color/text, block model, sizing, content, positioning, transitions.
  See [`rdom-style`](../rdom-style/#supported-properties) for the
  current list.
- **Values** — colors (`#rgb`, `#rrggbb`, `#rrggbbaa` (alpha dropped),
  `rgb()`, `rgba()`, named colors, `reset`), lengths (cells, `fr`,
  `auto`), `var(--name)` and `var(--name, fallback)` in color positions,
  modifiers (`bold`, `italic`, `underline`), shorthands (4-/3-/2-/1-value
  `padding`), comma-separated `transition` lists.
- **Custom properties** — `--name: value;` declarations under `:root`
  populate the `Stylesheet::vars` map. M1 ships custom properties for
  `<color>` values only; generalization to `padding: var(--gap)` is
  follow-up work.
- **`!important`** — recognized on any declaration; routed to the
  property's `ImportantMask` bit. Cascade ladder lives in `rdom-tui`.
- **Comments** — `/* … */`, nested or unterminated handled with
  warnings.
- **Whitespace** — CSS-faithful (whitespace required between adjacent
  identifiers, optional around `:`, `;`, `{`, `}`).
- **UTF-8** — identifiers, strings, comments all UTF-8 throughout.

## Not yet supported

These produce a `Warning` and the parse continues — matching browser
behavior, so copy-pasting CSS from MDN doesn't blow up:

- **At-rules.** `@media`, `@import`, `@keyframes`, `@supports`,
  `@font-face`. Tokens recognized; the rule body is skipped with
  `WarningKind::UnsupportedAtRule(name)`. `@keyframes` and `@media`
  are flagged for future milestones.
- **`calc()` / `min()` / `max()`.** Reserved for M5 (layout primitives).
- **Length units other than cells and `fr`.** `px`, `em`, `rem`, `%` —
  M5.
- **CSS variables in non-color values.** `padding: var(--gap)` — M5.
- **CSS Nesting** (`.parent { .child { … } }`). Modern CSS feature; not
  in M1.
- **`&` parent reference.** Same.
- **Margin shorthand and the longhands.** No `margin` property in M1.
  Use `padding` on the parent for the equivalent visual effect, or
  wait for M5.

## Lenient vs. strict

Two parallel APIs at every surface. Lenient is the default — both for
top-level stylesheets and inline attributes — because copy-pasted CSS
from real-world stylesheets always has *something* unsupported in it,
and you want the rest to still apply.

```rust
// Lenient — Warnings collect; the rest of the parse continues.
let result   = rdom_css::parse(source);
let result_i = rdom_css::parse_inline(source);
let sheet    = rdom_css::from_css(source);             // Stylesheet, warnings dropped

// Strict — first Warning is returned as ParseError instead.
let sheet    = rdom_css::parse_strict(source)?;
let style    = rdom_css::parse_inline_strict(source)?;
let sheet    = rdom_css::from_css_strict(source)?;
```

## Warnings

```rust
pub enum WarningKind {
    UnknownProperty(String),
    InvalidValue { property: String, value: String },
    UnsupportedAtRule(String),
    InvalidSelector(String),
    UnterminatedComment,
    UnterminatedString,
}
```

Each `Warning` carries the kind plus line + column. `ParseError`
(returned by the strict APIs) maps the warning into a smaller
`ParseErrorKind` enum suitable for fixed terminal error reporting.

## Round-tripping with the builder

Rules constructed via the `Stylesheet::new().rule(sel, style)` fluent
builder and rules parsed from a CSS source produce the same `TuiStyle`.
This is verified in `rdom-tui`'s `cssom::tests` round-trip suite — a
representative set of declarations parsed from CSS and constructed
through the builder hash-compare equal after cascade.

```rust
let from_builder = TuiStyle::new()
    .fg(Color::Red)
    .padding(Padding::all(1));

let from_css = rdom_css::parse_inline_strict("color: red; padding: 1")?;

assert_eq!(from_builder, from_css);
```

## Pointers

- [`DESIGN.md`](../../specs/DESIGN.md) — architectural overview.
- [`DIVERGENCES.md`](../../specs/DIVERGENCES.md) — every deliberate departure from the web platform (selectors, at-rules, value-system simplifications).
- [`rdom-style`](../rdom-style/) — the data model this crate parses into.
- [`rdom-tui`](../rdom-tui/) — the cascade + layout + paint consumer.

## Testing

```text
cargo test -p rdom-css
```

Covers tokenizer (comments, whitespace, identifiers, strings, hex
colors, function tokens), selector integration, per-property parsing
(one test per row in `RDOM_CSS_PARSER.md` §5), `padding` shorthand
forms, color values (hex / `rgb()` / named / `var()` chains), custom
properties at `:root`, `!important` routing, length parsing, lenient
vs strict mode, `<style>` block extraction, and the
`parse_inline` ↔ `from_css` consistency tests.
