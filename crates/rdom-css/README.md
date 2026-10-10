# rdom-css

CSS string parser for [rdom](https://github.com/miskun/rdom). Turns real
CSS source — standalone stylesheets, `<style>` blocks in templates, or
inline `style="…"` attributes — into [`Stylesheet`] and [`TuiStyle`]
values consumed by [`rdom-tui`](../rdom-tui/)'s cascade.

Hand-rolled. Zero external parser dependencies (no `cssparser`, no
`lightningcss`). Depends only on `rdom-core` and `rdom-style`.

## Quick start

```rust,no_run
use rdom_css::from_css;
use rdom_tui::prelude::*;

fn main() -> std::io::Result<()> {
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
    let dom: TuiDom = TuiDom::new();
    // ... build the tree ...
    App::new(dom, sheet)?.run()
}
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
| Standalone stylesheet string | `from_css(s)` / `parse(s)` / `parse_strict(s)` | Full rule list, custom-property declarations under any selector, `var()` in any property. |
| `<style>…</style>` in a template | automatic under `rdom_tui::App` (live: re-parsed when the text changes, dropped when removed; warnings from `App::style_element_warnings`); `rdom_tui::extend_from_style_tags(&dom, &mut sheet)` for a snapshot without an `App` | Finds every `<style>` element, feeds its text content through `parse`. |
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
  (`[lang]`, `[lang="en"]`, `~=`, `|=`, `^=`, `$=`, `*=`, the `i` / `s` case
  flags), pseudo-classes
  (`:hover`, `:active`, `:focus`, `:not(...)`, `:first-child`, `:last-child`,
  `:only-child`, `:nth-child(An+B [of S])`, `:nth-last-child()`, `:nth-of-type()`,
  `:nth-last-of-type()`, `:first-of-type`, `:last-of-type`, `:only-of-type`,
  `:empty`, `:root`, `:link`, `:any-link`, `:visited` (never matches),
  `:lang()`, `:dir()`, `:has()`, `:checked`, `:indeterminate`,
  `:open`, `:is(...)`, `:where(...)`, …), pseudo-elements (`::before`, `::after`, `::marker`,
  `::first-line`, `::first-letter`, `::selection`, `::highlight(name)`, `::details-content`, `::backdrop`,
  the nested `::before::marker` / `::after::marker`; the CSS 2.1 spellings
  `:before` / `:after` / `:first-line` / `:first-letter`; `::before` / `::after` / `::marker` / `::first-letter` followed by
  `:hover` / `:active`), descendant / child / next-sibling / subsequent-sibling
  combinators, comma-separated lists.
- **Properties** — the `rdom-style::property_dispatch` table (`property_names()` lists them; incl. `counter-reset` (with `reversed()`) / `counter-increment` / `counter-set`, `content` (`counter()`, `counters()`, `symbols()`, quotes, alt text) and `quotes`; `transition-timing-function` takes `cubic-bezier()` and `steps()`):
  colour and text, the box model, flexbox, grid, tables, lists and generated content, positioning, containment, transitions and animations, transforms, filters, blending, clipping, multi-column layout and anchor positioning.
  See [`rdom-style`](../rdom-style/#supported-properties) for the
  current list.
- **Values** — colors: the CSS Color 4 / 5 `<color>` grammar — hex
  (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`), the 148 named colors,
  `transparent`, `currentcolor`, the system colors (`Canvas`,
  `CanvasText`, …), `rgb()` / `rgba()` and `hsl()` / `hsla()` in the
  modern and legacy syntax, `hwb()`, `lab()` / `lch()` / `oklab()` /
  `oklch()`, `color()`, `color-mix()`, relative colors (`rgb(from …)`)
  and `light-dark()`, plus rdom's `reset` (the terminal default) —
  and `color-scheme`; lengths (cells, `fr`,
  `auto`, `%`, `ch`, the viewport units `vw` / `vh` / `vmin` / `vmax` / …),
  fractional numbers, angles, math functions (`calc()`, `min()`, `max()`,
  `clamp()`, `round()`, `mod()`, `rem()`, `abs()`, `sign()`, the
  trigonometric and exponential functions), `var(--name)` and
  `var(--name, fallback)` and `attr(name type(<syntax>), fallback)` in any
  property (see *Custom properties* below), modifiers (`bold`, `italic`,
  `underline`), shorthands (4-/3-/2-/1-value `padding`), comma-separated
  `transition` lists.
