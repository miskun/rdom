# CSS-COVERAGE — what CSS rdom supports, against the spec

**Status:** AUDIT (2026-10-03, `main` at `6e4443f`). Read-only inventory, made to decide what to
implement before the acid test ([`ACID.md`](ACID.md)) is built. Nothing here is a commitment.

**Scope.** Standard (REC / CR / WD) properties, values, selectors and at-rules of the modules
listed in §3, judged for a character-cell terminal. Vendor-prefixed and browser-specific
features are out of scope. rdom-specific extensions are listed separately (§4).

**Method.** Every *Supported* / *Partial* claim is grounded in a parser path that was read:
the property list is `PROPERTY_NAMES` in `crates/rdom-style/src/property_dispatch/table.rs`
(70 names), the value grammars are the `match name` arms of `property_dispatch/set.rs` and the
`crates/rdom-style/src/parse/values/*.rs` parsers, selectors are `crates/rdom-core/src/selectors.rs`,
pseudo-elements are `crates/rdom-style/src/stylesheet/selector_text.rs::extract_pseudo_suffix`,
and at-rules are `crates/rdom-css/src/top_level.rs::consume_at_rule`. A property absent from
`fields_of` is rejected as `UnknownProperty`; a value the parser does not match is rejected as
`InvalidValue` and the declaration is dropped (CSS Syntax 3 behavior), so "Missing" means "the
declaration is dropped with a warning".

## Classes

| Class | Meaning |
|---|---|
| **Supported** | Every value that makes sense in a terminal grid works. |
| **Partial** | Parsed, but some values or forms are rejected or ignored — listed in the row. |
| **Missing** | Not parsed, and has a sensible terminal meaning (the row says what). |
| **N/A** | No meaning in a character-cell grid (one-line reason given). |

**Doc'd** column: `Yes` = `DIVERGENCES.md` describes the gap; `Blanket` = covered only by the
blanket "No at-rule is evaluated" entry; `No` = not mentioned; `Wrong` = `DIVERGENCES.md` says
something the code contradicts (see §6). `—` for Supported / N/A rows.

**Where** abbreviations (all paths under `crates/`):

| Key | Path |
|---|---|
| `DISP` | `rdom-style/src/property_dispatch/{table,set,serialize}.rs` — a new property needs a `PROPERTY_NAMES` entry, a `Field`, a `fields_of` arm, a `set_from_tokens` arm, a serializer |
| `TS` | `rdom-style/src/tui_style.rs` (storage field) + `rdom-style/src/computed.rs` |
| `KW` | `rdom-style/src/layout/keywords.rs` (keyword enums) |
| `BOX` | `rdom-style/src/layout/box_model.rs` / `layout/sizing.rs` |
| `V/<f>` | `rdom-style/src/parse/values/<f>.rs` |
| `COL` | `rdom-style/src/parse/values/color.rs` + `rdom-style/src/tui_color.rs` + `rdom-style/src/color/` |
| `CALC` | `rdom-style/src/calc.rs` + `parse/values/calc.rs` |
| `TOK` | `rdom-style/src/parse/token.rs` |
| `CASC` | `rdom-tui/src/style/cascade/` (`apply.rs`, `inherit.rs`, `content.rs`, `counters.rs`) |
| `FLEX` | `rdom-tui/src/render/layout_pass/flex/` (`main_axis.rs`, `lines.rs`, `content.rs`, `align.rs`, `cross.rs`, `placement.rs`) |
| `BLOCK` | `rdom-tui/src/render/layout_pass/block/` |
| `IFC` | `rdom-tui/src/render/layout_pass/ifc.rs` + `rdom-tui/src/render/inline/` |
| `POS` | `rdom-tui/src/render/layout_pass/positioning/` + `layout_pass/sticky.rs` + `render/stacking.rs` |
| `PAINT` | `rdom-tui/src/render/paint_pass/` (`background.rs`, `border/`, `border_join/`, `text.rs`, `scrollbar.rs`, `group.rs`) |
| `SGR` | `rdom-tui/src/render/sgr.rs` + `rdom-style/src/modifier.rs` |
| `SEL` | `rdom-core/src/selectors.rs` (+ matching in `rdom-core/src/query_selector.rs`) |
| `PE` | `rdom-style/src/stylesheet/selector_text.rs` + `stylesheet/mod.rs::PseudoElementTarget` |
| `AT` | `rdom-css/src/top_level.rs` |
| `DECL` | `rdom-css/src/declarations.rs` |
| `TR` | `rdom-style/src/parse/values/transition.rs` + `rdom-style/src/transition.rs` + `rdom-tui` transition runner |
| `UA` | `rdom-style/src/ua.rs` |
| `RT` | `rdom-tui/src/runtime/` |

**Sizes:** S = a parser arm plus a contained layout/paint change (hours to a day);
M = a new layout/paint behavior across a few modules (days); L = a new algorithm or
formatting context (a week or more).

---

## 1. Summary

Rows counted are the per-module table rows of §3 (a row is one property, property family, value function, unit family, selector, pseudo-element or at-rule; cross-references "See §…" are not counted twice).

| Module | Supported | Partial | Missing | N/A | Total |
|---|---:|---:|---:|---:|---:|
| 3.1 Syntax, cascade and inheritance (Syntax 3, Cascade 4/5, CSS 2.1 §6) | 18 | 1 | 0 | 2 | 21 |
| 3.2 Custom properties (CSS Variables 1) | 6 | 0 | 0 | 1 | 7 |
| 3.3 Values and units (Values 4) | 17 | 0 | 1 | 4 | 22 |
| 3.4 Color (Color 4 / 5) | 15 | 1 | 0 | 1 | 17 |
| 3.5 Backgrounds and borders (Backgrounds 3, Borders 4) | 13 | 1 | 0 | 2 | 16 |
| 3.6 Box model and sizing (Box 3, Sizing 3/4) | 7 | 2 | 0 | 0 | 9 |
| 3.7 Display and visibility (Display 3) | 8 | 0 | 1 | 2 | 11 |
| 3.8 Flexbox and box alignment (Flexbox 1, Align 3) | 17 | 0 | 0 | 0 | 17 |
| 3.9 Grid (Grid 1/2) | 9 | 0 | 1 | 0 | 10 |
| 3.10 Positioned layout (Position 3, CSS 2.1 §9) | 6 | 0 | 0 | 1 | 7 |
| 3.11 Overflow, scrolling and scrollbars (Overflow 3/4, Scroll Snap 1, Overscroll 1, Scrollbars 1) | 13 | 0 | 1 | 0 | 14 |
| 3.12 Inline text (Text 3/4, Inline 3, CSS 2.1 §10.8) | 16 | 0 | 0 | 5 | 21 |
| 3.13 Text decoration (Text Decoration 3/4) | 4 | 0 | 0 | 2 | 6 |
| 3.14 Fonts (Fonts 4) | 3 | 0 | 0 | 2 | 5 |
| 3.15 Lists, counters and generated content (Lists 3, Generated Content 3, Counter Styles 3, Pseudo-Elements 4) | 10 | 0 | 0 | 2 | 12 |
| 3.16 Pseudo-elements (Pseudo-Elements 4, Selectors 4) | 10 | 0 | 0 | 6 | 16 |
| 3.17 Selectors (Selectors 4) | 32 | 0 | 2 | 4 | 38 |
| 3.18 Transitions and animations (Transitions 1/2, Animations 1/2, Easing 1/2) | 4 | 2 | 4 | 0 | 10 |
| 3.19 User interface (UI 4) | 2 | 1 | 8 | 1 | 12 |
| 3.20 Tables (Tables 3, CSS 2.1 §17) | 0 | 0 | 4 | 0 | 4 |
| 3.21 Conditional rules and containment (Conditional 3/5, Contain 2/3, Will Change 1) | 0 | 0 | 6 | 1 | 7 |
| 3.22 Logical properties and writing modes (Logical 1, Writing Modes 4) | 5 | 2 | 1 | 1 | 9 |
| 3.23 Transforms, filters, masking, compositing | 0 | 0 | 6 | 4 | 10 |
| 3.24 Other modules (CSS 2.1 leftovers, Multi-column, Images, Speech, Fragmentation) | 0 | 0 | 2 | 4 | 6 |
| **Total** | **215** | **10** | **37** | **45** | **307** |

