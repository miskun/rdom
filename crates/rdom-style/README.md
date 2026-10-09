# rdom-style

CSS data model + property dispatch for [rdom](https://github.com/miskun/rdom).
The value types that [`rdom-css`](../rdom-css/) parses *into* and that
[`rdom-tui`](../rdom-tui/) cascades, lays out, and paints *from*.

Leaf crate. Depends only on `rdom-core`. No backend, no runtime, no I/O.

You normally don't depend on `rdom-style` directly — `rdom-tui` re-exports
everything you'll touch (`TuiStyle`, `Stylesheet`, `Color`, `TuiColor`,
`Modifier`, `Padding`, `Border`, `Display`, `Direction`, `Size`,
`Position`, …) under its own crate name. Reach for `rdom-style` only if
you are:

- Building an alternate backend (a non-terminal renderer, a layout-only
  consumer) that wants the cascade data model without `rdom-tui`'s
  presentation code.
- Writing tooling that introspects properties (a linter, a devtools
  panel) and wants `property_dispatch`'s name → setter table directly.
- Writing your own parser front-end that produces `TuiStyle` values
  without `rdom-css`.

## Quick start

```rust
use rdom_style::{Stylesheet, TuiStyle, color::named, property_dispatch};

// Build a TuiStyle by hand.
let mut style = TuiStyle::new()
    .fg(named::RED)
    .bold(true);

// Or drive the property table by name (this is what
// rdom-css's parser and rdom-tui's StyleDeclaration both do).
property_dispatch::set("color", "#3d90ce", &mut style).expect("a valid color");
property_dispatch::set("font-weight", "bold", &mut style).expect("a valid weight");
property_dispatch::set("text-decoration", "underline wavy red", &mut style).expect("a decoration");

// A Stylesheet is a list of (selector, style) rules + a vars map.
let sheet = Stylesheet::new() // UA defaults baked in
    .rule(".hero", style)
    .expect("a valid selector")
    .define_var("accent", "#3d90ce");
```

## What's in the box

The leaf crate carries the **values**, not the cascade. Cascade lives in
`rdom-tui`.

| Module / type | Role |
|---|---|
| `TuiStyle` | Author-input style block. Every cascade rule writes here. Optional per-field — `None` means "not set; inherit / default". |
| `ComputedStyle` | Post-cascade snapshot. What layout and paint read. |
| `Stylesheet`, `Rule` | Rule collection + UA defaults + `var()` map. Built via `Stylesheet::new().rule(sel, style)`; `Stylesheet::bare()` skips UA defaults (tests). |
| `Color`, `TuiColor`, `Modifier` | Color + modifier primitives. `Color` is the concrete terminal color: `Reset` (the terminal default), `Indexed` (the 256-color palette), `Rgb` and `Rgba` (truecolor, with alpha); the 148 CSS named colors are constants in `color::named`. `TuiColor` is the declared form the cascade resolves: `Literal`, `CurrentColor`, `System` (system colors), `Function` (a color function that needs the element — `currentcolor` inside, `light-dark()`), and `Var` with its fallback chain. `ColorScheme` / `ColorSchemeList` carry `color-scheme`. |
| `Specificity`, `ImportantMask` | Cascade primitives — `(inline, id, class+attr+pc, type+pe)` lexicographic order; per-property `!important` bits. |
| `Value<T>` | A declared value: `Specified(T)` or a CSS-wide keyword (`Inherit`, `Initial`, `Revert`, `RevertLayer`; `unset` resolves when parsed). Every `TuiStyle` property field is an `Option<Value<T>>` — the per-side longhands (`margin`, `padding`, the border sides and corners) a `Sides` / `Corners` of them. |
| `property_dispatch` | The **single** `name → (setter, serializer, mask, remover)` table. Both `rdom-css` (parser) and `rdom-tui`'s `StyleDeclaration` consume this — there is no parallel list to drift. |
| `parse` | The tokenizer (`parse::token`), the source cursor (`parse::SourceCursor`) `rdom-css`'s block parser walks, and the value parsers (`parse::values`) `property_dispatch::set` calls. |
| `layout::*` | `Display`, `Direction`, `WhiteSpace`, `Size`, `Padding`, `Border`, `Position`, `Length`, `ZIndex`, `Overflow`, `LayoutRect`, … |
| `transition::*` | Transition declarations — `TimingFunction`, `TransitionProperty` (`all`, `none`, a property name, another ident), `TransitionRule`. |
| `animation::*` | Each longhand's animation type (`AnimationType`, `Longhand`, `animation_type`), the longhands a `transition-property` name covers, and the interpolation and addition of computed values a running transition or animation composites onto a `ComputedStyle`. |
| `keyframes::*` | `@keyframes` rules (`KeyframesRule`, `Keyframe`, resolved by offset) and the `animation-*` values (`AnimationName`, `AnimationDuration`, `IterationCount`, …). |

