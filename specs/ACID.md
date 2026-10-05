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
| 7 | **Flex layout** | `flex` shorthand grow / shrink / basis; `gap`; shrinking to min-content; column direction; cross-axis stretch; auto margins | Item widths and positions equal to the Flexbox §9.7 algorithm's output. |
| 8 | **Inline formatting** | inline spans with background / bold / italic / underline across a wrap; `white-space` normal / nowrap / pre / pre-wrap; collapsing; inline-block atoms with padding and border; a long unbreakable word | Exact line breaks and per-cell styles; a span's background continues on its next line. |
| 9 | **Generated content & counters** | `::before` / `::after` with strings, `attr()` and `counter()`; nested `counter-reset` / `counter-increment`; `ol > li > p` markers; a block-first `::before` on its own line; inline-element pseudos inside a wrapped line | Exact marker numbers and positions; pseudo text wraps with its line (DIVERGENCES §list markers). |
| 10 | **Positioning** | `relative` offsets; `absolute` with `top` / `right` / `bottom` / `left` / `inset` against the nearest positioned ancestor; auto offsets at the static position; `fixed` against the viewport | Boxes at the CSS 2.1 §10.3.7 / §10.6.4 positions. |
| 11 | **Stacking contexts** | overlapping positioned boxes with negative, zero and positive `z-index`; a `z-index` contest inside a child context that cannot escape it; positioned boxes above in-flow content | The Appendix E paint order, visible as which box's glyphs and colour sit on top in each overlap cell. |
| 12 | **Group opacity** | `0.5` inside `0.5` (= 0.25); translucent box over borders; translucent box over text; `opacity: 0`; the one-glyph-per-cell rule | Exact blended colours computed from the CSS Color 4 compositing formula; glyph choice per the documented contest (DIVERGENCES §opacity). |
| 13 | **Overflow & scrollbars** | `overflow: hidden` clipping text and child boxes; `overflow: auto` with a pre-scrolled offset; `scrollbar-gutter: stable`; `::scrollbar` / `::scrollbar-thumb` styling; a nested scroll container that must not leak into its parent (the `SCROLL-OVERFLOW-NESTED-ANON-1` regression) | Exact clip edges, thumb size and position, styled scrollbar cells; the outer box has no scrollbar. |
| 14 | **Tables & border collapse** | collapsed borders with conflict resolution (`hidden` wins, double beats solid, wider wins); `colspan`; junction glyphs | The CSS Tables 3 §11.5 winners, visible in the junction and edge glyphs. |
| 15 | **Form controls** | text input and textarea UA chrome (padding, field background); `::placeholder` styled; checkbox / radio glyphs with `checked`; button labels including a value-less submit; closed `<select>`; `<progress>` / `<meter>` / range; `<fieldset disabled>` → `:disabled`; author `:invalid` styling on a `required` empty field | Each control's UA rendering exactly; disabled and invalid controls in their author colours. |
| 16 | **Display & visibility** | `display: none` (takes no space); inline vs block vs inline-block vs flex; `<details>` closed and open; the `hidden` attribute; `:empty` boxes | Exact presence and absence of boxes and the space they take. |
| 17 | **Selection & `user-select`** | a pre-set selection range crossing `user-select: none`, `contain` and `all` regions; `::selection` styled; generated content never highlighted | Highlighted cells exactly where the used `user-select` value allows (CSS UI 4 §6.1). |

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

## Coverage — gaps found while inventorying

Building the inventory turned up CSS that rdom does not parse at all and that `DIVERGENCES.md`
mostly does not mention. The acid page can only use what is supported, so these need a decision
— ship them, or document them as not yet shipped — before the coverage test can be honest:

- **Flex alignment:** `justify-content`, `align-items`, `align-self` (the layout engine has the
  alignment types; CSS cannot set them).
- ~~**Flex:** `flex-wrap`; the `flex-grow` / `flex-basis` longhands (only the `flex` shorthand and
  `flex-shrink` parse).~~ Shipped: C6-FLEX-LONGHANDS (the longhands), C6-WRAP (`flex-wrap`,
  `flex-flow`).
- **Text:** `line-height`, `text-overflow`, `white-space: pre-line`.
- **Overflow:** a non-clipping descendant's text lines that overflow its box do not count toward
  the ancestor's scrollable overflow (found while fixing `SCROLL-OVERFLOW-NESTED-ANON-1`).

## Proposed build order

1. Decide the gaps above (ship, or move to DIVERGENCES §3).
2. The reference format and the colour-aware comparator, with the per-tile failure report.
3. Tiles 1–17, each reference reviewed against its spec section before it is committed.
4. The coverage test.
5. Stage 2.