- **Custom properties** — `--name: value;` under any selector (and in a
  `style` attribute) rides on the rule as `TuiStyle::custom_properties`;
  the cascade scopes it per element and inherits it — `:root`'s from the
  document root, the root element (`Stylesheet::vars` holds only what
  `define_var` puts there, beneath every rule). `var()` works in every
  property (CSS Variables 1 §3): a declaration holding it is kept as
  tokens (`TuiStyle::pending`) and the cascade substitutes and parses it
  per element, fallbacks with arbitrary tokens included.
- **CSS Nesting** — style rules nest inside style rules
  (`.card { color: red; &:hover { … } > p { … } }`): `&` anywhere in a
  selector (`&.x`, `.x &`, `:not(&)`), an implicit descendant combinator
  when `&` is absent, relative selectors (`> p`, `+ p`, `~ p`),
  declarations interleaved with nested rules (a later run becomes a
  nested declarations rule, in order), and nested `@layer`. `&` has the
  specificity of `:is(<parent list>)`.
- **`!important`** — recognized on any declaration; routed to the
  property's `ImportantMask` bit. Cascade ladder lives in `rdom-tui`.
- **Comments** — `/* … */`, nested or unterminated handled with
  warnings.
- **Whitespace** — CSS-faithful (whitespace required between adjacent
  identifiers, optional around `:`, `;`, `{`, `}`).
- **UTF-8** — identifiers, strings, comments all UTF-8 throughout.

## At-rules

