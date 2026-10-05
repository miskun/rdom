# ACID — an rdom acid test

**Status:** PROPOSAL (2026-10-03) — tile list for review; nothing built yet.

## Why

Most rdom tests check one feature at a time. The bugs found after 0.5.0 all came from features
meeting each other: group opacity over borders over text, a scroll container's whitespace boxes
inside another scroll container, a late-inserted input inside a flex row. An acid page renders
many features together, into one picture that is visibly right or wrong — the idea of the early
web Acid tests, adapted to a terminal grid.

## Ground rules

1. **The reference comes from the spec, not from rdom.** Every expected cell is derived by hand
   from what CSS / HTML mandate (or, where rdom deliberately differs, from the cited
   `DIVERGENCES.md` entry). A snapshot recorded from rdom's own output would only pin today's
   bugs, as the translucency snapshot did.
2. **Colour-aware comparison.** The reference records glyph, foreground, background and
   modifiers (bold / italic / underline) per cell, through a small legend (`a` = `rgb(…) on rgb(…)
   bold`). Glyph-only snapshots let the bright-`p` opacity artefact through.
3. **Tiles, not one picture.** The page is a grid of labelled tiles, each one feature
   *interaction*. A failure reports the tile, the cell and expected-vs-actual, so it points at a
   combination, not "the page differs".
4. **Coverage is enforced mechanically.** A companion test fails when any CSS property name rdom
   dispatches (`property_dispatch::property_names()`), any `PseudoClass` or any
   `PseudoElementTarget` is not used somewhere in the acid page. New features cannot skip it.
