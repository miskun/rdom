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
| `FLEX` | `rdom-tui/src/render/layout_pass/flex/` (`main_axis.rs`, `cross.rs`, `placement.rs`) |
| `BLOCK` | `rdom-tui/src/render/layout_pass/block/` |
| `IFC` | `rdom-tui/src/render/layout_pass/ifc.rs` + `rdom-tui/src/render/inline/` |
| `POS` | `rdom-tui/src/render/layout_pass/{positioning,sticky}.rs` + `render/stacking.rs` |
| `PAINT` | `rdom-tui/src/render/paint_pass/` (`border.rs`, `border_join.rs`, `text.rs`, `scrollbar.rs`, `group.rs`) |
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
| 3.3 Values and units (Values 4) | 13 | 4 | 1 | 4 | 22 |
| 3.4 Color (Color 4 / 5) | 3 | 3 | 9 | 2 | 17 |
| 3.5 Backgrounds and borders (Backgrounds 3, Borders 4) | 3 | 5 | 6 | 2 | 16 |
| 3.6 Box model and sizing (Box 3, Sizing 3/4) | 2 | 4 | 3 | 0 | 9 |
| 3.7 Display and visibility (Display 3) | 1 | 0 | 8 | 2 | 11 |
| 3.8 Flexbox and box alignment (Flexbox 1, Align 3) | 3 | 3 | 11 | 0 | 17 |
| 3.9 Grid (Grid 1/2) | 0 | 0 | 10 | 0 | 10 |
| 3.10 Positioned layout (Position 3, CSS 2.1 §9) | 3 | 1 | 2 | 1 | 7 |
| 3.11 Overflow, scrolling and scrollbars (Overflow 3/4, Scroll Snap 1, Overscroll 1, Scrollbars 1) | 1 | 3 | 10 | 0 | 14 |
| 3.12 Inline text (Text 3/4, Inline 3, CSS 2.1 §10.8) | 0 | 1 | 14 | 6 | 21 |
| 3.13 Text decoration (Text Decoration 3/4) | 0 | 1 | 3 | 2 | 6 |
| 3.14 Fonts (Fonts 4) | 0 | 2 | 1 | 2 | 5 |
| 3.15 Lists, counters and generated content (Lists 3, Generated Content 3, Counter Styles 3, Pseudo-Elements 4) | 1 | 3 | 6 | 2 | 12 |
| 3.16 Pseudo-elements (Pseudo-Elements 4, Selectors 4) | 4 | 1 | 5 | 6 | 16 |
| 3.17 Selectors (Selectors 4) | 16 | 2 | 15 | 5 | 38 |
| 3.18 Transitions and animations (Transitions 1/2, Animations 1/2, Easing 1/2) | 3 | 3 | 4 | 0 | 10 |
| 3.19 User interface (UI 4) | 2 | 1 | 8 | 1 | 12 |
| 3.20 Tables (Tables 3, CSS 2.1 §17) | 0 | 0 | 4 | 0 | 4 |
| 3.21 Conditional rules and containment (Conditional 3/5, Contain 2/3, Will Change 1) | 0 | 0 | 6 | 1 | 7 |
| 3.22 Logical properties and writing modes (Logical 1, Writing Modes 4) | 0 | 0 | 6 | 3 | 9 |
| 3.23 Transforms, filters, masking, compositing | 0 | 0 | 6 | 4 | 10 |
| 3.24 Other modules (CSS 2.1 leftovers, Multi-column, Images, Speech, Fragmentation) | 0 | 0 | 2 | 4 | 6 |
| **Total** | **79** | **38** | **140** | **50** | **307** |