## Supported properties

`property_dispatch::property_names()` is the source of truth at runtime
(also driving `rdom-tui`'s `StyleDeclaration` camelCase aliases via
`build.rs`). The current set, by area — every name is in the generated
list at the end:

- **Color / text / interaction** — `color`, `background-color`, the
  `background` shorthand and its longhands (`background-image` /
  `-position` / `-size` / `-repeat` / `-attachment` / `-origin` /
  `-clip`; images parse but draw nothing), `border-color`, `opacity`,
  `color-scheme`, `caret-color`, `caret-text-color`, the `font` shorthand
  and `font-weight` / `font-style` (drawn bold / italic) with the inert
  `font-size` / `font-family` / `font-stretch` / `font-variant`, the
  `text-decoration` shorthand and its longhands (`-line`, `-style`,
  `-color`, `-thickness`), `text-underline-offset` /
  `-position`, `text-decoration-skip-ink`, `pointer-events`,
  `user-select`.
- **Block model** — `display` (the CSS Display 3 keywords: `contents`,
  `flow-root`, the two-keyword forms, `list-item`, `table` /
  `inline-table` and the table parts), `visibility`, `flex-direction` (+ `-reverse`), `flex-wrap`, `flex-flow`, `justify-content`, `align-content`, `align-items`, `align-self`, `justify-items`, `justify-self`, the `place-*` shorthands, `flex`, `flex-grow`,
  `flex-shrink`, `flex-basis`, `order`,
  `white-space` (with `white-space-collapse` / `text-wrap-mode`), `text-wrap`
  (with `text-wrap-style`), `text-align` (with `text-align-all` /
  `text-align-last`), `text-justify`, `text-indent`, `text-transform`,
  `tab-size`, `line-height`, `letter-spacing`, `word-spacing`,
  `vertical-align`, `word-break`,
  `overflow-wrap` / `word-wrap`, `line-break`, `hyphens`, `overflow` (one or two values, `clip` included), `overflow-x`, `overflow-y`,
  `overflow-block`, `overflow-inline`, `overflow-clip-margin`, `text-overflow`, `line-clamp`
  (`max-lines`, `block-ellipsis`, `continue`, the legacy `-webkit-line-clamp` /
  `-webkit-box-orient` / `display: -webkit-box`), `scrollbar-gutter`, `scrollbar-width`, `scrollbar-color`, `scroll-behavior`.
- **Sizing and box** — `width`, `height`, `min-width`, `max-width`,
  `min-height`, `max-height` (cells, `%`, `calc()`, `none` for `max-*`,
  and the intrinsic keywords `min-content` / `max-content` /
  `fit-content` / `fit-content()`, and `calc-size()` — on `width` / `height`,
  `min-*`, `max-*` and `flex-basis`),
  `interpolate-size`, `box-sizing` (initial `content-box`),
  `aspect-ratio`, `contain-intrinsic-size` (+ `-width` / `-height` /
  `-inline-size` / `-block-size`), `gap` (+ `row-gap` / `column-gap`), `padding` and
  `margin` (+ four longhands each, `margin: auto`), `margin-trim`, `border` and
  `border-top` / `-right` / `-bottom` / `-left` (width, style and color
  in any order: `border: 1px solid red`), `border-style`,
  `border-color` and `border-width` (1–4 values, + four per-side
  longhands each), `border-radius` (+ four per-corner longhands),
  `box-shadow`, `border-collapse`, `border-spacing`, `table-layout`,
  `caption-side`.