5. **Two stages.** *Static* (one frame, no input) first; *interactive* (scripted input against the
   headless `App`, like Acid3's scripted half) second.
6. **It lives in the showcase** (`Built-ins → Acid`, plus `cargo run -p rdom-showcase --example
   acid`), with the reference and the tests in `crates/rdom-showcase/tests/`. A fixed viewport
   (proposed 120 × 50) keeps the reference stable.
7. **Fixing what it finds** happens as separate TDD items in the crate that owns the bug, never by
   adjusting the reference.

## Stage 1 — static tiles

Each tile: what it combines → what the spec says the cells must show.

| # | Tile | Combines | Expected result (from the spec) |
|---|---|---|---|
| 1 | **Cascade order** | UA vs author sheet vs a later pushed sheet vs a `<style>` element vs inline `style=""`; `!important` in a sheet vs a normal inline style; equal-specificity ties decided by order | A row of words, each coloured by exactly the declaration CSS Cascade 4 §6 says wins; any wrong winner shows as a wrong colour. Covers the live-`<style>`-before-App-sheets order (DIVERGENCES §Cascade). |
| 2 | **Specificity** | type < class < id; `:where()` adds zero; `:not()` adds its argument's; selector lists take the matching branch; attribute = class weight | Same word-colour technique; one cell per specificity contest. |
| 3 | **Inheritance & keywords** | inherited `color` / `font-*`; non-inherited `background` / `user-select`; `inherit` / `initial` / `unset` on both kinds; custom properties with `var()` and fallback, including a missing var | Nested boxes whose colours prove which values inherited; `unset` on `user-select` behaves as `initial`. |
| 4 | **Selectors** | descendant, `>`, `+`, `~`; attribute `=` `~=` `\|=` `^=` `$=` `*=`; case-insensitive `type` values; `:first-child` `:last-child` `:only-child` `:empty` `:root`; comma lists | A small list where each item is styled by exactly one selector; unmatched items stay unstyled. |
| 5 | **Box model** | fixed / percent / `calc()` widths and heights; `min-*` / `max-*` clamping; `aspect-ratio`; per-side padding; per-side border styles (solid, double, rounded, dashed→solid, hidden) and colours | Boxes of exact, spec-computed sizes; border glyphs per style. |
| 6 | **Margin collapsing** | adjacent siblings; parent / first child; empty collapse-through block; negative margins; collapse blocked by padding, border and a line box; no collapse between flex items | Row offsets equal to the CSS 2.1 §8.3.1 results; flex-item case per DIVERGENCES (additive). |
| 7 | **Flex layout** | `flex` shorthand grow / shrink / basis; `row-gap` / `column-gap`; shrinking to min-content; row (the initial direction) and column, `row-reverse` / `column-reverse`; `flex-wrap` / `wrap-reverse` lines; `order`; `justify-content`, `align-items` / `align-self` (baselines included), `align-content`; cross-axis stretch; auto margins; anonymous and `::before` / `::after` items | Item widths and positions equal to the Flexbox §9.7 algorithm's output. |
| 8 | **Inline formatting** | inline spans with background / bold / italic / underline across a wrap; `white-space` normal / nowrap / pre / pre-wrap; collapsing; inline-block atoms with padding and border; a long unbreakable word | Exact line breaks and per-cell styles; a span's background continues on its next line. |
| 9 | **Generated content & counters** | `::before` / `::after` with strings, `attr()` and `counter()`; nested `counter-reset` / `counter-increment`; `ol > li > p` markers; a block-first `::before` on its own line; inline-element pseudos inside a wrapped line | Exact marker numbers and positions; pseudo text wraps with its line (DIVERGENCES §list markers). |
| 10 | **Positioning** | `relative` offsets; `absolute` with `top` / `right` / `bottom` / `left` / `inset` against the nearest positioned ancestor; auto offsets at the static position; `fixed` against the viewport | Boxes at the CSS 2.1 §10.3.7 / §10.6.4 positions. |
| 11 | **Stacking contexts** | overlapping positioned boxes with negative, zero and positive `z-index`; a `z-index` contest inside a child context that cannot escape it; positioned boxes above in-flow content | The Appendix E paint order, visible as which box's glyphs and colour sit on top in each overlap cell. |
| 12 | **Group opacity** | `0.5` inside `0.5` (= 0.25); translucent box over borders; translucent box over text; `opacity: 0`; the one-glyph-per-cell rule | Exact blended colours computed from the CSS Color 4 compositing formula; glyph choice per the documented contest (DIVERGENCES §opacity). |
| 13 | **Overflow & scrollbars** | `overflow: hidden` clipping text and child boxes; `overflow: auto` with a pre-scrolled offset; `overflow: clip` per axis with `overflow-clip-margin`; `text-overflow: ellipsis` (an `rtl` line overflowing its left edge, C8-RTL-LINE-OVERFLOW) and `line-clamp`; an absolutely positioned box past the content counting in `scrollHeight` (C8-ABSPOS-OVERFLOW); `scrollbar-gutter: stable` and `stable both-edges`; `::scrollbar` / `::scrollbar-thumb` styling beside `scrollbar-width: thin` and `scrollbar-color`, which turn the pseudo-elements off (C8-SCROLLBAR); a nested scroll container that must not leak into its parent (the `SCROLL-OVERFLOW-NESTED-ANON-1` regression) | Exact clip edges, thumb size and position, styled scrollbar cells; the outer box has no scrollbar. |
| 14 | **Tables & border collapse** | collapsed borders with conflict resolution (`hidden` wins, double beats solid, wider wins); `colspan`; junction glyphs | The CSS Tables 3 §11.5 winners, visible in the junction and edge glyphs. |
| 15 | **Form controls** | text input and textarea UA chrome (padding, field background); `::placeholder` styled; checkbox / radio glyphs with `checked`; button labels including a value-less submit; closed `<select>`; `<progress>` / `<meter>` / range; `<fieldset disabled>` → `:disabled`; author `:invalid` styling on a `required` empty field | Each control's UA rendering exactly; disabled and invalid controls in their author colours. |
| 16 | **Display & visibility** | `display: none` (takes no space); inline vs block vs inline-block vs flex vs inline-flex; `display: contents` (no box, children joining the parent); `flow-root` (no margin collapse through it); the multi-keyword syntax; `visibility: hidden` (space kept, nothing drawn) and `collapse` (a flex item's strut, a table row's removal); flex items blockified; `<details>` closed and open; the `hidden` attribute; `:empty` boxes | Exact presence and absence of boxes and the space they take. |
| 17 | **Selection & `user-select`** | a pre-set selection range crossing `user-select: none`, `contain` and `all` regions; `::selection` styled; generated content never highlighted | Highlighted cells exactly where the used `user-select` value allows (CSS UI 4 §6.1). |
| 18 | **Grid layout** (proposed with Phase 7, C7-*) | `grid-template` with named areas, row sizes and line names; `fr` / `minmax()` / `fit-content()` / `repeat(auto-fill)` tracks and `auto-fit` collapse; line-based and area placement beside `dense` auto-placement into implicit tracks; spanning items in intrinsic tracks; `justify-self` / `align-self` (a baseline row group, an `aspect-ratio` item), `auto` margins, `justify-content: space-between` widening a spanned area; a `subgrid` with padding whose items size the parent's columns; an absolutely positioned child in a named area of a bordered grid; a long unbreakable word in a `1fr` column beside the same word in a `minmax(0, 1fr)` one; two items in one cell ordered by `z-index`; an `inline-grid` inside a line of text; an `rtl` grid; an item spanning `grid-column: 1 / -1` (Phase 7 API gate) | Track sizes equal to the CSS Grid 2 §11 algorithm in whole cells (DIVERGENCES §1), every item at its §8 area and §10 alignment, the subgrid's items on the parent's lines (§9), the abspos box inside the padding edge (§9.1, CSS 2.1 §10.1); the `1fr` column held at the word's min-content width and the `minmax(0, 1fr)` one shrunk with the word overflowing (§6.6, §7.2.4); the higher `z-index` item on top (§6.5); the `inline-grid` on the text's baseline row as an atom (§5.1, DIVERGENCES §2); the `rtl` grid's first column on the right (§7.1); the spanning item across every explicit column (§8.3). |
| 19 | **Floats** (proposed with C8-FLOAT) | a left and a right float beside a paragraph that wraps around both; a third float that does not fit going below (CSS 2.1 §9.5.1 rules 2–3, 7–8); a float met mid-line joining its line; a word too wide beside a float moving below it; `clear: left / both`; a `flow-root` box containing its floats (§10.6.7) and an `overflow: hidden` box going beside the parent's float; `float: inline-start` under `rtl`; a float inside `display: contents`; a block's background under a float (Appendix E step 5) | Line breaks and float rects exactly as §9.5 places them; the float's cells over the next block's background; no float in a flex row floating. |

## Stage 2 — interactive script

Driven against the headless `App` with a controllable clock; each step compares the affected
tiles to a per-step reference.

| # | Step | Checks |
|---|---|---|
| I1 | Pointer over a nested child | `:hover` on the child **and its ancestors** (Selectors 4 §9.2); sibling combinators reading `:hover` restyle |
| I2 | Press and release | `:active` set on press, cleared before `mouseup` / `click` run (Blink order) |
| I3 | Tab vs click focus | `:focus-visible` after Tab, not after a mouse click on a button; always on a text field |
| I4 | Type into the `required` field | `:invalid` → `:valid`; the form's `:valid` follows |
| I5 | Toggle a checkbox and a radio group | `:checked`; the radio group follows its form owner |
| I6 | Advance the clock mid-transition | the transitioned colour equals the timing function's value at that time |
| I7 | Rewrite a `<style>` element and an inline style through the CSSOM | both restyle on the next frame, in the documented cascade order |
| I8 | Smooth scroll and `scrollIntoView` | offsets at a fixed time and at settle; outer containers untouched |
| I9 | Caret blink and an edit | caret cell on / off per phase; the edit lands in the focused control |
| I10 | `pointer-events: none` overlay | a click passes through to the box underneath |
| I11 | Wheel, keys and Tab in scroll containers (C8-OVERSCROLL, C8-SCROLL-PADDING, C8-SNAP) | a `y mandatory` snap container moving one item per wheel tick and stopping at a `scroll-snap-stop: always` item; a `proximity` one resting between items; an `overscroll-behavior: contain` inner scroller not chaining to its parent at its end; Tab revealing a focused item inside the `scroll-padding`; a re-snap after an item is inserted above the snapped one |

## Coverage — gaps found while inventorying

Building the inventory turned up CSS that rdom does not parse at all and that `DIVERGENCES.md`
mostly does not mention. The acid page can only use what is supported, so these need a decision
— ship them, or document them as not yet shipped — before the coverage test can be honest:

- ~~**Flex alignment:** `justify-content`, `align-items`, `align-self` (the layout engine has the
  alignment types; CSS cannot set them).~~ Shipped: C6-JUSTIFY, C6-ALIGN (the alignment types —
  `Align`, with `Alignment` — set by CSS).
- ~~**Flex:** `flex-wrap`; the `flex-grow` / `flex-basis` longhands (only the `flex` shorthand and
  `flex-shrink` parse).~~ Shipped: C6-FLEX-LONGHANDS (the longhands), C6-WRAP (`flex-wrap`,
  `flex-flow`).
- ~~**Grid:** not usable for a tile before Phase 7.~~ Shipped: C7-GRID-CORE, -AUTO, -PLACE, -AREAS,
  -ALIGN, -RERESOLVE, C7-SUBGRID. Proposed as tile 18 of its own rather than an extension of tile 7:
  grid is a second layout algorithm (§11 track sizing, §8 placement) whose interactions — areas
  with placement, alignment with spanned tracks, a subgrid sizing its parent — would double tile 7's
  cell budget and blur which algorithm a failing cell points at.
- **Text:** `line-height`, `white-space: pre-line` (`text-overflow` shipped: C8-TEXT-OVERFLOW).
- ~~**Overflow:** a non-clipping descendant's text lines that overflow its box do not count toward
  the ancestor's scrollable overflow (found while fixing `SCROLL-OVERFLOW-NESTED-ANON-1`).~~
  Shipped: C8-OVERFLOW-TEXT.

## Proposed build order

1. Decide the gaps above (ship, or move to DIVERGENCES §3).
2. The reference format and the colour-aware comparator, with the per-tile failure report.
3. Tiles 1–19, each reference reviewed against its spec section before it is committed.
4. The coverage test.
5. Stage 2.
