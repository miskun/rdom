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
| `Value` | The raw token-tree value produced by `parse::Cursor`. What property setters consume. |
| `property_dispatch` | The **single** `name → (setter, serializer, mask, remover)` table. Both `rdom-css` (parser) and `rdom-tui`'s `StyleDeclaration` consume this — there is no parallel list to drift. |
| `parse::Cursor` | Tokenizer + cursor used by `property_dispatch::set` and re-exported for `rdom-css`'s block parser. |
| `layout::*` | `Display`, `Direction`, `WhiteSpace`, `Size`, `Padding`, `Border`, `Position`, `Length`, `ZIndex`, `Overflow`, `LayoutRect`, … |
| `transition::*` | Animation type system — `AnimatableProperty`, `TimingFunction`, `TransitionProperty`, `TransitionRule`. |

## Supported properties

`property_dispatch::property_names()` is the source of truth at runtime
(also driving `rdom-tui`'s `StyleDeclaration` camelCase aliases via
`build.rs`). The current set:

- **Color / text / interaction** — `color`, `background-color`, the
  `background` shorthand and its longhands (`background-image` /
  `-position` / `-size` / `-repeat` / `-attachment` / `-origin` /
  `-clip`; images parse but draw nothing), `border-color`, `opacity`,
  `color-scheme`, `caret-color`, `caret-text-color`, `font-weight`,
  `font-style`, `text-decoration`, `pointer-events`, `user-select`.
- **Block model** — `display`, `flex-direction`, `flex`, `flex-shrink`,
  `white-space`, `overflow`, `overflow-x`, `overflow-y`,
  `scrollbar-gutter`, `scroll-behavior`.
- **Sizing and box** — `width`, `height`, `min-width`, `max-width`,
  `min-height`, `max-height`, `aspect-ratio`, `gap`, `padding` and
  `margin` (+ four longhands each, `margin: auto`), `border`,
  `border-top` / `-right` / `-bottom` / `-left`, `border-style` (+ four
  longhands), `border-collapse`.
- **Generated content** — `content`, `counter-reset`,
  `counter-increment`.
- **Positioning** — `position` (incl. `sticky`), `top`, `right`,
  `bottom`, `left`, `inset`, `z-index`.
- **Transitions** — `transition` (+ `-property`, `-duration`,
  `-timing-function`, `-delay` longhands).
- **Custom properties** — `--*`, and `all`.

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

250+ tests covering color parsing, modifier composition, `Specificity`
ordering, `ImportantMask` routing, every `property_dispatch::set` /
`serialize` / `remove` path, length parsing, and `transition` value
parsing.