- **Grid** — `display: grid` / `inline-grid`, `grid-template-columns` /
  `-rows` / `-areas`, the `grid-template` and `grid` shorthands,
  `grid-auto-columns` / `-rows` / `-flow`, `grid-row` / `grid-column` (+
  `-start` / `-end`) and `grid-area`, `subgrid`, and the `place-*`
  alignment shorthands above.
- **Lists and tables** — `list-style` (+ `-type`, `-position`, `-image`),
  `marker-side`, `empty-cells`; the table properties are under sizing and
  box.
- **Scrolling** — `overscroll-behavior` (+ `-x` / `-y` / `-block` /
  `-inline`), `scroll-padding` and `scroll-margin` (+ the physical and
  logical longhands), `scroll-snap-type` / `-align` / `-stop`.
- **Containment and conditional rules** — `contain`, `content-visibility`,
  `container` (+ `-type`, `-name`), `will-change`; `@media`, `@supports` and
  `@container` conditions are `conditional::*`.
- **Transforms, filters, compositing and clipping** — `translate`,
  `transform` (translate functions move a box by whole cells; the others,
  `rotate`, `scale`, `transform-origin` and `transform-box` parse, cascade
  and animate, and draw nothing), `filter` and `backdrop-filter`,
  `mix-blend-mode`, `isolation`, `background-blend-mode` (inert),
  `clip-path` (`inset()`, `circle()`, `ellipse()`, `polygon()`), the legacy
  `clip: rect()`, and `mask` / `mask-border` with their longhands (parsed
  and kept; images draw nothing).
- **Multi-column layout and fragmentation** — `columns` (+
  `column-count`, `column-width`), `column-gap`, `column-rule` (+ `-color`,
  `-style`, `-width`), `column-span`, `column-fill`, `break-before` /
  `-after` / `-inside` and the legacy `page-break-*` aliases, `orphans`,
  `widows`, `box-decoration-break`.
- **Anchor positioning** — `anchor-name`, `anchor-scope`,
  `position-anchor`, `position-area`, `position-try` (+ `-fallbacks`,
  `-order`), `position-visibility`; `anchor()` / `anchor-size()` in insets,
  sizes and margins; `@position-try` rules are `PositionTryRule`s.
- **Writing modes and logical properties** — `direction`, `writing-mode`
  (horizontal; the vertical values parse and lay out horizontally), and
  the flow-relative forms, mapped for the element's `direction`:
  `inline-size` / `block-size` (+ `min-` / `max-`), `margin-inline` /
  `-block`, `padding-inline` / `-block`, `inset-inline` / `-block` (+
  `-start` / `-end` longhands each), `border-inline` / `-block` (+
  `-start` / `-end`, and `-color` / `-style` / `-width` of each), and
  `border-start-start-radius` and the other three corners.
- **Generated content** — `content` (strings, `attr()`, `counter()` /
  `counters()` in every predefined counter style or `symbols()`, quotes,
  alt text), `quotes`, `counter-reset` (with `reversed()`),
  `counter-increment`, `counter-set`; `@counter-style` rules are
  `counters::CounterStyleRule`s.
- **Positioning** — `position` (incl. `sticky`), `top`, `right`,
  `bottom`, `left`, `inset`, `z-index`, `float`, `clear`, `overlay`.
- **User interface** — `outline` (+ `-color`, `-style`, `-width`,
  `-offset`), `cursor` (keywords, and `url()` images with a keyword
  fallback), `caret` (+ `caret-shape`, `caret-animation`; `caret-color`
  above), `accent-color`, `appearance` (and `-webkit-appearance`),
  `field-sizing`, `resize`.
