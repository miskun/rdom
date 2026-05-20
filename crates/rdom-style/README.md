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
use rdom_style::{Stylesheet, TuiStyle, Color, property_dispatch};

// Build a TuiStyle by hand.
let mut style = TuiStyle::new()
    .fg(Color::Red)
    .bold(true);

// Or drive the property table by name (this is what
// rdom-css's parser and rdom-tui's StyleDeclaration both do).
property_dispatch::set("color", "#3d90ce", &mut style)?;
property_dispatch::set("font-weight", "bold", &mut style)?;

// A Stylesheet is a list of (selector, style) rules + a vars map.
let sheet = Stylesheet::new()                         // UA defaults baked in
    .rule(".hero", style)?
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
| `Color`, `TuiColor`, `Modifier` | Color + modifier primitives. `Color` is the concrete terminal palette (named, indexed, truecolor). `TuiColor` is the unresolved cascade form (`Literal`, `Var`, fallback chain). |
| `Specificity`, `ImportantMask` | Cascade primitives — `(inline, id, class+attr+pc, type+pe)` lexicographic order; per-property `!important` bits. |
| `Value` | The raw token-tree value produced by `parse::Cursor`. What property setters consume. |
| `property_dispatch` | The **single** `name → (setter, serializer, mask, remover)` table. Both `rdom-css` (parser) and `rdom-tui`'s `StyleDeclaration` consume this — there is no parallel list to drift. |
| `parse::Cursor` | Tokenizer + cursor used by `property_dispatch::set` and re-exported for `rdom-css`'s block parser. |
| `layout::*` | `Display`, `Direction`, `WhiteSpace`, `Size`, `Padding`, `Border`, `Position`, `Length`, `ZIndex`, `Overflow`, `LayoutRect`, … |
| `transition::*` | Animation type system — `AnimatableProperty`, `TimingFunction`, `TransitionProperty`, `TransitionRule`. |

## Supported properties

`property_dispatch::property_names()` is the source of truth at runtime
(also driving `rdom-tui`'s `StyleDeclaration` camelCase aliases via
`build.rs`). The current set covers the M1–M3 milestones:

- **Color / text** — `color`, `background-color`, `border-color`,
  `font-weight`, `font-style`, `text-decoration`.
- **Block model** — `display`, `flex-direction`, `white-space`,
  `user-select`, `overflow`, `overflow-x`, `overflow-y`.
- **Sizing** — `width`, `height`, `gap`, `padding` (+ four longhands),
  `border`.
- **Generated content** — `content`.
- **Positioning** — `position`, `top`, `right`, `bottom`, `left`,
  `z-index`, `inset`.
- **Transitions** — `transition` (+ `-property`, `-duration`,
  `-timing-function`, `-delay` longhands).

- **Layout primitives** — `margin` (+ longhands + `auto`), `min-width`,
  `max-width`, `min-height`, `max-height`, `aspect-ratio`,
  `border-collapse`, plus `display: inline-block` and
  `position: sticky`.

See [`DESIGN.md`](../../specs/DESIGN.md#roadmap) for what's coming next
(`calc()` lands in 0.2.0).

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
use rdom_style::{Stylesheet, TuiStyle, TuiColor, Color};

let sheet = Stylesheet::new()
    .define_var("accent", "#3d90ce")
    .rule(".primary", TuiStyle::new().fg_var("accent"))?
    .rule(".fallback", TuiStyle::new().fg(TuiColor::var_with(
        "missing", TuiColor::Literal(Color::White))))?;
```

Custom properties are supported in `<color>` values only. Generalization
to other property types (`padding: var(--gap)`) lands with the `calc()`
value system in 0.2.0.

## Pointers

- [`DESIGN.md`](../../specs/DESIGN.md) — architectural overview.
- [`DIVERGENCES.md`](../../specs/DIVERGENCES.md) — every deliberate departure from the web platform.
- [`rdom-css`](../rdom-css/) — the CSS string parser that consumes this crate.
- [`rdom-tui`](../rdom-tui/) — the cascade + layout + paint consumer.

## Testing

```text
cargo test -p rdom-style
```

157 tests covering color parsing, modifier composition, `Specificity`
ordering, `ImportantMask` routing, every `property_dispatch::set` /
`serialize` / `remove` path, length parsing, and `transition` value
parsing.