When audited, 191 rows were Partial / Missing and **123 of them were not documented** in `DIVERGENCES.md` (Doc'd `No` or `Wrong`; 5 rows `Wrong`, where the document stated the opposite of the code) — see §5 and §6. The Doc'd column is the audit's record: Phase 0 of CSS-COMPLETE-2026-10 has since listed every gap in `DIVERGENCES.md` §3. The counts above are today's (recounted after the Phase 2 gates, then updated per item): 48 rows Partial / Missing (Phase 11 part 2: C11-MODAL-POPOVER `:modal` and `:popover-open`; C11-FORM-STATES `:read-only` / `:read-write`, `:indeterminate`, `:in-range` / `:out-of-range`, `:default` and `:user-valid` / `:user-invalid`; Phase 11 part 1: C11-ATTR-FLAGS the attribute case flags, C11-NTH the three `:nth-*` / `-of-type` rows, C11-LINK-LANG `:link` / `:any-link`, `:lang()` and `:dir()`, C11-SCOPE `:scope`, C11-HAS `:has()`; C10G-DETAILS-CONTENT-BOX made `::details-content` a box, Supported, and C10G-PSEUDO-MARKER the pseudo-element-chain row, with `::before::marker` / `::after::marker`; Phase 10 part 2 shipped C10-FIRST's `::first-line` / `::first-letter` and C10-HIGHLIGHT's `::highlight()`, C10-DETAILS-CONTENT made `::details-content` Partial, and C10-PSEUDO-CHAINS took the pseudo-element-chain row's user-action pseudo-classes, the row staying Partial for nested pseudo-elements; part 1 shipped twelve: C10-LEGACY-COLON the single-colon pseudo-elements, C10-CONTENT `content` and `counters()`, C10-QUOTES `quotes`, C10-COUNTERS the counter styles, `counter-reset` and `counter-set`, C10-COUNTER-STYLE `@counter-style` / `symbols()`, C10-LIST-ITEM the list properties, `marker-side`, `::marker` and `display: list-item`; C9-FONT shipped `font-weight`, `font-style` and `font`; C9-DECORATION shipped the four text decoration rows; C9-VERTICAL-ALIGN shipped `vertical-align` on inline content; C9-LINE-HEIGHT shipped `line-height` and closed C2-LH's `lh` / `rlh`; Phase 9 part 1 shipped eleven — C9-WHITE-SPACE two, C9-BREAKING four, C9-TAB-SIZE, C9-TEXT-TRANSFORM, C9-TEXT-INDENT and C9-TEXT-WRAP one each, C9-TEXT-ALIGN three; C8G-PSEUDO-BOXES moved `::before` / `::after` from Supported to Partial — their `display` was ignored — and C8G-PSEUDO-ATOMS back: the atom, float and flex / grid forms work).

Headline: rdom parses **260 property names** (`property_names()`: 192 in the table and 68 flow-relative ones, after Phase 9 and C9G-LETTER-SPACING). The cascade, selectors, generated content, positioning, overflow and form-state pseudo-classes are strong. The gap a web developer hits first is `@media`.

---

## 2. Missing — applicable, by expected impact on a web developer

Ordered by how often a web developer reaches for the feature and how badly its absence breaks
pasted CSS. The first block is what a typical component stylesheet hits in its first ten lines.

| # | Item | Terminal semantics | Size | Doc'd |
|---|---|---|---|---|
| 1 | `justify-content` | Shipped (C6-JUSTIFY; §3.8): every value, per line, whole cells (the remainder to the first spaces). | M | No |
| 2 | `align-items` / `align-self` | Shipped (C6-ALIGN; §3.8): every value, baselines as the first / last content rows. | M | No |
| 3 | `flex-wrap` / `flex-flow` / `align-content` | Shipped (C6-WRAP, C6-ALIGN-CONTENT; §3.8): multi-line flex containers, their lines placed by `align-content`. | L | No |
| 4 | `flex-grow` / `flex-basis` longhands | Shipped (C6-FLEX-LONGHANDS; §3.8): both longhands, and the `flex-basis` the flex base size of Flexbox §9.2; §9.7 resolves the flexible lengths from it. | M | Wrong |
| 5 | `display: grid` + `grid-template-*`, `grid-auto-*`, `grid-row/column*`, `grid-area`, `grid` | A cell-quantized grid formatting context: track sizing in cells / `fr` / `%` / `auto` / `minmax()` / `repeat()`, line- and area-based placement. The most-used modern layout after flex. | L | Yes |
| 6 | `line-height` | Shipped (C9-LINE-HEIGHT; §3.12): whole-row line boxes, half-leading with the odd row below. | M | No |
| 7 | `text-align` (+ `text-align-last`, `text-justify`) | Shipped (C9-TEXT-ALIGN; §3.12): every value, the shorthand of `text-align-all` / `text-align-last`, justification in whole cells (the remainder to the first opportunities). | M | Yes |
| 8 | `visibility` | Shipped (C6-VISIBILITY; §3.7): `hidden` keeps the space and draws nothing, is not hit or focused, a `visible` descendant shows; `collapse` leaves a strut on flex items and removes table rows. | S | No |
| 9 | `box-sizing` | Shipped (C5-BOX-SIZING; §3.6): `content-box` is the initial value, `border-box` sizes the border box and floors it at padding + border; the CHANGELOG gives the `*, *::before, *::after { box-sizing: border-box }` migration. | M | No |
| 10 | `outline` (+ `-color`, `-style`, `-width`, `-offset`) | A border ring drawn one cell outside the border box, taking no layout space, painted over neighbors on the top layer; `outline-offset` in whole cells. The natural keyboard-focus ring a TUI otherwise lacks. | M | No |
| 11 | `overflow-wrap` / `word-break` | Shipped (C9-BREAKING; §3.12): `overflow-wrap: anywhere / break-word` and `word-break: break-all` break an over-long word at a grapheme boundary; `keep-all` for CJK; `line-break`. | M | No |
| 12 | `text-overflow` | `ellipsis`: the last visible cell of a clipped line becomes `…`; `<string>` form uses that string; applies with `overflow: hidden` + `white-space: nowrap`. | S | No | *Shipped: C8-TEXT-OVERFLOW.*
| 13 | Per-side border colors (`border-*-color`, multi-value `border-color`) | Shipped (C4-BORDER-SIDES; §3.5): each side's glyphs in its own color; a corner takes its dominant side's. | M | Yes |
| 14 | `currentColor` | Shipped (C3-CURRENTCOLOR; §3.4): the element's computed `color`, and `border-color`'s initial value; `outline-color` / `text-decoration-color` take it as their initial value when they land (C12-OUTLINE, C9-DECORATION). | S | Yes |
| 15 | `min()` / `max()` / `clamp()` | Comparison functions inside every `calc()` position; resolve at layout like percent-bearing `calc()`. | S | Yes |
| 16 | `hsl()` / `hwb()` / `lab()` / `lch()` / `oklab()` / `oklch()` / `color()` | Shipped (C3-HSL-HWB, C3-LAB; §3.4): converted to sRGB at parse time (gamut-mapped), emitted as truecolor `Color::Rgb` — `Color::Rgba` with an alpha below opaque. | S | No |
| 17 | `:nth-child()` / `:nth-last-child()` / `:nth-of-type()` / `:nth-last-of-type()` / `:first-of-type` / `:last-of-type` / `:only-of-type` | Shipped (C11-NTH; §3.17): the full An+B microsyntax, `of S`, a per-pass nth-index cache. | S | Partial — `:nth-child`, `:nth-of-type` Yes; the `*-of-type` trio No |
| 18 | `:is()` / `:has()` | Shipped: `:is()` (C1G-IS-PARSE), `:has()` with targeted invalidation (C11-HAS; §3.17). | S / M | Yes |
| 19 | `@media` | Evaluate `width` / `height` (in cells) / `orientation` / `aspect-ratio`, `color` / `monochrome`, `prefers-color-scheme` (from the terminal's reported background), `prefers-reduced-motion`, `hover` / `pointer`; re-cascade on `resize`. Also the `<style media>` attribute. | M | Yes |
| 20 | `order` | Shipped (C6-ORDER; §3.7): flex items lay out, paint and hit-test in order-modified document order; focus, selection and the DOM keep document order. Grid with C7. | S | No |
| 21 | `@keyframes` + `animation-*` | Keyframed animation on the existing transition clock and interpolators. | L | Yes |
| 22 | `display: contents` / `display: flow-root` / multi-keyword `display` | Shipped (C6-DISPLAY-KEYWORDS; §3.7): `contents` generates no box and its children and pseudo-elements join the parent's formatting context; `flow-root` establishes a BFC; the two-keyword syntax with `list-item`. | M / S / S | No |
| 23 | `text-transform` | Shipped (C9-TEXT-TRANSFORM; §3.12): `uppercase` / `lowercase` / `capitalize` at layout, `full-width` to the full-width forms (2 cells each), `full-size-kana`, `math-auto`; copy keeps the DOM text, as §2.1 requires and Gecko does — Blink and WebKit copy the transformed text (DIVERGENCES §1). | S | No |
| 24 | `text-indent` | Shipped (C9-TEXT-INDENT; §3.12): the first line of each block starts `n` cells in (negative outdents), `hanging` / `each-line` keywords. | S | No |
| 25 | `ch` / `lh` / `rlh` and viewport units `vw` / `vh` / `vmin` / `vmax` (+ `s`/`l`/`d` variants) | `1ch` = one column exactly on a monospaced grid; `1lh` = one row; `1vw` = 1% of the terminal's columns, `1vh` = 1% of its rows (rdom knows the viewport). | S | Yes (classified as pixel-dependent; see §6) |
| 26 | Logical properties (`inline-size`, `block-size`, `margin-inline*`, `padding-block*`, `inset-inline*`, `border-inline*`, …) | Shipped (C5-LOGICAL; §3.22): the block axis and sizes are their physical twins (horizontal-tb), the inline axis maps by `direction` (C5-WRITING). | S | Yes |
| 27 | `border-radius` (+ per-corner) | Shipped (C4-RADIUS; §3.5): any non-zero radius → rounded corner glyphs `╭╮╰╯` per corner; `0` → square. | S | Yes |
| 28 | `border-width` (+ per-side) and width component of `border` | Shipped (C4-BORDER-WIDTH; §3.5): `0` = no border on that side; `thin` / `medium` / `1` = light glyphs; `thick` / `≥2` / ≥5px = heavy box-drawing glyphs (`━┃┏┓┗┛`); never more than one cell. | S | Yes |
| 29 | `list-style-type` / `list-style-position` / `list-style` / `::marker` / `display: list-item` | Shipped (C10-LIST-ITEM; §3.15): a marker from a counter style or `<string>`; `outside` hangs it in the padding, `inside` puts it on the first line. | M | — |
| 30 | `::first-line` / `::first-letter` | Style the first line box / first typographic letter (color, bold, italic, decoration, background). | M | Yes |
| 31 | `var()` outside colors | Shipped (C1-VAR-ANY; §3.2): substitution in every property. Listed here because of its impact — design-token CSS uses `padding: var(--space-2)` everywhere. | M | Yes |
| 32 | CSS Nesting (`&`, nested rules, nested `@media`) | Nested style rules desugared to `:is(parent) child` at parse time. | M | Yes |
| 33 | `all` / `revert` / `revert-layer` | `all: unset` (or `revert`) resets every property (common "CSS reset" idiom); `revert` rolls back to the UA origin. | S | No |
| 34 | `@layer` | Cascade layers ordering author rules; anonymous / named / nested layers, `@layer a, b;` statements. | M | Blanket |
| 35 | `@supports` | Evaluate `(prop: value)` against the dispatch table, `selector()`, `not` / `and` / `or`. Lets pasted CSS degrade intentionally. | S | Blanket |
| 36 | `white-space: pre-line` / `break-spaces` (+ Text 4 `white-space-collapse`, `text-wrap-mode`) | Shipped (C9-WHITE-SPACE; §3.12): the shorthand of the two longhands, every value, per element, with the §4.1 collapsing, segment break and hanging rules. | S | No |
| 37 | `flex-direction: row-reverse / column-reverse` | Shipped (C6-DIRECTION-REVERSE; §3.8): main-start and main-end swap, with `direction`; a reversed scroll container scrolls from its main-start edge with a negative `scrollLeft` / `scrollTop`. | S | No |
| 38 | `row-gap` / `column-gap` / two-value `gap` | Shipped (C6-GAP; §3.8): per-axis gaps, `normal`, the two-value shorthand. | S | No |
| 39 | Color syntax completeness: `rgb()` space syntax / `%` channels / `/ alpha`; `color-mix()`; relative color syntax; system colors (`Canvas`, `CanvasText`, …); `light-dark()` + `color-scheme` | Shipped (C3-RGB, C3-MIX, C3-RELATIVE, C3-SYSTEM, C3-SCHEME; §3.4) — system colors map onto the terminal's default fg / bg and the UA palette; `light-dark()` picks by the terminal's reported background and follows its theme changes (mode 2031). | S–M | No |
| 40 | `:read-only` / `:read-write`, `:in-range` / `:out-of-range`, `:default`, `:user-valid` / `:user-invalid`, `:modal`, `:link` / `:any-link`, `:lang()`, `:scope`, `:popover-open` | Shipped (§3.17): `:link` / `:any-link`, `:lang()`, `:scope` (C11-LINK-LANG, C11-SCOPE), the form states (C11-FORM-STATES), `:modal` with a top layer and `:popover-open` with the `popover` attribute (C11-MODAL-POPOVER). | S each | Partial — `:read-*`, `:user-*`, `:modal` Yes; rest No |
| 41 | `float` / `clear` | Line-box exclusion beside a floated box; sidebars and drop-caps. Deliberately out of scope today. | L | Yes | *Shipped: C8-FLOAT.*
| 42 | `cursor` | OSC 22 pointer-shape request (`pointer`, `text`, `default`, `move`, resize shapes) on terminals that honor it (kitty, foot, ghostty, WezTerm); ignored elsewhere. | S | No |
| 43 | `accent-color` | Color of checkbox / radio / range / progress glyphs in the UA chrome. | S | No |
| 44 | `appearance` | `none` drops the UA control chrome (brackets, glyphs) so authors can restyle controls; `auto` restores it. | M | No |
| 45 | `caret-shape` / `caret-animation` / `caret` | `bar` / `block` / `underscore` for the painted caret (or DECSCUSR on the hardware cursor); `manual` disables blink. | S | No |
| 46 | `scrollbar-width` / `scrollbar-color` | `scrollbar-width: none` hides the bar while keeping the box scrollable (`thin` = `auto`, already one cell); `scrollbar-color: <thumb> <track>` = the standard spelling of `::scrollbar-thumb` / `::scrollbar` colors. | S | Yes |
| 47 | `overscroll-behavior` (+ `-x`, `-y`, logical) | `contain` / `none` stop wheel scroll chaining into the ancestor at the scroll limit (keyboard scrolling never chains: DIVERGENCES). | S | No | *Shipped: C8-OVERSCROLL.*
| 48 | `scroll-padding*` / `scroll-margin*` / `scroll-snap-type` / `scroll-snap-align` / `scroll-snap-stop` | Insets for `scrollIntoView` / keyboard scrolling; snap scroll offsets to item edges (row-snapped lists). | S / M | Partial — padding / margin Yes; snap No | *Shipped: C8-SCROLL-PADDING, C8-SNAP.*
| 49 | Table properties: `border-spacing`, `vertical-align` (cells), `table-layout`, `caption-side`, `empty-cells` | Cell gaps in cells; `middle` / `bottom` cell alignment; `fixed` = first-row widths only; caption above / below; hide empty cells' borders. | S each (`vertical-align` M) | Partial — `vertical-align` Yes; rest No |
| 50 | `vertical-align` (inline) | Shipped (C9-VERTICAL-ALIGN; §3.12): every value in whole rows, `sub` / `super` one row. | M | Yes |
| 51 | `text-decoration-line` / `-color` / `-style`, multi-line `text-decoration`, `overline` | Shipped (C9-DECORATION; §3.13): SGR 4:x / 58 / 53 by the terminal's capabilities. | S | No |
| 52 | `font-weight` numeric / `bolder` / `lighter`, `font-style: oblique`, `font` shorthand | Shipped (C9-FONT; §3.14): SGR 1 from 600, no faint; `oblique` → italic. | S | No |
| 53 | `quotes` + `open-quote` / `close-quote` in `content` | Quote marks for `<q>` and nested quotations, from the `quotes` pairs. | S | No |
| 54 | `counters()`, `counter-set`, `reversed()` in `counter-reset`, more counter styles, `@counter-style`, `symbols()` | Shipped (C10-CONTENT, C10-COUNTERS, C10-COUNTER-STYLE; §3.15). | S / M | — |
| 55 | `line-clamp` (`max-lines`, `block-ellipsis`, `continue`) | Clamp a block to N rows, last row ends in `…`. | M | No | *Shipped: C8-LINE-CLAMP.*
| 56 | `filter` (color functions), `backdrop-filter`, `mix-blend-mode`, `isolation` | `grayscale()` / `invert()` / `brightness()` / `contrast()` / `sepia()` / `saturate()` / `hue-rotate()` / `opacity()` as per-cell color transforms (`blur()` / `drop-shadow()` N/A); blend modes per cell; `isolation: isolate` as a stacking-context trigger. | M (isolation S) | Yes (as non-existent stacking triggers) |
| 57 | `box-shadow` | Shipped (C4-SHADOW; §3.5): offset shade in the shadow's color (a translucent one darkens the cells beneath), spread grows it, `inset` inside the padding box; blur N/A. | M | Yes |
| 58 | `background-clip` | Shipped (C4-BG-CLIP; §3.5): `padding-box` / `content-box` fill only that area; a half-block border keeps its cells clear. | S | Yes |
| 59 | `contain` / `content-visibility` / `contain-intrinsic-size` / `will-change` | `contain: paint` clips and forms a stacking context; `content-visibility: hidden / auto` skips layout and paint of off-screen subtrees (large lists); `will-change` as a stacking-context trigger only. | S / M | Partial — `will-change`, `contain` Yes (as non-existent triggers); rest No |
| 60 | `@container` + container query units | Size queries against a container's cell size (`container-type`, `container-name`). | M | Blanket |
| 61 | `@property` | Typed custom properties: syntax, `inherits`, `initial-value`; makes `var()` interpolable. | M | Blanket |
| 62 | `@starting-style` | Starting values so a transition runs on first style (entry animations). | S | Blanket |
| 63 | `@scope` / `@import` | Scoped rules with lower bound; importing other sheets through a loader hook. | M | Blanket |
| 64 | `transition-timing-function: linear()`; negative `transition-delay`; more animatable properties | `linear()` piecewise easing; a negative delay starts mid-way; `opacity`, `margin`, `min-*` / `max-*`, `border-width` interpolable. | S | No |
| 65 | `round()` / `mod()` / `rem()` / `abs()` / `sign()`; trig / exponential functions | Cell math (`round(down, 50%, 1)`); trig rarely needed but cheap once `calc()` has a numeric evaluator. | S | No |
| 66 | Multi-column (`columns`, `column-count`, `column-width`, `column-rule*`, `column-span`, `column-fill`) | Newspaper columns of whole cells, rules drawn with `│`. | M | No |
| 67 | `field-sizing` | `content`: a textarea / input grows with its value. | S | No |
| 68 | `resize` | Drag a textarea's bottom-right cell to resize. | M | No |
| 69 | `translate` (+ `transform: translate()` with cell lengths) | Paint-time offset in whole cells without affecting layout (like `position: relative`); establishes a stacking context. Rotation / scale stay N/A. | S | Yes |
| 70 | `clip-path: inset()` | Rectangular clip in cells; other shapes N/A. | S | No |
| 71 | `nav-up` / `nav-down` / `nav-left` / `nav-right` (UI 4, at risk) | Directional focus navigation targets — arrow-key navigation is native to TUIs. | M | No |
| 72 | `margin-trim`, `inset` with `calc()` / `%`, `hyphens: manual` | Trim child margins at container edges; percent / `calc()` in the `inset` shorthand; break at U+00AD soft hyphens showing `-` (shipped, C9-BREAKING). | S | No / No / Yes |

### High-impact *Partial* items (fix alongside the list above)

These were parsed, so they did not show up as "unknown property", but common real-world values were
dropped. The audit's six, with where each stands:

1. **`border` shorthand is a single keyword.** `border: 1px solid red`, `border: solid red`,
   `border-top: 1px solid #ccc` are all `InvalidValue` (`V/border.rs::parse_border` and
   `parse_border_side` call `parse_keyword`, which requires exactly one token). The most common
   border declaration on the web is dropped. **Shipped: C4-BORDER-SHORTHAND.**
2. ~~**`var()` works in color positions only.**~~ *Shipped: C1-VAR-ANY* — `var()` in every
   property, `content` included.
3. ~~**`max-width` / `max-height` reject `none` and percentages.**~~ *Shipped: C2-PERCENT
   (percentages), C2G-MAX-NONE / C3G-API (`none`, `MaxSize::None`).*
4. ~~**`min-width` / `min-height` take `auto | <integer>` only.**~~ *Shipped: C2-PERCENT (`%`,
   `calc()`; C5-MINMAX-SIZE).*
5. ~~**`top` / `right` / `bottom` / `left` reject a bare `%`.**~~ *Shipped: C2-PERCENT.*
6. **`width` / `height` lack `min-content` / `max-content` / `fit-content`**, although the
   intrinsic sizes are computed (`layout_pass/intrinsic.rs`). **Open: C5-INTRINSIC.**
7. ~~**`font-weight` is `normal | bold` only** — `font-weight: 700` / `600` / `bolder` are dropped.~~
   *Shipped: C9-FONT.*
8. ~~**`text-decoration` is one keyword**, matched case-sensitively; no `overline`, no
   combinations, no color / style components.~~ *Shipped: C9-DECORATION* — the full shorthand and
   its longhands.
9. **`overflow` lacks `clip` and the two-value form** (`overflow: hidden auto`). **Doc'd: No.**
10. ~~**`rgb()` takes the legacy comma form with integer channels only** — `rgb(0 0 0 / 50%)`,
    `rgb(10%, 20%, 30%)` and `rgb(12.5, 0, 0)` are dropped.~~ *Shipped: C3-RGB* — the modern
    space syntax with `/ alpha`, percentage and fractional channels and `none`; the alpha
    composites (C3-ALPHA).

---

## 3. Per-module tables

### 3.1 Syntax, cascade and inheritance (Syntax 3, Cascade 4/5, CSS 2.1 §6)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| Rule / declaration parsing with Syntax 3 error recovery | Supported | Qualified rules, stray `}`, EOF inside a block, malformed declarations warn and continue. | — | `AT`, `DECL` |
| Comments, strings, string escapes | Supported | `/* */`; `"…"` / `'…'` with hex and newline escapes. | — | `TOK::read_string` |
| Identifier escapes (`\31 0`, `\:`) | Supported | Syntax 3 §4.3.7 escapes decode in selectors (type / class / id / attribute names and values), property names and keyword values; one decoder, `rdom_core::css_syntax`, serves the selector parser and the value tokenizer (C1-ESCAPES). | — | `rdom-core/src/css_syntax.rs`, `TOK`, `SEL::parse_ident` |
| Property-name case-insensitivity | Supported | Every dispatch entry point folds the name through `property_dispatch::canonical_property_name` (`COLOR: red` is `color: red`, CSSOM `setProperty("COLOR", …)` too); custom property names stay case-sensitive (C1-CASE). | — | `DISP/table.rs` |
| Keyword case-insensitivity | Supported | Keywords, units (`2S`, `300MS`), function names and pseudo-class names (`:HOVER`) match ASCII case-insensitively (C1-CASE). | — | `V/keyword.rs`, `V/transition.rs`, `SEL` |
| `!important` | Supported | Per-field `ImportantMask`; inline and sheet; custom properties too. The `style` attribute beats author rules at both importances, above every layer (Cascade 4 §6.1 element-attached styles; C1-INLINE-IMPORTANT). | — | `DECL::strip_trailing_important` |
| Origins: UA, author (sheets, `<style>`, `App` sheets), inline | Supported | Ordering documented (App sheets after `<style>`). | — | `CASC`, `UA` |
| Specificity | Supported | Selectors 4 §17 incl. `:not()` / `:where()`; pseudo-element adds a type. | — | `SEL::ComplexSelector`, `rdom-style/src/specificity.rs` |
| `inherit` | Supported | All properties. | — | `DISP/css_wide.rs` |
| `initial` | Supported | All properties. | — | `DISP/css_wide.rs` |
| `unset` | Supported | Resolved at parse time from `inherits()` (documented implementation detail). | — | `DISP/table.rs::inherits` |
| `revert` | Supported | `Value::Revert`; the cascade ladder rolls author / inline declarations back to the UA origin and UA declarations to `unset`, custom properties and `content` included (C1-REVERT). | — | `DISP/css_wide.rs`, `CASC/ladder.rs` |
| `revert-layer` | Supported | Rolls back to the cascade without the declaration's layer (and the ones above it), important layers included; outside author layers, as `revert` (C1-LAYER). | — | `DISP/css_wide.rs`, `CASC/ladder.rs` |
| `all` | Supported | Takes a CSS-wide keyword and sets every property of the dispatch table (`unset` resolved per property); `direction` / `unicode-bidi` (when they land) and custom properties excluded; driven by `PROPERTY_NAMES`, so a new property is covered without touching `all` (C1-ALL). | — | `DISP/table.rs` |
| `@layer` | Supported | Statement and block forms, anonymous layers, `a.b` / nested sublayers, order by first declaration; unlayered beats layered for normal declarations, reversed for `!important`; one layer order across all the sheets of a cascade (C1-LAYER). | — | `rdom-css/src/layer.rs`, `rdom-style/src/stylesheet/layers.rs`, `CASC/ladder.rs` |
| `@import` | Partial | Through a host-provided `rdom_css::ImportLoader` (`parse_with_loader`, `App::set_import_loader`): rules inserted at the import's position, `layer` / `layer(name)`, a late `@import` ignored, cycles cut by the loader's resolved URL (the root's own with `parse_with_loader_at`), relative URLs resolved by the loader against the importing sheet (`ImportLoader::load_from`), nesting capped at 16, a missing loader or load error warns (C1-IMPORT, C1G-IMPORT-EDGES). Its `supports()` and media conditions are recorded (`Stylesheet::imports`) but ignored — an imported sheet always applies — until C14-MEDIA / C14-SUPPORTS. | Yes | `rdom-css/src/import.rs` |
| `@scope` | Supported | `@scope [(start)] [to (end)] { … }`: roots and limits (limit subtrees out of scope), scoped rules relative to `:where(:scope)`, `&` = `:where(:scope)`, `:scope`, declarations on the root at zero specificity, prelude-less `@scope` rooted at the owner `<style>`'s parent (`Stylesheet::owner_node`), nesting in style rules and other `@scope`s, scope proximity sorted between specificity and order of appearance (Cascade 6 §6.1) (C1-SCOPE). | — | `rdom-css/src/scope.rs`, `rdom-style/src/stylesheet/scopes.rs`, `CASC/scope.rs` |
| `@charset` | N/A | Sources are Rust `&str` (already UTF-8); consumed harmlessly. | — | `AT` |
| `@namespace` | N/A | No XML namespaces (documented). | — | — |
| CSS Nesting (`&`, nested rules) | Supported | Nested style rules, `&` anywhere (`&.x`, `.x &`, `:not(&)`), implicit descendant combinator, relative selectors (`> p`, `+ p`, `~ p`), declarations interleaved with nested rules (nested declarations rules, in order), nested `@layer`; `&` is `:is(<parent>)` for matching and specificity (`SimpleSelector::Is`, also the parsed `:is()`). Nested `@media` / `@supports` / `@container` arrive with C14 (C1-NESTING). | — | `rdom-css/src/block.rs`, `SEL/nesting.rs`, `rdom-style/src/stylesheet/style_selector.rs` |
| Inherited-property set | Supported | `inherits()` lists the inherited properties the spec tables name, for every shipped property: `color`; the font (`font`, `font-weight`, `-style`, `-size`, `-family`, `-stretch` / `font-width`, `-variant`); CSS Text (`white-space` and its longhands, `word-break`, `overflow-wrap` / `word-wrap`, `line-break`, `hyphens`, `tab-size`, `text-transform`, `text-indent`, `text-align` and its longhands, `text-justify`, `text-wrap` / `text-wrap-style`, `letter-spacing`, `word-spacing`), `line-height`, the inherited decoration properties (`text-underline-offset`, `-position`, `text-decoration-skip-ink`; `text-decoration` propagates instead), `pointer-events`, `visibility`, `caret-color`, `caret-text-color`, `color-scheme`, `border-spacing`, `direction`, `writing-mode`, `block-ellipsis` and `scrollbar-color`; `border-collapse` is non-inherited by design (documented). `cascade_inherits_exactly_the_style_crates_inherited_set` pins the list against what the cascade inherits. | — | `DISP/table.rs::inherits` |

### 3.2 Custom properties (CSS Variables 1)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `--*` declarations | Supported | Any selector, inline, `!important`, inherited, verbatim token value. | — | `DISP/set.rs` (`--` prefix) |
| `var()` in color positions | Supported | Through the general substitution (row below); `rgb(var(--r), 0, 0)` too (C1-VAR-ANY). | — | `rdom-style/src/var.rs` |
| `var()` in `content` | Supported | Substituted, then parsed by `parse_content` (C1-VAR-ANY). | — | `rdom-style/src/var.rs` |
| `var()` in all other properties | Supported | A declaration holding `var()` is kept as tokens (`TuiStyle::pending`, `var()` syntax checked at parse time); the cascade substitutes it per element from the element's custom properties and parses it with the property's grammar — shorthands included, later declarations of the block replayed in order; a failure makes the property `unset` (invalid at computed-value time). Custom properties substitute where declared; cycles make them guaranteed-invalid. Substitution is token-level (`var(--n)fr` is a number and an ident, not `1fr`; strings and idents keep their escapes) and capped at 65 536 tokens (`backend::MAX_SUBSTITUTED_TOKENS`, §3.3; C1G-VAR-TOKENS); a failure says why (`backend::SubstitutionError`, C2G-SUBSTITUTION-ERRORS). A style without `var()` costs nothing extra (C1-VAR-ANY). | — | `rdom-style/src/var/`, `DISP/set.rs`, `CASC/ladder.rs` |
| `var()` fallback with arbitrary tokens | Supported | Any token sequence, commas included, `var()` inside substituted (C1-VAR-ANY). | — | `rdom-style/src/var.rs` |
| `@property` | Supported | `@property` and `CSS.registerProperty` (`App::register_property`, `Stylesheet::register_property`): syntax validated at computed-value time (invalid → `unset`), initial value, `inherits`; registered `<color>` / `<number>` / `<integer>` / `<length>` / `<percentage>` / `<length-percentage>` transition and their `var()` consumers follow. A registered `<length>` computes to absolute cells (`10vw` → `8` at 80 columns), a `<length-percentage>` keeps its percentage (`calc(2 + 50%)`) (C2G-REGISTERED-ABSOLUTE). Syntax components without a terminal value parser are rejected (DIVERGENCES) (C1-PROPERTY). | — | `rdom-style/src/registration/`, `rdom-css/src/property.rs`, `CASC/registered.rs`, `RT/animation/custom.rs` |
| `env()` | N/A | Safe-area / UA environment variables describe display hardware a terminal does not report. | — | — |

### 3.3 Values and units (Values 4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `<number>` cells (unitless) | Supported | rdom's length unit; see §4. An integer or a fraction (`1.5`, the same length as `calc(1.5)`), in every cell-length property (C4G-NUMBER-RANGE); an integer literal keeps its value in the token (a custom property passes it on whole, C5G-INT-CLAMP-SITE) and a length clamps it to `i32::MAX` where it consumes it (CSS Values 4 §5.1), then to the property's range or rejected by it. | Yes | `V/length.rs`, `V/spacing.rs` |
| `<percentage>` | Supported | Every length-bearing property (`width` / `height`, `min-*` / `max-*`, `padding`, `margin`, the insets, `gap`), bare and in math functions, each against its spec's basis; `opacity`, where a percentage is a number in math too (`calc(50%)`, `min(1, 50%)`, C2G-CALC-SEMANTICS). `flex-basis` against the flex container's inner main size (C6-FLEX-LONGHANDS). C2-PERCENT. | — | `V/numeric.rs::length_percentage` |
| `<number>` (fractional) | Supported | `opacity`, `cubic-bezier()`, times, flex factors (`flex: 0.5`, `flex-shrink: 1.5`, `1.5fr`; factors summing below one share that fraction of the free space, Flexbox §9.7) (C2-NUMBER); math functions of type `<number>` in every `<number>` and `<integer>` property (`z-index: calc(1 + 1)`, rounded) and registered `<number>` / `<integer>` / `<percentage>`; no basis folded at parse time — a percentage where none is allowed is invalid, a viewport unit in `<number>` math is rejected (documented) (C2G-CALC-SEMANTICS). | — | `V/numeric.rs::number`, `V/length.rs` |
| `calc()` | Supported | `+ - * /`, parentheses, nested `calc()`, percent-bearing forms resolved at layout, IEEE division (`1/0` is +∞, `0/0` NaN; the top level clamps to a symmetric range, NaN is 0), nesting ≤ 32 and tree depth ≤ 256 (documented); «percent» is its own type (`CalcKind::Percent`) (C2G-CALC-SEMANTICS, C2G-CALC-DEPTH). Whitespace relaxation documented. | Yes | `CALC` |
| `min()` / `max()` / `clamp()` | Supported | Inside and outside `calc()`, nested, mixed with percentages (resolved at layout), `clamp()` with `none` bounds (C2-MINMAX). | — | `CALC` |
| `round()` / `mod()` / `rem()` / `abs()` / `sign()` | Supported | All four rounding strategies, the argument-range rules (NaN / infinities), percentages resolved at layout (C2-STEPPED). | — | `CALC` |
| `sin()` … `atan2()`, `pow()` / `sqrt()` / `hypot()` / `log()` / `exp()` | Supported | With the constants `e`, `pi`, `infinity`, `-infinity`, `NaN` and Values 4 §10.9 type checking (an `<angle>` result is no length); math functions of type `<number>` also work in `opacity` and flex factors (C2-TRIG). | — | `CALC` |
| `px`, `cm`, `mm`, `Q`, `in`, `pt`, `pc` | N/A | No pixel / physical length on a cell grid (documented). | — | — |
| `em`, `rem`, `ex`, `cap`, `ic` | N/A | No font size or font metrics to scale against (documented). | — | — |
| `ch` | Supported | Exactly one column on a monospaced grid, in every length property and math function; fractions round where the value becomes a length (C2-CH). | — | `CALC/units.rs`, `V/numeric.rs` |
| `lh`, `rlh` | Supported | The element's and the root's used `line-height` in rows, in every length property and math function, absolute at computed-value time; in `line-height` itself `lh` is the parent's (C2-LH, C9-LINE-HEIGHT). | — | `CALC/units.rs`, `rdom-style/src/absolute.rs`, `CASC/text.rs` |
| `vw` / `vh` / `vmin` / `vmax` (+ `sv*` / `lv*` / `dv*`, `vi` / `vb`) | Supported | 1% of the terminal's columns / rows, absolute at computed-value time (the cascade resolves them; a resize cascades again) (C2-VIEWPORT). | — | `CALC/units.rs`, `rdom-style/src/absolute.rs`, `CASC` |
| `cqw` / `cqh` / `cqi` / `cqb` / `cqmin` / `cqmax` | Missing | Need `@container`. | No | `CALC` |
| `fr` | Supported | Grid's `<flex>` in every track list and track size (`1fr`, `minmax(0, 1fr)`, CSS Grid 2 §7.2.4, C7-GRID-CORE), sized by §11.7; also accepted on `width` / `height` as an rdom flex weight (§4). | Yes | `V/grid.rs`, `V/length.rs::parse_size` |
| `<time>` (`s`, `ms`) | Supported | Rounded to whole ms (documented). | Yes | `TR::parse_time_ms` |
| `<angle>` (`deg`, `grad`, `rad`, `turn`) | Supported | In the trigonometric functions and registered `<angle>` custom properties (which interpolate); `parse::values::parse_angle` gives degrees for the color hues of Phase 3 (C2-ANGLE); always finite — NaN is 0, ±∞ clamps to `MAX_ANGLE_DEGREES` (C2G-CALC-SEMANTICS). No rotation exists. | — | `CALC/units.rs`, `V/numeric.rs::parse_angle` |
| `<resolution>`, `<frequency>` | N/A | Image resolution / aural values. | — | — |
| `<string>` | Supported | In `content`. | — | `V/content.rs` |
| `url()` / `<image>` / gradients / `image-set()` | N/A | No images (documented). | — | — |
| `<ratio>` | Supported | One or two `<number [0,∞]>` terms, fractions and math functions included (C2-RATIO). | — | `V/number.rs::parse_aspect_ratio` |
| `attr()` | Supported | In any property and in custom properties, substituted like `var()` at computed-value time; `type(<syntax>)`, `number`, units, `raw-string`, fallbacks; a pseudo-element reads its originating element (C2-ATTR). Attribute values are not themselves searched for substitution functions (documented). | — | `rdom-style/src/attr.rs`, `rdom-style/src/var.rs` |
| `<custom-ident>` | Supported | Counter names, transition-property idents. | — | `V/content.rs`, `TR` |

### 3.4 Color (Color 4 / 5)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `color` | Supported | Every color form below that parses. | — | `DISP/set.rs` |
| Named colors (148) | Supported | Case-insensitive lookup. | — | `rdom-style/src/color/named.rs` |
| Hex `#rgb` / `#rgba` / `#rrggbb` / `#rrggbbaa` | Supported | Alpha kept (`Color::Rgba`) and composited over the backdrop (C3-ALPHA). | Yes | `tui_color.rs::parse_hex` |
| `rgb()` / `rgba()` | Supported | Modern syntax (space-separated, `/ alpha`, `none`) and legacy comma syntax; numbers with fractions and percentages, math functions in channels and alpha, out-of-range values clamped (C3-RGB). Alpha is kept and composited over the backdrop (C3-ALPHA). | — | `V/color/rgb.rs`, `V/color/channel.rs` |
| `transparent` | Supported | Transparent black (`Color::TRANSPARENT`, CSS Color 4 §6.3): a background shows what is beneath, text in it paints no glyph (the backdrop's stays), a border keeps its space and draws nothing (C3-TRANSPARENT). | — | `tui_color.rs::parse_simple_color`, `PAINT/mod.rs::fills`, `rdom-tui/src/render/buffer/write.rs` |
| `currentColor` | Supported | `TuiColor::CurrentColor`, any case: the element's final `color` in `background-color` / `border-color` (resolved after the ladder, `apply::ElementColors`), the inherited color in `color`; `caret-color` / `caret-text-color` keep it and resolve at paint; `border-color`'s initial value (C3-CURRENTCOLOR). An `inherit`ed `currentcolor` is the parent's resolved color (documented). | — | `V/color/mod.rs`, `tui_color.rs`, `CASC/apply.rs` |
| `hsl()` / `hsla()` | Supported | Modern syntax (hue as a number or `<angle>`, saturation / lightness as percentages or numbers, `none`, `/ alpha`, math functions) and the legacy comma syntax (percentages only); converted to sRGB at parse time, as CSS computes it (C3-HSL-HWB). | — | `V/color/hsl.rs`, `rdom-style/src/color/convert.rs` |
| `hwb()` | Supported | Hue, whiteness, blackness (`none`, `/ alpha`, math functions); whiteness + blackness ≥ 100% is a gray; converted to sRGB at parse time (C3-HSL-HWB). | — | `V/color/hsl.rs`, `rdom-style/src/color/convert.rs` |
| `lab()` / `lch()` / `oklab()` / `oklch()` | Supported | Numbers, percentages (Lab a / b: 100% = 125, LCH C: 150, Oklab a / b / C: 0.4), hues, `none`, `/ alpha`, math functions; lightness and chroma clamp at parsed-value time; converted to sRGB with CSS gamut mapping (OKLCh chroma reduction, §13.2) at parse time — the computed color is sRGB (documented) (C3-LAB). | — | `V/color/lab.rs`, `rdom-style/src/color/{convert,gamut}.rs` |
| `color()` | Supported | `srgb`, `srgb-linear`, `display-p3`, `a98-rgb`, `prophoto-rgb`, `rec2020`, `xyz` / `xyz-d65`, `xyz-d50`; numbers or percentages (100% = 1), `none`, `/ alpha`; gamut-mapped to sRGB at parse time (C3-LAB). | — | `V/color/lab.rs`, `rdom-style/src/color/{convert,gamut}.rs` |
| `color-mix()` (Color 5) | Supported | Every rectangular and polar interpolation space, the four hue methods, percentages (either order, math functions, §2.2 normalization and alpha scaling), the default Oklab method; missing components carried forward and powerless hues (Color 4 §12); nested functions. Mixed at parse time; with `currentcolor` inside, kept as `TuiColor::Function` and mixed at computed-value time (`var()` is substituted before parsing) (C3-MIX). | — | `V/color/mix.rs`, `rdom-style/src/color/interpolate.rs` |
| Relative color syntax (Color 5) | Supported | `from <color>` in `rgb()` / `rgba()` / `hsl()` / `hsla()` / `hwb()` / `lab()` / `lch()` / `oklab()` / `oklch()` / `color()`: the origin converted to the function's space, its channels and `alpha` bound to keywords usable alone or in math functions; with `currentcolor` as the origin, computed at computed-value time (C3-RELATIVE). | — | `V/color/relative.rs` |
| System colors (`Canvas`, `CanvasText`, `LinkText`, `ButtonFace`, …) | Supported | All nineteen (and the deprecated ones, mapped): `Canvas` / `ButtonFace` / `CanvasText` / `FieldText` are the terminal's defaults (`Color::Reset`), the rest the UA palette (`color::system`, documented); inside a color function the defaults take the canvas model's black / white (C3-SYSTEM). | — | `rdom-style/src/color/system.rs` |
| `light-dark()` + `color-scheme` | Supported | `color-scheme` (`normal`, `light` / `dark` / custom identifiers, `only`; inherited) picks each element's used scheme from the document's preferred one, which the `App` reads off the terminal's background at startup (OSC 11, Unix) or is given (`App::with_color_scheme` / `set_color_scheme`; dark by default); `light-dark()` resolves at computed-value time, also inside other color functions. Theme changes are followed: `App::run` enables DEC mode 2031 on Unix and rdom's own input reader parses its reports (C3-SCHEME, C3G-INPUT-READER). | — | `rdom-style/src/color/scheme.rs`, `V/color/mod.rs`, `CASC/colors.rs`, `RT/color_scheme/`, `RT/input/` |
| `opacity` | Partial | `<number>` / `<percentage>` clamped to 0–1 (`50%` since C2-PERCENT). Group opacity per cell (documented). | No | `V/number.rs::parse_opacity` |
| Alpha in colors | Supported | Composited per cell over the backdrop with the group-opacity rules — background blend, glyph contest, tint, canvas model of the color scheme (documented); animatable (C3-ALPHA). | — | `rdom-tui/src/render/buffer/translucent.rs`, `PAINT/mod.rs` |
| `forced-color-adjust`, `print-color-adjust` | N/A | No forced-colors mode or print. | — | — |

### 3.5 Backgrounds and borders (Backgrounds 3, Borders 4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `background-color` | Supported | Any parsed color; the initial value is `transparent`, which paints nothing, while `Canvas` / `reset` paint the terminal's default background (C11G-CANVAS-FILL). | — | `DISP/set.rs` |
| `background` | Supported | The full Backgrounds 3 §3.10 grammar: layers, the color on the final one; image layers parse and are stored but draw nothing (documented, C4-BACKGROUND). | — | `DISP/background.rs`, `V/background.rs` |
| `background-image` / `-position` / `-size` / `-repeat` / `-attachment` / `-origin` | N/A | No images (documented); they parse and are stored, inert (C4-BACKGROUND). | — | `DISP/background.rs` |
| `background-clip` | Supported | `border-box` (initial: under the border) / `padding-box` / `content-box`, the final layer's clipping the color; a half-block border keeps its cells clear (documented); `text` is N/A (documented) (C4-BG-CLIP). | — | `PAINT/background.rs::clip_box`, `CASC/decoration.rs` |
| `border` | Supported | `<line-width> || <line-style> || <color>` in any order on all four sides, omitted components reset (`border: 1px solid red`); widths in pixels pick a glyph weight (documented); rdom keywords kept (documented) (C4-BORDER-SHORTHAND). | — | `V/border.rs::parse_border`, `DISP/border.rs` |
| `border-top` / `-right` / `-bottom` / `-left` | Supported | The same grammar for one side's style, width and color (C4-BORDER-SHORTHAND). | — | `V/border.rs::parse_border_side_shorthand`, `DISP/border.rs` |
| `border-style` | Supported | 1–4 values, clockwise from the top (C4-BORDER-SIDES); rdom's `rounded` is a synonym of `solid` here and does not round — only the `border` shorthand's `rounded` sets a radius; `border-radius` rounds (documented). | — | `DISP/border.rs`, `V/border.rs::parse_sides` |
| `border-*-style` | Supported | Every CSS keyword parses, one longhand per side (C4-RADIUS split the shared storage); `dashed` / `dotted` draw dash glyphs on straight runs (`╌╎` / `┄┆`, heavy forms; corners solid, C4G-EDGE-TESTS); `ridge` / `groove` / `inset` / `outset` render as `solid` (documented). | Yes | `DISP/border.rs`, `PAINT/border_join/glyphs.rs::dash_glyph` |
| `border-color` | Supported | 1–4 colors, clockwise from the top (C4-BORDER-SIDES). | — | `DISP/border.rs`, `TS::border_color` |
| `border-top-color` / `-right-color` / `-bottom-color` / `-left-color` | Supported | One longhand per side, cascaded independently; a corner cell takes its dominant side's color — the heavier style, then the horizontal side (documented) (C4-BORDER-SIDES). | — | `DISP/border.rs`, `CASC/colors.rs`, `PAINT/border_join/mod.rs::dominant_contribution` |
| `border-width` / `border-*-width` | Supported | 1–4 values, per-side longhands; `0` = no border, `thin` / `medium` light glyphs, `thick` (5px / two cells and up) heavy, mixed junctions; pixel lengths bare or in a math function (`calc(2px)`, C4G-PX-CALC); always one cell wide (documented) (C4-BORDER-SIDES, C4-BORDER-WIDTH). | — | `DISP/border.rs`, `rdom-style/src/layout/border.rs::BorderWidth::weight`, `PAINT/border_join/` |
| `border-radius` / `border-*-radius` | Supported | 1–4 values with `/` for the vertical radii, per-corner longhands, cells / pixels / `em` / `%`, pixel math functions (`calc(8px / 2)`, C4G-PX-CALC); a non-zero radius rounds that corner's glyph (`╭╮╰╯`), the curve's size is N/A (documented) (C4-RADIUS). | — | `DISP/border.rs`, `V/border.rs::parse_border_radius`, `PAINT/border/mod.rs::Pen::corner_at` |
| `border-image*` | N/A | Image-sliced borders. | — | — |
| `box-shadow` | Supported | Shadows as whole-cell shades: outer under the background and outside the box, `inset` inside the padding box, first on top, offsets / spread in cells (a pixel length — bare or a math function over pixels, C4G-PX-CALC — one cell), colors with alpha composited; painted in CSS 2.1 Appendix E order — an in-flow block's with the backgrounds, in its unit's background phase (C4G-SHADOW-ORDER, C8G-PAINT-PHASES); a flex item's and an inline block's whole at its turn, as an atomic box (C5G-FLEX-SHADOW, C5G-INLINE-BLOCK-SHADOW); blur N/A (documented) (C4-SHADOW). | — | `V/shadow.rs`, `DISP/shadow.rs`, `CASC/colors.rs`, `PAINT/shadow.rs` |
| `border-collapse` | Supported | `separate` / `collapse`, with the documented scope / inheritance divergences. | Yes | `DISP/set.rs`, `rdom-tui/src/render/layout_pass/border_collapse.rs` |
| `border-spacing` | Partial | Parsed (one or two cell lengths), cascaded and inherited (C4-SPACING); the gaps between separated table cells land with the table formatting context (C13-TFC). | Yes | `DISP/border.rs`, `V/border.rs::parse_border_spacing`, `CASC/apply.rs` |

### 3.6 Box model and sizing (Box 3, Sizing 3/4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `margin` / `margin-*` | Supported | Signed cells, `auto`, `%`, `calc()` (C2-PERCENT); four independent longhands, each with its own `!important`, set by the shorthand (C6-MARGIN-SIDES). | — | `V/spacing.rs` |
| `padding` / `padding-*` | Supported | Cells, `%`, `calc()` (C2-PERCENT); four independent longhands, each with its own `!important` (C6-MARGIN-SIDES). | — | `V/spacing.rs` |
| `margin-trim` | Supported | `none | [block || inline] | [block-start || inline-start || block-end || inline-end]`; block containers trim their edge children's block-axis margins (no collapse out), flex containers the first / last item's main-axis and every item's cross-axis margins, intrinsic sizes included (C5-MARGIN-TRIM); grid containers the margins of the items in their first / last row and column, track sizes included (C7-GRID-CORE). | Yes | `V/spacing.rs`, layout `margin_trim.rs` |
| `width` / `height` | Partial | `auto`, cells, `%`, `calc()`, rdom `fr`, `min-content` / `max-content` / `fit-content` / `fit-content(<l>)` (C5-INTRINSIC; on the block axis the content height, CSS Sizing 3 §3.1); missing `stretch` (CSS Sizing 4). | Yes | `V/length.rs::parse_size`, `BOX::Size`, `layout_pass/intrinsic/keywords.rs` |
| `min-width` / `min-height` | Supported | `auto`, cells, `%`, `calc()` (C2-PERCENT), the intrinsic keywords (C5-INTRINSIC); they clamp intrinsic contributions too — an inline block's width, a shrink-to-fit container's (CSS Sizing 3 §5.2, C5G-SIZING-SITES). | — | `V/length.rs::parse_min_size` |
| `max-width` / `max-height` | Supported | Cells, `%`, `calc()` (C2-PERCENT), `none` (the initial value; C2G-MAX-NONE completes C5-MINMAX-SIZE), the intrinsic keywords (C5-INTRINSIC); they clamp intrinsic contributions too (C5G-SIZING-SITES). | — | `V/length.rs::parse_max_size` |
| `box-sizing` | Supported | `content-box` (the initial value) / `border-box`; every layout site converts through one `Sizer` (`render/layout_pass/box_sizing.rs`) — intrinsic contributions (a percentage of a definite containing block, `calc()`) and a table cell's author width included (C5G-SIZING-SITES) — and the border box is floored at padding + border (C5-BOX-SIZING). UA form controls follow the HTML rendering rules. | — | `KW`, layout `box_sizing.rs` |
| `aspect-ratio` | Supported | `auto || <ratio>`; `auto && <ratio>` sizes the content box; degenerate ratios behave as `auto` (C2-RATIO); a flex item's base size from a definite cross size (§9.2 step 3.B, C6G-FLEX-SPEC). Cell-grid rounding documented. | — | `V/number.rs`, `FLEX/cross.rs` |
| `contain-intrinsic-size` (+ longhands) | Partial | The shorthand, `contain-intrinsic-width` / `-height` and the logical `-inline-size` / `-block-size` (sharing the physical storage), `auto? [none | <length>]`, parse and cascade (C5-CONTAIN-SIZE); they size a box only under size containment, which lands with C14-CONTAIN. | Yes | `DISP` (`contain.rs`), `V/length.rs` |

### 3.7 Display and visibility (Display 3)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `display: block` / `inline` / `inline-block` / `flex` / `inline-flex` / `none` | Supported | Outer + inner (`Flow`) pair. An atomic inline — an inline block, and an inline flex container (`inline-flex` / `inline flex`, laid out as a flex container; C6G-INLINE-FLEX-ATOM, which it was not before: its contents packed as inline text) — sits in its line on the baseline, the line box growing to its margin box, and paints there as a box (C5G-ATOM-BOX). A flex container's runs of text are anonymous flex items, whitespace-only runs none, and its `::before` / `::after` items of their own (CSS Flexbox §4; their own box properties do not apply yet, DIVERGENCES §3) (C6G-ANON-FLEX-ITEMS). | — | `DISP/set.rs`, `KW::Display`, `KW::Flow` |
| `display: contents` | Supported | No box: its children and `::before` / `::after` take part in the parent's formatting context (block flow — a box-less child holding a block box gives its children in its place, `render/box_tree.rs` — flex items — its text joins the container's runs of text, each an anonymous item, and its `::before` / `::after` are items of their own (C6G-ANON-FLEX-ITEMS) — inline content; static positions of its out-of-flow children; its own `overflow` ignored) (C6G-CONTENTS-BOXTREE); inherited properties flow through; hit-testing reaches its children with it on the path (under `order` too); it stays focusable (HTML "being rendered"; Chromium); on replaced elements and form controls it behaves as `none` (CSS Display 3 Appendix B) (C6-DISPLAY-KEYWORDS). | — | `KW::Display`, `render/box_tree.rs` |
| `display: flow-root` | Supported | `Flow::FlowRoot`: block flow in a new BFC — no margin collapses through its edges (C6-DISPLAY-KEYWORDS). Floats (C8-FLOAT) will be contained by the same BFC. | — | `KW::Flow`, `BLOCK/margin_collapse.rs` |
| `display: list-item` | Supported | `list-item`, `inline list-item`, with `flow` / `flow-root` (CSS Display 3 §2.3) lay out as their outer / inner types (C6-DISPLAY-KEYWORDS) and increment `list-item` (C10-LIST-ITEM, C10-COUNTERS); every list item generates a `::marker` — an element, or a `::before` / `::after` (C10G-PSEUDO-MARKER); an inline one's is its first inline box, `outside` acting as `inside` (CSS Lists 3 §3.5, C10G-INLINE-LIST-ITEM). | — | `KW`, `CASC/counters/`, `inline/markers.rs` |
| `display: grid` / `inline-grid` | Supported | `grid`, `inline-grid`, `block grid`, `inline grid` (`Flow::Grid`): a grid formatting context (CSS Grid 2) — items built as flex items (elements, pseudo-elements, anonymous items for text runs), blockified, ordered by `order`, painted atomically; `inline-grid` an atomic inline; the track sizing algorithm (§11) in whole cells; the container's min- / max-content sizes its tracks' (§5.2); scrolling, `rtl` columns, `margin-trim` at the grid's edges (C7-GRID-CORE). Placement properties, implicit-track sizes, areas and box alignment: C7-GRID-PLACE / -AUTO / -AREAS / -ALIGN. | Yes | `rdom-tui/src/render/layout_pass/grid/` |
| `display: table` family | Missing | Real TFC (tables are tag-driven flex rows today). | Yes | `rdom-tui/src/runtime/builtins/table` |
| Multi-keyword `display` (`block flex`, `inline flow-root`) | Supported | `<display-outside> || <display-inside>` in either order, defaults `block` / `flow`, the legacy keywords as their pairs, serialized shortest (CSS Display 3 §2; `V/display.rs`) (C6-DISPLAY-KEYWORDS). `grid` / `table` / `ruby` / `run-in` with their phases. | — | `V/display.rs` |
| `display: run-in` | N/A | Unimplemented by browsers; no TUI use. | — | — |
| `display: ruby*` | N/A | Ruby annotations need half-height text above a base. | — | — |
| `visibility` | Supported | `visible` / `hidden` / `collapse`, inherited (CSS Display 3 §4): a hidden box keeps its place and draws nothing (no shadow, background, border, text, canvas or scrollbar), a `visible` descendant draws; it is no hit target (a visible descendant is, with it on the path), not focusable — Tab, `is_focusable` / `is_tab_focusable` and `focus()` share one answer, and a focused element that becomes hidden or `display: none` is blurred by the frame's focus fixup — and its text is not copied, by the used value mid-transition (C6G-VISIBILITY-ONE-ANSWER). `collapse` on a flex item is a strut (Flexbox §4.4, §9.4 step 10: no main size or main margins, the cross size of its line laid out uncollapsed — its items at their hypothetical main sizes — and otherwise ignored: no gap beside it, no `justify-content` share, no baseline; C6G-COLLAPSE); on a `<tr>` it removes the row while its cells still size the columns (CSS 2.1 §17.5.5); elsewhere it is `hidden`. Transitions: `visible` for the whole run with a `visible` end. Column collapse (`<col>`) with C13-TFC. | Yes | `KW::Visibility`, `render/visibility.rs`, `FLEX/strut.rs` |
| `order` | Supported | `<integer>` (math rounded, clamped to `i32`), not inherited (CSS Flexbox §5.4): flex items are laid out (margin-trim's first / last item included), painted and hit-tested in order-modified document order (`render/box_tree.rs::paint_order_children`); sequential focus, selection, copy and the DOM keep document order (§5.4.1); grid items are auto-placed, painted and hit-tested in that order too (CSS Grid 2 §6.3, C7-GRID-CORE). | — | `FLEX`, `render/box_tree.rs` |

### 3.8 Flexbox and box alignment (Flexbox 1, Align 3)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `flex-direction` | Supported | `row` / `row-reverse` / `column` / `column-reverse` (CSS Flexbox §5.1; the axis plus `flex_reverse`): a reversed main axis lays out from its main-start edge — a `row-reverse` from the right under `ltr`, from the left under `rtl`; a `column-reverse` from the bottom — with each item's main-start margin on that side and `margin-trim` mapped to it (C6-DIRECTION-REVERSE). A reversed scroll container's scrolling area origin is its main-start edge (CSSOM View §4): `scrollLeft` / `scrollTop` run `-overflow ..= 0`, starting at 0 (`TuiExt::scroll_y` is signed like `scroll_x`). The initial value is `row`; a block container ignores the property — its children stack on its block axis (C6-FLEX-DIRECTION-INITIAL). | — | `DISP/set.rs`, `KW::Direction`, `FLEX/placement.rs` |
| `flex-wrap` | Supported | `nowrap` / `wrap` / `wrap-reverse` (CSS Flexbox §5.2): §9.3 line breaking by outer hypothetical main size (the main-axis gap counted), §9.7 per line, each line as large as its largest outer hypothetical cross size with its `stretch` items filling it, `align-content: normal` stretching the lines (whole cells, remainder to the first lines), the cross-axis gap between lines; `wrap-reverse` stacks lines from cross-end and scrolls from there (negative offsets); `margin-trim` per line; §9.9 intrinsic sizes (min-content main size the largest item's, the cross size the lines') (C6-WRAP). | — | `FLEX/lines.rs`, `layout_pass/intrinsic/wrap.rs` |
| `flex-flow` | Supported | `<'flex-direction'> \|\| <'flex-wrap'>` (§5.3), either order, an omitted half its initial value; shortest serialization (C6-WRAP). | — | `V/flex.rs` |
| `flex` | Supported | Full Flexbox §7.2 grammar (`none`, `auto`, 1–3 values in either order, the unitless-zero rule), setting its three longhands — `flex-grow`, `flex-shrink`, `flex-basis` — and nothing else (C2G-FLEX-SHORTHAND, C6-FLEX-LONGHANDS); an omitted basis is `0%`, as in every engine (the spec text says `0`), serialized `1 1 0%` (C6G-FLEX-BASIS-ZERO). | — | `V/length.rs::parse_flex_shorthand` |
| `flex-grow` | Supported | `<number [0,∞]>`, initial 0 (Flexbox §7.3.1): the share of positive free space (§9.7); rdom's `width: <n>fr` is a basis of 0 growing by `n` when `flex-grow` is 0 (C6-FLEX-LONGHANDS). | — | `DISP`, `FLEX/distribute.rs` |
| `flex-shrink` | Supported | `<number [0,∞]>`, fractions included (C2-NUMBER). | — | `DISP/set.rs`, `FLEX/main_axis.rs` |
| `flex-basis` | Supported | `auto` (the main size property; its content size when that is `auto`), `content` (max-content), cells, `%` (of the container's inner main size; `content` when indefinite), `calc()`, the intrinsic keywords (C5-INTRINSIC), measuring the box `box-sizing` names: the flex base size (Flexbox §9.2 step 3) that §9.7 grows by `flex-grow` and shrinks by `flex-shrink ×` the inner (content-box) base — a line with no room shrinks too — min / max violations frozen by their total, the §4.5 automatic minimum included; positive free space left after that goes to `auto` margins (C6-FLEX-LONGHANDS). | — | `DISP`, `FLEX/main_axis.rs`, `FLEX/distribute.rs` |
| `justify-content` | Supported | Full Box Alignment 3 §5.2 grammar (`normal`, `flex-start` / `flex-end`, `start` / `end`, `left` / `right`, `center`, `space-between` / `-around` / `-evenly`, `stretch`, `safe` / `unsafe`): per line, after the flexible lengths and `auto` margins (Flexbox §8.2); `normal` / `stretch` are `flex-start`; `start` / `end` the writing mode's ends, `left` / `right` physical (a column's are `start`); distribution fallbacks `safe flex-start` / `safe center`; `safe` overflow aligns as `start`, the default as `unsafe` (as browsers). Whole cells: `center` rounds the lead down, distributions round rolling positions up (DIVERGENCES §1) (C6-JUSTIFY). In a grid (CSS Grid 2 §10.5): the tracks — those not collapsed by `auto-fit` — are the alignment subjects, the same keyword mapping and whole-cell shares as flex (`layout_pass::distribution`), the distributed space widening the gutters and the areas spanning them; `normal` / `stretch` stretch the `auto` tracks (§11.8), else pack to the start; `left` / `right` physical under `rtl` (C7-GRID-ALIGN). | — | `FLEX/content.rs`, `V/align.rs` |
| `align-items` | Supported | Box Alignment 3 §6.3 grammar (`normal`, `stretch`, `baseline` / `first baseline` / `last baseline`, `safe` / `unsafe` with `center`, `start` / `end`, `self-start` / `self-end`, `flex-start` / `flex-end`): `normal` / `stretch` fill the line (an `auto` cross size, clamped; §9.4 step 11), `flex-*` the line's cross edges (`wrap-reverse` swaps them), `start` / `end` the container's writing-mode edges, `self-*` the item's; baseline groups line up first / last content rows (DIVERGENCES §2), the group flush top / bottom, and size their line — a multi-line container's, or a single-line row's `auto` height (C6G-BASELINE-ROW); a column's baselines fall back to `safe self-start` / `-end`; `safe` overflow aligns as cross-start (C6-ALIGN). In a grid (CSS Grid 2 §10.4, Box Alignment §9.3): the `baseline` / `last baseline` items of a row form a group per row and preference — a spanning item in its first (last) row's — whose first (last) content rows meet, flush with the row's start (end), each item's shim counting toward the rows (§11.5 step 1); an item with an `auto` block margin does not take part (C7-GRID-ALIGN). | — | `FLEX/align.rs`, `FLEX/cross.rs` |
| `align-self` | Supported | `auto` (the container's `align-items`) or any `align-items` value, per item; `auto` cross margins win (§8.1) (C6-ALIGN). In a grid area (Grid §10.4): `normal` / `stretch` fill an `auto` height (`normal` not for an item with a preferred aspect ratio, whose height follows the ratio, §6.2), any other value places the content height — `start` / `end` / `center` / `self-*` / `flex-*`, `safe` keeping an overflowing item at the start — after `auto` margins (§10.2) (C7-GRID-ALIGN). | — | `FLEX/align.rs` |
| `align-content` | Supported | Box Alignment 3 §5.1 grammar: a multi-line flex container's free cross space (§9.4 step 15) — `normal` / `stretch` grow the lines, `flex-start` / `flex-end` (swapped by `wrap-reverse`), `start` / `end` (the baseline values fall back to them), `center`, the distributions with `justify-content`'s `safe` fallbacks and whole-cell rounding; no effect on a single-line container, as browsers (csswg-drafts#3052 kept it) (C6-ALIGN-CONTENT). On a block container (§5.1, Chromium 123) — of a definite height, or an `auto` one that `min-height` makes taller than its content (C6G-BLOCK-ALIGN) — it moves the block-level content, once laid out — `end`, `center`, the distributions' fallbacks; overflow honors `safe` and scroll containers — and makes the container an independent formatting context; a container whose content is inline moves its lines and their atoms the same way (C6G-DOCS), except upward — overflowing inline content aligned toward the end stays at the top, as `safe` (DIVERGENCES §4) (C6-PLACE). In a grid (CSS Grid 2 §10.5): the rows in a container whose height leaves free space, as `justify-content` the columns (C7-GRID-ALIGN). | — | `layout_pass/distribution.rs`, `FLEX/content.rs`, `BLOCK/align.rs`, `rdom-tui/src/render/layout_pass/grid/content.rs` |
| `justify-items` / `justify-self` | Supported | Box Alignment 3 §6.1 / §6.2 grammars (`legacy` and `legacy left / right / center` for `justify-items`, computed through the parent): a block-level box's `justify-self` (`auto` → the parent's `justify-items`) other than `normal` / `stretch` sizes an `auto` width `fit-content` and places it (`start` / `end` / `self-*` / `left` / `right` / `center`, the baseline values as `safe self-start` / `safe self-end` by the box's own direction, C6G-BLOCK-ALIGN; `auto` margins win; `safe`), as Chromium 130; ignored in flex (§6.1); a grid item's in its grid area (Grid §10.3: `auto` → `justify-items`, a `legacy` keyword as its side; `normal` / `stretch` fill an `auto` width — a preferred aspect ratio making `normal` the block-level width, §6.2 — else `fit-content` placed as for a block-level box, `auto` margins first, §10.2; C7-GRID-ALIGN); an absolutely positioned box's `justify-self` / `align-self` (`auto` is `normal`, §6.1) align it in its inset-modified containing block (CSS Position 3 §4.1, an `auto` inset counting as 0 beside a non-`auto` one), an aligned `auto` size `fit-content`, `auto` margins winning (C6G-DOCS); with both insets `auto` it stays at its static position (DIVERGENCES §4) (C6-PLACE). | — | `BLOCK/align.rs`, `BLOCK/width.rs` |
| `place-content` / `place-items` / `place-self` | Supported | `<align> <justify>?` (§5.5, §6.4, §6.5): one value sets both (`place-content: baseline` gives `justify-content: start`); shortest serialization; `cssText` lists them over their longhands (C6-PLACE). | — | `V/align.rs` |
| `gap` | Supported | `<'row-gap'> <'column-gap'>?` (CSS Box Alignment 3 §8.3), one value setting both; serialized as one when they agree (C6-GAP). | — | `V/spacing.rs::parse_gap_shorthand` |
| `row-gap` / `column-gap` | Supported | `normal | <length-percentage [0,∞]>`, initial `normal` (0 in flex and grid), one per axis (§8.1) — a grid's row and column gutters (C7-GRID-CORE): a row flex container's items are `column-gap` apart, a column's `row-gap` (a multi-line container's lines are the other gap apart, C6-WRAP); rdom also spaces a block container's block children by `row-gap` (§4 below). A `gap` transition animates both (C6-GAP). | — | `DISP`, `TS::row_gap` / `column_gap`, `layout_pass::resolve_gap` |
| Auto margins in flex | Supported | Main and cross axis. | — | `FLEX/main_axis.rs`, `FLEX/cross.rs` |
| Min-content protection (`min-width: auto`) | Supported | Flexbox §4.5: the automatic minimum size on the main axis, `auto` the initial value (undeclared = `auto`), clamped by a definite `max-*`, part of the hypothetical main size that decides growing or shrinking (§9.7 step 1); 0 on the cross axis (C3G-MIN-AUTO, C6G-FLEX-SPEC). | — | `rdom-tui/src/render/layout_pass/intrinsic.rs` |

### 3.9 Grid (Grid 1/2)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `grid-template-columns` / `grid-template-rows` | Supported | `none` and track lists — cells, `%`, `calc()`, `fr`, `auto`, `min-content`, `max-content`, `minmax()`, `fit-content()`, `repeat(<n> / auto-fill / auto-fit)` (§7.2.3.2, `auto-fit` collapsing empty repetitions), line names — parsed, computed and serialized as written; sized by the track sizing algorithm (§11.3–§11.8: intrinsic sizes by increasing span, spanning items into flexible tracks by flex factor, the `fr` size, `auto` tracks stretched), the columns and then the rows sized once more when the rows change an item's column contribution — an `aspect-ratio` item's width transferred from a height its row makes definite (§11.1 steps 3–4, C7-GRID-RERESOLVE) in whole cells (DIVERGENCES §1); `row-gap` / `column-gap` as fixed gutters, `normal` 0 (C7-GRID-CORE). The used track sizes — §7.2.6's resolved value — are read with `TuiAccessors::grid_tracks()`, cell ranges from the content box (C7G-DOCS-TESTS); CSSOM serializes the value as written. | Yes | `V/grid.rs`, `DISP/grid.rs`, `rdom-tui/src/render/layout_pass/grid/` |
| `grid-template-areas` | Supported | `none | <string>+`, each string a row of named cell tokens and `.` null cells (§7.3), the declaration invalid unless the rows have equal cell counts, hold no trash token and every name fills one rectangle; serialized with single spaces and one `.` per null cell. The areas size the explicit grid (§7.1, the tracks no template sizes taking `grid-auto-*`) and name their lines `<area>-start` / `-end` (§7.3.2), which `grid-area: <name>` and the other placement properties find (§8.3), absolutely positioned boxes too (C7-GRID-AREAS). | — | `layout/grid_areas.rs`, `V/grid_areas.rs`, `rdom-tui/src/render/layout_pass/grid/template.rs` |
| `grid-template` | Supported | `none | <'grid-template-rows'> / <'grid-template-columns'> | [ <line-names>? <string> <track-size>? <line-names>? ]+ [ / <explicit-track-list> ]?` (§7.4): the areas form sets the areas, a row track per string (`auto` when omitted, names on either side of a row joining the line) and the columns (`none` when omitted, no `repeat()`); serialized in the shortest form, `auto` row sizes left out, or not at all when the longhands have no form of it (repeating rows beside areas); `cssText` lists it over its longhands (C7-GRID-AREAS). | — | `V/grid_shorthand.rs`, `DISP/grid.rs` |
| `grid-auto-columns` / `grid-auto-rows` | Supported | `<track-size>+` (cells, `%`, `calc()`, `fr`, the keywords, `minmax()`, `fit-content()`), initial `auto`, serialized as written: the implicit tracks' sizes, repeated as a pattern forwards after the explicit grid and backwards before it (CSS Grid 2 §7.6, C7-GRID-AUTO). | Yes | `V/grid.rs`, `DISP/grid.rs`, `rdom-tui/src/render/layout_pass/grid/mod.rs` |
| `grid-auto-flow` | Supported | `[ row | column ] || dense`, initial `row`, serialized in the shortest form (`row dense` as `dense`): the auto-placement algorithm fills rows or columns, sparse or dense (CSS Grid 2 §7.7 / §8.5, C7-GRID-PLACE). | — | `V/grid_placement.rs`, `rdom-tui/src/render/layout_pass/grid/placement.rs` |
| `grid` | Supported | `<'grid-template'> | <'grid-template-rows'> / [ auto-flow && dense? ] <'grid-auto-columns'>? | [ auto-flow && dense? ] <'grid-auto-rows'>? / <'grid-template-columns'>` (§7.8), resetting the implicit grid's properties it does not name (not the gutters); serialized as the `grid-template` form when the implicit properties are initial, else the `auto-flow` form that holds them (C7-GRID-AREAS). | — | `V/grid_shorthand.rs`, `DISP/grid.rs` |
| `grid-row` / `grid-column` (+ `-start` / `-end`) | Supported | `<grid-line>` (`auto`, `<integer>` but 0 — negative from the explicit grid's end — `<custom-ident>`, `<integer> <custom-ident>`, `span <integer> || <custom-ident>`) and the shorthands, serialized in the shortest form; placed per §8.3 — a lone ident first matching `<ident>-start` / `-end`, missing named lines taken from the implicit grid — with §8.3.1's conflict handling, lines past or before the explicit grid adding implicit tracks (§7.5; lines clamped to ±10 000), and the §8.5 auto-placement algorithm; an absolutely positioned box whose containing block is a grid container takes the grid area its lines name, an `auto` or missing line the containing block's edge (§9.1) (C7-GRID-PLACE). | — | `V/grid_placement.rs`, `rdom-tui/src/render/layout_pass/grid/placement.rs` |
| `grid-area` | Supported | `<grid-line> [ / <grid-line> ]{0,3}` (row-start / column-start / row-end / column-end, omitted ones copying a lone ident, §8.4), placed by its four lines (C7-GRID-PLACE); a named area's lines come from `grid-template-areas` (§3.9 row above, C7-GRID-AREAS), so `grid-area: <name>` fills the area. | — | `V/grid_placement.rs`, `rdom-tui/src/render/layout_pass/grid/placement.rs` |
| `subgrid` (Grid 2) | Supported | `subgrid <line-name-list>?` on `grid-template-columns` / `-rows` (§9: `[ <line-names> | repeat( <integer> | auto-fill , <line-names>+ ) ]*`), parsed and serialized as written. A grid item that is a grid container takes, on each subgridded axis, the tracks its area spans in its parent — in its own direction, its edge tracks less its margin, border and padding, its own `normal` gap the parent's and any other taking half the difference off each side of the parent's gutters — the parent's line names merged with its own, its implicit grid clamped to the span (an auto-placed one spanning its name list); it is stretched there whatever its alignment or size. Its items size the parent's tracks in its place (§9.5) with its margin, border and padding (and gap difference) as extra margin at its edges, an empty edge contributing that alone; on an axis it does not subgrid it is measured with the other inherited, once a pass (nested subgrids cost their depth, not its square). No parent grid (or absolutely positioned): `none`. Its items' baselines align among themselves (DIVERGENCES §2) (C7-SUBGRID). | — | `layout/grid.rs`, `V/grid.rs`, `rdom-tui/src/render/layout_pass/grid/subgrid.rs`, `grid/places.rs` |
| `masonry` / `grid-lanes` (Grid 3, WD) | Missing | Low priority. | Yes | grid |

### 3.10 Positioned layout (Position 3, CSS 2.1 §9)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `position` | Supported | `static` / `relative` / `absolute` / `fixed` / `sticky` (sticky containing block simplified, documented); an absolutely positioned box's — or `::before` / `::after`'s — containing block is its nearest non-`static` ancestor's padding box less its scrollbar gutter, in a scroll container's scrolled content (CSS 2.1 §10.1, CSS Position 3 §2.1; C7-ABSPOS-PADDING-EDGE, C8-CB-COMPLETE), a grid container's grid area within it (CSS Grid 2 §9.1); `fixed` the viewport. An absolutely positioned element counts in the scrollable overflow of the nearest scroll container at or above its containing block (CSS Overflow 3 §2.2; C8-ABSPOS-OVERFLOW; positioned pseudo-elements do not, documented). | Yes | `V/keyword.rs::parse_position`, `POS` |
| `top` / `right` / `bottom` / `left` | Supported | `auto`, signed cells, `%`, `calc()` (C2-PERCENT / C8-INSETS). A positioned box's size honours `min-*` / `max-*`; a relative one — element or pseudo-element — only shifts, the inline-start inset winning when both are set (CSS 2.1 §9.4.3; C8-POS-MINMAX: C5-POS-MINMAX + C5G-REL-PSEUDO-INSETS). | — | `V/length.rs::parse_length` |
| `inset` | Supported | 1–4 values of `auto` / signed cells / `%` / `calc()` (C2-PERCENT / C8-INSETS). | — | `V/length.rs::parse_inset_shorthand` |
| `inset-block` / `inset-inline` (+ `-start` / `-end`) | Supported | Block axis → `top` / `bottom`, inline axis → `left` / `right` by `direction` (C5-LOGICAL). | — | `DISP` (`logical.rs`) |
| `z-index` | Supported | `auto` / any `<integer>`, a value past `i32` clamped (CSS Values 4 §5.1; C8-Z-INDEX); a numeric value stacks a positioned box, and a static flex or grid item too (CSS Flexbox §5.4, CSS Grid 2 §6.5, C7-GRID-PLACE). | Yes | `V/number.rs::parse_z_index` |
| `float` / `clear` | Supported | `float: none \| left \| right \| inline-start \| inline-end`, `clear: none \| left \| right \| both \| inline-start \| inline-end` (CSS 2.1 §9.5–§9.5.2, §9.7, §10.6.7, Appendix E step 5; CSS Logical 1 §2.3; C8-FLOAT): §9.5.1's placement, line boxes shortened beside floats (a line with no room moves below them), floats in inline content on the current line or the next, clearance (which also keeps a cleared first child's margin from collapsing with its parent's, C8G-CLEARANCE-COLLAPSE), block formatting context roots containing their floats and avoiding the parent context's, `display: contents`, blockification, relative offsets that move a float without moving its exclusion; floats paint between the in-flow block backgrounds and the inline content (Appendix E steps 4 / 5 / 7, C8G-PAINT-PHASES) and are hit-tested above the in-flow content; intrinsic sizes measure them (as a formatting context of the measured box's own) with block layout's own flow — margins collapsing, `box-sizing`, `min-*` / `max-*`, the clamp (C8G-FLOAT-MEASURE); a formatting context root dodges floats at its laid-out height, and a float met in inline content settles to its laid-out height; `margin-trim` trims the floats at the edges, line clamping hides what passes the clamp point, `text-overflow` marks the line box's edge beside a float. Flex and grid items do not float (nor the root's children, rdom's viewport-column items); simplifications in DIVERGENCES. | Yes | `V/float.rs`, `layout_pass/float/` |
| `clip` (CSS 2.1, deprecated) | N/A | Superseded by `clip-path`; no new content uses it. | — | — |

### 3.11 Overflow, scrolling and scrollbars (Overflow 3/4, Scroll Snap 1, Overscroll 1, Scrollbars 1)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `overflow` | Supported | `visible` / `hidden` / `clip` / `scroll` / `auto`, one or two values (x then y); a `visible` / `clip` axis beside a scrolling one computes to `auto` / `hidden` (CSS Overflow 3 §3.1; C8-OVERFLOW-CLIP). `clip` clips per axis at the overflow clip edge, is no scroll container (no scrolling, no scrollbar) and no formatting context. A scroll container's scrollport is its padding box less the scrollbar gutters (§5.2), its scrollable overflow area the scrollport ∪ its content plus the end padding, from the scroll origin (§2.2) — one answer for layout, the runtime, snapping and paint (C8G-SCROLLPORT). | — | `V/keyword.rs::parse_overflow_shorthand` |
| `overflow-x` / `overflow-y` | Supported | The five keywords (C8-OVERFLOW-CLIP). | — | `V/keyword.rs::parse_overflow` |
| `overflow-block` / `overflow-inline` | Supported | `overflow-y` / `overflow-x` in `horizontal-tb`, one storage (C8-OVERFLOW-CLIP). | — | `DISP` (`logical.rs`) |
| `overflow-clip-margin` | Supported | `<visual-box> \|\| <length [0,∞]>` in whole cells, on `clip` axes (CSS Overflow 3 §3.2; C8-OVERFLOW-CLIP); a viewport-relative length is rejected (DIVERGENCES). | Yes | `V/keyword.rs::parse_overflow_clip_margin`, `layout_pass/clip_edge.rs` |
| `text-overflow` | Supported | `clip` / `ellipsis` / `<string>`, one value (the end edge) or two (line-left, line-right), per line box of a block whose inline axis clips: whole characters hidden, markers counted in cells, the first character clipped, copy unaffected (CSS Overflow 4 §3; C8-TEXT-OVERFLOW). `fade` / `fade()` not parsed (sub-cell, DIVERGENCES). | Yes | `V/keyword.rs::parse_text_overflow`, `PAINT/inline_paint/text_overflow.rs` |
| `line-clamp` / `max-lines` / `block-ellipsis` / `continue` | Supported | The shorthand and its longhands, and the legacy `-webkit-line-clamp` with `display: -webkit-box` / `-webkit-inline-box` and `-webkit-box-orient` (CSS Overflow 4 §4; C8-LINE-CLAMP): a block container's automatic height ends after its Nth line box — its own, its anonymous boxes' or a block descendant's in its formatting context — what follows is hidden, and that line ends with the `block-ellipsis`, giving up whole characters for it. `discard` clamps as `collapse` (no fragmentation); a clamped flex or grid item whose lines are in block descendants is measured unclamped (DIVERGENCES); the prefixed declarations followed by the standard `line-clamp` clamp, a `-webkit-box` without a clamp is a flex container along its `-webkit-box-orient`, and the clamp cuts the scrollable overflow as it does paint (C8G-WEBKIT-CLAMP). A `::before` / `::after` box clamps its own lines — the rest not laid out, the Nth taking its `block-ellipsis` (C9G-PSEUDO-CLAMP). | Yes | `V/line_clamp.rs`, `layout_pass/line_clamp.rs`, `PAINT/inline_paint/text_overflow.rs` |
| `scroll-behavior` | Supported | `auto` / `smooth` (fixed curve, documented). | Yes | `V/keyword.rs` |
| `scrollbar-gutter` | Supported | `auto` / `stable` / `stable both-edges` (CSS Overflow 3 §3.3; C8-SCROLLBAR): `stable` reserves the vertical bar's gutter on an `overflow: hidden / scroll / auto` box whether or not a bar shows, `both-edges` a matching gutter on the opposite inline edge, left blank; layout, the containing block and intrinsic sizes count both. | — | `V/scrollbar.rs`, `layout_pass/gutter.rs` |
| `scrollbar-width` | Supported | `auto` / `thin` / `none` (CSS Scrollbars 1 §3; C8-SCROLLBAR): `none` — no bar, no gutter, still scrollable; `thin` — the one-cell bar drawn lighter (no track glyph, a light thumb; DIVERGENCES §1). Setting it turns the `::scrollbar*` pseudo-elements off (Chromium's precedence). | Yes | `V/scrollbar.rs`, `PAINT/scrollbar.rs` |
| `scrollbar-color` | Supported | `auto` / `<color> <color>` (thumb, track; CSS Scrollbars 1 §2; C8-SCROLLBAR), inherited, the colors resolved against the element where the bar paints (`currentcolor`, `var()`, `light-dark()`): the track cells filled with the track color, the thumb glyph in the thumb color. Setting it turns the `::scrollbar*` pseudo-elements off (Chromium's precedence). | — | `V/scrollbar.rs`, `PAINT/scrollbar.rs` |
| `overscroll-behavior` (+ `-x` / `-y` / `-block` / `-inline`) | Supported | `auto` / `contain` / `none`, the shorthand's one or two values (`x`, `y`), the logical longhands as `x` / `y` in `horizontal-tb` (CSS Overscroll Behavior 1 §3; C8-OVERSCROLL): a wheel scroll a scroll container cannot take on an axis chains to its scrollable ancestor under `auto`, stops there under `contain` / `none` (alike: no overscroll affordance in a terminal) — on every scroll container, overflowing or not, `overflow: hidden` included (§3, Chromium 144); a tick a mandatory snap holds in place does not chain (C8G-SNAP-TALL). Keyboard scrolling never chains (DIVERGENCES). | Yes | `V/scroll.rs`, `RT/router/mouse` |
| `scroll-padding*` / `scroll-margin*` | Supported | Every longhand, the flow-relative longhands and the shorthands (CSS Scroll Snap 1 §4; C8-SCROLL-PADDING): `scroll-padding` `auto` (0) or a non-negative length-percentage of the scrollport, insetting the optimal viewing region; `scroll-margin` a signed length in cells, outsetting the box's scroll snap area. `scrollIntoView` and keyboard / programmatic focus (HTML's focusing steps, `nearest`) align the area in the region; snapping reads both (C8-SNAP). | Yes | `V/scroll.rs`, `RT/scrollbar/into_view.rs` |
| `scroll-snap-type` / `scroll-snap-align` / `scroll-snap-stop` | Supported | `none \| [x \| y \| block \| inline \| both] [mandatory \| proximity]?`, `[none \| start \| end \| center]{1,2}`, `normal \| always` (CSS Scroll Snap 1 §5–§6; C8-SNAP): snap positions from each box the container is the snap container of, its snap area (`scroll-margin`) aligned in the snapport (`scroll-padding`), clamped to the range; snapping after the wheel, the keyboard, a track click, a released thumb drag, `scrollTo` / `scrollBy` / `scrollIntoView` (a smooth scroll lands snapped), and again after a layout change (§5.4); `scroll-snap-stop: always` not passed; `proximity` within 2 cells (DIVERGENCES). | Yes | `V/scroll.rs`, `RT/scroll_snap/` |
| `scroll-timeline*` / `view-timeline*` / `animation-timeline` / `animation-range*` | Missing | Scroll-driven animations; low priority, needs `@keyframes`. | Yes | `TR` |

### 3.12 Inline text (Text 3/4, Inline 3, CSS 2.1 §10.8)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `white-space` | Supported | Every Text 4 §3 form (`normal` / `pre` / `pre-wrap` / `pre-line` / `nowrap` / `break-spaces`, the longhand pair), a shorthand of the two longhands, serialized shortest; per element (C9-WHITE-SPACE). `white-space-trim` not parsed (DIVERGENCES §2). | — | `DISP/text.rs`, `IFC` (`inline/white_space.rs`, `packer/intake.rs`) |
| `white-space-collapse` / `text-wrap-mode` (Text 4) | Supported | Every value: §4.1 collapsing, preserved segment breaks, hanging (`pre-wrap`) and space-taking (`break-spaces`) trailing spaces (C9-WHITE-SPACE). | — | `DISP/text.rs`, `IFC` |
| `text-wrap` / `text-wrap-style` (Text 4) | Supported | The shorthand of `text-wrap-mode` and `text-wrap-style`; `auto` / `stable` greedy, `balance` (groups of up to six lines), `pretty` / `avoid-short-last-line` (no one-word last line) — the rules in DIVERGENCES §2 (C9-TEXT-WRAP). | — | `DISP/text.rs`, `IFC` (`inline/wrap.rs`) |
| `text-align` | Supported | The Text 3 shorthand of `text-align-all` and `text-align-last`: `start` / `end` (by `direction`) / `left` / `right` / `center` / `justify` / `match-parent` / `justify-all`, within the line box beside floats and past the indent; an overflowing line start-aligned (C9-TEXT-ALIGN). | — | `DISP/text.rs`, `IFC` (`inline/align.rs`) |
| `text-align-last` | Supported | Every value; the last line and the lines before a forced break (C9-TEXT-ALIGN). | — | `DISP/text.rs`, `IFC` |
| `text-justify` | Supported | `auto` (word separators and CJK gaps) / `none` / `inter-word` / `inter-character` (`distribute` its alias), whole cells (C9-TEXT-ALIGN). | — | `DISP/text.rs`, `IFC` |
| `text-indent` | Supported | `<length-percentage> && hanging? && each-line?`, either sign, whole cells; the first formatted line, lines after a forced break with `each-line`, inverted by `hanging`; counted in intrinsic sizes (C9-TEXT-INDENT; anonymous flex / grid items not indented, DIVERGENCES §2). | — | `DISP/text.rs`, `IFC` (`inline/indent.rs`) |
| `text-transform` | Supported | Every value and combination: full Unicode case mapping (Final_Sigma, titlecase for `capitalize`), `full-width` (2 cells), `full-size-kana`, `math-auto`; rendering only — caret, selection, copy read the source (C9-TEXT-TRANSFORM). | — | `DISP/text.rs`, `IFC` (`inline/transform.rs`) |
| `tab-size` | Supported | `<number>` / `<length>`, whole cells; a preserved tab advances to the next stop from the block's content edge, `0` hides tabs (C9-TAB-SIZE). | — | `DISP/text.rs`, `IFC` (`packer/emit.rs::layout_tabs`) |
| `word-break` | Supported | `normal` / `break-all` / `keep-all` / `break-word` (C9-BREAKING; the UAX #14 subset in DIVERGENCES §2). | — | `DISP/text.rs`, `IFC` (`inline/breaking.rs`) |
| `overflow-wrap` / `word-wrap` | Supported | `normal` / `break-word` / `anywhere`, `word-wrap` a legacy alias; `anywhere`'s breaks count for min-content, `break-word`'s do not (C9-BREAKING). | — | `DISP/text.rs`, `IFC` (`packer/emit.rs::split_word`) |
| `line-break` | Supported | `auto` / `loose` / `normal` / `strict` / `anywhere`; the CJK rules read off the characters (no `lang`, DIVERGENCES §2) (C9-BREAKING). | — | `DISP/text.rs`, `IFC` (`inline/breaking.rs`) |
| `hyphens` | Supported | `none` / `manual` / `auto` (= `manual`, no dictionary): a line broken at a soft hyphen shows `-` (C9-BREAKING). | — | `DISP/text.rs`, `IFC` (`packer/fragments.rs::show_hyphen`) |
| `letter-spacing` / `word-spacing` | Supported | `normal \| <length>` in whole cells (CSS Text 3 §9.1, §9.2): blank cells after each grapheme (letter), after each word separator (word), none at a line's end or inside a cursive script; a fraction floors, a negative length is none, a pixel / font-relative length or percentage invalid (DESIGN "Pixel lengths select"); the source map, caret, selection, justification, intrinsic sizes and `text-overflow` read the spacing as its unit's (C9G-LETTER-SPACING; DIVERGENCES §1). | — | `V/text.rs`, `DISP/text.rs`, `IFC` (`packer/spacing.rs`) |
| `hanging-punctuation` | N/A | Hanging a glyph into the margin is a typographic nicety without a TUI use. | — | — |
| `line-height` | Supported | `normal` / `<number>` / `<length-percentage>` in whole rows (floored, at least one — C9G-LINE-HEIGHT-FLOOR); half-leading around the glyph row, the odd row below; each inline box, `::before` / `::after` text and the block's strut; inline-block baselines, intrinsic heights, `line-clamp`, scroll extents, caret and hit-testing follow (C9-LINE-HEIGHT; DIVERGENCES §2). | — | `DISP/text.rs`, `V/inline.rs`, `IFC` (`packer/frames.rs`, `vertical.rs`, `baselines.rs`) |
| `vertical-align` | Supported | Every value on inline elements, `::before` / `::after` text and atomic inlines, whole rows: `sub` / `super` one row, lengths and percentages (of the line height) raise, `middle` / `text-top` / `text-bottom` against the glyph row, `top` / `bottom` aligned subtrees (C9-VERTICAL-ALIGN; DIVERGENCES §2). Table cells: C13-TABLE-PROPS (§3.20). | — | `V/inline.rs`, `IFC` (`packer/frames.rs`) |
| `dominant-baseline` / `alignment-baseline` / `baseline-shift` / `baseline-source` | N/A | One text baseline per row. | — | — |
| `initial-letter` | N/A | Multi-row drop caps need scaled glyphs. | — | — |
| `text-emphasis*` | N/A | Emphasis marks sit above / below a glyph, inside the same cell. | — | — |
| `text-shadow` | N/A | Sub-cell glyph shadow. | — | — |

### 3.13 Text decoration (Text Decoration 3/4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `text-decoration` | Supported | The Text Decoration 4 shorthand of line, thickness, style and color, any order, serialized shortest; propagated to in-flow descendants' text in the decorating box's color and style (§2.1), not into atomic inlines or out-of-flow boxes (C9-DECORATION; DIVERGENCES §2). | — | `V/text_decoration.rs`, `DISP/text_decoration.rs`, `CASC/text_decoration.rs`, `PAINT/text.rs` |
| `text-decoration-line` | Supported | `none` / `underline` / `overline` / `line-through` / `blink` combined: SGR 4, 53, 9, 5 — 53 only to a terminal that has it (C9-DECORATION). `spelling-error` / `grammar-error` not parsed. | — | `DISP`, `SGR` |
| `text-decoration-color` | Supported | The underline's color, SGR 58, to a terminal that has it; the overline and line-through take the text's color (no SGR) (C9-DECORATION). | — | `DISP`, `CASC/colors.rs`, `SGR` |
| `text-decoration-style` | Supported | The underline's `4:1`–`4:5` (`solid` / `double` / `wavy` = curly / `dotted` / `dashed`) to a terminal that has them, else a plain underline (C9-DECORATION). | — | `DISP`, `SGR` |
| `text-decoration-thickness` / `text-underline-offset` / `text-underline-position` | N/A | The terminal draws decorations; position and thickness are not addressable. Parsed and cascaded, inert (C9-DECORATION). | — | `DISP/text_decoration.rs` |
| `text-decoration-skip-ink` / `-skip` | N/A | Font-outline dependent. `-skip-ink` parsed and inert (C9-DECORATION); `-skip` not parsed. | — | `DISP/text_decoration.rs` |

### 3.14 Fonts (Fonts 4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `font-weight` | Supported | `normal` / `bold` / `bolder` / `lighter` / `<number [1,1000]>`, the relative keywords by §2.2's table against the parent's weight; SGR bold from 600, lighter weights normal (C9-FONT; DIVERGENCES §2). | — | `V/font.rs`, `DISP/font.rs`, `CASC/font.rs` |
| `font-style` | Supported | `normal` / `italic` / `oblique <angle [-90deg,90deg]>?`; SGR italic, `oblique 0deg` upright (C9-FONT). | — | `V/font.rs`, `DISP/font.rs` |
| `font` | Supported | The full grammar incl. `/ line-height` and the system font keywords: weight and style honoured, `line-height` reset, size / family / stretch / variant kept inert; serialized shortest (C9-FONT). | — | `V/font.rs`, `DISP/font.rs` |
| `font-family` / `font-size` / `font-stretch` (`font-width`) / `font-size-adjust` / `font-optical-sizing` / `font-kerning` / `font-feature-settings` / `font-variation-settings` / `font-language-override` / `font-synthesis*` / `font-palette` / `font-variant*` | N/A | The terminal owns the font; one monospaced face at one size (documented). `font-family`, `font-size`, `font-stretch` / `font-width` and `font-variant` (CSS 2.1's form) parse, cascade and serialize, inert (C9-FONT); the others are not parsed. | — | `DISP/font.rs` |
| `@font-face` / `@font-feature-values` / `@font-palette-values` | N/A | Same. | — | — |

### 3.15 Lists, counters and generated content (Lists 3, Generated Content 3, Counter Styles 3, Pseudo-Elements 4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `content` | Supported | `none` / `normal` / `<string>` / `attr()` (any form, C2-ATTR) / `var()` / `counter()` / `counters()` / `open-quote` / `close-quote` / `no-open-quote` / `no-close-quote`, concatenated, with alt text after `/` (strings, counters, `attr()`; `ComputedStyle::content_alt`, not painted). On an element it is computed and generates nothing, as in every engine (DIVERGENCES §2); images and the paged-media items are N/A (C10-CONTENT). | — | `V/content.rs::parse_content`, `CASC/content.rs` |
| `quotes` | Supported | `auto` (the content language's marks, a CLDR subset: DIVERGENCES §2) / `none` / `match-parent` / `[<string> <string>]+`, inherited; the depth runs in tree order (C10-QUOTES). | — | `rdom-style/src/quotes.rs`, `CASC/quotes.rs` |
| `counter-reset` | Supported | `none` / `[<counter-name> <integer>? \| reversed(<counter-name>) <integer>?]+`; a reversed counter without an integer computes its initial value from its scope (CSS Lists 3 §4.2); a counter replaces its previous sibling's same-named one (§4.5) (C10-COUNTERS). | — | `V/content.rs::parse_counter_ops`, `CASC/counters/` |
| `counter-increment` | Supported | `none` / `<ident> <integer>?` list. | — | `V/content.rs::parse_counter_ops` |
| `counter-set` | Supported | Sets after reset and increment (§4.4), instantiating a counter not in scope; `<li value>` maps to it (C10-COUNTERS). | — | `CASC/counters/` |
| `counter()` styles | Supported | Every simple predefined style of CSS Counter Styles 3 §6 (numeric in every script, alphabetic, additive, symbolic, fixed), `none`, any name (undefined: `decimal`); one table-driven generator with range, fallback, `negative`, `pad`, a 60-code-point cap. The complex styles of §7 format as `decimal` (DIVERGENCES §2) (C10-COUNTERS). | — | `rdom-style/src/counters/` |
| `counters()` | Supported | `counters(<name>, <string>[, <counter-style>])`: every counter of the name in scope, outermost first, joined (C10-CONTENT). | — | `V/content.rs`, `CASC/counters.rs` |
| `@counter-style` / `symbols()` | Supported | Every descriptor (`system` incl. `fixed <n>` / `extends`, `symbols`, `additive-symbols`, `negative`, `prefix`, `suffix`, `range`, `pad`, `fallback`, `speak-as` — inert); the last definition of a name wins by layer, sheet and source order; the six protected names and `none` are refused; `symbols()` with every `<symbols-type>`. Image symbols are N/A (DIVERGENCES §2) (C10-COUNTER-STYLE). | — | `rdom-css/src/counter_style.rs`, `rdom-style/src/counters/` |
| `list-style-type` / `list-style-position` / `list-style` | Supported | `<counter-style> \| <string> \| none`, `inside \| outside`, the shorthand; inherited. `outside` hangs the marker beside the item's first line, its end at the item's border edge — in the list's UA `padding-inline-start` of four cells (HTML's 40px; a wider marker overflows the list's box, C10G-MARKER-CLIP); `inside` makes it the line's first inline box (C10-LIST-ITEM). | — | `V/list.rs`, `IFC` (`inline/markers.rs`) |
| `list-style-image` | N/A | Images. | — | — |
| `marker-side` | Supported | `match-self` hangs an outside marker on the item's inline-start side, `match-parent` on its parent's (C10-LIST-ITEM). | — | `V/list.rs`, `inline/markers.rs` |
| `string-set` / `bookmark-*` / `running()` / `content()` / `leader()` / `target-counter()` | N/A | Paged-media features. | — | — |

### 3.16 Pseudo-elements (Pseudo-Elements 4, Selectors 4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `::before` / `::after` | Supported | Inline, block-first, positioned — an absolutely or fixed positioned one a box placed as a positioned element is (containing block, grid area, intrinsic sizes, `min-*` / `max-*`, static position), stacked by its own `z-index` in its host's context, hit-tested to its host and counted in its scroll container's overflow; a relative or sticky one laid out in flow and moved (C10-PSEUDO-UNIFY; the moved one paints with its flow, DIVERGENCES §2); `display` and `float` honoured (CSS Pseudo 4 §2) — initial `inline`, `none` generates nothing, a block-level one a block box of its host's flow (`content: ""` an empty one, `clear` applying: the clearfix; C8G-PSEUDO-BOXES), an `inline-block` / `inline flow-root` / `inline-flex` / `inline-grid` one an atomic inline of its host's line, a floated one a float, and a `flex` / `grid` one lays its content — one anonymous item — out as a flex or grid container (C8G-PSEUDO-ATOMS); `display: table` waits for the table formatting context (Phase 13, DIVERGENCES). A pseudo-element with no compound before it attaches to the implicit `*` (`::before`, `div ::before`, `div > ::after`; Selectors 4 §5.2, C5G-BARE-PSEUDO). An element whose only content is its `::before` / `::after` shows it, a line tall (C5G-PSEUDO-ONLY). | Yes | `PE` |
| `::selection` | Supported | A highlight pseudo-element (CSS Pseudo 4 §3): rules cut to `color`, `background-color` and the text decoration properties, painted as the topmost highlight overlay through the one path `::highlight()` uses (C10-HIGHLIGHT; DIVERGENCES §2). | — | `PE`, `inline_paint/highlight_overlay.rs` |
| `::placeholder` | Supported | Layered on the host's `::before` box (documented); the `::first-line` properties, a `var()` value included (C6G-CSSOM-EDGES). | Yes | `PE` |
| `::backdrop` | Supported | Modal dialogs (no top layer, documented). | Yes | `PE` |
| `::marker` | Supported | Rules cut to CSS Lists 3 §3.2's properties; `content` (`normal`: `list-style-type`'s text), color, font, `white-space`, `direction`; laid out as generated content through the packer, riding the item's first line box wherever it is; a point on an outside marker hits its item (`::marker:hover` included, C10G-MARKER-HIT); `<ol type>` / `<ul type>` / `<li type>` are presentational hints (HTML §15.3.8); transitions do not run (DIVERGENCES §3) (C10-LIST-ITEM). A list-item `::before` / `::after` has its own, `::before::marker` / `::after::marker`, riding its box's lines (C10G-PSEUDO-MARKER). An inline list item's marker is its first inline box (C10G-INLINE-LIST-ITEM). | — | `PE`, `CASC`, `inline/markers.rs` |
| `::first-line` / `::first-letter` | Supported | The selectors parse (both spellings, C10-LEGACY-COLON); their rules keep CSS Pseudo 4 §2.2.1's / §2.3.1's properties and are cascaded onto block containers. `::first-line` styles the first formatted line — the block's own, or its first block child's — through the fictional tag sequence (inline children and `::before` inherit it): color, background, font weight / style and decorations at paint, `text-transform` / `letter-spacing` / `word-spacing` in the packer, which switches style at the line's end and reshapes the word carried to the next line. `::first-letter` takes the first typographic letter unit (§2.3.2: punctuation, a letter or number, punctuation), in `::before` text or the DOM's, as a run of its own — or, floated, as a float (a drop cap: `float`, `line-height`, padding). The DOM text is not split, so the caret, selection and copy stay on the source (C10-FIRST; DIVERGENCES §2). | — | `PE`, `IFC`, `inline/first_line.rs`, `inline/first_letter.rs` |
| Legacy single-colon `:before` / `:after` / `:first-line` / `:first-letter` | Supported | Selectors 4 §15: each is its pseudo-element, in any ASCII case, with a pseudo-element's specificity; an escaped colon (`.a\:before`) stays part of an identifier. `:first-line` / `:first-letter` match nothing until `::first-line` / `::first-letter` do (C10-FIRST). Stylesheet selectors only: `querySelector` rejects every pseudo-element (C10-LEGACY-COLON). | — | `PE` |
| `::highlight()` | Supported | CSS Custom Highlight API 1: `Highlight` / `HighlightRegistry` in rdom-core (`Dom::highlights_mut().set(name, Highlight::new(ranges))`), the ranges live across DOM mutations; `::highlight(name)` styles a registered highlight's text with `color`, `background-color` and decorations, overlapping highlights by priority then registration, the selection above them (C10-HIGHLIGHT; DIVERGENCES §2). | — | `PE`, `rdom-core` (`highlight.rs`), `inline_paint/highlight_overlay.rs` |
| `::details-content` | Supported | The slot of a `<details>` element's content (every child but its first `<summary>`) and a block box of the box tree between the element and that content (`box_tree::slot`): its background, border, padding, sizes and `overflow` (clip, scroll) apply, its content inherits from it, and while the element is closed or the slot is `display: none` the content is hidden — text included (C10-DETAILS-CONTENT, C10G-DETAILS-CONTENT-BOX). Its transitions run as an element's; geometry transitions do not move layout and the closed content computes `display: none` (DIVERGENCES §2). | Yes | `PE`, `cascade/details.rs`, `render/box_tree/slot.rs` |
| `::target-text` | N/A | No URL fragment navigation. | — | — |
| `::spelling-error` / `::grammar-error` | N/A | No spellchecker. | — | — |
| `::file-selector-button` | N/A | `<input type=file>` is not rendered (documented). | — | — |
| `::cue` / `::cue-region` | N/A | No media / captions. | — | — |
| `::part()` / `::slotted()` | N/A | No Shadow DOM (documented). | — | — |
| `::view-transition*` | N/A | View transitions snapshot pixels. | — | — |
| More than one pseudo-element / pseudo-element followed by a pseudo-class (`::before:hover`) | Supported | `::before` / `::after` / `::marker` / `::first-letter` take trailing user-action pseudo-classes (Selectors 4 §3.6.3; `Rule::pseudo_state`, counted as pseudo-classes): `:hover` / `:active` match while the pointer is over / presses that pseudo-element (`HitTestExt::hit_test_pseudo`, kept by the `App` while a sheet reads it), the focus ones parse and never match (C10-PSEUDO-CHAINS; DIVERGENCES §2). The nested pseudo-elements Pseudo-Elements 4 §4 defines, `::before::marker` / `::after::marker` (a list-item `::before` / `::after`'s marker, CSS Lists 3 §3.1), parse — two pseudo-elements in the specificity — and take them too; any other nesting is invalid (C10G-PSEUDO-MARKER). | Yes | `PE::extract_pseudo_chain`, `style/pseudo_pointer.rs`, `hit_test/pseudo.rs` |

### 3.17 Selectors (Selectors 4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `*`, type, `.class`, `#id` | Supported | Type names case-sensitive (documented). | Yes | `SEL` |
| `[attr]`, `=`, `~=`, `\|=`, `^=`, `$=`, `*=` | Supported | HTML case-insensitive attribute list honored. | — | `SEL::AttrOp` |
| Attribute case flags `i` / `s` | Supported | `[x=v i]` folds ASCII case, `[x=v s]` compares exactly, overriding HTML §4.16.2's case-insensitive list either way (Selectors 4 §6.3; `AttrCase`, C11-ATTR-FLAGS). | — | `SEL` |
| Namespace prefixes (`ns\|E`, `*\|E`) | N/A | No namespaces (documented). | — | — |
| Descendant, `>`, `+`, `~` | Supported | Backtracking over candidates (Servo's bounded outcomes); `+` / `~` relate element siblings only (C11-COMBINATORS). | — | `SEL::Combinator` |
| Column combinator `\|\|` | Missing | Cells of a `<col>`; moved to Phase 13 (C13-COLUMN), which brings real table columns. | Yes | `SEL` |
| Selector list `a, b` | Supported | — | — | `SEL` |
| `:not(<complex-list>)` | Supported | Full selector list. | — | `SEL` |
| `:where()` | Supported | Zero specificity. | — | `SEL` |
| `:is()` | Supported | `SimpleSelector::Is` (shared with the nesting `&`); forgiving argument list (an invalid argument is dropped, an empty `:is()` matches nothing); specificity of the most specific argument (Selectors 4 §4.2, §17; C1G-IS-PARSE, landing C11-IS). | — | `SEL` |
| `:has()` | Supported | Relative selectors with `>`, `+`, `~` or none (Selectors 4 §4.5), inside `:is()` / `:not()`; unforgiving, not nestable; specificity of its most specific argument. A per-pass cache (`SelectorCaches`) makes `:has(.x)` over nested anchors linear. Invalidation: the cascade flags anchors; a change a `:has()` argument reads walks the changed element's ancestors (and earlier siblings for `+` / `~`) and restyles flagged anchors only — interaction states included; zero work without a `:has()` rule (`style::has_triggers`, C11-HAS; costs in TECH_DEBT `HAS-COST-1`). | — | `SEL`, `rdom-core/src/query_selector/has.rs`, `style/has_triggers.rs` |
| `:first-child` / `:last-child` / `:only-child` | Supported | An element without a parent matches (Selectors 4 §13.3, Level 4; C11-NTH). | — | `SEL` |
| `:nth-child()` / `:nth-last-child()` (+ `of S`) | Supported | The full An+B microsyntax (CSS Syntax 3 §6.2); a per-pass nth-index cache (`SelectorCaches`) indexes each sibling list once per kind of count; a sibling's change of what `S` reads restyles the siblings (`SiblingTriggers`) (C11-NTH). | — | `SEL`, `rdom-core/src/query_selector/nth.rs` |
| `:nth-of-type()` / `:nth-last-of-type()` | Supported | Through the same cache (C11-NTH). | — | `SEL` |
| `:first-of-type` / `:last-of-type` / `:only-of-type` | Supported | C11-NTH. | — | `SEL` |
| `:empty` | Supported | — | — | `SEL` |
| `:root` | Supported | — | — | `SEL` |
| `:scope` | Supported | The `@scope` root (`Dom::matches_list_in_scope` / `matches_list_with`, inside `:is()` / `:not()` too), `:root` at a sheet's top level, and the node a query method was called on — `query_selector_in` / `query_selector_all_in` / `matches` / `closest` (DOM §4.2.6; C11-SCOPE). | — | `SEL`, `rdom-core/src/query_selector/mod.rs` |
| `:hover` / `:active` / `:focus` / `:focus-within` / `:focus-visible` | Supported | Primary-button `:active` (documented). | Yes | `SEL` |
| `:checked` | Supported | Attribute-reflected (documented). | Yes | `SEL` |
| `:indeterminate` | Supported | A checkbox with its indeterminate flag (reflected into an `indeterminate` attribute; activation clears it, the UA draws `[-]`), a radio whose group has no checked member (restyled when the group changes, `FormStateMarks`), a `<progress>` without `value` (HTML §4.16.3; C11-FORM-STATES). | — | `SEL`, `rdom-core/src/form_pseudo.rs` |
| `:placeholder-shown` | Supported | — | — | `SEL` |
| `:open` | Supported | `<details>` / `<dialog>`. | — | `SEL` |
| `:disabled` / `:enabled` | Supported | HTML "actually disabled". | — | `SEL` |
| `:valid` / `:invalid` / `:required` / `:optional` | Supported | Forms and fieldsets too. | — | `SEL` |
| `:user-valid` / `:user-invalid` | Supported | An `<input>`, `<textarea>` or `<select>` whose user validity is set — the user committed a change (a text field on losing focus after an edit, which now fires `change`; a toggle, step, slider move or pick) or its form's submission was attempted, `novalidate` included — and that is valid / invalid; a form reset clears it (HTML §4.16.3, §4.10.18.1; `Dom::user_validity_state`, `ControlState::UserValidity`; C11-FORM-STATES). | — | `SEL`, `RT` |
| `:read-only` / `:read-write` | Supported | HTML §4.16.3's mutability: a mutable `<input>` that `readonly` applies to, a mutable `<textarea>`, and editing hosts and their editable content (`contenteditable`, inherited, `false` stopping it) are `:read-write`; every other element is `:read-only` (`Dom::is_read_write`, C11-FORM-STATES). | — | `SEL`, `rdom-core/src/form_pseudo.rs` |
| `:in-range` / `:out-of-range` | Supported | A candidate for constraint validation with range limitations — `min` / `max` on `number` and the date-like types (HTML §2.3.5's microsyntaxes; a reversed `time` range wraps midnight), every `range` input — in or out of them (`Dom::range_state`, through the control-state hook; C11-FORM-STATES). | — | `SEL`, `rdom-core/src/control_state.rs`, `validation/dates.rs` |
| `:default` | Supported | A form's default button (its first submit button in tree order), checkboxes / radios checked by default and options selected by default — the defaults rdom keeps beside the live attributes (`Dom::is_default`, `ControlState::DefaultChecked` / `DefaultSelected`); restyled when an earlier submit button arrives or a default changes (`FormStateMarks`; C11-FORM-STATES). | — | `SEL`, `rdom-core/src/form_pseudo.rs` |
| `:modal` | Supported | A `<dialog>` shown with `showModal()`: in the document's top layer as a modal dialog (`Dom::top_layer`, `TopLayerKind::ModalDialog`), rendered above everything, centred by the UA, the document beneath inert — to the pointer, focus, Tab and keys (`Dom::is_inert`, HTML §6.3, with the `inert` attribute); no fullscreen (C11-MODAL-POPOVER, C11G-MODAL-INERT). | — | `SEL`, `rdom-core/src/top_layer.rs`, `render/paint_pass/top_layer.rs` |
| `:popover-open` | Supported | A showing popover — in the top layer as `TopLayerKind::Popover`. The `popover` attribute (`auto` / `manual` / `hint`), `showPopover()` / `hidePopover()` / `togglePopover()`, `beforetoggle` / `toggle`, the auto and hint stacks, `popovertarget` / `popovertargetaction`, focus, light dismiss (a press and release outside, Esc as a close request, nested stacks) and the UA rules (`runtime::builtins::popover`, C11-MODAL-POPOVER). | — | `SEL`, `RT` |
| `:link` / `:any-link` | Supported | `a` / `area` with `href` (HTML §4.16.3); every link is unvisited (C11-LINK-LANG). | — | `SEL` |
| `:visited` / `:local-link` / `:target` / `:target-within` | N/A | No navigation history or URL fragments; `:visited` parses and matches nothing, so `a:link, a:visited` keeps its `:link` half (C11-LINK-LANG). | — | — |
| `:lang()` | Supported | Identifier or string ranges, RFC 4647 §3.3.2 extended filtering against the inherited `xml:lang` / `lang` (`Dom::language`, shared with `quotes: auto`); an empty or absent language is unknown — matched by `""` only (C11-LINK-LANG). | — | `SEL`, `rdom-core/src/language.rs` |
| `:dir()` | Supported | HTML directionality (`Dom::directionality`, HTML §3.2.6.4): `dir`, `auto` / `<bdi>` by the first strong character (Bidi_Class approximated, DIVERGENCES §2), `<input type=tel>`, inheritance — not the CSS `direction` property; another argument matches nothing (C11-LINK-LANG). | — | `SEL`, `rdom-core/src/directionality.rs` |
| `:defined` / `:state()` / `:host*` | N/A | No custom elements / Shadow DOM (documented). | — | — |
| `:autofill`, `:fullscreen`, `:picture-in-picture`, `:playing` / `:paused` / `:seeking` / `:buffering` / `:stalled` / `:muted` / `:volume-locked`, `:current` / `:past` / `:future` | N/A | No autofill, fullscreen, media or timed text. | — | — |
| `:blank` | Missing | Low priority (spec unstable). | No | `SEL` |

### 3.18 Transitions and animations (Transitions 1/2, Animations 1/2, Easing 1/2)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `transition-property` | Supported | `all` / `none` / any property name — a shorthand covers its longhands, a flow-relative name its physical twin by `direction`, the last entry naming a property wins; unknown idents inert. | — | `TR`, `rdom-style/src/animation` |
| `transition-duration` | Supported | `<time>` list. | — | `TR::parse_time_ms` |
| `transition-timing-function` | Partial | `linear` / `ease*` / `step-start` / `step-end` / `cubic-bezier()` / `steps()` with all positions; `linear(<stops>)` (Easing 2) rejected. | No | `TR` |
| `transition-delay` | Partial | Negative delays rejected (Transitions 1 allows them: start part-way). | No | `TR::parse_time_ms` |
| `transition` | Supported | Shorthand list. | — | `TR` |
| `transition-behavior` | Missing | `allow-discrete`. | Yes | `TR` |
| Animatable property set | Supported | Every longhand of the dispatch table animates by its spec's animation type (by computed value, repeatable list, shadow list, discrete, not animatable — `rdom_style::animation`), per longhand, on elements, `::before` / `::after` and `::details-content`; the running value is the computed value layout, paint and inheritance read (C12-ANIMATABLE). `interpolate-size: allow-keywords` and `calc-size()` (CSS Values 5 §10–§11) animate `width` / `height` to and from `auto` and the intrinsic keywords; sums linear in `size` (DIVERGENCES §2). | Yes | `rdom-style/src/animation`, `rdom-tui/src/runtime/animation` |
| `@keyframes` | Missing | — | Yes | `AT`, `TR` |
| `animation` / `animation-name` / `-duration` / `-timing-function` / `-delay` / `-iteration-count` / `-direction` / `-fill-mode` / `-play-state` / `-composition` | Missing | — | Yes | `DISP`, `TR` |
| `@starting-style` | Missing | Entry transitions. | Blanket | `AT`, `TR` |

### 3.19 User interface (UI 4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `caret-color` | Supported | `auto` / `transparent` / `<color>`. | — | `DISP/set.rs` |
| `caret-shape` / `caret-animation` / `caret` | Missing | Caret glyph shape / blink control. | No | `DISP`, `PAINT` caret |
| `user-select` | Supported | `auto` / `text` / `none` / `all` / `contain`. | Yes | `DISP/set.rs` |
| `pointer-events` | Partial | `auto` / `none` (SVG values N/A, documented). | Yes | `DISP/set.rs` |
| `cursor` | Missing | OSC 22 pointer shapes. | No | `DISP`, `RT` |
| `outline` / `outline-color` / `outline-style` / `outline-width` / `outline-offset` | Missing | Non-layout ring outside the border box. | No | `DISP`, `PAINT` |
| `accent-color` | Missing | Control glyph color. | No | `DISP`, `UA` |
| `appearance` | Missing | `none` / `auto`. | No | `DISP`, `UA`, builtins |
| `resize` | Missing | Corner-drag resizing. | No | `RT` |
| `field-sizing` | Missing | Content-sized controls. | No | builtins |
| `nav-up` / `nav-down` / `nav-left` / `nav-right` | Missing | Directional focus (at-risk in UI 4). | No | `RT` focus |
| `input-security` | N/A | Password masking is control behavior, not styling, in a TUI. | — | — |

### 3.20 Tables (Tables 3, CSS 2.1 §17)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `border-collapse` | Supported | See §3.5. | Yes | — |
| `border-spacing` | Partial | See §3.5. | Yes | — |
| `table-layout` | Missing | `fixed` algorithm. | No | `rdom-tui/src/runtime/builtins/table` |
| `caption-side` | Missing | `top` / `bottom`. | No | table builtin |
| `empty-cells` | Missing | `show` / `hide`. | No | table builtin |
| `vertical-align` on cells | Missing | `top` / `middle` / `bottom`. | Yes | table builtin |

### 3.21 Conditional rules and containment (Conditional 3/5, Contain 2/3, Will Change 1)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `@media` | Missing | Cell-sized viewport and preference queries. | Yes | `AT`, `CASC` |
| `@supports` | Missing | Feature queries against the dispatch table. | Blanket | `AT` |
| `@container` / `container-type` / `container-name` / `container` | Missing | Container size queries. | Blanket | `AT`, layout |
| `@page` and page properties | N/A | No paged media. | — | — |
| `contain` | Missing | `paint` / `layout` / `size` / `strict` / `content`. | Yes | layout, `POS` |
| `content-visibility` | Missing | `hidden` / `auto` skip work. | No | layout |
| `will-change` | Missing | Stacking-context hint only. | Yes | `POS` |

### 3.22 Logical properties and writing modes (Logical 1, Writing Modes 4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `inline-size` / `block-size` / `min-*-size` / `max-*-size` | Supported | Their physical twins in horizontal-tb, one storage (C5-LOGICAL). | — | `DISP` |
| `margin-inline` / `margin-block` (+ `-start` / `-end`) | Supported | Block axis → top / bottom when declared; inline axis → left / right by the element's `direction` in the cascade, in declaration order (C5-LOGICAL). | — | `DISP` |
| `padding-inline` / `padding-block` (+ `-start` / `-end`) | Supported | As the margins (C5-LOGICAL). | — | `DISP` |
| `border-inline` / `border-block` (+ `-start` / `-end`, `-color` / `-style` / `-width`) | Supported | As the margins; one or two values for the axis longhands (C5-LOGICAL). | — | `DISP` |
| `border-start-start-radius` / … (4 corners) | Supported | Block side first, the inline side by `direction` (C5-LOGICAL). | — | `DISP` |
| `inset-inline` / `inset-block` | Supported | See §3.10 (C5-LOGICAL). | — | `DISP` |
| `text-align: start / end`, `float: inline-start`, `resize: block / inline` | Missing | Logical keywords (follow their properties). *`float` / `clear: inline-start / inline-end` shipped: C8-FLOAT; `text-align: start / end`: C9-TEXT-ALIGN; `resize` remains (C12-CONTROLS).* | No | — |
| `writing-mode` | Partial | All five values parse, inherit and compute (C5-WRITING); every box lays out as `horizontal-tb` — vertical flow could be emulated, but glyphs cannot be rotated in a cell (DIVERGENCES §1). | Yes | `KW`, `CASC` |
| `direction` / `unicode-bidi` | Partial | `direction: ltr \| rtl` (C5-WRITING; the element's directionality through HTML's `[dir]:dir(…)` / `bdi:dir(…)` UA rules, `dir=auto` by its first strong character, C11-LINK-LANG): inline-start is the right edge — line starts (a line wider than its box overflows the left edge, C8-RTL-LINE-OVERFLOW), block over-constraint, flex rows / column cross axis, positioned insets, `margin-trim`, the vertical scrollbar side, the scroll origin (`scrollLeft` ≤ 0, C5G-RTL-SCROLL). `unicode-bidi` and bidi reordering N/A: terminals differ (DIVERGENCES §1). | Yes | `KW`, `IFC`, `BLOCK`, `FLEX`, `POS` |
| `text-orientation` / `text-combine-upright` | N/A | Glyph rotation / compression in a cell. | — | — |

### 3.23 Transforms, filters, masking, compositing

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `translate` / `transform: translate()` | Missing | Whole-cell paint offset (stacking-context trigger). | Yes | `POS`, `PAINT` |
| `transform` (other functions) / `rotate` / `scale` / `transform-origin` / `transform-box` | N/A | Rotation and scaling of glyphs are impossible in a cell grid (documented). | — | — |
| `transform-style` / `perspective` / `perspective-origin` / `backface-visibility` | N/A | 3D. | — | — |
| `filter` | Missing | Color-matrix functions per cell; `blur()` / `drop-shadow()` / `url()` N/A. | Yes | `PAINT/group.rs` |
| `backdrop-filter` | Missing | Same functions on the backdrop cells. | No | `PAINT/group.rs` |
| `mix-blend-mode` | Missing | Per-cell color blending. | Yes | `PAINT/group.rs` |
| `isolation` | Missing | Stacking-context trigger. | Yes | `POS` |
| `background-blend-mode` | N/A | One background layer (a color); nothing to blend. | — | — |
| `clip-path` | Missing | `inset()` rectangles only; shapes N/A. | No | `PAINT` |
| `mask*` / `mask-border*` / `shape-outside` / `shape-margin` / `shape-image-threshold` | N/A | Image / shape geometry: a float excludes its margin box in whole cells, a shape would need sub-cell edges. | — | — |

### 3.24 Other modules (CSS 2.1 leftovers, Multi-column, Images, Speech, Fragmentation)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `columns` / `column-count` / `column-width` / `column-rule*` / `column-span` / `column-fill` | Missing | Multi-column layout. | No | new layout pass |
| `object-fit` / `object-position` / `image-rendering` / `image-orientation` / `image-resolution` | N/A | No images. | — | — |
| Speech (`speak`, `voice-*`, `pause*`, `rest*`, `cue*`, `azimuth`, …) | N/A | Aural rendering. | — | — |
| Fragmentation (`break-*`, `page-break-*`, `orphans`, `widows`, `box-decoration-break`) | N/A | No paged or multi-column fragmentation. | — | — |
| `zoom` | N/A | No pixel scaling. | — | — |
| `anchor-name` / `position-anchor` / `position-area` / `@position-try` (Anchor Positioning 1, WD) | Missing | Tooltips / popovers anchored to another box; useful, outside the requested module list. | No | `POS` |

---

## 4. rdom extensions (not in any CSS spec)

| Extension | Where | Doc'd |
|---|---|---|
| Unitless integers as cells (`width: 20`, `padding: 1 2`) | `V/length.rs`, `V/spacing.rs` | Yes |
| `fr` on `width` / `height` as a flex weight | `V/length.rs::parse_size` | Yes |
| `caret-text-color` property | `DISP/set.rs` | Yes |
| `border-style: half-block` | `V/border.rs` | Yes |
| `border: rounded` (= `solid` + `border-radius: 1`) and `border: single` (= `solid`) | `V/border.rs::parse_border` | Yes |
| `border: top` / `bottom` / `left` / `right` (one solid side) | `V/border.rs::parse_border` | No |
| Bare integer color `0–255` → xterm-256 palette index (`color: 208`) | `tui_color.rs::parse_simple_color` | No |
| `reset` color keyword (terminal default fg / bg) | `tui_color.rs::parse_simple_color` | No |
| `::scrollbar`, `::scrollbar-thumb`, `::scrollbar-thumb:vertical` / `:horizontal` | `PE` | Yes |
| `row-gap` between a block container's block-level children (CSS Box Alignment 3 §8 applies gaps to flex, grid and multi-column containers only) | `BLOCK/mod.rs` | Yes |

---

## 5. Undocumented gaps

Every *Partial* or *Missing* row above whose Doc'd column is `No` or `Wrong`. These need either an
implementation or a `DIVERGENCES.md` entry before the acid page's coverage test can be honest.

133 rows as audited. Through C6-GAP, 46 have shipped and two have partly shipped
(each annotated *Shipped* where it stands); 85 remain open (Phase 0 listed each of them in
`DIVERGENCES.md` §3).

**3.1 Syntax, cascade and inheritance (Syntax 3, Cascade 4/5, CSS 2.1 §6)**

- Identifier escapes (`\31 0`, `\:`) — Missing: Backslash escapes in identifiers and selectors are not decoded. *Shipped: C1-ESCAPES.*
- Property-name case-insensitivity — Partial: Names are matched exactly: `COLOR: red` is `UnknownProperty` (CSS property names are ASCII case-insensitive). *Shipped: C1-CASE.*
- Keyword case-insensitivity — Partial: `parse_keyword` is case-insensitive, but `text-decoration` (`V/keyword.rs::parse_text_decoration`) matches exactly. *Shipped: C1-CASE.*
- `revert` — Missing: Roll back to the UA-origin value. *Shipped: C1-REVERT.*
- `revert-layer` — Missing: Needs `@layer`. *Shipped: C1-LAYER.*
- `all` — Missing: Shorthand for every property in the table (custom properties excluded). *Shipped: C1-ALL.*

**3.2 Custom properties (CSS Variables 1)**

- *(shipped: C1-VAR-ANY)* `var()` in `content` — was Missing: `parse_content` has no `var()` arm — `content: var(--x)` is dropped. Only the Rust builder `Content::Var` reaches the resolver (`computed.rs`).
- *(shipped: C1-VAR-ANY)* `var()` fallback with arbitrary tokens.

**3.3 Values and units (Values 4)**

- `<percentage>` — Partial *(DIVERGENCES says otherwise)*: `width` / `height` / `gap` / inside `calc()`; rejected bare on `padding` / `margin` (documented), `top` / `right` / `bottom` / `left` / `inset` (doc says accepted), `min-*` / `max-*` / `flex-basis`.
- `round()` / `mod()` / `rem()` / `abs()` / `sign()` — Missing: Stepped-value / sign functions — natural on an integer grid.
- `sin()` … `atan2()`, `pow()` / `sqrt()` / `hypot()` / `log()` / `exp()` — Missing: Numeric functions; low value, cheap once the evaluator is general.
- `lh`, `rlh` — Missing: One row (× `line-height` once that exists). *Shipped: C2-LH with C9-LINE-HEIGHT.*
- `cqw` / `cqh` / `cqi` / `cqb` / `cqmin` / `cqmax` — Missing: Need `@container`.
- `<angle>` (`deg`, `grad`, `rad`, `turn`) — Missing: Only needed for color hues (`hsl()`, `oklch()`); no rotation exists.
- *(shipped: C2-ATTR)* `attr()` — Partial: In `content` only, no fallback, no type (`attr(x type(<length>))`, Values 5).

**3.4 Color (Color 4 / 5)**

- `rgb()` / `rgba()` — Partial: Legacy comma syntax with integer `0–255` channels only; no space syntax, no `/ alpha`, no `%` channels, no fractional numbers, no `none`; alpha dropped (documented). *Shipped: C3-RGB.*
- `transparent` — Partial: Maps to `Color::Reset`: as a background it does not fill (ancestor shows through — correct); as `color` / `border-color` it is the terminal's default foreground, not invisible. *Shipped: C3-TRANSPARENT (transparent black); `background-color`'s initial value since C11G-CANVAS-FILL.*
- `hsl()` / `hsla()` — Missing: Convert to sRGB. *Shipped: C3-HSL-HWB.*
- `hwb()` — Missing: Convert to sRGB. *Shipped: C3-HSL-HWB.*
- `lab()` / `lch()` / `oklab()` / `oklch()` — Missing: Convert + gamut-map to sRGB. *Shipped: C3-LAB.*
- `color()` — Missing: Predefined spaces (`srgb`, `display-p3`, …) converted to sRGB. *Shipped: C3-LAB.*
- `color-mix()` (Color 5) — Missing: Mix at parse time (or computed time with `var()` / `currentColor`). *Shipped: C3-MIX.*
- Relative color syntax (Color 5) — Missing: `rgb(from var(--x) r g b / 50%)`. *Shipped: C3-RELATIVE.*
- System colors (`Canvas`, `CanvasText`, `LinkText`, `ButtonFace`, …) — Missing: `Canvas` / `CanvasText` = terminal default bg / fg (`Color::Reset`); others map to UA palette entries. *Shipped: C3-SYSTEM.*
- `light-dark()` + `color-scheme` — Missing: Pick by the terminal's reported background (OSC 11 / mode 2031). *Shipped: C3-SCHEME (mode 2031 with C3G-INPUT-READER).*
- `opacity` — Partial: `<number>` clamped to 0–1; `<percentage>` (`opacity: 50%`) rejected. Group opacity per cell (documented).

**3.5 Backgrounds and borders (Backgrounds 3, Borders 4)**

- `border` — Partial: One keyword only: a style (`none` / `hidden` / `solid` / `double` / `dashed` / `dotted` / `ridge` / `groove` / `inset` / `outset`) plus rdom keywords; no width or color component — `border: 1px solid red` is dropped. *Shipped: C4-BORDER-SHORTHAND.*
- `border-top` / `-right` / `-bottom` / `-left` — Partial: One style keyword only; no width / color. *Shipped: C4-BORDER-SHORTHAND.*
- `border-style` — Partial: One value applied to all four sides; the 2–4-value form is rejected. *Shipped: C4-BORDER-SIDES.*
- `border-color` — Partial: One color for all sides; 2–4 values rejected. *Shipped: C4-BORDER-SIDES.*
- `border-top-color` / `-right-color` / `-bottom-color` / `-left-color` — Missing: Per-side glyph color. *Shipped: C4-BORDER-SIDES.*
- `border-width` / `border-*-width` — Missing: `0` = none; `thin` / `medium` = light; `thick` = heavy glyphs. *Shipped: C4-BORDER-WIDTH.*
- `border-radius` / `border-*-radius` — Missing: Non-zero → rounded corner glyphs. *Shipped: C4-RADIUS.*
- `box-shadow` — Missing: One-cell offset shade; blur / spread N/A. *Shipped: C4-SHADOW (spread whole cells, blur inert).*
- `border-spacing` — Missing: Gaps between separated table cells. *Partly shipped: C4-SPACING parses and inherits it; the layout lands with C13-TFC.*

**3.6 Box model and sizing (Box 3, Sizing 3/4)**

- `margin-trim` — Missing: Trim children's margins at the container edges. *Shipped: C5-MARGIN-TRIM.*
- `width` / `height` — Partial: `auto`, cells, `%`, `calc()`, rdom `fr`; missing `min-content` / `max-content` / `fit-content` / `fit-content(<l>)` / `stretch`. *Shipped: C5-INTRINSIC (all but `stretch`).*
- `min-width` / `min-height` — Partial: `auto` / cells only; no `%`, `calc()`, intrinsic keywords. *Shipped: C2-PERCENT, C2G-MAX-NONE, C5-INTRINSIC.*
- `max-width` / `max-height` — Partial: Cells / constant `calc()` only; `none` (the initial value), `%`, percent `calc()`, intrinsic keywords rejected. *Shipped: C2-PERCENT, C2G-MAX-NONE, C5-INTRINSIC.*
- `box-sizing` — Missing: rdom is implicitly `border-box`; `content-box` (CSS initial) is not expressible. *Shipped: C5-BOX-SIZING.*
- `contain-intrinsic-size` (+ longhands) — Missing: Placeholder size for `content-visibility: auto`. *Partly shipped: C5-CONTAIN-SIZE parses and cascades them; layout with C14-CONTAIN.*

**3.7 Display and visibility (Display 3)**

- `display: contents` — Missing: No box; children join the parent. *Shipped: C6-DISPLAY-KEYWORDS.*
- `display: flow-root` — Missing: Block that establishes an independent BFC. *Shipped: C6-DISPLAY-KEYWORDS.*
- Multi-keyword `display` (`block flex`, `inline flow-root`) — Missing: Two-value syntax. *Shipped: C6-DISPLAY-KEYWORDS.*
- `visibility` — Missing: `visible` / `hidden` / `collapse`. *Shipped: C6-VISIBILITY.*
- `order` — Missing: Visual reorder of flex / grid items. *Shipped: C6-ORDER.*

**3.8 Flexbox and box alignment (Flexbox 1, Align 3)**

- `flex-direction` — Partial: `row` / `column`; `row-reverse` / `column-reverse` rejected. *Shipped: C6-DIRECTION-REVERSE.*
- `flex-wrap` — Missing: Single-line only. *Shipped: C6-WRAP.*
- `flex-flow` — Missing: Shorthand of the two above. *Shipped: C6-WRAP.*
- `flex-grow` — *Shipped: C6-FLEX-LONGHANDS.* Missing *(DIVERGENCES says otherwise)*: Not in the property table (DIVERGENCES suggests `flex-grow: 1` as a workaround).
- `flex-basis` — Missing: Basis ignored (documented as part of `flex`); the longhand does not exist. *Shipped: C6-FLEX-LONGHANDS.*
- `justify-content` — Missing: Main-axis distribution. *Shipped: C6-JUSTIFY.*
- `align-items` — Missing: Cross-axis placement (always `stretch` unless a cross margin is `auto`). `KW::Align` exists, unused. *Shipped: C6-ALIGN.*
- `align-self` — Missing: Per-item override. *Shipped: C6-ALIGN.*
- `align-content` — Missing: Needs `flex-wrap`. *Shipped: C6-ALIGN-CONTENT.*
- `justify-items` / `justify-self` — Missing: Grid / block-level alignment. *Shipped (block-level): C6-PLACE.*
- `place-content` / `place-items` / `place-self` — Missing: Shorthands. *Shipped: C6-PLACE.*
- `gap` — Partial: One value for both axes (cells, `%`, `calc()`); two-value form rejected. *Shipped: C6-GAP.*
- `row-gap` / `column-gap` — Missing: Per-axis gap. *Shipped: C6-GAP.*

**3.10 Positioned layout (Position 3, CSS 2.1 §9)**

- `top` / `right` / `bottom` / `left` — Partial *(DIVERGENCES says otherwise)*: `auto`, signed cells, `calc()`; bare `%` rejected.
- `inset` — Partial: 1–4 values of `auto` / signed cells; `calc()` and `%` rejected.
- `inset-block` / `inset-inline` (+ `-start` / `-end`) — Missing: Logical aliases. *Shipped: C5-LOGICAL.*

**3.11 Overflow, scrolling and scrollbars (Overflow 3/4, Scroll Snap 1, Overscroll 1, Scrollbars 1)**

- `overflow` — Partial: `visible` / `hidden` / `scroll` / `auto`; `clip` and the two-value form rejected. *Shipped: C8-OVERFLOW-CLIP.*
- `overflow-x` / `overflow-y` — Partial: Same keyword set; no `clip`. *Shipped: C8-OVERFLOW-CLIP.*
- `overflow-block` / `overflow-inline` — Missing: Logical aliases. *Shipped: C8-OVERFLOW-CLIP.*
- `overflow-clip-margin` — Missing: Cells outside the box that `clip` still paints. *Shipped: C8-OVERFLOW-CLIP.*
- `text-overflow` — Missing: `clip` (today) / `ellipsis` / `<string>`. *Shipped: C8-TEXT-OVERFLOW.*
- `line-clamp` / `max-lines` / `block-ellipsis` / `continue` — Missing: Row clamping with `…`. *Shipped: C8-LINE-CLAMP.*
- `scrollbar-gutter` — Partial: `auto` / `stable`; `both-edges` rejected.
- `overscroll-behavior` (+ `-x` / `-y` / `-block` / `-inline`) — Missing: Stop scroll chaining. *Shipped: C8-OVERSCROLL.*
- `scroll-snap-type` / `scroll-snap-align` / `scroll-snap-stop` — Missing: Snap positions. *Shipped: C8-SNAP.*

**3.12 Inline text (Text 3/4, Inline 3, CSS 2.1 §10.8)**

- `white-space` — Partial: `normal` / `pre` / `pre-wrap` / `nowrap`; `pre-line` and `break-spaces` rejected. *Shipped: C9-WHITE-SPACE.*
- `white-space-collapse` / `text-wrap-mode` (Text 4) — Missing: Longhands of `white-space`. *Shipped: C9-WHITE-SPACE.*
- `text-wrap` / `text-wrap-style` (Text 4) — Missing: `balance` / `pretty` / `stable` line breaking. *Shipped: C9-TEXT-WRAP.*
- `text-align-last` — Missing: Last-line alignment. *Shipped: C9-TEXT-ALIGN.*
- `text-justify` — Missing: Justification method (`inter-word` is the only sensible one). *Shipped: C9-TEXT-ALIGN.*
- `text-indent` — Missing: First-line indent in cells. *Shipped: C9-TEXT-INDENT.*
- `text-transform` — Missing: Case mapping, `full-width`. *Shipped: C9-TEXT-TRANSFORM.*
- `word-break` — Missing: `break-all` / `keep-all`. *Shipped: C9-BREAKING.*
- `overflow-wrap` / `word-wrap` — Missing: `anywhere` / `break-word`. *Shipped: C9-BREAKING.*
- `line-break` — Missing: CJK break strictness; low priority. *Shipped: C9-BREAKING.*
- `line-height` — Missing: Whole-row line boxes. *Shipped: C9-LINE-HEIGHT.*

**3.13 Text decoration (Text Decoration 3/4)**

- `text-decoration` — Partial *(DIVERGENCES says otherwise)*: One keyword: `none` / `underline` / `line-through` (SGR 4 / 9); `overline`, combinations, color and style components rejected; case-sensitive. *Shipped: C9-DECORATION.*
- `text-decoration-line` — Missing: Longhand; `overline` via SGR 53. *Shipped: C9-DECORATION.*
- `text-decoration-color` — Missing: SGR 58 underline color. *Shipped: C9-DECORATION.*
- `text-decoration-style` — Missing: SGR 4:1–4:5 (`solid` / `double` / `curly` = `wavy` / `dotted` / `dashed`). *Shipped: C9-DECORATION.*

**3.14 Fonts (Fonts 4)**

- `font-weight` — Partial: `normal` / `bold`; numeric weights, `bolder` / `lighter` rejected. *Shipped: C9-FONT.*
- `font-style` — Partial: `normal` / `italic`; `oblique [<angle>]` rejected. *Shipped: C9-FONT.*
- `font` — Missing: Shorthand: honor weight / style, ignore size / family. *Shipped: C9-FONT.*

**3.15 Lists, counters and generated content (Lists 3, Generated Content 3, Counter Styles 3, Pseudo-Elements 4)**

- `content` — Partial *(DIVERGENCES says otherwise)*: `none` / `normal` / `<string>` / `attr(<ident>)` / `counter(<ident>[, <style>])`, concatenated; missing `counters()`, `open-quote` / `close-quote` / `no-*-quote`, `var()`, alt text (`/ "alt"`).
- `quotes` — Missing: Quote pairs.
- `counter-reset` — Partial: `none` / `<ident> <integer>?` list; `reversed(<ident>)` rejected.
- `counter()` styles — Partial: `decimal`, `lower-alpha` / `lower-latin`, `upper-alpha` / `upper-latin`, `lower-roman`, `upper-roman`; missing `disc` / `circle` / `square` / `disclosure-*`, `decimal-leading-zero`, `lower-greek`, `none`, the other predefined styles.
- `marker-side` — Missing: Low priority.

**3.16 Pseudo-elements (Pseudo-Elements 4, Selectors 4)**

- Legacy single-colon `:before` / `:after` / `:first-line` / `:first-letter` — Missing: CSS 2.1 spellings, still valid; `:before` is an "unsupported pseudo-class" error.
- `::highlight()` — Missing: Custom Highlight API ranges (search hits, diagnostics).
- `::details-content` — Missing: The collapsible part of `<details>`.
- More than one pseudo-element / pseudo-element followed by a pseudo-class (`::before:hover`) — Partial: One trailing suffix per selector; only the rdom `::scrollbar-thumb:vertical/horizontal` forms take a pseudo-class.

**3.17 Selectors (Selectors 4)**

- Column combinator `\|\|` — Missing: Cells of a `<col>`; low priority.
- `:first-of-type` / `:last-of-type` / `:only-of-type` — Missing: —
- `:scope` — Partial: matches the `@scope` root; the query APIs do not set it yet (C11-SCOPE).
- `:indeterminate` — Partial: `<progress>` without `value` only; indeterminate checkboxes / radio groups never match.
- `:in-range` / `:out-of-range` — Missing: Number / range inputs exist.
- `:default` — Missing: Default submit button, default-checked controls / options.
- `:popover-open` — Missing: Needs the `popover` attribute behavior.
- `:link` / `:any-link` — Missing: `<a href>` exists.
- `:lang()` — Missing: `lang` attribute inheritance.
- `:blank` — Missing: Low priority (spec unstable).

**3.18 Transitions and animations (Transitions 1/2, Animations 1/2, Easing 1/2)**

- `transition-timing-function` — Partial: `linear` / `ease*` / `step-start` / `step-end` / `cubic-bezier()` / `steps()` with all positions; `linear(<stops>)` (Easing 2) rejected.
- `transition-delay` — Partial: Negative delays rejected (Transitions 1 allows them: start part-way).
- Animatable property set — Partial: `color`, `background-color`, `border-color`, `width`, `height`, `padding`, `gap`, `top` / `right` / `bottom` / `left`, `z-index`; not `opacity`, `margin`, `min-*` / `max-*`, `inset` as a name.

**3.19 User interface (UI 4)**

- `caret-shape` / `caret-animation` / `caret` — Missing: Caret glyph shape / blink control.
- `cursor` — Missing: OSC 22 pointer shapes.
- `outline` / `outline-color` / `outline-style` / `outline-width` / `outline-offset` — Missing: Non-layout ring outside the border box.
- `accent-color` — Missing: Control glyph color.
- `appearance` — Missing: `none` / `auto`.
- `resize` — Missing: Corner-drag resizing.
- `field-sizing` — Missing: Content-sized controls.
- `nav-up` / `nav-down` / `nav-left` / `nav-right` — Missing: Directional focus (at-risk in UI 4).

**3.20 Tables (Tables 3, CSS 2.1 §17)**

- `table-layout` — Missing: `fixed` algorithm.
- `caption-side` — Missing: `top` / `bottom`.
- `empty-cells` — Missing: `show` / `hide`.

**3.21 Conditional rules and containment (Conditional 3/5, Contain 2/3, Will Change 1)**

- `content-visibility` — Missing: `hidden` / `auto` skip work.

**3.22 Logical properties and writing modes (Logical 1, Writing Modes 4)**

- `inline-size` / `block-size` / `min-*-size` / `max-*-size` — Missing: Aliases of `width` / `height` / `min-*` / `max-*`. *Shipped: C5-LOGICAL.*
- `margin-inline` / `margin-block` (+ `-start` / `-end`) — Missing: Aliases of physical margins. *Shipped: C5-LOGICAL.*
- `padding-inline` / `padding-block` (+ `-start` / `-end`) — Missing: Aliases of physical padding. *Shipped: C5-LOGICAL.*
- `border-inline` / `border-block` (+ `-start` / `-end`, `-color` / `-style` / `-width`) — Missing: Aliases of physical borders. *Shipped: C5-LOGICAL.*
- `border-start-start-radius` / … (4 corners) — Missing: Aliases of `border-*-radius`. *Shipped: C5-LOGICAL.*
- `text-align: start / end`, `float: inline-start`, `resize: block / inline` — Missing: Logical keywords (follow their properties). *`float` / `clear` part shipped: C8-FLOAT; `text-align` part: C9-TEXT-ALIGN.*

**3.23 Transforms, filters, masking, compositing**

- `backdrop-filter` — Missing: Same functions on the backdrop cells.
- `clip-path` — Missing: `inset()` rectangles only; shapes N/A.

**3.24 Other modules (CSS 2.1 leftovers, Multi-column, Images, Speech, Fragmentation)**

- `columns` / `column-count` / `column-width` / `column-rule*` / `column-span` / `column-fill` — Missing: Multi-column layout.
- `anchor-name` / `position-anchor` / `position-area` / `@position-try` (Anchor Positioning 1, WD) — Missing: Tooltips / popovers anchored to another box; useful, outside the requested module list.

Also undocumented: the rdom extensions marked `No` in §4.

---

## 6. `DIVERGENCES.md` statements the code contradicts

Found while grounding the rows above; each should be corrected when the related row is decided.

*All six corrected in `DIVERGENCES.md` by CSS-COMPLETE-2026-10 item C0 (2026-10-03); the
undocumented gaps of §5 and extensions of §4 are now listed there too (§3 and §2 Values).*

1. **`text-decoration: line-through` is said to be unimplemented** (§2 Layout, "`text-align`,
   `vertical-align`, `text-decoration: line-through` are not implemented"). It parses
   (`V/keyword.rs::parse_text_decoration`) and paints as SGR 9 (`CASC/apply.rs`,
   `TextDecoration::LineThrough => Modifier::CROSSED_OUT`).
2. **A `flex-grow` longhand is implied to exist** ("write `width: auto; flex-grow: 1` instead";
   "`flex-grow` / `flex-shrink` / `flex: <n>` accept integers only"). `flex-grow` is not in
   `PROPERTY_NAMES`; the declaration is `UnknownProperty`. *Resolved: C6-FLEX-LONGHANDS — the
   longhand exists.*
3. **`top` / `right` / `bottom` / `left` are said to take a bare percentage** (§Values, "Width /
   height / top / right / bottom / left / gap take both forms"). `parse_length` has no
   `Token::Percentage` arm; only `calc(…%)` works.
4. **`var()` is said to be consumed in `content`** (§Cascade & selectors). `parse_content` has
   no `var()` arm; only the Rust builder `Content::Var` reaches the resolver.
5. **`box-sizing`**: `BLOCK/width.rs` says the implicit `border-box` is "documented in
   `DIVERGENCES.md` under Values"; it is not. *Resolved: C5-BOX-SIZING — `box-sizing` is a
   property with the CSS initial value, and the comment is gone.*
6. **Units** (§1, "Length units"): `ch` and the viewport units are grouped with `px` / `em` as
   depending "on a pixel or font-size concept the terminal grid doesn't have". `1ch` is exactly
   one column on a monospaced grid and `vw` / `vh` are percentages of the terminal size rdom
   already tracks; the classification is a choice, not a medium constraint.
