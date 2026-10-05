# CSS-COMPLETE-2026-10 — every CSS feature that means something in a terminal

**Status:** IN PROGRESS (started 2026-10-03). Ships as **0.6.0**. Routing moves to 0.7.0, async
tasks to 0.8.0.

**Goal.** Implement every row of [`CSS-COVERAGE.md`](CSS-COVERAGE.md) classed *Partial* or
*Missing — applicable*, so that ordinary CSS written for a browser works in rdom wherever it has a
meaning on a character grid. Rows classed *Not applicable* (fonts, images, sub-cell geometry,
print, 3D) stay out, each with its one-line reason in `DIVERGENCES.md`.

**Decided exclusions** (spec unstable or at risk, recorded in `DIVERGENCES.md` §2 with the reason):
`masonry` / `grid-lanes` (Grid 3 WD, syntax still moving), `:blank` (Selectors 4 marks it at
risk), `nav-up` / `nav-down` / `nav-left` / `nav-right` (UI 4 at risk). Everything else in the
audit's two classes is in scope — including `float` / `clear` and a real table formatting context,
which earlier programs had declined.

**Rules.** As in STABILIZE: one item per commit, failing test first (the test cites the spec
section), per-crate tests then the workspace gate (`fmt`, `clippy -D warnings`, `test --workspace`,
rustdoc `-D warnings`), docs move with the code (`DIVERGENCES.md`, `CSS-COVERAGE.md` row updated,
CHANGELOG `## [Unreleased]`), grumpy architect + API gates at the end of each phase, findings fixed
before the next phase starts. Files that grow past the few-hundred-line bar while a phase works in
them are split in that phase (CLAUDE.md §Architecture Hygiene); each phase's architect gate checks
this. The 0.5.1 fixes already in `## [Unreleased]` ship with 0.6.0.

**Tracking.** The item tables below are the program's todo list. Each row's status is updated in
the commit that lands it (`done <sha>`), and the log at the end records phase gates and decisions.

## Phases

| Phase | Scope | Status |
|---|---|---|
| 0 | Docs truthful: DIVERGENCES contradictions fixed, every undocumented gap listed, roadmap moved | done |
| 1 | Syntax, cascade, custom properties | done 2026-10-05 (both gates; 20 gate fixes `C1G-*`; their re-review rides with the Phase 2 gate) |
| 2 | Values, units, math functions | done 2026-10-05 (both gates; 20 gate fixes `C2G-*`; their re-review rides with the Phase 3 gate; C2-LH partial until C9-LINE-HEIGHT) |
| 3 | Color | done 2026-10-06 (both gates; 16 gate fixes `C3G-*` incl. rdom's own terminal input reader; re-review rides with the Phase 4 gate) |
| 4 | Backgrounds and borders | done 2026-10-06 (both gates; 19 gate fixes `C4G-*`; their re-review rides with the Phase 5 gate; C4-SPACING layout with C13-TFC) |
| 5 | Box model and sizing (incl. logical properties) | done 2026-10-07 (both gates; 19 gate fixes `C5G-*`; their re-review rides with the Phase 6 gate; C5-CONTAIN-SIZE use with C14-CONTAIN) |
| 6 | Display, visibility, flexbox, box alignment | done 2026-10-08 (both gates; 28 gate fixes `C6G-*`; their re-review rides with the Phase 7 gate) |
| 7 | Grid | gates run 2026-10-09; `C7G-*` fixes in progress |
| 8 | Positioning, floats, overflow, scrolling | |
| 9 | Inline text and decoration | |
| 10 | Lists, counters, generated content, pseudo-elements | |
| 11 | Selectors | |
| 12 | Transitions, animations, user interface | |
| 13 | Tables (real table formatting context) | |
| 14 | Conditional rules, containment | |
| 15 | Transforms, filters, compositing, multi-column, anchor positioning | |
| 16 | Acid test (static tiles + interactive script, coverage-enforced) — `ACID.md` | |
| 17 | Release 0.6.0 (publish on Miska's go-ahead) | |

Phases follow dependencies: values and color before the properties that use them; flex alignment
before grid (Box Alignment is shared); text before lists (markers are inline content); everything
before the acid test, whose coverage check needs the final property table.

## Items

Ids are `C<phase>-<slug>`. Each row is one commit unless noted; the audit section names where the
row comes from.

### Phase 1 — Syntax, cascade, custom properties (audit §3.1, §3.2)

| Id | Item | Status |
|---|---|---|
| C1-ESCAPES | Identifier escapes in selectors and values (`\31 0`, `\:`) — Syntax 3 §4.3.7 | done |
| C1-CASE | Property names and all keywords ASCII case-insensitive | done |
| C1-REVERT | `revert` (roll back to the UA origin) | done |
| C1-LAYER | `@layer` (statement + block, anonymous layers, layer order) and `revert-layer` | done |
| C1-ALL | `all` shorthand | done |
| C1-IMPORT | `@import` through a host-provided loader, with layer / supports / media conditions | done |
| C1-SCOPE | `@scope` with an optional lower bound, and `:scope` inside it | done |
| C1-NESTING | CSS Nesting (`&`, nested style rules, nested at-rules) | done |
| C1-VAR-ANY | `var()` in every property via token-level substitution at computed-value time; fallback with arbitrary tokens; `var()` in `content` | done |
| C1-PROPERTY | `@property` (syntax, inherits, initial-value) | done |
| C1-INLINE-IMPORTANT | Inline `style="… !important"` beats author `!important` (Cascade 4 §6.1 element-attached styles; found during C1-REVERT) | done |

### Phase 2 — Values, units, math functions (audit §3.3)

| Id | Item | Status |
|---|---|---|
| C2-PERCENT | `<percentage>` everywhere the spec allows (padding, margin, insets, min/max sizes, opacity) | done |
| C2-NUMBER | Fractional `<number>` where the spec allows (flex factors, …) | done |
| C2-MINMAX | `min()` / `max()` / `clamp()` | done |
| C2-STEPPED | `round()` / `mod()` / `rem()` / `abs()` / `sign()` | done |
| C2-TRIG | `sin()` … `atan2()`, `pow()` / `sqrt()` / `hypot()` / `log()` / `exp()` | done |
| C2-CH | `ch` (one column) | done |
| C2-LH | `lh` / `rlh` (one row × `line-height`; lands with C9-LINE-HEIGHT) | partial — revisit with C9-LINE-HEIGHT (one row each until `line-height` exists) |
| C2-VIEWPORT | `vw` / `vh` / `vmin` / `vmax` and the `sv*` / `lv*` / `dv*` / `vi` / `vb` variants (terminal size) | done |
| C2-ANGLE | `<angle>` (`deg` / `grad` / `rad` / `turn`) | done |
| C2-RATIO | Full `<ratio>` (bare numbers, decimals, `auto && <ratio>`) | done |
| C2-ATTR | `attr()` with fallback and `type()` (Values 5) | done |

(`cq*` units land with C14-CONTAINER.)

### Phase 3 — Color (audit §3.4)

| Id | Item | Status |
|---|---|---|
| C3-RGB | Modern `rgb()` / `rgba()`: space syntax, `/ alpha`, percentages, `none` | done |
| C3-TRANSPARENT | `transparent` as a real fully transparent color (not `Reset`) | done |
| C3-CURRENTCOLOR | `currentColor` | done |
| C3-HSL-HWB | `hsl()` / `hsla()` / `hwb()` | done |
| C3-LAB | `lab()` / `lch()` / `oklab()` / `oklch()` / `color()` with gamut mapping to sRGB | done |
| C3-MIX | `color-mix()` | done |
| C3-RELATIVE | Relative color syntax (`rgb(from …)`) | done |
| C3-SYSTEM | System colors (`Canvas`, `CanvasText`, `LinkText`, `ButtonFace`, …) | done |
| C3-SCHEME | `color-scheme` and `light-dark()` (terminal background via OSC 11 / mode 2031) | done (mode 2031 with C3G-INPUT-READER) |
| C3-ALPHA | Color alpha composited over the backdrop (shares the opacity compositor) | done |

### Phase 4 — Backgrounds and borders (audit §3.5)

| Id | Item | Status |
|---|---|---|
| C4-BACKGROUND | `background` shorthand (color layer; image layers parse and are inert, documented) | done |
| C4-BG-CLIP | `background-clip` (`border-box` / `padding-box` / `content-box`) | done |
| C4-BORDER-SHORTHAND | `border` / `border-top` … with width, style and color in any order | done |
| C4-BORDER-SIDES | `border-style` / `border-color` / `border-width` 1–4 values; per-side longhands for style, color and width | done |
| C4-BORDER-WIDTH | `border-width` mapping (`0` = none, thin / medium = light, thick = heavy glyphs) | done |
| C4-RADIUS | `border-radius` and per-corner longhands → rounded corner glyphs | done |
| C4-SHADOW | `box-shadow` (one-cell offset shade; blur / spread documented N/A) | done |
| C4-SPACING | `border-spacing` (lands with the table phase if it needs the TFC) | partial — layout lands with C13-TFC |

### Phase 5 — Box model and sizing (audit §3.6, §3.22)

| Id | Item | Status |
|---|---|---|
| C5-BOX-SIZING | `box-sizing` (`content-box` is the CSS initial value — breaking default change, migration note) | done |
| C5-INTRINSIC | `min-content` / `max-content` / `fit-content()` on width / height / min / max | done |
| C5-MINMAX-SIZE | `min-*` / `max-*`: `none`, `%`, `calc()` | done (`%` / `calc()` with C2-PERCENT, `none` with C2G-MAX-NONE) |
| C5-MARGIN-TRIM | `margin-trim` | done |
| C5-CONTAIN-SIZE | `contain-intrinsic-size` (+ longhands) | partial — used with C14 contain |
| C5-LOGICAL | Logical properties: `inline-size` / `block-size` / `min-*` / `max-*`, `margin-*` / `padding-*` / `border-*` / `inset-*` / radius logical forms (horizontal-tb ltr mapping) | done |
| C5-WRITING | `direction` and `writing-mode` for the values a terminal can render (rtl lines; vertical documented N/A if not) | done |

### Phase 6 — Display, visibility, flexbox, box alignment (audit §3.7, §3.8)

| Id | Item | Status |
|---|---|---|
| C6-MARGIN-SIDES | `margin` / `padding` stored per side (each side its own longhand with its own `!important` bit), so a logical or physical side cascades alone — finishes C5G-LOGICAL-IMPORTANT || done |
| C6-DISPLAY-KEYWORDS | `display: contents` / `flow-root` / multi-keyword syntax | done |
| C6-VISIBILITY | `visibility: visible / hidden / collapse` | done |
| C6-ORDER | `order` | done |
| C6-DIRECTION-REVERSE | `flex-direction: row-reverse / column-reverse` | done |
| C6-FLEX-LONGHANDS | `flex-grow` / `flex-basis` longhands; full `flex` shorthand (incl. basis) | done |
| C6-FLEX-DIRECTION-INITIAL | `flex-direction` initial value `row` (Flexbox §5.1): decouple the block-container axis from `flex-direction`, remove the DIVERGENCES §2 entry | done |
| C6-WRAP | `flex-wrap` / `flex-flow`, multi-line flex containers | done |
| C6-JUSTIFY | `justify-content` (all distribution values) | done |
| C6-ALIGN | `align-items` / `align-self` (incl. `baseline` where meaningful) | done |
| C6-ALIGN-CONTENT | `align-content` | done |
| C6-PLACE | `place-content` / `place-items` / `place-self`, `justify-items` / `justify-self` (block-level) | done |
| C6-GAP | `row-gap` / `column-gap` and two-value `gap` | done |
| C6-SPLIT | File-size pass on `layout_pass/flex/*` after the above | done |

### Phase 7 — Grid (audit §3.9)

| Id | Item | Status |
|---|---|---|
| C7-GRID-CORE | `display: grid` / `inline-grid`, `grid-template-columns` / `-rows` with cells / `%` / `fr` / `auto` / `minmax()` / `repeat()` | done |
| C7-GRID-PLACE | `grid-row` / `grid-column` (+ start / end), `grid-area`, auto-placement, `grid-auto-flow` (`dense`) | done |
| C7-GRID-AREAS | `grid-template-areas`, `grid-template`, `grid` shorthands | done |
| C7-GRID-AUTO | `grid-auto-columns` / `grid-auto-rows` | done |
| C7-GRID-ALIGN | Box Alignment in grid (`justify-*` / `align-*` / `place-*`) | done |
| C7-SUBGRID | `subgrid` | done |
| C7-GRID-RERESOLVE | CSS Grid 2 §11.1 steps 3–4: the columns, then the rows, sized again once when the rows changed an item's column contribution (part 1 follow-up) | done |
| C7-ABSPOS-PADDING-EDGE | An absolutely positioned box's containing block is its positioned ancestor's padding box (CSS 2.1 §10.1, Grid §9.1) (part 1 follow-up) | done |
| C7-SPLIT | File-size pass on `layout_pass/grid/*` and the files Phase 7 touched (TECH_DEBT `SIZE-1`) | done |

### Phase 8 — Positioning, floats, overflow, scrolling (audit §3.10, §3.11)

| Id | Item | Status |
|---|---|---|
| C8-INSETS | `top` / `right` / `bottom` / `left` / `inset`: `%` and `calc()` | done (with C2-PERCENT) |
| C8-Z-INDEX | `z-index` full integer range | |
| C8-FLOAT | `float` / `clear` (line-box exclusion, clearance) | |
| C8-OVERFLOW-CLIP | `overflow: clip`, two-value `overflow`, `overflow-clip-margin`, logical `overflow-block` / `-inline` | |
| C8-TEXT-OVERFLOW | `text-overflow: clip / ellipsis / <string>` | |
| C8-LINE-CLAMP | `line-clamp` / `max-lines` / `block-ellipsis` / `continue` | |
| C8-SCROLLBAR | `scrollbar-gutter: both-edges`, `scrollbar-width`, `scrollbar-color` | |
| C8-OVERSCROLL | `overscroll-behavior` (+ axis / logical longhands) | |
| C8-SCROLL-PADDING | `scroll-padding*` / `scroll-margin*` | |
| C8-SNAP | `scroll-snap-type` / `-align` / `-stop` | |
| C8-OVERFLOW-TEXT | A non-clipping descendant's overflowing line boxes count toward the ancestor's scrollable overflow | |
| C8-CB-COMPLETE | Containing block for positioned boxes, completing C7-ABSPOS-PADDING-EDGE: `sticky` ancestors in the ancestor walks, the scrollbar gutter excluded, scroll offsets applied inside a positioned scroller, one shared ancestor walk (elements and pseudo-elements, incl. the §9.1 grid area for pseudos) | |
| C8-POS-MINMAX | Positioned boxes (elements and pseudo-elements) honour `min-*` / `max-*` (CSS 2.1 §10.4 / §10.7 with §10.3.7 / §10.6.4); a `position: relative` pseudo with both insets takes its declared width | done (C5-POS-MINMAX + C5G-REL-PSEUDO-INSETS) |

(Scroll-driven animations land in phase 12.)

### Phase 9 — Inline text and decoration (audit §3.12, §3.13, §3.14)

| Id | Item | Status |
|---|---|---|
| C9-WHITE-SPACE | `white-space: pre-line / break-spaces`; `white-space-collapse` / `text-wrap-mode` longhands | |
| C9-TEXT-WRAP | `text-wrap` / `text-wrap-style` (`balance` / `pretty` / `stable`) | |
| C9-TEXT-ALIGN | `text-align` (incl. `start` / `end` / `justify`), `text-align-last`, `text-justify` | |
| C9-TEXT-INDENT | `text-indent` | |
| C9-TEXT-TRANSFORM | `text-transform` (case mapping, `full-width`) | |
| C9-TAB-SIZE | `tab-size` and real tab stops in `pre` | |
| C9-BREAKING | `word-break`, `overflow-wrap` / `word-wrap`, `line-break`, `hyphens` (soft hyphens) | |
| C9-LINE-HEIGHT | `line-height` (whole-row line boxes) | |
| C9-VERTICAL-ALIGN | `vertical-align` for inline-blocks and inline content | |
| C9-DECORATION | `text-decoration` full shorthand; `-line` (incl. `overline`), `-color` (SGR 58), `-style` (SGR 4:x) | |
| C9-FONT | `font-weight` numeric / `bolder` / `lighter`, `font-style: oblique`, `font` shorthand (weight / style honored, size / family inert) | |

### Phase 10 — Lists, counters, generated content, pseudo-elements (audit §3.15, §3.16)

| Id | Item | Status |
|---|---|---|
| C10-CONTENT | `content` full grammar (quotes, `var()`, `counters()`, alt text) | |
| C10-QUOTES | `quotes` | |
| C10-COUNTERS | `counter-reset reversed()`, `counter-set`, `counters()`, all predefined counter styles | |
| C10-COUNTER-STYLE | `@counter-style` and `symbols()` | |
| C10-LIST-ITEM | `display: list-item`, `list-style-type` / `-position` / `list-style`, `marker-side`, `::marker` (replaces the `li::before` divergence) | |
| C10-FIRST | `::first-line` / `::first-letter` | |
| C10-LEGACY-COLON | Single-colon `:before` / `:after` / `:first-line` / `:first-letter` | |
| C10-HIGHLIGHT | `::highlight()` with a Custom Highlight API surface | |
| C10-DETAILS-CONTENT | `::details-content` | |
| C10-PSEUDO-CHAINS | Pseudo-element followed by user-action pseudo-classes (`::before:hover`) and nested pseudo-elements where defined | |

### Phase 11 — Selectors (audit §3.17)

| Id | Item | Status |
|---|---|---|
| C11-ATTR-FLAGS | Attribute selector case flags `i` / `s` | |
| C11-IS | `:is()` | done (landed early as C1G-IS-PARSE) |
| C11-HAS | `:has()` with invalidation | |
| C11-NTH | `:nth-child()` / `:nth-last-child()` (+ `of S`), `:nth-of-type()` / `:nth-last-of-type()`, `:first-of-type` / `:last-of-type` / `:only-of-type` | |
| C11-SCOPE | `:scope` (query APIs and `@scope`) | |
| C11-FORM-STATES | `:indeterminate` (checkbox, radio group), `:user-valid` / `:user-invalid`, `:read-only` / `:read-write`, `:in-range` / `:out-of-range`, `:default` | |
| C11-MODAL-POPOVER | `:modal`; the `popover` attribute and `:popover-open` | |
| C11-LINK-LANG | `:link` / `:any-link`, `:lang()` | |
| C11-COLUMN | Column combinator `\|\|` (with the table phase) | |

### Phase 12 — Transitions, animations, user interface (audit §3.18, §3.19)

| Id | Item | Status |
|---|---|---|
| C12-TIMING | `transition-timing-function` full (`linear()`, `steps()` positions), negative `transition-delay` | |
| C12-BEHAVIOR | `transition-behavior: allow-discrete` | |
| C12-ANIMATABLE | Every animatable property this program adds interpolates | |
| C12-KEYFRAMES | `@keyframes` and all `animation-*` properties, animation events | |
| C12-STARTING | `@starting-style` | |
| C12-SCROLL-DRIVEN | `scroll-timeline*` / `view-timeline*` / `animation-timeline` / `animation-range*` | |
| C12-OUTLINE | `outline` / `-color` / `-style` / `-width` / `-offset` (non-layout ring) | |
| C12-CURSOR | `cursor` (OSC 22 pointer shapes) | |
| C12-CARET | `caret-shape` / `caret-animation` / `caret` | |
| C12-CONTROLS | `accent-color`, `appearance`, `field-sizing`, `resize` | |

### Phase 13 — Tables (audit §3.20)

| Id | Item | Status |
|---|---|---|
| C13-TFC | A real table formatting context: `display: table` family on any element, `rowspan`, automatic and `fixed` `table-layout` (replaces `TABLE-TFC-1`) | |
| C13-TABLE-PROPS | `caption-side`, `empty-cells`, `border-spacing` (separated borders), `vertical-align` on cells | |

### Phase 14 — Conditional rules, containment (audit §3.21)

| Id | Item | Status |
|---|---|---|
| C14-MEDIA | `@media` (cell-sized viewport, `prefers-color-scheme`, `prefers-reduced-motion`, `hover` / `pointer`, …) and `matchMedia` | |
| C14-SUPPORTS | `@supports` (feature queries against the dispatch table; `CSS.supports`) | |
| C14-CONTAINER | `@container`, `container-type` / `-name` / `container`, `cq*` units | |
| C14-CONTAIN | `contain`, `content-visibility`, `will-change` (stacking-context hint) | |

### Phase 15 — Transforms, filters, compositing, multi-column, anchor positioning (audit §3.23, §3.24)

| Id | Item | Status |
|---|---|---|
| C15-TRANSLATE | `translate` and `transform: translate()` (whole-cell offsets; other transforms documented N/A) | |
| C15-FILTER | `filter` color-matrix functions; `backdrop-filter` | |
| C15-BLEND | `mix-blend-mode`, `isolation` | |
| C15-CLIP-PATH | `clip-path: inset()` | |
| C15-COLUMNS | Multi-column layout (`columns`, `column-count` / `-width` / `-rule*` / `-span` / `-fill`) | |
| C15-ANCHOR | Anchor positioning (`anchor-name`, `position-anchor`, `position-area`, `@position-try`) | |

## Log

- 2026-10-03 — Program opened at Miska's request: "address all partials and missing but meaningful
  in a terminal", CSS completeness as 0.6.0 before routing (now 0.7.0). Built from the 307-row
  audit in `CSS-COVERAGE.md` (`ba585c7`).
- 2026-10-03 — Phase 0 done (C0-CONTRADICTIONS, C0-NOT-SHIPPED): the six `DIVERGENCES.md`
  statements of `CSS-COVERAGE.md` §6 corrected against the code; the four undocumented rdom
  extensions of §4 documented; DIVERGENCES §3 rewritten as the complete gap list, grouped by
  module with item ids; the decided exclusions recorded in §2 (the header above said §3; corrected —
  they are permanent divergences, not scheduled work); N/A rows grouped into one §1 entry;
  README roadmap and feature lists corrected. Found while verifying: `marker-side` has no item
  row (listed under C10-LIST-ITEM); the audit classes color alpha and `direction` N/A while
  C3-ALPHA and C5-WRITING schedule them — the program's items win.
- 2026-10-04 — Phase 1 gates. Architect: 1 blocking (style invalidation ignores `@scope` preludes and
  `SimpleSelector::Is` — silent `_ => false` wildcards), 17 non-blocking (per-element cascade
  allocations; `var()` cost; `@property` transitions re-cascade the subtree every frame and a stale
  `computed_prev`; registered-property validation order; escaped strings / dimensions / length cap
  through `var()`; importance within one block; `@import` cycle / base URL / depth; `:root` seeding vs
  layers; scope matching cost; two property registries; `ext.rs` and other files over the bar;
  `&` in `@scope` specificity to verify; `:is()` unparsed). API: 1 blocking (rdom-css README
  contradicts shipped `var()` / `@import`), 8 non-blocking (`@import` base URL; `extend_from_style_tags`
  loses `@scope` owner and loader; missing `rdom_tui` re-exports; `String` errors on
  `register_property`; stale DESIGN / DIVERGENCES / COVERAGE / CHANGELOG lines; API surface
  duplication). Decision: fix all as `C1G-*` items in two batches before Phase 2.
- 2026-10-04 — C1G-README: rdom-css README no longer limits `var()` to colors nor lists `@import`
  as unsupported; DIVERGENCES no longer schedules `@property`; COVERAGE `@import` is *Partial*
  (conditions ignored) and the §1 counts are recounted (192 Partial / Missing, 124 undocumented at
  audit time); `define_var` rustdoc and the C1-ALL / `TuiStyle::pending` CHANGELOG bullets corrected.
- 2026-10-04 — C1G-INVALIDATION: one walker (`rdom-tui/src/style/selector_walk.rs`) over rule
  selectors and `@scope` starts / ends, recursing into `:not()` / `:is()` / `:where()`, feeds the
  sibling-combinator check, the sibling triggers and the validity check; unknown `SimpleSelector`
  kinds `debug_assert!`. The gate's example (inserting `.a` before `.b`) was already correct — a
  child-list change marks every sibling unconditionally — and is kept as a regression test; the
  real gaps were attribute / class / state changes (prelude start and limit, nested `&`).
  Specificity (rdom-core, exhaustive) and the rule index (keys only, `:is()` falls to the universal
  bucket) needed no change.
- 2026-10-04 — C1G-BLOCK-IMPORTANCE: rdom-css collects a block's declarations
  (`declarations::DeclarationRun`) and applies normal ones first, then important ones, each in
  order — Cascade 4 §6.4 within one block, for typed fields, shorthands, `var()` declarations
  (`TuiStyle::pending`) and custom properties alike, without a per-property importance check.
- 2026-10-04 — C1G-REGISTERED-ORDER: `var::resolve_custom_properties_with` takes a computed-value
  step; the cascade passes the registered-syntax check (`Registry::computed_value`) so a dependent
  substitutes the validated value, at the element and at the sheet level (`Registry::seed_root`
  replaces `settle_root`; `validate_declared` is gone).
- 2026-10-04 — C1G-VAR-TOKENS: CSSOM string / identifier serializers in `rdom_core::css_syntax`,
  used by `render_value` (custom-property storage); a real `<dimension-token>` (`Token::Dimension`)
  so adjacency is kept — `var(--n)fr` and `1 fr` are a number and an ident, rejected by the unit
  grammars; a 65 536-token substitution limit (`var::MAX_SUBSTITUTED_TOKENS`). No existing
  property test depended on `<number> <unit>` with whitespace; the tokenizer's own tests for
  `1em` / `1e` / `200ms` now expect a dimension.
- 2026-10-04 — C1G-TRANSITION-PREV: after the per-frame registered-property re-cascade, the frame
  pipeline copies `computed` (and the pseudo slots) into `computed_prev` for the restyled subtrees
  (`animation::settle_restyled`), so the next style change diffs against the animated values.
  The per-frame subtree re-cascade itself (its cost) remains a non-blocking architect finding.
- 2026-10-04 — C1G-IMPORT-EDGES: `ImportLoader::load_from(url, base) -> LoadedSheet` (default:
  `load`), `parse_with_loader_at` (the root's URL on the cycle stack and as the first base),
  cycle identity = the loader's resolved URL (the loader canonicalises), `MAX_IMPORT_DEPTH` = 16
  with `ImportTooDeep`; `Scope::owner` carried by `append`, so `extend_from_style_tags` (which now
  sets each `<style>` sheet's owner) keeps prelude-less `@scope` roots;
  `extend_from_style_tags_with_loader`. The `App`'s `<style>` sheets have no URL, so their imports
  get no base (decided: an inline sheet's base would be the document URL, which rdom has none of).
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): A prelude-less `@scope`'s root is the
  `<style>` element's parent (CSS Cascade 6 §2.5.1).
- 2026-10-04 — C1G-ROOT-SEED: the `:root` mirror is computed after the parse in cascade order
  (`rdom-css/src/root_vars.rs`) instead of eagerly per block; DESIGN's "published twice" section
  rewritten. Found: `:root` matches the tree's root node, which has no computed style, so the
  mirror is the only path by which `:root` custom properties reach elements — the gate's "the
  cascaded root `--c` is blue" did not hold; every element saw the mirror's `red`. Cross-sheet
  layer / importance precedence of the seed stays last-sheet-wins (accepted, DESIGN).
- 2026-10-04 — C1G-IS-PARSE (lands C11-IS): `:is(<forgiving-selector-list>)` parsed into
  `SimpleSelector::Is`; an invalid argument is skipped to the next top-level `,` / `)`, an empty
  `:is()` matches nothing (`names_only_scope` no longer treats an empty list as `:scope`).
  `:where()` stays unforgiving (DIVERGENCES; Selectors 4 §4.4 makes it forgiving too — C11 work).
- 2026-10-04 — C1G-SCOPE-AMP-SPEC: verified against the Editor's Drafts — Cascade 6 scoped style
  rules: "The `&` selector is defined to behave as `:where(:scope)`" (":scope has a specificity of
  (0,1,0), whereas & has a specificity of 0"); CSS Nesting 1 §3.3.1: "`&` behaves like
  `:where(:scope)` in @scope rules". The current behaviour is right; no code change, the cascade
  test now cites the text and pins it against an id `<scope-start>`.
- 2026-10-04 — C1G-CASCADE-ALLOC: `ladder::Rollback` allocates its memo on the first `revert` /
  `revert-layer` read; `apply_cascade_ladder` returns at once with no declarations; a pseudo-element
  with no matched rule and no legacy content returns before building a style; `matching::Scratch`
  (candidates, matches, sorted rules, layer ranks, `Plan`) is reused for a pass. Test-only work
  counters (`ladder::probe`): one `div` with one rule now walks 2 ladders (element + UA
  `*::selection`), was 5, and allocates no rollback memo, was 5. `walk.rs` (666 after the change)
  split into `walk` / `matching` / `pseudo`.
- 2026-10-04 — C1G-REGISTERED-CLONES: `cascade::PropertyRegistry` (was `Registry`) is the one
  registry; `FramePrelude::sheets_changed` rebuilds it, the frame hands it to the cascade
  (`cascade_all_with` / `cascade_subtrees_all_with`) and to `AnimationRegistry` (an `Rc`; the
  animation side derives the interpolation kind per lookup). The stateless `CascadeExt` methods still
  build one per call. Counter test: restyles and transition frames build none, a sheet change one
  (was 5 for the same sequence). `settle_undeclared` compares `Option<&String>` before cloning.
- 2026-10-04 — C1G-VAR-COST: `rdom_style::CustomValue` (text + `Arc<[Token]>`, tokenized once in
  `CustomValue::new` or made from substituted tokens) is the value type of `VarMap`,
  `CustomDeclaration`, the sheet-level vars, the registry's initial values and the animated values.
  The cascade applies `TuiStyle::substituted_pending` after each rule's own block
  (`ladder::Declarations::rule_blocks`) instead of cloning the block per element. `Arc`, not `Rc`, so
  `TuiStyle` stays `Send + Sync`. Counter test (`custom_value::probe`): 50 substitutions of three
  `var()`s tokenized 150 values before, 0 after; theme-token cascade test pins correctness.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): Public shape: `VarMap` is `Rc<HashMap<String,
  CustomValue>>`; `CustomDeclaration::value`, `Stylesheet::vars()`, `resolve_tui_color`'s map,
  `lookup_in` / `substitute` (the lookup returns `Result<CustomValue, SubstitutionError>`),
  `resolve_custom_properties` (the computed-value step maps `Option<CustomValue>`) take or give
  `CustomValue`, and `ContentContext` is implemented for `HashMap<String, CustomValue>`.
  `CustomValue` derefs to `str` (`as_str()`), builds with `CustomValue::new(text)` or `From<&str>`;
  `define_var` / `define_var_mut` take `impl Into<CustomValue>`, so `&str` callers are unchanged.
- 2026-10-04 — C1G-PROPERTY-RESTYLE: `advance_custom` pushes a restyle only when `write` changed
  the animated value (none inside the delay). `cascade::restyle_vars` walks the restyle roots in
  `walk::Mode::Restyle`: each element's boxes reload their matches from `TuiExt::matched`
  (`matching::MatchedRules`, stamped with the sheet set's registry `Rc`; recorded by every cascade,
  reused without allocation when unchanged), and an element whose computed style is unchanged keeps
  its subtree (`CounterState::exact` replays the kept subtree's counter ops). The counter predicate
  counts a `var()` declaration only for the counter properties, `content` and `all` (the UA sheet
  uses counters, so the counter path is the common one). Not done literally: the restyle re-runs
  each changed element's whole ladder rather than only the `var()`-using properties — inherited
  properties of descendants depend on it, and the ladder without matching is the cheap part.
  Counter tests: a `--theme` transition frame over 7 elements matched 35 boxes before, 0 after; no
  restyle inside the delay. C1G-TRANSITION-PREV's test still holds. `tui_ext_size_tripwire` raised
  432 → 440 (the `matched` pointer).
- 2026-10-04 — C1G-SCOPE-COST: `scope::ScopeMemo` (in the pass's `matching::Scratch`) memoizes per
  (sheet, scope, node) whether the node is a scoping root and its in-scope roots, nearest first —
  built from the parent's (`roots_of`), so a node costs one root test and one limit test per root
  above it; `match_rule` tries the rule's selector once per root. Counter test (`scope::probe`): 41
  elements, a limit, a nested `@scope`, three rules — 13 152 matches before, 204 after.
  `RuleIndex` files a subject keyed only inside `:is()` under each argument's key (`compound_keys`),
  universal only when an argument has none.
- 2026-10-04 — C1G-REEXPORTS: `PropertyRegistration`, `PropertySyntax`, `LayerId`, `StyleSelector`,
  `RuleContext`, `CustomValue` re-exported at the `rdom_tui` root and in `rdom_tui::style`;
  `ImportLoader`, `LoadedSheet` (rdom-css) at the root. `App::set_import_loader` /
  `register_property` name them through `crate::`; a doctest on `register_property` drives both
  with `rdom_tui` paths only.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): `App::register_property`'s doc example uses
  `rdom_tui` paths only, so the re-exports are exercised by a doctest.
- 2026-10-04 — C1G-TYPED-ERRORS: `rdom_style::{RegisterPropertyError, PropertySyntaxError}`
  (`#[non_exhaustive]`, `Display` keeps the old messages, so `@property` warnings read the same);
  `PropertyRegistration::new`, `PropertySyntax::parse`, `App::register_property` return them. A test
  per variant (rdom-style) and for `AlreadyRegistered` through the `App`.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): The variants, as the 0.5 → Unreleased
  CHANGELOG listed them: `RegisterPropertyError::{InvalidName,
  InvalidSyntax(PropertySyntaxError::{UnsupportedComponent, InvalidComponent}), MissingInitialValue,
  InitialValueMismatch, NotComputationallyIndependent}` — the web API's `SyntaxError` — and
  `AlreadyRegistered`, its `InvalidModificationError` (`is_invalid_modification()`).
- 2026-10-04 — C1G-API-SURFACE: `Stylesheet::add_rule_in_layer` removed (`add_style_rule` with a
  `RuleContext` is the one way; unreleased, so only the C1-LAYER bullet changes). Decided: the
  `var()` hooks go to a documented `rdom_style::backend` module rather than `#[doc(hidden)]` — a
  sibling backend (CLAUDE.md "Substrate First, Backend Second") needs them, so they are public with
  a stated contract; `var` is private, `set_parsed` / `set_unset` leave `property_dispatch`'s public
  surface. `register_property` / `registered_properties` moved to `stylesheet/registrations.rs`.
- 2026-10-04 — C1G-SPLITS: `ext.rs` (765 lines, ~642 production) → `ext/mod.rs` (`TuiExt`, its
  inline-style accessors and cloning steps, `TypeaheadState`), `ext/presentation.rs`
  (`PresentationStyle`, `StyleSlot`, `PseudoSlot`, the `presentation_for*` accessors),
  `ext/layout_cache.rs` (`PseudoLayout`, `StaticPosition`, `MarginChainMemo`, `AnonymousIfc`),
  `ext/tests.rs`. `runtime/app/mod.rs` (603, touched by C1G-REGISTERED-CLONES) → `App::handle_event`
  and `note_route` in `app/input.rs`. Checked against the bar and left whole: `cascade/walk.rs`
  (509 — split in C1G-CASCADE-ALLOC; what is left is the tree recursion and the element ladder,
  one concern), `cascade/apply.rs` (492 — the per-property applicators, one table-shaped concern),
  rdom-css `block.rs` (498 — the declaration-block parser). Over the bar but outside this batch's
  edits: `rdom-core/src/query_selector.rs` (519 production of 1 388) and
  `rdom-tui/src/style/dirty_tracker.rs` (561 of 1 145) are under it once their inline tests are
  excluded.
- 2026-10-04 — C1G-INTEGRATION: `rdom-tui/tests/integration/css_phase1.rs` cascades the gate's
  example (layers `base, theme`, `:root` tokens, `padding: var(--p)` with `--p: 1 2`, `& > p`, nested
  `@layer base { border-color: var(--c) }`) plus `revert-layer` (unlayered → the layers' yellow),
  `@scope (.card) to (.slot)`, `all: revert` (UA `display: block`, inherited color, not bold), `.\31 0`
  and an `@import 'tokens.css' layer(theme)` through a closure loader feeding `var(--w)`. It passed
  on first run — it pins the batch's features working together; it guards against vacuous passes by
  asserting each expected value differs from the initial one. Second batch of Phase 1 gate fixes
  complete.
- 2026-10-05 — Phase 1 closed: 11 items + 20 gate fixes. The gate fixes' re-review is folded into the
  Phase 2 gate (range from the first C1G commit).
- 2026-10-04 — C2-PERCENT: every length-bearing property parses through one leaf
  (`rdom-style/src/parse/values/numeric.rs::length_percentage`) and one component splitter, so
  later units and math functions reach all of them at once. `MinSize` gains `Calc`, `max-*` become
  `MaxSize` (breaking). Found: intrinsic sizing resolved percentage padding against the box's own
  width / its cross budget (fixed, test). Also closes C8-INSETS and the `%` / `calc()` half of
  C5-MINMAX-SIZE. Split: `layout_pass/mod.rs` (852) → `tree`, `auto_height`, `scroll_extent`,
  `gutter`; `layout_pass/intrinsic.rs` (621) → `intrinsic/{mod,inline}.rs` (the duplicated
  wrap-row measurement is one `wrapped_rows`).
- 2026-10-04 — C2-NUMBER: `Size::Flex(f32)` and `flex_shrink: f32` (breaking). Grow and shrink
  distribute in `f64` with the rolling floor kept (integer weights lay out exactly as before), and
  §9.7 step 4.b applies: factors summing below one share only that fraction of the free space /
  overflow. The C6 flex longhands build on `numeric::number` / `parse_flex_factor`.
- 2026-10-04 — C2-MINMAX: `CalcExpr` gains `Function { func: MathFunction, args }` and `None` (an
  absent `clamp()` bound) (it was briefly made `#[non_exhaustive]`; reverted in C2-VIEWPORT — DESIGN
  classes it closed data); the parser recognizes a math function at
  top level and nested, so every property on the shared leaf takes them. NaN propagates through
  `min` / `max` and resolves to 0 at the top (Values 4 §10.9). `calc.rs` became `calc/{mod,tests}.rs`.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): `clamp()`'s minimum wins when it exceeds the
  maximum, and a lone math function serializes as written (`min(50%, 30)`).
- 2026-10-04 — C2-TRIG: math expressions are type-checked (`CalcExpr::kind` → `CalcKind::{Number,
  Length, Angle}`, Values 4 §10.9) with rdom's number-is-a-cell relaxation (a number unifies with a
  length); `parse_calc` rejects an ill-typed tree and each property checks the kind it takes. Newly
  rejected as ill-typed: a product of two lengths (`calc(50% * 10%)`), a division by a length.
  Angles are radians inside the evaluator. `<number>` properties (`opacity`, flex factors) take math
  functions of type `<number>`. Constants are numbers once parsed (`pi` serializes as its value).
  `calc/mod.rs` split into `functions.rs` (the functions and their evaluation) and `types.rs`.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): The result becomes whole cells where it
  becomes a length, a NaN is 0 and an infinity clamps (CSS Values 4 §10.9); an inverse trigonometric
  result is an angle, which a length rejects.
- 2026-10-04 — C2-CH: `CalcExpr::Dimension { value, unit: CalcUnit }` (`calc/units.rs`) is the leaf for
  every unit Phase 2 adds; `ch` folds to cells outside a percent-bearing expression. A registered
  `<length>` (`@property`) takes unit dimensions too.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): `ch` is ASCII case-insensitive, and a
  fractional `ch` rounds where the value becomes a length.
- 2026-10-04 — C2-LH: `lh` / `rlh` are one row each (`CalcUnit::{Lh, Rlh}`), the fixed line height.
  Partial: they must follow the element's / root's computed `line-height` once C9-LINE-HEIGHT lands —
  then `lh` needs the cascade's value, so it becomes a context unit like the viewport units.
- 2026-10-04 — C2-VIEWPORT: decided — resolve viewport units at computed-value time (Values 4
  §6.1.2), not in layout: the cascade (`Sheets` carries the `Viewport`) calls
  `ComputedStyle::resolve_viewport_units` per element and pseudo-element, which folds an expression
  left without a percentage to cells, so no layout site needs the viewport. Resize already sets
  `Redraw::Cascade` (whole-tree cascade); the `App` also records the size it last cascaded at and
  cascades the whole tree at any other size (a backend resize with no event). `CascadeExt` gains
  `cascade_all_in` / `cascade_subtrees_all_in`; the old forms use a 0 × 0 viewport (documented).
  Also: `CalcExpr`'s `#[non_exhaustive]` from C2-MINMAX reverted (DESIGN classes it closed data);
  DESIGN's lists name the new value types.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): `ResolveCtx::viewport` is `Option<Viewport>`,
  `None` in layout: a viewport unit evaluated there is a debug assertion (a computed-style field the
  cascade missed); a percentage beside a viewport unit stays for layout.
- 2026-10-04 — C2-ANGLE: `CalcUnit::{Deg, Grad, Rad, Turn}` (type `<angle>`, radians inside the
  evaluator); `parse::values::parse_angle` returns degrees for Phase 3's hues; `@property` takes
  `<angle>` (was rejected) and a registered angle interpolates in degrees.
- 2026-10-04 — C2-RATIO: `AspectRatio { numerator: f32, denominator: f32, auto }` (breaking); the
  style holds `Option<AspectRatio>` with `auto` alone as `None`, applied like any value. Layout uses
  `value()` (None for a degenerate ratio) and, for `auto && <ratio>`, takes the main axis's padding
  and border off before the ratio and adds the cross axis's back (content box, Sizing 4 §5.1). The
  property-dispatch test "70000 / 1 is rejected" became "kept as written" (terms are numbers now).
- 2026-10-04 — C2-ATTR: `attr()` is an arbitrary substitution function beside `var()`
  (`rdom-style/src/attr.rs`; Values 5 §8.7 in the current Editor's Draft — the brief's §7.7 is an
  older numbering): any declaration holding one goes to `TuiStyle::pending`
  (`var::contains_substitution`), and the cascade substitutes it with the element's attributes —
  the originating element's for a pseudo-element — through `substituted_pending_on` /
  `resolve_custom_properties_on` (so `--w: attr(data-w type(<length>))` resolves where declared).
  `parse_content` no longer parses `attr()`: CSS `content: attr(x)` substitutes as a string first;
  `Content::Attr` stays for Rust-built styles (the UA sheet), as `TuiColor::Var` did in C1-VAR-ANY.
  Attribute changes already re-cascade the element (DirtyTracker), so values stay live (test).
  Decided: attribute values are not searched for substitution functions (documented).
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): `attr()` types: `type(<syntax>)`, `number`, a
  CSS unit (`%`, `ch`, `deg`, …), `raw-string` / no type (a string). Fallback rules (CSS Values 5):
  untyped and missing gives the empty string; typed, missing and without a fallback is invalid at
  computed-value time. The grammar is checked at parse time — an unknown `<attr-type>` makes the
  declaration invalid, so `width: 10; width: attr(x bogus)` keeps 10 — and `content: attr(x)` goes
  through the same substitution. `CustomValue::has_substitution` was `has_var` before
  C2G-SUBSTITUTION-ERRORS.
- 2026-10-04 — Phase 2 items done (C2-LH partial — revisit with C9-LINE-HEIGHT). Unit decision:
  absolute (`px`, `cm`, …) and font-relative (`em`, `rem`, `ex`, …) units stay N/A as
  `CSS-COVERAGE.md` classes them — no terminal mapping; recorded in DIVERGENCES §1 "Length units".
  Phase 2 gates (architect + API, with the C1G re-review) are next.
- 2026-10-05 — Phase 2 gates (with the C1G re-review: all 20 fixed at the root). Architect: 3 blocking —
  flex factors summing to 1 lose a cell (f32 sum below 1.0 trips step 4.b); counter replay drops
  `::before` / `::after` ops on kept subtrees (cascade and restyle paths); no calc nesting cap (attr
  data can overflow the stack). API: 1 blocking — cascade forms without a viewport resolve `vw`
  against 0x0, including the README's DirtyTracker pattern. ~35 non-blocking (IEEE division,
  `-infinity` negation overflow, zero-basis folding in number/angle math, percentages in `CalcKind`,
  math in integer / registered properties, viewport field list hand-kept, `max-width: none`
  unrepresentable, `flex` shorthand ignores shrink, parse-time `attr()` validation, `:root` `attr()`,
  u16 overflows, unvalidated `AspectRatio` / `Flex`, restyle walk cost, renames, re-exports, README
  0.2.0 history edit, changelog paths and hints). Decision: fix all as `C2G-*` items, two batches.
- 2026-10-05 — C2G-FLEX-SUM: §9.7 step 4.b's "sum below one" and the rolling floors of grow and
  shrink use one relative tolerance (`FACTOR_TOLERANCE`, four `f32` epsilons) instead of an exact
  `f64` comparison and a fixed `1e-9`. Decided against summing in `f32`: it only moves the rounding
  (`10 × 0.1` sums to 1.0000001 in `f32`) and leaves the floors seeing `71.9999999`, which also lost
  a cell for a genuine 0.9 sum. Tests: 0.1 / 0.2 / 0.7 fills 80 (grow and shrink), 0.2 + 0.7 takes
  exactly 72, a 0.1 item frozen by `max-width` leaves 0.9 to share 72.
- 2026-10-05 — C2G-COUNTER-PSEUDO: `CounterState::replay_element` (with `StoredOps`, the `Rc`s of
  the element's, `::before`'s and `::after`'s computed styles) replays a kept element in tree order —
  element, `::before`, children, `::after` — and is the one replay used between `cascade_subtrees`
  roots, for a restyle's kept element (its pseudos; its own ops were applied computing it) and for
  its kept children (`replay_children`, was `replay_subtree`). Tests: a class change on the third of
  three `h2::before`-numbered headings reads "3. " (was "1. "); `restyle_vars` keeping a root `h2` and
  a `div` of two reads "4. " for the next; a kept `div`'s `::after` counts after its children; a kept
  `div::before { counter-reset }` scopes its children (green before the fix too — an order guard).
- 2026-10-05 — C2G-CALC-DEPTH: the calc parser caps nesting (`MAX_CALC_NESTING` = 32 math
  functions / parentheses; the parser recursed ~4 frames per level) and tree depth
  (`MAX_CALC_DEPTH` = 256, tracked as nodes are built, so an over-deep chain is rejected before it
  exists and nothing — type check, evaluation, `absolutize`, serialization, `Drop` — ever walks one).
  Chosen over an iterative `Drop` / balanced trees: one bound covers every walker, `-` and `/` do not
  re-associate, and 256 operands is far past hand-written CSS. A run of unary `+` is a loop. Red: a
  20 000-level `attr()` value aborted the test process (stack overflow); green: invalid, fallback.
- 2026-10-05 — C2G-VIEWPORT-DOC: the viewport is the document's. rdom-core gains document data
  (`Dom::document_data` / `set_document_data` / …, one value per Rust type, `document_data.rs`) — the
  substrate's renderer-free hook for per-document backend state, as `Ext` is per node; the root is a
  fragment with no `Ext`, so no node could hold it. rdom-tui stores the `Viewport` there
  (`style/cascade/viewport.rs`); every cascade form and `restyle_vars` read it,
  `CascadeExt::set_viewport` / `viewport` set and read it, `layout_dom(area)` records its area, the
  `App` sets its terminal's size each frame (its `cascaded_viewport` still decides the full
  re-cascade). Decided: `cascade_all_in` / `cascade_subtrees_all_in` removed (unreleased) — one way
  to give the size. `Viewport` joins the prelude. Phase 14's `@media` reads the same value.
- 2026-10-05 — C2G-CALC-SEMANTICS: IEEE division (parse-time literal rejection and the runtime
  zero-gives-0 both gone); `calc::to_cells` clamps a top-level result to `±i32::MAX` (symmetric;
  `cells_i32` and `CalcExpr::resolve` share it) and `right` / `bottom` negate with `saturating_neg`;
  `CalcKind::Percent` with `kind()` (percent joins a length or number as a length) and
  `kind_as_number()` (percent is a number: `opacity`); `numeric::number_math` rejects `<number>` math
  holding a percentage or a viewport unit, `parse_angle` likewise and clamps to
  `MAX_ANGLE_DEGREES` (NaN 0); `numeric::integer` (round half toward +∞) feeds `z-index` (clamped to
  `i16`) and registered `<integer>`; registered `<number>` / `<percentage>` take math. Decided:
  viewport units in `<number>` / `<angle>` math are rejected, not kept symbolic — every number
  property stores a resolved `f32`, and none of them is a plausible place for `vw`; the `<length> /
  <length>` ban is kept (one length unit: the ratio is the numbers' division). Both in DIVERGENCES.
  `parse_unsigned` (public, unused) deleted. Changed expectations: `calc(10 / 0)` was rejected
  (now `u16::MAX` cells), `10 / 0` resolved to 0 (now `i32::MAX`), `opacity: calc(50%)` was invalid
  (now 0.5).
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): An infinite `<integer>` math result clamps to
  the property's range.
- 2026-10-05 — C2G-VIEWPORT-FIELDS: `ResolveCtx::viewport` is `Option<Viewport>` (unreleased field),
  `None` from `ResolveCtx::new` — every layout resolve — and `CalcUnit::canonical` debug-asserts a
  viewport unit never meets `None` (0 cells in release). The hand-kept field list in
  `ComputedStyle::resolve_viewport_units` stays (one place, typed per field), guarded by a test that
  sets every property of `property_dispatch::property_names()` to `10vw` (or `10vw 10vw`), cascades
  at 80 × 20 and requires no `Viewport(` in the computed style's `Debug` and a clean layout — so a
  length property added later is covered without editing the test. Checked red: dropping `gap` from
  the list fails it.
- 2026-10-05 — C2G-MAX-NONE (completes C5-MINMAX-SIZE): `TuiStyle::max_width` / `max_height` are
  `Option<Value<Option<MaxSize>>>` like `aspect_ratio`, applied with `value!` (the declared `Option` is
  the computed one; `ComputedStyle` keeps `Option<MaxSize>`, `None` = `none`); `parse_max_size`
  takes `none`, serialization writes it back. `set_max_width` / `set_max_height` take
  `impl Into<Option<MaxSize>>` and always declare (`None` is `none`; removal through the CSSOM) —
  decided over a double `Option`. The C2-PERCENT changelog bullets are rewritten to the final shape
  with migration hints from 0.5.0. COVERAGE keeps the row *Partial* (intrinsic keywords, C5-INTRINSIC).
- 2026-10-05 — C2G-FLEX-SHORTHAND: `parse_flex_shorthand` → `FlexShorthand { grow, shrink, basis }`
  with the Flexbox §7.2 grammar (`none`; `<grow> <shrink>? || <basis>` in either order; omitted grow /
  shrink 1, omitted basis 0; a number is a factor unless two factors precede it). The dispatch writes
  grow → `width` / `height` as before, shrink → `flex_shrink` (was 0 for any zero grow: `flex: 0 1
  auto` overflowed), basis → the new `TuiStyle` / `ComputedStyle::flex_basis` (`FlexBasis`, mask bit
  45, cascaded, viewport units absolutized, in `layout_differs`) — stored, not laid out: C6-FLEX-LONGHANDS
  is now *partial* with that gap. `flex` serializes as `<grow> <shrink> <basis>`. DIVERGENCES' flex entry
  rewritten (the stale `Size::Flex(1)` / `parse/values.rs`); the C2-NUMBER changelog example now says
  what `flex: 1.5 0.5 0%` sets. The `initial`-keyword apply test perturbs `flex_basis` through `flex`.
- 2026-10-05 — C2G-ATTR-PARSE: `attr::valid_args` parses the head at parse time (an unknown unit, a
  bad `type()` syntax, a non-identifier name are invalid; a head holding `var()` / `attr()` is checked
  when substituted), so `width: 10; width: attr(x bogus)` keeps 10. `PendingDeclaration` stores
  `attr::AttrHeads` (the heads, parsed once, keyed by the `attr(` token's index; `substitute_at`
  carries the offset into heads and fallbacks), so a substitution parses none — probe: 50
  substitutions parsed 50 heads before, 0 after. `AttrLookup` returns a borrowed `&str` (no `String`
  per lookup) and `PropertySyntax::matches_tokens` drops the second tokenization of a `type()` value.
  `merge_root_vars` takes the dom and reads the root element's attributes for the `:root` mirror
  (a fragment root has none — DIVERGENCES). Found: an element root already got the right value
  through its own cascade; only the mirror (what the root's parent seeds) read no attribute. The
  C2-ATTR test's `furlong` case moved: it is a parse error now, not a fallback.
- 2026-10-05 — C2G-LAYOUT-SAFETY: `Padding::horizontal` / `vertical` (saturating) replace the
  hand-summed sides at every layout site — the four named plus two found by the test
  (`geometry::compute_content_area_collapsed`, the intrinsic child cross budget); `padding: 0 40000`
  panicked in `intrinsic/inline.rs`, then `geometry.rs`. `AspectRatio`'s fields are private
  (accessors; `new` the only constructor; DESIGN lists it as sealed, with `TuiExt`). `Size::validated`
  / `valid_flex_factor` keep Rust-built flex factors in `<number [0,∞]>` at the builder and node
  setters (a non-positive grow is `Size::Auto`, the parser's `flex: 0`). Flex items' `min-height` /
  `max-height` percentages use `nearest_block_ancestor_height_is_definite` (block flow's test) for
  the container's height: an `auto`-height column (main) or row (cross) resolves them as 0 / `none`
  (was: 25% of the available height). `flex/main_axis.rs` is 545 lines, under the split bar.
  Phase 2 gate batch A (C2G-FLEX-SUM … C2G-LAYOUT-SAFETY) complete.
- 2026-10-05 — C2G-RESTYLE-WALK: partial walks moved to `style/cascade/subtrees.rs`. Roots are reduced
  to the outermost (a root inside another was cascaded twice when it came first) and ordered by tree
  order. `TuiExt` gains `tree_has_counters` (bottom-up: an op or a `counter()` read in the subtree,
  pseudo-elements included) and `reads_counters`; a root whose subtree takes no part in counters is
  cascaded alone (cascaded again through the ordered walk if it gained one), and the ordered walk
  skips subtrees without counters, so a counter-free leaf restyle visits 1 node where it visited
  10 052 (probe test, 50 × 100 list items). Decided over cached per-node counter state: the snapshot
  per element costs O(N × instances) memory for the one case (a counter-using root) the scoping does
  not already make cheap. `CounterState` notes a changed op (element, `::before` before the children,
  `::after`); the walk then goes on past the last root and restyles (`Mode::Restyle`) every later
  subtree that takes part, recomputing an element that reads a counter even when its style is
  unchanged — so a transitioning `counter-increment: c var(--step)` reaches a later sibling's
  `counter(c)` (was the cascaded end value), and a `cascade_subtrees` class change renumbers later
  headings. `restyle_vars` returns every root it restyled, and the App settles those. The walks step
  through children by sibling links (no child `Vec` per node).
- 2026-10-05 — C2G-STATELESS-REGISTRY: the stateless `CascadeExt` forms take the registry from
  `registered::document_registry` — document data holding the last sheet set's registry, keyed by
  each sheet's `Stylesheet::version()` (new in rdom-style: a process-unique stamp from one atomic
  counter, renewed by every `&mut` / builder mutation, fresh on `Clone`, so equal keys are the same
  sheets unchanged; decided over pointer identity, which a dropped-and-reallocated sheet would
  alias). `Sheets::new` takes the registry (no `Option`). Test: two `cascade`s and a
  `cascade_subtrees` with one sheet build 1 registry (was 3) and keep the element's match record
  (`Rc::ptr_eq`); a mutated sheet and another list each build one.
- 2026-10-05 — C2G-REGISTERED-ABSOLUTE: `PropertySyntax::computed(value, viewport)`
  (`rdom-style/src/registration/computed.rs`; `registration.rs` became `registration/mod.rs`, 556 lines
  plus this) — the first alternative the value matches decides; a `<length>` (or list item) is
  whole cells (viewport units against the viewport, `ch` / `lh` / math folded), a
  `<length-percentage>` is cells, a percentage, or `calc(<cells> ± <p>%)` (number before
  percentage, Values 4 §10.10.1) when linear (`CalcExpr::linear_parts`), else its math with the
  lengths folded. The registry applies it in `computed_value` (all registry entry points now take the
  viewport); an initial value is computed once at build unless it holds a viewport unit (decided:
  viewport units are computationally independent — the viewport is global information CSS cannot
  change — so `initial-value: 10vw` stays valid and computes per use). The transition engine gains
  `Kind::LengthPercentage` (cells and percentage interpolate apart; a non-linear value is discrete).
  Tests: `10vw` → `8`, `calc(2ch + 50%)` → `calc(2 + 50%)` inherited; `10vw` → `50vw` is 24 cells
  half way (was: no transition); `10` → `calc(20 + 50%)` is `calc(15 + 25%)` half way.
- 2026-10-05 — C2G-CONTENT-ATTR: `Content::Attr`, `ContentContext::attr` and `resolve_content_on`'s
  attribute lookup are deleted; the five UA rules that read an attribute (`input` / `textarea`
  placeholder, `input[type=button|submit|reset]` value, `input[type=image]` alt, `optgroup` label) are
  CSS declarations built with `ua::css` (`property_dispatch::set`), so they go through the `attr()`
  substitution path like an author's `content: attr(x)`. Breaking for rdom-style (both were in 0.5.0):
  CHANGELOG migration hint. No test expectation changed; the one cascade test that built
  `Content::Attr` declares `content: attr(data-status)` instead. UA-dependent tests (placeholder,
  button labels, optgroup) green.
- 2026-10-05 — C2G-SUBSTITUTION-ERRORS: `SubstitutionError { Undefined, Cycle, InvalidAttr, TooLong,
  Syntax }` (`#[non_exhaustive]`, `Display` + `Error`) from `backend::substitute` and the lookup
  (`backend::Lookup`); `resolve_custom_properties` returns the declared properties it invalidated, with
  why (a cycle's members report `Cycle(self)`, a dependent without a fallback the property it read).
  `SubstitutionContext<'a, 'c> { attrs, computed }` replaces `substitute_with` /
  `resolve_custom_properties_with` / `_on` / `substituted_pending_on`; two lifetimes because an
  `AttrLookup<'a>` (`&'a dyn Fn(&str) -> Option<&'a str>`) is invariant and cannot shrink to a
  caller-local computed-value closure. Renames: `has_var` → `has_substitution` (`PendingDeclaration`,
  `CustomValue`), `CalcExpr::None` → `NoBound`. `var.rs` (639 lines) split into `var/{mod,pending,
  resolve,tests}.rs`. All unreleased: CHANGELOG bullets rewritten to the final names. Red: the new
  test did not compile (no error type); green: each failure kind reported.
- 2026-10-05 — C2G-CELLS-CONVERSIONS: `Size::cells` / `cells_u16` and `Length::cells` (rdom-style
  `layout/sizing.rs`) replace the size-to-cells matches in `flex/main_axis.rs` (natural size, auto-min
  cap), `flex/cross.rs`, `block/width.rs` (`resolve_size_to_cells` deleted), `block/height.rs` (a fifth
  copy, found) and `positioning.rs::resolve_size_axis`, and the inset matches in `positioning.rs`
  (`length_to_cells` / `length_to_cells_opt` deleted, `resolve_length_offset`) and `sticky.rs`.
  `calc::round_half_to_even` (public since 0.5.0) is gone for `f64::round_ties_even` — Breaking,
  CHANGELOG hint. No behaviour change: the layout and calc suites pass unchanged; new unit test of the
  conversions.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): They replaced four hand copies of the size
  conversion and three of the inset one; a size is an extent clamped to `0..=u16::MAX`, like
  `MinSize::cells` / `MaxSize::cells`, while an inset is signed.
- 2026-10-05 — C2G-FLEX-SPLIT: `flex/main_axis.rs` (525 lines after C2G-CELLS-CONVERSIONS) keeps the
  §9.2 gathering (`ChildMain`, `MainNatural`, `collect_main_axis_items`, 221 lines); the §9.7
  distribution — `MainAxisBudget`, `resolve_flexible_lengths`, `FACTOR_TOLERANCE`, the grow / shrink
  freeze loops and the §4.5 auto-min floor — moves verbatim to `flex/distribute.rs` (314 lines). No
  behaviour change; flex suites unchanged.
- 2026-10-05 — C2G-REEXPORT-CALC: `pub use rdom_style::calc` in `rdom_tui` (the module, so `CalcExpr`,
  `ResolveCtx`, `to_cells`, … are all reachable; `Viewport` stays at the root too) and
  `MinSize::percent` / `MaxSize::percent` (`Calc(Percent(p))`, the parser's form). Red: the integration
  test did not compile (no `rdom_tui::calc`, no `percent`); green: a `set_max_width(MaxSize::percent(50.0))`
  child of an 80-column box is 40 wide.
- 2026-10-05 — C2G-DOCS: README's 0.2.0 bullet restored to its released text ("`calc()` value system",
  from before `4252faf`); the math-function text moves to a new "Unreleased (0.6.0, in progress)"
  section. CHANGELOG: the C2-ATTR bullet names the final `backend::` / `SubstitutionContext` API
  (done with C2G-SUBSTITUTION-ERRORS); `parse_content` no longer parsing `attr()` is a "Breaking —
  rdom-style" bullet with a migration; the `AspectRatio` hint covers `f32` terms, the accessors, zero
  terms and `value()` vs `as_f32()`; the `MaxSize` hint (checked against C2G-MAX-NONE's shape) names
  `MaxSize::percent`. The `aspect_ratio` builder doc no longer claims a panic. CSS-COVERAGE §1:
  recounted from the §3 tables (82 / 35 / 140 / 50 = 307 — unchanged; cross-references not counted),
  the headline no longer lists `var()` outside colors, §2 row 31 and the `attr()` appendix line say
  shipped. rdom-css README "Values" lists `%`, `ch`, viewport units, math functions, angles and
  `attr()`. DIVERGENCES: the `flex` entry checked current after C2G-FLEX-SHORTHAND; §1 gains "a
  literal fractional length is invalid; a computed one rounds".
- 2026-10-05 — C2G-TEST-GAPS: every test the gate listed already exists, so none was added — calc
  nesting depth (`parse/values/calc_tests.rs`: `MAX_CALC_NESTING` / `MAX_CALC_DEPTH` caps; the
  gates suite's hostile `attr()`), `1/0` (`calc/semantics_tests.rs::division_by_zero_is_ieee`), an
  inset of `-infinity` (`css_phase2_gates.rs::infinite_insets_lay_out_without_overflow`), opacity with
  a percentage calc (`semantics_tests.rs::percentages_have_their_own_type`: `calc(50%)`, `calc(50% +
  0.25)`, `calc(50% * 50%)`), a fractional factor with `max-width` in the freeze loop
  (`fractional_factor_with_max_width_in_the_freeze_loop`) and an indefinite-height flex `max-height:
  %` (`max_height_percent_in_an_auto_height_flex_container_is_none`). `ScopeMemo`'s O(N × depth)
  memory is recorded in TECH_DEBT as the accepted simplification `SCOPE-MEMO-1`, with its bound.
  Phase 2 gate batch B (C2G-RESTYLE-WALK … C2G-TEST-GAPS) complete.
- 2026-10-05 — Phase 2 closed: 11 items + 20 gate fixes. Gate-fix re-review folded into the Phase 3 gate.
- 2026-10-05 — Phase 3 items, recorded at C3G-DOCS (the gate found no per-item entries; the
  commits and CHANGELOG bullets are the detailed record). C3-RGB: `Color::Rgba` (alpha < 255;
  `Color::rgba` normalizes an opaque alpha to `Rgb`, so equal colors compare equal — decided over an
  alpha field on every color); modern and legacy `rgb()` per CSS Color 4 §5.1, out-of-range
  channels clamped; alpha painted opaque until C3-ALPHA. C3-TRANSPARENT: `transparent` is
  `Rgba(0, 0, 0, 0)` (§6.3), not `Reset`; `reset` stays rdom's keyword for the terminal default;
  a transparent color paints nothing (text keeps the glyph beneath, a border keeps its space).
  C3-CURRENTCOLOR: `TuiColor::CurrentColor`, resolved after the whole ladder against the element's
  final `color` (§6.4), the inherited one in `color`; `caret-color` inherits it as specified.
  C3-HSL-HWB / C3-LAB: converted to sRGB at parse time — the computed color is sRGB, gamut-mapped by
  §13.2 OKLCh chroma reduction (decided: a terminal cell is sRGB, so a wider computed value would
  only be mapped later anyway; the divergence is in DIVERGENCES); conversions dependency-free
  (`color::{convert, gamut, matrices}`). C3-MIX: mixed at parse time; one holding `currentcolor`
  was deferred **as its CSS text** and re-parsed per element — decision superseded by
  C3G-COLOR-FUNCTION-PARSED (a parsed form). C3-RELATIVE: channel keywords substituted as numbers,
  then the function's own modern grammar parses the result (one grammar per function); a comma is
  rejected at the top level only (C3G-RELATIVE-COMMA). C3-SYSTEM: `Canvas` / `ButtonFace` /
  `CanvasText` / `FieldText` are the terminal's default colors (`reset`), the rest the UA palette
  (`color::system`), which the UA sheet now paints with; inside a color function the canvas ones
  take the canvas model of the element's scheme. C3-SCHEME: the document's preferred scheme from the
  terminal's OSC 11 background (`ColorScheme::for_background`, WCAG contrast of black vs white
  text), dark without an answer; mode 2031 left out then (crossterm could not parse the report),
  done with C3G-INPUT-READER. C3-ALPHA: translucent paint through a layer composited by the
  group-opacity per-cell rules (one compositor for `opacity` and color alpha). Decided: **the paint
  pass blends `Reset` against the canvas of the document's preferred scheme, not each element's used
  scheme** — the terminal's default colors are one pair for the whole screen whatever an element's
  `color-scheme` says, so the document's (the terminal's) scheme is the one that matches what is
  actually beneath; inside color functions the element's used scheme still applies (CSS Color
  Adjust §2). Also decided then: **`Cell::set_fg` / `set_bg` composited a translucent color against
  a fixed dark canvas** (the cell had no scheme) — superseded by C3G-SCHEME-CONSISTENCY, which made
  the `Buffer` the one canvas model and `Cell` opaque storage.
- 2026-10-05 — Phase 3 gates (with the C2G re-review: all 20 at the root). Architect: 2 blocking —
  color-function nesting has no depth cap (attr / var / CSS can overflow the stack; also quadratic);
  a positioned pseudo-element's translucent background composites twice under its text. API: 0
  blocking. Non-blocking: late OSC 11 replies become keystrokes and the query reads only stdin; relative
  colors reject any comma; caret colors ignore the scheme; two canvas models (`Cell` vs `Buffer`);
  a layer allocation per translucent write; `TuiColor::Function` deferred as text; powerless-hue
  thresholds differ from the sample code; transition `Reset` fallback; duplicated `border-color`
  initial; `uses_counters` ignores inline styles; files over the bar (`node.rs`, `positioning.rs`,
  `border.rs`); missing C3 log entries and decisions; 200 ms delay undocumented; no "scheme detected"
  accessor; a non-compiling migration hint; DESIGN lists; `CascadeExt` unsealed; sizing API asymmetry;
  re-exports; stale color docs and READMEs. Mode 2031: crossterm cannot parse the report in any release;
  decision — rdom owns the terminal input reader (`C3G-INPUT-READER`), which also fixes late replies.
  Fix all as `C3G-*`, two batches.
- 2026-10-05 — C3G-COLOR-DEPTH: `ColorCx::nested` caps color-function nesting at `MAX_COLOR_NESTING`
  = 32 (public beside `parse_color`, like `MAX_CALC_NESTING`); every color function — top level,
  a `color-mix()` / `light-dark()` argument, a relative origin — enters through `color::function`,
  which checks the cap before its level scans anything. `parse_absolute` takes a nested function's
  arguments from its component (which already ends at the `)`) instead of `closing_paren` rescanning
  the rest of the value per level; a value now costs at most 32 bounded passes. The registration
  matcher's `<color>` parsed every value twice (`parse_color` then `parse_color_at`); once now.
  Red: `color_function_nesting_is_capped` (33 levels parsed) failed and
  `hostile_color_nesting_is_invalid_not_a_stack_overflow` aborted the test binary (stack overflow);
  green: both pass, and a 10 000-level `attr(data-c type(<color>), …)` takes the fallback
  (`css_phase3_gates.rs`). DIVERGENCES: color functions nest at most 32 levels.
- 2026-10-05 — C3G-PSEUDO-TINT: `positioned_pseudos` writes its `content` with
  `glyph_style_from_computed` (the box is tinted first), so the background composites once under the
  text; `text.rs`'s "cannot double-blend" note is gone (a glyph write composites its style's
  background). Every other painter that fills a translucent background and then writes text was
  checked and pinned: a block's own text (`fill_bg` layer + glyph style), an inline element (bg only
  in its fragments' style, no fill), a static `::before` (likewise), a tree row highlight (`tint`
  after the label's glyph-style write), `::backdrop` (no text), `::selection` (one `set_style`).
  Red: the positioned pseudo was `(192, 0, 0)` under "hi", `(128, 0, 0)` beside it; green: both
  `(128, 0, 0)`; the four other cases pass before and after. Found: a positioned pseudo's
  `width` / `height` are not read (its size comes from the insets or the content) — the test sizes
  it with `right`.
- 2026-10-05 — C3G-RELATIVE-COMMA: `relative::parse` rejects a comma only at the arguments' top level
  (`channel::top_level_comma`, the check the legacy split already used), so a math function's own
  commas pass. Red: `rgb(from red min(r, 100) g b)` parsed to `None`; green: `rgb(100, 0, 0)`, and
  `oklch(from red clamp(0.2, l, 0.5) c h)` equals `oklch(from red 0.5 c h)`; through `var()` end to
  end in `css_phase3_gates.rs`. A comma between channels stays invalid.
- 2026-10-05 — C3G-SCHEME-CONSISTENCY: (1) the caret resolves `caret-color` / `caret-text-color` with
  `ColorContext::with_scheme(used scheme)`, and its `reset` fallbacks take that scheme's canvas
  (were white / black in every scheme). (2) One canvas model, the `Buffer`'s. Decided over passing
  the scheme into `Cell`: a cell is opaque storage, and only the buffer knows the scheme a
  translucent color blends against; the setters taking a scheme would put compositing in two places
  again. `Cell::set_fg` / `set_bg` store the color (transparent paints nothing) and debug-assert it is
  not translucent (release: stored, emitted as its opaque channels). The one internal path that
  reached a cell translucent — a tree guide in a translucent `border-color`, contributed straight to
  the buffer and written by the joiner — now contributes in a layer (`paint_translucent`) like
  `paint_border`. (3) `ActiveAnimation` records the element's used scheme; `lerp_color(a, b, t,
  reset)` interpolates a `reset` endpoint in Oklab as `AnimatedProp::reset_color` — the canvas
  background for `background-color`, the canvas text for `color` / `border-color` — and returns the
  endpoints exactly at t = 0 / 1; the sRGB fallback and its fixed `(192, 192, 192)` are deleted. A
  registered `<color>` has no role, so a `reset` endpoint there changes discretely (decided: no
  canvas color is right for every use of a custom property). Red: light-dark caret took the dark arm
  `rgb(4 5 6)`; the auto caret was white-on-black in a light document; `reset` → blue in a light
  document was `(96, 96, 224)` at the midpoint; `Cell::set_*` accepted a translucent color; the
  translucent tree guide tripped the new assertion in the joiner. Green: all six, plus a cell test
  that opaque / transparent writes behave as before.
- 2026-10-05 — C3G-POWERLESS-HUE: source fetched 2026-10-05 — CSS Color 4 §4.4.1 (a hue is powerless
  when the chroma or saturation is ≤ the space's ε; lightness is not a criterion) and its sample
  code: `conversions.js` `Lab_to_LCH` ε = 0.0015, `OKLab_to_OKLCH` ε = 0.000004 (both `chroma <=
  epsilon`), `better-rgbToHsl.js` ε = 1/100000 of a saturation of 1 (`sat <= epsilon`); `hwb()` §8,
  whiteness + blackness ≥ 100%. `interpolate::hue_is_powerless` (extracted from `in_space`) uses
  exactly those, cited in its doc; dropped: `L ≤ 0` for LCH / Oklch and HSL's lightness 0% / 100%
  test (the conversion already gives such a color a saturation of 0, as the sample code does).
  Red: LCH chroma 0.0016 counted as powerless (old ε 0.005625); green: boundary tests at each ε and
  just above, HWB at 100% / 99.9%, `L = 0` with chroma not powerless. No existing expectation
  changed.
- 2026-10-05 — C3G-SMALL-FIXES: (1) `border-color`'s initial `currentcolor` has one owner,
  `colors::BORDER_COLOR_INITIAL`: `ElementColors` notes whether any `border-color` declaration took
  part and `finalize` resolves an undeclared one as the initial value, after the final `color`;
  `apply::finalize_border_fg` and its two calls (`walk.rs`, `pseudo.rs`) are deleted. Refactor, no
  behaviour change: the suites pass unchanged. (2) `subtrees::uses_counters` takes the dom and the
  roots: a sheet rule, a top-level subtree already flagged `tree_has_counters`, or a `style`
  attribute in a root's subtree (iterative scan) puts the partial cascade on the ordered walk; the
  per-style predicate is `style_uses_counters`, shared by rules and inline styles. Red (bare sheet,
  `style="counter-reset: c 5"` on the list, `counter-increment: c; content: counter(c)` on two items):
  `cascade_subtrees` of the second item gave `1`; green: `7`.
- 2026-10-05 — C3G-OSC-ROBUST (interim until C3G-INPUT-READER): `reply::wait_left` (pure) decides
  the startup query's wait — 200 ms (`QUERY_TIMEOUT`) for a reply to begin, `REPLY_GRACE` (800 ms)
  more once `Replies::started` (an `ESC ]`, `ESC [ ?` or trailing `ESC` arrived; a typed key starts
  nothing), zero once complete; a reply not begun by 200 ms is not waited for. `read_replies` polls
  through it, so a reply that starts during a poll extends the wait. Input is stdin, or `/dev/tty`
  when stdin is not a terminal (crossterm's rule); stdout must be a terminal. The query returns the
  background, and `App::apply_detected_background` keeps it: `App::detected_background() ->
  Option<Color>` (decided over a `color_scheme_source()` enum: the color says more and `None` is
  "no answer"). Documented in `App::run` / `with_color_scheme` rustdoc, DIVERGENCES' scheme entry and
  the rdom-tui README: when it runs, the 200 ms worst case without an answer (+ 800 ms for a slow
  begun reply), dropped keystrokes, late replies as keys. Red (stubs compiled): `started` false for a
  partial OSC 11, `wait_left` zero for a begun reply, `detected_background` `None` after an answer;
  green: all three, plus the late-reply guard. The `/dev/tty` path has no automated test (needs a
  pty).
- 2026-10-05 — C3G-API: CHANGELOG hints fixed (`s.definite(scheme)`; `ColorContext::new(..)
  .with_scheme(..)`). DESIGN names `Color`, `TuiColor`, `ColorScheme` as closed data and
  `ColorContext` as an options bag, and sealed traits beside sealed types. `CascadeExt:
  sealed::Sealed` (private module; only `Dom<TuiExt>`); red: a `compile_fail` doctest implementing it
  for a local type compiled; green: it fails to compile. `ColorFunction`, `SystemColor`,
  `ColorSchemeList` at the `rdom_tui` root (red: unresolved imports). Sizing — one convention, decided:
  each type carries its keyword as a variant (`Size::Auto`, `MinSize::Auto`, new `MaxSize::None`,
  replacing C2G-MAX-NONE's `Option<MaxSize>`, so `TuiStyle::max_*: Option<Value<MaxSize>>`,
  `ComputedStyle::max_*: MaxSize`); a `u16` converts to cells for all three (`From<u16> for Size`
  added); `percent(p: f32)` on all three (`Size::percent` added; `MinSize` / `MaxSize` took `f64`);
  `cells(basis: Option<u16>) -> Option<u16>` on all three (`Size::cells` took `i32` and returned a
  signed value, `cells_u16` is gone; a size's percentage against an indefinite basis is `None`, i.e.
  `auto`; `Length::cells(i32) -> Option<i32>` stays signed — an inset is an offset, not an extent);
  the builder and the node setters all take `impl Into<T>` (`set_min_*` took `Option<MinSize>`,
  `set_max_*` `impl Into<Option<MaxSize>>`, so `set_max_width(40u16)` did not compile). Red: the new
  sizing tests did not compile (`From<u16> for Size`, `MaxSize::None`, `Size::percent` missing);
  green. Behaviour: block width now clamps a negative `calc()` width to 0 before the margin
  equation (it fed the negative value in, then clamped the result) — CSS Values 4 §10.12; no
  existing test changed for it. Changed expectations: tests that built `Some(MaxSize::…)` /
  `Value::Specified(None)` / `set_min_width(Some(..))` now use the variant forms; the
  `Size::cells` unit test reads `Some(0)` for `calc(50% - 50)` of 80 (was `-10`), and `cells_u16`
  cases moved to `cells`. Left as found: `ComputedStyle::min_*` stays `Option<MinSize>` — the flex
  cross axis treats an unset `min-*` (no floor) unlike an explicit `auto` (intrinsic floor), which
  CSS does not distinguish; making it `MinSize` would change that layout, so it is recorded in
  TECH_DEBT (`MIN-AUTO-UNSET-1`).
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): Final `min-*` / `max-*` shape (superseding
  C2G-MAX-NONE's `Option<MaxSize>`): `MaxSize::{None, Cells(u16), Percent(f32),
  Calc(Box<CalcExpr>)}` with `none` a variant, as `MinSize::Auto` is; `TuiStyle::max_width:
  Option<Value<MaxSize>>`, `ComputedStyle::max_width: MaxSize` (initial `MaxSize::None`);
  `parse_max_size` returns `Option<MaxSize>`. `MinSize` / `MaxSize` resolve with `cells(basis:
  Option<u16>) -> Option<u16>`, `None` for an indefinite basis, which makes a percentage 0 (min) or
  `none` (max), CSS 2.1 §10.7; `MinSize::percent` / `MaxSize::percent` build a percentage, and
  `MinSize` is no longer `Copy`.
- 2026-10-05 — C3G-INPUT-READER: rdom reads terminal input itself on Unix. `runtime/input/`:
  `mod.rs` (`Input { Event, Background, DeviceAttributes, ColorScheme }`, `ESC_GRACE` 25 ms),
  `parse/{mod,keys,mouse,csi,osc}.rs` (the parser, modeled on crossterm 0.28's
  `event/sys/unix/parse.rs`; corpus `parse/tests.rs`, its cases taken from crossterm's own tests)
  and `reader.rs` (rustix `poll` over the tty — stdin, or `/dev/tty` when redirected — and a
  `signal_hook::low_level::pipe` SIGWINCH self-pipe, crossterm's mechanism; new unix dependency
  `signal-hook`, already in the lock through crossterm). Output, modes and Windows input stay on
  crossterm (the Windows reader wraps `crossterm::event`). Decisions: (1) every CSI is framed by its
  final byte (ECMA-48 §5.4) and an unknown one consumed — crossterm holds every byte after
  `CSI ? 997 ; 1 n`; (2) a C0 / DEL / non-ASCII byte inside a CSI ends it and is read again
  (crossterm waits on); (3) `ESC ] <digit>` starts an OSC string, ended by BEL / ST, cancelled by
  CAN / SUB, ended by any other `ESC` (re-read), capped at 4 KiB; `ESC ]` + non-digit stays Alt+`]`;
  (4) a lone `ESC`, `ESC [`, `ESC O`, `ESC ]` is a key after 25 ms with nothing after it (crossterm:
  `ESC` at a read's end at once, the others held) — the grace also joins a sequence split right
  after its `ESC`; (5) `ESC [ ;` frames like a numbered CSI (crossterm's byte-wise parser drops it,
  though its whole-buffer test passes); (6) a zero mouse coordinate is dropped, not wrapped
  (crossterm's `- 1` underflows); (7) `\n` is Ctrl+J (raw mode, as crossterm under raw mode); (8)
  the terminal closing (EOF / hang-up) makes `poll` an error and `run` returns it (was a swallowed
  error per iteration). Bracketed paste is parsed but still not enabled (pastes arrive as keys, as
  before). The startup query now reads through the reader (`reply.rs`'s byte scanner replaced by
  `Replies::take` over parsed inputs; `wait_left` takes the reader's `in_sequence`), so keys typed
  during its wait are put back and handled; a late OSC 11 reply is the answer
  (`note_terminal_background`), a late DA1 nothing. Mode 2031: `enter_theme_reports` (`CSI ? 2031 h`,
  Unix, after the query in `App::run` — not in `enter_tui_mode`, whose crossterm-reading users
  would stall on the reports), reset in `leave_tui_mode` (guard and panic hook); a report sets the
  document scheme and cascades (`note_terminal_scheme`). Decided: a scheme the app set
  (`with_color_scheme` / `set_color_scheme`) is not overridden by reports or late replies.
  C3-SCHEME done. Red: the corpus and reader tests against a parser that dropped every byte (23 of
  26 failed; the three passing test idle / EOF / consumed replies); the scheme and theme-mode tests
  did not compile (`handle_input`, `enter_theme_reports`). Green: 26 + 3 scheme + 1 mode test; the
  reply tests rewritten over `Input`s (the byte-form cases moved to the corpus). The tty / SIGWINCH
  path has no automated test (needs a pty); the reader is tested over a socket pair.
- 2026-10-05 — C3G-TRANSLUCENT-FAST: `paint_translucent` paints into a scratch layer kept on the
  `Buffer` (`Scratch(Option<Box<Buffer>>)`: a clone starts without one, `Debug` hides it) and
  `copy_region_into` refills it in place, so a translucent write reallocates only when it covers more
  cells than any before it; the layer is taken out while in use, so a translucent paint inside the
  closure uses the layer's own scratch. Decided over a direct single-cell composite path: the paint
  closures write through the general `Buffer` API (glyphs, borders, half-block quads), and
  `composite_cell` reads a layer's border state, so a one-cell special case would have duplicated
  both; reuse covers every size. A test-only counting allocator (`test_alloc.rs`, per-thread count,
  `#[global_allocator]` under `cfg(test)`) measures it. Red: three 1×1 translucent writes after a
  warm-up made 12 allocations (four layers × three `Vec`s); green: 0, and the cells composite as
  before (the translucent buffer and paint suites pass unchanged).
- 2026-10-05 — C3G-COLOR-FUNCTION-PARSED: `ColorFunction { text: Arc<str>, expr: Arc<ColorExpr> }`
  (`parse/values/color/expr.rs`): the parsers of `color-mix()`, `light-dark()`, relative colors and
  a function's color arguments return a `ColorExpr` — `Absolute` (folded at parse time) or a node
  for what needs the element: `CurrentColor`, `Canvas(SystemColor)`, `LightDark`, `Mix` (the
  method and percentages parsed: `mix::Method`), `Relative` (origin node + the channel tokens,
  `Arc<[Token]>`). `ColorFunction::compute` evaluates it; nothing is tokenized or parsed per element,
  and the text (`context::render`) is only `css_text` now, so the render → tokenize round trip is
  gone from computation. `ColorCx` keeps only the nesting cap (its parse-time stand-ins and the
  `needs_element` flag are replaced by the node kinds). Decided: `Arc`, not `Rc` — `TuiStyle` is
  `Send + Sync` (as `CustomValue` decided); equality and hashing by text (the form is a function of
  it, and holds `f64`s). A relative color whose origin needs the element re-binds its kept channel
  tokens per element (the channels' math reads the origin's values); its grammar is checked at
  parse time against a stand-in origin, as before. `var()`: a custom property's color is still parsed
  per element by the substitution (its tokens can differ per element) but no longer a second time
  to compute. Red: computing `rgb(from color-mix(in srgb, currentcolor 50%, light-dark(white,
  black)) r g b / 50%)` for ten elements made 10 top-level function parses (test probe in
  `parse_function`); green: 0 after the declaration's 1, with the same colors as the functions
  written out. This supersedes the text-based deferral decision of C3-CURRENTCOLOR / C3-MIX.
- 2026-10-05 — C3G-MIN-AUTO (TECH_DEBT `MIN-AUTO-UNSET-1` paid): `ComputedStyle::min_width` /
  `min_height` are `MinSize`, initial `MinSize::Auto` (CSS Sizing 3 §5.2); the cascade applies them
  with `value!` like `max-*` (the `optional!` macro and `apply_optional` are gone). Decided — the
  cross-axis rule: `auto` is 0 there (`min_raw.cells(basis)`), per Flexbox §4.5 (the automatic
  minimum is a main-axis rule) and Sizing 3 §5.2, as the debt's pay-down said; folding the unset case
  into the old explicit-`auto` behaviour (an intrinsic floor on the cross axis) would have been
  non-spec. So the layout change is for an explicit `min-*: auto` on the cross axis only; on the main
  axis unset already took the automatic minimum (`main_axis.rs` matched `None | Some(Auto)`). Red:
  the computed-value test did not compile (`Option<MinSize>`); a column item `width: 10%` of 20 with
  `hello` nowrap and an explicit `min-width: auto` was 5 wide (floored at its content), spec and
  browsers 2. Green: both, unset and explicit alike. No showcase snapshot or other test expectation
  changed (no snapshot declares `min-*: auto`; the four tests that do are main-axis auto-min tests,
  unchanged); three cascade tests changed from `Some(MinSize::Cells(_))` to the variant. Breaking
  for rdom-style (CHANGELOG, migration hint).
- 2026-10-05 — C3G-PSEUDO-SIZE (found by C3G-PSEUDO-TINT): `positioned_pseudos::compute_placed_rect`
  sizes each axis with `positioning::resolve_size_axis` — the element path, now `pub(super)` and
  documented as shared — so a positioned pseudo's declared `width` / `height` (cells, `%` of the
  containing block, `calc()`, `Flex` as the extent) is its size; `auto` keeps the old rule (both
  insets → the span, else the `content`'s size). `axis_size_from_edges` became private to
  `positioning.rs`. CSS 2.1 §10.3.7 / §10.6.4: with `width` and both insets the box is
  over-constrained and `right` is ignored (ltr) — `axis_position_anchored` already placed it from
  `left`. Red: `left: 1; top: 1; width: 4; height: 2; content: "x"` was 1 × 1; `left: 2; right: 2;
  width: 3` was 6 wide. Green: 4 × 2, `50%` / `50%` of a 10 × 4 block from the far edges is 5 × 2 at
  (5, 2), and 3 wide; `auto` with both insets (6) and with one (content, 3 × 1) unchanged. Left as
  found: a `position: relative` pseudo with both horizontal insets still derives its width from them
  (relative positioning should only shift); `min-*` / `max-*` are not applied to positioned boxes,
  element or pseudo (both pre-existing, unchanged here).
- 2026-10-05 — C3G-SPLITS: no behaviour change. `paint_pass/border.rs` (566) → `background.rs` (96:
  `fill_bg` and its cell / border-state clears) + `border/mod.rs` (270: `paint_border`; the ten
  identical `add_dir` calls go through a `Pen { fg, priority, corner_style }`, so Phase 4's
  per-side widths and colors change one place) + `border/half_block.rs` (84); the ten
  `#[allow(dead_code)]` box-drawing constants (the joiner owns the glyphs) and a `let _ =
  Style::new()` are deleted. `node.rs` (641) → `node/{mod (45), read (154: TuiNodeExt), write (160:
  TuiNodeMutExt), tree (180: editable scope, text descendants, child text, rendered-ness), tests
  (135)}.rs`, re-exported at the old paths. `layout_pass/positioning.rs` (697 after C3G-PSEUDO-SIZE)
  → `positioning/{mod (108: containing block, accessors, re-exports), static_pos (158), relative
  (78), place (193: phase 2, `resolve_size_axis`), axis (93), tests (128)}.rs`; the items the rest
  of `layout_pass` uses are `pub(in crate::render::layout_pass)` and re-exported from `mod.rs`.
  CSS-COVERAGE's `PAINT` / `POS` keys point at the new paths. The suites pass unchanged.
- 2026-10-05 — C3G-DOCS: rustdoc — `rdom-style/src/color.rs` lists the four `Color` shapes (the
  ANSI-16 variants are long gone); the `tui_color` module doc names every `TuiColor` variant and the
  full `<color>` grammar; `parse_color` documents the grammar and its `None` for `currentcolor`,
  `light-dark()` and functions holding them. READMEs: root "Unreleased (0.6.0)" gains the color
  work; rdom-css's color list is the CSS Color 4 / 5 grammar; rdom-style's `TuiColor` row names its
  five variants and the property list is `PROPERTY_NAMES` (with `color-scheme`, `opacity`, the
  caret colors, `flex`, the border longhands, counters); rdom-tui's string-color paragraph and
  `App::run` loop description are current. Every example used a removed `Color::Red` / `White` /
  `DarkGray` / `Blue` / `Gray` (and one `.dim(true)`, removed in 0.2): rewritten as complete
  snippets, and the three READMEs are now doctests (`#[cfg(doctest)] #[doc =
  include_str!("../README.md")] struct ReadmeDoctests;` per `lib.rs`; `App::run` examples
  `no_run`), 17 blocks passing — decided over `ignore` blocks, which would rot again; no hidden `#`
  lines (GitHub shows them), so `fn main` / `.expect` where an example needs an error type. Recorded
  in CLAUDE.md §Testing Commands. CSS-COVERAGE: the "High-impact Partial items" mark 2–5 shipped
  (C1-VAR-ANY, C2-PERCENT, C2G-MAX-NONE / C3G-API, C5-MINMAX-SIZE) and 1 / 6 open with their items;
  row 16 notes `Color::Rgba` with alpha. The C3-MIX CHANGELOG bullet no longer says text. Per-item
  Phase 3 entries added above the gate entry, with the three decisions the gate found unrecorded.
- 2026-10-05 — C3G-COLOR-INTEGRATION: `tests/integration/css_phase3_colors.rs`, eight tests, each a
  sheet through `rdom_css::from_css_strict` → `CascadeExt::cascade` (under a set document scheme) →
  `layout_dom` → `paint_dom`, asserting painted cells: `oklch(100% 0 0)` white and sRGB red's Oklch
  back to red (±2); `rgb(0 0 0 / 50%)` over `rgb(200 0 0)` is `(100, 0, 0)` (±1) and the parent
  untouched beside it; a `currentColor` border in the element's `color` at both corners;
  `color-mix(in srgb, red, blue)` `(128, 0, 128)` and `color-mix(in srgb, currentColor, white)`
  as a background; `light-dark()` per document scheme and under `color-scheme: light` in a dark
  document; `rgb(from var(--brand) calc(r + 10) g b)`; `CanvasText` / `Canvas` painted `reset` in
  both schemes; `transparent` text leaves the cells blank over the parent's background. A
  characterisation of shipped behaviour: all eight passed on first run (no red phase — nothing was
  changed to make them pass). The batch's rustdoc gate found a private intra-doc link C3G-INPUT-READER
  added (`leave_tui_mode` → `enter_theme_reports`); made plain text here. Phase 3 gate batch B (C3G-INPUT-READER … C3G-COLOR-INTEGRATION)
  complete.
- 2026-10-06 — Phase 3 closed: 10 items + 16 gate fixes. Found during C3G-PSEUDO-SIZE: positioned boxes ignore
  `min-*` / `max-*` — added as C8-POS-MINMAX. The input reader (C3G-INPUT-READER) gets a focused look in the
  Phase 4 gate's re-review.
- 2026-10-06 — C4-BACKGROUND: `background` parses Backgrounds 3 §3.10 in full (`V/background.rs`):
  comma-separated layers, each sub-value at most once and in any order, the color on the final layer
  only; omitted sub-values reset to their initial values. The six image longhands and
  `background-clip` are new `TuiStyle` fields (one importance bit each, bits 47–53); images,
  positions and sizes are stored as validated CSS text (`url()` normalized to `url("…")`), the keyword
  families typed (`VisualBox`, `BackgroundRepeat`, `BackgroundAttachment`). Set / serialize arms live
  in the new `property_dispatch/background.rs`. Decided: `background-clip: text` (Backgrounds 4) is
  rejected — a cell cannot show a background through a glyph's shape (DIVERGENCES §1). Red: seven
  dispatch tests (`url(x.png) red` → `InvalidValue`, the longhands `UnknownProperty`); green after
  wiring; `css_phase4.rs` paints an image layer's color end to end. Changed expectation: the old
  `background_shorthand_sets_background_color` asserted `url(x.png) red` is invalid. No showcase
  snapshot changes (every demo `background:` is a lone color).
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): `background` resets every sub-value it omits:
  `background: none` is no image and a `transparent` color. `background-clip` took effect with
  C4-BG-CLIP.
- 2026-10-06 — C4-BG-CLIP: found as specified for the default — the box fill already covered the
  border box, so border cells took the background (`border-box`, §3.8's initial value). The final
  layer's clip computes into `ComputedStyle::background_clip` (new `cascade/decoration.rs`, which the
  later Phase 4 applicators join); `paint_pass/background.rs::paint_background` (moved out of
  `paint_box`) fills the border / padding / content box, a translucent color through the layer path.
  Content box: the laid-out content rect, or under `border-collapse: collapse` (whose content rect
  reaches into the shared ring) the box derived from the padding box. Decided: half-block keeps its
  cells clear — with a half-block side `border-box` acts as `padding-box`, the behaviour it had
  hard-coded, now documented against the property; `text` stays N/A (C4-BACKGROUND). Red: three of
  four `css_phase4.rs` clip tests painted the border / padding cells red; the default test was green
  (characterisation). Green after. No showcase snapshot changes (no demo sets `background-clip`, and
  the default fill box is unchanged).
- 2026-10-06 — C4-BORDER-SHORTHAND: `border` / `border-<side>` parse `<line-width> || <line-style> ||
  <color>` (Backgrounds 3 §4.4) in `V/border.rs`; set / serialize arms moved to the new
  `property_dispatch/border.rs` (`set.rs` / `serialize.rs` lost theirs). Storage: the colors and widths
  are true per-side longhands — `TuiStyle::border_color` / `border_width: Sides<Option<Value<_>>>`, one
  importance bit each (the `Field` table now addresses a path, `border_color.top`); `ImportantMask` grew
  to `u128` (Phase 4 needs more than 64 bits). The styles keep their shared field until C4-BORDER-SIDES.
  Computed: `border_color: Sides<Color>`, each side resolved like `color` (`currentcolor` initial per
  side); the painter's `Pen` carries per-side colors, and sides of different alpha paint in separate
  passes (translucent ones through the layer path). Decided: widths keep pixel / `em` lengths in pixels
  (`PaintLength`, new in `layout/border.rs`, split out of `box_model.rs`), since a width only picks a
  glyph weight — documented in DIVERGENCES §2; rdom's keywords kept, none deprecated (`rounded` now also
  beside a width and color). Found: with widths and colors in `border`, a rounded ring set from Rust
  could no longer round-trip through the `style` attribute (the corner flag was expressible only as
  `border: rounded`); fixed by making `border-style: rounded` round the ring (and other styles square
  it), CHANGELOG Changed. Red: `css_phase4.rs` `border: 1px solid red` and `border-top: 1px solid red`
  failed to parse (strict sheet) against the old code; the new dispatch tests and the
  `border-style: rounded` test red before their code. Green after. Changed expectations: tests reading
  `computed.border_fg` read `border_color.top`; `declared_count` counts `border_fg(..)` as four. No
  showcase snapshot changes.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): `parse::values::parse_border` returns a
  `BorderRing`: the four side styles, the width and the color every side takes, and whether it was
  rdom's `rounded`. An omitted component resets to its initial value: width `medium`, style `none`,
  color `currentcolor`.
- 2026-10-06 — C4-BORDER-SIDES: `border-style` / `-color` / `-width` take 1–4 values
  (`V/border.rs::parse_sides` over `Sides::from_values`), serialized in the shortest form; the eight
  `border-<side>-color` / `-width` names join the table, one field each. Decided — the corner rule: a
  corner cell of one box goes to its dominant side, the heavier style (Tables 3 §11.5's ranking) and
  then the horizontal side (`border_join.rs::dominant_contribution`, a third key after rank and
  priority); the browser's diagonal split cannot fit one glyph, and giving the cell to the top /
  bottom keeps those edges whole lines — documented in DIVERGENCES §2. (C4-BORDER-WIDTH puts the
  weight first.) Kept: the styles still share one field — they split per side with C4-RADIUS, where
  the corner flag they carry becomes `border-radius`. `rounded` rounds the ring only as
  `border-style`'s one value. Red: four dispatch tests (`red blue` → `InvalidValue`, the longhands
  `UnknownProperty`) and `per_side_colors_and_the_corner_rule` (the top-right corner blue: the
  joiner read the right side's S first). Green after; the per-side cascade and double-dominance paint
  tests were green on first run (characterisation of C4-BORDER-SHORTHAND's storage and the existing
  rank rule). No showcase snapshot changes (no demo colors sides differently).
- 2026-10-06 — C4-BORDER-WIDTH: `BorderWidth::weight` maps a width to `BorderWeight::{Light, Heavy}` or
  none — `thin` / `medium` light, `thick` heavy, pixel lengths heavy from `thick`'s 5px, cell lengths
  from two cells (rounded onto the grid), any non-zero length at least light (a browser draws a
  sub-pixel border one device pixel wide). The cascade keeps the declared styles in the new
  `ComputedStyle::border_style` and makes `border` the used border (`Border::with_widths`, after the
  viewport units resolve, `cascade/decoration.rs::finalize_used_border`), so layout and paint drop a
  zero-width side without a change, and `inherit` copies the declared style. Paint: contributions
  carry a weight; `merge` and the corner rule rank weight first (Tables 3 §11.5 rule 3); the joiner
  draws each direction at its winner's weight from an 81-entry light / heavy table generated from the
  Unicode names (`border_join/glyphs.rs`, split out of `border_join.rs` with the other tables), so a
  heavy top over light sides gives `┍━┑`. Decided: always one cell wide (weight by glyph); `double`
  ignores weight; a heavy rounded corner is square (no heavy arcs in Unicode). Red: four
  `css_phase4.rs` tests (`thick` drew `┌`, `5px` `┌`, `border: 0 solid` took the corner cell, mixed
  corners `┌`). Green after; the inherit test was written green against the design. No showcase
  snapshot changes (every demo border is `medium`).
- 2026-10-06 — C4-RADIUS: `border-radius` (1–4 values, `/` vertical radii; the top-level `/` only)
  and the four corner longhands (one or two radii) parse into `BorderRadius { horizontal, vertical }`
  (`PaintLength`s, percentages allowed) per corner (`Corners<T>`, beside `Sides`), cascade into
  `ComputedStyle::border_radius` (viewport units resolved), and paint per corner: `Pen::corner_at`
  gives a corner cell its corner's `CornerStyle`, rounded when both radii are non-zero against the
  border box (§5.1). The corner flag left `Border`, so the border styles split into four longhands too
  (`TuiStyle::border_style: Sides<…>`) — the shared-storage divergence is gone for every border
  property (it remains for padding / margin). Decided: `border: rounded` stays as sugar for `solid` +
  `border-radius: 1`, written as a side effect of the value (the shorthand's importance / keywords do
  not reach the radius — documented, `border-radius` recommended); elsewhere `rounded` is `solid`.
  C4-BORDER-SHORTHAND's `border-style: rounded` (added there only so a rounded ring could round-trip
  through the `style` attribute) is reverted: the radius serializes on its own now, and its CHANGELOG
  bullet is dropped. The background is not clipped to the curve (one cell); heavy / double / junction
  corners stay square. Red: five `css_phase4.rs` tests (four strict-parse failures on
  `border-radius`, and `border-top: none` in a later rule erasing all four sides — the shared style
  storage). Green after. Changed expectations: tests building `Border::rounded()` now build `single()`
  + `BorderRadius::cells(1.0)` (same paint); the two node-setter tests use a double ring (a node setter
  sets styles only); `border_shorthand_keeps_rdom_keywords` asserts the radius instead of the corner
  flag. No showcase snapshot changes (the UA dialog's rounded ring is now a radius; same glyphs).
- 2026-10-06 — C4-SHADOW: `box-shadow` parses `none | <shadow>#` (`V/shadow.rs`: lengths contiguous,
  blur non-negative, color and `inset` on either side; `split_commas` moved to `numeric.rs`, shared
  with the background layers; `paint_length` takes a sign range). Storage `BoxShadow<C>`: declared
  with a `TuiColor`, computed with a `Color` — the cascade resolves the colors in
  `ElementColors::finalize` against the element's final `color` (a winning list waits like the
  border colors; a color a `var()` chain leaves unresolved makes the declaration invalid at
  computed-value time → no shadow). Paint (`paint_pass/shadow.rs`): outer shadows before the
  background, as the border box moved by the offsets and grown by the spread, minus the border box
  (up to four bands); inset ones after the background, the padding box minus itself moved and shrunk;
  reverse order so the first is on top; opaque colors fill (occluding glyphs beneath), translucent
  ones through the layer path. Decided: offsets / spread are whole cells, a pixel length one cell by
  its sign (`PaintLength::offset_cells`), blur inert — documented; a blur-only glow therefore draws
  nothing. `tui_style/builder.rs` (587 lines, past the bar with the new setter) split:
  `builder/{mod, decoration}.rs`, the background / border / shadow setters in the second; and
  `paint_border_sides` moved from `paint_pass/mod.rs` (591 lines) into `border/mod.rs` (534 / 380). Red: six
  `css_phase4.rs` tests failed to parse `box-shadow`; green after (the spread test's own setup was
  fixed — it set a `style` attribute the headless cascade does not read). No showcase snapshot
  changes (no demo declares a shadow).
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): A shadow's color is initially `currentcolor`;
  the lengths are written together, with `inset` and the color in any order around them.
- 2026-10-06 — C4-SPACING (partial): `border-spacing` parses one or two non-negative cell lengths
  (`V/border.rs::parse_border_spacing`; `GapValue` per axis, viewport units resolved at computed-value
  time like `gap`), cascades (`value!`) and inherits (CSS 2.1 §17.6.1: added to `inherits` and
  `inherit_inheritable_from`, probed by the inherited-set test). Decided: no layout now — rdom's
  tables are flex rows with a column-sync pass, and border spacing belongs to the separated-borders
  table model (spacing between cells *and* between the cells and the table's border); emulating it
  with `gap` would be wrong at the table's edges. Status `partial — layout lands with C13-TFC`. Pixel
  lengths are not taken: unlike a border width this length is geometry. Red: the dispatch test
  (`UnknownProperty`); green after, with the cascade test in `css_phase4.rs`. No showcase snapshot
  changes.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): `border-spacing` takes rdom's cell lengths
  only — no percentages (CSS 2.1 §17.6.1).
- 2026-10-06 — Phase 4 gates (with the C3G re-review: all 16 at the root; the input reader matches
  crossterm 0.28 case by case and is stricter on unknown CSI, C0 inside CSI, zero mouse coordinates,
  EOF and EINTR). Architect: 1 blocking — the ESC grace check flushes a lone ESC *before* reading
  bytes already queued, so a loop late by > 25 ms splits a sequence and types its tail into the
  focused field. API: 1 blocking — `box-shadow` offsets / spread saturate to `i32::MAX` and overflow
  in shadow geometry (debug panic from a `style=""` attribute). Non-blocking: the OSC cap does not
  discard (and CR does not end an OSC, so Alt+`]` + digit can swallow typing); `ESC ESC` drops one;
  `CSI >` / `=` / DCS / APC unframed; Ctrl+F3 eaten as a cursor report; `leave_tui_mode` skips
  cleanup after a failed write; mixed double / single corners use the dominant table's glyph; an
  opaque outer shadow erases an earlier sibling's text (Appendix E paints shadows with backgrounds,
  before text); `paint_border_sides` allocates per frame and composites the whole box for a
  translucent side; `ImportantMask` at `u128` already 72 bits used; `paint_pass/mod.rs` 534 lines;
  pixel lengths only bare (not inside `calc()`); CHANGELOG corner rule stale; coverage audit stale on
  `border-style: rounded` and §5's Phase 4 rows; the `border: rounded` radius surviving a later
  `border` and the `border` colour reset are undocumented behaviour changes; Phase 4 value types and
  `MinSize` / `MaxSize` not in the prelude or root (the CHANGELOG hint does not compile prelude-only),
  no `set_border_radius`, `MinSize` lacks `Default`; DESIGN does not classify the new public types;
  kept image / position text serializes with stray spaces and `url(0001.png)` loses zeros; README
  gaps; only `CascadeExt` sealed. Accepted: pixel-to-cell conventions (rule to be written into
  DESIGN), `dashed` / `dotted` solid, inert images, a partial CSI never timing out, C1 controls
  dropped, no pty test for the tty paths. Fix all as `C4G-*`, two batches: A — input reader and
  paint (`C4G-ESC-GRACE`, `C4G-OSC-DISCARD`, `C4G-ESC-ESC`, `C4G-CSI-FRAMING`, `C4G-CTRL-F3`,
  `C4G-LEAVE-TUI`, `C4G-SHADOW-CLAMP`, `C4G-MIXED-CORNERS`, `C4G-SHADOW-ORDER`, `C4G-BORDER-COST`,
  `C4G-PAINT-SPLIT`); B — API and docs (`C4G-IMPORTANT-BITSET`, `C4G-REEXPORTS`, `C4G-SERIALIZE`,
  `C4G-PX-CALC`, `C4G-SEALED`, `C4G-DOCS`, `C4G-EDGE-TESTS`).
- 2026-10-06 — C4G-ESC-GRACE: `InputReader::poll` no longer flushes an expired escape prefix before
  reading. Once the grace has passed it first reads what is already queued (`read_queued`: a
  zero-timeout readiness check and read) and flushes only when nothing came; bytes that did come
  restart the grace if they leave a prefix. A read that fills the 1 KiB buffer is marked "more may
  follow" (`Source::read` returns it) and reading goes on without waiting, as crossterm does.
  Decided: the grace is measured from when rdom read the prefix, not from when it arrived — the
  reader cannot know the latter; a sequence queued behind it is always joined. Red:
  `a_late_poll_reads_queued_bytes_before_the_grace_flushes` (ESC, a sleep past the grace, `[A`, a
  zero-timeout poll gave Esc) and `a_full_read_reads_on` (1023 + 3 bytes: 1023 inputs after one poll,
  Up missing); green after, with `a_lone_escape_flushes_on_a_late_zero_timeout_poll` pinning the
  lone-ESC case.
- 2026-10-06 — C4G-OSC-DISCARD: new `parse/string.rs` holds the command-string framing (ECMA-48
  §5.6): `string::byte` classifies a byte (body 0x08–0x0D / 0x20–0x7E, `ESC`, BEL, CAN / SUB, other);
  `osc::parse` uses it, so a byte outside the string range aborts the OSC and is read again (it was
  kept as string body), and a string past `MAX_LEN` (4 KiB) returns the new `Step::Discard`: the
  parser drops the buffer and discards the rest without buffering (`Parser::discard`, a two-state
  `string::Discard`) until BEL / ST / CAN / SUB, or until an `ESC` + non-`\` or an aborting byte,
  which are read again. It used to return `Invalid` and type the remaining bytes as keys.
  `in_sequence` is true while discarding. Decided per the gate: CR is in the string range, so it does
  not end an OSC — Alt+`]` + a digit typed within the grace swallows typing up to the next control
  (Backspace, Ctrl+C, an arrow's `ESC`); pinned by a test. Red: both new corpus tests (5000 `x`s
  typed back as keys; Backspace and Ctrl+C swallowed); green after.
- 2026-10-06 — C4G-ESC-ESC: `ESC ESC` consumed both bytes for one Esc (crossterm's reading), so
  two quick Esc presses were one. `parse::escape_then` now decides on the third byte: `ESC` + a CSI or
  SS3 key is Alt + that key; anything else is Esc with the second `ESC` read again (`ESC ESC x` →
  Esc, Alt+x; `ESC ESC` + mouse report → Esc, the mouse event). `awaits_prefix` adds `ESC ESC` and
  `ESC ESC [` / `O`, and `flush_prefix` gives each leading `ESC` its own Esc. Decided: legacy
  `ESC ESC [ A` is Alt+Up — rxvt sends it for Alt+arrow and Terminal.app with Option as Meta sends
  `ESC` before the key's sequence; crossterm typed it as Esc, `[`, `A`. Red: the new corpus test
  (`ESC ESC x` gave Esc, plain `x`); green after. Changed expectation: `alt_keys` asserted
  crossterm's one Esc for `ESC ESC` — removed, the new test covers both readings.
- 2026-10-06 — C4G-CSI-FRAMING: `csi::parse` frames every sequence whose first byte after `ESC [` is
  a parameter or intermediate byte (0x20–0x3F; it took only digits, `;`, `<`, `?`, so `CSI > …` was
  `Invalid` and its parameters were typed); `dispatch` consumes one with intermediates (no key or
  report rdom reads has them) or a private marker other than `<` / `?`. `string.rs` now frames all five
  command strings: `string::parse(buf, finish)` with the start rule per introducer — OSC a digit, DCS a
  parameter / intermediate byte (XTVERSION `>|`, DECRQSS `1$r`, XTGETTCAP `1+r` all do), APC / PM /
  SOS any string byte — and OSC's body, cap, discard and abort rules; `osc.rs` keeps only OSC 11.
  `ESC P` / `_` / `^` / `X` join the escape prefixes (Alt + the key after the grace, or before a byte
  that cannot start the string). Decided: Alt+Shift+P / X, Alt+`_` / `^` followed by another key
  within the grace read as a string start, the trade-off OSC already makes; no terminal sends PM or
  SOS, but framing them costs nothing. Red: DA2 typed `41;388;0c`, XTVERSION typed `>|XTerm(388)`;
  green after.
- 2026-10-06 — C4G-CTRL-F3: grepped for a cursor position request — no `CSI 6 n`, no
  `crossterm::cursor::position` anywhere in the workspace; the startup query sends OSC 11 and DA1 only.
  So `csi::dispatch` no longer consumes `CSI … R`: `CSI 1 ; m R` goes to `keys::modified` (F3 with
  modifiers) and a bare `CSI R` is F3 beside `P` / `Q` / `S`. Red: `csi_r_is_f3` (`CSI R` gave
  nothing); green after. Changed expectation: `other_replies_are_consumed` dropped its
  `CSI 20 ; 10 R` (now F3 with a modifier mask, as a terminal would mean it).
- 2026-10-06 — C4G-LEAVE-TUI: `leave_tui_mode` delegates to the private `restore_terminal(writer,
  raw_off)`, which `queue!`s each step on its own — pop the keyboard flags (first: kitty keeps a stack
  per screen), focus off, mouse off, bracketed paste off (added: never enabled by rdom, but an app may
  have), cursor shown, alternate screen left, mode 2031 off (Unix), SGR reset — then flushes and calls
  `raw_off`, recording only the first error. The raw-mode switch is a parameter so a test can observe
  it. Red: a writer failing its first write left nothing written (the `?1000l` assertion), and an
  always-failing writer never reached `raw_off` (0 calls); green after. (The first attempt at the test
  failed with `ErrorKind::Interrupted`, which `write_all` retries forever — the test writer uses
  `io::Error::other`.)
- 2026-10-06 — C4G-SHADOW-CLAMP: `PaintLength::offset_cells` clamps to ±`u16::MAX` cells (it saturated
  at ±`i32::MAX`, and `grow` doubled the spread). `paint_pass/shadow.rs` now does its geometry over a
  private `Edges` (signed left / top / right / bottom, every operation saturating) and converts to a
  grid `Rect` only after clipping; the old `LayoutRect` form clamped a grown shade's extent to
  `u16::MAX`, so a spread of 65 535 around a box at x = 1 ended at x = 1 instead of past the grid —
  found by the new test once the panic was gone. `minus` returns an iterator (no `Vec`). Red:
  `box_shadow_huge_lengths_do_not_overflow` panicked "attempt to multiply with overflow" in `grow`;
  `offset_cells_clamps_to_the_grid_range` gave 2147483647. Green after, with
  `grow_keeps_edges_past_a_u16_extent`. Note: the gate's bare `9999999999` does not parse — an integer
  literal outside `i32` tokenizes as a `Float`, which cell lengths do not take — so the tests use
  `2147483647` (the largest integer token) and `9999999999ch`; both panicked.
- 2026-10-06 — C4G-MIXED-CORNERS: `border_join` reads each direction's line (`glyphs::Line`: none,
  light, heavy, double) and `glyphs::junction_glyph` picks the glyph — single lines by weight as before,
  all-double from `DOUBLE_TABLE`, a double axis crossing a light one from two new 16-entry tables
  (`VERTICAL_DOUBLE_TABLE`, `HORIZONTAL_DOUBLE_TABLE`; each glyph's Unicode name checked with Python's
  `unicodedata` and written beside it). `None` — heavy meeting double, or an axis double on one side
  and single on the other — falls back to the old dominant-style rule (documented in DIVERGENCES §2).
  The rounded-corner path now requires every line light (it ignored a non-dominant double side).
  Decided against the item text: it asked for `╓` at `border-style: double solid`'s top-left, but
  that value is double top / bottom and single left / right, so the corner joins a double line going
  right and a single one going down — U+2552 `╒` DOWN SINGLE AND RIGHT DOUBLE; `╓` (DOWN DOUBLE AND
  RIGHT SINGLE) is `solid double`'s corner. Both are tested. Red: `double solid` drew `╔═══╗` over
  `│`, the collapsed pair `╔═══╦═══╗`; green after, plus a unit test over all 18 mixed glyphs and the
  three no-glyph cases. No showcase snapshot changed (no demo mixes double and single sides).
- 2026-10-06 — C4G-SHADOW-ORDER: fixed, without a second walk. `stacking::collect_layers`, which
  already walks a context's whole in-flow subtree (stopping at nested contexts, descending into
  `z-index: auto` boxes), now also gathers the in-flow boxes with an outer shadow (`ShadowEntry`, with
  the clip they paint into and their paint unit: the context, or the `z-index: auto` box they lie in —
  Appendix E step 8 paints such a box "as if it created a new stacking context";
  `Layers::shadows_of(unit)`, `LayerEntry::unit`). Only boxes the content walk paints as boxes count
  (`paint_pass::paints_child_box`: not under an IFC or a canvas, not an orphan inline). Paint, two
  strokes (`shadow::Shadows`): (1) the unit's background phase — after the root's box and the
  negative layers, before its content — paints those boxes' opaque shadows over what is beneath
  (`paint_backdrop_shadows`); (2) at the box's own turn the shadow paints again *under* the glyphs
  painted so far (`background::tint_bg`: background set, border contributions cleared, glyphs kept),
  so it lies over the earlier siblings' backgrounds and borders (tree order, step 4) but under their
  text (step 7). A unit root's own shadow paints as before; a translucent shadow composites once, at
  its turn (the layer composite keeps glyphs beneath). Decided over a pure background-phase pass,
  which would have put every shadow under every earlier background (a focus ring on the second of
  two buttons with backgrounds would lose its overlap), and over a full step-4 / step-7 split of
  `paint_box` / `paint_content`, which is the large refactor. The remaining per-box interleave —
  a later block's background over an earlier block's overflowing text — is now stated in
  DIVERGENCES §2 (the Appendix E line claimed the full order). Red:
  `an_outer_shadow_paints_under_earlier_siblings_text` (row 0 blank, the ring erased `aaaa`, with
  and without a background on the first block); green after, with
  `an_outer_shadow_covers_a_negative_z_layer`, `a_shadow_inside_a_positioned_box_covers_the_page_beneath`
  (fails if the unit's background phase is removed — checked by deleting the call) and
  `a_later_background_covers_an_earlier_shadow`. `paint_pass/mod.rs` is 583 lines; C4G-PAINT-SPLIT
  next. No showcase snapshot changed.
- 2026-10-06 — C4G-BORDER-COST: `paint_border_sides` reads `border_width` / `border_radius` in place
  (`Sides::each` / `Corners::each`, no clone — a `calc()` radius cloned two boxes per corner), and walks
  the ring once for every opaque side (the `Ink` already carries per-side colors; the `only` mask
  selects them) instead of once per distinct color. Translucent sides go by alpha (sides of one alpha
  share a layer; a fixed-array scan, no `Vec`), and each layer covers only the ring: `ring_strips`
  cuts up to four disjoint strips of the visible border box (top / bottom rows, the columns between;
  a corner joins its column when its row is not a strip), each composited on its own — the whole
  `outer_grid` was copied and composited per translucent color. `paint_border` takes the real
  buffer's area (`bounds`) for its off-buffer rule, since a layer now covers part of the box (before,
  a translucent pass used the layer's area, so a box cut by an overflow clip lost the directions at
  the cut — the opaque path did not). Red: `border/tests.rs::a_border_paints_without_allocating`
  (four side colors, one translucent, a `calc()` radius) counted 9 allocations with the
  C3G-TRANSLUCENT-FAST `test_alloc` allocator; green: 0. The translucent border tests
  (`color_tests.rs`) pass unchanged.
- 2026-10-06 — C4G-PAINT-SPLIT: no behaviour change. `paint_pass/mod.rs` (583 lines after
  C4G-SHADOW-ORDER) → `mod.rs` (188: `PaintExt`, `layout_rect_to_grid`, the module map),
  `stacking_walk.rs` (177: `paint_stacking_context` / `_body`, `paint_layers`, `paint_plain`,
  `recurse_children`, `orphan_inline`, `paints_child_box`) and `box_paint.rs` (254: `BoxFrame`,
  `paint_box`, `paint_content`, `compute_border_priority`, `fills`); `fills`, `paints_child_box` and
  `paint_stacking_context` re-exported at their old paths. Every test passes unchanged. Phase 4 gate
  batch A (C4G-ESC-GRACE … C4G-PAINT-SPLIT) done; batch B open.

- 2026-10-06 — C4G-IMPORTANT-BITSET: `ImportantMask` is an opaque `[u64; N]` (`tui_style/important.rs`),
  `N` from the property table's row count (`IMPORTANT_BITS`, emitted by `define_fields!`). Each row of
  the table names its constant and gets the row's index as its bit (`ImportantMask::bit(Field as
  usize)`), so the 72 hand-numbered bits are gone and a new field cannot collide or run out of width.
  `bits()` / `from_bits_truncate` dropped (no width-independent use; `count()` replaces the one test
  use), `union` is `const` for group constants, `Debug` prints the names. Decided: one bit per field,
  so the four `transition-*` longhands, which shared `TRANSITIONS`, each get theirs (Cascade 4 §6.4,
  importance is per declaration; `transition-duration: 1s !important` made `transition-delay`
  important) — `TRANSITIONS` stays as their union for the builder and the shorthand; the cascade
  reads the per-longhand bits. Red: `every_dispatched_field_has_a_distinct_important_bit`
  ("TransitionProperty and TransitionDuration share a bit"); green after, with
  `operations_span_every_word` over the last bit. The hand-listed `important_mask_bits_are_unique`
  is replaced by that test (its FLOW / POINTER_EVENTS regression kept as its own test).
- 2026-10-06 — C4G-REEXPORTS: the `rdom_tui` root and prelude re-export the Phase 4 value types
  (`BorderRadius`, `BorderWidth`, `BorderWeight`, `BorderStyle`, `CornerStyle`, `BoxShadow`,
  `PaintLength`, `BorderSpacing`, `Sides`, `Corners`, `VisualBox`, `RepeatStyle`,
  `BackgroundRepeat`, `BackgroundAttachment`) and the sizing types (`MinSize`, `MaxSize`,
  `FlexBasis`, `AspectRatio`); the prelude adds what the CHANGELOG hints name (`CalcExpr`,
  `CustomValue`, `ContentContext`, the color types, `parse_color`, `resolve_tui_color`), and the root
  re-exports rdom-style's `parse`, `property_dispatch` and `backend` modules, so the hints' module
  paths (`property_dispatch::set`, `parse::Token`, `backend::SubstitutionContext`, `calc::to_cells`)
  resolve from `rdom_tui`. Three hints were rewritten to those paths (`rdom_style::property_dispatch`,
  bare `Token`, bare `SubstitutionContext`). `MinSize` derives `Default` (`Auto`). Decided — uniform
  percentages: `MinSize` / `MaxSize` gain `Percent(f32)`, the shape `Size::Percent` has; `percent()`
  builds it in all three and the parser stores a bare percentage there (a math function stays
  `Calc`; a percentage past `percent_fraction`'s range stays `Calc` as before, so nothing that
  parsed stops parsing). Resolution is unchanged (`Size::percent_of`, ties to even — what `calc()`
  gave). New node accessors `TuiNodeMutExt::set_border_radius` / `TuiNodeExt::border_radius` beside
  `set_border` / `border`. Red: `tests/integration/prelude_migration.rs` (one test per hint group,
  `use rdom_tui::prelude::*` only) failed to compile — 30+ unresolved names, `MinSize::default`,
  `MinSize::Percent`, `set_border_radius`; green after, with
  `min_max_percentages_share_the_size_shape` (dispatch) written against the decision. Changed
  expectation: `css_phase2_gates::max_width_percent_from_rdom_tui_paths` asserted
  `MaxSize::percent(50.0) == Calc(Percent(50))`; it now asserts `Percent(50.0)` and that the `calc()`
  form resolves the same.
- 2026-10-06 — C4G-NUMBER-RANGE: the tokenizer keeps digits-only literals integer-typed and clamps
  them to `i32::MAX` (CSS Syntax 3 §4.3.12 type flag, §4.3.13 conversion; Values 4 §5.1 clamps a value
  outside the implementation's range) — `read_number` returned `Float` past `i32`; a dimension's number
  part follows (`integer: true`, value `i32::MAX`). Decided — unitless fractions: rdom's cell takes any
  `<number>`; `length_percentage` maps a `Float` to `Cells` (the leaf a `calc(1.5)` already produced),
  so every cell-length property (`width`, `gap`, `padding`, `margin`, insets, `border-spacing`,
  `border-width`, `border-radius`, `box-shadow`, `min-*` / `max-*`, `flex`'s basis) takes `1.5` alike;
  the alternative (reject it everywhere) would have left `1.5` invalid where `calc(1.5)` is valid.
  Integer literals past a property's storage range are still rejected where they were (`width:
  9999999999`), clamped only where the property clamps (paint lengths). DIVERGENCES §1 "Length units"
  states the rule; the coverage row is `<number>` cells. Red: `oversized_integer_clamps_and_stays_integer`
  (`Float(99999999999.0)`), `box_shadow_takes_an_integer_past_the_range` and
  `unitless_fractions_are_cell_lengths_everywhere` (`box-shadow: 1.5 -0.5 red` → `InvalidValue`); green
  after, and `css_phase4::box_shadow_huge_lengths_do_not_overflow` runs the bare `9999999999` too.
  Changed expectations: the tokenizer test `oversized_integer_is_a_float_not_zero` is replaced by the
  clamping one; `flex_shorthand_full_grammar` listed `1 2 0.5` as invalid — it is `1 2 0` now (moved
  to the valid cases as `1 2 2.5`).
- 2026-10-06 — C4G-PX-CALC: `paint_length` (`V/border.rs`), the one leaf of border widths, radii and
  shadow lengths, routes a math function holding a pixel-family dimension to `pixel_math`: each pixel
  leaf becomes a context-free `ch` length of its pixel value, the calc parser builds and types the
  expression, and `CalcExpr::kind_strict` (new; CSS's own §10.9 typing, `Typing { percent, cells }`
  threaded through `kind_with` / `unify` / `function_kind`) requires a `<length>` with numbers as
  factors only. The result is pixels (`PaintLength::Px`), clamped into the property's range (§10.12)
  and NaN-safe; a non-pixel unit or a percentage beside a pixel is invalid. Decided: the durable rule
  is written into DESIGN ("Pixel lengths select, cells measure") — a length that only selects a
  discrete option (glyph weight, corner shape, a one-cell offset's sign) may take pixels, geometry never
  does; `outline-width`, `outline-offset` and `text-shadow` will follow it. A pixel math function is
  folded when parsed (CSSOM reads `2px`), as `<number>` math is (DIVERGENCES §1, the border-width
  entry). Red:
  `pixel_lengths_inside_math_functions` (`border-width: calc(2px)` → `InvalidValue`); green after,
  with eight invalid mixes (`calc(2px + 1)`, `calc(2px * 2px)`, `max(1px, 2)`, pixels with `ch` /
  `%` / `vw` / `deg`). No showcase snapshot changes.
- 2026-10-06 — C4G-SERIALIZE: two roots. (1) `render_value` put a space before every token. Decided
  against keeping source spans or whitespace tokens: a browser does not echo the author's whitespace
  for these specified values either — CSSOM §6.7.2 serializes them from their parsed form, not from the source text — so the renderer now writes
  that form: one space between component values, none inside parentheses or before a comma, one after
  it, and still never two tokens that would re-tokenize as one (an ident stays apart from a following
  `(`, a number from an ident). The one token whose meaning depends on the whitespace the tokenizer
  drops is `-` / `+` (the sign is a delimiter, not part of the number): it is glued to a following
  number unless it follows an operand inside a math function, the only place CSS has a binary one
  (`calc(50% - 1px)`); the renderer tracks the math-function nesting for that. (2) `url(0001.png)`
  lost its zeros because the tokenizer had no `<url-token>`: `read_ident_or_function` now follows CSS
  Syntax 3 §4.3.4 — `url(` before a quote is a function, otherwise §4.3.6 consumes a `Token::Url` with
  the raw text (escapes decoded), or a `Token::BadUrl` (§4.3.14) for whitespace inside, a quote, a `(`
  or a non-printable; `Token` is `#[non_exhaustive]`, so both are additions. `image_text` takes the url
  token and the quoted function; `join_components` lost its sign special case. Found: custom
  properties had two setting paths (rdom-css's block parser and `set_parsed`); both now go through
  `property_dispatch::set_custom`, which rejects a bad-url value (CSS Variables 1 §2.1). `token.rs`
  past 600 lines with the url tests: its tests moved to `token_tests.rs` (production 450). Red:
  `kept_background_text_reads_back_like_a_browser` (`linear-gradient( red , blue )`) and
  `custom_property_text_reads_back_without_stray_spaces` (`… ) - 1px url( x . png )`), both through
  CSSOM `getPropertyValue` / `cssText`; green after, with the tokenizer's url tests (render → re-tokenize
  round trip) and `custom_property_rejects_a_bad_url`. Changed expectations: rdom-css
  `colors::{var_simple, var_with_fallbacks_is_kept_for_the_cascade, border_color_var}` pinned the
  stray spaces (`var( --accent , red )`); they read `var(--accent, red)` now. No showcase snapshot
  changes.
- 2026-10-06 — C4G-SEALED: decided once in DESIGN ("Which rdom-tui traits a consumer implements"):
  call-only extension traits are sealed, injection points are not. Real list, from a grep of
  `pub trait` in rdom-tui: sealed now — `LayoutExt`, `PaintExt`, `HitTestExt`, `TuiDispatchExt`,
  `TuiDocAccessors`, `TuiNodeExt`, `TuiNodeMutExt`, `TuiAccessors`, `TuiAccessorsMut`, `TuiTimers`
  (the gate's list plus the last four; `CascadeExt` already was); implementable — `Backend`,
  `Clipboard`, `UrlOpener`. One seal for all: `crate::sealed::Sealed` (CascadeExt's private module
  moved there), implemented for `Dom<TuiExt>`, `NodeRef<TuiExt>`, `NodeMut<TuiExt>` and
  `EventCtx<TuiExt>`. Every one of the ten was publicly implementable, so CHANGELOG Breaking. Red:
  `src/sealed/doctests.md`, ten `compile_fail` doctests, each a full outside implementation of one
  trait (generated from the compiler's missing-item list, so a missing method cannot be what fails —
  stable rustdoc does not check a `compile_fail` error code), all ten compiled ("FAILED"); green
  after the supertraits. No behaviour change.
- 2026-10-06 — C4G-DOCS: docs only. CSS-COVERAGE — the `border-style` row no longer claims `rounded`
  rounds (it is `solid` there; only `border: rounded` sets a radius); §5's nine Phase 4 rows annotated
  (*Shipped* C4-BORDER-SHORTHAND / -SIDES / -WIDTH / C4-RADIUS / C4-SHADOW, `border-spacing` *partly*)
  and its count restated (133 as audited: 27 shipped, 1 partly, 105 open); §2 item 10 struck
  (`rgb(0 0 0 / 50%)` shipped with C3-RGB); §1 recounted from the §3 tables — the module counts
  stand (104 / 29 / 125 / 49), the headline's property count is 90, not 70. DIVERGENCES — the
  `border: rounded` entry says a later `border` keeps the radius (`.card.flat { border: solid }`
  stays rounded, an author's `dialog { border: 1px solid #ccc }` keeps the UA radius;
  `border-radius: 0` squares). CHANGELOG — Breaking (rdom-style) gains the two behaviour changes
  (the `border` shorthands reset color and width; the `border: rounded` radius survives a later
  `border`), the C4-BORDER-SIDES corner bullet ranks width first, and the input-reader bullet no
  longer cites the `pub(crate)` `runtime::input` (nor does the rdom-tui README). DESIGN — the
  `#[non_exhaustive]` section classifies every new Phase 4 public type: the border / background
  values and the parser's shorthand records are closed data beside `FlexShorthand`, `Sides` /
  `Corners` geometry; decided `PaintLength` stays closed (each form resolves differently into a
  weight, a corner or an offset — a painter meeting an unknown one would guess). rdom-style README —
  `border-spacing` listed, and the `Value` row describes `Value<T>`, the CSS-wide-keyword wrapper.
- 2026-10-06 — C4G-EDGE-TESTS: `tests/integration/css_phase4_gates.rs`, one test per edge case.
  (1) An opaque side meeting a translucent one at a corner: green on first run — the corner goes to
  the opaque horizontal side, whole, and the translucent left / right blend on their own cells only
  (C4G-BORDER-COST's ring strips leave the corners to the opaque pass). (2) `border-width: -1px`
  (and `-1`, `-0.5em`, inside `border` / `border-left`) is rejected, `calc(-1px)` clamps to 0: green
  (characterisation). (3) A percentage radius: `50%` / `25%` / `50% / 25%` of a 2 × 2 border box
  round; against a zero-size border box nothing is drawn, no panic, no NaN — green. Found while
  writing it: `width: 0; border: solid` draws nothing, because rdom sizes as `border-box` without
  flooring the box at its border and padding (CSS Box Sizing 3 §3.1 floors the content box at zero);
  layout, so not fixed here — written into DIVERGENCES §2's border-box entry, to be fixed with
  C5-BOX-SIZING. (4) `border: var(--b)` with `--b: rounded` (and `1px rounded red`): the radius
  arrives — the substituted declaration goes through the same `border` set arm, side effect
  included; green. (5) Decided: `dashed` / `dotted` draw dash glyphs — `glyphs::dash_glyph` on a
  straight run (N + S or E + W, one weight): `dashed` the double dash `╌╎` (heavy `╍╏`), `dotted`
  the finer triple dash `┄┆` (heavy `┅┇`); corners, junctions, stubs and weight mixes have no
  dashed glyph and stay solid (so a rounded dashed ring is `╭╌╌╌╮`). Red:
  `dashed_and_dotted_draw_dash_glyphs` drew `┌───┐`; green after, with
  `dashed_and_dotted_runs_pick_the_dash_glyphs` (code points, and the `None` cases). DIVERGENCES
  §2 border-style entry and the coverage row updated; CHANGELOG Changed. No showcase snapshot
  changed: no demo or UA rule uses `dashed` / `dotted`.
- 2026-10-06 — Phase 4 closed: 8 items (C4-SPACING partial — layout with C13-TFC) + 19 gate fixes (batch A 11, batch B 8). Gate-fix re-review folded into the Phase 5 gate. Carried to Phase 5: a box smaller than its border and padding is not floored at them (found by C4G-EDGE-TESTS, DIVERGENCES §2) — C5-BOX-SIZING.
- 2026-10-07 — C5-SPLIT: no behaviour change. `accessors/mod.rs` (676) → `mod.rs` (51: module docs,
  the module map, re-exports), `read_api.rs` (378: `TuiAccessors`, `DomRect`) and `write_api.rs` (261:
  `TuiAccessorsMut`) beside the existing impl files. `runtime/timers.rs` (1153: 603 code, 550 tests) →
  `runtime/timers/{mod (182: shared handle, current-scheduler guard, `TimerId`, `TimerCtx`),
  scheduler (262: the queues), pump (107: the drains and the microtask checkpoint), ext (86:
  `TuiTimers`), tests (550)}`. Public paths unchanged (`runtime::timers::{TimerCtx, TimerId,
  TuiTimers}`, `accessors::*`). Every test passes unchanged.
- 2026-10-07 — C5-BOX-SIZING: `box-sizing: content-box | border-box` (CSS UI 3 §3.1, now CSS Sizing 3
  "Box Edges for Sizing"), initial `content-box`, not inherited (`BoxSizing`, a closed keyword enum;
  `TuiStyle` / `ComputedStyle::box_sizing`, `ImportantMask::BOX_SIZING`, builder, node setter and
  accessor, root and prelude re-exports). Decided — one conversion point: `render/layout_pass/
  box_sizing.rs::Sizer` (per axis: the box-sizing and the padding + border, padding percentages
  against the containing block's width) turns a declared size into the border box layout stores
  (`outer`: + chrome under `content-box`, `max(size, chrome)` under `border-box`), a bound into the
  content box `auto` heights clamp (`inner`), and floors any used border box at the chrome (`floor`).
  Every size-reading site goes through it: block width / height / auto height, flex main sizes, the
  auto minimum (its floor is at least the chrome, so a scroll container shrunk by flex keeps its
  border), the cross size, `aspect-ratio` (Sizing 4 §5.1: the ratio applies to the box `box-sizing`
  names — content box for `content-box` or `auto && <ratio>`), positioned boxes and pseudo-elements,
  and intrinsic contributions (`Fixed` short-circuit, `wrapped_rows`). A table's used column width is
  already a border box and is not converted. Found: `auto_height` already clamped `min-height` /
  `max-height` as content sizes while block width clamped them as border sizes — both now follow the
  property. UA, matching the browser: the HTML rendering section's rule (`input:is([type=radio],
  [type=checkbox], [type=reset], [type=button], [type=submit], [type=color], [type=search]), select,
  button { box-sizing: border-box }`) plus `meter` / `progress` (Chromium `html.css`) is one new
  11-selector UA rule (UA count 150 → 161); text inputs and textareas stay `content-box` (22 columns
  with the UA padding — browser-faithful); `hr` is `height: 0` (one row, its border, under either
  sizing). Found: rdom rejects a bare pseudo-element (`::before` for `*::before`, Selectors 4 §5.2),
  so the common `*, ::before, ::after` reset drops its whole rule — the migration and fixtures use
  `*, *::before, *::after`; recorded in DIVERGENCES §3 under C10-PSEUDO-CHAINS. Red: the rdom-style
  dispatch tests failed to compile (`BoxSizing`, `box_sizing` missing) and every
  `css_phase5::box_sizing` sheet failed `from_css_strict` (`box-sizing` unknown); green after, with
  `layout_dirty_flag_reacts_to_box_sizing` and the `Sizer` unit tests. Changed expectations: the C4G
  pin `a_percentage_radius_against_a_zero_size_box` asserted the old bug (a zero-size bordered box
  drew nothing) — it now asserts the floored 2-cell boxes, rounded; chrome-geometry unit tests
  (`content_layout_insets_by_padding` / `_by_border`, `nested_containers_lay_out_independently`,
  `percent_padding_resolves_against_the_containing_block_width`) now assert content-box geometry;
  `textarea_wraps_long_input_and_enter_inserts_newline` sizes its textarea's content box (14, was 16
  as a border box); `css_values` gives `.c` (the `auto && <ratio>` case) and the percent-padding item
  `box-sizing: border-box` so their arithmetic stands; the UA user-select tests no longer assume one
  rule per selector. Fixtures written against border-box sizing (the border / collapse / paint unit
  tests, `css_phase4*`, `border_model_contract`, the collapse and padding-box files) declare it — the
  integration files through `common::border_box`. Showcase: the shell declares the reset and eight
  demos the same reset scoped to their root class (demo sheets stay class-scoped, `registry` test);
  rdom-tui's `parse_and_render` example declares it too; `rdom-showcase/src/shell.rs` (858 lines, touched)
  split: `shell/mod.rs` (536, the chrome builder) and `shell/base_css.rs` (333, `BASE_CSS`); one snapshot changed — `raf_progress`: its
  `.track { height: 1; border: solid }` drew only a top border under the unfloored sizing (the bar had
  no row); floored it is 2 rows, so the demo now says `height: 3` and the snapshot shows the full
  track with its content row. DIVERGENCES §2 "Boxes size as border-box" and §3's `box-sizing` line
  removed.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): CHANGELOG pointer (condensed out of the
  Breaking bullet): two changes reach a box even under the `*, ::before, ::after { box-sizing:
  border-box }` reset — a used border box is floored at its padding plus border (`box-sizing:
  border-box; height: 1; border: solid` takes 2 rows, not 1), and `min-height` / `max-height` on an
  `auto` height clamp the size `box-sizing` names, under `border-box` the border box (0.5 clamped
  the content height: `min-height: 5; border: solid` was 7 rows, now 5). Migration for those: size
  the box by its content (`content-box`), or add the padding and border to the bound.
- 2026-10-07 — C5-INTRINSIC: `min-content | max-content | fit-content | fit-content(<length-percentage
  [0,∞]>)` (CSS Sizing 3 §3.1–§3.3) on `width` / `height` / `min-*` / `max-*`: `IntrinsicSize`, a
  closed enum (the limit a `CalcExpr` — `Length` for cells; viewport units absolutized at computed
  time like the other lengths), as `Size` / `MinSize` / `MaxSize::Intrinsic` (breaking for exhaustive
  matches); `cells()` is `None` for it. Decided — one door for declared sizes:
  `layout_pass/intrinsic/keywords.rs::Keywords` (box, axis, measuring budget, containing-block width
  and the C5-BOX-SIZING `Sizer`) answers `size` / `min` / `max` as border boxes — a length through the
  sizer, a keyword measured: inline axis `min-content` = `content_min_size`, `max-content` = the new
  `content_max_size` (both `ContentOnly`, so a box's own declared width never short-circuits its
  keyword), `fit-content` = `min(max, max(min, stretch-fit))`, `fit-content(l)` = `min(max, max(min,
  outer(l)))` (the limit is a size, so `box-sizing` applies; an indefinite percentage limit is
  `max-content`); block axis — every keyword is the content height at the box's width ("equivalent to
  its automatic size", §3.1): `height: <kw>` takes the `auto` paths (block height, `auto_height`'s
  gate, flex column main size, indefinite for gaps / percentages / collapse-through), while a keyword
  `min-height` / `max-height` is that content height (so `min-height: min-content` lifts a too-short
  fixed height). Sites: block width (`fit-content` against the space the margins leave; `dom` / `id`
  now threaded into `resolve_block_width` / `block_content_width`), block height bounds, flex main
  sizes and bounds (`fit-content` against the container's main size), the flex auto-minimum's
  specified-size suggestion, flex cross sizes (a keyword never stretches, Flexbox §9.4), positioned
  boxes (`resolve_size_axis`'s content closure takes the keyword and the stretch-fit span), positioned
  pseudo-elements (every keyword is the content string's width — DIVERGENCES §2), intrinsic
  contributions (a child's inline keyword contributes its own min- / max-content; `fit-content`
  follows the measurement) and `wrapped_rows`. Keyword ↔ length transitions snap (discrete), as
  before for non-`Fixed` sizes. Not done: `stretch` (CSS Sizing 4; not in this item's row, though
  DIVERGENCES §3 listed it with C5-INTRINSIC) — now its own unscheduled §3 line; coverage `width` /
  `height` stays *Partial* for it, `min-*` / `max-*` are *Supported*. Red: the dispatch tests failed
  to compile (`IntrinsicSize`, the variants) and all nine `css_phase5::intrinsic` tests failed
  `from_css_strict` ("valid declaration"); green after, with `keywords_from_rust_through_the_prelude`.
  No existing expectation changed; no snapshot changed. Split (`block/mod.rs` was 739 lines and is
  touched): `block/place.rs` (139, `BlockPlace` / `lay_out_block_child`) and `block/runs.rs` (117, run
  partitioning); `mod.rs` 501.
- 2026-10-07 — C5-POS-MINMAX (found by C5-INTRINSIC, a fix between items): an absolutely positioned
  box's placed width / height ignored `min-*` / `max-*` (CSS 2.1 §10.4 / §10.7 apply to it), as did a
  positioned pseudo-element's. `compute_placed_rect` now clamps the tentative width by `max-width`, then
  `min-width` (through `Keywords`, so `box-sizing` and the keywords apply), before the height is
  measured at it, then the height the same way; pseudo-elements clamp through their `Sizer`, a keyword
  bound being the content string's size (DIVERGENCES §2). Red: `positioned_boxes_honour_min_and_max`
  gave (10, 1) for `width: 10; max-width: 4; height: 1; min-height: 3`; green after, with
  `a_positioned_pseudo_honours_min_and_max`. No existing expectation changed.
- 2026-10-07 — C5-MARGIN-TRIM: `margin-trim: none | [block || inline] | [block-start ||
  inline-start || block-end || inline-end]` (CSS Box 4 §3; the brief's seven single keywords plus the
  grammar's combinations), not inherited: `MarginTrim` (four logical bools, closed data, `NONE` /
  `BLOCK` / `INLINE`), `parse_margin_trim` (each keyword once, the axis and side forms not mixed),
  serialized in the shortest canonical form (`block`, `inline`, `block inline`, else the sides in the
  grammar's order). Layout: `layout_pass/margin_trim.rs` maps the logical sides onto the container's
  physical edges in one place (`trimmed_edges`, horizontal-tb ltr — C5-WRITING's `direction` lands
  there) and to flex axes (`FlexTrim`). Decided — clean on both: block containers trim the block axis
  only (§3: the inline values do not apply to them); the first (last) block child adjoining a trimmed
  block-start (block-end) edge contributes no margin (`suppress_*_margin`), and
  `parent_collapses_*_with_*_child` is false on a trimmed edge, so nothing collapses out through the
  container. Flex containers (single-line): the first (last) item's main-start (main-end) margin and
  every item's cross-start / cross-end margins (§3.2; `collect_main_axis_items` and `place_cross`).
  Intrinsic contributions drop the same margins. Not done: a collapsed-through empty edge child keeps
  its other margin (DIVERGENCES §2); grid with C7. Red: the dispatch test failed to compile
  (`MarginTrim`, `margin_trim`) and five of the six `css_phase5::margin_trim` layout tests failed
  (e.g. `(3, 6, 10)` for `(1, 4, 8)`; the sixth, block-container `inline` being a no-op, passed as
  it should); green after. Test fixed while writing: `block-start inline` mixes the axis and side
  forms, which the grammar rejects — the column test says `block-start inline-start inline-end`.
- 2026-10-07 — C5-CONTAIN-SIZE (partial — used with C14 contain): `contain-intrinsic-size`,
  `contain-intrinsic-width` / `-height` and the logical `-inline-size` / `-block-size` (CSS Sizing 4
  §6.1): `auto? [ none | <length [0,∞]> ]` (no percentages — the grammar is `<length>`), the shorthand
  one or two (width, height). `ContainIntrinsicSize { auto, length: Option<CalcExpr> }` (cells as
  `CalcExpr::Length`; viewport units folded to cells at computed time); two physical fields. Decided:
  the logical longhands map onto the physical fields at parse time (`fields_of` names them the same
  fields) — exact in rdom, whose layout is horizontal-tb only (C5-WRITING computes the vertical modes
  but lays them out horizontally), so CSS Logical 1 §4's "later declaration wins" holds by storage.
  The dispatch arms live in `property_dispatch/contain.rs` (set.rs / serialize.rs stay under the bar).
  Not used in layout: rdom has no size containment yet (`contain` is C14-CONTAIN), so the computed
  values wait there; not in `layout_differs` until then. Red: the dispatch tests failed to compile
  (`ContainIntrinsicSize`, the fields); the cascade tests were written with them and first ran after
  the cascade wiring — `contain_intrinsic_size_cascades` then failed on `25vw` (0, not 10: the shared
  `css_phase5::lay_out` cascaded before any viewport was set), fixed in the helper (viewport first, as
  C2-VIEWPORT documents), not in production code.
- 2026-10-07 — C5-WRITING (done before C5-LOGICAL, which maps the inline sides by `direction`):
  `direction: ltr | rtl` and `writing-mode: horizontal-tb | vertical-rl | vertical-lr | sideways-rl |
  sideways-lr` (CSS Writing Modes 4 §2.1 / §3.1), both inherited; `direction` stays out of `all` (CSS
  Cascade 4 §3.2 — it was already in `ALL_EXCLUDES`). Rust names `text_direction` / `TextDirection`
  because `ComputedStyle::direction` / `Direction` are `flex-direction` (renaming those is a separate,
  wider break; not done). UA: `[dir=ltr]` / `[dir=rtl]` set `direction` (HTML rendering, "Bidirectional
  text"; `dir` is already an ASCII case-insensitive attribute in rdom-core's matcher; `dir=auto` needs
  the first strong character — no). Layout under `rtl`: (1) lines start at the right edge
  (`render/inline/align.rs`, a post-pass over the packed lines, so paint / hit-test / caret follow) —
  decided against the brief's "inline boxes flow right to left": without the bidi algorithm,
  reversing boxes would print `world hello` for `hello world`, which no browser does; the browser
  result for left-to-right text in an `rtl` paragraph is the right-aligned line in logical order,
  which is what rdom draws (DIVERGENCES §1); (2) an over-constrained block drops its left margin (CSS
  2.1 §10.3.3, the parent's `direction`); (3) flex containers lay their items out in a mirrored frame
  (a row's main-start / a column's cross-start margin is the right one) and flip the x positions back
  (`placement::mirror_x`); (4) `margin-trim`'s inline sides follow the direction
  (`margin_trim::trimmed_edges` / the logical `FlexTrim`, intrinsic sums included); (5) relative and
  over-constrained absolute boxes keep `right` (§9.4.3 / §10.3.7 — reading the box's own direction,
  DIVERGENCES §2; the static position stays left); (6) the vertical scrollbar sits on the left
  (`gutter::bar_on_left` / `vertical_bar_column`, used by the gutter reservation, the paint and the
  hit-test; the corner cell moves to the bottom-left) — Chromium and Gecko both put an `rtl` box's
  scrollbar on its inline-start side. Not done: `text-align: start / end` (no `text-align` until
  C9-TEXT-ALIGN — lines are `start`-aligned, which `rtl` now honours), `:dir()` (Phase 11: coverage row
  N/A → Missing), vertical writing modes (compute, lay out horizontally — DIVERGENCES §1; coverage
  `writing-mode` and `direction` rows *Partial*). Red: the dispatch tests failed to compile
  (`TextDirection`, `WritingMode`, the fields); the layout tests in `css_phase5/writing.rs` were written
  first but first ran after the implementation (green). Changed expectation:
  `all_shorthand_sets_every_property_in_the_table` iterated every table name — `direction` is in the
  table now and, per Cascade 4 §3.2, not in `all`, so the test skips it and asserts it stays unset.
  `cascade_inherits_exactly_the_style_crates_inherited_set` gains probes for `direction` and
  `writing-mode` (and the non-inherited `box-sizing` / `margin-trim` of the earlier items).
- 2026-10-07 — C5-LOGICAL (after C5-WRITING): the 52 flow-relative properties of CSS Logical 1
  §2–§6 (`property_dispatch/logical.rs`, its `NAMES` appended to `property_names()`, which is now
  `PROPERTY_NAMES` + those). Decided — no duplicated storage, order preserved: (1) the block-axis
  properties and the sizes map onto their physical twins *when declared* (`set_mapped` → `set_parsed`
  of the physical name; a `Pair` takes one or two components, a `Both` the whole value) — exact in
  rdom, whose layout is `horizontal-tb` only (C5-WRITING), so CSS Logical 1 §4's "later declaration
  wins" holds by storage, within a block and across the cascade; (2) the inline-axis properties and
  the corner radii depend on `direction`, known only per element, so they ride the order-preserving
  replay `var()` already has: the declaration (value checked when declared; a CSS-wide keyword
  lower-cased) goes on the block's `pending` list as `directional` and every later declaration of the
  block follows it there; the cascade replays the list with `SubstitutionContext::direction`
  (`set_parsed_in` / `set_unset_in`). The element's own `direction` decides, and the ladder is what
  computes it: `compute_element_style` / the pseudo-element path run the ladder with the inherited
  direction and, when a matched block holds a directional declaration and the element's computed
  direction differs, once more with it (a logical property cannot change `direction`, so two runs
  suffice; elements without one pay one scan). `fields_of` for a flow-relative name is its physical
  properties' fields (both sides for an inline-axis one, built once from the mapping), which routes
  `!important`, removal and the CSS-wide keywords; the two edges this leaves (an important inline-axis
  declaration marks both sides important in its block; CSSOM `removeProperty` clears both) are in
  DIVERGENCES §2. CSSOM reads an inline-axis property back as written (from `pending`), a block-axis
  one from its physical twin. Found and fixed: the `pending` replay started from an empty `margin` /
  `padding`, so a replayed side longhand dropped the block's shorthand sides — `margin: 1;
  margin-left: var(--x)` came out `0 0 0 x` (now seeded from the block; regression test
  `a_substituted_longhand_keeps_its_shorthands_other_sides`). Not done: `overflow-block` /
  `overflow-inline` (C8-OVERFLOW-CLIP), the logical keywords of other properties (`text-align: start`
  …, with their properties). Red: `logical_tests.rs` (rdom-style) and `css_phase5/logical.rs` were
  written first — the former did not compile (`SubstitutionContext::with_direction`) — and both first
  ran after the implementation; their round-trip rows also needed `canonical_values` entries
  (`property_names_matches_canonical_values_table` failed until added). Changed expectations: none
  beyond the canonical table's new rows. Found by the workspace run: CSSOM `cssText` / `length` /
  `item()` enumerate `property_names()`, so a shared-storage name listed `width` twice (`width: 16;
  inline-size: 16;` in `set_width_from_a_listener_lays_out_on_the_next_frame`, and the `dom_api`
  demo's snapshot) — `property_dispatch::is_storage_alias` keeps the block-axis flow-relative names out
  of those enumerations (listed under the physical name); test `cssom_lists_shared_storage_once`. No
  snapshot or expectation changed.
  The batch's rustdoc gate (run before this last commit) found a link C5-SPLIT broke —
  `read_api.rs`'s `[`TuiAccessorsMut::style_mut`]` no longer in scope after the move — fixed here with
  a `super::` path, along with a link from `set_parsed`'s docs to the crate-private `set_parsed_in`.
- 2026-10-07 — Phase 5 gates (with the C4G re-review: all 19 at the root except C4G-SHADOW-ORDER).
  Architect: 2 blocking — flex items paint atomically but their shadows were queued as step-4 block
  shadows, so a shadow lands under an earlier item's text (a regression from C4G-SHADOW-ORDER); `rtl`
  scrolling moves mirrored content the wrong way, so left overflow is unreachable. API: 1 blocking — a
  bare / descendant pseudo-element (`*, ::before, ::after`, `div ::before`) is dropped or mis-matched,
  and the content-box default makes that reset the migration path. Non-blocking: `ESC X` / `_` / `^`
  framing swallows typing after a late frame; logical properties re-parse per element and the
  direction re-run repeats the ladder; intrinsic contributions ignore `min-*` / `max-*` / percentages
  (inline-block atoms, table cell widths skip the `Sizer`); the tokenizer integer clamp corrupts
  custom properties; C8-POS-MINMAX half done (relative pseudo with both insets); `fit-content`
  measures twice; red-less writing / logical tests; `pixel_math` rides on `ch`; inline-block atoms
  paint no shadow; content-box migration note incomplete (floor, `min-height` as border size, no
  upgrade callout, no README line); `TextDirection` / `WritingMode` / `MarginTrim` /
  `ContainIntrinsicSize` not re-exported, no Phase 5 prelude test; root `backend` / `parse` /
  `property_dispatch` modules misleading; prelude grown into a migration surface; custom properties
  re-rendered, kept text not lowercased; CSSOM reads of inline-axis longhands / priority wrong; logical
  `!important` marking both sides gives a wrong cascade result; setter docs use Rust field names;
  missing `From<IntrinsicSize>`, `fit_content` constructor, `intersects`, node setters, radius shape
  mismatch; showcase snapshots render without the shell reset and ignore background; stale counts
  and DESIGN lists; DIVERGENCES spread contradiction. Accepted: tint_bg cost, unitless fractions,
  rtl margin / positioned / no-bidi decisions, sealed-doctest fragility, rustdoc link broken on
  b2e2bcd..34a9cfb (history not rewritten). Fix all as `C5G-*`, two batches (A correctness, B API
  and docs).
- 2026-10-07 — C5G-BARE-PSEUDO (gate fix, blocking): a pseudo-element with no compound before it
  attaches to the implicit `*` (Selectors 4 §5.2: a compound without a type selector has an implied
  universal selector). `extract_pseudo_suffix` returns a `Cow` core and, when the text before the
  pseudo-element is empty or ends in whitespace or a combinator (`>` `+` `~`, not an escaped one),
  appends `*`: `::before` → `*::before`, `div ::before` → `div *::before`, `div > ::after` →
  `div > *::after`. The nine suffixes became one table. Decided: done in the suffix stripper (the one
  place a pseudo-element is split off), for every supported pseudo-element, not only `::before` /
  `::after`. Red: `extract_bare_pseudo_attaches_to_an_implicit_universal` and
  `the_bare_pseudo_reset_list_parses` (rdom-style) failed with "`::before` requires a host selector";
  `css_phase5/bare_pseudo.rs` failed — the reset list and `.a > ::after` failed the strict parse,
  `.a ::before` painted `>AB` (the `div`'s own `::before`) instead of `A>B`; green after. Changed
  expectations: the four `extract_rejects_bare_*` assertions (the old rejection) became the new
  test. DIVERGENCES §3's line removed; the C5-BOX-SIZING migration hint names the bare reset again.
- 2026-10-07 — C5G-FLEX-SHADOW (gate fix, blocking): flex items paint exactly as inline blocks
  (CSS Flexbox §5.4), i.e. atomically, as if they created a stacking context whose positioned
  descendants still belong to the parent (CSS 2.1 Appendix E, 7.2.1.4.1.1). C4G-SHADOW-ORDER had
  queued a flex item's shadow as a step-4 block shadow, so it painted under an earlier item's text.
  New `stacking::paints_atomically` (an inline block, an inline flex container, or a child of a flex
  container — through a fragment, the element above it): `collect_layers` no longer queues an atomic
  box's shadow or its in-flow descendants' (`unit: Option<usize>`, `None` below an atom; positioned
  descendants still join the context's layers, and a `z-index: auto` one starts its own unit as
  before); `paint_plain` paints an atomic box with `Shadows::Whole`, then its own background phase —
  `stacking::for_each_atom_shadow`, a walk of its in-flow boxes that stops at positioned boxes,
  contexts and nested atoms (each box visited by one unit; no allocation) — then its content.
  Decided: the document root's children (laid out as flex items, DIVERGENCES §1) stay block boxes for
  paint, as a browser's `<body>` children are — `an_outer_shadow_paints_under_earlier_siblings_text`
  pins that. No grid items yet (Phase 7). Inline-block atoms inside an inline formatting context paint
  through `paint_ifc`, which paints no box shadow and no block descendants — their own shadow is
  C5G-INLINE-BLOCK-SHADOW. Red: `css_phase5/atom_shadows.rs` — the flex item's shadow and the shadow
  of a block inside a flex item both left `aaaab` (glyphs kept under a tinted shade) where the
  browser's `aa  b` is expected; green after. A third test (a later block's shadow inside one flex
  item stays under that item's earlier text) passed before and after. No expectation changed.
- 2026-10-07 — C5G-INLINE-BLOCK-SHADOW (gate fix): an inline block in a line paints its outer
  `box-shadow` at its turn, as an atomic box (CSS 2.1 Appendix E, 7.2.1.4.1.1; consistent with
  C5G-FLEX-SHADOW: `Shadows::Whole`, over the line content painted before it; a translucent shade
  composites once and keeps the glyphs). `paint_inline_layout`'s atom branch paints it before the
  atom's content — unless the atom's parent's content paint also reaches it as a box
  (`paints_child_box`: an inline block beside bare text, laid out through an anonymous box, which
  `recurse_children` → `paint_plain` already paints with its shadows), so it paints once. Found while
  writing the tests: an inline block in a line paints no background, border or padding at all, and
  beside bare text it is painted twice (box and line content disagree with a border) — recorded as
  TECH_DEBT `ATOM-BOX-1` (one owner: lay the atom out at its block size in the line, C9-VERTICAL-ALIGN),
  not fixed here. Red: `an_inline_blocks_shadow_paints_at_its_turn` (`aaaab`, no shade) and
  `a_translucent_inline_block_shadow_keeps_the_glyphs_beneath` (`Reset` under the shade) failed; the
  first draft without the `paints_child_box` check made
  `an_inline_blocks_translucent_shadow_composites_once_beside_bare_text` fail (`(192, 0, 63)`, two
  composites, against `(128, 0, 127)`); green after. The tests put an inline element before the atom:
  with bare text the block lays the run out through an anonymous box, whose text paints after the
  block children (`paint_content`), so the shade lands under it there — part of `ATOM-BOX-1`.
- 2026-10-07 — C5G-RTL-SCROLL (gate fix, blocking): an `rtl` scroll container scrolls from its right
  edge. CSSOM View §4 puts the scrolling area origin at a box's inline-start edge, and `scrollLeft`
  is measured from it: 0 at the right edge of an `rtl` box, negative towards its left overflow
  (`-(scrollWidth − clientWidth)` at the far left) — the current spec and every current browser.
  Decided: `TuiExt::scroll_x` *is* `scrollLeft` — an `i32` (was `usize`, Breaking — rdom-tui; the
  `TuiNodeMutExt::set_scroll` raw setter takes `x: i32`) — rather than a distance from the origin that
  every physical consumer would flip: a larger value shows content further right in both directions,
  so layout (`x − scroll_x`, `placement::mirror_x` unchanged — it was right once the offset may be
  negative), the extent (`rect.x + scroll_x`), wheel and arrow keys (`± 1`), track paging and thumb
  drag (a delta) need no direction test. Only the range is direction-aware:
  `layout_pass::scroll_x_bounds` (`0 ..= overflow` or `-overflow ..= 0`), used by the layout clamp
  (`clamp_scroll_offset`), the new `scroll::ScrollBounds` / `scroll_bounds` (replacing `max_offsets`;
  the one clamp of `set_scroll`, `write_offsets`, `perform_scroll`, the wheel — whose private
  `apply_scroll` duplicate is gone — and `scrollIntoView`), and `scroll_x_from_area_start` /
  `geometry::offset_from_area_start` (`scroll_x − min`), the physical position the thumb is painted
  and hit-tested at and the autoscroll bands measure — so the thumb starts at the right. Caret reveal's
  `ClampTo::NextLayout` bounds only the origin side, which is `≤ 0` for an `rtl` box.
  `scrollIntoView`'s `inline: start` / `end` align the right / left edge in an `rtl` container (CSS
  Writing Modes 4 §2.1). The smooth-scroll and painted / laid-out bookkeeping carry the signed value.
  Not covered: a line wider than its `rtl` inline formatting context (`render/inline/align.rs`, not
  examined here), `writing-mode` vertical scrolling (laid out horizontally). Red: `runtime/scrollbar/rtl_tests.rs` — every test but
  the initial-layout one failed (`scrollLeft` stuck at 0 for the API, wheel, keys and
  `scrollIntoView`; the drag reached `+6`; the thumb sat at the left; the over-wide block's `-6`
  unreachable); green after. `scroll_into_view_inline_start_is_the_right_edge` was confirmed red with
  the start / end swap disabled (`(-6, 0)` against `(-8, 2)`). No existing expectation changed.
- 2026-10-07 — C5G-STRING-INTRO (gate fix): rdom's input parser no longer frames PM (`ESC ^`) or SOS
  (`ESC X`) strings, and takes APC (`ESC _`) as a string only when `G` follows — the kitty graphics
  reply, the one APC a terminal sends. C4G-CSI-FRAMING had started all three on any string byte
  (ECMA-48 §5.6 lets a string hold any of them), so Alt+Shift+X then `hello⏎` — or the same bytes
  after a frame that came late (C4G-ESC-GRACE) — became an SOS string and the typing was swallowed.
  Decided: frame only what a terminal replies with (OSC: a digit; DCS: a parameter / intermediate
  byte; APC: `G`); `ESC ^` / `ESC X` are complete Alt keys at once (no prefix wait). Red:
  `alt_x_and_alt_caret_then_typing_are_keys` (no events: the typing was a string) and the new
  `ESC _ x` row of `dcs_and_apc_strings_are_consumed` failed; green after. Changed expectation: that
  test (was `dcs_apc_pm_sos_strings_are_consumed`) drops its PM / SOS rows and their prefix-wait rows,
  which asserted the behaviour removed here.
- 2026-10-07 — C5G-LOGICAL-COST (gate fix): an inline-axis flow-relative declaration put every later
  declaration of its block on `pending`, which the cascade replayed per element (a throwaway
  `TuiStyle`, the token re-parse, a `Vec` per element). Now a rule whose kept declarations need no
  substitution (`TuiStyle::needs_substitution`: no `var()` / `attr()`) carries their two
  direction-mapped forms, built once in `Stylesheet::rules_for` from `substituted_pending` with each
  direction (`TuiStyle::directional_overlays`, shared by the items of a selector list through an
  `Arc`; the `::placeholder` subset gets its own) — `Rule::directional_overlay(direction)`.
  `Declarations` carries the element's direction (`with(substituted, direction)`) and `rule_blocks`
  applies the rule's overlay after its block, exactly where the per-element replay went, so CSS
  Logical 1 §4's declaration order is unchanged; `Substituted::new` now substitutes only blocks that
  need it (and inline styles, which are per element anyway). Decided: built per rule at sheet build,
  not cached inside `TuiStyle` — its fields are public, so a cache there could go stale; a `Rule` is
  immutable once in a sheet. The direction re-run of the ladder stays (the element's own `direction`
  is what the ladder computes) and is capped by a `debug_assert!` at two runs (walk.rs, pseudo.rs).
  Red: `a_logical_block_cascades_at_the_cost_of_its_physical_twin` (counting allocator, warm second
  cascade) failed — 27 allocations against 24 for `margin-left`; green after (≤).
  `a_direction_change_reruns_the_ladder_once` (three ladder walks: the element twice, `::selection`)
  and `declaration_order_holds_under_rtl` pin the cap and the order (both passed before; regression
  guards). No expectation changed.
- 2026-10-07 — C5G-SIZING-SITES (gate fix): the remaining sizing sites go through the `Sizer` and the
  `min-*` / `max-*` clamp. (1) Intrinsic contributions (CSS Sizing 3 §5.2: the box's outer size with
  its preferred size, if definite, in place of the content, "and with its min and max sizes
  applied"): `intrinsic_size_inner`'s `BoxSize` mode is now `intrinsic/contribution.rs`
  (`box_contribution`) over the split-out `content_size` (the old body) — the table column width
  first (final), then an inline-axis keyword as before, a length / percentage / `calc()` through
  `Keywords::size` (so the `Sizer`) when definite (a percentage of a known containing block width; a
  height's or a cyclic one behaves as `auto`, §5.2.1), else the content; then `max-*` caps and
  `min-*` floors (CSS 2.1 §10.4), `fit-content` in a min / max being the min-content size in a
  min-content measurement and the max-content size otherwise. Decided: one door for every caller —
  the inline-block atom (`atomic_inline_block_intrinsic_width` already asks `intrinsic_size`, so
  `inline/mod.rs` is unchanged at 591 lines), the flex basis, the children's contributions. (2) A table
  cell's author width (`runtime/builtins/table`) is the box its `box-sizing` names: `cell_sizer` takes
  the cascaded style's `Sizer`, and before the first cascade (`App::build` runs the pass first) the UA
  cell's (`padding: 0 1`, `content-box`) with the inline `box-sizing`; content widths add that sizer's
  chrome. `box_sizing` is `pub(crate)` in `layout_pass` for it. Red: `css_phase5/sizing_sites.rs` —
  the button with `min-width: 10` was 6, the `max-width: 3` atom 6, the `50%` atom 3 (content), the
  column container of a `min-width: 7` item 2 — and the new table test (`width: 10` → 10, not 12)
  failed; green after. Changed expectations (the old border-box reading of a cell's width):
  `explicit_author_width_is_respected_not_overwritten` 20 → 22, and
  `colspan_excess_spreads_over_author_sized_columns_too` now pins column 0 with `width: 4` (6 with the
  padding) to keep its arithmetic. No snapshot changed.
- 2026-10-07 — C5G-INT-CLAMP-SITE (gate fix): C4G-NUMBER-RANGE clamped an integer literal to
  `i32::MAX` in the tokenizer, which custom properties — kept as tokens and re-rendered — passed on:
  `--n: 99999999999` read back as `2147483647`, and `calc(var(--n) / 1e9)` came out 2.147 instead of
  100. CSS Syntax 3 §4.3.12 gives the token its value and type flag; CSS Values 4 §5.1's clamp to the
  implementation's range belongs to the value's consumer. `Token::Number` holds an `i64` (Breaking —
  rdom-style; saturating past `i64`, the one remaining cap — a literal of 20+ digits), a dimension's
  number part its full value; the integer-typed consumers clamp: the unitless cell
  (`LengthPercentage::Integer`, new `numeric::clamp_i32`), counter values, `steps()`'s count (to `u32`);
  `numeric::integer` already returned `i64` for its callers to clamp. Decided: `i64` over `f64` — the
  type keeps integer patterns (`Token::Number(0)`) and exact values, and the brief allowed either.
  Red: `a_large_integer_survives_a_custom_property` (`Some("2147483647")` against
  `Some("99999999999")`); green after, with its `calc(var(--n) / 1e9)` = 100 half. Changed expectation:
  `oversized_integer_clamps_and_stays_integer` → `oversized_integer_keeps_its_value_and_stays_integer`
  (the token no longer clamps; a 23-digit literal saturates at `i64::MAX`; the dimension keeps
  `99999999999.0`). `box_shadow_takes_an_integer_past_the_range` gains the 23-digit and the
  `99999999999ch` cases — the box-shadow cases still parse and clamp (`PaintLength::Cells(i32::MAX)`),
  and `box_shadow_huge_lengths_do_not_overflow` (rdom-tui) passes unchanged.
- 2026-10-07 — C5G-REL-PSEUDO-INSETS (gate fix; completes C8-POS-MINMAX, row set to `done
  (C5-POS-MINMAX + C5G-REL-PSEUDO-INSETS)`): a `position: relative` pseudo-element with both insets
  stretched to their span — `compute_placed_rect` sized it like an absolute box
  (`resolve_size_axis` with the insets). CSS 2.1 §9.4.3: a relative box only moves; with both `left`
  and `right` the inline-start one wins (`left` under `ltr`, `right` under `rtl`), with both `top` and
  `bottom` `top`, and its size is its own. Now a relative pseudo is sized with `auto` insets, and
  shifted by `positioning::relative_offset`, extracted from the element path's
  `apply_relative_shift` — one rule for elements and pseudo-elements, the box's `direction` included;
  `axis::axis_position_relative_shift` (left always won) is gone. The element path was checked: it
  never sized by the insets and already kept `right` under `rtl`
  (`rtl_positioned_boxes_keep_the_right_inset`); `a_relative_element_with_both_insets_keeps_its_width`
  pins its width. Red:
  `a_relative_pseudo_with_both_insets_only_shifts` failed — `(10, 1, 8, 2)` (stretched both ways)
  against `(10, 1, 1, 1)`; green after, `(8, 1, 1, 1)` under `rtl`. No expectation changed.
- 2026-10-07 — C5G-ATOM-BOX (gate fix, closes TECH_DEBT `ATOM-BOX-1`): an inline block in a line is
  one atomic box, laid out and painted by its line. Layout (CSS 2.1 §10.8 / §10.8.1): line boxes have
  heights — `LineBox` gains `top` / `height` / `baseline`, `InlineFragment` `y` / `height` (Breaking —
  rdom-tui, the data model moved to `render/inline/boxes.rs` to keep `inline/mod.rs` under the bar);
  the packer measures each atom (`inline/vertical.rs`: its border-box height through the `Sizer`,
  its positive vertical margins, its baseline — the last row of its content, or its bottom margin
  edge with no content or clipping overflow) and settles each line: the baseline row is the most rows
  any atom has above its baseline, the height adds the most below; text and generated content sit on
  the baseline row. `atomic_placements` lays the atom out at its border box in the line (the IFC's
  atoms now in the *scrolled* content rect, as paint reads them). Paint (Appendix E 7.2.1.4.1.1): the
  line paints each atom at its turn through `stacking_walk::paint_line_atom` — the same in-flow path
  as a block child (shadows whole, background, inset shadows, border, its background phase, content;
  a stacking context as one) — and `recurse_children` / `paints_child_box` skip an inline block in a
  block container (`in_a_line`), so the anonymous-box path no longer paints it twice and the
  `painted_as_box` special case of C5G-INLINE-BLOCK-SHADOW is gone. Consumers moved from "line *i* is
  row *i*" to the line geometry: hit-testing (`line_at_row`, `LineBox::covers`), the caret
  (`text_row`; Up / Down step line box by line box), static positions, the opacity group's extent,
  the band paint clips to. Decided: whole-row baseline alignment with `vertical-align: baseline` only
  (C9-VERTICAL-ALIGN adds the rest); a negative vertical margin on an atom counts as zero (an atom
  above its line would cover the previous one); the baseline is the content's last row (DIVERGENCES
  §2, Layout). Red: `css_phase5/atom_box.rs` — inside text the bordered atom painted `aab  cc` on one
  row (no border, no background; `Rgb(0, 0, 255)` where the translucent fill should composite to
  `(128, 0, 127)`), padding gave `aab  cc`; beside bare text the box and the line content disagreed
  (`aab─┐cc` / `  │b│  `, padding `aabb cc` — painted twice), the atom was 1 row (`height: 1`, not
  3), the next line's `cc` was drawn under the box, and the atom's rows did not hit-test to it; green
  after, with `vertical.rs` / `boxes.rs` unit tests for the line arithmetic. No existing expectation
  or snapshot changed.
- 2026-10-07 — C5G-PSEUDO-ONLY (gate fix): an element whose only content is its `::before` /
  `::after` shows it (CSS 2.1 §12.1 — they are its first / last child boxes — and §9.2.1.1, an
  anonymous line box). Root cause, one idea in three places: "content" meant child nodes. Block
  layout (`layout_block_children`) returned a zero measurement for a box with no (in-flow) children
  before it ever formed runs; it now forms one pseudo-only run (`Run::pseudo_only`) when no child
  bears a line and the host has visible inline pseudo-elements, which the anonymous-box paint
  already draws. Margin collapse-through (`is_collapse_through_shape`) took such a box for empty and
  let the next sibling over it; a visible pseudo now counts as a line box. The intrinsic Column size
  of a childless box counted only padding and border (`pseudo_main` is Row-only); it now takes the
  rows the pseudos pack to (`wrapped_rows`), so a flex item or inline block measures them. Inline
  hosts (`<p>a<span class=b></span>c</p>`) were already right — the packer walks an inline box's
  pseudos — and are pinned as regression guards. Red: `css_phase5/pseudo_only.rs` — the block host
  with `::before`, `::after` and both painted `next` on row 0 (nothing of `<`/`>`), the flex item the
  same, `margin: 1 0` left the sibling at row 1 (`["      ", "next  ", …]`); green after. No
  expectation or snapshot changed.
- 2026-10-07 — C5G-MIGRATION-DOCS (gate fix, docs): the content-box Breaking note (rdom-tui) now names
  the two changes that reach a box even under the `border-box` reset — the padding-plus-border floor
  (`height: 1; border: solid` is 2 rows; points to the Fixed entry) and `min-height` / `max-height` on
  an `auto` height bounding the box `box-sizing` names (0.5 clamped the content height: checked
  against `v0.5.0`'s `resolve_auto_height`; `min-height: 5; border: solid` 7 rows → 5). New "Upgrading
  from 0.5" callout at the top of `[Unreleased]`: content-box and the floor, `border` resets, signed
  `scroll_x`, sealed traits, opaque `ImportantMask`, the typed values / new variants, line-box
  heights, `var()` / `attr()` on `pending`. rdom-tui README: the content-box default and the reset,
  under Stylesheets (it has no layout section). Root README: 163 UA rules (the count
  `ua::tests` pins; was 150). rdom-style README: the sizing keywords and `none`, `contain-intrinsic-*`,
  `margin-trim`, and a writing-modes / logical-properties group. Docs only — no test.
- 2026-10-07 — C5G-REEXPORTS-AND-ROOT (gate fix, API): the root re-exports `TextDirection`,
  `WritingMode`, `MarginTrim` and `ContainIntrinsicSize`; rdom-style's `backend`, `parse` and
  `property_dispatch` modules moved from the root to `rdom_tui::style::` (`style::backend` no longer
  sits beside the terminal `Backend` trait). The prelude went back to a typical app's set: removed
  `PaintLength`, `BorderWeight`, `BorderWidth`, `CornerStyle`, `BorderSpacing`, `Sides`, `Corners`,
  `VisualBox`, `RepeatStyle`, `BackgroundRepeat`, `BackgroundAttachment`, `FlexBasis`, `AspectRatio`,
  `ColorContext`, `ColorFunction`, `ColorSchemeList`, `SystemColor`, `ContentContext`,
  `CustomValue`, `CalcExpr`, `parse_color` and `resolve_tui_color` (all still at the root); kept the
  builders' values it had gained (`BorderRadius`, `BorderStyle`, `BoxShadow`, `BoxSizing`, `MinSize`,
  `MaxSize`, `IntrinsicSize`, `ColorScheme`) and `Viewport` (the argument of the prelude's
  `CascadeExt::set_viewport`; the rdom-tui README doctest that names it failed without it), and
  added `TextDirection`. No other in-tree code, example or README used a removed name. `tests/integration/prelude_migration.rs` is
  `migration_hints.rs` and compiles every hint with `use rdom_tui::*;` — module paths as `calc::…`
  and `style::parse::…` — plus two new groups: Phase 5 (`box_sizing` and the reset, the intrinsic
  keywords, `margin_trim` + `parse_margin_trim`, `contain_intrinsic_*`, `text_direction` /
  `writing_mode`, `set_box_sizing`) and batch A (`scroll_x` / `set_scroll` signed, `Token::Number`
  as `i64`, `LineBox` / `InlineFragment` rows). DESIGN §6 now says what the root and the prelude are
  for. Decided: no Breaking bullet for the move — the root `backend` / `parse` / `property_dispatch`
  re-exports and the prelude additions came with C4G-REEXPORTS, after 0.5.0, so no published version
  has them; the C4G-REEXPORTS Added bullet is rewritten to the new paths instead. Red: the rewritten
  test failed to compile — `cannot find parse / backend / property_dispatch in style` (10),
  `ContainIntrinsicSize` / `MarginTrim` / `TextDirection` / `WritingMode` not in scope (5); green
  after.
- 2026-10-07 — C5G-API-EDGES (gate fix, API): `From<IntrinsicSize>` for `Size` / `MinSize` / `MaxSize`
  and `IntrinsicSize::fit_content(cells)` / `fit_content_percent(p)` (the variant holds a `CalcExpr`,
  so one constructor per limit kind); `ImportantMask::intersects` (const, word-wise); `From<BorderRadius>`
  for `Corners<BorderRadius>` with `set_border_radius` and the `border_radius` builders taking
  `impl Into<Corners<BorderRadius>>` — what `border_radius()` returns; node setters / getters for
  `direction` (`set_text_direction`, `set_direction` staying `flex-direction`), `writing-mode` and
  `margin-trim`, like `box-sizing`'s. `setter!` takes the CSS property name first and its docs name it
  (`flex-direction`, `direction`, `font-weight`, `font-style`); `flow` — no CSS property of its own —
  has hand-written setters documented as `display`'s inner type. `ImportantMask::DIRECTION` is
  `FLEX_DIRECTION` (Breaking — rdom-style, the bit was in 0.5.0; hint given; the cascade's apply table
  follows). The `width` / `height` builders, node setters and getters say they measure the box
  `box-sizing` names. Red: `intrinsic_keywords_convert_into_every_size`,
  `intersects_and_the_flex_direction_bit`, `a_radius_converts_into_four_corners` (rdom-style) and
  `phase5_node_setters_round_trip` (rdom-tui) failed to compile — no `From<IntrinsicSize>`, no
  `fit_content` / `fit_content_percent` / `intersects` / `FLEX_DIRECTION`, no corner `From`, no
  `set_text_direction` / `text_direction` / `writing_mode` / `margin_trim`; green after. The doc
  changes have no test. Splits (files past the bar): the builder's sizing setters moved to
  `tui_style/builder/sizing.rs` (`builder/mod.rs` 615 → 514), and `layout/sizing.rs`'s tests to
  `layout/sizing_tests.rs` (609 → 537).
- 2026-10-07 — C5G-CSSOM-LOGICAL (gate fix): CSSOM reads of the inline-axis flow-relative properties
  follow CSSOM §6.6. `getPropertyValue`: a longhand reads the last kept declaration that sets it —
  its own or a shorthand's component — and a shorthand only when every longhand is set; the value is
  computed by replaying those declarations onto a scratch style under `ltr` and serializing the
  physical properties (`logical::serialize_inline_axis`, with `inline_longhands`: `margin-inline` →
  start + end, `border-inline-start` → its width / style / color, `border-inline-color` → the two
  sides' colors, …), so the text is what the physical twin serializes. With no declaration of any
  longhand it falls back to the fields (`all: unset` still reads `initial`). `getPropertyPriority`:
  the declaration's own `!important`, now recorded on the kept declaration —
  `PendingDeclaration::important`, set by the new `property_dispatch::set_important` (the one
  recording path: `rdom-css`'s block parser and the CSSOM `setProperty` call it) and read by
  `is_important` (a shorthand is important when every longhand's last declaration is). `cssText` /
  `length` list a set logical shorthand once (`listed_under_logical_shorthand`, as `padding`
  suppresses its sides). Decided: the physical sides' bits are still set for an important
  inline-axis declaration here — the cascade reads them until C5G-LOGICAL-IMPORTANT moves it to the
  per-declaration flag; that item adds the "a normal `margin-left` beside it is not important"
  assertions. Red: `inline_axis_reads_expand_shorthands` (`None` for `margin-inline-start` under
  `margin-inline: 1 2`, against `Some("1")`) and `cssom_reads_logical_longhands_and_priorities`
  (`""` against `"1"`) failed; `inline_axis_priority_is_the_declarations_own` failed to compile (no
  `set_important` / `is_important`); green after. Changed along the way: the first draft returned
  nothing for an undeclared inline-axis name, which broke `all_shorthand_sets_every_property_in_the_table`
  (`None` against `initial`) — the fallback to the fields above fixes it; no expectation changed.
- 2026-10-07 — C5G-LOGICAL-IMPORTANT (gate fix; margin / padding finished by C6-MARGIN-SIDES): an `!important`
  inline-axis declaration is important on the side it maps to only (CSS Cascade 4 §6.4: importance
  is per declaration; CSS Logical 1 §4). `set_important` no longer sets the physical bits for an
  inline-axis name — its flag on the kept declaration (C5G-CSSOM-LOGICAL) is its importance. The
  replay (`pending_overlay`) marks each replayed declaration's written fields
  (`property_dispatch::mapped_mask`: the physical targets for the direction) with its own priority,
  and splits a block's kept declarations into a normal and an important overlay
  (`substituted_pending_split`), so a field both write keeps both values for the two passes —
  `inset-inline-start: 3 !important; left: 1` gives `left: 3`. Built on C5G-LOGICAL-COST: a rule's
  prebuilt overlays are now `[[normal, important]; 2]` (`Rule::directional_overlay` returns the pair),
  the per-element substitution (`Substituted`) keeps pairs too, and `rule_blocks` / `inline_blocks`
  chain both after the block. An overlay leaves `margin` / `padding` unset unless a replayed
  declaration wrote them (it copied the block's every time). `removeProperty` of an inline-axis
  name (`logical::remove_inline_axis`) removes the kept declarations of its longhands and nothing
  physical — it took both sides' fields and bits — splitting a wider shorthand into its remaining
  longhands (values and priority kept); `remove` keeps the kept declarations while an inline-axis
  one remains (it cleared them when no `var()` was left). DIVERGENCES §2: the "both sides important /
  `removeProperty` clears both" edges removed from the flow-relative entry. Decided — left to C6-MARGIN-SIDES:
  `margin` and `padding` keep one value for their four sides (the existing per-side-longhands entry,
  DIVERGENCES §2), so an important side makes the rule's whole margin important; the brief's
  `.a { margin-inline-start: 1 !important; margin-right: 2 } .a.b { margin-right: 5 }` still gives 2
  (probed: `Margin { right: 2, left: 1, … }`), now documented in that entry. Fixing it needs
  per-side margin / padding storage (`TuiStyle::margin` as four longhands, with their own bits) —
  a data-model change of its own, not taken here. Red: `logical_importance_marks_only_the_mapped_side`
  (`("red", "blue")` against `("red", "lime")` under `ltr`), `logical_importance_orders_against_the_physical_side`
  (`right` `Cells(2)` against `Cells(5)`) and `cssom_priority_and_removal_keep_the_physical_declaration`
  (`margin-left` priority `"important"` against `""`) failed; green after, with the rdom-style unit
  `replayed_declarations_mark_their_own_side` and the `margin-left` assertion C5G-CSSOM-LOGICAL left
  for this item.
- 2026-10-07 — C5G-CUSTOM-SERIALIZE (gate fix): custom properties and declarations holding `var()` /
  `attr()` keep their source text (CSS Variables 1 §2: the value is the token sequence as written;
  §3 for a pending-substitution value), trimmed, comments and inner whitespace included. The
  tokenizer gains `tokenize_spans` (`SpannedTokens`: byte ranges, `TokenSpan`, beside the positions; `tokenize_at` is it without
  them; `Cursor::offset`), `rdom-css`'s block parser cuts each value's text from the body (first
  value token to last, `!important` excluded) and passes it through the new
  `property_dispatch::set_from_source` / `set_custom_source`; `set` (the CSSOM path) passes its
  string. A custom property stores the text as its `CustomValue` (tokenized from it as before); a
  kept `var()` declaration records it (`PendingDeclaration::text`, read by `value_text()`), and the
  substitution still works on the tokens. Decided — kept non-custom text (background image,
  position, size) lowercases keywords, function names and units, keeping `--*` names, strings and
  URLs (CSSOM §6.7.2; `render_keywords_lowercase`); the DIVERGENCES background entry says so. Red:
  `custom_properties_and_var_declarations_keep_their_text` (rdom-css: `Some("1 -2")` against
  `Some("1 - 2")`), `set_keeps_a_custom_propertys_text` (same) and
  `kept_background_text_lowercases_keywords` (`LINEAR-GRADIENT(TO RIGHT, RED 10PX, BLUE)` kept
  upper-case) failed; green after. No expectation changed. Split: `property_dispatch/set.rs` reached
  613 lines — the declaring entry points (`set`, `set_from_source`, `set_from_tokens`, the custom
  paths) moved to `property_dispatch/declare.rs` (159), `set.rs` keeps `set_parsed` (468).
- 2026-10-07 — C5G-PERF-AND-TESTS (gate fix). (1) `fit-content` measuring: an intrinsic keyword box
  measures its subtree's min- and max-content Row sizes (`Keywords::keyword`), and each enclosing
  keyword box re-walked it — quadratic in the nesting depth. Row-axis `content_size` is now memoized
  for the layout pass (`intrinsic/memo.rs`): a `(node, max?, cross budget, containing-block width) →
  cells` table held as document data that `layout_dom` opens at its start and drops at its end, so
  nothing is reused across passes or outside one. Decided: document data rather than a `TuiExt`
  field — a per-element slot cost one pointer and tripped `tui_ext_size_tripwire` (448 > 440 B), and
  a per-pass table cannot go stale; intrinsic sizes are pure within a pass (styles, text and the
  table column widths sized before it). Red: `nested_fit_content_boxes_measure_each_subtree_once`
  (a counting test, `ROW_WALKS`) — 48 walks at 4 levels, 780 at 16 (bound `6 × (depth + 1)`); green
  after: 20 and 92. (2) `pixel_math`: the pixel math function had its leaves rewritten into `ch`,
  evaluated on the assumption that `ch` takes no context. New `CalcUnit::Px` (a `<length>`, resolves
  to its number; `CalcUnit::parse` never returns it) and `parse_pixel_calc` (the calc parser in
  pixel mode: pixel-family units normalize to `px`, any other unit or a percentage fails); DESIGN
  "Pixel lengths select" names it. Red: `pixel_math_has_a_pixel_unit` failed to compile (no
  `CalcUnit::Px` / `parse_pixel_calc`); green after, and `pixel_lengths_inside_math_functions`
  unchanged. (3) Mutation checks of the C5-WRITING / C5-LOGICAL layout tests (each mutation applied
  alone, `css_phase5::` run, reverted): W1 `rtl` lines not flushed right → `rtl_lines_start_at_the_right_edge`
  (+ the scrollbar test); W2 the over-constrained `rtl` margin → `rtl_block_sits_at_the_inline_start_edge`
  and three logical tests; W3 `mirror_x` the identity → `rtl_flex_row_runs_right_to_left` (+
  margin-trim); W4 `inline_reversed` false → `rtl_margin_trim_inline_start_is_the_right_edge` (+ flex);
  W5 the positioned `rtl` inset rule off → `rtl_positioned_boxes_keep_the_right_inset`; W6
  `bar_on_left` false → `rtl_vertical_scrollbar_is_on_the_left`; W7 the relative offset's `rtl` arm
  off → `rtl_positioned_boxes_keep_the_right_inset`; W8 UA `[dir=rtl]` → `ltr` →
  `dir_attribute_sets_direction_and_both_inherit` (+ one); L1 `inline_axis` ignoring `rtl` → five
  logical tests; L2 `margin-block-start` → `margin-bottom` → `flow_relative_sizes_and_spacing_lay_out`;
  L3 `inline-size` → `height` → that and `cssom_lists_shared_storage_once`; L4 a corner radius mapped
  to the wrong inline side → `logical_corner_radii_follow_the_direction`; L5 the cascade picking the
  `ltr` overlay for every element → five logical tests. Every mutation is caught, so no test was
  added for them.
- 2026-10-07 — C5G-DOCS-AND-SHOWCASE (gate fix, docs + showcase). DIVERGENCES §1's sub-cell list said
  `box-shadow` spread was unsupported, against the shadow entry above it (spread is whole cells):
  now "blur (its spread is whole cells, above)". DESIGN `#[non_exhaustive]`: `BoxSizing`,
  `TextDirection`, `WritingMode` join the closed keyword enums, `MarginTrim` and
  `ContainIntrinsicSize` the closed value records (`IntrinsicSize` was there), `ScrollBounds` is
  `pub(crate)`, outside the rule. `walk.rs`: the "start from initial + inherit" comment, orphaned
  above the rule gathering since C5-LOGICAL moved that code into the direction loop, moved back to
  it. Showcase — decided: the shell's `box-sizing` reset is scoped to its chrome (`.app-shell`,
  `.app`, `.app-header *`, `.app-body`, `.sidebar *`, `.main`, `.view-content`, `.source-disclosure *`,
  `.status-bar *` and their pseudos), not `*`: a demo lays out under its own CSS in the app as in
  its standalone example and its snapshot, so `ua_chrome` ("pure defaults, no author CSS") shows the
  UA defaults with no opt-out rule, and `raf_progress` drops its own reset to show the `content-box`
  default — `.track { height: 1; width: 48; border: solid }` (a 50 × 3 border box as before), the
  fill computed against the 48-cell inside (`TRACK_CELLS`; a full bar was 50, past the border). The
  chrome's layout is unchanged (`chrome_layout_contract`, `chrome_dump` and every shell test pass).
  The snapshot harness (`tests/integration/common`) now does what the app does before a frame:
  `seed_inline_styles` (as `App::build`), then `cascade_all(&[base_stylesheet, demo sheet])`; and a
  snapshot carries a background layer — the glyph grid again with a key letter per distinct
  background color (`.` the terminal default), then the key — whenever a cell has a fill. Red /
  re-baseline: with the shell's base sheet applied (and, checked separately, with its old global
  `*` reset) no glyph row changed in any snapshot; the background layer was added to seven snapshots
  (`dom_api`, `inline_formatting`, `mutation_observer`, `parse_and_render`, `selectable_text`,
  `sticky`, `tab_form` — their glyph rows unchanged, each layer the demo's panel fill, plus
  `inline_formatting`'s yellow highlight); `raf_progress`'s first layer showed a full 48-cell bar —
  the harness had never parsed its `style="width: 0"` — and with the seeding the initial paint is
  the empty track the app shows, so its snapshot is unchanged from before this item.
- 2026-10-07 — Phase 5 closed: 7 items (C5-CONTAIN-SIZE partial — used with C14-CONTAIN) + 19 gate
  fixes (batch A 9, batch B 10). Gate-fix re-review folded into the Phase 6 gate. Carried, recorded:
  margin / padding importance stays whole-field (C5G-LOGICAL-IMPORTANT; DIVERGENCES §2, the
  per-side-longhands entry — per-side margin / padding storage would fix it); `vertical-align` beyond
  `baseline` for inline blocks (C9-VERTICAL-ALIGN; C5G-ATOM-BOX laid the line-box heights it builds on).
- 2026-10-08 — C6-MARGIN-SIDES (finishes C5G-LOGICAL-IMPORTANT): `margin-*` and `padding-*` are
  independent longhands (CSS Box 3 §3.2 / §4.2), the model C4-BORDER-SIDES gave the borders.
  `TuiStyle::margin` / `padding` are `Sides<Option<Value<MarginValue | PaddingValue>>>` (Breaking —
  rdom-style); the field table (C4G-IMPORTANT-BITSET) has a row per side, so each side has its own
  `ImportantMask` bit (`MARGIN_TOP` … `PADDING_LEFT`; `MARGIN` / `PADDING` gone) and `fields_of`
  gives a longhand its side and a shorthand the four, which routes importance, `removeProperty`
  and the CSS-wide keywords per side (`padding-top: inherit` inherits the top only). The cascade
  applies each side as its own declaration (`apply.rs`'s `value!` takes a field path). Removed with
  the single value: `parse::values::current_margin` / `current_padding` (the longhands' merge
  helpers) and the `pending` replay's seeding of the block's margin / padding (C5-LOGICAL's
  `a_substituted_longhand_keeps_its_shorthands_other_sides` still holds — the replayed side is its
  own field now). Decided: `ComputedStyle::margin` / `padding` stay `Margin` / `Padding` (computed
  values are per box, not per declaration); `Margin` / `Padding` ↔ `Sides` conversions; the
  shorthands serialize only when every side is set, in the shortest form (CSSOM §6.7.2, the
  `shortest_sides` helper border.rs had, now shared in `value_serializers.rs`), and `cssText` lists
  `margin` once when it serializes (as `padding` already did); rdom-tui re-exports `Margin`,
  `MarginValue`, `PaddingValue` at its root. `declared_count` now counts margins (it skipped them)
  and each padding side. DIVERGENCES §2 "Per-side longhands share the shorthand's storage" removed.
  Red: `css_phase6/margin_sides.rs` — the gate example `.a { margin-inline-start: 1 !important;
  margin-right: 2 } .a.b { margin-right: 5 }` gave right `Cells(2)` against `Cells(5)` (and the
  padding twin), a later rule's `margin-left` zeroed an earlier `margin-top` (`Cells(0)` against 1),
  `padding-top: inherit` took the whole padding (`Cells(4)` left against 7), an important
  `margin-top` beat a later `margin: 1` on every side; green after. `property_dispatch/spacing_tests.rs`
  (per-side bits, a longhand alone, shortest serialization, a keyword on one side) was written with
  the new names and so cannot compile against the old storage. Changed expectations: the shortest
  shorthand form (`10% 2` for `10% 2 10% 2`, `-10% auto`, `1` for `1 1 1 1` in rdom-css
  `important.rs`); `declared_count` 10 → 13 and 24 → 27 (padding counts four sides); the dispatch
  and rdom-css tests read sides instead of one `Margin` / `Padding`, and
  `padding_side_important_sets_its_sides_bit` (was `…_sets_padding_bit`) asserts the top bit only.
  No layout or paint expectation changed; no snapshot changed.
- 2026-10-08 — C6-DISPLAY-KEYWORDS: `display` takes CSS Display 3 §2's grammar —
  `[<display-outside> || <display-inside>]` (outer `block | inline`, inner `flow | flow-root |
  flex`; an omitted outer is `block`, an omitted inner `flow`), `<display-listitem>`
  (`list-item` with an optional outer and `flow` / `flow-root`), `contents | none`, and the legacy
  keywords as their pairs (`parse/values/display.rs`, which serializes the shortest form back);
  `run-in` / `grid` / `table` / `ruby` stay invalid until their phases. Model: `Display::Contents`,
  `Flow::FlowRoot` (+ `Flow::is_block_flow`), and `list_item` on `TuiStyle` / `ComputedStyle`
  (owned by `display` with the two types, a field-table row, cascaded; no layout effect until
  C10-LIST-ITEM's marker). `inline flow-root` stays `(InlineBlock, Block)` — an inline block always
  establishes a BFC. Found: `display: inline` left the inner type unwritten, so `display: flex`
  then `display: inline` in one cascade made an inline flex container — `inline` writes `flow`
  now. `flow-root`: `finalize_bfc_formation` marks it a BFC root, so no margin collapses through
  it (floats, Phase 8, will be contained by the same flag); every `Flow::Block` match takes
  `FlowRoot` too. `contents` (§2.5) — decided, one module: `render/box_tree.rs` owns "which nodes
  generate boxes": `is_contents`, `box_parent` (the nearest ancestor that is not box-less — used
  for the scroll offset, the relative-shift and percentage bases, the parent's direction, the
  definite-height chain, the containing block, atom paint), and `box_sequence` (a block
  container's child nodes with each box-less child that holds a block-level box replaced by its
  own sequence, between `BoxItem::Generated` items for its visible static `::before` / `::after`;
  one holding only inline content stays one inline-level item, which the packer walks as a
  box-less inline box — its pseudos at its start / end). Block runs, the line-bearing and
  first-line placement of generated content (`generated.rs`), margin collapse (the chain walks,
  collapse-through shape, the formatting-context parent walk), `is_ifc_block` and the
  text-to-anonymous-box lookup (`inline_flow_for_text`, via `box_index`) read that sequence;
  `element_children_of` unwraps box-less children like fragments, so flex items, layout recursion,
  scroll extents and intrinsic sizes see its children; positioned descendants are found through
  it. Paint: the stacking walks and `recurse_children` pass through it as through a fragment (no
  box, no shadow, no background — a fragment of its own text paints without its background);
  `is_positioned` / `creates_stacking_context` / `computed_position` / sticky are false for it
  (its `position`, `z-index`, `opacity` apply to no box). Hit-testing descends into its children
  and puts it on the path. Its rect is zero at the parent's content origin
  (`collapse_hidden_children`). Focus: unchanged — it stays focusable (HTML "being rendered"
  includes an element whose rendering is delegated to its children; Chromium since 2023).
  Appendix B: on a replaced element or form control it behaves as `none`
  (`apply::finalize_unusual_contents`: `br`, `wbr`, `meter`, `progress`, `canvas`, `embed`,
  `object`, `audio`, `iframe`, `img`, `video`, `frame`, `frameset`, `input`, `textarea`, `select`).
  Red: the rdom-style `display_tests.rs` failed to compile (`Display::Contents`, `Flow::FlowRoot`,
  `list_item`); all seven `css_phase6/display.rs` tests failed the strict parse ("valid
  declaration"); after the parser landed, the layout halves first ran with the box-tree code in
  place (green). Mutation checks (each alone, `css_phase6::display`, reverted): M1 no box-less
  unwrap in `element_children_of` → `contents_children_are_flex_items`; M2 `box_sequence` never
  expands → the block-flow, pseudo and hit-test tests; M3 hit-test without the box-less arm →
  `contents_children_hit_test_through_it`; M4 no Appendix B rule → the focus test; M5 `flow-root`
  not a BFC → `flow_root_keeps_its_childs_margin_inside`. Found while writing the flex test:
  rdom's initial `flex-direction` is `column` (CSS: `row`) and DIVERGENCES does not say so —
  carried to C6-DIRECTION-REVERSE. Changed expectations: the C1 `initial` coverage test perturbs
  `list_item` by hand (no single `display` value moves the outer type, the inner type and
  `list-item` at once). DIVERGENCES: the §3 display line removed, the list-item lines say the
  keyword parses and the marker is C10's. No snapshot changed.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): `AnonymousIfc::child_range` indexes the
  box-tree sequence (`render::box_tree`): the parent's child nodes unless a `display: contents`
  child holds a block box, which gives its children in its place between its static `::before` /
  `::after`.
- 2026-10-08 — C6-VISIBILITY: `visibility: visible | hidden | collapse` (CSS Display 3 §4),
  inherited (`Visibility`, `TuiStyle` / `ComputedStyle::visibility`, `ImportantMask::VISIBILITY`,
  builder, root re-export). Decided — one answer: `render/visibility.rs::visibility_of` (the
  presented value of a running transition, else the computed one) is what paint, hit-testing and
  focus read. Paint: a non-visible box draws nothing of its own (`BoxFrame::visible`: no outer /
  inset / backdrop shadow, background, border, canvas callback, chrome, scrollbars); text draws by
  its owner's visibility and generated content by the pseudo-element's, so a `visible` descendant
  shows inside a hidden box; the single-row path advances past a hidden run. Hit-testing: a hidden
  box is no target — `descend_plain` / `hit_stacking_context` search its content and put it on a
  visible descendant's path (the DOM ancestors), and an IFC point on a hidden inline's cells
  resolves to what is beneath. Focus: a hidden element is skipped by Tab, its subtree still walked
  (HTML §6.6.3). Copy: a hidden node's text is not copied (HTML §3.2.7 rendered text). `collapse`:
  on a flex item (`flex::is_collapsed` — an element flex container's item; the document root's
  children, flex items of rdom's viewport column only as a layout device, take `hidden`, as a
  browser's `<body>` children would) the main-axis pass makes it a strut (Flexbox §4.4: main size,
  min, max and main margins 0) and the cross pass keeps its cross size, which holds the line's;
  intrinsic sizes drop its main contribution. The gaps beside a strut stay (it is still an item of
  the line, CSS Box Alignment §8.1) — *wrong, corrected by C6G-COLLAPSE: CSS Flexbox §9.4 step 10
  ignores a collapsed item after noting its strut size, gaps and `justify-content` included*. On a table row (`tree::is_collapsed_table_row`: a `<tr>` of a
  `<table>` or row group) it leaves the flow (`is_in_flow` false, its geometry zeroed with the
  `display: none` subtrees) while `size_columns`, which reads every row, still sizes the columns
  with its cells (CSS 2.1 §17.5.5); column collapse needs columns, so `<col>` waits for C13-TFC
  (DIVERGENCES §3). Elsewhere `collapse` is `hidden`. Transitions: `AnimatableProperty::Visibility`
  → `AnimatedProp::Visibility`; `lerp_visibility` is `visible` for every progress strictly inside
  (0, 1) when either end is `visible`, the ends their own values; a change between two non-visible
  values is not interpolable and is not registered (CSS Display 3 §4's animation type, after CSS
  Transitions 1 §2.1); `PresentationStyle` gains `visibility` (non_exhaustive). `layout_differs`
  includes `visibility` (`collapse` moves boxes). Red: the rdom-style test failed to compile
  (`Visibility`); with the data model and cascade in, all six `css_phase6/visibility.rs` tests
  failed for their reasons — the hidden box drew its border, background and `aavvii`, the hidden
  block was hit (`NodeId(3)` against the parent), all three buttons were focusable, the
  collapsed item took 7 cells (`c` at 10, not 3), the collapsed row kept its row (`y` 1, not 0),
  `collapse` drew `aa`; green after. The root-children case was found by
  `collapse_elsewhere_is_hidden` (the first `is_collapsed` made `.a` a strut, `y` 0). The
  transition tests were written after `lerp_visibility`; mutation-checked: `lerp_visibility` made
  a plain midpoint step → both transition tests fail. Changed
  expectations: the canonical-values table, the important-setter coverage and the inherited-set
  probe gain `visibility`; the C1 `initial` test perturbs it. The rdom-style README lists
  `visibility` and the display keywords (C6-DISPLAY-KEYWORDS left them out).
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): A `visibility` transition with a `visible` end
  presents `visible` for its whole run (`hidden → visible` shows at once, `visible → hidden` at the
  end); a hidden box's text is left out of a copy (HTML §3.2.7).
- 2026-10-08 — C6-ORDER: `order: <integer>` (CSS Flexbox §5.4; a math function rounds, a value
  past `i32` clamps, CSS Values 4 §10.9 / §5.1), not inherited (`TuiStyle` / `ComputedStyle::order`,
  `ImportantMask::ORDER`, builder, `parse::values::parse_order`). Decided — one ordering:
  `render/box_tree.rs::sort_by_order` (a stable sort by `order`, no-op when every item's is 0; a
  child that is not a flex item keys 0) orders the flex container's in-flow items before
  `layout_flex_children` and the intrinsic children of a flex container (so `margin-trim` trims
  the first / last items of that order), and `paint_order_children` (an element flex container's
  items, through fragments and box-less children, in that order; any other node's child nodes) is
  what the in-flow paint walk (`stacking_walk::children_of`), the stacking walks
  (`collect_layers`, `atom_shadows_in`) and hit-testing (`descend_children_reverse`) iterate —
  §5.4: `order` affects painting as it does layout. The root fragment's children (rdom's viewport
  column) are not reordered, like C6-VISIBILITY's `collapse`. Not changed (§5.4.1): Tab order,
  selection, copy, the DOM. Red: the rdom-style test failed to compile (`order` field); with the
  model in, the three `css_phase6/order.rs` tests failed — `[0, 2, 4, 6]` for `[6, 0, 4, 2]`,
  `abb` for `baa`, the reordered button left of the other; green after. Mutation checks (each
  alone, reverted): paint ignoring `order` → `bba` (the painting test); hit-testing ignoring it →
  the hit at (1, 0) is `b`; layout ignoring it → all three tests. Changed expectations: the
  canonical-values table, important-setter coverage and the C1 `initial` test gain `order`.
- 2026-10-08 — C6-DIRECTION-REVERSE: `flex-direction: row | row-reverse | column | column-reverse`
  (CSS Flexbox §5.1). Model: the axis stays `direction` (`Direction` is the layout axis
  everywhere, so it gains no variants) and `flex_reverse` joins it — `TuiStyle` / `ComputedStyle`,
  `ImportantMask::FLEX_REVERSE`, owned by `flex-direction` like `display`'s `list_item`; the
  `direction` builder and node setter write `false` as `flex-direction: row` does,
  `direction_reverse(axis)` the reversed form. Layout — decided, one rule for both mirrors:
  `flex::AxisFlip` says which axes run from their physical end — a row's main axis under `rtl`
  XOR `row-reverse`, a column's under `column-reverse`, a column's cross axis under `rtl` — and
  the existing mirrored frame does the rest: main-axis margins swap (now on the vertical axis
  too), the placement flips x by `mirror_x` and y by the new `mirror_y`; C5-WRITING's
  `rtl`-only mirror is the `AxisFlip` of an unreversed row. `FlexTrim::of` swaps main-start /
  main-end under reversal (main-start is the inline-end / block-end side), and the intrinsic
  contribution pass reads the reversed axis's margins and trimmed edges in the same order.
  Scrolling (CSSOM View §4) — the scrolling area origin is the main-start edge, so the overflow
  past main-end is reached with negative offsets, as Chromium and Gecko give a reversed
  container: `scroll_extent::origin_at_end` (a flex container's `AxisFlip`, else `rtl` on the
  horizontal axis) decides both `scroll_x_bounds` and the new `scroll_y_bounds`, and
  `TuiExt::scroll_y` is a signed `scrollTop` (`i32`, was `usize` — Breaking, rdom-tui), the model
  C5G-RTL-SCROLL gave `scroll_x`: `ScrollBounds` gains `min_y`, the wheel / keys / API / into-view
  clamp through it, and the vertical thumb, its hit-test and the autoscroll bands measure
  `scroll_y_from_area_start`. The brief's "overflow goes toward main-start" is read as: content
  starts at main-start and overflows past main-end, which the negative offsets reach (a
  `row-reverse` item wider than its box sticks out on the left). Found and recorded:
  `flex-direction`'s initial value is `column` (CSS: `row`) — load-bearing (block containers
  measure their children through that axis; the root's viewport column is one) — DIVERGENCES §2,
  not changed here. Red: the rdom-style test failed to compile (`flex_reverse`); with the model in,
  all five `css_phase6/direction_reverse.rs` tests failed — `[0, 2, 4]` for `[8, 6, 4]`, `[0, 1,
  2]` for `[5, 4, 3]`, the main-start margin on the left (1 for 5), the trim on the wrong item
  (1 for 8), `(0, 3)` for `(1, -2)`; after the layout half, the scroll test still failed
  (`scrollLeft` clamped at 0 for -2); green after. `runtime/scrollbar/reverse_tests.rs` (wheel,
  keys, thumb, initial position of a `column-reverse` scroller) was written with the scrolling
  half. Mutation checks (each alone, reverted): the intrinsic pass ignoring reversal → the
  max-content assertion (9 for 7); `FlexTrim::of` not swapping → the trim test; the vertical
  origin never at the end → three `reverse_tests`. Changed expectations: `declared_count` 13 → 14
  and 27 → 28 (`direction` writes the reverse flag); the `flex-direction` mask test gains
  `FLEX_REVERSE`; scroll-reading tests and the showcase's scroll readout take `i32`
  (`sidebar_scroll_end_regression`, `scrollable_list_keyboard_scroll`, the app tests). No snapshot
  changed.
- 2026-10-08 — C6-FLEX-LONGHANDS (done; was partial): the `flex-grow` (`<number [0,∞]>`,
  initial 0) and `flex-basis` (`content | <'width'>`: `auto`, `content`, cells, `%`, `calc()`,
  and the intrinsic keywords — `FlexBasis::Intrinsic`, Breaking) longhands (CSS Flexbox §7.3), and
  the `flex` shorthand sets exactly its three longhands (`fields_of("flex")`), no longer `width` /
  `height` (Breaking — rdom-style; `flex: inherit` now inherits the factors). Layout — decided,
  spec order: `main_axis` computes each item's flex base size (§9.2 step 3): a definite
  `flex-basis` through `Keywords` (so the `Sizer`: `box-sizing` applies, as for `width`; a
  percentage against the container's inner main size, `content` when that is indefinite), `auto`
  the main size property (its content size when `auto`), `content` the max-content size
  (`content_max_size`, ignoring a declared main size); rdom's `width: <n>fr` (`Size::Flex`) is a
  base of 0 growing by `n` when `flex-grow` is 0; a table cell's used column width is a base that
  does not grow. `distribute` is §9.7 as written: the factor is grow when the outer hypothetical
  sizes leave free space, else shrink; inflexible items (factor 0, or a max / min clamp against
  the direction) freeze at their hypothetical size; the loop shares the free space (scaled when the
  factors sum below one) by `flex-grow` or the overflow by `flex-shrink × base` (rolling whole
  cells), clamps, and freezes by the sign of the total violation; the §4.5 automatic minimum is
  resolved lazily, and skipped when growing from a content-sized base (it cannot bind). Then §9.5
  step 12: only the space left goes to `auto` main-axis margins (`flex/mod.rs`; they took it
  before `flex-grow` did). Found: with grow no longer in `height`, the §9.8 definite-height chain
  lost the `flex: 1` panes — `nearest_block_ancestor_height_is_definite` now treats any item of an
  element flex container as its container's (§9.8: the post-flexing main size and the stretched
  cross size are definite), and a growing child of the root fragment as definite (it was
  `Size::Flex`); DIVERGENCES' "Percentage height" entry rewritten (its row-stretch gap is closed).
  Red: the rdom-style tests failed to compile (`flex_grow`, `FlexBasis::Intrinsic`); the nine
  `css_phase6/flex_longhands.rs` tests failed — seven on the strict parse (`flex-grow` /
  `flex-basis` unknown), `flex: 1 1 auto` gave `[6, 6]` for `[7, 5]` and the zero-basis item
  `[6, 6]` for `[8, 4]`; green after. Mutation checks (each alone, reverted): `flex-basis`
  ignored → seven tests; no automatic minimum when growing → the zero-basis test; shrink weighted
  by the factor alone → the scaled-shrink test. Changed expectations:
  `flex_auto_margin_starves_flex_grow` asserted the reverse of §9.7 / §8.1 — now
  `flex_grow_takes_the_free_space_before_auto_margins` (A 30 wide at 0, B at 30);
  `flex_shorthand_full_grammar` asserts `flex_grow` and untouched `width` / `height`; the numeric
  math test reads `flex_grow`; the canonical-values table, important-setter coverage and the C1
  `initial` test (`flex: 2 0 7`) gain the longhands. No snapshot changed. Split:
  `property_dispatch/table.rs` reached 602 lines — the name list, case folding and `all`'s names
  moved to `property_dispatch/names.rs` (172; `table.rs` 439), paths re-exported unchanged.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): `flex: inherit` inherits the three factors
  (not the main size), and `parse::values::parse_flex_basis` is public.
- 2026-10-08 — C6-GAP: `row-gap` / `column-gap` (`normal | <length-percentage [0,∞]>`, initial
  `normal`) and the two-value `gap` shorthand (`<'row-gap'> <'column-gap'>?`), CSS Box Alignment 3
  §8.1 / §8.3. Checked first: `gap` parsed one `<length-percentage>` into one field for both
  axes, and its two-value form was invalid. Model — ready for grid's two axes: `TuiStyle` /
  `ComputedStyle` `row_gap` + `column_gap` (Breaking — the `gap` field and `ImportantMask::GAP`
  are gone), `GapValue::Normal` (Breaking; 0 in flex, `resolve` / `as_cells`; `border-spacing`
  shares the type but never parses it), `parse_gap_shorthand`, `serialize_gap` (shared with
  `border-spacing`'s serializer); `gap` serializes as one value when the two agree. Layout — one
  mapping: `layout_pass::gap_along(axis)` is `column-gap` between items placed horizontally and
  `row-gap` between items stacked vertically, used by the flex main axis (a row's `column-gap`, a
  column's `row-gap`; the other axis waits for C6-WRAP's lines), the intrinsic sum and the block
  container's `row-gap` between block children. Found and recorded: that last one is an rdom
  extension — CSS gives gaps to flex, grid and multi-column containers only — now in DIVERGENCES
  §2 and COVERAGE §4. Transitions: `AnimatedProp::Gap` animates the pair
  (`AnimatedValue::Gaps`; `PresentationStyle::row_gap` / `column_gap` replace `gap`, Breaking —
  rdom-tui); `row-gap` / `column-gap` as `transition-property` names are inert (C12-ANIMATABLE).
  CSSOM: `cssText` lists `gap` once over its longhands (`shorthand_family_of`, and `margin`'s,
  missing since C6-MARGIN-SIDES); `length` / `item()` list the shorthand and its longhands, as
  they do for `padding` (D-M4-2, unchanged). Red: the rdom-style test failed to compile
  (`row_gap`, `GapValue::Normal`); both `css_phase6/gap.rs` tests failed the strict parse
  (`column-gap` / `row-gap` unknown); green after. Mutation check: `gap_along` with the axes
  swapped → both tests. Changed expectations: tests reading `gap` read `row_gap` (and
  `column_gap`); `declared_count` 14 → 15 and 28 → 29 (`gap` is two fields); the CSSOM `length` /
  `item()` tests count `gap`'s two longhands (4, `row-gap`, `column-gap`); the C1 `initial` test
  perturbs `gap: 1 2`. Snapshot: `dom_api` — the demo's `cssText` line is unchanged; its
  `length` reads 9 (was 7) and its `item()` list gains `row-gap` and `column-gap` after `gap`,
  the longhands the demo's `gap: 2` now has, as `padding` lists its four.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): `TuiNodeExt::gap()` reads the inline gap only
  when `row-gap` and `column-gap` are the same cells; `set_gap` writes both. `length` / `item()`
  list the `gap` shorthand and its longhands, as for `padding`.
- 2026-10-08 — C6 file-size pass (part 1, no behaviour change): `rdom-showcase/src/nav.rs` (634
  lines, touched by C6-DIRECTION-REVERSE) — its inline test module moved to `nav_tests.rs` (195;
  `nav.rs` 441). Production files touched by part 1 now under the bar: `property_dispatch/table.rs`
  439 (C6-FLEX-LONGHANDS split), `intrinsic/mod.rs` 585 and `tui_style/builder/mod.rs` 569 — near
  it, for C6-SPLIT; the `flex/*` files are 219–403.
- 2026-10-08 — C6-FLEX-DIRECTION-INITIAL: `flex-direction`'s initial value is `row` (CSS Flexbox
  §5.1; `ComputedStyle::initial().direction` and `Direction::default()`, Breaking — rdom-style).
  Root fix first — decided, one rule: `layout_pass::flow_axis(computed)` is the axis a box's in-flow
  children lay out along: a flex container's `flex-direction` axis, every other box's block axis
  (vertical; every box is `horizontal-tb`, DIVERGENCES §1). `flex-direction` applies to flex
  containers only (§5.1 "Applies to"), so the intrinsic children pass (`along`, the sum-or-max
  choice) and the `border-collapse` parent-edge insets read `flow_axis`, not `direction`; the flex
  line reads `direction` (a flex container's), and the fragment root lays its children out in an
  explicit viewport column (`Flow::Flex` + `Direction::Column`, was `ComputedStyle::initial()`).
  Red: `css_phase6/flex_direction_initial.rs` — the `display: flex` container without
  `flex-direction` placed its items at `[(0, 0), (0, 0), (0, 0)]` against `[(0, 0), (2, 0), (4, 0)]`
  (a column of 0-high items), and a block with `flex-direction: row` inside a row measured its
  shrink-to-fit width as the sum of its children (6 against 3) — the coupling, a bug before the
  initial value changed; the root-stack test passed (guard); the rdom-style `initial_is_safe_defaults`
  assertion was updated to `Row`. Green after. Mutation checks (each alone, reverted, the file
  touched): `flow_axis` returning `direction` for block flow → the block test; the fragment's
  column dropped (its style the initial `row`) → the root-stack test and three `visibility` tests.
  Showcase, examples, UA sheet, READMEs: every `display: flex` rule already declares
  `flex-direction` (checked by a scan of each rule block), and the UA's flex rule (`<tr>`) sets
  `Direction::Row`; the UA comments that said "default column" now say block container. Changed
  expectations (test probes, not layout): the C1 `initial` test perturbs `flex-direction:
  column-reverse` (`row-reverse` now leaves the axis at its initial value) and the inherited-set
  probe's parent takes `Direction::Column`. No layout expectation changed; no snapshot changed.
  DIVERGENCES §2's entry removed; COVERAGE §3.8's row says `row`.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): Before, rdom-tui read `flex-direction` on a
  block container and measured its children along that axis, so `flex-direction: row` on a block
  made its shrink-to-fit width the sum of its children's.
- 2026-10-08 — C6-WRAP: `flex-wrap: nowrap | wrap | wrap-reverse` (CSS Flexbox §5.2; `FlexWrap`,
  `TuiStyle` / `ComputedStyle::flex_wrap`, `ImportantMask::FLEX_WRAP`, builder, not inherited) and
  `flex-flow` (§5.3, `<'flex-direction'> || <'flex-wrap'>`, owning `flex-direction`'s two fields and
  `flex_wrap`, shortest serialization; cssText leaves the longhands to it only if it serializes —
  `flex-flow` is not added to `shorthand_family_of`, as `flex` is not). The keyword grammars moved to
  `parse/values/flex.rs` (`parse_flex_direction` / `_wrap` / `_flow` and serializers). Layout —
  decided, spec order in one orchestrator: `layout_flex_children` gathers the items once
  (`collect_main_axis_items`, now a plain `Vec<ChildMain>`; each line sums its own margins), breaks
  them into lines (`flex/lines.rs::break_lines`, §9.3 step 5: outer hypothetical main size — the
  base clamped by min / max, the §4.5 automatic minimum resolved for a non-content base — with the
  main-axis gap counted between items, Box Alignment §8.1; an item too big for an empty line takes
  one alone), resolves each line's flexible lengths and `auto` margins (`resolve_line_main`, §9.7 /
  §9.5 step 12), sizes the lines (single-line: the container's inner cross size, §9.4 step 8;
  multi-line: the largest outer hypothetical cross size, `cross::hypothetical_outer_cross` — the
  cross size at the used main size with nothing stretched — plus `stretch_lines`,
  `align-content: normal` = `stretch` in flex, Box Alignment §5.3) and places each line `line_offset`
  along the cross axis, the cross-axis gap (`row-gap` in a row) apart; a stretched item fills its
  line, percentages still resolve against the container (`CrossSpace { line, container }`).
  `wrap-reverse` joins `AxisFlip::cross` (a row's cross axis under `wrap-reverse`, a column's under
  `rtl` XOR `wrap-reverse`): the mirrored frame flips the lines, the cross margins swap on both
  axes now, `FlexTrim::of` swaps cross-start / -end, and `scroll_extent::origin_at_end` reads the
  flip, so the overflow past cross-end (the top) is reached with a negative `scrollTop`.
  `margin-trim` in a multi-line container (CSS Box 4 §3.2): each line's first / last item loses its
  main-start / -end margin (`trim_line_edges`), the first line's items their cross-start and the last
  line's their cross-end margins. Decided: lines break on the margins as gathered (only the
  container's first and last items trimmed), then each line's edge margins are trimmed — a trimmed
  margin cannot move an item onto the line before it. Intrinsic sizes (§9.9) — one algorithm:
  `intrinsic/children.rs` (the children half of `measure_content`, moved, see below): along the main
  axis a multi-line container's min-content size is its largest outer contribution (each item can
  take a line alone), its max-content size one line (the sum); across it, `intrinsic/wrap.rs` finds
  the inner main size to break at — a row's content width (its declared width, else the budget, as
  `wrapped_rows`), a column's content height (declared, else its single-line content height clamped
  by `max-height`, §9.3's indefinite main size) — and `lines::lines_cross_size` runs the layout's
  steps (gather, break, per-line §9.7, hypothetical cross sizes with a cyclic cross percentage as
  `auto`). Found and fixed: a non-stretched flex item's cross size (an `auto` cross margin, an inline
  block) measured its content against the container's cross size — a row item's text wrapped to
  the container's height; it measures at the item's used main size now (no existing expectation
  changed). DIVERGENCES §1 gains "Flex free space is shared in whole cells" (rolling grow / shrink,
  equal shares with a remainder cell to each of the first `auto` margins / lines); §3's `flex-wrap`
  line removed. Red: the rdom-style `flex_wrap_takes_its_three_keywords` / `flex_flow_sets_direction_and_wrap`
  failed to compile (`FlexWrap`, `flex_wrap`); with the data model in, all nine
  `css_phase6/wrap.rs` tests failed — five items in one shrunk line `[(0, 0), (3, 0), (5, 0), (7, 0),
  (9, 0)]` against three lines, `ys` `[0, 0, 0]` for `[0, 0, 4]` / `[0, 0, 3]` / `[0, 0, 5]`, the
  min-content row `(14, 1)` for `(6, 3)`, `wrap-reverse` `[0, 0, 0, 0, 0]` for `[4, 4, 2, 2, 0]`,
  `scrollTop` `Some(0)` for `Some(-1)`; `margin_trim_applies_per_line` first failed its own sheet
  (`margin-trim: inline block-start` is invalid; rewritten as `inline-start inline-end
  block-start`). Green after. Mutation checks (each alone, `css_phase6::wrap`, reverted and touched):
  no line break → all nine; no per-line main trim → the trim test; no line stretch → the
  `align-content` test; `AxisFlip::cross` ignoring `wrap-reverse` → the reverse, trim and scroll
  tests; the main-axis min-content max → the intrinsic test; the cross-axis lines intrinsic → the
  row, intrinsic and trim tests; the cross-start trim on every line → survived at first (the line
  stretch absorbed the trimmed cell), so the trim test gained a stretched-items case and catches it;
  `FlexTrim` not swapping under `wrap-reverse` → the trim test's `wrap-reverse` case (added for it).
  Changed expectations: the canonical-values table, important-setter coverage, the C1 `initial`
  perturbation and the inherited-set probe gain `flex-wrap` (and the table `flex-flow`). No
  snapshot changed. Split: `intrinsic/mod.rs` reached 608 lines — the children half of
  `measure_content` (outer contributions, trims, the sum / max / lines, text runs) moved to
  `intrinsic/children.rs` (201; `mod.rs` 453).
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): Consumer-visible change recorded from the
  CHANGELOG: a non-stretched flex item's cross size (an `auto` cross margin, an inline block) is
  measured at its used main size — a row item's text wraps to the item's width, not the container's
  height.
- 2026-10-08 — C6-JUSTIFY: `justify-content` (CSS Box Alignment 3 §5.2 grammar: `normal |
  <content-distribution> | <overflow-position>? [ <content-position> | left | right ]`; not
  inherited). Model — decided, one vocabulary for all six alignment properties, extending the type
  ACID.md found CSS could not set: `Align` (was `{Start, Center, End, Stretch}`, unused) is every Box
  Alignment keyword plus `normal` / `auto`, and `Alignment { keyword, overflow: OverflowAlign,
  legacy }` the computed value (`layout/alignment.rs`; closed value types, DESIGN). One parser
  (`parse/values/align.rs`) reads any alignment property from a `Grammar` (which of `auto`,
  baseline, distribution, self positions, `left` / `right` it takes); `serialize_alignment` writes the
  shortest form. Layout — `flex/justify.rs::justify_offsets`, per line after §9.7 and the `auto`
  margins (which take positive free space first, §8.1; `LineMain` now carries the signed free space):
  the extra space before each item, added to the gap. Mapping in the main-start frame: `normal` /
  `stretch` / `flex-start` start, `flex-end` end; `start` / `end` are the writing mode's ends —
  main-start unless `row-reverse` / `column-reverse` (under `rtl` both mirrors agree); `left` /
  `right` physical on a row (`end` / `start` when the main axis is flipped), `start` on a column
  (§4.2); distribution fallbacks per Flexbox §8.2 as now written — `space-between` to `safe
  flex-start`, `space-around` / `space-evenly` to `safe center` (one item, or negative free space);
  `safe` that overflows aligns as `start` (§4.4); no overflow keyword aligns as `unsafe`, as browsers
  do (§4.4 lets the UA choose; a smarter default would hide which cells were dropped). Rounding —
  decided, whole cells: `center`'s lead is the free space halved rounded down (toward main-start,
  negative free space included — the odd cell after the items); a distribution places item `i` at
  `ceil(free × k_i / d)` (`space-between` `i / (n − 1)`, `space-around` `(2i + 1) / 2n`,
  `space-evenly` `(i + 1) / (n + 1)`), so each space is a whole-cell step and the remainder cells go
  to the first spaces; DIVERGENCES §1's whole-cell flex entry says so. Not changed: the static
  position of an absolutely positioned child of a flex container stays the content box's start
  (DIVERGENCES §2, D-M2-2). Red: `justify_content_takes_its_grammar` failed to compile
  (`justify_content`, `ImportantMask::JUSTIFY_CONTENT`); with the data model in, all seven layout
  tests in `css_phase6/justify.rs` failed — `[0, 2, 4]` for `space-between`'s `[0, 4, 8]` and for
  overflowing `center`'s `[-1, 1, 3]`, a single `space-around` item at `[0]` for `[4]`, the second
  line at `0` for `2`, the column's `end` at `[0, 1]` for `[4, 5]`, `row-reverse` `start` at
  `[8, 6, 4]` for `[4, 2, 0]` (the `auto`-margin test passed: the margins already took the space).
  Green after. Mutation checks (each alone, `css_phase6::justify`, reverted and touched): `center` lead
  0 → four tests; rolling positions rounded down → the distribution test; `safe` ignored → the
  overflow test; `auto` margins not taking precedence → the margin test; `left` ignoring the flip and
  `start` ignoring `row-reverse` → the axes test. Changed expectations: the canonical-values table,
  important-setter coverage, the C1 `initial` perturbation and the inherited-set probe gain
  `justify-content`. No snapshot changed.
- 2026-10-08 — C6-ALIGN: `align-items` (`normal | stretch | <baseline-position> |
  <overflow-position>? <self-position>`, Box Alignment 3 §6.3) and `align-self` (`auto |
  <'align-items'>`, §6.1), not inherited, on the C6-JUSTIFY model (`Alignment`, the grammar-driven
  parser). Layout — decided, one module: `flex/align.rs` plans a line's cross alignment
  (`LinePlan`): the item's `align-self`, its container's `align-items` for `auto`; `normal` behaves
  as `stretch` (§6.1), which stretches only an `auto` cross size with no `auto` cross margin (§9.4
  step 11, the `max-*` / `min-*` clamp as before) — `place_cross` now takes the item's alignment, and
  `auto` cross margins still win (§8.1). Keyword mapping in the cross-start frame (`CrossFrame`):
  `flex-start` / `flex-end` the line's cross edges (so `wrap-reverse` swaps them); `start` / `end`
  the container's writing-mode edges — a row's block axis (top first, whatever `wrap-reverse`), a
  column's inline axis (right first under `rtl`); `self-start` / `self-end` the item's own (a
  column item's `direction`); `center` rounds the leading space down (DIVERGENCES §1); `safe`
  aligns an overflowing item as cross-start (§4.4), the default overflows (as browsers). Baselines —
  decided: a box's first / last baseline is its first / last content row
  (`inline/vertical.rs::content_rows`, the C5G-ATOM-BOX measurement, factored out so the inline
  block's baseline and flex share it); none → synthesized at the border box's bottom row (§9.1).
  `baseline` / `first baseline` group a row's participating items (§9.4 step 8: no `auto` cross
  margin) so their first baseline rows share the row of the one with the most rows above it, the
  group flush with the line's top; `last baseline` likewise from the bottom (the group's fallback
  side, `self-start` / `self-end` — a row's block axis, so `wrap-reverse` does not move it); the
  group's extent is a multi-line line's floor (`lines::line_cross_size`, now shared by layout and
  intrinsic sizing — the single-line path skips the hypothetical measurement). In a column there is
  no baseline on the cross axis: `safe self-start` / `safe self-end` (§9.3). DIVERGENCES §2 gains
  "A box's baselines are its first and last content rows". Not changed: the static position of an
  absolutely positioned flex child still ignores `align-items` / `justify-content` (DIVERGENCES §2,
  D-M2-2). Red: `align_items_and_align_self_take_their_grammars` failed to compile (`align_items`,
  `ImportantMask::ALIGN_ITEMS`); with the data model in, all seven layout tests in
  `css_phase6/align.rs` failed — every item stretched (`(0, 6)` for `(2, 1)` / `(5, 1)`), baselines
  `[0, 0, 0]` for `[0, 2, 1]`, the baseline line's second item at 0 for 2, the overflowing centered
  item at 0 for -1, the column item 10 wide at 0 for `(4, 1)`, `wrap-reverse` `flex-start` `(0, 6)`
  for `(5, 1)`; the `stretch_respects_limits_and_auto_margins` test passed (that behaviour existed).
  Green after. Mutation checks (each alone, `css_phase6::align`, reverted and touched): `align-self`
  always deferring to `align-items` → the override test; stretch ignoring the alignment → five
  tests; the first-baseline row the minimum → both baseline tests; the baseline extent left out of
  the line size → the line-size test; `self-start` ignoring the item's direction, `start` ignoring
  the container's → the column test; `safe` ignored → the overflow test. Changed expectations: the
  canonical-values table, important-setter coverage, the C1 `initial` perturbation and the
  inherited-set probe gain the two properties. No snapshot changed.
  From its CHANGELOG bullet (moved by C6G-CHANGELOG): A stretched item's `auto` cross size is
  clamped by its `min-*` / `max-*` cross sizes; a box without content synthesizes its baseline at
  its bottom row, and a multi-line container's line grows to hold its baseline-aligned group.
- 2026-10-08 — C6-ALIGN-CONTENT: `align-content` (`normal | <baseline-position> |
  <content-distribution> | <overflow-position>? <content-position>`, Box Alignment 3 §5.1; not
  inherited). Layout (Flexbox §8.4, §9.4 step 15): `flex/justify.rs` became `flex/content.rs` — one
  whole-cell distribution (`offsets`: the C6-JUSTIFY placements and rounding) behind
  `justify_offsets` and the new `align_content_offsets`; `layout_flex_children` measures the free
  cross space of a multi-line container (its cross size less the lines and the gaps between them)
  and either stretches the lines (`normal`, `stretch` → `stretch_lines`, C6-WRAP) or adds each
  line's lead to its offset. Mapping in the cross-start frame: `flex-start` / `flex-end` the cross
  edges (`wrap-reverse` swaps them), `start` / `end` the writing mode's (a row's top / bottom, a
  column's inline-start / -end), `baseline` / `last baseline` their fallbacks `start` / `end` (Box
  Alignment §9.3 outside a table cell); fallbacks `space-between` → `safe flex-start`, `space-around`
  / `space-evenly` → `safe center`, `safe` overflow → `start`. Single-line containers — checked:
  Flexbox §8.4 says `align-content` has no effect there, and every current browser agrees; the
  CSSWG investigated applying it (csswg-drafts#3052) and kept the rule for web compatibility, so
  rdom matches (a `nowrap` line is the container's cross size anyway; a `wrap` container with one
  line is multi-line and aligned). Red: `align_content_takes_its_grammar` failed to compile
  (`align_content`); with the data model in, all six `css_phase6/align_content.rs` tests failed
  (the lines stretched: `[0, 0, 5]` for `flex-start`'s `[0, 0, 1]` and `space-between`'s `[0, 0,
  9]`; `[0, 1, 2]` for overflowing `center`'s `[-1, 0, 1]`; the columns at `[0, 4, 7]` for
  `[2, 4, 6]`). Green after. Mutation checks (each alone, reverted and touched): the offsets never
  computed → all six; `start` ignoring the flipped axis → the `wrap-reverse` test; `safe` ignored →
  the overflow test; the lead not added → all six. Applying it to single-line containers too is an
  equivalent mutation (their one line is the container's size, so there is no free space) and
  survives, as it must. Changed expectations: the canonical-values table, important-setter coverage,
  the C1 `initial` perturbation and the inherited-set probe gain `align-content`. No snapshot
  changed.
- 2026-10-08 — C6-PLACE: `justify-items` (Box Alignment 3 §6.2, with `legacy` / `legacy left |
  right | center`) and `justify-self` (§6.1), and the `place-content` / `place-items` / `place-self`
  shorthands (§5.5 / §6.4 / §6.5: `<align> <justify>?`, one value for both — a baseline
  `place-content` gives `justify-content: start` — each owning its two longhands, shortest
  serialization, `cssText` naming the shorthand over its longhands like `gap`). The grammar gained
  `legacy`; a shorthand splits its tokens at the first point where both halves parse. `legacy` —
  decided, at cascade time: `justify-items`' initial value is `legacy` (`Alignment::LEGACY`), and
  `finalize_justify_items` (element and pseudo-element cascades) computes it to the parent's value
  when that is `legacy` with a side, else `normal` (§6.2), so `legacy center` reaches descendants
  that do not set the property. Checked first: Chromium 130 (Oct 2024) ships `justify-self` for
  block-level boxes (Firefox: bug 1930584), Chromium 123 `align-content` for block containers;
  `align-self` does not apply to block-level boxes (§6.2). Layout — `block/align.rs`: a block-level
  box's effective `justify-self` (`auto` → the parent's `justify-items`, a `legacy` value as its
  side) other than `normal` / `stretch` sizes an `auto` width `fit-content` (`resolve_block_width`,
  clamped by `min-*` / `max-*`) and offsets its left margin: `start` / `end` by the containing
  block's direction, `self-start` / `self-end` by the box's, `left` / `right`, `center` (the lead
  rounded down), `flex-*` as `start` / `end`, the baseline values `safe start` / `safe end` (§9.3);
  an `auto` margin takes the space instead (CSS 2.1 §10.3.3 as before). `align-content` on a block
  container: `layout_children_aligned` (`layout_pass/mod.rs`) measures the content and, when
  `align_content_lead` moves it, lays the children out again at the offset — `end` / `flex-end` /
  `last baseline` at the bottom, `center` / `space-around` / `space-evenly` centered, the rest at
  the top (the distributions' block fallbacks); never for a box whose height is its content's (an
  `auto` height in block flow); content taller than the box keeps its start under `safe` or in a
  scroll container. A non-`normal` `align-content` makes a block container an independent
  formatting context (§5.1; `finalize_bfc_formation`, DESIGN updated). In flex, `justify-items` /
  `justify-self` parse, cascade and do nothing (§6.1); grid uses them with C7-GRID-ALIGN. Recorded in
  DIVERGENCES §3: `align-content` does not move an inline-only container's lines (its IFC is laid out
  at the content box; only block-level content moves), and `justify-self` / `align-self` on
  absolutely positioned boxes. Red: with the data model in (the rdom-style grammar tests were
  written with it), four `css_phase6/place.rs` tests failed — the block child stretched `(0, 20)`
  for `justify-self: start`'s `(0, 3)` and `justify-items: center`'s `(8, 3)`, the overflowing
  `center` at 0 for -5, `align-content: center` at 0 for 4; the `place-*` flex tests passed (the
  shorthands set the flex properties); green after. The independent-formatting-context test was
  written with that rule and mutation-checked. Mutation checks (each alone, reverted and touched):
  `auto` ignoring the parent's `justify-items` → the `justify-items` test; `legacy` never inherited
  → the same; no block alignment → three tests; `auto` margins not taking precedence → the margin
  test; the scroll-container overflow rule off → the `align-content` test; no re-layout at the lead
  → the same; `center` at 0 → three tests; no independent formatting context → the margin-collapse
  test. Changed expectations: the C1 `initial` test checks `justify-items` apart — `initial` is
  `legacy`, which computes to `normal` there; the canonical-values table, important-setter
  coverage, the perturbation and the inherited-set probe gain the five properties. No snapshot
  changed.
- 2026-10-08 — C6-SPLIT (no behaviour change): `tui_style/builder/mod.rs` had reached 625 lines —
  it crossed the bar with C6-ALIGN's two setters (601) and was not split there, a slip this item
  repays: the gap, flex-factor, `flex-direction` / `flex-wrap`, alignment and `order` setters and
  the `display: flex` conveniences moved to `builder/flex.rs` (174; `mod.rs` 465), paths and
  signatures unchanged. Checked, under the bar: `layout_pass/flex/*` (`mod.rs` 467, `cross.rs` 449,
  `lines.rs` 349, `distribute.rs` 273, `main_axis.rs` 282, `align.rs` 271, `placement.rs` 231,
  `content.rs` 210, `collapse.rs` 130), `intrinsic/mod.rs` 452 (split by C6-WRAP, `children.rs`
  201, `wrap.rs` 78), `layout_pass/mod.rs` 473, `block/mod.rs` 525, `parse/values/align.rs` 319.
  Near it and left, recorded for the Phase 6 architect gate: `computed.rs` and `tui_style/mod.rs`
  (543 each — the two field lists, which grow by a line or two per property) and
  `property_dispatch/serialize.rs` (522, one arm per property).
- 2026-10-08 — Phase 6 gates (with the C5G re-review: all 19 at the root; C5G-ATOM-BOX brought the
  B3 cost below). Architect: 3 blocking — a `display: contents` element keeps stale `inline_layout` /
  anonymous boxes / scroll extents from its box days (caret, hit-test, Tab stop wrong after a toggle);
  a single-line `align-items: baseline` row with auto height is sized without the baseline shift;
  nested inline-blocks measure exponentially (`atom_rows` / `content_rows` unmemoized Column
  measurements). API: 2 blocking — an inline flex container in block flow is packed as inline text
  (no flex layout, no box) while paint treats it as an atom; `LineBox` silently lost `Default`.
  Non-blocking: per-node `Vec` per frame from `paint_order_children`; hit path drops a contents wrapper
  under `order`; text / pseudos of a contents child of a flex container lost; a contents element's own
  overflow clips; static position inside contents in an IFC; hit-test does not descend into an atom
  in a pure IFC; `visibility: collapse` strut measured at main size 0 and still counted by gaps /
  justify; visibility answers differ (public `is_tab_focusable`, programmatic focus, blur on hide,
  copy); flex spec gaps (§4.5 minimum not clamped by `max-*`, hypothetical sizes without the lazy
  minimum, shrink weight from the border-box base, `net <= 0`, inline-block items never stretched,
  §9.2 3.B); flex `Vec`s and repeated measurement; block `align-content` re-lays the subtree out
  (exponential under nesting) and ignores `min-height`; `justify-self: baseline` fallback direction;
  CSSOM removal edges and `::placeholder` dropping `pending`; direction re-run bounded only by
  `debug_assert!`; `declared_count` hand-copied and drifted; files 520–578 lines; minor box-tree /
  clamp / gutter / gap items; scroll docs incomplete (reverse / wrap-reverse negatives, a false clamp
  doc, no public range); alignment builders without `impl Into` and accepting out-of-grammar keywords;
  `flex-direction` split over two fields; margin / padding builders write four sides, no per-side
  setters; stale docs and README lines; coverage rows over-claim; DIVERGENCES whole-cell entry too
  narrow; upgrade callout order and missing flex base-size line; prelude gaps; no Phase 6 node setters;
  `set_from_source` without `important`, `SpannedTokens` a tuple; ACID tiles do not name Phase 6;
  `LineBox` / `InlineFragment` / `Align` closed but due to grow; `flex: <n>` basis `0` vs `0%`;
  CHANGELOG `[Unreleased]` unusable at 144 KB (restructure: callout split silent / compile, an
  old → new table, ≤ 3-line bullets). Downstream (read-only): rdom-charts / rdom-virtualtable /
  lens-tui pin 0.3.14; no reliance on the `column` default; rdom-virtualtable has two compile breaks
  (whole-`Margin` assignment, `padding.is_none()`). Accepted: signed scroll offsets, `Token::Number(i64)`,
  Phase 5 public helpers, module moves, scroll re-clamp, abspos `order: 0`, rounding toward main-start.
  Fix all as `C6G-*`, three batches: A correctness and cost, B flex spec / visibility / CSSOM,
  C API, docs and the CHANGELOG restructure.
- 2026-10-08 — C6G-CONTENTS-STATE (AB1): a box-less element (`display: contents` / `none`, CSS
  Display 3 §2.5) now drops every value it derived from having a box. Layout writes `inline_layout`,
  `anonymous_blocks`, `scroll_content_*` only on a node it lays out, and the contents reset
  (`zero_contents_children`) cleared only the rects and the margin-chain memo, so `<p><span
  style="display:block">hello</span></p>` toggled to `contents` left `inline_flow_for_text` stopping
  at the span's dead line boxes (caret, selection, hit-test on a zero rect), and a former `overflow:
  auto` scroller kept its extent and its Tab stop (`is_scroll_container`). Decision: one reset,
  `tree::clear_box_state` — rects, margin-chain memo, line boxes, anonymous blocks, scroll extent and
  offsets (a re-boxed scroller starts at 0, as a browser's does), scroll bookkeeping, static position
  — shared by the `contents` reset and the `display: none` collapse. Found on the way: the root
  fragment never ran the collapse at all (`layout_fragment_children`), so a root child turned `none`
  kept its rect, and a `contents` root child its box state; it now does, at the viewport origin.
  Red: `a_box_turned_contents_keeps_no_box_state` — `Ifc { block: span }` for `Ifc { block: p }`;
  with the line-box reset in, the former scroller still in `focusable_elements` (the root-fragment
  gap); `a_root_child_turned_none_reads_zero` — `(0, 0, 10, 1)` for the zero rect. Green after.
  Mutation checks (each alone, reverted and touched): line boxes kept → the `Ifc` assertion; scroll
  height kept → the Tab-stop assertion; no root-fragment collapse → both tests. No snapshot changed.
- 2026-10-08 — C6G-CONTENTS-BOXTREE (AN2–AN5, AN19's box-tree part): the walks that pair a box with
  its children now read a box-less element (CSS Display 3 §2.5) as its children. Hit-testing (AN2):
  with an `order` sibling, `paint_order_children` hands back the items with box-less wrappers
  unwrapped, so the wrapper arm never ran and `<span contents><b>x</b></span>` beside `<i
  style="order:-1">` hit `b` with no `span` on the path (`:hover` missed); a hit now inserts the
  unwrapped element ancestors below the container, outermost first (`insert_box_less_ancestors`).
  A flex container's text (AN3, CSS Flexbox §4): the text-leaf predicate read direct text only, so
  `<div flex><span contents>hello</span></div>` had no element item and no text and painted
  nothing; it and the intrinsic sizer's twin (`has_non_whitespace_text`) now share
  `box_tree::holds_loose_text` — text, or a box-less child's text or visible static pseudo — and the
  container lays it out as its one anonymous item. Decision: the general case — text runs and the
  container's own `::before` / `::after` *beside* element items, which are dropped today with or
  without `contents` (checked: `<div flex>ab<span>hello</span></div>` paints `hello`) — is an
  anonymous-flex-item feature the flex algorithm (`NodeId` items throughout) does not model; recorded
  in DIVERGENCES §3, not yet scheduled, and a box-less child's pseudos joining the one item rather
  than being items of their own is noted there. Scroll extent (AN4, CSS Overflow 3 §3.1):
  `extend_scrollable_overflow` took a box-less element's `overflow` as a clip and stopped, so its
  children were lost to the ancestor scroller's extent; it now neither counts the element's (zero)
  rect nor clips at it. `is_scroll_container` needed no guard: C6G-CONTENTS-STATE zeroes a box-less
  element's scroll extent. Static position (AN5): `static_position_in_ifc` ordered fragments by the
  parent's direct child index, which an out-of-flow child of a box-less element does not have, so
  it sat at the line's origin; `BoxOrder` numbers the box-tree children with a span per box-less
  child (one position before its children, one after), which also orders its `::before` / `::after`
  around them. Cost (AN19): `push_sequence` asked `holds_block_box` of every box-less child and then
  recursed into it, re-walking each level once per enclosing level; one walk now collects a child's
  items and keeps them only when they hold a block box (else truncates back to the child), and
  `box_index` no longer asks `holds_block_box` per child before building the sequence. Red:
  `css_phase6/contents.rs` — the hit path `[f, b]` without the wrapper; the flex row `"        "`
  for `"<hello> "`; the scroll extent 1 for 4 (rewritten first: a box-less element directly in the
  scroller is unwrapped by `element_children_of` and passed before the fix, so the test puts it
  under an in-flow box); the static position `(0, 0)` for `(5, 0)` (re-checked against the reverted
  file); `box_sequence_visits_each_node_once` — 15 visits at 4 levels for at most 5. Green after.
  Mutation checks (each alone, reverted and touched): no ancestor insertion → the hit-path test; the
  `contents` clip kept → the extent test; direct-text-only leaf predicate → the flex test.
  No snapshot changed.
- 2026-10-08 — C6G-INLINE-FLEX-ATOM (PB1): every inline-level box whose inner display type is not
  `flow` is an atomic inline (CSS Display 3 §2.4, CSS 2.1 §9.2.2 / §10.8), decided in one place,
  `box_tree::is_atomic_inline` — `inline-block` (`inline flow-root`, which the parser maps to
  `Display::InlineBlock`), `inline-flex` (`Display::Inline` + `Flow::Flex`) and `Display::Inline` +
  `Flow::FlowRoot` (reachable through the builder, serialized `inline flow-root`). `inline-table`
  does not apply: there is no `display: table` (C13-TFC). The packer (`compute_inline_layout`'s
  direct children and `walk_subtree`) atomized only `Display::InlineBlock`, so `<div>x <span
  style="display:inline-flex;gap:1;border:solid">…</span></div>` packed its items' text as inline
  text with no flex layout and no box, while paint (`paints_atomically`) took it for an atom; now
  the packer, `is_ifc_block` (an atom neither establishes nor prevents an IFC), paint's `in_a_line`
  and `paints_atomically` all ask the one predicate. CSS-COVERAGE's `display` row claimed
  `inline-flex` Supported on the inline-block sentence; rewritten to say what holds now. Red:
  `an_inline_flex_container_is_an_atom_in_its_line` — the span at `(0, 0, 0)` (x, width, height) for
  `(2, 5, 3)`; green after. `an_inline_flex_container_paints_once_in_its_line` was added with the fix
  (bare text and inside an inline formatting context). Mutation checks (each alone, reverted and
  touched): the packer back on `InlineBlock` → the first test; paint's `in_a_line` back on
  `InlineBlock` → the paints-once test (beside bare text the atom painted twice: `rgb(192, 0, 63)`
  for `rgb(128, 0, 127)`), which the first test alone did not catch. `is_ifc_block` treating the atom
  as inline content is an equivalent mutation for both tests (the IFC path and the anonymous-box
  path pack the atom alike) and survives; the predicate is shared for consistency. Noted, not
  changed: the max-content width of an IFC (`intrinsic::inline::inline_content_width`) sums an
  atom's text rather than its border box, for inline blocks as before. No snapshot changed.
- 2026-10-08 — C6G-ATOM-HIT (AN6): `hit_content` resolved a point in an inline formatting context
  to its fragment's owner and stopped, so an atomic inline's fragment returned the atom itself —
  `<p><i>Go</i> <span inline-block><b>k</b></span></p>` hit the span, not `b` (beside bare text the
  anonymous-box path descends through the child list and worked) — and a hidden atom sent the point
  "beneath" by walking up, never trying its `visible` child. `hit_fragment` now says whether the
  fragment is an atom; an atom is hit as the box it is (`hit_in_flow_element`, factored out of
  `descend_children_reverse`: positioned → its layer, a stacking context → atomically, else
  `descend_plain` with its `visibility` / `pointer-events` rules), under the inline ancestors it sits
  in (`inline_ancestors`). Red: `css_phase5/atom_box.rs` `hit_testing_descends_into_an_atom` — the
  atom (node 5) for its child `b` (node 9) inside the inline formatting context; green after, both
  layouts. Mutation check (reverted and touched): the atom pushed and its content searched without
  its own visibility rules → the hidden atom's border cell hit the atom (5) for the paragraph (2).
  No snapshot changed.
- 2026-10-08 — C6G-BASELINE-ROW (AB2): a flex container's `auto` cross size comes from intrinsic
  sizing (`children_size`), which for a single-line container took the plain largest outer cross
  contribution, while only the multi-line path (`lines_cross_size` → `line_cross_size`, §9.4 step 8)
  counted the baseline-aligned extent — so `.f { display: flex; align-items: baseline }` holding a
  `padding-top: 2` item beside a `padding-bottom: 2` one was 3 rows, the browser's 5, with the
  second item hanging out (every `align.rs` baseline test fixed `height: 6`). Decision: a single-line
  *row*'s height is its one line's, measured the way a multi-line container's lines are — through
  `wrap::wrapped_cross_size` → `lines_cross_size`, which now keeps every item on one line when the
  container is `nowrap` (§9.3) — so the hypothetical cross sizes are taken at the items' used main
  sizes, as layout places them. A single-line *column* keeps the largest contribution: its cross
  axis is the inline axis, where `baseline` falls back (§8.3) and the min- / max-content measure
  applies, which the lines path (a max-content measurement) would lose. Red:
  `baseline_alignment_sizes_a_single_line` — height 3 for 5; green after. Mutation check (reverted
  and touched): `lines_cross_size` breaking a single-line container into lines → the overflowing
  `nowrap` case, 6 for 5. One snapshot changed, justified: `rdom-showcase` `parse_and_render` — its
  `body` is a single-line row of two `flex: 1` cards, and its height was the cards' heights each
  measured at the row's whole width (about 70 cells), where the accent card's text took 3 lines; at
  its used width (31) it takes 4, so the card's fourth bullet (`• Unicode: …`) and its bottom
  padding row were clipped by its `overflow-y: hidden` — now shown, two rows taller, as a browser
  sizes the row. No other test expectation changed.
- 2026-10-08 — C6G-ATOM-COST (AB3): the layout pass memoized intrinsic content sizes on the Row axis
  only (C5G-PERF-AND-TESTS). An atom's rows in its line (`inline::vertical::atom_rows`: its height,
  `intrinsic_size`, and its baseline, `content_rows` → `content_max_size`) and a flex item's
  baseline box (`flex::cross::baseline_box`: `resolve_cross_size` and `content_rows`) are Column
  measurements of the subtree, and a Column measurement of an inline formatting context packs its
  atoms, which measure their own subtrees again — so every level re-measured every level below it.
  Decision: memoize the Column axis too — the key gains the axis (`memo::Key`); intrinsic sizes are
  as pure within a pass on this axis as on the other (checked: nothing under `intrinsic/` or the
  packer reads a rect the pass writes; `table_used_width` is sized before the pass). Taking the
  baseline from the height measurement instead was considered: an atom's height is its box
  contribution (a declared `height` wins) and its baseline its content's last row, so they are two
  questions; with the memo they share the walk where they share a key. (The `ComputedStyle` "clone"
  in `atom_rows` is an `Rc` clone.) Red (counting tests in `intrinsic/memo_tests.rs`, `COLUMN_WALKS`
  beside `ROW_WALKS`): `nested_inline_blocks_measure_each_subtree_once` — 25 walks at 4 levels for at
  most 10 (9 / 25 / 49 / 81 at 2 / 4 / 6 / 8 levels: quadratic); `nested_baseline_rows_measure_each_subtree_once`
  — 135 for at most 5 (12 / 135 / 1266 / 11469: exponential). Green after: the rows `depth + 1`
  (one Column measurement per subtree), the inline blocks `2 × depth + 1` — two widths per subtree,
  its own in its line and the containing block's, where the enclosing box's intrinsic block-flow
  estimate stacks an inline run's atom as a block child (an existing approximation of mixed-content
  intrinsic sizing, not changed here). No test expectation and no snapshot changed.
- 2026-10-08 — C6G-ORDER-ALLOC (AN1): `box_tree::paint_order_children` (CSS Flexbox §5.4: `order`
  reorders painting and hit-testing) returned a `Vec` for every node it was asked about — a flex
  container whose items are all `order: 0` built two, its items and then its child list — and its
  callers' other branch (the children of a box-less child or fragment) collected the child list too;
  it runs per node per paint (`stacking_walk`, `stacking` units and atom shadows) and per hit-test.
  It now returns `PaintOrder`, a double-ended iterator that walks the child list in place
  (`first_child` / `next_sibling` from the front, `last_child` / `previous_sibling` from the back,
  for hit-testing's reverse order) and collects and sorts only when an item's `order` is not 0
  (`any_reordered`, through fragments and box-less children, allocation-free); `PaintOrder::tree`
  replaces the callers' collected child lists. The stale "allocates nothing" doc of
  `for_each_atom_shadow` now says when it does. Red: `paint_order_allocates_only_for_reordered_items`
  (`test_alloc`) — 4 allocations for two walks of an all-`order: 0` flex container, for 0 (first
  observed as 5 with a test buffer that reallocated, fixed in the test); green after, both
  directions, the reordered case still in order-modified document order. No snapshot changed.
- 2026-10-08 — C6G-FLEX-COST (AN10), no layout change. The §4.5 automatic minimum was resolved in
  `lines::break_lines` (hypothetical sizes for line breaking) and again in the §9.7 freeze loop,
  each with its own lookup, and for items it cannot affect: an item whose base is its specified main
  size (`flex-basis: auto` with a definite `width`) has an automatic minimum no larger than that size
  (the specified size suggestion), so clamping a size at or above the base by it is a no-op — when
  no `max-*` below the base could let it win (the minimum is not clamped by `max-*` yet, AN9, batch
  B, so the skip requires that). `ChildMain` now carries `specified_base` and a lazily filled
  `auto_min` cell (`ChildMain::auto_min`) that line breaking and every freeze-loop iteration share,
  and both skip the resolution where `auto_min_cannot_bind_above_base` (line breaking always,
  the loop when growing, as it already did for content-sized bases). The freeze loop allocated a
  clamped copy of the targets and a list of the unfrozen indices per iteration, plus a hypothetical
  and an auto-min `Vec`; it now keeps two (the targets, which start as the hypothetical sizes, and
  the frozen flags), recomputing a clamp where it needs one (cheap with the minimum cached), and
  freezes in one pass (freezing an item changes only its own state). The second Column measurement
  of a non-stretched multi-line item (`hypothetical_outer_cross`, then `place_cross`) is served from
  the pass memo since C6G-ATOM-COST; the test pins it. Not reduced: the per-container `Vec`s of
  lines, line plans, justify / align offsets (about ten per container) — linear in the items, one
  per line or container, left for the grid work that will share them. Red (`flex/cost_tests.rs`):
  `a_multi_line_row_measures_each_item_once` — 12 automatic-minimum resolutions for 4 (two items
  that can bind, in each of the two flex runs: the parent measuring the container's height, then
  its layout); `the_freeze_loop_allocates_nothing_per_iteration` (`test_alloc`) — 10 allocations in
  three iterations for 6 in one. Green after: 4 resolutions, at most 5 Column measurements (one per
  item and the container's), 2 allocations in one iteration and in three. Mutation checks (each
  alone, reverted and touched): the specified-base skip off → 8 resolutions; the Column memo off →
  13 Column measurements. No test expectation and no snapshot changed.
- 2026-10-08 — C6G-BLOCK-ALIGN (AN11, AN12, AN13). AN11: `layout_children_aligned` laid a block
  container's content out, measured it, and laid it out again at the `align-content` offset — in
  the scrollbar second pass and the scroll-clamp re-layout too — so under k aligned ancestors the
  innermost box was laid out 2^k times. Decision: layout is translation-invariant, so the content is
  moved, not re-laid: `tree::shift_content` (the container's anonymous block boxes and its
  children's subtrees) over a `tree::shift_subtree` that now serves `position: sticky` as well (its
  private copy in `sticky.rs` removed), moving rects, anonymous boxes, positioned-pseudo rects and —
  new — recorded static positions, which an out-of-flow box inside the moved content was placed
  from. AN12 (CSS 2.1 §10.7): `align_content_lead` returned 0 for every content-sized box (an
  `auto` height in block flow), but `min-height` can make one taller than its content —
  `min-height: 10; align-content: center` around 2 rows centers them at row 4 in Chromium; the lead
  now compares against the height the box resolves to, the content clamped by `min-height` /
  `max-height`, from `auto_height::used_content_height`, which `resolve_auto_height` and the lead
  share with the gating (`is_content_sized`). AN13 (Box Alignment 3 §4.2, checked against the TR:
  "the fallback alignment for first baseline is safe self-start (for self-alignment)", last baseline
  `safe self-end`): `justify_offset` fell back to the containing block's `start` / `end`; it now uses
  the box's own (`self_rtl`), and the doc cites §4.2 (it said §9.3). Red: `css_phase6/place.rs`
  `align_content_uses_the_resolved_height` — 0 for 4; `justify_self_baseline_falls_back_on_the_boxs_own_direction`
  — 0 for 16 (an `rtl` box in an `ltr` container); `block/align_tests.rs`
  `nested_aligned_containers_lay_out_each_box_once` (a `layout_node` counter) — 16 layouts at 3
  levels for 5. Green after: one layout per box. The static-position assertion (an absolutely
  positioned box after the moved content, at row 6) was added with the shift. Mutation checks (each
  alone, reverted and touched): no shift → the two `align-content` tests; static positions not
  moved → row 2 for 6. No snapshot changed.
- 2026-10-08 — C6G-LINEBOX-API (PB2, PN13): C5G-ATOM-BOX moved `LineBox` to `render/inline/boxes.rs`
  and dropped its `Default` derive, which 0.5.0's own Breaking hint tells consumers to use
  (`..Default::default()` in a struct literal) — a silent second break. And both `LineBox` and
  `InlineFragment` grow again with C9-VERTICAL-ALIGN. Decision: both are `#[non_exhaustive]` (DESIGN's
  rule for public types that will grow), which forbids struct literals outside rdom-tui — functional
  update included — so 0.5.0's hint is superseded, not kept: `LineBox` gets `Default` back (written
  by hand: an empty one-row line at the top, `height: 1` — a derived `height: 0` would be no line)
  and `LineBox::new(fragments, width, top)`; `InlineFragment` gets `InlineFragment::text(node,
  text_node, source_byte_offset, x, text)` (width from the text's cells, one row) and
  `InlineFragment::atom(node, x, width, height)`. `InlineFragment` has no `Default`: there is no
  meaningful `NodeId` to default its two node fields to (`NodeId` has none), so the constructors are
  its migration. The public fields stay public (readable and assignable). The C5G-ATOM-BOX Breaking
  bullet and the upgrade callout point at the constructors; a Breaking bullet records the change.
  Red: the new `migration_hints::line_box_construction_hints` group failed to compile (no
  `LineBox::new` / `default`, no `InlineFragment::text` / `atom`); green after. The
  `scroll_token_and_line_hints` group's struct literals (which `#[non_exhaustive]` makes illegal
  outside the crate) now use the constructors; a `compile_fail` doctest on `LineBox` pins that a
  struct literal no longer builds outside rdom-tui (its twin, `LineBox::default()` with a field set,
  compiles and runs). No snapshot changed.
- 2026-10-08 — C6G-RERUN-BOUND (AN15): `cascade_element` re-ran an element's ladder with its own
  `direction` when a flow-relative inline property met an inherited direction that differs (CSS
  Logical 1 §4, C5-LOGICAL), in a `loop` whose only bound was `debug_assert!(runs < 2)` — a release
  build would spin if the invariant (a flow-relative property cannot set `direction`) ever broke.
  The policy is now `walk::settle_direction`: run with the inherited direction, and once more with
  the element's own when it differs — never a third time — returning the last result and whether its
  direction held (the call site keeps a `debug_assert!` on that, as a check, not a bound). Red:
  `cost_tests::the_direction_rerun_runs_at_most_twice` failed to compile (no `settle_direction`);
  green after (a ladder that flips direction on every run is run twice and its second result kept;
  one that keeps it, once). Mutation check (reverted and touched): a third run → `(3, Ltr)` for
  `(2, Rtl)`. No test expectation and no snapshot changed.
- 2026-10-08 — C6G-ANON-FLEX-ITEMS (batch A's DIVERGENCES §3 finding; closes C6G-CONTENTS-BOXTREE's
  partial AN3): a flex container dropped every run of text beside an element item and its own
  `::before` / `::after` (`<div flex>ab<span>hello</span></div>` painted `hello`), because the flex
  algorithm took `NodeId` items. CSS Flexbox §4: each contiguous run of child text is an anonymous
  block container flex item (a run of only white space — the characters `white-space` affects,
  under any `white-space` value — is not rendered), and the pseudo-elements, child boxes, are
  blockified items. Decision: the flex algorithm takes `flex::item::FlexItem` — an element, or an
  `AnonymousItem` (a run of the container's box-tree children with the anonymous box style,
  `cascade::anonymous_box_style`: inherited properties from the container, every other one
  initial) — from `item::flex_items`, built on `box_tree::flex_sequence` (the container's
  pseudo-elements, its child nodes, every box-less child replaced by its own between its
  pseudo-elements). An anonymous item is measured by packing its run (`pack_run`): max-content /
  min-content width, rows at a width, first / last line as baselines; its base is its max-content
  size, its automatic minimum its min-content size, its cross size the line when stretched; laid
  out, it is an `AnonymousIfc` on the container (`child_range` into the flex sequence), so paint,
  scroll extent, content shifting, the caret (`box_index` reads the flex sequence) and selection
  read it as a block container's anonymous boxes. `order` is 0, alignment, wrap, gaps and
  `border-collapse` (no border) apply as to any item; the intrinsic sizes of a flex container
  measure its items, anonymous ones and pseudo-elements included (they were added beside the
  element items as block text runs — the widest on the inline axis). A text-only flex container
  no longer takes the pure-text-leaf path: its text is its one anonymous item, so `justify-content`
  / `align-items` place it and `text-align` aligns within the item, as in browsers. Hit-testing:
  `inline_target_at` matched an anonymous box by rows only, and a flex row's items share rows; it
  now prefers the box whose columns hold the point. Out-of-flow children neither are items nor
  split a run. A box-less child's pseudo-elements are items of their own (DIVERGENCES had them
  joining the text). Not done, recorded in DIVERGENCES §3: a pseudo-element item is laid out with
  the anonymous style — its own sizes, `flex`, `order`, margins, padding and border do not apply.
  Red (`css_phase6/anon_items.rs`, 7 tests): `["hello     "]` for `["abhello   "]`; `["xay  ]"]` for
  `["[ ab x y ]  "]`; `["xb    "]` for `["xab   "]`; `hi` top-left for centered on the bottom row;
  `["efgh  ", ""]` for two lines; width 3 for 7; the caret on `cd` resolved into `ab`'s text node.
  Green after (and `a_contents_childs_pseudo_elements_are_items_of_their_own`, added with the
  fix). Mutation checks (each alone, restored and touched): the hit-test column check off → the
  caret test (`(ab, 2)` for `(cd, 1)`); whitespace-only runs rendered → the pseudo / gap test (an
  extra gap); anonymous items measured as 0 in intrinsic sizing → width 4 for 7. No other test
  expectation and no snapshot changed.
- 2026-10-08 — C6G-FLEX-SPEC (AN9), six fixes in the flex algorithm, each red first. (1) CSS
  Flexbox §4.5, "in all cases, the size is clamped by the maximum main size if it's definite":
  `ChildMain::auto_min` clamps the resolved minimum by the item's `max-*`, so a `max-width: 3` item
  holding `abcdefgh` in a 2-wide row is 3, not 8. (2) §9.7 step 1 decides growing or shrinking by
  the outer *hypothetical* main sizes, the automatic minimum included; line breaking had it
  (resolved where it can raise the base) but the freeze loop clamped the bases by the explicit
  `min-*` only, so a `flex: 1 1 0` item holding `abcdefgh` beside a `width: 4; min-width: 0` one
  in 10 cells grew, then froze at its minimum and overflowed (8 / 4); both now take
  `ChildMain::hypothetical` (8 / 2, the spec's). (3) §9.7 step 4.c scales the shrink factor by the
  *inner* flex base size: `ChildMain::inner_base` (the base less the main-axis padding and border)
  — a 10-wide item and a `content-box` 2-wide one with 2 cells of padding a side over 12 cells are
  7 / 5, were 8 / 4. (4) A line with no room (`net <= 0`) returned the hypothetical sizes unshrunk;
  the loop shrinks it like any other (a `width: 5; min-width: 0` item in a `width: 0` row is 0).
  (5) CSS Display 3 §2.7: a flex item is blockified, but the cross resolver measured an
  `inline-block` item at its content size instead of stretching it (§9.4 step 11); it stretches
  now — except among the document root's children, rdom's viewport column standing in for a
  browser's `<body>`, where an inline block is inline-level and hugs its content (the
  `<input type=submit>` button tests pin that half; they failed with the first, unconditional
  fix). The computed `display` itself is not blockified — recorded in DIVERGENCES §3. (6) §9.2 step
  3.B, implemented: an item with a preferred aspect ratio, a used flex basis of `content` and a
  definite cross size (a length, or a percentage of a definite container cross size) takes its base
  from that cross size through the ratio (`main_axis::aspect_base`, through
  `cross::aspect_cross_from_main`, so `box-sizing` / `auto && <ratio>` apply as on the cross axis):
  `aspect-ratio: 2; height: 3` is 6 wide, was 0. C6G-FLEX-COST revisited: its skip condition
  (`auto_min_cannot_bind_above_base`, a specified base *and* no `max-*` below it) waited on (1);
  with the minimum clamped by `max-*`, a specified or content-sized base is never below it, so
  `ChildMain::min_main_above_base` skips on either alone — shared by the hypothetical size and the
  growing freeze loop. Red (`css_phase6/flex_spec.rs`): 8 for 3; (8, 4) for (8, 2); (8, 4) for
  (7, 5); 5 for 0; height 1 for 3; (0, 3) for (6, 3). Green after. Mutation checks (each alone,
  restored and touched): each fix reverted fails its own test and only it. Four existing
  `layout_pass::tests` expectations encoded the unstretched `inline-block` flex item (the
  pre-C6G "OOTB" hug): the column-parent case now asserts the stretch (width 80) and the row-parent
  case the stretched height (24), renamed; the pseudo-chrome and relative-shift cases, which test
  the intrinsic width, now set `align-items: flex-start`. No snapshot changed.
- 2026-10-08 — C6G-FLEX-BASIS-ZERO (PN15): `parse_flex_shorthand` gave an omitted
  `<'flex-basis'>` `FlexBasis::Cells(0)`. CSS Flexbox §7.2's text says `0`, but Chromium, Gecko and
  WebKit take `0%` (the web-compat reading; Chromium serializes `flex: 1` as `1 1 0%` and
  `flex-basis` as `0%` — from its serialization, not re-checked in a live browser here), and §7.3.3
  makes a percentage basis against an indefinite container main size `content`. Decision: follow
  the engines — the omitted basis is `Calc(Percent(0))`, which layout already resolved as `content`
  where the main size is indefinite (`main_axis` passes no percentage basis for an `auto`-height
  column) and as 0 where it is definite; rdom's serializer prints it `0%`. Red:
  `flex_tests::an_omitted_flex_basis_is_zero_percent` — `Cells(0)` for `Percent(0)`;
  `css_phase6/flex_basis_zero.rs` `flex_n_items_of_an_auto_height_column_size_by_content` — an
  equal split (2, 2) for (1, 3). Green after; `flex_n_items_of_a_definite_row_share_it_equally`
  (5 / 5 in a `width: 10` row) guards the definite case and passed before and after. Mutation check
  (restored and touched): the default back to `Cells(0)` → (2, 2). Existing expectation changed:
  `flex_shorthand_full_grammar`'s `2`, `2 3` and `1 0` rows (basis `Cells(0)` → `0%`), which
  encoded the old default. No snapshot changed (the showcase's `flex: 1` panes sit in definite
  containers).
- 2026-10-08 — C6G-COLLAPSE (AN7): `visibility: collapse` on a flex item, against CSS Flexbox §4.4
  and §9.4 step 10 ("note the cross size of the line they're in as the item's strut size, and
  restart layout […] treat the collapsed items as having zero main size [when collecting lines] …
  ignore the collapsed items entirely (as if they were `display: none`) except that […] if any
  line's cross size is less than the largest strut size among all the collapsed items in the line,
  set its cross size to that strut size"). C6-VISIBILITY made a collapsed item a zero-main-size
  item and measured its cross size at that zero size — `<span>a b c</span>` wrapped to three rows
  and held a 3-row line, the spec's 1 — and kept it an item of the line, so a gap sat on each side
  of it and `justify-content` gave it a share (its Log entry cited CSS Box Alignment §8.1 for the
  gaps; corrected there). Decision: the first round is only what the strut needs —
  `flex::strut::make_struts` breaks the *uncollapsed* items into lines (`collect_main_axis_items`
  now gathers a collapsed item like any other) and sizes each line that holds a collapsed item:
  a single-line container's line is its definite cross size; otherwise the line's outer
  hypothetical cross sizes with each item at its hypothetical main size (`line_cross_size`, the
  baseline extent included) — then `ChildMain::make_strut` zeroes the item's main size, factors,
  min / max and main margins and records `strut`. The second round is the ordinary algorithm,
  which reads `strut` where the spec ignores the item: line breaking (no size, no gap, joins the
  line it falls in), the line's gap total and border-collapse savings, `justify-content` (the
  shares go to the other items; a strut's offset is 0), the gap and pullback in placement, the
  baseline plan (a strut is placed at cross-start), and the line's cross size (at least the strut
  size); a strut's box is its strut size at the line's cross-start. Intrinsic sizing: a row's
  `auto` height runs the same two rounds (`lines_cross_size`, the cross size unknown, so measured),
  and the main-axis sum counts no gap beside a collapsed item. Red (`css_phase6/collapse.rs`):
  container height 3 for 1 (and the strut 3 rows); `b` at x 5 for 3 under `gap: 2`; `b` at x 5
  for 9 under `space-between`. Green after; C6-VISIBILITY's `a_collapsed_flex_item_leaves_a_strut`
  still holds. Mutation checks (each alone, restored and touched): the strut measured at main
  size 0 → height 3; the gap placed beside a strut → x 5; `justify-content` counting the strut →
  x 5. No snapshot changed.
- 2026-10-08 — C6G-VISIBILITY-ONE-ANSWER (AN8): four answers to "is this element rendered and
  visible" disagreed. Tab (`collect`) pruned `display: none` subtrees and skipped hidden elements,
  but the public `tab_index` / `is_focusable` / `is_tab_focusable` checked only the element's own
  `display` — true for a `visibility: hidden` button and for a button inside a closed box — so
  programmatic `focus()` (`TuiAccessorsMut::focus`, which asks `is_focusable`) focused both; a
  focused element that became hidden kept focus; and copy read the computed `visibility` while paint
  draws by the used one (`visibility_of`, a running transition's value). Decision: one predicate,
  `runtime::focus::tabindex::is_rendered_and_visible` (public) — no `display: none` on the element
  or an ancestor (`node::is_rendered`), and a used `visibility` of `visible` — HTML §6.6.2's "being
  rendered" requirement of a focusable area, with the engines' visibility rule (Chromium's and
  Gecko's focusability checks refuse a non-visible element). `tab_index` (so `is_focusable`,
  `is_tab_focusable`, `focus()`, autofocus and the dialog's restore) asks it; Tab's tree walk,
  which prunes `display: none` subtrees itself, asks its own-element half. The focus fixup (HTML
  "update the rendering": a focused area that is no longer a focusable area runs the focusing
  steps for the viewport, which fire `blur` / `focusout`): `focus::fix_up`, run by
  `App::draw_if_dirty` after a frame that cascaded, against that frame's styles. Copy asks
  `render::visibility::shows` of the text's owner, as paint does. Found on the way:
  `dialog::show_modal` runs the dialog focusing steps synchronously (HTML §4.11.4), before the
  cascade that renders the opened dialog, so with the ancestor walk its descendants read as
  `display: none` and three dialog tests failed (no focus moved in); a browser flushes style for
  the check. `tabindex::is_focusable_in_opened` reads the just-opened dialog as rendered and
  everything else as the predicate does (`autofocus::focus_within_opened`, `first_focusable_in`).
  The general case — code that shows a box and focuses into it in one handler, before the next
  cascade — is refused; recorded in DIVERGENCES (Runtime & focus). Red
  (`css_phase6/visibility_answers.rs`): `is_tab_focusable` true for the hidden button; `focus()`
  focused it; the button turned `h` / `n` kept focus, no `blur`; `rendered_text_tests::
  copy_reads_the_presented_visibility_mid_transition` — `"ac"` for `"aBc"`. Green after. Mutation
  checks (each alone, restored and touched): no ancestor walk → the `display: none` cases; no
  visibility check → all three focus tests; no fixup → the blur test; copy on the computed value →
  `"ac"`. No snapshot changed.
- 2026-10-08 — C6G-CSSOM-EDGES (AN14). Verified first against CSSOM §6.6 `removeProperty` ("if
  property is a shorthand, for each longhand property longhand that property maps to … remove
  longhand"; `margin`'s longhands are the four physical sides, CSS Box 4 §3.2, and
  `margin-inline-start` is a property of its own in the same logical group, CSS Logical 1 §4 —
  Chromium and Gecko leave it). Three edges. (1) `margin-inline-start: 1; margin-left: 2` keeps
  `margin-left` as a kept (`pending`) declaration after the flow-relative one, for order;
  `remove("margin")` dropped only kept declarations *named* `margin`, so the kept `margin-left`
  replayed at the cascade — left 2 after the removal, the browsers' 1. And a kept shorthand
  (`margin-inline-start: 1; margin: 3`) replayed whole after `removeProperty("margin-left")`.
  `table::remove_kept_longhands` now drops a kept declaration that sets only the removed
  property's fields and keeps one that sets others too, restricted. (2) `remove_inline_axis`
  split a kept shorthand into declarations of its remaining longhands by serializing them; a
  `var()` value (pending substitution, CSS Variables 1 §3) serializes to nothing per longhand, so
  removing `margin-inline-start` from `margin-inline: var(--m)` dropped `margin-inline-end` too. It
  now keeps such a declaration whole, restricted. Decision for both: `PendingDeclaration::
  restriction` (`var::Restriction`: `All`, `Without(longhands)` — mapped by the element's
  direction at replay — or `Within(mask)`), which the replay honours by writing the declaration
  apart and copying the fields it still sets (`property_dispatch::copy_fields`, over a
  `Field::EVERY` the table macro now emits), its `!important` bits restricted alike; CSSOM reads a
  restricted shorthand as not set (`""`), and the inline-axis longhand lookups skip a longhand it
  dropped. (3) `TuiStyle::first_line_subset` (`::placeholder`'s properties, CSS Pseudo-Elements 4
  §4.3) dropped `pending`, so `::placeholder { color: var(--c) }` set no colour; it keeps the kept
  declarations of the subset's properties (a shorthand like `background` restricted `Within` the
  subset) while one waits for substitution. The `::placeholder` overlay at
  `style_selector::rules_for` was dead and is removed, as redundant: no `::first-line` property is
  flow-relative and the subset keeps declarations only with a substitution, for which
  `directional_overlays` precomputes nothing. Red: `logical_tests::removing_margin_removes_a_kept_physical_longhand`
  ([0, 0, 0, 2] for [0, 0, 0, 1]); `removing_a_longhand_of_a_kept_shorthand_keeps_its_other_sides`
  ([3, 3, 3, 3] for [3, 3, 3, 1]); `removing_one_side_of_a_var_shorthand_keeps_the_other` (no
  right margin for 2); `cascade::tests::a_placeholder_color_from_a_custom_property_applies` (the
  UA grey for red). Green after. Mutation checks (each alone, restored and touched): the old
  name-only removal → (1) and its shorthand twin; no restriction recorded → the shorthand case;
  the `var()` split back → (2); the subset's kept declarations cleared → (3). Keeping a declaration whose every field was
  removed, restricted to nothing, instead of dropping it is an equivalent mutation (it writes no
  field) and survives. No snapshot changed.
- 2026-10-08 — C6G-DECLARED-COUNT (AN16): `TuiStyle::declared_count` was a hand copy of the
  field list (some 150 lines in `tui_style/mod.rs`) that had drifted — no `z-index`, `opacity`,
  `position`, insets, `box-shadow`, transition longhands, counters, and no flow-relative
  declaration (which writes no field until the cascade). Decision: derive it from `define_fields!`
  — the macro emits `Field::is_set` and `property_dispatch::set_field_count` folds over
  `Field::EVERY` — plus the custom properties and the kept declarations that write no field (a
  flow-relative one, or one waiting for substitution). A shorthand still counts its longhands, as
  the existing tests pin (15, 29). Red: `tui_style::tests::declared_count_counts_every_field_kind`
  — 3 for 14 (the expected count from `property_mask(name).count()` per declared property, plus
  the directional, the `var()` and the custom one). Green after. Mutation check (restored and
  touched): flow-relative declarations not counted → 13. `tui_style/mod.rs` 566 → 396 lines.
- 2026-10-08 — C6G-MINOR (AN19's parts outside the box tree). Behavioural, each red first: (1)
  `flex::cross::baseline_box` took a scroll container's last baseline from its content rows; CSS
  Box Alignment 3 §9.1 ("for legacy reasons … a block-level or inline-level block container that
  is a scroll container always has a last baseline set, whose baselines all correspond to its
  block-end margin edge") puts it at the margin box's bottom row, as `inline::vertical::atom_rows`
  already does for an inline block — which also keeps a horizontal scrollbar gutter row out of the
  answer. Red: `css_phase6/minor.rs` `a_scroll_containers_last_baseline_is_its_block_end_edge` — `b`
  at row 0 for 2 beside an `overflow: hidden; padding-bottom: 2` item. (2) `intrinsic::children`
  summed `row-gap` between every in-flow element child of a block container, while block layout
  places it between block-level children only (an inline child's anonymous block has none): a
  block of `p`, `span`, `p` with `row-gap: 2` measured 7 rows for its layout's 5 —
  `the_intrinsic_row_gap_skips_inline_children`, 7 for 5. (3) `ClampTo::NextLayout` (caret reveal)
  inferred the origin side from a negative minimum, so an `rtl` (or reversed) box that did not
  overflow yet clamped to `[0, ∞)` — the wrong side for one frame; `ScrollBounds` now carries
  `origin_at_end` from the box (`layout_pass::origin_at_end`, now crate-visible). Red:
  `rtl_tests::a_caret_reveal_keeps_an_rtl_offset_on_the_origin_side_before_overflow` — 0 for -3.
  Green after; mutation checks (each alone, restored and touched): each fix reverted fails its own
  test. Not behavioural, no test: `is_collapsed_table_row` reads the visibility before the tag
  compare (`is_in_flow` runs it for every in-flow element); `parse_order` uses
  `numeric::clamp_i32` (its range test unchanged); the `cell_sizer` doc says what it reads now (the
  computed style after a cascade — an app re-syncing — else the UA `<td>` / `<th>` padding with
  `content-box` or the cell's inline `box-sizing`, percentages against 0). No snapshot changed.
- 2026-10-08 — C6G-SPLITS (AN18), ahead of grid; pure moves, no behaviour change, no test
  changed. `rdom-style/src/computed.rs` 543 → 457: its tests to `computed_tests.rs` (`#[path]`).
  `tui_style/mod.rs` is 396 after C6G-DECLARED-COUNT, no split. `property_dispatch/serialize.rs`
  520 → `serialize/mod.rs` 62 (the substitution / flow-relative / CSS-wide prelude and the
  family chain) with one module per family, each `fn serialize(name, style) -> Option<Option<
  String>>` like the existing `background` / `border` ones: `box_model` 155 (overflow and scroll
  keywords, `margin-trim`, `direction` / `writing-mode`, `box-sizing`, sizes, `padding`,
  `margin`, `border-collapse`), `flex` 142 (`display`, flexbox, alignment, gaps), `paint` 131
  (color, font keywords, `text-decoration`, `opacity`, `white-space`, `user-select`,
  `pointer-events`, `visibility`, caret colors, `color-scheme`, `content`), `position` 100
  (position, insets, `z-index`, transitions, counters) — arms moved verbatim. Over 550, split by
  concern: `render/inline/packer.rs` 578 → `packer/mod.rs` 360 (state and intake: text,
  generated content, hard breaks, the grapheme and break rules) + `packer/emit.rs` 251
  (committing words, fragments, atoms, settling and breaking lines); `editing/movement/mod.rs`
  555 → `mod.rs` 358 (dispatch, horizontal moves, deletion) + `vertical.rs` 212 (Up / Down with
  sticky-x, line edges; re-exported at the old paths). Under 550, left and recorded in
  TECH_DEBT `SIZE-1`: `cascade/ladder.rs` 544, `block/mod.rs` 529, `layout/border.rs` 528,
  `inline_paint/mod.rs` 520, and from this batch `inline/mod.rs` 560, `property_dispatch/table.rs`
  531, `flex/mod.rs` 516.
- 2026-10-08 — C6G-BLOCKIFY (C6G-FLEX-SPEC's DIVERGENCES §3 finding): a flex item's computed
  `display` is blockified (CSS Display 3 §2.7, CSS Flexbox §4) at computed-value time, so every
  reader of `ComputedStyle` and every layout path see one answer. `cascade::apply::blockify` maps an
  inline-level outer type to `block` and keeps the inner one — `inline` → `block`, `inline-flex` →
  `flex`, `inline flow-root` → `flow-root` — and `inline-block` (rdom's `inline flow-root`) to
  `block flow-root`; `block`, `contents`, `none` and the `list-item` flag are kept. It runs after
  Appendix B's `contents` → `none` and before the BFC predicate, for an element whose parent box is
  a flex container (`children_are_flex_items`: the parent, or past a `display: contents` parent its
  nearest boxed ancestor, §2.5) and for a flex container's `::before` / `::after` (child boxes, CSS
  Pseudo-Elements 4 §4) — a box-less host's pseudos ask its parent box. A restyle (`Mode::Restyle`)
  no longer keeps a `contents` element's subtree when its own style is unchanged: its children
  read their parent box from above it. Decision on the document root's children: not blockified.
  rdom lays them out in its viewport column only as a layout device standing in for a browser's
  `<body>`, whose children are blocks and inlines in normal flow, not flex items (a non-element
  parent is no flex container); so they keep their computed `display`, and an atomic inline among
  them hugs its content. Batch B's special case (`hugs_as_inline_level` asking whether the item's
  box parent is a flex container) is now a read of the computed style alone — an atomic inline
  there can only be a root child — and covers `inline-flex` like `inline-block`. Red
  (`css_phase6/blockify.rs`): `(Inline, Block)` for `(Block, Block)` (a `span` item; the
  `inline-block` / `inline-flex` / `inline flow-root` rows behind it), the `contents` child's `span`
  likewise, `(InlineBlock, Block)` for `(Block, FlowRoot)` on `::before`, `Inline` for `Block` before a
  toggle; `the_document_roots_children_are_not_blockified` passed before and after. Green after;
  `apply_tests::a_restyle_unblockifies_the_children_of_a_contents_item` added with the restyle guard.
  Mutation check (restored and touched): the guard off → `Block` for `Inline`. DIVERGENCES §3's
  entry removed. The two functions are `cascade/blockify.rs` (55 lines; `apply.rs` stays 464);
  `walk.rs` 564 → 575, recorded in TECH_DEBT `SIZE-1`. No other test expectation and no snapshot
  changed.
- 2026-10-08 — C6G-PSEUDO-FLEX-ITEMS (C6G-ANON-FLEX-ITEMS' DIVERGENCES §3 finding): a `::before` /
  `::after` flex item (CSS Flexbox §4: a child box of the flex container, or of its box-less child,
  blockified since C6G-BLOCKIFY) applies its own box properties. C6G-ANON-FLEX-ITEMS laid it out as
  an anonymous item with the anonymous box style, so its sizes, `flex`, `order`, margins, padding,
  border and alignment were ignored. Decision: one box model for an item without a node.
  `flex::anonymous::AnonymousItem` (split out of `item.rs`, which would have passed 500 lines) carries
  its computed style — the anonymous box style for a text run, the pseudo-element's own
  (`computed_before` / `computed_after`) for a generated item — and its edges (padding plus border,
  percentages against the container's width); its measures are border boxes, its content packed
  inside the edges. The flex algorithm's anonymous-only branches are gone: `main_axis`,
  `distribute::resolve_auto_min`, `cross::resolve_cross_size` and the intrinsic contribution
  (`intrinsic/children.rs`) run the element path for every item through `FlexItem::keywords` /
  `intrinsic_size` / `content_extreme` (a `Keywords` now measures an element or a run,
  `Keywords::for_run`), and `FlexItem::order` reads the box's `order`. A text run's anonymous box
  has initial sizes, no edges and `order: 0`, so it lays out as before. Laid out, a generated item's
  `AnonymousIfc` holds its content box in `rect` (where its lines sit, as paint, hit-testing and the
  caret read it) and its border box in the new `generated: Option<GeneratedBox>` (host, slot, border
  box), which content shifting, the scroll extent and the paint group bounds read
  (`AnonymousIfc::border_box`); paint draws its background and border there before its lines
  (`paint_pass/generated_box.rs`, with a running transition's `background-color` / `border-color`
  and its own `visibility`). Public API: `AnonymousIfc` is `#[non_exhaustive]` with
  `AnonymousIfc::new` (it gains a field; Breaking bullet), `GeneratedBox` is new
  (`#[non_exhaustive]`, `GeneratedBox::new`). Not done: a generated item's `aspect-ratio` (§9.2 step
  3.B reads an element's), and hit-testing still resolves its cells to the container (DIVERGENCES §2,
  pseudo-elements are not hit-test targets). Red (`css_phase6/pseudo_items.rs`): the next item at x 2
  for 8 (no margin, width, padding or border); at x 0 for 7 (`flex: 1; order: -1` ignored);
  `["p   ", …]` for `p` on the bottom row (`align-self: flex-end`); container width 2 for 6
  (`width: max-content` without the item's padding and margin). Green after (the first test's row
  expectation corrected while green: it had miscounted the border box's columns). Mutation checks
  (each alone, restored and touched): `order` back to 0 → the order test; the box paint call off →
  the box test (no border drawn). No other test expectation and no snapshot changed. DIVERGENCES
  §3's entry removed.
- 2026-10-08 — C6G-SCROLL-API (PN1): the scroll docs named only `rtl` (for `scrollLeft`) and
  `column-reverse` (for `scrollTop`) as origins at the right / bottom edge, but
  `scroll_extent::origin_at_end` reads `AxisFlip`: a flex row's main-start is its right edge under
  `row-reverse` XOR `rtl` (CSS Flexbox §5.1), a flex column's cross-start under `rtl` XOR
  `wrap-reverse` and a flex row's cross-start (its bottom) under `wrap-reverse` (§5.2), each a
  negative range. Corrected: `TuiExt::scroll_x` / `scroll_y`, `TuiAccessors::scroll_left` /
  `scroll_top`, `TuiAccessorsMut::set_scroll_left`, `layout_pass::scroll_x_bounds` /
  `scroll_y_bounds`, `ScrollBounds`, and the two CHANGELOG Breaking bullets; `set_scroll_top`'s
  false "clamped to `[0, scroll_height - viewport_height]`" is now the legal range. Decision: a
  public read rather than a public `origin_at_end` — `TuiAccessors::scroll_range() ->
  Option<ScrollRange>` (`ScrollRange::x()` / `y()`: `RangeInclusive<i32>`, private fields with
  `new`, so it can grow without a break), from the same `scroll_bounds` the writes clamp with
  (the padding-box scrollport, CSS Overflow 3 §3), so a virtual table need not reimplement
  `AxisFlip`; `TuiAccessors` is sealed, so the method is not a break. Red
  (`css_phase6/scroll_range.rs`): failed to compile (no `ScrollRange`, no `scroll_range`). Green
  after: `0..=6` / `0..=0` for an `ltr` box, `-6..=0` for a `row-reverse` row, `-2..=0` for a
  `wrap-reverse` row's `scrollTop`, `None` for a text node. No snapshot changed.
- 2026-10-08 — C6G-ALIGN-API (PN2, PN3, PN14). (1) The six alignment builders were `setter!`s typed
  `Alignment`, so `.justify_content(Align::Center)` did not compile (the hints wrote `.into()`); they
  take `impl Into<Alignment>` (`align_setter!`), and a bare `Align::X.into()` argument no longer
  infers — dropped from the tests and the migration hints. (2) Out-of-grammar values
  (`.align_self(Align::SpaceBetween.into())`) were stored and laid out as `start`. Decision:
  per-property validation, from one grammar table — `AlignProperty::grammar` (the `Grammar` records
  move from the parser to `layout/alignment.rs`; `parse::values::align` reads them) behind the public
  `Alignment::is_valid_for(AlignProperty)`. Consistent with the other builders, which return `Self`
  and never a `Result`: where a value has an in-range neighbour a builder keeps it there
  (`valid_flex_factor` clamps a negative factor); an alignment keyword has none, so the setter
  `debug_assert!`s (a programming error, loud in tests) and in a release build leaves the field
  unset, as a CSS parser drops an invalid declaration. Recorded in DESIGN's `#[non_exhaustive]`
  section beside the debug-asserting wildcard. (3) `ComputedStyle::flex_direction() ->
  FlexDirection` (new, closed: `row | row-reverse | column | column-reverse`), with
  `FlexDirection::new` / `axis` / `is_reversed`, `TuiStyle::flex_direction` /
  `flex_direction_important` and the missing `direction_reverse_important`. (4) `Align` is
  `#[non_exhaustive]` ahead of `anchor-center` (C15-ANCHOR): DESIGN listed it as closed data (a
  renderer wildcard a silent fallback); it moves to the open vocabularies — a consumer reading an
  alignment can treat an unknown keyword as `start`, and `is_valid_for` keeps the grammars checkable
  without matching every keyword — and rdom-tui's four exhaustive matches (`block/align.rs`,
  `flex/align.rs`, two in `flex/content.rs`) keep a wildcard that `debug_assert!`s, per the DESIGN
  rule for sibling crates. DESIGN also corrected: line boxes / fragments, `AnonymousIfc` and
  `GeneratedBox` are `#[non_exhaustive]` with constructors (C6G-LINEBOX-API,
  C6G-PSEUDO-FLEX-ITEMS). Red: the rdom-style lib tests failed to compile (no `FlexDirection`, no
  `AlignProperty`, `justify_content(Align::Center)` not `Into`-generic). Green after:
  `the_typed_check_agrees_with_each_propertys_grammar` (17 keywords × default / `safe` / `unsafe` /
  `legacy` × 6 properties: valid exactly when the serialized value parses back for the property),
  the `should_panic` refusal, the builder and `flex_direction()` tests, and the
  `migration_hints::alignment_api_hints` group. Mutation check (restored and touched): `left` /
  `right` accepted by every grammar → `AlignContent: left` fails the agreement test. Existing test
  code changed: thirteen `Align::X.into()` builder arguments (no longer inferable) lost the `.into()`.
  No snapshot changed.
- 2026-10-08 — C6G-SIDE-SETTERS (PN4, PN9, PN10). Since C6-MARGIN-SIDES a side is a longhand of its
  own (CSS Box 3 §3.2 / §4.2), but the builders only wrote all four. Builder: `margin_top` / `_right`
  / `_bottom` / `_left` (`impl Into<MarginValue>`) and `padding_*` (`impl Into<PaddingValue>`), each
  with an `_important` twin that marks its side's bit alone (`side_setter!`, in the new
  `tui_style/builder/spacing.rs` beside the moved shorthands — `builder/mod.rs` 465 → 438), and
  `padding` takes `impl Into<Padding>` as `margin` takes `impl Into<Margin>` (`From<u16> for Padding`,
  `From<u16> for PaddingValue`, `From<i16> for MarginValue`). Not added: logical-side builders
  (`margin_inline_start` …) — a flow-relative declaration is a kept declaration mapped at cascade
  time (C5-LOGICAL), not a field, so a typed setter would need a `PendingDeclaration` constructor;
  `property_dispatch::set("margin-inline-start", …)` covers it. Node setters, following Phase 5's
  pattern (`write_inline_style`): `set_visibility`, `set_order`, `set_flex_wrap`,
  `set_flex_direction`, the six alignment setters (`impl Into<Alignment>`, through the builders, so
  checked against each grammar as C6G-ALIGN-API decided), `set_margin`, and `set_padding(impl
  Into<Padding>)`. Prelude: `Margin`, `Visibility`, `GapValue`, `FlexDirection`. CHANGELOG: the
  C6-MARGIN-SIDES hint now shows the per-side forms for exactly rdom-virtualtable's two breaks (a
  whole-`Margin` assignment to `TuiStyle.margin`; `padding.is_none()`), pinned by
  `migration_hints::spacing_assignment_hints`; its "builders unchanged" sentence was no longer true.
  Red: the rdom-style lib tests and the integration tests failed to compile (no `margin_left`,
  `padding(2u16)` not accepted, no `set_flex_direction` / `set_order` / …, `Margin` / `Visibility`
  / `GapValue` / `FlexDirection` not in the prelude). Green after:
  `per_side_setters_write_one_longhand`, `padding_takes_a_count_as_margin_does`,
  `node_setters_drive_layout::phase6_node_setters_drive_layout` (a row-reverse, centered, stretched
  container built only through node setters: `b` at (6, 4), `a` ordered after it at x 4). Mutation
  check (restored and touched): `set_order` writing nothing → `(0, Hidden)` for `(1, Hidden)`. No
  snapshot changed.
- 2026-10-08 — C6G-FRONTEND-API (PN11). `property_dispatch::set_from_source(name, value, text)` had
  no importance: rdom-css called `set_important` after it, a call-order protocol a second front end
  would have to rediscover (an inline-axis declaration's importance rides on its kept declaration,
  C5G-LOGICAL-IMPORTANT, so the order matters), while `set_custom_source` takes `important`.
  Decision: the same shape — `set_from_source(name, value, text: Option<&str>, important, style)`;
  the one body (`set_with_text`) takes the priority, routes a custom property to
  `set_custom_source` with it, and marks any other after declaring it (`set_important`: the
  fields' bits, or the kept declaration's flag). rdom-css's `apply_declaration` makes one call.
  `parse::token::SpannedTokens` was a 3-tuple of parallel `Vec`s; it is a struct (`tokens`,
  `positions`, `spans`; closed, a parse result whose every field is meaningful, DESIGN), built in
  place by `tokenize_spans`. Red: the lib tests failed to compile (five arguments to a
  four-argument `set_from_source`; no `tokens` / `positions` / `spans` fields). Green after:
  `set_from_source_takes_important` (a plain property, a custom one, a `var()` value, an inline-axis
  one, and a normal one left normal) and `tokenize_spans_names_its_parallel_lists`. Mutation check
  (restored and touched): the importance dropped for non-custom properties → `color` not important.
  No snapshot changed.
- 2026-10-08 — C6G-DOCS (PN5, PN6, PN7, PN12). Stale docs fixed (PN5): `Margin`'s "rdom does not
  collapse vertical margins" (it does, since BFC-1; M5-MARGIN-1 retired); `layout/keywords.rs`'s
  module doc (listed `align-items`, which lives in `alignment`; now the enums it holds);
  `Display::Block` ("Standalone flex item" — a block-level box, a flex item only in a flex
  container); `ifc.rs`'s `InlineBlock` arm (an atom, skipped above); the rdom-tui README's inline
  "out of scope" (baseline alignment is supported: atoms on the text row, flex `align-items:
  baseline`; `vertical-align` is C9); the rdom-style README's "every field is an
  `Option<Value<T>>`" (the per-side longhands are a `Sides` / `Corners` of them); CSS-COVERAGE's
  `row-gap` row ("`flex-wrap`'s lines will take the other" — they do). PN6: the two gaps the
  `align-content` and `justify-items` / `justify-self` rows over-claimed are fixed rather than
  marked Partial. (1) CSS Box Alignment 3 §5.1 aligns a block container's content as a whole, its
  lines too: `layout_children_aligned` computed a lead only from a block measurement, so an IFC or
  a text leaf (`inline_layout`, no `BlockMeasurement`) never moved; it now takes the lines' height
  and, for inline content, moves the lines (`tree::shift_lines`, `LineBox::top`) and the atoms'
  boxes (`tree::shift_content`). `LineBox::top` is unsigned, so an upward lead (inline content
  overflowing under an unsafe `end` / `center`) is not applied — recorded in DIVERGENCES §4. The
  auto height is unchanged (the lines end at most at the `min-height` that made the free space).
  (2) §6.1 / §6.2 with CSS Position 3 §4.1: an absolutely positioned box's `justify-self` /
  `align-self` (`auto` is `normal` for it) other than `normal` / `stretch` aligns its margin box in
  the inset-modified containing block — the containing block less the insets, an `auto` one 0 beside
  a non-`auto` one — with `justify_offset` (shared with block layout; `start` / `end` by the
  containing block's direction, `self-*` by the box's, baselines as `safe self-*`, `safe`), and an
  aligned `auto` size is `fit-content` there; `auto` margins on both sides win (§6.1);
  `place::self_align` / `inset_modified`. Not modeled, DIVERGENCES §4: both insets `auto` (CSS
  Position 3's static-position rectangle) and positioned pseudo-elements. DIVERGENCES §3's two
  entries removed. PN7: §1's "Flex free space is shared in whole cells" is "Alignment free space",
  naming block `justify-self` / `align-content` and absolutely positioned `align-self` / `auto`
  margins (all round the leading space down). PN12: ACID tiles 7 and 16 name the Phase 6 features
  (directions and reverses, wrap, `order`, the alignment properties, anonymous / pseudo items;
  `inline-flex`, `contents`, `flow-root`, the multi-keyword syntax, `visibility`, blockification).
  The rdom-tui README says `display: flex` is a row and block containers ignore `flex-direction`.
  Red (`css_phase6/gaps_closed.rs`): `["ab  ", …]` for the text on row 4 under `align-content: end`;
  an atom in an IFC at row 0 for 2 (the first version of this test, text plus an atom only, went
  through the anonymous-block path and passed before the fix — an inline `<i>` made it an IFC);
  `justify-self: end` at x 2 for 14; `center` at `(0, 20)` for `(9, 2)`; `align-self: end` at y 0 for
  8; one `auto` inset at x 4 for 17. `auto_margins_win_over_justify_self` passed before and after.
  Green after; the red runs are the mutation checks (each fix reverted fails its own tests). No
  other test expectation and no snapshot changed.
- 2026-10-08 — C6G-CHANGELOG (PN8, AN17, the restructure): `[Unreleased]` was about 155 KB in 279
  bullets — most past three lines, the longest near 1,900 characters — which no consumer could use.
  Restructured: "Upgrading from 0.5" in two parts — **silent behaviour changes** ordered by impact
  (`display: flex` a row; content-box with what the border-box reset does not restore; the flex
  base size and automatic minimum, with the `min-width: 0` advice for `flex: 1` panes holding wide
  content (AN17); block containers ignoring `flex-direction`; the `border` reset; `display: inline`
  resetting the inner type; per-side longhands cascading alone; CSSOM's shortest-form and as-written
  serialization; signed scroll offsets; blockified flex items; `transparent`; anonymous and pseudo
  flex items; `min-*: auto` on the cross axis; ill-typed math) and **compile breaks** by kind — then
  "API changes from 0.5", a per-crate old → new table (63 rows: the forks' 58 rename / reshape rows
  plus five that gather the struct-literal / pattern breaks, the new enum arms, the `flex-direction`
  default, the `flex` shorthand's fields and the root re-exports), whose last column names the
  `migration_hints.rs` group compiling the new form; then the Breaking / Added / Changed / Fixed
  sections with every bullet kept (279), each ending in its item ids, at most about three lines
  (≤ 341 characters) — except four rdom-tui bullets whose ids (`EDIT-CLICK-IN-CONTROL-1`,
  `ANON-WHITESPACE-RUN-1`, `INPUT-SEED-ON-INSERT-1`, the six-id terminal-reader bullet) have no Log
  entry to hold their detail. The section is about 88 KB. Nothing lost: facts a bullet dropped that
  its item's Log entry lacked were appended to that entry ("From its CHANGELOG bullet"), 24 entries;
  the 22 items with no Log entry of their own (Phase 1–3 items recorded only in the item tables)
  have their 55 original bullets kept verbatim in the next entry. Fixed in passing: the C6-GAP hint's
  "match `ROW_GAP | COLUMN_GAP`" is `contains(ImportantMask::ROW_GAP | ImportantMask::COLUMN_GAP)`
  (`ImportantMask` is opaque); the C5-BOX-SIZING caveat moved into the upgrade list. The 0.5.0 and
  older sections are byte-identical. `migration_hints.rs` gains the groups the table needed —
  `typed_error_hints` (C1G-TYPED-ERRORS, C2G-SUBSTITUTION-ERRORS), `sealed_trait_hints`
  (C4G-SEALED, C2-VIEWPORT, C3-SCHEME), `front_end_hints` (C6G-FRONTEND-API),
  `anonymous_box_hints` (C6G-PSEUDO-FLEX-ITEMS) — and `sizing_hints` / `flex_longhand_hints` cite
  and check C2-NUMBER, C2-RATIO, C2G-LAYOUT-SAFETY (`aspect_ratio: Option<Value<Option<…>>>`) and
  C2G-FLEX-SHORTHAND (`FlexShorthand`); every group is named by a row, and every row but one (the
  `ActiveAnimation` field, which no consumer builds) by a group. Red: the new groups ran green on
  first build bar a type-alias slip (`rdom_tui::Result` shadows `std::result::Result` under the
  glob import); they pin existing API, so there is no behaviour to fail first. Docs-only otherwise.
- 2026-10-08 — C6G-CHANGELOG (narrative moved): the restructured CHANGELOG keeps a bullet of at most
  about three lines per change; the items below had no Log entry of their own (their record was the
  item table and their CHANGELOG bullet), so their CHANGELOG bullets are kept here verbatim, as they
  stood before the restructure: C1-ALL, C1-CASE, C1-ESCAPES, C1-IMPORT, C1-INLINE-IMPORTANT,
  C1-LAYER, C1-NESTING, C1-PROPERTY, C1-REVERT, C1-SCOPE, C1-VAR-ANY, C2-STEPPED, C3-ALPHA,
  C3-CURRENTCOLOR, C3-HSL-HWB, C3-LAB, C3-MIX, C3-RELATIVE, C3-RGB, C3-SCHEME, C3-SYSTEM,
  C3-TRANSPARENT.
  - **Selectors decode CSS escapes** (CSS Syntax 3 §4.3.7). Type, class, id and attribute names and
    attribute values — quoted or not — decode `\` + 1–6 hex digits (one following whitespace belongs
    to the escape) and `\` + any other code point: `.\31 0` matches class `10`, `#a\:b` matches id
    `a:b`, `[title="a\"b"]` matches `a"b`. A `\` before a newline does not continue an identifier.
    The decoder is the new public module `rdom_core::css_syntax` (`consume_escape`, `consume_ident`,
    `consume_string`, `would_start_ident`, …), which rdom-style's tokenizer shares. (C1-ESCAPES)
  - **Nested rule selectors** (CSS Nesting 1 §2): `selectors::parse_nested(text, &parent)` parses a
    nested style rule's selector against its parent rule's list — `&` anywhere (`&.x`, `.x &`,
    `:not(&)`), a leading combinator (`> p`, `+ p`, `~ p`) anchored at `&`, and an implicit `& `
    descendant prefix when the selector has neither; a one-item parent is spliced in place of `&`
    where that is equivalent. `&` resolves to the new `SimpleSelector::Is(list)`, which matches like
    `:is()` and counts the specificity of its most specific item; the `:is()` text itself stays
    C11-IS. `selectors` is now a directory module (`parser.rs`, `nesting.rs`). (C1-NESTING)
  - **`:scope`** (Selectors 4 §14.3): `PseudoClass::Scope` matches the scoping root given to the new
    `Dom::matches_list_in_scope(id, list, scope)` — an `@scope` root — and `:root` without one
    (`matches_list` passes none); a `&` outside a nested rule parses as `:scope` (CSS Nesting 1 §2).
    `selectors::parse_scoped(text)` parses a scoped style rule's selector (CSS Cascade 6 §2.5.2):
    relative to `:where(:scope)` (no added specificity), `&` is `:where(:scope)`, and a selector
    holding `:scope` or `&` is absolute. The query APIs do not set `:scope` to their root yet
    (C11-SCOPE). (C1-SCOPE)
  - **Pseudo-class names are ASCII case-insensitive** (Selectors 4 §3.1): `a:HOVER` is `a:hover`.
    (C1-CASE)
  - `Value<T>` gains `Revert`, the CSS-wide keyword `revert` (CSS Cascade 4 §7.3), which depends on
    the declaration's origin and so is stored as written for the cascade to resolve (`unset` stays
    resolved at parse time). `Value` is closed data by design (DESIGN: non-exhaustive rule), so this
    is breaking. Migration: add a `Value::Revert` arm to matches on `Value` — in a cascade, roll
    back to the user-agent origin's value; elsewhere treat it like the other keywords. (C1-REVERT)
  - `Value<T>` gains `RevertLayer`, the CSS-wide keyword `revert-layer` (CSS Cascade 5 §7.4),
    resolved by the cascade like `Revert`. Migration: add a `Value::RevertLayer` arm next to
    `Value::Revert`. (C1-LAYER)
  - `TuiStyle` gains the field `pending: Vec<PendingDeclaration>` (declarations holding `var()`, CSS
    Variables 1 §3), and the color grammar no longer parses `var()`: CSS text `color: var(--x)` is
    kept on `pending` for the cascade instead of becoming `TuiColor::Var` (which stays, for
    Rust-built styles). Once a declaration block has one `var()` declaration, every later
    declaration in that block is recorded on `pending` as well as in its typed field (in order, so
    the cascade replays it after the `var()` one) — a reader of `pending` sees those later
    declarations too; `parse::values::parse_var_args` is removed. Migration: build a `TuiStyle` with
    `TuiStyle::new()` / `..Default::default()` rather than a full struct literal; read a parsed
    `var()` declaration from `pending` (or serialize it); call `TuiStyle::substituted(&vars)` to
    resolve it. (C1-VAR-ANY)
  - `ComputedStyle` gains `animated_vars: Option<VarMap>` — the custom properties with running
    transitions applied, which `var()` substitution reads (CSS Properties and Values API 1 §6.2).
    Migration: build a `ComputedStyle` from `ComputedStyle::initial()` rather than a full struct
    literal; destructuring patterns add `animated_vars: _`. (C1-PROPERTY)
  - `Color` gains `Rgba(r, g, b, a)`, a truecolor with alpha below 255 (CSS Color 4 §4.2), with
    `Color::rgba` (normalizes an opaque alpha to `Rgb`), `alpha()`, `is_translucent()`, `opaque()`
    and `Color::TRANSPARENT`; hex `#rgba` / `#rrggbbaa` and `rgb()` keep their alpha instead of
    dropping it. `Color` is closed data (DESIGN), so this is breaking. Migration: add a
    `Color::Rgba(r, g, b, a)` arm to matches on `Color` — or match `c.opaque()` where alpha does not
    matter; build a color with alpha through `Color::rgba`. (C3-RGB)
  - `TuiColor` gains `CurrentColor` (`currentcolor`, CSS Color 4 §6.4), resolved at computed-value
    time, and `resolve_tui_color` takes a `&ColorContext` (the new `#[non_exhaustive]` struct
    holding `current_color`, the color `currentcolor` is). `TuiColor` is closed data, so this is
    breaking. Migration: add a `TuiColor::CurrentColor` arm (resolve it to the element's color —
    `TuiColor::resolve` does); pass `&ColorContext::new(element_color).with_scheme(scheme)` to
    `resolve_tui_color` (the element's used `ColorScheme`; `ColorContext::new` alone is dark).
    (C3-CURRENTCOLOR)
  - `TuiColor` gains `Function(ColorFunction)`: a color function whose value depends on the element
    (`color-mix(in srgb, currentcolor, blue)`), kept in a parsed form (C3G-COLOR-FUNCTION-PARSED)
    with its CSS text (`ColorFunction::css_text`) and computed at computed-value time
    (`ColorFunction::compute(cx)`, also through `TuiColor::resolve`). Migration: add a
    `TuiColor::Function(f)` arm — resolve it with `f.compute(&ColorContext::new(element_color))`, or
    serialize it with `f.css_text()`. (C3-MIX)
  - `TuiColor` gains `System(SystemColor)` (CSS Color 4 §6.2), which serializes as its keyword and
    resolves through `SystemColor::color`. Migration: add a `TuiColor::System(s)` arm — `s.color()`
    is its value (`s.definite(scheme)` where a definite sRGB color is needed, `scheme` a
    `ColorScheme`). (C3-SYSTEM)
  - `TuiStyle` and `ComputedStyle` gain `color_scheme` (`color-scheme`, CSS Color Adjust 1 §2;
    `ColorSchemeList`, initial `normal`), and `ImportantMask` `COLOR_SCHEME`. Migration: build a
    `TuiStyle` / `ComputedStyle` with `TuiStyle::new()` / `ComputedStyle::initial()` rather than a
    full struct literal; a destructuring pattern adds `color_scheme`. (C3-SCHEME)
  - **The value tokenizer decodes identifier escapes** (CSS Syntax 3 §4.3.7 / §4.3.11) through
    `rdom_core::css_syntax`, so they work in property names (`col\6f r: red`) and keyword values
    (`display: fl\65x`); an identifier may start with an escape. A selector list no longer splits on
    an escaped comma. New `Cursor::rest` / `Cursor::advance`. (C1-ESCAPES)
  - **`revert`** is accepted for every property (ASCII case-insensitive) and serializes as written.
    (C1-REVERT)
  - **The `all` shorthand** (CSS Cascade 4 §3.2): `all: initial | inherit | unset | revert |
    revert-layer` sets every property in the dispatch table — `unset` resolved per property — except
    `direction`, `unicode-bidi` (when they land) and custom properties; `!important` covers every
    property, `removeProperty("all")` clears them, any other value is invalid. It is derived from
    `PROPERTY_NAMES`, so a property added to the table is covered automatically. (C1-ALL)
  - **Cascade layers in the data model** (CSS Cascade 5 §6.4): `Rule::layer: Option<LayerId>`; a
    sheet records the layers it declares in order of first declaration (`Stylesheet::layers`, `Layer
    { name, parent }`) through `declare_layer(parent, &["a", "b"])` /
    `declare_anonymous_layer(parent)`, and `add_style_rule(&selector, style,
    RuleContext::default().in_layer(layer))` adds a rule to one. `LayerOrder::new(&sheets)` merges
    the layers of the sheets of one cascade by name in sheet order and ranks them (siblings by first
    declaration, sublayers below their parent's own rules, unlayered last).
    `Stylesheet::append(&other)` appends another sheet's rules, layers (named ones merged) and root
    variables. `revert-layer` is accepted for every property. (C1-LAYER)
  - **`StyleSelector`**: a style rule's selector list parsed once — `StyleSelector::parse(text)` or,
    for a nested rule, `StyleSelector::parse_nested(text, &parent)` (`&` stands for the parent's
    items without a pseudo-element, `nesting_list()`) — and `Stylesheet::add_style_rule(&selector,
    style, ctx)`, which adds one rule per item at a `RuleContext` (layer and scope, built with
    `RuleContext::default().in_layer(…).in_scope(…)`). `Stylesheet::rule` / `add_rule` go through
    it. (C1-NESTING)
  - **`@scope` in the data model** (CSS Cascade 6 §2.5): `Rule::scope: Option<ScopeId>`; a sheet
    records its scopes (`Stylesheet::scopes`, `declare_scope(Scope::new(start, end, parent))`, each
    with its resolved `<scope-start>` / `<scope-end>` selector lists and enclosing scope) and its
    CSSOM owner node (`owner_node` / `set_owner_node`), where a prelude-less `@scope` roots.
    `StyleSelector::parse_scoped` parses a scoped rule's selector. `Stylesheet::append` carries
    scopes over. (C1-SCOPE)
  - **`Import` records** (CSS Cascade 5 §3): `Stylesheet::imports()` lists the `@import`s that
    loaded — URL, the layer the imported rules sit in, and the `supports()` / media condition text —
    and `record_import` adds one; `append` carries them over. (C1-IMPORT)
  - **`var()` in every property** (CSS Variables 1 §3): `PendingDeclaration` and the backend hooks
    (now `rdom_style::backend`, C1G-API-SURFACE) `contains_var`, `valid_var_syntax`,
    `substitute(tokens, lookup, cx)` (fallbacks of any tokens, nested `var()`),
    `resolve_custom_properties(vars, names, cx)` (custom properties substituted where declared; a
    dependency cycle removes every property on it, §2.3) and `TuiStyle::substituted(&vars, &cx)`,
    which replays a block's pending declarations through the property grammar and sets a property
    invalid at computed-value time to `unset`. `property_dispatch::set_from_tokens` keeps a `var()`
    value as tokens after checking its `var()` syntax (`backend::set_parsed` / `backend::set_unset`
    are the typed halves), `serialize` returns it as written, `remove` drops it. `tui_style.rs` is
    split into `tui_style/{mod,builder,tests}.rs`. (C1-VAR-ANY)
  - **Registered custom properties** (CSS Properties and Values API 1): the `registration` module —
    `PropertySyntax` (parse / `matches` / `interpolation`), `SyntaxComponent`, `Multiplier` and
    `PropertyRegistration::new(name, syntax, inherits, initial_value)`, which validates as
    `CSS.registerProperty` does — and `Stylesheet::register_property` / `registered_properties`
    (carried by `append`). Supported syntax components: `*`, `<length>`, `<number>`, `<integer>`,
    `<percentage>`, `<length-percentage>`, `<color>`, `<time>`, `<custom-ident>`, keywords, `+` /
    `#`, `|`. A custom property named in `transition-property` keeps its case. (C1-PROPERTY)
  - **`round()` / `mod()` / `rem()` / `abs()` / `sign()`** (CSS Values 4 §10.3, §10.7):
    `round(<rounding-strategy>?, A, B?)` with `nearest` (ties toward +∞), `up`, `down`, `to-zero`
    and B defaulting to 1; `mod()` takes the divisor's sign, `rem()` the dividend's; the spec's
    argument-range rules (a zero step is NaN, infinities) hold.
    `MathFunction::{Round(RoundingStrategy), Mod, Rem, Abs, Sign}`, `RoundingStrategy`. (C2-STEPPED)
  - **Modern `rgb()` / `rgba()`** (CSS Color 4 §5.1): space-separated channels with `/ alpha`,
    `none`, numbers with fractions and percentages (mixed in the modern syntax), math functions in
    channels and alpha, out-of-range values clamped; the legacy comma syntax takes all-number or
    all-percentage channels and an optional alpha. `color::serialize_alpha` gives the CSSOM alpha
    text; a translucent color serializes as `rgba(r, g, b, a)`. A translucent color paints opaque
    until C3-ALPHA. (C3-RGB)
  - **`currentcolor`** (CSS Color 4 §6.4), in any case, in every color property; serializes as
    `currentcolor`. `TuiColor::parse(css)` parses the full grammar keeping it,
    `TuiColor::substitute_vars(vars)` looks up `var()` references, `TuiColor::resolve(vars, cx)`
    computes the color, and `depends_on_element()` says whether a value waits for the element's
    `color`. (C3-CURRENTCOLOR)
  - **`hsl()` / `hsla()` / `hwb()`** (CSS Color 4 §7, §8): the hue a number of degrees or an
    `<angle>` (wrapping), saturation / lightness / whiteness / blackness as percentages or numbers,
    `none`, `/ alpha`, math functions; `hsl()`'s legacy comma syntax (percentages only). Converted
    to sRGB at parse time, as CSS computes them. (C3-HSL-HWB)
  - **`lab()` / `lch()` / `oklab()` / `oklch()` and `color()`** (CSS Color 4 §9, §10) with the
    predefined spaces `srgb`, `srgb-linear`, `display-p3`, `a98-rgb`, `prophoto-rgb`, `rec2020`,
    `xyz` / `xyz-d65`, `xyz-d50`: numbers, percentages, hues, `none`, `/ alpha`, math functions.
    They convert through XYZ (the spec's matrices, Bradford D50 / D65 adaptation) and are
    gamut-mapped into sRGB by CSS gamut mapping (§13.2: OKLCh chroma reduction to within a deltaEOK
    of 0.02) — dependency-free, in `color::{convert, gamut}`. `color::interpolate_oklab(from, to,
    t)` interpolates two sRGB colors in Oklab with premultiplied alpha (§12). (C3-LAB)
  - **`color-mix()`** (CSS Color 5 §2): `in` any rectangular space (`srgb`, `srgb-linear`,
    `display-p3`, `a98-rgb`, `prophoto-rgb`, `rec2020`, `lab`, `oklab`, `xyz`, `xyz-d50`, `xyz-d65`)
    or polar one (`hsl`, `hwb`, `lch`, `oklch`) with `shorter` / `longer` / `increasing` /
    `decreasing hue`, Oklab when the method is omitted; percentages in either order, math functions,
    and the §2.2 normalization (a sum below 100% scales the alpha, a zero sum is invalid).
    Interpolation follows Color 4 §12: missing components take the other color's and are carried
    forward to analogous components, an achromatic color's hue is powerless, alpha is premultiplied.
    Mixed at parse time; a mix holding `currentcolor` computes at computed-value time.
    `color::palette::xterm_rgb(n)` gives a palette index's color, which mixes and transitions now
    use. (C3-MIX)
  - **Relative color syntax** (CSS Color 5 §4): `from <color>` in `rgb()` / `rgba()`, `hsl()` /
    `hsla()`, `hwb()`, `lab()`, `lch()`, `oklab()`, `oklch()` and `color()` — the origin converted
    to the function's space and its channels bound to `r g b` / `h s l` / `h w b` / `l a b` / `l c
    h` / `x y z` and `alpha`, as numbers, alone or in math functions (`hsl(from var(--brand) calc(h
    + 180) s l)`); an origin of `currentcolor` computes at computed-value time. Conversions among
    sRGB, HSL and HWB no longer pass through XYZ, so exact values stay exact. (C3-RELATIVE)
  - **System colors** (CSS Color 4 §6.2): `AccentColor`, `AccentColorText`, `ActiveText`,
    `ButtonBorder`, `ButtonFace`, `ButtonText`, `Canvas`, `CanvasText`, `Field`, `FieldText`,
    `GrayText`, `Highlight`, `HighlightText`, `LinkText`, `Mark`, `MarkText`, `SelectedItem`,
    `SelectedItemText`, `VisitedText`, case-insensitive, and the deprecated ones mapped to them
    (§6.2.1). `Canvas` / `ButtonFace` / `CanvasText` / `FieldText` are the terminal's default colors
    (`reset`); the rest are the UA sheet's own palette — the accent, the selection, the muted text —
    which now lives in `color::system` and is what the UA rules paint with (`color::SystemColor`;
    the mapping is in DIVERGENCES). (C3-SYSTEM)
  - **`color-scheme` and `light-dark()`** (CSS Color Adjust 1 §2, CSS Color 5 §5): `color-scheme:
    normal | [light | dark | <custom-ident>]+ && only?`, inherited, and
    `ColorSchemeList::used(preferred)` — the element's used scheme given the document's preferred
    one (§2.1); `light-dark(<light>, <dark>)` picks by it at computed-value time, alone or inside
    other color functions. `ColorScheme` (light / dark, dark by default) with
    `ColorScheme::for_background(color)` — light when black text on that background has more
    contrast than white text; `ColorContext::scheme` / `with_scheme`, under which the terminal's
    default colors inside a color function take the canvas model's values (white / black when
    light). (C3-SCHEME)
  - **`transparent` is transparent black** (CSS Color 4 §6.3): `parse_color("transparent")` is
    `Color::TRANSPARENT` (`Rgba(0, 0, 0, 0)`), where it was `Color::Reset`, and serializes as
    `transparent`. As a background it paints as before (what is beneath shows); as `color` the text
    is now invisible and as `border-color` the border draws nothing, where both used to take the
    terminal's default foreground. Migration: write `reset` for the terminal's default color.
    (C3-TRANSPARENT)
  - **Property names, keywords and units are ASCII case-insensitive** (CSS Values 4 §2.1; CSSOM
    `setProperty` folds the name). New `property_dispatch::canonical_property_name`, which `set` /
    `set_from_tokens` / `serialize` / `property_mask` / `remove` / `inherits` all fold through, so
    `COLOR: RED` is `color: red` in a sheet, an inline style and CSSOM alike; custom property names
    (`--Foo` vs `--foo`) stay case-sensitive (CSS Variables 1 §2). `text-decoration: UNDERLINE` and
    `transition-duration: 2S` parse — they were the last keyword and unit matched exactly. (C1-CASE)
  - **`@layer`** (CSS Cascade 5 §6.4.1): the statement form `@layer a, b.c;` declares layers, the
    block form `@layer a { … }` / `@layer { … }` parses its rules into a named or anonymous layer,
    and an `@layer` inside a block nests under it; at-rule names match ASCII case-insensitively. An
    invalid prelude (two names on a block, whitespace around a `.`, a reserved CSS-wide keyword)
    drops the rule with the new `WarningKind::InvalidAtRulePrelude { name, prelude }`. (C1-LAYER)
  - **CSS Nesting** (CSS Nesting 1 §2–§3): style rules nest in style rules. A block item that starts
    `<ident>:` with no `{}` block before its `;` is a declaration, anything else a nested rule (CSS
    Syntax 3 "consume a block's contents"), so `.a { p:hover { … } }` nests; the declarations before
    the first nested rule are the rule's own, each later run is a nested declarations rule with the
    parent's selector, in order of appearance (`.a { color: red; & { color: blue } color: green }`
    is green); a nested `@layer` holds declarations and rules in the layer. An invalid nested
    selector drops that rule alone; an item ending at `;` before a block is a malformed declaration;
    other nested at-rules are reported and skipped (the conditional rules join with C14). A nested
    rule used to be swallowed as a malformed declaration. (C1-NESTING)
  - **`@scope`** (CSS Cascade 6 §2.5): `@scope [(<scope-start>)] [to (<scope-end>)] { … }` at the
    top level, in a style rule (start relative to the rule) and in another `@scope` (start relative
    to its root); the body holds scoped style rules (relative to the root), nested rules, `@layer`,
    and declarations that apply to the root as `:where(:scope)`. An invalid prelude drops the rule
    with `InvalidAtRulePrelude`. (C1-SCOPE)
  - **`@import`** (CSS Cascade 5 §3) through a host-provided loader: `ImportLoader` (implemented by
    any `Fn(&str) -> Result<String, String>`) and `parse_with_loader(source, &loader)`. The imported
    sheet's rules are parsed in at the import's position; `layer` / `layer(name)` put them in an
    anonymous / named layer (their own layers nest inside); `supports()` and media conditions are
    recorded on `Stylesheet::imports` and count as true until conditional rules land (C14). New
    warnings: `ImportIgnored(url)` for an `@import` after other rules (`@charset` and `@layer`
    statements excepted) or in a block, `ImportCycle(url)`, and `ImportFailed { url, reason }` for a
    missing loader (`parse`) or a loader error; an import without a URL is an
    `InvalidAtRulePrelude`. (C1-IMPORT)
  - **`@property`** (CSS Properties and Values API 1 §3): `syntax`, `inherits` and `initial-value`
    register the custom property in the sheet; a missing or invalid descriptor, an invalid name, an
    unsupported syntax or a non-matching initial value registers nothing and reports the new
    `WarningKind::InvalidPropertyRule { name, reason }`. (C1-PROPERTY)
  - `from_css` / `from_css_strict` merge the parsed sheet with `Stylesheet::append` instead of
    re-adding each rule by its selector text, so rules keep their cascade layer. (C1-LAYER)
  - **Escapes in a selector prelude are copied through intact**, so an escaped `{`, `}`, quote or
    `,` (`.x\{\,y`) neither ends the prelude nor splits the selector list. (C1-ESCAPES)
  - `:ROOT { --x: … }` publishes its custom properties to `Stylesheet::vars()` like `:root`.
    (C1-CASE)
  - `CascadeExt` gains the required methods `set_color_scheme` / `color_scheme` (implemented for
    `Dom<TuiExt>`): the document's preferred color scheme, stored as document data like the
    viewport. An out-of-tree implementor must add them. (C3-SCHEME)
  - `Buffer` gains a private field — the color scheme its canvas model takes (`color_scheme()` /
    `set_color_scheme`, dark by default; the paint pass sets the document's) — so a struct literal
    no longer builds one. Migration: build a `Buffer` with `Buffer::empty` / `filled` /
    `with_cells`. (C3-ALPHA)
  - **The cascade resolves `revert`** (CSS Cascade 4 §7.3): in an author rule or inline style it
    rolls the property back to the value the user-agent origin gives (`button { color: revert }` is
    the UA button color), and to the `unset` value where the UA declares nothing; in a UA rule it
    acts as `unset`. Custom properties and `content` revert too. The ladder is now one plan
    (`cascade/ladder.rs`) that properties, custom properties and `content` all walk — `content` had
    its own copy — and the rollback states are replayed on demand and memoized per step, so a
    cascade without `revert` does no extra work. (C1-REVERT)
  - **Cascade layers** (CSS Cascade 5 §6.4): author rules cascade layer by layer — later layers beat
    earlier ones whatever the specificity, unlayered rules beat every layer, and `!important`
    reverses the order; a layer's own rules beat its sublayers. `revert-layer` rolls a property back
    to the cascade without its layer (and the ones above it), from the first layer to the UA origin.
    All the sheets of one cascade run share one layer order, merged by name in run order — for an
    `App`, its `<style>` sheets in tree order, then its own sheets in push order (DIVERGENCES).
    `extend_from_style_tags` keeps layers (`Stylesheet::append`). (C1-LAYER)
  - **`@scope` in the cascade** (CSS Cascade 6 §2.5, §6.1): a scoped rule matches an element in
    scope of one of its roots — an inclusive descendant of the root not inside a scoping limit —
    with that root as `:scope`; a nested `@scope`'s roots must be in the enclosing scope. Scope
    proximity (generations from the nearest matching root; unscoped rules infinitely far) sorts
    after specificity and before order of appearance. A `<style>` element's sheet records the
    element as its owner node, so a prelude-less `@scope` in it scopes to the element's parent.
    (C1-SCOPE)
  - **`App::set_import_loader(loader)`**: the document's `<style>` sheets resolve `@import` through
    it (CSS Cascade 5 §3) and are re-parsed; without a loader an `@import` warns
    (`style_element_warnings`) and imports nothing. (C1-IMPORT)
  - **The cascade substitutes `var()`** (CSS Variables 1 §3) in every property, `content` and
    shorthands included: after an element's custom properties are folded (and their own `var()`s
    substituted), each matched block holding `var()` is substituted and parsed once for the element,
    and every ladder step, rollback, `content` and the `border-color` fallback read the substituted
    blocks. An element whose blocks hold no `var()` pays one scan. The color-only `var()` path is
    gone — colors go through the same substitution. (C1-VAR-ANY)
  - **Registered custom properties in the cascade** (CSS Properties and Values API 1 §2): a
    registered property starts at its initial value, does not inherit when `inherits: false`, takes
    its initial value for `initial` (and for `unset` when it does not inherit), and a value that
    does not match its syntax after `var()` substitution is invalid at computed-value time
    (`unset`). `App::register_property(registration)` is `CSS.registerProperty` — it wins over
    `@property`, and a second registration of a name is an `Err`. A registered `<color>` /
    `<number>` / `<integer>` / `<length>` / `<percentage>` transitions (`transition: --c 1s`): the
    engine writes its animated value into the element's `PresentationStyle::custom_properties`, the
    cascade applies it (`ComputedStyle::animated_vars`, for the element and its descendants),
    re-cascades the subtree each frame so `var()` consumers follow, and dispatches `transitionstart`
    / `transitionend` / `transitioncancel` naming `--c` (`AnimationRegistry::take_restyle`,
    `take_pending_custom_events`, `PendingCustomEvent`). `runtime/animation.rs` is split into
    `animation/{mod,diff,interpolate,custom,tests,custom_tests}.rs`. (C1-PROPERTY)
  - **`currentcolor` resolves against the element's final `color`** (CSS Color 4 §6.4):
    `background-color` / `border-color` (including `border-color: initial`) take the `color` the
    whole cascade settles on, not the one cascaded so far; in `color` it is the inherited color;
    `caret-color` / `caret-text-color` inherit it as specified and resolve it at paint against each
    element, as they now do `var()` (which they used to replace with the text color).
    (C3-CURRENTCOLOR)
  - **The terminal's color scheme.** At startup `App::run` asks the terminal for its background with
    OSC 11 (Unix, when stdout is a terminal; DA1 marks the end of the replies — 200 ms for a reply
    to begin, up to 800 ms more once one has, C3G-OSC-ROBUST; keys typed meanwhile are kept and a
    later reply is still taken, C3G-INPUT-READER) and prefers the scheme it calls for, which
    `light-dark()` and `color-scheme: normal` follow; dark when the terminal does not say.
    `App::with_color_scheme(scheme)` sets it instead (no query), `App::set_color_scheme` changes it
    and restyles the tree, `App::color_scheme()` reads it, `App::detected_background()` is the
    background the terminal reported (`None` without an answer, C3G-OSC-ROBUST);
    `rdom_tui::ColorScheme`. The scheme follows the terminal's theme changes (DEC mode 2031,
    C3G-INPUT-READER); a scheme the app set is not overridden. (C3-SCHEME)
  - **Color alpha composites over the backdrop** (CSS Color 4 §4.2), with the group-opacity per-cell
    rules: a translucent background blends with the background beneath and tints the glyphs and
    borders it leaves; translucent text contests the glyph beneath (it takes the cell at alpha ≥ 0.5
    or over an empty one) and blends with the background; a translucent border blends like a glyph;
    a translucent `::backdrop`, row highlight, `::selection` or caret style likewise. `Color::Reset`
    blends as the canvas of the document's color scheme (white on black when dark, black on white
    when light). Each translucent paint is made in a layer over just its cells
    (`Buffer::write_styled`, `paint_translucent`), so `Buffer::set_symbol` / `set_style` /
    `set_string*` composite a translucent style too; a `Cell` is opaque storage — `Cell::set_fg` /
    `set_bg` store the color (a transparent one paints nothing) and debug-assert that it is not
    translucent, since only the buffer knows the color scheme to blend with
    (C3G-SCHEME-CONSISTENCY). (C3-ALPHA)
  - **A fully transparent color paints nothing.** `Cell::set_fg` / `set_bg` / `apply_style` leave
    the cell's color when given one with alpha 0, and a `Buffer` glyph write in a transparent
    foreground (`set_symbol`, `set_string*`) keeps the glyph the cell shows and paints only the
    style's background; the paint pass fills no background and draws no border in a transparent
    color (the border keeps its space). An `<input>` with a transparent background inverts its caret
    against the cell beneath. (C3-TRANSPARENT)
  - **Color transitions interpolate in Oklab** (CSS Color 4 §12.1) with premultiplied alpha, for
    `color`, `background-color`, `border-color` and registered `<color>` properties: red → blue
    passes through `rgb(140, 83, 162)` where it used to pass through sRGB's `rgb(128, 0, 128)`. A
    palette index interpolates as its xterm color; an endpoint that is `reset` as the canvas model's
    color for the property's role (background for `background-color`, text for `color` and
    `border-color`) in the element's color scheme, and a registered property's `reset` endpoint (no
    role) changes discretely (C3G-SCHEME-CONSISTENCY). (C3-LAB)
  - **An inline `!important` beats an author `!important`** (CSS Cascade 4 §6.1 "element-attached
    styles"): the `style` attribute wins over the author's rules at both importances, so `<p
    style="color: blue !important">` is blue against `p { color: red !important }` — the ladder
    applied author important after inline important. The style attribute also sorts above every
    cascade layer (Cascade 5 §6.1). (C1-INLINE-IMPORTANT)
- 2026-10-08 — Phase 6 closed: 14 items + 28 gate fixes (batch A 11, batch B 9, batch C 8).
  Gate-fix re-review folded into the Phase 7 gate. Carried, recorded: inline content is not aligned
  upward by `align-content` and an absolutely positioned box with both insets `auto` is not
  self-aligned (DIVERGENCES §4); a generated flex item's `aspect-ratio` (§9.2 step 3.B reads an
  element's); logical-side builders (a kept declaration, not a field); `SIZE-1` (TECH_DEBT) files,
  `walk.rs` and `layout_pass/mod.rs` added.
- 2026-10-08 — C7-GRID-CORE, prep (no behaviour change, no test changed): the item model leaves
  `flex/`, so grid shares it without reaching into flex — `flex/item.rs` and `flex/anonymous.rs`
  are `layout_pass/items/{mod,anonymous}.rs`, `FlexItem` is `items::Item` and `flex_items`
  `items::items_of` (CSS Grid 2 §6.1 builds grid items exactly as CSS Flexbox §4 builds flex
  items). Moves and renames only.
- 2026-10-08 — C7-GRID-CORE, found on the way (its first commit's two `ComputedStyle` fields tipped
  `scope_matching_is_bounded_per_pass`, a 41-deep chain, into a stack overflow): the cascade walk
  recursed once per tree level with every style an element computes in its frame — the element's,
  its six pseudo-elements' before the children and `::after`'s after them, moved through temporaries
  — about 50 KB a level in a debug build. Fix at the root: `walk::cascade_subtree` keeps only `Rc`s
  across the recursion; `style_element` (the element and the pseudo-elements before its children)
  and `finish_element` (`::after` and the aggregates) compute in frames of their own
  (`#[inline(never)]`) that are gone before the children start, and the element's style is
  `Rc`-wrapped once instead of cloned into one. Measured with a temporary stack-address probe: 5 808
  bytes a level with the `::before` style still inline in the carried struct, 1 200 with it behind an
  `Rc`. Red: `cost_tests::a_deep_tree_cascades_on_a_small_stack` (a 400-deep chain, every level a
  `::before` and a scroll container, on a 1 MiB thread) overflowed against the old walk; green after.
  Split with it (walk.rs would have reached 685 lines): `compute_element_style` and
  `settle_direction` move to `cascade/element.rs` (155; `walk.rs` 548), the cut TECH_DEBT `SIZE-1`
  named, so `walk.rs` leaves that list. No other test expectation changed.
- 2026-10-08 — C7-GRID-CORE, part 1 of 2 (the track lists; the grid container and its layout are
  part 2): `grid-template-columns` / `grid-template-rows` (CSS Grid 2 §7.2) parse, cascade, compute
  and serialize. Model — the specified grammar, kept as written so CSSOM serializes what the author
  wrote (`repeat(2, 1fr)` stays a repetition; layout expands it): `layout/grid.rs` —
  `GridTemplate { None, Tracks(TrackList) }` (`#[non_exhaustive]`: `subgrid` is C7-SUBGRID's),
  `TrackList { line_names, items }` (one name list per line, so one more than the components),
  `TrackListItem { Size, Repeat }`, `TrackRepeat { count: RepeatCount { Count, AutoFill, AutoFit },
  line_names, sizes }`, `TrackSize { Breadth, MinMax, FitContent }` with `min_sizing` /
  `max_sizing` / `fit_content_limit` (§11.1's min and max track sizing functions: a lone `fr` and
  `fit-content()` have the minimum `auto`, `fit-content()` the maximum `max-content`), and
  `TrackBreadth { Cells, Percent, Calc, Fr, MinContent, MaxContent, Auto }` (`cells(basis)`; a
  percentage against an indefinite basis is `None`). Parser (`parse/values/grid.rs`): `none |
  <track-list> | <auto-track-list>` — `minmax(<inflexible-breadth>, <track-breadth>)` (an `fr`
  minimum invalid), `fit-content(<length-percentage>)`, `repeat(<integer [1,∞]> | auto-fill |
  auto-fit, …)` (no nesting), `[<custom-ident>*]` line names (not `span` / `auto` / the CSS-wide
  keywords / `default`; two groups in a row invalid), and §7.2.3.1's rule that a list with an
  automatic repetition has exactly one and only fixed sizes (`TrackList::is_valid`, which the
  builders check as C6G-ALIGN-API's do — debug panic, release refusal). Dispatch in
  `property_dispatch/grid.rs` (set / serialize, the family the later grid properties join); viewport
  units absolutized per breadth (`absolute.rs`); `layout_differs` compares both. Red:
  `parse::values::grid::tests` (6 tests) against a stub returning `None` — all failed; the dispatch
  tests failed to compile (no fields, bits or builders); green after, with
  `viewport_units_in_a_track_list_compute_to_cells` and the debug-panic test added with the model.
  Changed expectations: the canonical-values table, the important-setter coverage and the C1
  `initial` perturbation gain the two properties. No layout yet: a track list is inert until part 2.
- 2026-10-08 — C7-GRID-CORE, part 2 of 2: the grid container (CSS Grid 2; sections are Grid 1's
  numbers, which Level 2 keeps through §8 and shifts by one after its §9 "Subgrids"). Style:
  `Flow::Grid` (Breaking — rdom-style) with `Flow::is_flex_or_grid`; `display: grid | inline-grid |
  block grid | inline grid` (`grid list-item` invalid), serialized `grid` / `inline-grid`;
  `TuiStyle::grid()` / `inline_grid()`. Box tree — decided, one model for "a container whose children
  are items": the flex predicates become flex-or-grid where the concept is the same —
  `box_tree::item_sequence` (was `flex_sequence`) and `is_flex_or_grid_container` (paint order,
  `box_index`), `blockify::children_are_items` (CSS Display 3 §2.7 blockifies grid items:
  `inline-grid` → `grid`), `is_ifc_block`, `paints_atomically` (§6.5), the margin-collapse
  independent-formatting-context walk (§6.1), the text-leaf exclusion and `establishes_new_bfc`; a
  grid item's percentage heights resolve against its area (§6.2, `height_is_definite_below`). Which
  formatting context lays out an element's children moved out of `flex/mod.rs` into
  `layout_pass/dispatch.rs` (`flex/mod.rs` 520 → 359, out of TECH_DEBT `SIZE-1`); the flex arm is
  `flex::layout_flex_container`. Layout — `layout_pass/grid/`: `template.rs` (the explicit grid:
  `repeat()` written out, `auto-fill` / `auto-fit` counted per §7.2.3.2 — a track's max sizing
  function when definite else its min, gaps included, the container's definite size or max, else its
  min, else once — line names per line for C7-GRID-PLACE / -AREAS, the grid clamped at 10 000 tracks
  an axis per §8's note), `placement.rs` (until C7-GRID-PLACE every item auto-placed, span 1, row by
  row, in order-modified document order — §8.5's sparse `row` flow; at least one column; implicit
  rows `auto` until C7-GRID-AUTO), `track.rs` (§11.1 min / max sizing functions resolved — a
  percentage against an indefinite basis `auto`, `fit-content()` the max-content maximum capped —
  §11.4 initialization, gutters as fixed tracks, an `auto-fit` empty repetition collapsed with its
  gutters), `sizing/` (§11.3: §11.5 single-span tracks, spanning items by increasing span, items
  crossing flexible tracks together by flex factor, §11.5.1's distribution with limits, beyond-limit
  rules and infinitely growable limits; §11.6 maximize, redone against a definite max; §11.7 the `fr`
  size, definite and indefinite, against the container's min / max; §11.8 stretching `auto` tracks
  under `normal` / `stretch` content distribution), `contribution.rs` (each item's min-content,
  max-content and §6.6 minimum contribution — the content-based minimum shared with flex,
  `items::content_based_minimum`, moved out of `flex/distribute.rs`; capped by the area's fixed
  maxima — margins included, measured once per run), `arrange.rs` (the grid area from the track
  extents, `rtl` mirroring the columns, scroll offsets; until C7-GRID-ALIGN an `auto`-sized item
  stretches where its self-alignment is `normal` / `stretch`, else takes its fit-content width or
  content height at the area's start), `intrinsic.rs` (§5.2: the container's min- / max-content
  width its columns under that constraint, its content height its rows at the columns a width
  gives — `intrinsic::measure_content` asks it). Decided: an indefinite block size sizes the rows as
  under a max-content constraint (§5.2: an `auto` height is the max-content height), so `minmax(1,
  5)` rows of an `auto`-height grid are 5, as in browsers; the rows of a container whose height is a
  length, a percentage of a definite height or the size its flex / grid container gave it are
  definite; §11.1 steps 3–4 (re-sizing the columns after the rows) are not run (DIVERGENCES §2).
  `margin-trim` on a grid container trims the items in its first / last row and column (CSS Box 4
  §3), sizing included. Shared with flex: the item model (`layout_pass::items`), `Item::contribution`
  (the intrinsic children sum now calls it), and whole-cell shares — `layout_pass/shares.rs`
  (`FACTOR_TOLERANCE`, `floor_cells`, `Rolling`, moved out of `flex/distribute.rs`, which uses it).
  Rounding — decided, consistent with DIVERGENCES §1's alignment entry: equal shares give the
  remainder cell to each of the first tracks that can take one, `fr` and by-factor shares roll
  (`2 1fr 2fr` in 13 cells: 2 + 3 + 8). Red: `css_phase7/{container,tracks}.rs` — all 23 failed
  against `Flow::Grid` laid out as block flow (`["ABC       ", …]` for three items in two columns,
  widths 0 for every track test, `inline-grid` 2 wide for 3); green after, with the track sizing unit
  tests (`grid/sizing/tests.rs`) added with the algorithm. Added green, then mutation-checked: the
  fit-content container, the grid flex item, the caret into anonymous items, percentages and margins
  inside an item, `margin-trim`, paint and hit order, atomic paint, and the sizing tests for step 3.3,
  limited contributions and infinitely growable limits. Mutation checks (each alone, restored and
  touched; `target/claude-logs/mut/`): maximize off → minmax, fit-content and the measurement test;
  stretch off → the auto-track test; §11.7 off → ten track tests; the remainder to the last track →
  the fit-content container; no `auto-fit` collapse → its test; one repetition → the three repeat
  tests; no `rtl` mirror → its test; the intrinsic hook off → seven tests; blockification flex-only →
  its test; step 3.3 off → its unit test (the integration measure test survives: the indefinite flex
  fraction compensates); growable marking off, step 4 shared equally, the limited cap off → their unit
  tests; the run cache off → the cost test; `box_index`, the definiteness, margin-collapse, text-leaf,
  definite-rows, paint-order, atomic-paint and `margin-trim` sites → their tests. One equivalent
  survivor, kept for consistency with flex: `is_ifc_block`'s grid arm (blockification already makes
  every grid item block-level). Cost (`grid/cost_tests.rs`): a pass sizes a grid twice (its parent's
  measurement, its layout), each run measuring an item's contributions once each — 10 an item a pass
  for `auto` columns and rows, the same with `1fr` rows read twice a run — and the content walks
  behind them are the pass memo's (at most 2 Row and 2 Column an item). Changed expectations:
  rdom-style's invalid-`display` list loses `grid` / `inline grid` (gaining `grid list-item`, `grid
  flex`, `inline-grid flow`, `grid grid`) and its serialization table gains the two grid forms. No
  snapshot changed. DIVERGENCES: §1 "Grid tracks are whole cells"; §2 the columns not re-sized after
  the rows, and `border-collapse` scoped to flex and block containers; §3's grid list loses the core
  and says what holds until each remaining item.
- 2026-10-08 — C7-GRID-AUTO: `grid-auto-columns` / `grid-auto-rows` (CSS Grid 2 §7.6:
  `<track-size>+`, initial `auto`, not inherited) parse, cascade (viewport units absolutized per
  breadth), compute and serialize as written (`parse_track_sizes` / `serialize_track_sizes`; `TuiStyle`
  / `ComputedStyle::grid_auto_columns` / `grid_auto_rows: Vec<TrackSize>`, Breaking — rdom-style;
  builders that refuse an empty list or an `fr` minimum as the template ones do; node setters). Layout:
  `grid::tracks_of` sizes every implicit track from the pattern — the first after the explicit grid
  takes its first size and so on forwards, the last before it its last size and so on backwards — over
  a `Placement` that now says how many implicit tracks precede the explicit grid (`columns_before` /
  `rows_before`, 0 until C7-GRID-PLACE places an item before it; the backwards half is pinned by its
  tests there). Red: `css_phase7/auto.rs` (4 tests) failed the strict parse (`grid-auto-rows`
  unknown) and the dispatch test failed to compile (no fields, bits, builders); green after. Mutation
  checks (restored and touched): the pattern always its first size → the repeating test; every implicit
  track `auto` → three of the four (the flexible-rows test survives: three stretched `auto` rows share
  the height alike). Changed expectations: the canonical-values table, the important-setter coverage and
  the C1 `initial` perturbation gain the two properties. No snapshot changed.
- 2026-10-08 — C7-GRID-PLACE, part 1 of 3 (the properties; the placement algorithm is part 2,
  absolutely positioned grid children and item paint order part 3): `grid-row-start` / `-end`,
  `grid-column-start` / `-end` (CSS Grid 2 §8.3: `<grid-line> = auto | <custom-ident> | [ <integer>
  && <custom-ident>? ] | [ span && [ <integer [1,∞]> || <custom-ident> ] ]` — line 0, a span below one
  and the names `span` / `auto` invalid, the integer a math function too and clamped to `i32`), the
  shorthands `grid-row` / `grid-column` / `grid-area` (§8.4: an omitted end copies a lone
  `<custom-ident>`, else `auto`; `grid-area` row-start / column-start / row-end / column-end) and
  `grid-auto-flow` (§7.7: `[ row | column ] || dense`, initial `row`) parse, cascade, compute and
  serialize in the shortest form (`span 2 a`, `-2 a`, `span a` for a span of one, a shorthand's end
  dropped when the start gives it back, `dense` for `row dense`). Model: `layout/grid_placement.rs` —
  `GridLine { Auto, Name, Line { index, name }, Span { count, name } }` (closed data, `is_valid`) and
  `GridAutoFlow { direction: Direction, dense }` (`ROW`, `COLUMN`, `.dense()`); parsers in
  `parse/values/grid_placement.rs`, dispatch in `property_dispatch/grid.rs`; builders that refuse an
  invalid line as the other grid builders do (`grid_row`, `grid_column`, `grid_area` over the four
  checked longhands); node setters `set_grid_row` / `set_grid_column` / `set_grid_auto_flow`; root and
  prelude re-exports. Red: the dispatch tests (5) failed to compile before the model; with the model,
  stubbed `parse_grid_line` / `parse_grid_auto_flow` failed the three parse tests; green after.
  Changed expectations: the canonical-values table, the important-setter coverage and the C1
  `initial` perturbation gain the eight names. Inert until part 2.
- 2026-10-08 — C7-GRID-PLACE, part 2 of 3: the placement algorithm (CSS Grid 2 §8). Lines
  (`grid/placement.rs::resolve`, numbered from 0 at the explicit grid's first line): a positive
  integer counts from the explicit grid's start, a negative one from its end (§8.3); `<n> <ident>`
  the nth line of that name from either end, every implicit line on that side counting when the
  explicit grid has too few; a lone `<custom-ident>` first the line named `<ident>-start` (`-end` for
  an end edge) — the hook named areas fill (C7-GRID-AREAS: `grid-template-areas` adds those names to
  `template::Explicit::names`, which `Lines` reads) — else `1 <ident>`; `span <n>` / `span <ident>`
  from the other edge's line, searching outward. §8.3.1: lines in the wrong order swap, an end equal to
  the start drops, the end's span drops beside the start's, a lone span to a name is a span of one;
  lines clamped to ±10 000 (§8's note). Auto-placement (`place`, generic over the flow's major and
  minor axes): step 1 the items definite on both axes; step 2 the items locked to a major track (sparse:
  past the step's earlier items in that track; dense: from the first line); step 3 the minor extent
  (the explicit grid, every definite minor line, the widest auto span); step 4 the cursor — a definite
  minor position moves it (a new major track when behind it), an automatic one takes the next fitting
  position from it, `dense` restarting from the grid's start each time. The implicit grid spans every
  line used, before the explicit grid too (`Placement::{columns,rows}_before`, which C7-GRID-AUTO's
  backwards pattern sizes). Occupancy is a per-major-line list of minor ranges. Red:
  `css_phase7/place.rs` — all 13 failed against the auto-placing placement of C7-GRID-CORE (after
  two coincidental passes were made discriminating: the row-locked item first in document order, and
  the area-edge test's `main-start` moved off line 1); green after, with two more added for step 2's
  ordering and the cursor's new row (each red under its mutation below). Mutation checks (each alone,
  restored and touched): `dense` ignored → the flow test; step 2 off → the row-locked test; the
  `-start` / `-end` lookup off → the area-edge test; no swap → the conflicts test; no implicit named
  lines → its test; column flow as row → the flow test; negative lines off → two tests; the cursor's
  new-row bump off → its test; named spans as plain spans → the named-lines test. No other test
  expectation and no snapshot changed. DIVERGENCES §3: the placement line goes; absolutely
  positioned grid children (part 3) are listed until then.
- 2026-10-08 — C7-GRID-PLACE, part 3 of 4: absolutely positioned boxes in a grid (CSS Grid 2 §9.1).
  A grid container keeps its lines after layout — `TuiExt::grid_lines` (crate-private; cleared by
  every other formatting context in `dispatch::layout_children`): per axis the explicit grid's size
  and line names, the implicit tracks before it, and each track's start- and end-side edge, absolute
  and unscrolled, an `rtl` grid's columns starting at their right edge (`grid/lines.rs`, filled by
  `arrange`). `positioning::containing_block` asks `grid::abspos_area` when the positioned ancestor
  is a grid container: each axis resolves its two lines as in-flow placement does (`Lines::definite`,
  named and numbered lines, the `-start` / `-end` hook), but an `auto` line — and a span beside one,
  and a line the implicit grid does not have ("a non-existent line … is instead treated as `auto`") —
  is the containing block's own edge, with no span-of-one default (abspos boxes are not
  auto-placed); lines in the wrong order swap and an end equal to the start is `auto`. The
  containing block an `auto` edge falls back to is the one rdom gives any absolutely positioned box
  (its positioned ancestor's layout rect). The static position is unchanged: the content box's
  start, as for the sole item of a grid area the content edges bound (§9.2). Red:
  `css_phase7/abspos.rs` (3 tests) — every box at the container's rect `(0, 0, 20, 3)`; green
  after, two expectations corrected while green (`grid-column: 2` ends at the container's edge, its end
  `auto`, as the test's own doc now says), and the `rtl` test added. Mutation checks (restored and
  touched): the hook off → all four; the `rtl` edges unmirrored → the `rtl` test; a missing line
  clamped instead of `auto` → its test. DIVERGENCES §3's abspos line goes.
- 2026-10-08 — C7-GRID-PLACE, part 4 of 4: grid items in the stacking order (CSS Grid 2 §6.5: grid
  items "paint exactly the same as inline blocks, except that order-modified document order is used
  in place of raw document order, and `z-index` values other than `auto` create a stacking context even
  if `position` is `static`"). Order and atomic painting landed with C7-GRID-CORE; `z-index` was
  missing for flex items too, whose §5.4 says the same — decided, one predicate for both:
  `stacking::is_layered(dom, parent, c)` — positioned, or a flex / grid item (`is_item_of`, the
  parent walk `paints_atomically` already did, now shared) with a numeric `z-index` — is what the
  layer collection (`Walk::children`, a layered non-positioned box being a stacking context), the
  atomic-shadow walk, paint's in-flow walk and hit-testing's in-flow descent skip or layer; such an
  item clips like a relatively positioned one (its parent's content clip). The document root's
  children stay block boxes for paint (no item). Red: `css_phase7/stacking.rs` — the grid and flex
  items with `z-index: 1` painted and hit under the later item (`abb `, not `aab `), the `z-index: -1`
  item's `x` painted over its container's background; green after. Mutation checks (restored and
  touched): items never layered → all three; a layered item not a context → the negative test; paint's
  in-flow walk skipping positioned boxes only → the negative test; hit-testing's likewise → the
  negative test's hit (asserted after it survived the first run). Consumer-visible for flex: listed
  among the CHANGELOG's silent behaviour changes. No other test expectation and no snapshot changed.
  DIVERGENCES §2's stacking-context entry names the items.
- 2026-10-08 — C7-GRID-AREAS, part 1 of 2 (`grid-template-areas`; the `grid-template` / `grid`
  shorthands are part 2): `grid-template-areas` (CSS Grid 2 §7.3: `none | <string>+`, initial `none`,
  not inherited) parses, cascades, computes and serializes (each string's tokens one space apart, a
  null cell token one `.`, as browsers give it). Model — decided, valid by construction:
  `layout/grid_areas.rs` — `GridTemplateAreas` (private rows; `GridTemplateAreas::new(rows)` reads each
  string as §7.3 tokenizes it — name code points, `.` runs, whitespace, anything else a trash token —
  and refuses unequal rows, a trash token or a name whose cells are not one filled rectangle, so the
  builder needs no grammar check), `NamedArea { name, rows, columns }` (`areas()`, first-appearance
  order); parser `parse/values/grid_areas.rs`; dispatch, field, bit, builder, node setter
  `set_grid_template_areas`, root and prelude re-exports. Layout (`grid/template.rs`): the explicit
  grid grows to the areas' rows / columns (§7.1: "the larger of the number of rows/columns defined by
  `grid-template-areas` and the number … sized by `grid-template-rows`/`grid-template-columns`"; the
  unsized ones take `grid-auto-*` — the pattern continuing from the last sized track, as the tracks
  after the explicit grid do) — `Explicit::count` beside `sizes` — and each area names its edges
  `<area>-start` / `<area>-end` on both axes (§7.3.2; `Explicit::with_areas`, the names `Cow`s so the
  template's own stay borrowed), which part 1's seam reads: a lone `<custom-ident>` already tried
  `<ident>-start` / `-end` (§8.3), so `grid-area: head` fills the area and an absolutely positioned box
  finds it through the kept `GridLines` (§9.1). Red: `grid_areas_tests` (3) failed against a parser
  stub returning `None` and an unchecked rectangle (the L-shape accepted); `css_phase7/areas.rs` (4)
  failed with the areas ignored (every item auto-placed in one column — `[(0, 0, 2, 1), (0, 1, 2, 1),
  …]` — the named items at implicit lines `(16, 4, 4, 1)`, the abspos box the whole container); green
  after. Mutation checks (restored and touched): the implicit `-start` / `-end` names off → three of
  the four (the explicit-size test survives); the explicit grid not grown → that test. Changed
  expectations: the canonical-values table, the important-setter coverage and the C1 `initial`
  perturbation gain the property. No snapshot changed. DIVERGENCES §3's grid line keeps the shorthands.
- 2026-10-08 — C7-GRID-AREAS, part 2 of 2: the shorthands. `grid-template` (CSS Grid 2 §7.4: `none |
  [ <'grid-template-rows'> / <'grid-template-columns'> ] | [ <line-names>? <string> <track-size>?
  <line-names>? ]+ [ / <explicit-track-list> ]?`) and `grid` (§7.8: `<'grid-template'> |
  <'grid-template-rows'> / [ auto-flow && dense? ] <'grid-auto-columns'>? | [ auto-flow && dense? ]
  <'grid-auto-rows'>? / <'grid-template-columns'>`) parse and serialize (`parse/values/grid_shorthand.rs`,
  `GridTemplateShorthand` / `GridShorthand`, the track-list helpers of `grid.rs` and `split_slashes`
  shared, not copied). The areas form: a row track per string, `auto` when no size follows it, the
  names after one row and before the next joined on one line (`[mid] [m2]` → `[mid m2]`), columns a
  track list without `repeat()` or `none`; `grid` resets the three implicit properties it does not
  name and leaves the gutters (Grid 2 dropped Grid 1's gap reset). Serialization (CSSOM §6.7.2, the
  forms browsers give): `grid-template` is `none`, `<rows> / <columns>`, or with areas each row's
  names, string and non-`auto` size then `/ <columns>` unless `none` — nothing when the longhands have
  no form of it (rows that repeat, or not one per area row); `grid` is the `grid-template` form when
  the implicit properties are initial (so `auto-flow / 1` reads back `none / 1`, equivalent), else the
  `auto-flow` form that holds them, else nothing. Found on the way (fixed here, the shorthands made it
  worse): `cssText` listed a set `grid-area` beside `grid-row`, `grid-column` and its four longhands —
  `shorthand_family_of` maps a longhand to one parent and the grid shorthands nest. Fix at the root
  of the nesting: `listed_under_grid_shorthand` lists, per grid property, every shorthand that covers
  it, and the property is skipped when one serializes (`grid` over `grid-template`, `grid-area` over
  `grid-row` / `grid-column`). Red: `grid_shorthand_tests` (6) failed — both names unknown; green after.
  Added after the implementation (no red of their own): `css_phase7/areas.rs`'s two shorthand layout
  tests and the `cssText` test — each mutation-checked: `grid-template` not writing the rows / `grid`
  not writing `grid-auto-rows` → both layout tests (`(0, 1, 7, 1)`, rows 1 tall); the grid suppression
  off → the `cssText` test (`grid-row-start: a; …; grid-row: a / 3; grid-column: 2 / b; grid-area: a /
  2 / 3 / b;`). Changed expectations: the canonical-values table gains the two names. No snapshot
  changed. DIVERGENCES §3's grid list loses the line.
- 2026-10-08 — C7-GRID-ALIGN, part 1 of 3 (self-alignment; baseline alignment is part 2, content
  distribution part 3): each grid item aligns in its grid area (CSS Grid 2 §10.2–§10.4, Box Alignment
  3 §6.1 / §6.2). `grid/arrange.rs::fit` now works in physical cells (the area mirrored for `rtl`
  first, no longer the item after) and reuses block layout's `block::justify_offset` — the one
  keyword-to-offset map block-level and absolutely positioned boxes use (`start` / `end` by the
  container's direction, `self-*` by the item's, `left` / `right`, `center` rounded down, the baseline
  values as `safe self-start` / `self-end`, `safe` at the start on overflow) — on both axes, the block
  axis top to bottom. Sizing per §6.2: `normal` / `stretch` fill an `auto` size, any other value takes
  `fit-content` (inline) or the content height (block); `stretch` with a definite size behaves as
  `start` (Box Alignment §6.1); an item with a preferred aspect ratio is, under `normal`, "sized
  consistent with the size calculation rules for block-level elements" — its `auto` width fills the
  area (or follows the ratio from a definite height), its `auto` height follows the ratio, and it
  sits at the start; the ratio transfer is flex's `aspect_cross_from_main`, moved to
  `layout_pass::box_sizing` so both use it (no fork). `auto` margins take positive free space first
  (§10.2: two share it, the leading one rounded down; one takes it), and are 0 when there is none, the
  alignment applying then. `justify-items: legacy <side>` aligns as its side. Red: `css_phase7/align.rs`
  — 8 tests, all failed against part 1's arrange (every non-stretched item at its area's start:
  `justify-self: end` at 0, `align-self: end` at 0, `margin-left: auto` at 0, an `aspect-ratio: 2` item
  10 × 8 instead of 10 × 5, the overflowing `end` item at 0 not -4); green after. Two expectations were
  corrected while green, both the test's own arithmetic: the `rtl` test's column is the container's
  right half (x 10–20), and the abspos test names its area's end lines (`2 / 2 / 3 / 3`: with `2 / 2`
  the end lines are `auto`, the containing block's edges, §9.1). The abspos check (C6G-DOCS's
  self-alignment in the grid-area containing block) passed as it stood — the area is the containing
  block, so the existing alignment applies; it is kept as the pin. Mutation checks (restored and
  touched): `auto` margins never absorbing → the margins test; the aspect ratio ignored → its test;
  `auto` not taking `*-items` → the justify, align and margins tests. No other test expectation and no
  snapshot changed.
- 2026-10-08 — C7-GRID-ALIGN, part 2 of 3: baseline self-alignment in grid rows (CSS Grid 2 §10.4,
  CSS Box Alignment 3 §9.1 / §9.3; Grid §11.5 step 1). `grid/baseline.rs`: the items whose
  `align-self` (or the container's `align-items`) is `baseline` / `last baseline` and whose block
  margins are not `auto` form one group per row and preference — a spanning item in its first row's
  first-baseline group, its last row's last-baseline group (§9.3) — each measured once, after the
  columns, at the width it will have in its area and its unstretched height (`arrange::baseline_size`:
  `arrange::fit` is now an `ItemFit` whose width and height take an indefinite area height, so the
  measurement and the layout size the item one way); each gets the shim that brings its baseline to
  the group's, which `contribution::Measured` adds to its three row contributions (cached without
  it) and `arrange` places it by — the group flush with the row's start, a last-baseline group with
  its end. Shared, not forked: the block-axis baseline geometry (`BaselineBox`: margins, height,
  first / last content rows, the synthesized bottom row, a scroll container's last baseline at its
  margin edge) moves from `flex/cross.rs` to `items/baseline.rs` with `BaselineBox::measure`, which
  flex's `baseline_box` now calls after its cross sizing. The columns have no baseline alignment (a
  horizontal item has no inline-axis baseline: `justify-self: baseline` is `safe self-start` through
  `block::justify_offset`, as part 1 left it). Red: `css_phase7/baseline.rs` (4) — every baseline
  item at its row's top (`(4, 0, 4, 1)` for `(4, 2, 4, 1)`), the shimmed row 4 tall where it must be
  6, the last-baseline items at `safe self-end` (`(0, 2, 4, 3)` for `(0, 1, 4, 3)`); green after.
  Mutation checks (restored and touched): no shims in the row contributions → the shim test; the
  first-baseline group keyed by the item's last row → the spanning test. No other test expectation and
  no snapshot changed (flex's baseline tests pass through the moved measurement).
- 2026-10-08 — C7-GRID-ALIGN, part 3 of 3: aligning the grid (CSS Grid 2 §10.5, Box Alignment 3
  §5). `justify-content` / `align-content` distribute the content box's free space around the tracks
  once they are sized (`grid/content.rs::distribute`, applied in `arrange` before the areas and the
  abspos lines are read off the extents, so both see the aligned tracks): each track not collapsed by
  `auto-fit` is an alignment subject, a collapsed one keeping its predecessor's shift; the space
  before a subject widens the gutter, and the area of every item spanning it. Shared, not forked:
  flex's keyword mapping and whole-cell shares move from `flex/content.rs` to
  `layout_pass/distribution.rs` (`Distribution`, `Ends` — each caller names its axis's `start` /
  `end`, `flex-start` / `flex-end`, `left` / `right` in its frame — `content_distribution`,
  `offsets`); flex's `justify_offsets` / `align_content_offsets` are now ends plus the shared call,
  behaviour unchanged (every flex test passes as it stood). `normal` / `stretch` keep §11.8's
  stretching of `auto` tracks and otherwise pack to the start; `safe` keeps overflowing tracks at the
  start; `left` / `right` are physical under `rtl`. Red: four tests in `css_phase7/align.rs` failed —
  every track at the start (`[0, 2]` for `end`'s `[16, 18]`, the spanning area 5 wide for 11, the
  `rtl` `end` at `[18, 16]`); green after (one expectation of the test's own arithmetic corrected
  while red: `space-around` over 6 rows of free space puts the second row at 7 — shares 2 and 3 —
  not 6). Added green, then mutation-checked: the collapsed-track test (collapsed tracks counted as
  subjects → `[0, 4]`). Mutation checks (restored and touched): the rows not distributed → the
  `align-content` test. No other test expectation and no snapshot changed. DIVERGENCES: §3's interim
  grid alignment line goes; §1's whole-cell alignment entry names grid.
- 2026-10-08 — C7-GRID-RERESOLVE, prep (no behaviour change, no test changed): the sizing run leaves
  `grid/mod.rs` (491 lines with C7-GRID-ALIGN, and §11.1 steps 3–4 grow it) for `grid/size.rs` —
  `size_grid`, `tracks_of`, `run`, `trim` and their records; `mod.rs` keeps the axes, `Grid`, the
  shared item helpers and `layout_grid_children` (225 / 288 lines). Moves only.
- 2026-10-08 — C7-GRID-RERESOLVE (part 1 follow-up; row added to the Phase 7 table with the other two):
  CSS Grid 2 §11.1 steps 3–4. In a terminal an item's min-content width depends on its height only
  through a preferred aspect ratio (no orthogonal flows, no replaced content; text wrapping changes a
  height, never a min-content width), so that is the change step 3 detects. CSS Sizing 4 §5.1 first:
  an `auto`-width item with a ratio and a definite height — declared, a percentage of its grid area
  once the rows are sized, or stretched under `align-self: stretch` (Grid §6.2: a stretched grid item's
  size is definite) — takes its width from the ratio, in layout (`arrange::ItemFit::transferred_width`,
  replacing part 1's `normal`-only transfer from a declared height) and as its min- and max-content
  contribution to its columns (`arrange::ratio_width` → `Budgets::transfers` →
  `contribution::Measured::with_transfers`). `size::size_grid` then runs the steps: (1) the columns
  with the transfers known without rows, (2) the rows, (3) the transfers again against the rows' areas
  and, when any differs, the columns sized again, once — (4) then the rows again, once, when the
  columns' extents moved. Bounded as the spec bounds it: at most two runs an axis per `size_grid` call
  (`cost_tests::re_resolution_is_bounded_to_once_an_axis`, counting the runs per call with a test-only
  `RUNS` log: `[C, R, C, R]` for the re-resolving grid, `[C]` / `[C, R]` for a plain one). Red:
  `css_phase7/reresolve.rs` (3) failed with the transfers ignored (the ratio item's column 0 wide, `t`
  at x 0; `(0, 20, 2)` for the step-4 grid); with step 3 off two failed (and the counting test), with
  step 4 off the step-4 test (`(10, 10, 2)`: the row left at 2); green after. Changed expectation:
  `css_phase7/align.rs`'s aspect-ratio test — `align-self: stretch` now stretches the height to 8 and
  the ratio gives the `auto` width 16 from it (was 10, the area's width): the stretched size is
  definite, so §5.1 transfers it; the item overflows its fixed 10-cell column. No snapshot changed.
  DIVERGENCES: §2's "columns are not sized again after the rows" entry goes; §1's `aspect-ratio`
  entry names grid.
- 2026-10-08 — C7-SUBGRID, part 1 of 2 (the value; the layout is part 2): `grid-template-columns` /
  `-rows: subgrid <line-name-list>?` (CSS Grid 2 §9: `<line-name-list> = [ <line-names> |
  <name-repeat> ]+`, `<name-repeat> = repeat( [ <integer [1,∞]> | auto-fill ], <line-names>+ )`)
  parse and serialize as written. Model: part 1's `#[non_exhaustive]` `GridTemplate` gains
  `Subgrid(LineNameList)` (not breaking: the enum was non-exhaustive for this); `LineNameList { items:
  Vec<LineNameItem> }`, `LineNameItem { Names, Repeat { count, names } }`, `is_valid` (non-empty
  counted repetitions, never `auto-fit`, one `auto-fill` at most), `explicit_lines` (the lines named
  without the `auto-fill` repetition — an auto-placed subgrid's span plus one) and `expand(lines)` (the
  list written out over a subgrid's lines, `auto-fill` repeating whole times into what the rest
  leaves). `GridTemplate::tracks()` is `None` for a subgrid, so every reader keeps treating it as
  `none` until part 2 — which is also its used value where the element has no parent grid. Red:
  `grid_template_takes_subgrid_with_line_names` failed to compile without the model, then against a
  `None` parser stub; green after. Added green, then mutation-checked:
  `a_line_name_list_expands_over_the_subgrids_lines` (`auto-fill` repeated `fill` times rather than
  `fill / names` → `[…, "c", "d", "c"]`). No other test expectation and no snapshot changed.
- 2026-10-08 — C7-SUBGRID, part 2 of 2: the subgrid layout (CSS Grid 2 §9, §9.5). Structure — the
  grid modules grow by concern, not into `size.rs`: `grid/places.rs` (new: a grid's explicit grids and
  its items placed — `place_grid` / `PlacedGrid`, moved out of `size_grid` with `trim`, plus §9's
  clamping), `grid/subgrid.rs` (new: which items subgrid (`axes`), what a subgrid takes from its parent
  (`Inherit` / `Inherited`: tracks, merged names, extents in its content box), from the parent's
  sizing state (`inherited`) or its laid-out lines (`from_parent`), and the parent's view of its items
  (`flatten`)); `track.rs` takes `tracks_of` / `Extent` and gains `TrackGrid::fixed` (an inherited
  axis's tracks). Behaviour: (1) an in-flow grid item that is a grid container with `subgrid` on an
  axis takes that axis from its parent — not when absolutely positioned or not a grid's item (`none`,
  §9; the parent's `GridLines` now lists its subgrid children and their spans, so a grid that is no
  item of a grid finds none). (2) Its explicit grid there is the spanned tracks (`Explicit::subgrid`:
  the parent's names for those lines, reversed with the tracks when the directions differ, and its own
  `<line-name-list>` expanded over them; named areas add names but no tracks); placement clamps every
  area into it before auto-placement ("the same procedure as for clamping placement in an overly-large
  grid") and after, so it has no implicit tracks there; an auto-placed subgrid spans its name list's
  explicit lines less one. (3) Its tracks there are the parent's (`Grid::inherited_*`, placed by
  `arrange` instead of distributed): offsets from its content box — the edge tracks losing its margin,
  border and padding — and with a gap of its own half the difference taken off each side of each
  inner gutter, the leading half rounded down (whole cells); `normal` keeps the parent's. (4) It is
  stretched on a subgridded axis whatever its self-alignment or size. (5) The parent's sizing run on an
  axis a child subgrids (`size::run_items`): the child contributes nothing itself (its row in the run
  list kept, index-aligned, with a zero size and trimmed margins) and its items — recursively through
  nested subgrids on that axis — follow, mapped into the parent's tracks, with the child's edge margin,
  border and padding (and gap difference) as `Placed::extra` margin, an edge no item touches holding a
  zero-sized item of that margin alone (§9.5); on the rows their widths are the child's inherited
  columns or, for a rows-only subgrid, its own columns sized at its content width (`Placed::width`).
  On an axis the child does not subgrid it is measured as a grid with the other axis inherited
  (`measure_subgrid` → `intrinsic::content_size_with`; `Placed::size` carries the min- and
  max-content border boxes into `contribution::Measured`), not through the generic intrinsic path,
  whose parent lines would be the last pass's. Bounded: unmemoized, each subgrid's own layout
  re-measured its nested subgrids, so a chain of depth d made (d + 1)(d + 4)/2 `size_grid` calls a pass;
  the measurement is now memoized per pass (`intrinsic::memo`'s second table, keyed by the element and
  the axis, area and inherited tracks it was measured with), 2 + 2d —
  `cost_tests::nested_subgrids_are_sized_a_bounded_number_of_times`, red without the memo (`5` for `4`
  at depth 1). Decided, recorded in DIVERGENCES §2: a subgrid's items align baselines among
  themselves (§9 shares the groups with the parent's row). Red: `css_phase7/subgrid.rs` — 9 of 10
  failed against part 1 (every subgrid a one-column grid of its own: `[(0, 12), (0, 12), (0, 12)]`);
  the `none` guard passed as it stood. Added after (green, mutation-checked): the direction test.
  Mutation checks (each alone, restored and touched): `flatten` off → the sizing and padding tests;
  the edge margin off → the padding test; the parent's names off → the line-names test (which
  survived until it was made discriminating — a missing name clamps into the last track, which the
  first version's expectation happened to be; it now names the subgrid's first line); the own gap
  ignored → the gap test; the pre-placement clamp off → the clamping test (`(3, 0)`: the auto item
  took the far item's cell); the auto span off → its test; the direction reversal off → the direction
  test. No other test expectation and no snapshot changed.
- 2026-10-08 — C7-ABSPOS-PADDING-EDGE (part 1 follow-up): checked first whether the grid fallback alone
  was wrong — it was not: `positioning::containing_block` gave every absolutely positioned box its
  positioned ancestor's layout rect, the border box, and `positioned_pseudos::resolve_containing_block`
  the same for `::before` / `::after` (the host's or an ancestor's). CSS 2.1 §10.1: "the containing
  block is formed by the padding edge of the ancestor" (CSS Position 3 §2.1 alike). Fixed at the root
  for all of them: `positioning::padding_box` (the layout rect less the used border,
  `geometry::compute_padding_box`, the scrollport helper paint already uses) is what both resolve to;
  the grid area of §9.1 is cut from it, so a grid's `auto` lines are its padding edges with no change
  of their own. Overlap with Phase 8 (positioned layout, §3.10): this is the CSS 2.1 containing block
  those items build on; `CSS-COVERAGE` §3.10's `position` row now says so. Red:
  `css_phase7/abspos.rs` — `top: 0; left: 0` in a bordered, padded `position: relative` box at `(0, 0)`
  where §10.1 puts it at `(1, 1)`, `inset: 0` 14 × 7 for 12 × 5, the grid's `auto` column end at the
  border box (17 wide for 16) — and `positioned_pseudos_tests` (`::after` with `inset: 0` at `(0, 0, 14,
  6)` for `(1, 1, 12, 4)`); green after. Changed (demo, not a test): `rdom-showcase`'s translucency demo
  placed its inner card with offsets tuned against the outer card's border box (`top: 6; left: 6`); its
  design is kept with `top: 5; left: 5`, the snapshot unchanged (with the old offsets the card moved one
  cell down and right, against the outer card's right border — the spec's result for that CSS). No
  test expectation changed. Consumer-visible: listed among the CHANGELOG's silent behaviour changes.
- 2026-10-08 — C7-SPLIT (no behaviour change, no test changed): the file-size pass over
  `layout_pass/grid/*` and every production file Phase 7 touched. The grid directory was kept under
  500 lines a file as it grew (`size.rs` out of `mod.rs` for C7-GRID-RERESOLVE, `places.rs` and
  `subgrid.rs` for C7-SUBGRID, `tracks_of` into `track.rs`); largest now `subgrid.rs` 484,
  `placement.rs` 471. Two touched files were past the bar: `property_dispatch/table.rs` 566 (the grid
  fields and shorthands) → `table.rs` 337 + `fields.rs` 237 (`fields_of` and `all_fields`, the
  property → field map; `table` re-exports `fields_of`, so its callers are unchanged), and
  `layout_pass/mod.rs` 533 → 482 + `flow.rs` 63 (`flow_axis` / `resolve_gap` / `gap_along`, the cut
  `SIZE-1` named; re-exported). Moves only. TECH_DEBT `SIZE-1` drops both and lists what is left (none
  of it touched by Phase 7).
- 2026-10-08 — Phase 7 items complete (C7-GRID-CORE, -AUTO, -PLACE, -AREAS, -ALIGN, C7-SUBGRID and the
  part 1 follow-ups C7-GRID-RERESOLVE, C7-ABSPOS-PADDING-EDGE, C7-SPLIT); the phase's architect and API
  gates are next. CSS-COVERAGE §3.9: 9 of 10 rows *Supported* (`masonry` / `grid-lanes` the decided
  exclusion). ACID.md: grid is now usable for a tile — proposed as a new tile 18 (Grid layout), not an
  extension of tile 7 (the reasons are in its coverage note).
- 2026-10-09 — Phase 7 gates (with the C6G re-review: all 28 at the root). Architect: 1 blocking —
  grid line edges are stored absolute and not moved by `shift_subtree`, so an `align-content` shift
  leaves positioned grid children on the old edges. API: 2 blocking — no grid example in the doctested
  READMEs and two docs still say grid does not exist; DESIGN classifies none of the 17 new public types
  (and `GridTemplate` alone is `#[non_exhaustive]` while read through accessors, so a future value would
  lay out as `none`). Non-blocking: `ComputedStyle::initial()` allocates two `Vec`s for every element;
  public track lists can panic layout (no validity check on read, `TrackList::default()`); grid
  `content_size` reads pass-written line state (memo purity), subgrid shims measured then discarded;
  subgrid memo key a `format!`, `flatten` unmemoized (quadratic), name strings per run; padding-edge
  containing block incomplete (sticky, gutter, scroll offsets, duplicated walks — scheduled as
  C8-CB-COMPLETE); focus fixup only after a cascade (a visibility transition ending hidden keeps
  focus); inline max-content ignores atom boxes; two stacking-context predicates; minor
  (`clear_box_state` misses `grid_lines`, contents-restyle recascades whole subtree, blockify vs
  `is_item_of` on fragments, span loop cost, `minmax(max<min)` in the auto-repeat count); SIZE-1
  drifted; grid builder checks panic where DESIGN's rule says clamp (`span(0)`, `Count(0)`), negative /
  NaN `fr` and `%` reach layout, empty track `Vec` panics; node setters undocumented panics, no
  `set_grid_area` / grid `display` node setter / `place_*` builders, four `GridLine`s for an area;
  upgrade guide misses `focus()` refusal before the next cascade, #16 (padding edge) under-ranked and
  without a `top: -1` hint, ~7 table rows describe never-shipped APIs, `TuiNodeExt::direction()`
  returns `Row` for `row-reverse`, over-length bullets; stale DIVERGENCES / COVERAGE / `Display::Block`
  docs; missing integration tests (`1fr` vs `minmax(0,1fr)`, `place-items: center`, the page
  layout); no public read of used track sizes. Accepted: re-resolve detecting aspect-ratio only
  (record the `column wrap` flex exception), the debug-panic / release-drop builder rule, CSSOM
  restriction model, `is_focusable_in_opened`, subgrid gap halving, z-index on items. Full reports:
  `target/claude-logs/c7_gate_{architect,api}.md`. Fix as `C7G-*`, two batches (A correctness and cost,
  B API and docs).
- 2026-10-09 — C7G-LINES-SHIFT (architect B1): a grid's lines (`TuiExt::grid_lines`, CSS Grid 2
  §9.1) were stored in absolute cells, and `tree::shift_subtree` — C6G-BLOCK-ALIGN's `align-content`
  shift, run before `place_positioned` — moved the grid's rects but not its lines, so a positioned
  child's definite lines named the unshifted area. Decision: one source of truth — the edges are now
  offsets from the container's content-box start (the right edge for `rtl` columns), the tuples
  `content::distribute` already produces, and `abspos_area` resolves them against the grid's current
  `content_layout`, which `shift_subtree` moves; `GridLines::origin` is gone, and
  `subgrid::from_parent`, which converted the absolute edges back to offsets, reads them directly.
  Audit of every other `TuiExt` position cache: `layout`, `content_layout`, `static_position`,
  `before_layout` / `after_layout` and the anonymous boxes' `rect` / `border_box` are absolute and
  shifted; `inline_layout` (line `top`, fragment `x` / `y`) and the anonymous boxes' line boxes are
  relative to their content box; `scroll_state` holds offsets, `scroll_content_*` and
  `table_used_width` sizes, `margin_chain` margins — nothing else absolute. Red:
  `css_phase7/abspos.rs::a_shifted_grids_lines_move_with_it` — a grid at rows 8–9 under `height: 10;
  align-content: end`, `grid-row: 2 / 3` at `(2, 1, 3, 1)` for `(2, 9, 3, 1)` (checked by stashing the
  fix after green, then restoring and touching it). Green after; no other test and no snapshot
  changed.
- 2026-10-09 — C7G-INITIAL-ALLOC (architect N1): `ComputedStyle::initial()` — the start of every
  element's, pseudo-element's and anonymous box's cascade — built `grid_auto_columns` /
  `grid_auto_rows` as `vec![TrackSize::AUTO]`, two allocations per call and per clone. Decision:
  the computed fields are `Cow<'static, [TrackSize]>`, the initial value the borrowed
  `TrackSize::AUTO_LIST` (a `const`, no global state); a declared list is owned (the cascade's new
  `apply_converted`, `apply_value` for a property whose declared form differs, the `TuiStyle` side
  staying `Vec<TrackSize>`), and `resolve_viewport_units` makes a list owned only when it holds a
  math expression. Chosen over "an empty `Vec` means `auto`", which would give a public field a value
  outside the grammar, and over an `Rc` slice, which needs a thread-local to share. Red:
  `cascade::cost_tests::the_initial_style_allocates_nothing_for_grid` — `initial()` 3 allocations for
  1 (the custom-property map's `Rc`, which predates grid and stays); clone pinned at 0 (was 2). Green
  after. `grid_tests::viewport_units_in_an_auto_track_list_compute_to_cells` covers the owned path
  (mutation: never making the list owned → `50vw minmax(10vh, 1fr)` kept; reverted and touched).
  No test expectation or snapshot changed.
- 2026-10-09 — C7G-TRACK-VALIDITY (architect N2, API N1): layout reads only valid grid values. Where
  a value has an in-range neighbour the builder clamps to it (DESIGN's clamp-or-panic rule):
  `TrackSize::fr` / `percent` keep `[0,∞]` through `valid_flex_factor` (CSS Grid 2 §7.2.1 / §7.2.4),
  `GridLine::span` / `span_named` and `TrackRepeat::new` (so `TrackList::repeat`) make a count of 0 one
  (§8.3, §7.2.3), and `From<TrackList>` / `From<Vec<TrackSize>>` give a list of no tracks as
  `GridTemplate::None` (§7.2) — so `grid_template_columns(Vec::new())` and `TrackList::default()` are
  `none`. `TrackBreadth::is_valid` (new) refuses a negative, NaN or infinite `fr` / `%`, and
  `TrackSize::is_valid` asks it of every breadth; `TrackSize::is_valid_list` (new) is `<track-size>+`,
  shared by the `grid-auto-*` builders and the cascade. Making the types' fields private was not
  enough on its own (an empty list is still constructible) and is an API change for batch B, so the
  boundary is the cascade: `apply_grid` ignores a declared grid template, auto-track list or line
  outside its grammar, written straight into a public `TuiStyle` field — as a CSS parser ignores an
  invalid declaration (CSS Syntax 3 §8.1), so a lower declaration applies. Chosen over filtering in
  `GridTemplate::tracks()`, which would have kept the invalid value as computed and laid it out as
  `none` over a valid lower declaration. A `ComputedStyle` written by hand into `TuiExt::computed`
  is outside this (the cascade's output, not an input). Red (`css_phase7/validity.rs`, 5 tests):
  `span 0`, `repeat(0, 3) 4` and the empty list debug-panicked in the builders; a NaN `fr` laid the
  `1fr` beside it out 0 wide (`(0, 0)` for `(0, 10)`); a `TrackList::default()` field value panicked
  layout (`template.rs:93`, index out of bounds). Green after. Mutation: no validity check for the
  template in `apply_grid` → the index panic again (reverted, touched). DESIGN's clamp-or-panic
  paragraph names the grid builders and the cascade check.
- 2026-10-09 — C7G-MEMO-PURITY (architect N3): the per-pass intrinsic memo (`intrinsic::memo`)
  promised that what it holds is pure within a pass, but `grid::content_size` takes a subgrid's
  inherited tracks from its parent's `grid_lines` (`subgrid::from_parent`), which the parent's
  `arrange` writes during the pass; and `size_grid` measured every baseline-aligned item for a shim
  (`baseline::shims` → `arrange::baseline_size` → `intrinsic_size`), subgrids included, before
  dropping theirs. So a subgrid was measured before its parent's lines existed and the memo served
  that size to the parent's arrangement. Two changes, each at its root: (1) `shims` takes an
  `aligns(k)` predicate and does not measure a subgrid at all (CSS Grid 2 §9: it is stretched, in no
  baseline-sharing group — the existing rule, now applied before the measurement); (2) the memo
  never stores a size that reads pass-written state: `grid::reads_parent_lines` (a grid container
  with a `subgrid` template, `subgrid::styled_axes`, now shared with `axes` and `from_parent`) skips
  the table, and `memo.rs` states the exception — the second table, keyed by the inherited tracks,
  stays pure. Red: `css_phase7/subgrid.rs::a_baseline_grids_subgrid_is_right_on_the_first_frame` —
  the scenario of the report, the subgrid `(0, 0, 6, 2)` for `(0, 0, 6, 1)`;
  `memo_tests::a_subgrids_size_is_not_memoized_across_its_parents_lines` — measured with no parent
  lines (2) and again after they are written in the same pass, 2 for 1;
  `grid::cost_tests::a_baseline_grid_measures_no_shim_for_its_subgrid` — 9 `size_grid` calls for
  `align-items: baseline` against 5 for `start`. Green after. Mutation (each fix stashed alone,
  restored and touched): without (1) the cost test fails (9 for 5) while the first-frame test passes
  (the memo no longer keeps the shim's size); without (2) the memo test fails and the first-frame test
  passes (no early measurement). `nested_subgrids_are_sized_a_bounded_number_of_times` still pins 2 +
  2 × depth. No snapshot changed.
- 2026-10-09 — C7G-SUBGRID-COST (architect N4). (1) The subgrid size memo's key was
  `format!("{dimension:?} {area} {inherit:?}")`, a Debug string of every inherited name and extent per
  lookup; it is now structural (`grid::subgrid_memo`: `Inherit` / `Inherited` derive `Hash` / `Eq`, a
  lookup hashes the borrowed key and compares on a hit, so a hit copies nothing; the table is the
  pass's document data as before, `memo::with_subgrids`). (2) "Memoize `flatten` / `place_grid`":
  measured first. Chains subgridding one axis throughout (columns, rows, both) were already linear
  (4–7 placements a level: an inner level's subgridded axis is fixed, so only the outermost run
  flattens the chain). The superlinear shape is a chain alternating the axis — 7, 14, 39, 61, 161,
  228, 598 sizing calls at depths 1–7 — and its root was not a missing memo: `flatten` pushed a nested
  subgrid on the *other* axis as a plain item, measured through `intrinsic_size` as a grid with no
  inherited tracks (its parent's lines not laid out yet), which C7G-MEMO-PURITY rightly keeps out of
  the memo, so it was sized again for every run above it — and sized wrong: as `run_items` does for
  a grid's own items, it must be measured with the axis it inherits. `flatten`'s per-item work is
  now `own_items`, which gives such an item `size::measure_subgrid`'s size (memoized, keyed by the
  inheritance). With that, a `flatten` memo was tried and made no difference to any count (mutation:
  disabled, the tests stay green), so it was not kept. (3) `size_grid` takes `with_lines`: only a
  layout builds `GridLines` (copying each line name to a `String`); a measurement builds none, and
  `arrange` debug-asserts it got them. `Explicit::with_areas`' per-area `format!` is unchanged (its
  names are needed by placement). Red, in `grid::cost_tests`:
  `nested_subgrids_are_placed_a_bounded_number_of_times` — the alternating chain's counts above (and
  the column chain's placements pinned linear, green before and after);
  `measuring_a_grid_copies_no_line_names` — 93 allocations for named lines against 81 unnamed, now 84
  (the three per-line lists placement reads, names borrowed); `subgrid_memo::tests::
  a_lookup_allocates_nothing` pins the hit at 0. Found on the way and fixed with (2):
  `css_phase7/subgrid.rs::a_nested_subgrid_on_the_other_axis_sizes_its_ancestors_rows` — a columns
  subgrid inside a rows subgrid, the outer row 2 tall for 1 on the first frame (mutation: the plain-item
  path back → 2; restored and touched). No snapshot changed.
- 2026-10-09 — C7G-FOCUS-FIXUP (architect N6, API upgrade-guide finding): `draw_if_dirty` ran the
  focus fixup (HTML "update the rendering": a focused area that is no longer a focusable area is
  blurred) only on frames that cascaded, but the used `visibility` also changes when a running
  transition steps — a transition to `hidden` presents `visible` until it ends (CSS Display 3 §4), and
  the frame it ends in has no cascade. Decision: the hook is the frame pipeline after the transitions
  advance — the fixup runs on every frame that `laid_out` (a cascade, or a step of the running
  transitions; `style_and_layout` advances them only then), one ancestor walk. Red:
  `css_phase6/visibility_answers.rs::a_visibility_transition_that_ends_hidden_blurs` — `transition:
  visibility 40ms`, the button still focused after the frame the transition ended in (`Some(b)` for
  `None`); green after (the test sleeps past the transition: the frame clock is `Instant::now()`).
  The second half — `focus()` on an element shown in the same handler, "open panel, focus input" —
  was weighed and not landed: a browser flushes style inside `focus()`, and doing so here needs the
  cascade's inputs, which are `App` state (the author, `<style>` and `@property` sheets, the
  transition registry a flushed change must start its transitions in, the dirty tracker the frame must
  then not cascade again) that a handler's `Dom` cannot reach; `is_focusable_in_opened` generalizes
  only to boxes the caller itself opened. Moving that state behind the document is a runtime change
  of its own, recorded as TECH_DEBT `FOCUS-FLUSH-1`; DIVERGENCES keeps "Focusability reads the last
  cascade's styles", its fixup sentence now true, and names the debt. The upgrade-guide line for the
  refusal is batch B's.