- **Transitions** — `transition` (+ `-property`, `-duration`,
  `-timing-function` — keywords, `cubic-bezier()`, `steps()`, `linear()` —
  `-delay`, negative included, `-behavior` longhands); `@starting-style`
  rules (parsed by rdom-css) give a newly rendered element its starting
  style.
- **Animations** — `animation` (+ `-name`, `-duration`, `-timing-function`,
  `-delay`, `-iteration-count`, `-direction`, `-fill-mode`, `-play-state`,
  `-composition`, `-timeline` longhands); `@keyframes` rules are
  `keyframes::KeyframesRule`s.
- **Scroll-driven animations** — `scroll-timeline` (+ `-name`, `-axis`),
  `view-timeline` (+ `-name`, `-axis`, `-inset`), `timeline-scope`,
  `animation-timeline`'s `scroll()` / `view()` / named timelines, and
  `animation-range` (+ `-start`, `-end`).
- **Custom properties** — `--*`, and `all`.

<details><summary>Every property name the dispatch table knows (generated from <code>property_names()</code>)</summary>

<!-- property-names:begin -->
`-webkit-appearance`, `-webkit-box-orient`, `-webkit-line-clamp`, `accent-color`, `align-content`, `align-items`, `align-self`, `anchor-name`, `anchor-scope`, `animation`, `animation-composition`, `animation-delay`, `animation-direction`, `animation-duration`, `animation-fill-mode`, `animation-iteration-count`, `animation-name`, `animation-play-state`, `animation-range`, `animation-range-end`, `animation-range-start`, `animation-timeline`, `animation-timing-function`, `appearance`, `aspect-ratio`, `backdrop-filter`, `background`, `background-attachment`, `background-blend-mode`, `background-clip`, `background-color`, `background-image`, `background-origin`, `background-position`, `background-repeat`, `background-size`, `block-ellipsis`, `block-size`, `border`, `border-block`, `border-block-color`, `border-block-end`, `border-block-end-color`, `border-block-end-style`, `border-block-end-width`, `border-block-start`, `border-block-start-color`, `border-block-start-style`, `border-block-start-width`, `border-block-style`, `border-block-width`, `border-bottom`, `border-bottom-color`, `border-bottom-left-radius`, `border-bottom-right-radius`, `border-bottom-style`, `border-bottom-width`, `border-collapse`, `border-color`, `border-end-end-radius`, `border-end-start-radius`, `border-inline`, `border-inline-color`, `border-inline-end`, `border-inline-end-color`, `border-inline-end-style`, `border-inline-end-width`, `border-inline-start`, `border-inline-start-color`, `border-inline-start-style`, `border-inline-start-width`, `border-inline-style`, `border-inline-width`, `border-left`, `border-left-color`, `border-left-style`, `border-left-width`, `border-radius`, `border-right`, `border-right-color`, `border-right-style`, `border-right-width`, `border-spacing`, `border-start-end-radius`, `border-start-start-radius`, `border-style`, `border-top`, `border-top-color`, `border-top-left-radius`, `border-top-right-radius`, `border-top-style`, `border-top-width`, `border-width`, `bottom`, `box-decoration-break`, `box-shadow`, `box-sizing`, `break-after`, `break-before`, `break-inside`, `caption-side`, `caret`, `caret-animation`, `caret-color`, `caret-shape`, `caret-text-color`, `clear`, `clip`, `clip-path`, `color`, `color-scheme`, `column-count`, `column-fill`, `column-gap`, `column-rule`, `column-rule-color`, `column-rule-style`, `column-rule-width`, `column-span`, `column-width`, `columns`, `contain`, `contain-intrinsic-block-size`, `contain-intrinsic-height`, `contain-intrinsic-inline-size`, `contain-intrinsic-size`, `contain-intrinsic-width`, `container`, `container-name`, `container-type`, `content`, `content-visibility`, `continue`, `counter-increment`, `counter-reset`, `counter-set`, `cursor`, `direction`, `display`, `empty-cells`, `field-sizing`, `filter`, `flex`, `flex-basis`, `flex-direction`, `flex-flow`, `flex-grow`, `flex-shrink`, `flex-wrap`, `float`, `font`, `font-family`, `font-size`, `font-stretch`, `font-style`, `font-variant`, `font-weight`, `font-width`, `gap`, `grid`, `grid-area`, `grid-auto-columns`, `grid-auto-flow`, `grid-auto-rows`, `grid-column`, `grid-column-end`, `grid-column-start`, `grid-row`, `grid-row-end`, `grid-row-start`, `grid-template`, `grid-template-areas`, `grid-template-columns`, `grid-template-rows`, `height`, `hyphens`, `inline-size`, `inset`, `inset-block`, `inset-block-end`, `inset-block-start`, `inset-inline`, `inset-inline-end`, `inset-inline-start`, `interpolate-size`, `isolation`, `justify-content`, `justify-items`, `justify-self`, `left`, `letter-spacing`, `line-break`, `line-clamp`, `line-height`, `list-style`, `list-style-image`, `list-style-position`, `list-style-type`, `margin`, `margin-block`, `margin-block-end`, `margin-block-start`, `margin-bottom`, `margin-inline`, `margin-inline-end`, `margin-inline-start`, `margin-left`, `margin-right`, `margin-top`, `margin-trim`, `marker-side`, `mask`, `mask-border`, `mask-border-mode`, `mask-border-outset`, `mask-border-repeat`, `mask-border-slice`, `mask-border-source`, `mask-border-width`, `mask-clip`, `mask-composite`, `mask-image`, `mask-mode`, `mask-origin`, `mask-position`, `mask-repeat`, `mask-size`, `mask-type`, `max-block-size`, `max-height`, `max-inline-size`, `max-lines`, `max-width`, `min-block-size`, `min-height`, `min-inline-size`, `min-width`, `mix-blend-mode`, `opacity`, `order`, `orphans`, `outline`, `outline-color`, `outline-offset`, `outline-style`, `outline-width`, `overflow`, `overflow-block`, `overflow-clip-margin`, `overflow-inline`, `overflow-wrap`, `overflow-x`, `overflow-y`, `overlay`, `overscroll-behavior`, `overscroll-behavior-block`, `overscroll-behavior-inline`, `overscroll-behavior-x`, `overscroll-behavior-y`, `padding`, `padding-block`, `padding-block-end`, `padding-block-start`, `padding-bottom`, `padding-inline`, `padding-inline-end`, `padding-inline-start`, `padding-left`, `padding-right`, `padding-top`, `page-break-after`, `page-break-before`, `page-break-inside`, `place-content`, `place-items`, `place-self`, `pointer-events`, `position`, `position-anchor`, `position-area`, `position-try`, `position-try-fallbacks`, `position-try-order`, `position-visibility`, `quotes`, `resize`, `right`, `rotate`, `row-gap`, `scale`, `scroll-behavior`, `scroll-margin`, `scroll-margin-block`, `scroll-margin-block-end`, `scroll-margin-block-start`, `scroll-margin-bottom`, `scroll-margin-inline`, `scroll-margin-inline-end`, `scroll-margin-inline-start`, `scroll-margin-left`, `scroll-margin-right`, `scroll-margin-top`, `scroll-padding`, `scroll-padding-block`, `scroll-padding-block-end`, `scroll-padding-block-start`, `scroll-padding-bottom`, `scroll-padding-inline`, `scroll-padding-inline-end`, `scroll-padding-inline-start`, `scroll-padding-left`, `scroll-padding-right`, `scroll-padding-top`, `scroll-snap-align`, `scroll-snap-stop`, `scroll-snap-type`, `scroll-timeline`, `scroll-timeline-axis`, `scroll-timeline-name`, `scrollbar-color`, `scrollbar-gutter`, `scrollbar-width`, `tab-size`, `table-layout`, `text-align`, `text-align-all`, `text-align-last`, `text-decoration`, `text-decoration-color`, `text-decoration-line`, `text-decoration-skip-ink`, `text-decoration-style`, `text-decoration-thickness`, `text-indent`, `text-justify`, `text-overflow`, `text-transform`, `text-underline-offset`, `text-underline-position`, `text-wrap`, `text-wrap-mode`, `text-wrap-style`, `timeline-scope`, `top`, `transform`, `transform-box`, `transform-origin`, `transition`, `transition-behavior`, `transition-delay`, `transition-duration`, `transition-property`, `transition-timing-function`, `translate`, `user-select`, `vertical-align`, `view-timeline`, `view-timeline-axis`, `view-timeline-inset`, `view-timeline-name`, `visibility`, `white-space`, `white-space-collapse`, `widows`, `width`, `will-change`, `word-break`, `word-spacing`, `word-wrap`, `writing-mode`, `z-index`
<!-- property-names:end -->