Parsed into the sheet: `@import` (through the host's `ImportLoader` with
`parse_with_loader` / `parse_with_loader_at` — relative URLs resolved by
the loader against the importing sheet — its `supports()` and media list
conditioning the imported rules), `@layer` (statement and block forms,
anonymous and nested layers), `@scope` (`Stylesheet::scopes`,
`Rule::scope`), `@media`, `@supports` and `@container` (the sheet's
conditions, `Stylesheet::conditions`, `Rule::condition` — at the top
level, in `@layer` and nested in a style rule; `@supports` evaluated as it
is parsed, `rdom_style::supports_condition` being `CSS.supports()`,
`@media` and `@container` by the backend's cascade), `@property`
(`Stylesheet::registered_properties`), `@counter-style`
(`Stylesheet::counter_styles`), `@keyframes` (`Stylesheet::keyframes`),
`@starting-style`, and `@position-try` (`Stylesheet::position_try_rules`,
CSS Anchor Positioning 1 §4.1).

A rule that defines nothing, or a declaration it drops, is reported with
a typed warning: `InvalidAtRulePrelude` for an invalid prelude,
`InvalidCounterStyleRule` / `CounterStyleDescriptorDropped`,
`InvalidKeyframeSelector` / `ImportantInKeyframe`, and
`PositionTryDescriptorDropped` for a declaration that is no
`@position-try` descriptor or is `!important`.

Every other at-rule (`@charset`, `@font-face`, `@page`, …) is consumed
whole per CSS Syntax 3 §5.4.2 and reported with
`WarningKind::UnsupportedAtRule(name)`; the rules around it are
unaffected — matching browser behavior, so copy-pasting CSS from MDN
doesn't blow up.

## Lengths

Lengths are terminal cells: a bare number is cells, and `fr`, `%`, `ch`,
`lh` / `rlh`, the viewport units (`vw`, `vh`, `vmin`, …, of the terminal)
and the container units (`cqw`, …) are supported. Pixel and
font-relative units (`px`, `em`, `rem` and the other absolute units) have
no cell-grid meaning, so they **never become geometry**: a declaration
that would size, place or space a box with one (`width: 10px`, `gap:
1em`) is dropped as `InvalidValue`. They are taken where they only
*select* a discrete option: a border's glyph weight (`border: 1px
solid`), a `@media` / `@container` feature value and `column-width` /
`columns` (8px a column, 16px = 1em a row, so `@media (min-width: 768px)`
applies from 96 columns).

## Lenient vs. strict

Two parallel APIs at every surface. Lenient is the default — both for
top-level stylesheets and inline attributes — because copy-pasted CSS
from real-world stylesheets always has *something* unsupported in it,
and you want the rest to still apply.

```rust
fn main() -> Result<(), rdom_css::ParseError> {
    let source = ".hero { color: red }";
    let inline = "color: red; padding: 1";

    // Lenient — Warnings collect; the rest of the parse continues.
    let result = rdom_css::parse(source); // stylesheet + warnings
    let result_i = rdom_css::parse_inline(inline); // style + warnings
    let sheet = rdom_css::from_css(source); // Stylesheet, warnings dropped

    // Strict — the first Warning is returned as a ParseError instead.
    let sheet = rdom_css::parse_strict(source)?;
    let style = rdom_css::parse_inline_strict(inline)?;
    let sheet = rdom_css::from_css_strict(source)?;
    Ok(())
}
```

## Warnings

```rust
use rdom_css::WarningKind;

// `WarningKind` is `#[non_exhaustive]`: match what you report and keep
// a catch-all arm.
fn describe(kind: &WarningKind) -> String {
    match kind {
        WarningKind::UnknownProperty(name) => format!("unknown property {name}"),
        WarningKind::InvalidValue { property, value } => format!("{property}: bad value {value}"),
        WarningKind::MalformedDeclaration(text) => format!("not `name: value`: {text}"),
        WarningKind::UnsupportedAtRule(name) => format!("@{name} is not supported"),
        WarningKind::InvalidAtRulePrelude { name, prelude } => format!("@{name} {prelude}"),
        WarningKind::InvalidSelector(selector) => format!("bad selector {selector}"),
        WarningKind::ImportIgnored(url)
        | WarningKind::ImportCycle(url)
        | WarningKind::ImportTooDeep(url) => format!("@import {url} skipped"),
        WarningKind::ImportFailed { url, reason } => format!("@import {url}: {reason}"),
        WarningKind::InvalidPropertyRule { name, reason } => format!("@property {name}: {reason}"),
        WarningKind::UnterminatedComment => "unterminated comment".into(),
        WarningKind::UnterminatedString => "unterminated string".into(),
        _ => "other".into(),
    }
}

let result = rdom_css::parse(".x { font-weight: ultraviolet }");
assert_eq!(describe(&result.warnings[0].kind), "font-weight: bad value ultraviolet");
```

The parser is bounded against hostile depth as against hostile values:
blocks — style rules and the block at-rules, counted together — nest at
most `rdom_css::MAX_BLOCK_DEPTH` (32) deep, and a deeper block is
skipped whole with `WarningKind::BlockTooDeep`; a conditional prelude
nests parentheses at most 32 deep, and a selector's arguments 32 deep
(`rdom_core::selectors::MAX_SELECTOR_NESTING`), its `&` expansions to
4096 simple selectors. No input overflows the stack.

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
use rdom_tui::{Color, Padding, TuiStyle};

fn main() -> Result<(), rdom_css::ParseError> {
    let from_builder = TuiStyle::new()
        .fg(Color::Rgb(255, 0, 0))
        .padding(Padding::all(1));

    let from_css = rdom_css::parse_inline_strict("color: red; padding: 1")?;

    assert_eq!(from_builder, from_css);
    Ok(())
}
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
colors, function tokens), selector integration, per-property parsing, `padding` shorthand
forms, color values (every `<color>` form, `var()` chains), custom
properties at `:root`, `!important` routing, length parsing, lenient
vs strict mode, `<style>` block extraction, and the
`parse_inline` ↔ `from_css` consistency tests.