Of the 191 Partial / Missing rows, **123 were not documented** in `DIVERGENCES.md` when audited (Doc'd `No` or `Wrong`; 5 rows `Wrong`, where the document stated the opposite of the code) — see §5 and §6. The Doc'd column is the audit's record: Phase 0 of CSS-COMPLETE-2026-10 has since listed every gap in `DIVERGENCES.md` §3.

Headline: rdom parses **70 property names** (`PROPERTY_NAMES`). The cascade, selectors, generated content, positioning, overflow and form-state pseudo-classes are strong. The gaps a web developer hits first are flex alignment (`justify-content`, `align-items`), `flex-wrap`, the `flex-grow` / `flex-basis` longhands, the `border: 1px solid red` shorthand form, `var()` outside colors, `line-height`, `text-align`, `visibility`, `box-sizing`, and grid.

---

## 2. Missing — applicable, by expected impact on a web developer

Ordered by how often a web developer reaches for the feature and how badly its absence breaks
pasted CSS. The first block is what a typical component stylesheet hits in its first ten lines.

| # | Item | Terminal semantics | Size | Doc'd |
|---|---|---|---|---|
| 1 | `justify-content` | Distribute free main-axis cells: `flex-start` / `flex-end` / `center` / `space-between` / `space-around` / `space-evenly` (`start` / `end` aliases); integer remainder goes to the first gaps. | M | No |
| 2 | `align-items` / `align-self` | Cross-axis placement: `stretch` (today's only behavior) / `flex-start` / `flex-end` / `center` / `baseline` (= first text row). `Align` enum already exists in `KW` but nothing sets or reads it. | M | No |
| 3 | `flex-wrap` / `flex-flow` / `align-content` | Break items onto multiple flex lines when the main size overflows; `align-content` distributes the lines. Needed for tag clouds, toolbars that wrap, card grids without grid. | L | No |
| 4 | `flex-grow` / `flex-basis` longhands | `flex-grow: <n>` alone (today only the `flex` shorthand grows); `flex-basis: auto / <cells> / <pct> / content` as the real hypothetical main size instead of the collapsed `0%`. | M | Wrong |
| 5 | `display: grid` + `grid-template-*`, `grid-auto-*`, `grid-row/column*`, `grid-area`, `grid` | A cell-quantized grid formatting context: track sizing in cells / `fr` / `%` / `auto` / `minmax()` / `repeat()`, line- and area-based placement. The most-used modern layout after flex. | L | Yes |
| 6 | `line-height` | Rows per line box in whole rows: `normal` / `1` = one row; `2` = text on the first row of each two-row line box (blank row below, or half-leading split rounded); `<cells>`. | M | No |
| 7 | `text-align` (+ `text-align-last`, `text-justify`) | Per-line horizontal placement of inline content in the line box: `start` / `end` / `left` / `right` / `center` / `justify` (spread whole spaces between words). | M | Yes |
| 8 | `visibility` | `hidden`: box keeps its space, paints nothing, is not hit-testable, children may override with `visible`; `collapse` = `hidden` (table rows: removed). | S | No |
| 9 | `box-sizing` | rdom is implicitly `border-box` everywhere (code comment in `BLOCK/width.rs`); `content-box` (the CSS initial value) would add padding + border cells to `width` / `height`. Pasted CSS that relies on the default is off by `2 × (padding + border)`. | M | No |
| 10 | `outline` (+ `-color`, `-style`, `-width`, `-offset`) | A border ring drawn one cell outside the border box, taking no layout space, painted over neighbors on the top layer; `outline-offset` in whole cells. The natural keyboard-focus ring a TUI otherwise lacks. | M | No |
| 11 | `overflow-wrap` / `word-break` | `overflow-wrap: anywhere / break-word` and `word-break: break-all` break an over-long word at a cell boundary instead of overflowing and clipping (today's `overflow-wrap: normal`); `word-break: keep-all` for CJK. | M | No |
| 12 | `text-overflow` | `ellipsis`: the last visible cell of a clipped line becomes `…`; `<string>` form uses that string; applies with `overflow: hidden` + `white-space: nowrap`. | S | No |
| 13 | Per-side border colors (`border-*-color`, multi-value `border-color`) | Each side's glyphs in its own color; the joiner's existing conflict winner picks the junction color. | M | No |
| 14 | `currentColor` | Resolves to the element's computed `color`; should also be the initial value of `border-color`, `outline-color`, `caret-color`, `text-decoration-color`. | S | Yes |
| 15 | `min()` / `max()` / `clamp()` | Comparison functions inside every `calc()` position; resolve at layout like percent-bearing `calc()`. | S | Yes |
| 16 | `hsl()` / `hwb()` / `lab()` / `lch()` / `oklab()` / `oklch()` / `color()` | Convert to sRGB at parse time (gamut-mapped), emit as truecolor `Color::Rgb`. Common in design-token CSS. | S | No |
| 17 | `:nth-child()` / `:nth-last-child()` / `:nth-of-type()` / `:nth-last-of-type()` / `:first-of-type` / `:last-of-type` / `:only-of-type` | Structural matching (`An+B`, `odd` / `even`, `of S`); zebra-striped lists and tables. | S | Partial — `:nth-child`, `:nth-of-type` Yes; the `*-of-type` trio No |
| 18 | `:is()` / `:has()` | `:is()` shipped (C1G-IS-PARSE); `:has()` relational matching with invalidation on descendant change (M). | S / M | Yes |
| 19 | `@media` | Evaluate `width` / `height` (in cells) / `orientation` / `aspect-ratio`, `color` / `monochrome`, `prefers-color-scheme` (from the terminal's reported background), `prefers-reduced-motion`, `hover` / `pointer`; re-cascade on `resize`. Also the `<style media>` attribute. | M | Yes |
| 20 | `order` | Reorders flex (and grid) items for layout and paint, not for DOM / focus order. | S | No |
| 21 | `@keyframes` + `animation-*` | Keyframed animation on the existing transition clock and interpolators. | L | Yes |
| 22 | `display: contents` / `display: flow-root` / multi-keyword `display` | `contents`: the element generates no box, children are laid out in the parent; `flow-root`: a block that establishes an independent formatting context (stops margin collapse); `block flex` / `inline flow-root` two-value syntax. | M / S / S | No |
| 23 | `text-transform` | `uppercase` / `lowercase` / `capitalize` at shaping time (copy keeps the DOM text, as browsers do); `full-width` maps ASCII to U+FF01–FF5E (2 cells each). | S | No |
| 24 | `text-indent` | First line of each block starts `n` cells in (negative = hanging); `hanging` / `each-line` keywords. | S | No |
| 25 | `ch` / `lh` / `rlh` and viewport units `vw` / `vh` / `vmin` / `vmax` (+ `s`/`l`/`d` variants) | `1ch` = one column exactly on a monospaced grid; `1lh` = one row; `1vw` = 1% of the terminal's columns, `1vh` = 1% of its rows (rdom knows the viewport). | S | Yes (classified as pixel-dependent; see §6) |
| 26 | Logical properties (`inline-size`, `block-size`, `margin-inline*`, `padding-block*`, `inset-inline*`, `border-inline*`, …) | 1:1 aliases of the physical properties (rdom is horizontal-tb, LTR only). | S | No |
| 27 | `border-radius` (+ per-corner) | Any non-zero radius → rounded corner glyphs `╭╮╰╯` (`CornerStyle::Rounded`, reachable today only via the rdom keyword `border: rounded`); `0` → square. | S | No |
| 28 | `border-width` (+ per-side) and width component of `border` | `0` = no border on that side; `thin` / `medium` / `1` = light glyphs; `thick` / `≥2` = heavy box-drawing glyphs (`━┃┏┓┗┛`); never more than one cell. | S | No |
| 29 | `list-style-type` / `list-style-position` / `list-style` / `::marker` / `display: list-item` | Marker from a counter style or `<string>`; `outside` hangs it in the padding, `inside` puts it on the first line (today's behavior via `li::before`). | M | Yes |
| 30 | `::first-line` / `::first-letter` | Style the first line box / first typographic letter (color, bold, italic, decoration, background). | M | Yes |
| 31 | `var()` outside colors | Covered under Partial (§3.2): substitution in every property. Listed here because of its impact — design-token CSS uses `padding: var(--space-2)` everywhere. | M | Yes |
| 32 | CSS Nesting (`&`, nested rules, nested `@media`) | Nested style rules desugared to `:is(parent) child` at parse time. | M | Yes |
| 33 | `all` / `revert` / `revert-layer` | `all: unset` (or `revert`) resets every property (common "CSS reset" idiom); `revert` rolls back to the UA origin. | S | No |
| 34 | `@layer` | Cascade layers ordering author rules; anonymous / named / nested layers, `@layer a, b;` statements. | M | Blanket |
| 35 | `@supports` | Evaluate `(prop: value)` against the dispatch table, `selector()`, `not` / `and` / `or`. Lets pasted CSS degrade intentionally. | S | Blanket |
| 36 | `white-space: pre-line` / `break-spaces` (+ Text 4 `white-space-collapse`, `text-wrap-mode`) | `pre-line` collapses spaces but keeps newlines; `break-spaces` keeps and wraps trailing spaces. | S | No |
| 37 | `flex-direction: row-reverse / column-reverse` | Reverse main-axis placement order. | S | No |
| 38 | `row-gap` / `column-gap` / two-value `gap` | Separate gaps per axis (today `gap` is one value for both). | S | No |
| 39 | Color syntax completeness: `rgb()` space syntax / `%` channels / `/ alpha`; `color-mix()`; relative color syntax; system colors (`Canvas`, `CanvasText`, …); `light-dark()` + `color-scheme` | System colors map onto the terminal's default fg / bg (`Color::Reset`), a natural fit; `light-dark()` picks by the terminal's reported background. | S–M | No |
| 40 | `:read-only` / `:read-write`, `:in-range` / `:out-of-range`, `:default`, `:user-valid` / `:user-invalid`, `:modal`, `:link` / `:any-link`, `:lang()`, `:scope`, `:popover-open` | Form / link / context state rdom already tracks (or can) for every one. | S each | Partial — `:read-*`, `:user-*`, `:modal` Yes; rest No |
| 41 | `float` / `clear` | Line-box exclusion beside a floated box; sidebars and drop-caps. Deliberately out of scope today. | L | Yes |
| 42 | `cursor` | OSC 22 pointer-shape request (`pointer`, `text`, `default`, `move`, resize shapes) on terminals that honor it (kitty, foot, ghostty, WezTerm); ignored elsewhere. | S | No |
| 43 | `accent-color` | Color of checkbox / radio / range / progress glyphs in the UA chrome. | S | No |
| 44 | `appearance` | `none` drops the UA control chrome (brackets, glyphs) so authors can restyle controls; `auto` restores it. | M | No |
| 45 | `caret-shape` / `caret-animation` / `caret` | `bar` / `block` / `underscore` for the painted caret (or DECSCUSR on the hardware cursor); `manual` disables blink. | S | No |
| 46 | `scrollbar-width` / `scrollbar-color` | `scrollbar-width: none` hides the bar while keeping the box scrollable (`thin` = `auto`, already one cell); `scrollbar-color: <thumb> <track>` = the standard spelling of `::scrollbar-thumb` / `::scrollbar` colors. | S | Yes |
| 47 | `overscroll-behavior` (+ `-x`, `-y`, logical) | `contain` / `none` stop wheel / keyboard scroll chaining into the ancestor at the scroll limit. | S | No |
| 48 | `scroll-padding*` / `scroll-margin*` / `scroll-snap-type` / `scroll-snap-align` / `scroll-snap-stop` | Insets for `scrollIntoView` / keyboard scrolling; snap scroll offsets to item edges (row-snapped lists). | S / M | Partial — padding / margin Yes; snap No |
| 49 | Table properties: `border-spacing`, `vertical-align` (cells), `table-layout`, `caption-side`, `empty-cells` | Cell gaps in cells; `middle` / `bottom` cell alignment; `fixed` = first-row widths only; caption above / below; hide empty cells' borders. | S each (`vertical-align` M) | Partial — `vertical-align` Yes; rest No |
| 50 | `vertical-align` (inline) | `top` / `middle` / `bottom` of an inline-block in a taller line box; `sub` / `super` are N/A (sub-cell). | M | Yes |
| 51 | `text-decoration-line` / `-color` / `-style`, multi-line `text-decoration`, `overline` | Underline color via SGR 58; style via SGR 4:1–4:5 (`solid` / `double` / `wavy` / `dotted` / `dashed` on kitty, WezTerm, foot, ghostty); `overline` via SGR 53; `underline line-through` together. | S | No |
| 52 | `font-weight` numeric / `bolder` / `lighter`, `font-style: oblique`, `font` shorthand | `≥ 600` / `bold` / `bolder` → SGR 1; `≤ 300` / `lighter` → SGR 2 (faint) is optional; `oblique` → italic; `font` shorthand reads weight / style and ignores family / size. | S | No |
| 53 | `quotes` + `open-quote` / `close-quote` in `content` | Quote marks for `<q>` and nested quotations, from the `quotes` pairs. | S | No |
| 54 | `counters()`, `counter-set`, `reversed()` in `counter-reset`, more counter styles, `@counter-style`, `symbols()` | Nested "1.2.3" numbering; set without reset; `<ol reversed>`; `disc` / `circle` / `square` / `decimal-leading-zero` / `lower-greek` / author-defined styles. | S / M | Partial — `counters()`, `counter-set` Yes; rest No |
| 55 | `line-clamp` (`max-lines`, `block-ellipsis`, `continue`) | Clamp a block to N rows, last row ends in `…`. | M | No |
| 56 | `filter` (color functions), `backdrop-filter`, `mix-blend-mode`, `isolation` | `grayscale()` / `invert()` / `brightness()` / `contrast()` / `sepia()` / `saturate()` / `hue-rotate()` / `opacity()` as per-cell color transforms (`blur()` / `drop-shadow()` N/A); blend modes per cell; `isolation: isolate` as a stacking-context trigger. | M (isolation S) | Yes (as non-existent stacking triggers) |
| 57 | `box-shadow` | Offset shadow one cell right / down in a darker background or `░▒▓` shade glyphs (Turbo Vision style); blur and spread N/A. | M | No |
| 58 | `background-clip` | `padding-box` / `content-box`: fill only that area (half-block borders hard-code `padding-box` today). | S | Yes |
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
| 72 | `margin-trim`, `inset` with `calc()` / `%`, `hyphens: manual` | Trim child margins at container edges; percent / `calc()` in the `inset` shorthand; break at U+00AD soft hyphens showing `-`. | S | No / No / Yes |

### High-impact *Partial* items (fix alongside the list above)

These are parsed, so they do not show up as "unknown property", but common real-world values are
dropped:

1. **`border` shorthand is a single keyword.** `border: 1px solid red`, `border: solid red`,
   `border-top: 1px solid #ccc` are all `InvalidValue` (`V/border.rs::parse_border` and
   `parse_border_side` call `parse_keyword`, which requires exactly one token). The most common
   border declaration on the web is dropped. **Doc'd: No.**
2. **`var()` works in color positions only** (`V/color.rs::parse_var_args`); every other property
   rejects it. **Doc'd: Yes**, but the claim that `content` accepts it is wrong (§6).
3. **`max-width` / `max-height` reject `none`** (their initial value) and percentages:
   `parse_unsigned` takes a bare integer or a constant `calc()` only. `max-width: 100%` and
   `max-width: none` are both dropped. **Doc'd: No.**
4. **`min-width` / `min-height` take `auto | <integer>` only** — no `%`, no `calc()`. **Doc'd: No.**
5. **`top` / `right` / `bottom` / `left` reject a bare `%`** (`V/length.rs::parse_length` has no
   `Token::Percentage` arm; `calc(50%)` works). **Doc'd: Wrong.**
6. **`width` / `height` lack `min-content` / `max-content` / `fit-content`**, although the
   intrinsic sizes are computed (`layout_pass/intrinsic.rs`). **Doc'd: No.**
7. **`font-weight` is `normal | bold` only** — `font-weight: 700` / `600` / `bolder` are dropped.
   **Doc'd: No.**
8. **`text-decoration` is one keyword**, matched case-sensitively; no `overline`, no
   combinations, no color / style components. **Doc'd: Wrong** (says `line-through` is missing;
   it ships).
9. **`overflow` lacks `clip` and the two-value form** (`overflow: hidden auto`). **Doc'd: No.**
10. **`rgb()` takes the legacy comma form with integer channels only** — `rgb(0 0 0 / 50%)`,
    `rgb(10%, 20%, 30%)` and `rgb(12.5, 0, 0)` are dropped. **Doc'd: No.**

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
| Inherited-property set | Supported | `inherits()` lists `color`, `font-weight`, `font-style`, `white-space`, `pointer-events`, `caret-color`, `caret-text-color`; `border-collapse` is non-inherited by design (documented). Correct for the shipped set. | — | `DISP/table.rs::inherits` |

### 3.2 Custom properties (CSS Variables 1)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `--*` declarations | Supported | Any selector, inline, `!important`, inherited, verbatim token value. | — | `DISP/set.rs` (`--` prefix) |
| `var()` in color positions | Supported | Through the general substitution (row below); `rgb(var(--r), 0, 0)` too (C1-VAR-ANY). | — | `rdom-style/src/var.rs` |
| `var()` in `content` | Supported | Substituted, then parsed by `parse_content` (C1-VAR-ANY). | — | `rdom-style/src/var.rs` |
| `var()` in all other properties | Supported | A declaration holding `var()` is kept as tokens (`TuiStyle::pending`, `var()` syntax checked at parse time); the cascade substitutes it per element from the element's custom properties and parses it with the property's grammar — shorthands included, later declarations of the block replayed in order; a failure makes the property `unset` (invalid at computed-value time). Custom properties substitute where declared; cycles make them guaranteed-invalid. Substitution is token-level (`var(--n)fr` is a number and an ident, not `1fr`; strings and idents keep their escapes) and capped at 65 536 tokens (`var::MAX_SUBSTITUTED_TOKENS`, §3.3; C1G-VAR-TOKENS). A style without `var()` costs nothing extra (C1-VAR-ANY). | — | `rdom-style/src/var.rs`, `DISP/set.rs`, `CASC/ladder.rs` |
| `var()` fallback with arbitrary tokens | Supported | Any token sequence, commas included, `var()` inside substituted (C1-VAR-ANY). | — | `rdom-style/src/var.rs` |
| `@property` | Supported | `@property` and `CSS.registerProperty` (`App::register_property`, `Stylesheet::register_property`): syntax validated at computed-value time (invalid → `unset`), initial value, `inherits`; registered `<color>` / `<number>` / `<integer>` / `<length>` / `<percentage>` transition and their `var()` consumers follow. Syntax components without a terminal value parser are rejected (DIVERGENCES) (C1-PROPERTY). | — | `rdom-style/src/registration.rs`, `rdom-css/src/property.rs`, `CASC/registered.rs`, `RT/animation/custom.rs` |
| `env()` | N/A | Safe-area / UA environment variables describe display hardware a terminal does not report. | — | — |

### 3.3 Values and units (Values 4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `<integer>` cells (unitless) | Supported | rdom's length unit; see §4. | Yes | `V/length.rs`, `V/spacing.rs` |
| `<percentage>` | Supported | Every length-bearing property (`width` / `height`, `min-*` / `max-*`, `padding`, `margin`, the insets, `gap`), bare and in math functions, each against its spec's basis; `opacity`. (`flex-basis` takes it when the property lands, C6-FLEX-LONGHANDS.) C2-PERCENT. | — | `V/numeric.rs::length_percentage` |
| `<number>` (fractional) | Supported | `opacity`, `cubic-bezier()`, times, flex factors (`flex: 0.5`, `flex-shrink: 1.5`, `1.5fr`; factors summing below one share that fraction of the free space, Flexbox §9.7) (C2-NUMBER). | — | `V/numeric.rs::number`, `V/length.rs` |
| `calc()` | Supported | `+ - * /`, parentheses, nested `calc()`, percent-bearing forms resolved at layout, divide-by-zero rejected. Whitespace relaxation documented. | Yes | `CALC` |
| `min()` / `max()` / `clamp()` | Supported | Inside and outside `calc()`, nested, mixed with percentages (resolved at layout), `clamp()` with `none` bounds (C2-MINMAX). | — | `CALC` |
| `round()` / `mod()` / `rem()` / `abs()` / `sign()` | Supported | All four rounding strategies, the argument-range rules (NaN / infinities), percentages resolved at layout (C2-STEPPED). | — | `CALC` |
| `sin()` … `atan2()`, `pow()` / `sqrt()` / `hypot()` / `log()` / `exp()` | Supported | With the constants `e`, `pi`, `infinity`, `-infinity`, `NaN` and Values 4 §10.9 type checking (an `<angle>` result is no length); math functions of type `<number>` also work in `opacity` and flex factors (C2-TRIG). | — | `CALC` |
| `px`, `cm`, `mm`, `Q`, `in`, `pt`, `pc` | N/A | No pixel / physical length on a cell grid (documented). | — | — |
| `em`, `rem`, `ex`, `cap`, `ic` | N/A | No font size or font metrics to scale against (documented). | — | — |
| `ch` | Supported | Exactly one column on a monospaced grid, in every length property and math function; fractions round where the value becomes a length (C2-CH). | — | `CALC/units.rs`, `V/numeric.rs` |
| `lh`, `rlh` | Partial | One row each, in every length property and math function (C2-LH); they follow `line-height` when it lands (C9-LINE-HEIGHT). | Yes | `CALC/units.rs` |
| `vw` / `vh` / `vmin` / `vmax` (+ `sv*` / `lv*` / `dv*`, `vi` / `vb`) | Supported | 1% of the terminal's columns / rows, absolute at computed-value time (the cascade resolves them; a resize cascades again) (C2-VIEWPORT). | — | `CALC/units.rs`, `rdom-style/src/absolute.rs`, `CASC` |
| `cqw` / `cqh` / `cqi` / `cqb` / `cqmin` / `cqmax` | Missing | Need `@container`. | No | `CALC` |
| `fr` | Partial | Accepted on `width` / `height` as an rdom flex weight (§4); grid's `fr` does not exist. | Yes | `V/length.rs::parse_size` |
| `<time>` (`s`, `ms`) | Supported | Rounded to whole ms (documented). | Yes | `TR::parse_time_ms` |
| `<angle>` (`deg`, `grad`, `rad`, `turn`) | Supported | In the trigonometric functions and registered `<angle>` custom properties (which interpolate); `parse::values::parse_angle` gives degrees for the color hues of Phase 3 (C2-ANGLE). No rotation exists. | — | `CALC/units.rs`, `V/numeric.rs::parse_angle` |
| `<resolution>`, `<frequency>` | N/A | Image resolution / aural values. | — | — |
| `<string>` | Supported | In `content`. | — | `V/content.rs` |
| `url()` / `<image>` / gradients / `image-set()` | N/A | No images (documented). | — | — |
| `<ratio>` | Partial | `aspect-ratio` only, `<int> / <int>` (documented). | Yes | `V/number.rs::parse_aspect_ratio` |
| `attr()` | Partial | In `content` only, no fallback, no type (`attr(x type(<length>))`, Values 5). | No | `V/content.rs` |
| `<custom-ident>` | Supported | Counter names, transition-property idents. | — | `V/content.rs`, `TR` |

### 3.4 Color (Color 4 / 5)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `color` | Supported | Every color form below that parses. | — | `DISP/set.rs` |
| Named colors (148) | Supported | Case-insensitive lookup. | — | `rdom-style/src/color/named.rs` |
| Hex `#rgb` / `#rgba` / `#rrggbb` / `#rrggbbaa` | Supported | Alpha validated then dropped (documented with `rgba()`). | Yes | `tui_color.rs::parse_hex` |
| `rgb()` / `rgba()` | Partial | Legacy comma syntax with integer `0–255` channels only; no space syntax, no `/ alpha`, no `%` channels, no fractional numbers, no `none`; alpha dropped (documented). | No | `V/color.rs::parse_rgb_args` |
| `transparent` | Partial | Maps to `Color::Reset`: as a background it does not fill (ancestor shows through — correct); as `color` / `border-color` it is the terminal's default foreground, not invisible. | No | `tui_color.rs::parse_simple_color` |
| `currentColor` | Missing | Computed `color`. | Yes | `COL`, `CASC` |
| `hsl()` / `hsla()` | Missing | Convert to sRGB. | No | `COL` |
| `hwb()` | Missing | Convert to sRGB. | No | `COL` |
| `lab()` / `lch()` / `oklab()` / `oklch()` | Missing | Convert + gamut-map to sRGB. | No | `COL` |
| `color()` | Missing | Predefined spaces (`srgb`, `display-p3`, …) converted to sRGB. | No | `COL` |
| `color-mix()` (Color 5) | Missing | Mix at parse time (or computed time with `var()` / `currentColor`). | No | `COL` |
| Relative color syntax (Color 5) | Missing | `rgb(from var(--x) r g b / 50%)`. | No | `COL` |
| System colors (`Canvas`, `CanvasText`, `LinkText`, `ButtonFace`, …) | Missing | `Canvas` / `CanvasText` = terminal default bg / fg (`Color::Reset`); others map to UA palette entries. | No | `COL` |
| `light-dark()` + `color-scheme` | Missing | Pick by the terminal's reported background (OSC 11 / mode 2031). | No | `COL`, `RT` |
| `opacity` | Partial | `<number>` / `<percentage>` clamped to 0–1 (`50%` since C2-PERCENT). Group opacity per cell (documented). | No | `V/number.rs::parse_opacity` |
| Alpha in colors | N/A | Terminal cells are opaque; translucency is `opacity` (documented). | — | — |
| `forced-color-adjust`, `print-color-adjust` | N/A | No forced-colors mode or print. | — | — |

### 3.5 Backgrounds and borders (Backgrounds 3, Borders 4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `background-color` | Supported | Any parsed color. | — | `DISP/set.rs` |
| `background` | Partial | A lone `<color>` only (documented). | Yes | `DISP/set.rs` |
| `background-image` / `-position` / `-size` / `-repeat` / `-attachment` / `-origin` | N/A | No images (documented). | — | — |
| `background-clip` | Missing | `padding-box` / `content-box` fill. | Yes | `PAINT/border.rs::fill_bg` |
| `border` | Partial | One keyword only: a style (`none` / `hidden` / `solid` / `double` / `dashed` / `dotted` / `ridge` / `groove` / `inset` / `outset`) plus rdom keywords; no width or color component — `border: 1px solid red` is dropped. | No | `V/border.rs::parse_border` |
| `border-top` / `-right` / `-bottom` / `-left` | Partial | One style keyword only; no width / color. | No | `V/border.rs::parse_border_side` |
| `border-style` | Partial | One value applied to all four sides; the 2–4-value form is rejected. | No | `DISP/set.rs` |
| `border-*-style` | Supported | Every CSS keyword parses; `dashed` / `dotted` / `ridge` / `groove` / `inset` / `outset` render as `solid` (documented). | Yes | `DISP/set.rs` |
| `border-color` | Partial | One color for all sides; 2–4 values rejected. | No | `DISP/set.rs`, `TS::border_fg` |
| `border-top-color` / `-right-color` / `-bottom-color` / `-left-color` | Missing | Per-side glyph color. | No | `DISP`, `BOX::Border`, `PAINT/border_join.rs` |
| `border-width` / `border-*-width` | Missing | `0` = none; `thin` / `medium` = light; `thick` = heavy glyphs. | No | `DISP`, `BOX`, `PAINT/border.rs` |
| `border-radius` / `border-*-radius` | Missing | Non-zero → rounded corner glyphs. | No | `DISP`, `BOX::CornerStyle` |
| `border-image*` | N/A | Image-sliced borders. | — | — |
| `box-shadow` | Missing | One-cell offset shade; blur / spread N/A. | No | `PAINT` |
| `border-collapse` | Supported | `separate` / `collapse`, with the documented scope / inheritance divergences. | Yes | `DISP/set.rs`, `rdom-tui/src/render/layout_pass/border_collapse.rs` |
| `border-spacing` | Missing | Gaps between separated table cells. | No | `DISP`, `rdom-tui/src/runtime/builtins/table` |

### 3.6 Box model and sizing (Box 3, Sizing 3/4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `margin` / `margin-*` | Supported | Signed cells, `auto`, `%`, `calc()` (C2-PERCENT); longhands share storage (documented). | — | `V/spacing.rs` |
| `padding` / `padding-*` | Supported | Cells, `%`, `calc()` (C2-PERCENT). | — | `V/spacing.rs` |
| `margin-trim` | Missing | Trim children's margins at the container edges. | No | `BLOCK`, `FLEX` |
| `width` / `height` | Partial | `auto`, cells, `%`, `calc()`, rdom `fr`; missing `min-content` / `max-content` / `fit-content` / `fit-content(<l>)` / `stretch`. | No | `V/length.rs::parse_size`, `BOX::Size` |
| `min-width` / `min-height` | Partial | `auto`, cells, `%`, `calc()` (C2-PERCENT); no intrinsic keywords (C5-INTRINSIC). | No | `V/length.rs::parse_min_size` |
| `max-width` / `max-height` | Partial | Cells, `%`, `calc()` (C2-PERCENT); `none` (the initial value, C5-MINMAX-SIZE) and intrinsic keywords rejected. | No | `V/length.rs::parse_max_size` |
| `box-sizing` | Missing | rdom is implicitly `border-box`; `content-box` (CSS initial) is not expressible. | No | `BLOCK/width.rs`, `FLEX` |
| `aspect-ratio` | Partial | `<int> / <int>` only (documented). | Yes | `V/number.rs` |
| `contain-intrinsic-size` (+ longhands) | Missing | Placeholder size for `content-visibility: auto`. | No | `DISP`, layout |

### 3.7 Display and visibility (Display 3)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `display: block` / `inline` / `inline-block` / `flex` / `inline-flex` / `none` | Supported | Outer + inner (`Flow`) pair. | — | `DISP/set.rs`, `KW::Display`, `KW::Flow` |
| `display: contents` | Missing | No box; children join the parent. | No | `KW::Display`, layout tree builder |
| `display: flow-root` | Missing | Block that establishes an independent BFC. | No | `KW::Flow`, `BLOCK/margin_collapse.rs` |
| `display: list-item` | Missing | Marker box. | Yes | `KW`, `CASC/counters.rs` |
| `display: grid` / `inline-grid` | Missing | Grid formatting context. | Yes | new `layout_pass/grid` |
| `display: table` family | Missing | Real TFC (tables are tag-driven flex rows today). | Yes | `rdom-tui/src/runtime/builtins/table` |
| Multi-keyword `display` (`block flex`, `inline flow-root`) | Missing | Two-value syntax. | No | `DISP/set.rs` |
| `display: run-in` | N/A | Unimplemented by browsers; no TUI use. | — | — |
| `display: ruby*` | N/A | Ruby annotations need half-height text above a base. | — | — |
| `visibility` | Missing | `visible` / `hidden` / `collapse`. | No | `DISP`, `PAINT`, hit test |
| `order` | Missing | Visual reorder of flex / grid items. | No | `FLEX` |

### 3.8 Flexbox and box alignment (Flexbox 1, Align 3)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `flex-direction` | Partial | `row` / `column`; `row-reverse` / `column-reverse` rejected. | No | `DISP/set.rs`, `KW::Direction` |
| `flex-wrap` | Missing | Single-line only. | No | `FLEX` |
| `flex-flow` | Missing | Shorthand of the two above. | No | `DISP` |
| `flex` | Partial | Grow from the first number; shrink partly; basis collapsed to `0%` (documented); writes `width` + `height`. | Yes | `V/length.rs::parse_flex_shorthand` |
| `flex-grow` | Missing | Not in the property table (DIVERGENCES suggests `flex-grow: 1` as a workaround). | Wrong | `DISP`, `FLEX/main_axis.rs` |
| `flex-shrink` | Supported | `<number [0,∞]>`, fractions included (C2-NUMBER). | — | `DISP/set.rs`, `FLEX/main_axis.rs` |
| `flex-basis` | Missing | Basis ignored (documented as part of `flex`); the longhand does not exist. | No | `DISP`, `FLEX/main_axis.rs` |
| `justify-content` | Missing | Main-axis distribution. | No | `FLEX/placement.rs` |
| `align-items` | Missing | Cross-axis placement (always `stretch` unless a cross margin is `auto`). `KW::Align` exists, unused. | No | `FLEX/cross.rs` |
| `align-self` | Missing | Per-item override. | No | `FLEX/cross.rs` |
| `align-content` | Missing | Needs `flex-wrap`. | No | `FLEX` |
| `justify-items` / `justify-self` | Missing | Grid / block-level alignment. | No | grid, `BLOCK` |
| `place-content` / `place-items` / `place-self` | Missing | Shorthands. | No | `DISP` |
| `gap` | Partial | One value for both axes (cells, `%`, `calc()`); two-value form rejected. | No | `V/spacing.rs::parse_gap` |
| `row-gap` / `column-gap` | Missing | Per-axis gap. | No | `DISP`, `TS::gap` |
| Auto margins in flex | Supported | Main and cross axis. | — | `FLEX/main_axis.rs`, `FLEX/cross.rs` |
| Min-content protection (`min-width: auto`) | Supported | Flexbox §4.5. | — | `rdom-tui/src/render/layout_pass/intrinsic.rs` |

### 3.9 Grid (Grid 1/2)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `grid-template-columns` / `grid-template-rows` | Missing | Track lists in cells / `fr` / `%` / `auto` / `minmax()` / `repeat()` / `min-content` / `max-content`. | Yes | new `layout_pass/grid` |
| `grid-template-areas` | Missing | Named areas. | Yes | grid |
| `grid-template` | Missing | Shorthand. | Yes | grid |
| `grid-auto-columns` / `grid-auto-rows` | Missing | Implicit tracks. | Yes | grid |
| `grid-auto-flow` | Missing | `row` / `column` / `dense`. | Yes | grid |
| `grid` | Missing | Shorthand. | Yes | grid |
| `grid-row` / `grid-column` (+ `-start` / `-end`) | Missing | Line-based placement. | Yes | grid |
| `grid-area` | Missing | Area placement. | Yes | grid |
| `subgrid` (Grid 2) | Missing | Nested tracks. | Yes | grid |
| `masonry` / `grid-lanes` (Grid 3, WD) | Missing | Low priority. | Yes | grid |

### 3.10 Positioned layout (Position 3, CSS 2.1 §9)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `position` | Supported | `static` / `relative` / `absolute` / `fixed` / `sticky` (sticky containing block simplified, documented). | Yes | `V/keyword.rs::parse_position`, `POS` |
| `top` / `right` / `bottom` / `left` | Supported | `auto`, signed cells, `%`, `calc()` (C2-PERCENT / C8-INSETS). | — | `V/length.rs::parse_length` |
| `inset` | Supported | 1–4 values of `auto` / signed cells / `%` / `calc()` (C2-PERCENT / C8-INSETS). | — | `V/length.rs::parse_inset_shorthand` |
| `inset-block` / `inset-inline` (+ `-start` / `-end`) | Missing | Logical aliases. | No | `DISP` |
| `z-index` | Partial | `auto` / `i16` (documented). | Yes | `V/number.rs::parse_z_index` |
| `float` / `clear` | Missing | Out of scope by decision. | Yes | `BLOCK`, `IFC` |
| `clip` (CSS 2.1, deprecated) | N/A | Superseded by `clip-path`; no new content uses it. | — | — |

### 3.11 Overflow, scrolling and scrollbars (Overflow 3/4, Scroll Snap 1, Overscroll 1, Scrollbars 1)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `overflow` | Partial | `visible` / `hidden` / `scroll` / `auto`; `clip` and the two-value form rejected. | No | `V/keyword.rs::parse_overflow` |
| `overflow-x` / `overflow-y` | Partial | Same keyword set; no `clip`. | No | `V/keyword.rs::parse_overflow` |
| `overflow-block` / `overflow-inline` | Missing | Logical aliases. | No | `DISP` |
| `overflow-clip-margin` | Missing | Cells outside the box that `clip` still paints. | No | `PAINT` |
| `text-overflow` | Missing | `clip` (today) / `ellipsis` / `<string>`. | No | `IFC`, `PAINT/text.rs` |
| `line-clamp` / `max-lines` / `block-ellipsis` / `continue` | Missing | Row clamping with `…`. | No | `BLOCK`, `IFC` |
| `scroll-behavior` | Supported | `auto` / `smooth` (fixed curve, documented). | Yes | `V/keyword.rs` |
| `scrollbar-gutter` | Partial | `auto` / `stable`; `both-edges` rejected. | No | `V/keyword.rs::parse_scrollbar_gutter` |
| `scrollbar-width` | Missing | `none` hides the bar. | Yes | `DISP`, `PAINT/scrollbar.rs` |
| `scrollbar-color` | Missing | Thumb / track colors. | Yes | `DISP`, `PAINT/scrollbar.rs` |
| `overscroll-behavior` (+ `-x` / `-y` / `-block` / `-inline`) | Missing | Stop scroll chaining. | No | `RT` (wheel / key scroll routing) |
| `scroll-padding*` / `scroll-margin*` | Missing | Scroll-into-view insets. | Yes | `RT`, `TS` |
| `scroll-snap-type` / `scroll-snap-align` / `scroll-snap-stop` | Missing | Snap positions. | No | `RT` |
| `scroll-timeline*` / `view-timeline*` / `animation-timeline` / `animation-range*` | Missing | Scroll-driven animations; low priority, needs `@keyframes`. | Yes | `TR` |

### 3.12 Inline text (Text 3/4, Inline 3, CSS 2.1 §10.8)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `white-space` | Partial | `normal` / `pre` / `pre-wrap` / `nowrap`; `pre-line` and `break-spaces` rejected. | No | `DISP/set.rs`, `KW::WhiteSpace` |
| `white-space-collapse` / `text-wrap-mode` (Text 4) | Missing | Longhands of `white-space`. | No | `DISP`, `IFC` |
| `text-wrap` / `text-wrap-style` (Text 4) | Missing | `balance` / `pretty` / `stable` line breaking. | No | `IFC` |
| `text-align` | Missing | Line-box alignment. | Yes | `IFC` |
| `text-align-last` | Missing | Last-line alignment. | No | `IFC` |
| `text-justify` | Missing | Justification method (`inter-word` is the only sensible one). | No | `IFC` |
| `text-indent` | Missing | First-line indent in cells. | No | `IFC` |
| `text-transform` | Missing | Case mapping, `full-width`. | No | `IFC` |
| `tab-size` | Missing | Tab stops (tabs in `pre` render as one space, documented). | Yes | `IFC` |
| `word-break` | Missing | `break-all` / `keep-all`. | No | `IFC`, `rdom-tui/src/render/inline/mod.rs` |
| `overflow-wrap` / `word-wrap` | Missing | `anywhere` / `break-word`. | No | `IFC`, `layout_pass/intrinsic.rs` |
| `line-break` | Missing | CJK break strictness; low priority. | No | `IFC` |
| `hyphens` | Missing | `manual`: break at soft hyphens and show `-` (soft hyphens documented as unsupported). | Yes | `IFC` |
| `letter-spacing` / `word-spacing` | N/A | Sub-cell by nature; whole-cell spacing is conceivable but documented as out of scope. | — | — |
| `hanging-punctuation` | N/A | Hanging a glyph into the margin is a typographic nicety without a TUI use. | — | — |
| `line-height` | Missing | Whole-row line boxes. | No | `IFC`, `BLOCK` |
| `vertical-align` | Missing | Inline-block `top` / `middle` / `bottom`; table-cell alignment (documented); `sub` / `super` / lengths N/A. | Yes | `IFC`, table builtin |
| `dominant-baseline` / `alignment-baseline` / `baseline-shift` / `baseline-source` | N/A | One text baseline per row. | — | — |
| `initial-letter` | N/A | Multi-row drop caps need scaled glyphs. | — | — |
| `text-emphasis*` | N/A | Emphasis marks sit above / below a glyph, inside the same cell. | — | — |
| `text-shadow` | N/A | Sub-cell glyph shadow. | — | — |

### 3.13 Text decoration (Text Decoration 3/4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `text-decoration` | Partial | One keyword: `none` / `underline` / `line-through` (SGR 4 / 9); `overline`, combinations, color and style components rejected; case-sensitive. | Wrong | `V/keyword.rs::parse_text_decoration`, `CASC/apply.rs` |
| `text-decoration-line` | Missing | Longhand; `overline` via SGR 53. | No | `DISP`, `SGR` |
| `text-decoration-color` | Missing | SGR 58 underline color. | No | `DISP`, `SGR` |
| `text-decoration-style` | Missing | SGR 4:1–4:5 (`solid` / `double` / `curly` = `wavy` / `dotted` / `dashed`). | No | `DISP`, `SGR` |
| `text-decoration-thickness` / `text-underline-offset` / `text-underline-position` | N/A | The terminal draws decorations; position and thickness are not addressable. | — | — |
| `text-decoration-skip-ink` / `-skip` | N/A | Font-outline dependent. | — | — |

### 3.14 Fonts (Fonts 4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `font-weight` | Partial | `normal` / `bold`; numeric weights, `bolder` / `lighter` rejected. | No | `DISP/set.rs` |
| `font-style` | Partial | `normal` / `italic`; `oblique [<angle>]` rejected. | No | `DISP/set.rs` |
| `font` | Missing | Shorthand: honor weight / style, ignore size / family. | No | `DISP` |
| `font-family` / `font-size` / `font-stretch` (`font-width`) / `font-size-adjust` / `font-optical-sizing` / `font-kerning` / `font-feature-settings` / `font-variation-settings` / `font-language-override` / `font-synthesis*` / `font-palette` / `font-variant*` | N/A | The terminal owns the font; one monospaced face at one size (documented). | — | — |
| `@font-face` / `@font-feature-values` / `@font-palette-values` | N/A | Same. | — | — |

### 3.15 Lists, counters and generated content (Lists 3, Generated Content 3, Counter Styles 3, Pseudo-Elements 4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `content` | Partial | `none` / `normal` / `<string>` / `attr(<ident>)` / `counter(<ident>[, <style>])`, concatenated; missing `counters()`, `open-quote` / `close-quote` / `no-*-quote`, `var()`, alt text (`/ "alt"`). | Wrong | `V/content.rs::parse_content` |
| `quotes` | Missing | Quote pairs. | No | `DISP`, `CASC/content.rs` |
| `counter-reset` | Partial | `none` / `<ident> <integer>?` list; `reversed(<ident>)` rejected. | No | `V/content.rs::parse_counter_ops` |
| `counter-increment` | Supported | `none` / `<ident> <integer>?` list. | — | `V/content.rs::parse_counter_ops` |
| `counter-set` | Missing | Set without a new scope. | Yes | `DISP`, `CASC/counters.rs` |
| `counter()` styles | Partial | `decimal`, `lower-alpha` / `lower-latin`, `upper-alpha` / `upper-latin`, `lower-roman`, `upper-roman`; missing `disc` / `circle` / `square` / `disclosure-*`, `decimal-leading-zero`, `lower-greek`, `none`, the other predefined styles. | No | `rdom-style/src/counters.rs::CounterStyle` |
| `counters()` | Missing | Nested string form. | Yes | `V/content.rs` |
| `@counter-style` / `symbols()` | Missing | Author counter styles. | Blanket | `AT`, `rdom-style/src/counters.rs` |
| `list-style-type` / `list-style-position` / `list-style` | Missing | Marker type / position. | Yes | `DISP`, `CASC` |
| `list-style-image` | N/A | Images. | — | — |
| `marker-side` | Missing | Low priority. | No | — |
| `string-set` / `bookmark-*` / `running()` / `content()` / `leader()` / `target-counter()` | N/A | Paged-media features. | — | — |

### 3.16 Pseudo-elements (Pseudo-Elements 4, Selectors 4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `::before` / `::after` | Supported | Inline, block-first, positioned (paint / hit-test notes documented). | Yes | `PE` |
| `::selection` | Supported | Highlight style. | — | `PE` |
| `::placeholder` | Supported | Layered on the host's `::before` box (documented). | Yes | `PE` |
| `::backdrop` | Supported | Modal dialogs (no top layer, documented). | Yes | `PE` |
| `::marker` | Missing | Marker box styling. | Yes | `PE`, `CASC` |
| `::first-line` / `::first-letter` | Missing | First line / letter styling. | Yes | `PE`, `IFC` |
| Legacy single-colon `:before` / `:after` / `:first-line` / `:first-letter` | Missing | CSS 2.1 spellings, still valid; `:before` is an "unsupported pseudo-class" error. | No | `SEL`, `PE` |
| `::highlight()` | Missing | Custom Highlight API ranges (search hits, diagnostics). | No | `PE`, `rdom-core` (highlight registry) |
| `::details-content` | Missing | The collapsible part of `<details>`. | No | `PE` |
| `::target-text` | N/A | No URL fragment navigation. | — | — |
| `::spelling-error` / `::grammar-error` | N/A | No spellchecker. | — | — |
| `::file-selector-button` | N/A | `<input type=file>` is not rendered (documented). | — | — |
| `::cue` / `::cue-region` | N/A | No media / captions. | — | — |
| `::part()` / `::slotted()` | N/A | No Shadow DOM (documented). | — | — |
| `::view-transition*` | N/A | View transitions snapshot pixels. | — | — |
| More than one pseudo-element / pseudo-element followed by a pseudo-class (`::before:hover`) | Partial | One trailing suffix per selector; only the rdom `::scrollbar-thumb:vertical/horizontal` forms take a pseudo-class. | No | `PE::extract_pseudo_suffix` |

### 3.17 Selectors (Selectors 4)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `*`, type, `.class`, `#id` | Supported | Type names case-sensitive (documented). | Yes | `SEL` |
| `[attr]`, `=`, `~=`, `\|=`, `^=`, `$=`, `*=` | Supported | HTML case-insensitive attribute list honored. | — | `SEL::AttrOp` |
| Attribute case flags `i` / `s` | Missing | `[x=v i]`. | Yes | `SEL` |
| Namespace prefixes (`ns\|E`, `*\|E`) | N/A | No namespaces (documented). | — | — |
| Descendant, `>`, `+`, `~` | Supported | — | — | `SEL::Combinator` |
| Column combinator `\|\|` | Missing | Cells of a `<col>`; low priority. | No | `SEL` |
| Selector list `a, b` | Supported | — | — | `SEL` |
| `:not(<complex-list>)` | Supported | Full selector list. | — | `SEL` |
| `:where()` | Supported | Zero specificity. | — | `SEL` |
| `:is()` | Supported | `SimpleSelector::Is` (shared with the nesting `&`); forgiving argument list (an invalid argument is dropped, an empty `:is()` matches nothing); specificity of the most specific argument (Selectors 4 §4.2, §17; C1G-IS-PARSE, landing C11-IS). | — | `SEL` |
| `:has()` | Missing | — | Yes | `SEL`, invalidation in `CASC` |
| `:first-child` / `:last-child` / `:only-child` | Supported | — | — | `SEL` |
| `:nth-child()` / `:nth-last-child()` (+ `of S`) | Missing | — | Yes | `SEL` |
| `:nth-of-type()` / `:nth-last-of-type()` | Missing | — | Yes | `SEL` |
| `:first-of-type` / `:last-of-type` / `:only-of-type` | Missing | — | No | `SEL` |
| `:empty` | Supported | — | — | `SEL` |
| `:root` | Supported | — | — | `SEL` |
| `:scope` | Partial | Matches the `@scope` root (`Dom::matches_list_in_scope`), `:root` outside one; the query APIs do not set it to their root yet (C11-SCOPE). | Yes | `SEL`, `rdom-core/src/query_selector.rs` |
| `:hover` / `:active` / `:focus` / `:focus-within` / `:focus-visible` | Supported | Primary-button `:active` (documented). | Yes | `SEL` |
| `:checked` | Supported | Attribute-reflected (documented). | Yes | `SEL` |
| `:indeterminate` | Partial | `<progress>` without `value` only; indeterminate checkboxes / radio groups never match. | No | `SEL` |
| `:placeholder-shown` | Supported | — | — | `SEL` |
| `:open` | Supported | `<details>` / `<dialog>`. | — | `SEL` |
| `:disabled` / `:enabled` | Supported | HTML "actually disabled". | — | `SEL` |
| `:valid` / `:invalid` / `:required` / `:optional` | Supported | Forms and fieldsets too. | — | `SEL` |
| `:user-valid` / `:user-invalid` | Missing | — | Yes | `SEL`, `RT` |
| `:read-only` / `:read-write` | Missing | — | Yes | `SEL` |
| `:in-range` / `:out-of-range` | Missing | Number / range inputs exist. | No | `SEL` |
| `:default` | Missing | Default submit button, default-checked controls / options. | No | `SEL` |
| `:modal` | Missing | — | Yes | `SEL` |
| `:popover-open` | Missing | Needs the `popover` attribute behavior. | No | `SEL`, `RT` |
| `:link` / `:any-link` | Missing | `<a href>` exists. | No | `SEL` |
| `:visited` / `:local-link` / `:target` / `:target-within` | N/A | No navigation history or URL fragments. | — | — |
| `:lang()` | Missing | `lang` attribute inheritance. | No | `SEL` |
| `:dir()` | N/A | No bidi (documented). | — | — |
| `:defined` / `:state()` / `:host*` | N/A | No custom elements / Shadow DOM (documented). | — | — |
| `:autofill`, `:fullscreen`, `:picture-in-picture`, `:playing` / `:paused` / `:seeking` / `:buffering` / `:stalled` / `:muted` / `:volume-locked`, `:current` / `:past` / `:future` | N/A | No autofill, fullscreen, media or timed text. | — | — |
| `:blank` | Missing | Low priority (spec unstable). | No | `SEL` |

### 3.18 Transitions and animations (Transitions 1/2, Animations 1/2, Easing 1/2)

| Item | Class | Detail | Doc'd | Where |
|---|---|---|---|---|
| `transition-property` | Supported | `all` / `none` / names; unknown idents inert. | — | `TR` |
| `transition-duration` | Supported | `<time>` list. | — | `TR::parse_time_ms` |
| `transition-timing-function` | Partial | `linear` / `ease*` / `step-start` / `step-end` / `cubic-bezier()` / `steps()` with all positions; `linear(<stops>)` (Easing 2) rejected. | No | `TR` |
| `transition-delay` | Partial | Negative delays rejected (Transitions 1 allows them: start part-way). | No | `TR::parse_time_ms` |
| `transition` | Supported | Shorthand list. | — | `TR` |
| `transition-behavior` | Missing | `allow-discrete`. | Yes | `TR` |
| Animatable property set | Partial | `color`, `background-color`, `border-color`, `width`, `height`, `padding`, `gap`, `top` / `right` / `bottom` / `left`, `z-index`; not `opacity`, `margin`, `min-*` / `max-*`, `inset` as a name. | No | `TR::parse_animatable_property` |
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
| `border-spacing` | Missing | See §3.5. | No | — |
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
| `inline-size` / `block-size` / `min-*-size` / `max-*-size` | Missing | Aliases of `width` / `height` / `min-*` / `max-*`. | No | `DISP` |
| `margin-inline` / `margin-block` (+ `-start` / `-end`) | Missing | Aliases of physical margins. | No | `DISP` |
| `padding-inline` / `padding-block` (+ `-start` / `-end`) | Missing | Aliases of physical padding. | No | `DISP` |
| `border-inline` / `border-block` (+ `-start` / `-end`, `-color` / `-style` / `-width`) | Missing | Aliases of physical borders. | No | `DISP` |
| `border-start-start-radius` / … (4 corners) | Missing | Aliases of `border-*-radius`. | No | `DISP` |
| `inset-inline` / `inset-block` | Missing | See §3.10. | No | `DISP` |
| `text-align: start / end`, `float: inline-start`, `resize: block / inline` | Missing | Logical keywords (follow their properties). | No | — |
| `writing-mode` | N/A | Vertical flow could be emulated, but glyphs cannot be rotated in a cell. | — | — |
| `direction` / `unicode-bidi` | N/A | No bidi (documented). | — | — |
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
| `mask*` / `mask-border*` / `shape-outside` / `shape-margin` / `shape-image-threshold` | N/A | Image / shape geometry; no floats to wrap around. | — | — |

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
| `border: rounded` (rounded corners) and `border: single` (= `solid`) | `V/border.rs::parse_border` | No |
| `border: top` / `bottom` / `left` / `right` (one solid side) | `V/border.rs::parse_border` | No |
| Bare integer color `0–255` → xterm-256 palette index (`color: 208`) | `tui_color.rs::parse_simple_color` | No |
| `reset` color keyword (terminal default fg / bg) | `tui_color.rs::parse_simple_color` | No |
| `::scrollbar`, `::scrollbar-thumb`, `::scrollbar-thumb:vertical` / `:horizontal` | `PE` | Yes |

---

## 5. Undocumented gaps

Every *Partial* or *Missing* row above whose Doc'd column is `No` or `Wrong`. These need either an
implementation or a `DIVERGENCES.md` entry before the acid page's coverage test can be honest.

133 rows.

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
- `lh`, `rlh` — Missing: One row (× `line-height` once that exists).
- `cqw` / `cqh` / `cqi` / `cqb` / `cqmin` / `cqmax` — Missing: Need `@container`.
- `<angle>` (`deg`, `grad`, `rad`, `turn`) — Missing: Only needed for color hues (`hsl()`, `oklch()`); no rotation exists.
- `attr()` — Partial: In `content` only, no fallback, no type (`attr(x type(<length>))`, Values 5).

**3.4 Color (Color 4 / 5)**

- `rgb()` / `rgba()` — Partial: Legacy comma syntax with integer `0–255` channels only; no space syntax, no `/ alpha`, no `%` channels, no fractional numbers, no `none`; alpha dropped (documented).
- `transparent` — Partial: Maps to `Color::Reset`: as a background it does not fill (ancestor shows through — correct); as `color` / `border-color` it is the terminal's default foreground, not invisible.
- `hsl()` / `hsla()` — Missing: Convert to sRGB.
- `hwb()` — Missing: Convert to sRGB.
- `lab()` / `lch()` / `oklab()` / `oklch()` — Missing: Convert + gamut-map to sRGB.
- `color()` — Missing: Predefined spaces (`srgb`, `display-p3`, …) converted to sRGB.
- `color-mix()` (Color 5) — Missing: Mix at parse time (or computed time with `var()` / `currentColor`).
- Relative color syntax (Color 5) — Missing: `rgb(from var(--x) r g b / 50%)`.
- System colors (`Canvas`, `CanvasText`, `LinkText`, `ButtonFace`, …) — Missing: `Canvas` / `CanvasText` = terminal default bg / fg (`Color::Reset`); others map to UA palette entries.
- `light-dark()` + `color-scheme` — Missing: Pick by the terminal's reported background (OSC 11 / mode 2031).
- `opacity` — Partial: `<number>` clamped to 0–1; `<percentage>` (`opacity: 50%`) rejected. Group opacity per cell (documented).

**3.5 Backgrounds and borders (Backgrounds 3, Borders 4)**

- `border` — Partial: One keyword only: a style (`none` / `hidden` / `solid` / `double` / `dashed` / `dotted` / `ridge` / `groove` / `inset` / `outset`) plus rdom keywords; no width or color component — `border: 1px solid red` is dropped.
- `border-top` / `-right` / `-bottom` / `-left` — Partial: One style keyword only; no width / color.
- `border-style` — Partial: One value applied to all four sides; the 2–4-value form is rejected.
- `border-color` — Partial: One color for all sides; 2–4 values rejected.
- `border-top-color` / `-right-color` / `-bottom-color` / `-left-color` — Missing: Per-side glyph color.
- `border-width` / `border-*-width` — Missing: `0` = none; `thin` / `medium` = light; `thick` = heavy glyphs.
- `border-radius` / `border-*-radius` — Missing: Non-zero → rounded corner glyphs.
- `box-shadow` — Missing: One-cell offset shade; blur / spread N/A.
- `border-spacing` — Missing: Gaps between separated table cells.

**3.6 Box model and sizing (Box 3, Sizing 3/4)**

- `margin-trim` — Missing: Trim children's margins at the container edges.
- `width` / `height` — Partial: `auto`, cells, `%`, `calc()`, rdom `fr`; missing `min-content` / `max-content` / `fit-content` / `fit-content(<l>)` / `stretch`.
- `min-width` / `min-height` — Partial: `auto` / cells only; no `%`, `calc()`, intrinsic keywords.
- `max-width` / `max-height` — Partial: Cells / constant `calc()` only; `none` (the initial value), `%`, percent `calc()`, intrinsic keywords rejected.
- `box-sizing` — Missing: rdom is implicitly `border-box`; `content-box` (CSS initial) is not expressible.
- `contain-intrinsic-size` (+ longhands) — Missing: Placeholder size for `content-visibility: auto`.

**3.7 Display and visibility (Display 3)**

- `display: contents` — Missing: No box; children join the parent.
- `display: flow-root` — Missing: Block that establishes an independent BFC.
- Multi-keyword `display` (`block flex`, `inline flow-root`) — Missing: Two-value syntax.
- `visibility` — Missing: `visible` / `hidden` / `collapse`.
- `order` — Missing: Visual reorder of flex / grid items.

**3.8 Flexbox and box alignment (Flexbox 1, Align 3)**

- `flex-direction` — Partial: `row` / `column`; `row-reverse` / `column-reverse` rejected.
- `flex-wrap` — Missing: Single-line only.
- `flex-flow` — Missing: Shorthand of the two above.
- `flex-grow` — Missing *(DIVERGENCES says otherwise)*: Not in the property table (DIVERGENCES suggests `flex-grow: 1` as a workaround).
- `flex-basis` — Missing: Basis ignored (documented as part of `flex`); the longhand does not exist.
- `justify-content` — Missing: Main-axis distribution.
- `align-items` — Missing: Cross-axis placement (always `stretch` unless a cross margin is `auto`). `KW::Align` exists, unused.
- `align-self` — Missing: Per-item override.
- `align-content` — Missing: Needs `flex-wrap`.
- `justify-items` / `justify-self` — Missing: Grid / block-level alignment.
- `place-content` / `place-items` / `place-self` — Missing: Shorthands.
- `gap` — Partial: One value for both axes (cells, `%`, `calc()`); two-value form rejected.
- `row-gap` / `column-gap` — Missing: Per-axis gap.

**3.10 Positioned layout (Position 3, CSS 2.1 §9)**

- `top` / `right` / `bottom` / `left` — Partial *(DIVERGENCES says otherwise)*: `auto`, signed cells, `calc()`; bare `%` rejected.
- `inset` — Partial: 1–4 values of `auto` / signed cells; `calc()` and `%` rejected.
- `inset-block` / `inset-inline` (+ `-start` / `-end`) — Missing: Logical aliases.

**3.11 Overflow, scrolling and scrollbars (Overflow 3/4, Scroll Snap 1, Overscroll 1, Scrollbars 1)**

- `overflow` — Partial: `visible` / `hidden` / `scroll` / `auto`; `clip` and the two-value form rejected.
- `overflow-x` / `overflow-y` — Partial: Same keyword set; no `clip`.
- `overflow-block` / `overflow-inline` — Missing: Logical aliases.
- `overflow-clip-margin` — Missing: Cells outside the box that `clip` still paints.
- `text-overflow` — Missing: `clip` (today) / `ellipsis` / `<string>`.
- `line-clamp` / `max-lines` / `block-ellipsis` / `continue` — Missing: Row clamping with `…`.
- `scrollbar-gutter` — Partial: `auto` / `stable`; `both-edges` rejected.
- `overscroll-behavior` (+ `-x` / `-y` / `-block` / `-inline`) — Missing: Stop scroll chaining.
- `scroll-snap-type` / `scroll-snap-align` / `scroll-snap-stop` — Missing: Snap positions.

**3.12 Inline text (Text 3/4, Inline 3, CSS 2.1 §10.8)**

- `white-space` — Partial: `normal` / `pre` / `pre-wrap` / `nowrap`; `pre-line` and `break-spaces` rejected.
- `white-space-collapse` / `text-wrap-mode` (Text 4) — Missing: Longhands of `white-space`.
- `text-wrap` / `text-wrap-style` (Text 4) — Missing: `balance` / `pretty` / `stable` line breaking.
- `text-align-last` — Missing: Last-line alignment.
- `text-justify` — Missing: Justification method (`inter-word` is the only sensible one).
- `text-indent` — Missing: First-line indent in cells.
- `text-transform` — Missing: Case mapping, `full-width`.
- `word-break` — Missing: `break-all` / `keep-all`.
- `overflow-wrap` / `word-wrap` — Missing: `anywhere` / `break-word`.
- `line-break` — Missing: CJK break strictness; low priority.
- `line-height` — Missing: Whole-row line boxes.

**3.13 Text decoration (Text Decoration 3/4)**

- `text-decoration` — Partial *(DIVERGENCES says otherwise)*: One keyword: `none` / `underline` / `line-through` (SGR 4 / 9); `overline`, combinations, color and style components rejected; case-sensitive.
- `text-decoration-line` — Missing: Longhand; `overline` via SGR 53.
- `text-decoration-color` — Missing: SGR 58 underline color.
- `text-decoration-style` — Missing: SGR 4:1–4:5 (`solid` / `double` / `curly` = `wavy` / `dotted` / `dashed`).

**3.14 Fonts (Fonts 4)**

- `font-weight` — Partial: `normal` / `bold`; numeric weights, `bolder` / `lighter` rejected.
- `font-style` — Partial: `normal` / `italic`; `oblique [<angle>]` rejected.
- `font` — Missing: Shorthand: honor weight / style, ignore size / family.

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

- `inline-size` / `block-size` / `min-*-size` / `max-*-size` — Missing: Aliases of `width` / `height` / `min-*` / `max-*`.
- `margin-inline` / `margin-block` (+ `-start` / `-end`) — Missing: Aliases of physical margins.
- `padding-inline` / `padding-block` (+ `-start` / `-end`) — Missing: Aliases of physical padding.
- `border-inline` / `border-block` (+ `-start` / `-end`, `-color` / `-style` / `-width`) — Missing: Aliases of physical borders.
- `border-start-start-radius` / … (4 corners) — Missing: Aliases of `border-*-radius`.
- `text-align: start / end`, `float: inline-start`, `resize: block / inline` — Missing: Logical keywords (follow their properties).

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
   `PROPERTY_NAMES`; the declaration is `UnknownProperty`.
3. **`top` / `right` / `bottom` / `left` are said to take a bare percentage** (§Values, "Width /
   height / top / right / bottom / left / gap take both forms"). `parse_length` has no
   `Token::Percentage` arm; only `calc(…%)` works.
4. **`var()` is said to be consumed in `content`** (§Cascade & selectors). `parse_content` has
   no `var()` arm; only the Rust builder `Content::Var` reaches the resolver.
5. **`box-sizing`**: `BLOCK/width.rs` says the implicit `border-box` is "documented in
   `DIVERGENCES.md` under Values"; it is not.
6. **Units** (§1, "Length units"): `ch` and the viewport units are grouped with `px` / `em` as
   depending "on a pixel or font-size concept the terminal grid doesn't have". `1ch` is exactly
   one column on a monospaced grid and `vw` / `vh` are percentages of the terminal size rdom
   already tracks; the classification is a choice, not a medium constraint.