</details>

See [`DESIGN.md`](../../specs/DESIGN.md#roadmap) for what's coming next.

## Why a leaf crate

Pre-refactor, `rdom-css` depended on `rdom-tui` for `TuiStyle`, which
transitively pulled in cascade, layout, paint, runtime, and crossterm.
That violated rdom's "the parser must not require a runtime or a
backend" rule.

Extracting the data model into this leaf inverts the dep direction:

```text
                  rdom-core         (pure DOM substrate)
                      ↑
                  rdom-style        (this crate — data model)
                    ↑   ↑
              rdom-css  rdom-tui    (parser and renderer; siblings)
```

`rdom-css` and `rdom-tui` are now independent consumers of the same
property table. Adding a property means editing one file in `rdom-style`
and the parser + serializer + CSSOM aliases pick it up automatically.

## Custom properties and `var()`

`Stylesheet::define_var` registers a custom property usable in any
`<color>`-typed property. The resolution rules live in `rdom-tui`'s
cascade; the *data model* — `TuiColor::Var { name, fallback }`,
`VarMap`, the cascade primitive — lives here.

```rust
use rdom_style::{Color, StyleError, Stylesheet, TuiColor, TuiStyle};

fn main() -> Result<(), StyleError> {
    let white = TuiColor::Literal(Color::Rgb(255, 255, 255));
    let sheet = Stylesheet::new()
        .define_var("accent", "#3d90ce")
        .rule(".primary", TuiStyle::new().fg_var("accent"))?
        .rule(".fallback", TuiStyle::new().fg(TuiColor::var_with("missing", white)))?;
    Ok(())
}
```

Custom properties (`TuiStyle::custom_properties`) are declared under any
selector and scoped per element by the cascade. `var()` works in every
property (CSS Variables 1 §3): the dispatch table keeps a declaration
holding it as tokens (`TuiStyle::pending`), and a backend's cascade
(through the hooks of `rdom_style::backend`) substitutes it from an element's custom properties
(`TuiStyle::substituted`), parses it with the property's grammar, and
makes it `unset` when that fails. The builder's typed
`TuiColor::Var` / `Content::Var` remain for Rust-built styles. The CSS-wide
keywords `inherit`, `initial`, `unset`, `revert` and `revert-layer` are accepted for every property, and
`all` sets one of them on every property at once.

## Pointers

- [`DESIGN.md`](../../specs/DESIGN.md) — architectural overview.
- [`DIVERGENCES.md`](../../specs/DIVERGENCES.md) — every deliberate departure from the web platform.
- [`rdom-css`](../rdom-css/) — the CSS string parser that consumes this crate.
- [`rdom-tui`](../rdom-tui/) — the cascade + layout + paint consumer.

## Testing

```text
cargo test -p rdom-style
```

About 700 unit tests covering color parsing, modifier composition, `Specificity`
ordering, `ImportantMask` routing, every `property_dispatch::set` /
`serialize` / `remove` path, length parsing, and `transition` value
parsing.
