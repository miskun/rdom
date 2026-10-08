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
| 1 | Syntax, cascade, custom properties | done 2026-10-04 (both gates; 20 gate fixes `C1G-*`; their re-review rides with the Phase 2 gate) |
| 2 | Values, units, math functions | done 2026-10-04 (both gates; 20 gate fixes `C2G-*`; their re-review rides with the Phase 3 gate; C2-LH closed with C9-LINE-HEIGHT) |
| 3 | Color | done 2026-10-05 (both gates; 16 gate fixes `C3G-*` incl. rdom's own terminal input reader; re-review rides with the Phase 4 gate) |
| 4 | Backgrounds and borders | done 2026-10-05 (both gates; 19 gate fixes `C4G-*`; their re-review rides with the Phase 5 gate; C4-SPACING layout with C13-TFC) |
| 5 | Box model and sizing (incl. logical properties) | done 2026-10-05 (both gates; 19 gate fixes `C5G-*`; their re-review rides with the Phase 6 gate; C5-CONTAIN-SIZE use with C14-CONTAIN) |
| 6 | Display, visibility, flexbox, box alignment | done 2026-10-05 (both gates; 28 gate fixes `C6G-*`; their re-review rides with the Phase 7 gate) |
| 7 | Grid | done 2026-10-05 (both gates; 15 gate fixes `C7G-*`; their re-review rides with the Phase 8 gate) |
| 8 | Positioning, floats, overflow, scrolling | done 2026-10-06 (both gates; 15 gate fixes `C8G-*`; their re-review rides with the Phase 9 gate) |
| 9 | Inline text and decoration | done 2026-10-06 (both gates; 14 gate fixes `C9G-*`; their re-review rides with the Phase 10 gate) |
| 10 | Lists, counters, generated content, pseudo-elements | done 2026-10-08 (both gates; 19 gate fixes `C10G-*`; their re-review rides with the Phase 11 gate) |
| 11 | Selectors | done 2026-10-08 (both gates; 15 gate fixes `C11G-*`; their re-review rides with the Phase 12 gate) |
| 12 | Transitions, animations, user interface | done 2026-10-08 (both gates; 18 gate fixes `C12G-*`; their re-review rides with the Phase 13 gate) |
| 13 | Tables (real table formatting context) | done 2026-10-09 (both gates; 17 gate fixes — 15 `C13G-*`, and `C13-ROOT-BLOCK` / `C13-ROOT-CANVAS`, the root block container; their re-review rides with the Phase 14 gate) |
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
| C2-LH | `lh` / `rlh` (one row × `line-height`; lands with C9-LINE-HEIGHT) | done (with C9-LINE-HEIGHT) |
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
| C4-SPACING | `border-spacing` (lands with the table phase if it needs the TFC) | done |

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
| C8-PARSE-ERROR | Every public error type implements `Display` and `std::error::Error` (found by C7G-README-GRID) | done |
| C8-Z-INDEX | `z-index` full integer range | done |
| C8-FLOAT | `float` / `clear` (line-box exclusion, clearance) | done |
| C8-OVERFLOW-CLIP | `overflow: clip`, two-value `overflow`, `overflow-clip-margin`, logical `overflow-block` / `-inline` | done |
| C8-TEXT-OVERFLOW | `text-overflow: clip / ellipsis / <string>` | done |
| C8-LINE-CLAMP | `line-clamp` / `max-lines` / `block-ellipsis` / `continue` | done |
| C8-SCROLLBAR | `scrollbar-gutter: both-edges`, `scrollbar-width`, `scrollbar-color` | done |
| C8-OVERSCROLL | `overscroll-behavior` (+ axis / logical longhands) | done |
| C8-SCROLL-PADDING | `scroll-padding*` / `scroll-margin*` | done |
| C8-SNAP | `scroll-snap-type` / `-align` / `-stop` | done |
| C8-OVERFLOW-TEXT | A non-clipping descendant's overflowing line boxes count toward the ancestor's scrollable overflow | done |
| C8-CB-COMPLETE | Containing block for positioned boxes, completing C7-ABSPOS-PADDING-EDGE: `sticky` ancestors in the ancestor walks, the scrollbar gutter excluded, scroll offsets applied inside a positioned scroller, one shared ancestor walk (elements and pseudo-elements, incl. the §9.1 grid area for pseudos) | done |
| C8-POS-MINMAX | Positioned boxes (elements and pseudo-elements) honour `min-*` / `max-*` (CSS 2.1 §10.4 / §10.7 with §10.3.7 / §10.6.4); a `position: relative` pseudo with both insets takes its declared width | done (C5-POS-MINMAX + C5G-REL-PSEUDO-INSETS) |
| C8-ABSPOS-OVERFLOW | An absolutely positioned box in its scroll container's scrollable overflow (CSS Overflow 3 §2.2; TECH_DEBT `ABSPOS-OVERFLOW-1`) | done |
| C8-RTL-LINE-OVERFLOW | An overflowing `rtl` line starts at the right edge and overflows the left (DIVERGENCES §4, from C8-TEXT-OVERFLOW) | done |

(Scroll-driven animations land in phase 12.)

### Phase 9 — Inline text and decoration (audit §3.12, §3.13, §3.14)

| Id | Item | Status |
|---|---|---|
| C9-WHITE-SPACE | `white-space: pre-line / break-spaces`; `white-space-collapse` / `text-wrap-mode` longhands | done |
| C9-TEXT-WRAP | `text-wrap` / `text-wrap-style` (`balance` / `pretty` / `stable`) | done |
| C9-TEXT-ALIGN | `text-align` (incl. `start` / `end` / `justify`), `text-align-last`, `text-justify` | done |
| C9-TEXT-INDENT | `text-indent` | done |
| C9-TEXT-TRANSFORM | `text-transform` (case mapping, `full-width`) | done |
| C9-TAB-SIZE | `tab-size` and real tab stops in `pre` | done |
| C9-BREAKING | `word-break`, `overflow-wrap` / `word-wrap`, `line-break`, `hyphens` (soft hyphens) | done |
| C9-LINE-HEIGHT | `line-height` (whole-row line boxes) | done |
| C9-VERTICAL-ALIGN | `vertical-align` for inline-blocks and inline content | done |
| C9-DECORATION | `text-decoration` full shorthand; `-line` (incl. `overline`), `-color` (SGR 58), `-style` (SGR 4:x) | done |
| C9-FONT | `font-weight` numeric / `bolder` / `lighter`, `font-style: oblique`, `font` shorthand (weight / style honored, size / family inert) | done |

### Phase 10 — Lists, counters, generated content, pseudo-elements (audit §3.15, §3.16)

| Id | Item | Status |
|---|---|---|
| C10-CONTENT | `content` full grammar (quotes, `var()`, `counters()`, alt text) | done |
| C10-QUOTES | `quotes` | done |
| C10-COUNTERS | `counter-reset reversed()`, `counter-set`, `counters()`, all predefined counter styles | done |
| C10-COUNTER-STYLE | `@counter-style` and `symbols()` | done |
| C10-LIST-ITEM | `display: list-item`, `list-style-type` / `-position` / `list-style`, `marker-side`, `::marker` (replaces the `li::before` divergence); a marker riding a descendant's line is measured through the packer, not by its raw width (from C9G-MISC-CORRECTNESS / C9G-PSEUDO-CLAMP) | done |
| C10-FIRST | `::first-line` / `::first-letter` | done |
| C10-LEGACY-COLON | Single-colon `:before` / `:after` / `:first-line` / `:first-letter` | done |
| C10-HIGHLIGHT | `::highlight()` with a Custom Highlight API surface | done |
| C10-DETAILS-CONTENT | `::details-content` | done |
| C10-PSEUDO-CHAINS | Pseudo-element followed by user-action pseudo-classes (`::before:hover`) and nested pseudo-elements where defined | done |
| C9-CARRY-INDENT | A `calc()` `text-indent` shared by the elements that inherit it, not cloned per element (from the Phase 9 close) | done |
| C10-PSEUDO-UNIFY | Positioned `::before` / `::after` on the generated-box path: the positioning layer places, stacks, hit-tests and scrolls them as elements (from C10-LIST-ITEM's note; `positioned_pseudos` gone) | done |

### Phase 11 — Selectors (audit §3.17)

| Id | Item | Status |
|---|---|---|
| C11-ATTR-FLAGS | Attribute selector case flags `i` / `s` | done |
| C11-IS | `:is()` | done (landed early as C1G-IS-PARSE) |
| C11-HAS | `:has()` with invalidation | done |
| C11-NTH | `:nth-child()` / `:nth-last-child()` (+ `of S`), `:nth-of-type()` / `:nth-last-of-type()`, `:first-of-type` / `:last-of-type` / `:only-of-type` | done |
| C11-SCOPE | `:scope` (query APIs and `@scope`) | done |
| C11-FORM-STATES | `:indeterminate` (checkbox, radio group), `:user-valid` / `:user-invalid`, `:read-only` / `:read-write`, `:in-range` / `:out-of-range`, `:default` | done |
| C11-MODAL-POPOVER | `:modal`; the `popover` attribute and `:popover-open` | done |
| C11-LINK-LANG | `:link` / `:any-link`, `:lang()` | done (with `:dir()`, deferred here by C5-WRITING, and `:visited` never matching) |
| C11-COLUMN | Column combinator `\|\|` | moved to Phase 13 as C13-COLUMN: it selects the cells a column spans, which needs C13-TFC's real table columns |

### Phase 12 — Transitions, animations, user interface (audit §3.18, §3.19)

| Id | Item | Status |
|---|---|---|
| C12-TIMING | `transition-timing-function` full (`linear()`, `steps()` positions), negative `transition-delay` || done |
| C12-BEHAVIOR | `transition-behavior: allow-discrete` || done (`content-visibility` with C14-CONTAIN) |
| C12-ANIMATABLE | Every animatable property this program adds interpolates. **Found by C10G-DETAILS-CONTENT-BOX: geometry transitions never reach layout** — a `width`, `height`, `padding`, `gap` or inset transition runs and fires its events, but layout reads the end value at once (paint alone follows `padding` / `gap`), for every element and `::details-content`; make layout read the animated value (DIVERGENCES §3) || done |
| C12-KEYFRAMES | `@keyframes` and all `animation-*` properties, animation events | done |
| C12-STARTING | `@starting-style` || done |
| C12-SCROLL-DRIVEN | `scroll-timeline*` / `view-timeline*` / `animation-timeline` / `animation-range*` | done |
| C12-OUTLINE | `outline` / `-color` / `-style` / `-width` / `-offset` (non-layout ring) | done |
| C12-CURSOR | `cursor` (OSC 22 pointer shapes) | done |
| C12-CARET | `caret-shape` / `caret-animation` / `caret` | done |
| C12-FOCUS-FLUSH | `focus()` (and other style-reading DOM calls) flushes pending style for the element first, as browsers do — TECH_DEBT `FOCUS-FLUSH-1`; needs the sheet set / transition registry / dirty tracker reachable from a handler's `Dom` | done |
| C12-CONTROLS | `accent-color`, `appearance`, `field-sizing`, `resize` | done |
| C12-SELECT-TOP-LAYER | `<select>`'s drop-down picker in the top layer, light-dismissed like a popover (Phase 11 API N7's "consider") | done |

### Phase 13 — Tables (audit §3.20)

| Id | Item | Status |
|---|---|---|
| C13-TFC | A real table formatting context: `display: table` family on any element, `rowspan`, automatic and `fixed` `table-layout` (replaces `TABLE-TFC-1`) | done |
| C13-TABLE-PROPS | `caption-side`, `empty-cells`, `border-spacing` (separated borders), `vertical-align` on cells | done |
| C13-COLUMN | Column combinator `\|\|` (Selectors 4; was C11-COLUMN): `col.x \|\| td` matches the cells of the columns a `<col>` spans, from C13-TFC's column model | done |

### Phase 14 — Conditional rules, containment (audit §3.21)

| Id | Item | Status |
|---|---|---|
| C14-MEDIA | `@media` (cell-sized viewport, `prefers-color-scheme`, `prefers-reduced-motion`, `hover` / `pointer`, …) and `matchMedia` | done |
| C14-SUPPORTS | `@supports` (feature queries against the dispatch table; `CSS.supports`) | done |
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

- 2026-10-04 — Program opened at Miska's request: "address all partials and missing but meaningful
  in a terminal", CSS completeness as 0.6.0 before routing (now 0.7.0). Built from the 307-row
  audit in `CSS-COVERAGE.md` (`ba585c7`).
- 2026-10-04 — Phase 0 done (C0-CONTRADICTIONS, C0-NOT-SHIPPED): the six `DIVERGENCES.md`
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
- 2026-10-04 — Phase 1 closed: 11 items + 20 gate fixes. The gate fixes' re-review is folded into the
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
- 2026-10-04 — Phase 2 gates (with the C1G re-review: all 20 fixed at the root). Architect: 3 blocking —
  flex factors summing to 1 lose a cell (f32 sum below 1.0 trips step 4.b); counter replay drops
  `::before` / `::after` ops on kept subtrees (cascade and restyle paths); no calc nesting cap (attr
  data can overflow the stack). API: 1 blocking — cascade forms without a viewport resolve `vw`
  against 0x0, including the README's DirtyTracker pattern. ~35 non-blocking (IEEE division,
  `-infinity` negation overflow, zero-basis folding in number/angle math, percentages in `CalcKind`,
  math in integer / registered properties, viewport field list hand-kept, `max-width: none`
  unrepresentable, `flex` shorthand ignores shrink, parse-time `attr()` validation, `:root` `attr()`,
  u16 overflows, unvalidated `AspectRatio` / `Flex`, restyle walk cost, renames, re-exports, README
  0.2.0 history edit, changelog paths and hints). Decision: fix all as `C2G-*` items, two batches.
- 2026-10-04 — C2G-FLEX-SUM: §9.7 step 4.b's "sum below one" and the rolling floors of grow and
  shrink use one relative tolerance (`FACTOR_TOLERANCE`, four `f32` epsilons) instead of an exact
  `f64` comparison and a fixed `1e-9`. Decided against summing in `f32`: it only moves the rounding
  (`10 × 0.1` sums to 1.0000001 in `f32`) and leaves the floors seeing `71.9999999`, which also lost
  a cell for a genuine 0.9 sum. Tests: 0.1 / 0.2 / 0.7 fills 80 (grow and shrink), 0.2 + 0.7 takes
  exactly 72, a 0.1 item frozen by `max-width` leaves 0.9 to share 72.
- 2026-10-04 — C2G-COUNTER-PSEUDO: `CounterState::replay_element` (with `StoredOps`, the `Rc`s of
  the element's, `::before`'s and `::after`'s computed styles) replays a kept element in tree order —
  element, `::before`, children, `::after` — and is the one replay used between `cascade_subtrees`
  roots, for a restyle's kept element (its pseudos; its own ops were applied computing it) and for
  its kept children (`replay_children`, was `replay_subtree`). Tests: a class change on the third of
  three `h2::before`-numbered headings reads "3. " (was "1. "); `restyle_vars` keeping a root `h2` and
  a `div` of two reads "4. " for the next; a kept `div`'s `::after` counts after its children; a kept
  `div::before { counter-reset }` scopes its children (green before the fix too — an order guard).
- 2026-10-04 — C2G-CALC-DEPTH: the calc parser caps nesting (`MAX_CALC_NESTING` = 32 math
  functions / parentheses; the parser recursed ~4 frames per level) and tree depth
  (`MAX_CALC_DEPTH` = 256, tracked as nodes are built, so an over-deep chain is rejected before it
  exists and nothing — type check, evaluation, `absolutize`, serialization, `Drop` — ever walks one).
  Chosen over an iterative `Drop` / balanced trees: one bound covers every walker, `-` and `/` do not
  re-associate, and 256 operands is far past hand-written CSS. A run of unary `+` is a loop. Red: a
  20 000-level `attr()` value aborted the test process (stack overflow); green: invalid, fallback.
- 2026-10-04 — C2G-VIEWPORT-DOC: the viewport is the document's. rdom-core gains document data
  (`Dom::document_data` / `set_document_data` / …, one value per Rust type, `document_data.rs`) — the
  substrate's renderer-free hook for per-document backend state, as `Ext` is per node; the root is a
  fragment with no `Ext`, so no node could hold it. rdom-tui stores the `Viewport` there
  (`style/cascade/viewport.rs`); every cascade form and `restyle_vars` read it,
  `CascadeExt::set_viewport` / `viewport` set and read it, `layout_dom(area)` records its area, the
  `App` sets its terminal's size each frame (its `cascaded_viewport` still decides the full
  re-cascade). Decided: `cascade_all_in` / `cascade_subtrees_all_in` removed (unreleased) — one way
  to give the size. `Viewport` joins the prelude. Phase 14's `@media` reads the same value.
- 2026-10-04 — C2G-CALC-SEMANTICS: IEEE division (parse-time literal rejection and the runtime
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
- 2026-10-04 — C2G-VIEWPORT-FIELDS: `ResolveCtx::viewport` is `Option<Viewport>` (unreleased field),
  `None` from `ResolveCtx::new` — every layout resolve — and `CalcUnit::canonical` debug-asserts a
  viewport unit never meets `None` (0 cells in release). The hand-kept field list in
  `ComputedStyle::resolve_viewport_units` stays (one place, typed per field), guarded by a test that
  sets every property of `property_dispatch::property_names()` to `10vw` (or `10vw 10vw`), cascades
  at 80 × 20 and requires no `Viewport(` in the computed style's `Debug` and a clean layout — so a
  length property added later is covered without editing the test. Checked red: dropping `gap` from
  the list fails it.
- 2026-10-04 — C2G-MAX-NONE (completes C5-MINMAX-SIZE): `TuiStyle::max_width` / `max_height` are
  `Option<Value<Option<MaxSize>>>` like `aspect_ratio`, applied with `value!` (the declared `Option` is
  the computed one; `ComputedStyle` keeps `Option<MaxSize>`, `None` = `none`); `parse_max_size`
  takes `none`, serialization writes it back. `set_max_width` / `set_max_height` take
  `impl Into<Option<MaxSize>>` and always declare (`None` is `none`; removal through the CSSOM) —
  decided over a double `Option`. The C2-PERCENT changelog bullets are rewritten to the final shape
  with migration hints from 0.5.0. COVERAGE keeps the row *Partial* (intrinsic keywords, C5-INTRINSIC).
- 2026-10-04 — C2G-FLEX-SHORTHAND: `parse_flex_shorthand` → `FlexShorthand { grow, shrink, basis }`
  with the Flexbox §7.2 grammar (`none`; `<grow> <shrink>? || <basis>` in either order; omitted grow /
  shrink 1, omitted basis 0; a number is a factor unless two factors precede it). The dispatch writes
  grow → `width` / `height` as before, shrink → `flex_shrink` (was 0 for any zero grow: `flex: 0 1
  auto` overflowed), basis → the new `TuiStyle` / `ComputedStyle::flex_basis` (`FlexBasis`, mask bit
  45, cascaded, viewport units absolutized, in `layout_differs`) — stored, not laid out: C6-FLEX-LONGHANDS
  is now *partial* with that gap. `flex` serializes as `<grow> <shrink> <basis>`. DIVERGENCES' flex entry
  rewritten (the stale `Size::Flex(1)` / `parse/values.rs`); the C2-NUMBER changelog example now says
  what `flex: 1.5 0.5 0%` sets. The `initial`-keyword apply test perturbs `flex_basis` through `flex`.
- 2026-10-04 — C2G-ATTR-PARSE: `attr::valid_args` parses the head at parse time (an unknown unit, a
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
- 2026-10-04 — C2G-LAYOUT-SAFETY: `Padding::horizontal` / `vertical` (saturating) replace the
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
- 2026-10-04 — C2G-RESTYLE-WALK: partial walks moved to `style/cascade/subtrees.rs`. Roots are reduced
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
- 2026-10-04 — C2G-STATELESS-REGISTRY: the stateless `CascadeExt` forms take the registry from
  `registered::document_registry` — document data holding the last sheet set's registry, keyed by
  each sheet's `Stylesheet::version()` (new in rdom-style: a process-unique stamp from one atomic
  counter, renewed by every `&mut` / builder mutation, fresh on `Clone`, so equal keys are the same
  sheets unchanged; decided over pointer identity, which a dropped-and-reallocated sheet would
  alias). `Sheets::new` takes the registry (no `Option`). Test: two `cascade`s and a
  `cascade_subtrees` with one sheet build 1 registry (was 3) and keep the element's match record
  (`Rc::ptr_eq`); a mutated sheet and another list each build one.
- 2026-10-04 — C2G-REGISTERED-ABSOLUTE: `PropertySyntax::computed(value, viewport)`
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
- 2026-10-04 — C2G-CONTENT-ATTR: `Content::Attr`, `ContentContext::attr` and `resolve_content_on`'s
  attribute lookup are deleted; the five UA rules that read an attribute (`input` / `textarea`
  placeholder, `input[type=button|submit|reset]` value, `input[type=image]` alt, `optgroup` label) are
  CSS declarations built with `ua::css` (`property_dispatch::set`), so they go through the `attr()`
  substitution path like an author's `content: attr(x)`. Breaking for rdom-style (both were in 0.5.0):
  CHANGELOG migration hint. No test expectation changed; the one cascade test that built
  `Content::Attr` declares `content: attr(data-status)` instead. UA-dependent tests (placeholder,
  button labels, optgroup) green.
- 2026-10-04 — C2G-SUBSTITUTION-ERRORS: `SubstitutionError { Undefined, Cycle, InvalidAttr, TooLong,
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
- 2026-10-04 — C2G-CELLS-CONVERSIONS: `Size::cells` / `cells_u16` and `Length::cells` (rdom-style
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
- 2026-10-04 — C2G-FLEX-SPLIT: `flex/main_axis.rs` (525 lines after C2G-CELLS-CONVERSIONS) keeps the
  §9.2 gathering (`ChildMain`, `MainNatural`, `collect_main_axis_items`, 221 lines); the §9.7
  distribution — `MainAxisBudget`, `resolve_flexible_lengths`, `FACTOR_TOLERANCE`, the grow / shrink
  freeze loops and the §4.5 auto-min floor — moves verbatim to `flex/distribute.rs` (314 lines). No
  behaviour change; flex suites unchanged.
- 2026-10-04 — C2G-REEXPORT-CALC: `pub use rdom_style::calc` in `rdom_tui` (the module, so `CalcExpr`,
  `ResolveCtx`, `to_cells`, … are all reachable; `Viewport` stays at the root too) and
  `MinSize::percent` / `MaxSize::percent` (`Calc(Percent(p))`, the parser's form). Red: the integration
  test did not compile (no `rdom_tui::calc`, no `percent`); green: a `set_max_width(MaxSize::percent(50.0))`
  child of an 80-column box is 40 wide.
- 2026-10-04 — C2G-DOCS: README's 0.2.0 bullet restored to its released text ("`calc()` value system",
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
- 2026-10-04 — C2G-TEST-GAPS: every test the gate listed already exists, so none was added — calc
  nesting depth (`parse/values/calc_tests.rs`: `MAX_CALC_NESTING` / `MAX_CALC_DEPTH` caps; the
  gates suite's hostile `attr()`), `1/0` (`calc/semantics_tests.rs::division_by_zero_is_ieee`), an
  inset of `-infinity` (`css_phase2_gates.rs::infinite_insets_lay_out_without_overflow`), opacity with
  a percentage calc (`semantics_tests.rs::percentages_have_their_own_type`: `calc(50%)`, `calc(50% +
  0.25)`, `calc(50% * 50%)`), a fractional factor with `max-width` in the freeze loop
  (`fractional_factor_with_max_width_in_the_freeze_loop`) and an indefinite-height flex `max-height:
  %` (`max_height_percent_in_an_auto_height_flex_container_is_none`). `ScopeMemo`'s O(N × depth)
  memory is recorded in TECH_DEBT as the accepted simplification `SCOPE-MEMO-1`, with its bound.
  Phase 2 gate batch B (C2G-RESTYLE-WALK … C2G-TEST-GAPS) complete.
- 2026-10-04 — Phase 2 closed: 11 items + 20 gate fixes. Gate-fix re-review folded into the Phase 3 gate.
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
- 2026-10-04 — Phase 3 gates (with the C2G re-review: all 20 at the root). Architect: 2 blocking —
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
- 2026-10-04 — C3G-COLOR-DEPTH: `ColorCx::nested` caps color-function nesting at `MAX_COLOR_NESTING`
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
- 2026-10-04 — C3G-PSEUDO-TINT: `positioned_pseudos` writes its `content` with
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
- 2026-10-04 — C3G-RELATIVE-COMMA: `relative::parse` rejects a comma only at the arguments' top level
  (`channel::top_level_comma`, the check the legacy split already used), so a math function's own
  commas pass. Red: `rgb(from red min(r, 100) g b)` parsed to `None`; green: `rgb(100, 0, 0)`, and
  `oklch(from red clamp(0.2, l, 0.5) c h)` equals `oklch(from red 0.5 c h)`; through `var()` end to
  end in `css_phase3_gates.rs`. A comma between channels stays invalid.
- 2026-10-04 — C3G-SCHEME-CONSISTENCY: (1) the caret resolves `caret-color` / `caret-text-color` with
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
- 2026-10-04 — C3G-POWERLESS-HUE: source fetched 2026-10-05 — CSS Color 4 §4.4.1 (a hue is powerless
  when the chroma or saturation is ≤ the space's ε; lightness is not a criterion) and its sample
  code: `conversions.js` `Lab_to_LCH` ε = 0.0015, `OKLab_to_OKLCH` ε = 0.000004 (both `chroma <=
  epsilon`), `better-rgbToHsl.js` ε = 1/100000 of a saturation of 1 (`sat <= epsilon`); `hwb()` §8,
  whiteness + blackness ≥ 100%. `interpolate::hue_is_powerless` (extracted from `in_space`) uses
  exactly those, cited in its doc; dropped: `L ≤ 0` for LCH / Oklch and HSL's lightness 0% / 100%
  test (the conversion already gives such a color a saturation of 0, as the sample code does).
  Red: LCH chroma 0.0016 counted as powerless (old ε 0.005625); green: boundary tests at each ε and
  just above, HWB at 100% / 99.9%, `L = 0` with chroma not powerless. No existing expectation
  changed.
- 2026-10-04 — C3G-SMALL-FIXES: (1) `border-color`'s initial `currentcolor` has one owner,
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
- 2026-10-05 — Phase 3 closed: 10 items + 16 gate fixes. Found during C3G-PSEUDO-SIZE: positioned boxes ignore
  `min-*` / `max-*` — added as C8-POS-MINMAX. The input reader (C3G-INPUT-READER) gets a focused look in the
  Phase 4 gate's re-review.
- 2026-10-05 — C4-BACKGROUND: `background` parses Backgrounds 3 §3.10 in full (`V/background.rs`):
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
- 2026-10-05 — C4-BG-CLIP: found as specified for the default — the box fill already covered the
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
- 2026-10-05 — C4-BORDER-SHORTHAND: `border` / `border-<side>` parse `<line-width> || <line-style> ||
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
- 2026-10-05 — C4-BORDER-SIDES: `border-style` / `-color` / `-width` take 1–4 values
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
- 2026-10-05 — C4-BORDER-WIDTH: `BorderWidth::weight` maps a width to `BorderWeight::{Light, Heavy}` or
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
- 2026-10-05 — C4-RADIUS: `border-radius` (1–4 values, `/` vertical radii; the top-level `/` only)
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
- 2026-10-05 — C4-SHADOW: `box-shadow` parses `none | <shadow>#` (`V/shadow.rs`: lengths contiguous,
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
- 2026-10-05 — C4-SPACING (partial): `border-spacing` parses one or two non-negative cell lengths
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
- 2026-10-05 — Phase 4 gates (with the C3G re-review: all 16 at the root; the input reader matches
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
- 2026-10-05 — C4G-ESC-GRACE: `InputReader::poll` no longer flushes an expired escape prefix before
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
- 2026-10-05 — C4G-OSC-DISCARD: new `parse/string.rs` holds the command-string framing (ECMA-48
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
- 2026-10-05 — C4G-ESC-ESC: `ESC ESC` consumed both bytes for one Esc (crossterm's reading), so
  two quick Esc presses were one. `parse::escape_then` now decides on the third byte: `ESC` + a CSI or
  SS3 key is Alt + that key; anything else is Esc with the second `ESC` read again (`ESC ESC x` →
  Esc, Alt+x; `ESC ESC` + mouse report → Esc, the mouse event). `awaits_prefix` adds `ESC ESC` and
  `ESC ESC [` / `O`, and `flush_prefix` gives each leading `ESC` its own Esc. Decided: legacy
  `ESC ESC [ A` is Alt+Up — rxvt sends it for Alt+arrow and Terminal.app with Option as Meta sends
  `ESC` before the key's sequence; crossterm typed it as Esc, `[`, `A`. Red: the new corpus test
  (`ESC ESC x` gave Esc, plain `x`); green after. Changed expectation: `alt_keys` asserted
  crossterm's one Esc for `ESC ESC` — removed, the new test covers both readings.
- 2026-10-05 — C4G-CSI-FRAMING: `csi::parse` frames every sequence whose first byte after `ESC [` is
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
- 2026-10-05 — C4G-CTRL-F3: grepped for a cursor position request — no `CSI 6 n`, no
  `crossterm::cursor::position` anywhere in the workspace; the startup query sends OSC 11 and DA1 only.
  So `csi::dispatch` no longer consumes `CSI … R`: `CSI 1 ; m R` goes to `keys::modified` (F3 with
  modifiers) and a bare `CSI R` is F3 beside `P` / `Q` / `S`. Red: `csi_r_is_f3` (`CSI R` gave
  nothing); green after. Changed expectation: `other_replies_are_consumed` dropped its
  `CSI 20 ; 10 R` (now F3 with a modifier mask, as a terminal would mean it).
- 2026-10-05 — C4G-LEAVE-TUI: `leave_tui_mode` delegates to the private `restore_terminal(writer,
  raw_off)`, which `queue!`s each step on its own — pop the keyboard flags (first: kitty keeps a stack
  per screen), focus off, mouse off, bracketed paste off (added: never enabled by rdom, but an app may
  have), cursor shown, alternate screen left, mode 2031 off (Unix), SGR reset — then flushes and calls
  `raw_off`, recording only the first error. The raw-mode switch is a parameter so a test can observe
  it. Red: a writer failing its first write left nothing written (the `?1000l` assertion), and an
  always-failing writer never reached `raw_off` (0 calls); green after. (The first attempt at the test
  failed with `ErrorKind::Interrupted`, which `write_all` retries forever — the test writer uses
  `io::Error::other`.)
- 2026-10-05 — C4G-SHADOW-CLAMP: `PaintLength::offset_cells` clamps to ±`u16::MAX` cells (it saturated
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
- 2026-10-05 — C4G-MIXED-CORNERS: `border_join` reads each direction's line (`glyphs::Line`: none,
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
- 2026-10-05 — C4G-SHADOW-ORDER: fixed, without a second walk. `stacking::collect_layers`, which
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
- 2026-10-05 — C4G-BORDER-COST: `paint_border_sides` reads `border_width` / `border_radius` in place
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
- 2026-10-05 — C4G-PAINT-SPLIT: no behaviour change. `paint_pass/mod.rs` (583 lines after
  C4G-SHADOW-ORDER) → `mod.rs` (188: `PaintExt`, `layout_rect_to_grid`, the module map),
  `stacking_walk.rs` (177: `paint_stacking_context` / `_body`, `paint_layers`, `paint_plain`,
  `recurse_children`, `orphan_inline`, `paints_child_box`) and `box_paint.rs` (254: `BoxFrame`,
  `paint_box`, `paint_content`, `compute_border_priority`, `fills`); `fills`, `paints_child_box` and
  `paint_stacking_context` re-exported at their old paths. Every test passes unchanged. Phase 4 gate
  batch A (C4G-ESC-GRACE … C4G-PAINT-SPLIT) done; batch B open.

- 2026-10-05 — C4G-IMPORTANT-BITSET: `ImportantMask` is an opaque `[u64; N]` (`tui_style/important.rs`),
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
- 2026-10-05 — C4G-REEXPORTS: the `rdom_tui` root and prelude re-export the Phase 4 value types
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
- 2026-10-05 — C4G-NUMBER-RANGE: the tokenizer keeps digits-only literals integer-typed and clamps
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
- 2026-10-05 — C4G-PX-CALC: `paint_length` (`V/border.rs`), the one leaf of border widths, radii and
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
- 2026-10-05 — C4G-SERIALIZE: two roots. (1) `render_value` put a space before every token. Decided
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
- 2026-10-05 — C4G-SEALED: decided once in DESIGN ("Which rdom-tui traits a consumer implements"):
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
- 2026-10-05 — C4G-DOCS: docs only. CSS-COVERAGE — the `border-style` row no longer claims `rounded`
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
- 2026-10-05 — C4G-EDGE-TESTS: `tests/integration/css_phase4_gates.rs`, one test per edge case.
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
- 2026-10-05 — Phase 4 closed: 8 items (C4-SPACING partial — layout with C13-TFC) + 19 gate fixes (batch A 11, batch B 8). Gate-fix re-review folded into the Phase 5 gate. Carried to Phase 5: a box smaller than its border and padding is not floored at them (found by C4G-EDGE-TESTS, DIVERGENCES §2) — C5-BOX-SIZING.
- 2026-10-05 — C5-SPLIT: no behaviour change. `accessors/mod.rs` (676) → `mod.rs` (51: module docs,
  the module map, re-exports), `read_api.rs` (378: `TuiAccessors`, `DomRect`) and `write_api.rs` (261:
  `TuiAccessorsMut`) beside the existing impl files. `runtime/timers.rs` (1153: 603 code, 550 tests) →
  `runtime/timers/{mod (182: shared handle, current-scheduler guard, `TimerId`, `TimerCtx`),
  scheduler (262: the queues), pump (107: the drains and the microtask checkpoint), ext (86:
  `TuiTimers`), tests (550)}`. Public paths unchanged (`runtime::timers::{TimerCtx, TimerId,
  TuiTimers}`, `accessors::*`). Every test passes unchanged.
- 2026-10-05 — C5-BOX-SIZING: `box-sizing: content-box | border-box` (CSS UI 3 §3.1, now CSS Sizing 3
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
- 2026-10-05 — C5-INTRINSIC: `min-content | max-content | fit-content | fit-content(<length-percentage
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
- 2026-10-05 — C5-POS-MINMAX (found by C5-INTRINSIC, a fix between items): an absolutely positioned
  box's placed width / height ignored `min-*` / `max-*` (CSS 2.1 §10.4 / §10.7 apply to it), as did a
  positioned pseudo-element's. `compute_placed_rect` now clamps the tentative width by `max-width`, then
  `min-width` (through `Keywords`, so `box-sizing` and the keywords apply), before the height is
  measured at it, then the height the same way; pseudo-elements clamp through their `Sizer`, a keyword
  bound being the content string's size (DIVERGENCES §2). Red: `positioned_boxes_honour_min_and_max`
  gave (10, 1) for `width: 10; max-width: 4; height: 1; min-height: 3`; green after, with
  `a_positioned_pseudo_honours_min_and_max`. No existing expectation changed.
- 2026-10-05 — C5-MARGIN-TRIM: `margin-trim: none | [block || inline] | [block-start ||
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
- 2026-10-05 — C5-CONTAIN-SIZE (partial — used with C14 contain): `contain-intrinsic-size`,
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
- 2026-10-05 — C5-WRITING (done before C5-LOGICAL, which maps the inline sides by `direction`):
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
- 2026-10-05 — C5-LOGICAL (after C5-WRITING): the 52 flow-relative properties of CSS Logical 1
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
- 2026-10-05 — Phase 5 gates (with the C4G re-review: all 19 at the root except C4G-SHADOW-ORDER).
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
- 2026-10-05 — C5G-BARE-PSEUDO (gate fix, blocking): a pseudo-element with no compound before it
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
- 2026-10-05 — C5G-FLEX-SHADOW (gate fix, blocking): flex items paint exactly as inline blocks
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
- 2026-10-05 — C5G-INLINE-BLOCK-SHADOW (gate fix): an inline block in a line paints its outer
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
- 2026-10-05 — C5G-RTL-SCROLL (gate fix, blocking): an `rtl` scroll container scrolls from its right
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
- 2026-10-05 — C5G-STRING-INTRO (gate fix): rdom's input parser no longer frames PM (`ESC ^`) or SOS
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
- 2026-10-05 — C5G-LOGICAL-COST (gate fix): an inline-axis flow-relative declaration put every later
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
- 2026-10-05 — C5G-SIZING-SITES (gate fix): the remaining sizing sites go through the `Sizer` and the
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
- 2026-10-05 — C5G-INT-CLAMP-SITE (gate fix): C4G-NUMBER-RANGE clamped an integer literal to
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
- 2026-10-05 — C5G-REL-PSEUDO-INSETS (gate fix; completes C8-POS-MINMAX, row set to `done
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
- 2026-10-05 — C5G-ATOM-BOX (gate fix, closes TECH_DEBT `ATOM-BOX-1`): an inline block in a line is
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
- 2026-10-05 — C5G-PSEUDO-ONLY (gate fix): an element whose only content is its `::before` /
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
- 2026-10-05 — C5G-MIGRATION-DOCS (gate fix, docs): the content-box Breaking note (rdom-tui) now names
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
- 2026-10-05 — C5G-REEXPORTS-AND-ROOT (gate fix, API): the root re-exports `TextDirection`,
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
- 2026-10-05 — C5G-API-EDGES (gate fix, API): `From<IntrinsicSize>` for `Size` / `MinSize` / `MaxSize`
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
- 2026-10-05 — C5G-CSSOM-LOGICAL (gate fix): CSSOM reads of the inline-axis flow-relative properties
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
- 2026-10-05 — C5G-LOGICAL-IMPORTANT (gate fix; margin / padding finished by C6-MARGIN-SIDES): an `!important`
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
- 2026-10-05 — C5G-CUSTOM-SERIALIZE (gate fix): custom properties and declarations holding `var()` /
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
- 2026-10-05 — C5G-PERF-AND-TESTS (gate fix). (1) `fit-content` measuring: an intrinsic keyword box
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
- 2026-10-05 — C5G-DOCS-AND-SHOWCASE (gate fix, docs + showcase). DIVERGENCES §1's sub-cell list said
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
- 2026-10-05 — Phase 5 closed: 7 items (C5-CONTAIN-SIZE partial — used with C14-CONTAIN) + 19 gate
  fixes (batch A 9, batch B 10). Gate-fix re-review folded into the Phase 6 gate. Carried, recorded:
  margin / padding importance stays whole-field (C5G-LOGICAL-IMPORTANT; DIVERGENCES §2, the
  per-side-longhands entry — per-side margin / padding storage would fix it); `vertical-align` beyond
  `baseline` for inline blocks (C9-VERTICAL-ALIGN; C5G-ATOM-BOX laid the line-box heights it builds on).
- 2026-10-05 — C6-MARGIN-SIDES (finishes C5G-LOGICAL-IMPORTANT): `margin-*` and `padding-*` are
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
- 2026-10-05 — C6-DISPLAY-KEYWORDS: `display` takes CSS Display 3 §2's grammar —
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
- 2026-10-05 — C6-VISIBILITY: `visibility: visible | hidden | collapse` (CSS Display 3 §4),
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
- 2026-10-05 — C6-ORDER: `order: <integer>` (CSS Flexbox §5.4; a math function rounds, a value
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
- 2026-10-05 — C6-DIRECTION-REVERSE: `flex-direction: row | row-reverse | column | column-reverse`
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
- 2026-10-05 — C6-FLEX-LONGHANDS (done; was partial): the `flex-grow` (`<number [0,∞]>`,
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
- 2026-10-05 — C6-GAP: `row-gap` / `column-gap` (`normal | <length-percentage [0,∞]>`, initial
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
- 2026-10-05 — C6 file-size pass (part 1, no behaviour change): `rdom-showcase/src/nav.rs` (634
  lines, touched by C6-DIRECTION-REVERSE) — its inline test module moved to `nav_tests.rs` (195;
  `nav.rs` 441). Production files touched by part 1 now under the bar: `property_dispatch/table.rs`
  439 (C6-FLEX-LONGHANDS split), `intrinsic/mod.rs` 585 and `tui_style/builder/mod.rs` 569 — near
  it, for C6-SPLIT; the `flex/*` files are 219–403.
- 2026-10-05 — C6-FLEX-DIRECTION-INITIAL: `flex-direction`'s initial value is `row` (CSS Flexbox
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
- 2026-10-05 — C6-WRAP: `flex-wrap: nowrap | wrap | wrap-reverse` (CSS Flexbox §5.2; `FlexWrap`,
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
- 2026-10-05 — C6-JUSTIFY: `justify-content` (CSS Box Alignment 3 §5.2 grammar: `normal |
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
- 2026-10-05 — C6-ALIGN: `align-items` (`normal | stretch | <baseline-position> |
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
- 2026-10-05 — C6-ALIGN-CONTENT: `align-content` (`normal | <baseline-position> |
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
- 2026-10-05 — C6-PLACE: `justify-items` (Box Alignment 3 §6.2, with `legacy` / `legacy left |
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
- 2026-10-05 — C6-SPLIT (no behaviour change): `tui_style/builder/mod.rs` had reached 625 lines —
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
- 2026-10-05 — Phase 6 gates (with the C5G re-review: all 19 at the root; C5G-ATOM-BOX brought the
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
- 2026-10-05 — C6G-CONTENTS-STATE (AB1): a box-less element (`display: contents` / `none`, CSS
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
- 2026-10-05 — C6G-CONTENTS-BOXTREE (AN2–AN5, AN19's box-tree part): the walks that pair a box with
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
- 2026-10-05 — C6G-INLINE-FLEX-ATOM (PB1): every inline-level box whose inner display type is not
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
- 2026-10-05 — C6G-ATOM-HIT (AN6): `hit_content` resolved a point in an inline formatting context
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
- 2026-10-05 — C6G-BASELINE-ROW (AB2): a flex container's `auto` cross size comes from intrinsic
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
- 2026-10-05 — C6G-ATOM-COST (AB3): the layout pass memoized intrinsic content sizes on the Row axis
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
- 2026-10-05 — C6G-ORDER-ALLOC (AN1): `box_tree::paint_order_children` (CSS Flexbox §5.4: `order`
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
- 2026-10-05 — C6G-FLEX-COST (AN10), no layout change. The §4.5 automatic minimum was resolved in
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
- 2026-10-05 — C6G-BLOCK-ALIGN (AN11, AN12, AN13). AN11: `layout_children_aligned` laid a block
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
- 2026-10-05 — C6G-LINEBOX-API (PB2, PN13): C5G-ATOM-BOX moved `LineBox` to `render/inline/boxes.rs`
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
- 2026-10-05 — C6G-RERUN-BOUND (AN15): `cascade_element` re-ran an element's ladder with its own
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
- 2026-10-05 — C6G-ANON-FLEX-ITEMS (batch A's DIVERGENCES §3 finding; closes C6G-CONTENTS-BOXTREE's
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
- 2026-10-05 — C6G-FLEX-SPEC (AN9), six fixes in the flex algorithm, each red first. (1) CSS
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
- 2026-10-05 — C6G-FLEX-BASIS-ZERO (PN15): `parse_flex_shorthand` gave an omitted
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
- 2026-10-05 — C6G-COLLAPSE (AN7): `visibility: collapse` on a flex item, against CSS Flexbox §4.4
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
- 2026-10-05 — C6G-VISIBILITY-ONE-ANSWER (AN8): four answers to "is this element rendered and
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
- 2026-10-05 — C6G-CSSOM-EDGES (AN14). Verified first against CSSOM §6.6 `removeProperty` ("if
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
- 2026-10-05 — C6G-DECLARED-COUNT (AN16): `TuiStyle::declared_count` was a hand copy of the
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
- 2026-10-05 — C6G-MINOR (AN19's parts outside the box tree). Behavioural, each red first: (1)
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
- 2026-10-05 — C6G-SPLITS (AN18), ahead of grid; pure moves, no behaviour change, no test
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
- 2026-10-05 — C6G-BLOCKIFY (C6G-FLEX-SPEC's DIVERGENCES §3 finding): a flex item's computed
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
- 2026-10-05 — C6G-PSEUDO-FLEX-ITEMS (C6G-ANON-FLEX-ITEMS' DIVERGENCES §3 finding): a `::before` /
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
- 2026-10-05 — C6G-SCROLL-API (PN1): the scroll docs named only `rtl` (for `scrollLeft`) and
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
- 2026-10-05 — C6G-ALIGN-API (PN2, PN3, PN14). (1) The six alignment builders were `setter!`s typed
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
- 2026-10-05 — C6G-SIDE-SETTERS (PN4, PN9, PN10). Since C6-MARGIN-SIDES a side is a longhand of its
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
- 2026-10-05 — C6G-FRONTEND-API (PN11). `property_dispatch::set_from_source(name, value, text)` had
  no importance: rdom-css called `set_important` after it, a call-order protocol a second front end
  would have to rediscover (an inline-axis declaration's importance rides on its kept declaration,
  C5G-LOGICAL-IMPORTANT, so the order matters), while `set_custom_source` takes `important`.
  Decision: the same shape — `set_from_source(name, value, text: Option<&str>, important, style)`;
  the one body (`set_with_text`) takes the priority, routes a custom property to
  `set_custom_source` with it, and marks any other after declaring it (`set_important`: the
  fields' bits, or the kept declaration's flag). rdom-css's `apply_declaration` makes one call.
  `parse::token::SpannedTokens` was a 3-tuple of parallel `Vec`s; it is a struct (`tokens`,
  `positions`, `spans`; closed, a parse result whose every field is meaningful — DESIGN did not
  list it until C7G-DESIGN-TYPES, though this entry said so), built in
  place by `tokenize_spans`. Red: the lib tests failed to compile (five arguments to a
  four-argument `set_from_source`; no `tokens` / `positions` / `spans` fields). Green after:
  `set_from_source_takes_important` (a plain property, a custom one, a `var()` value, an inline-axis
  one, and a normal one left normal) and `tokenize_spans_names_its_parallel_lists`. Mutation check
  (restored and touched): the importance dropped for non-custom properties → `color` not important.
  No snapshot changed.
- 2026-10-05 — C6G-DOCS (PN5, PN6, PN7, PN12). Stale docs fixed (PN5): `Margin`'s "rdom does not
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
- 2026-10-05 — C6G-CHANGELOG (PN8, AN17, the restructure): `[Unreleased]` was about 155 KB in 279
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
- 2026-10-05 — C6G-CHANGELOG (narrative moved): the restructured CHANGELOG keeps a bullet of at most
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
- 2026-10-05 — Phase 6 closed: 14 items + 28 gate fixes (batch A 11, batch B 9, batch C 8).
  Gate-fix re-review folded into the Phase 7 gate. Carried, recorded: inline content is not aligned
  upward by `align-content` and an absolutely positioned box with both insets `auto` is not
  self-aligned (DIVERGENCES §4); a generated flex item's `aspect-ratio` (§9.2 step 3.B reads an
  element's); logical-side builders (a kept declaration, not a field); `SIZE-1` (TECH_DEBT) files,
  `walk.rs` and `layout_pass/mod.rs` added.
- 2026-10-05 — C7-GRID-CORE, prep (no behaviour change, no test changed): the item model leaves
  `flex/`, so grid shares it without reaching into flex — `flex/item.rs` and `flex/anonymous.rs`
  are `layout_pass/items/{mod,anonymous}.rs`, `FlexItem` is `items::Item` and `flex_items`
  `items::items_of` (CSS Grid 2 §6.1 builds grid items exactly as CSS Flexbox §4 builds flex
  items). Moves and renames only.
- 2026-10-05 — C7-GRID-CORE, found on the way (its first commit's two `ComputedStyle` fields tipped
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
- 2026-10-05 — C7-GRID-CORE, part 1 of 2 (the track lists; the grid container and its layout are
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
- 2026-10-05 — C7-GRID-CORE, part 2 of 2: the grid container (CSS Grid 2; sections are Grid 1's
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
- 2026-10-05 — C7-GRID-AUTO: `grid-auto-columns` / `grid-auto-rows` (CSS Grid 2 §7.6:
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
- 2026-10-05 — C7-GRID-PLACE, part 1 of 3 (the properties; the placement algorithm is part 2,
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
- 2026-10-05 — C7-GRID-PLACE, part 2 of 3: the placement algorithm (CSS Grid 2 §8). Lines
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
- 2026-10-05 — C7-GRID-PLACE, part 3 of 4: absolutely positioned boxes in a grid (CSS Grid 2 §9.1).
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
- 2026-10-05 — C7-GRID-PLACE, part 4 of 4: grid items in the stacking order (CSS Grid 2 §6.5: grid
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
- 2026-10-05 — C7-GRID-AREAS, part 1 of 2 (`grid-template-areas`; the `grid-template` / `grid`
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
- 2026-10-05 — C7-GRID-AREAS, part 2 of 2: the shorthands. `grid-template` (CSS Grid 2 §7.4: `none |
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
- 2026-10-05 — C7-GRID-ALIGN, part 1 of 3 (self-alignment; baseline alignment is part 2, content
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
- 2026-10-05 — C7-GRID-ALIGN, part 2 of 3: baseline self-alignment in grid rows (CSS Grid 2 §10.4,
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
- 2026-10-05 — C7-GRID-ALIGN, part 3 of 3: aligning the grid (CSS Grid 2 §10.5, Box Alignment 3
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
- 2026-10-05 — C7-GRID-RERESOLVE, prep (no behaviour change, no test changed): the sizing run leaves
  `grid/mod.rs` (491 lines with C7-GRID-ALIGN, and §11.1 steps 3–4 grow it) for `grid/size.rs` —
  `size_grid`, `tracks_of`, `run`, `trim` and their records; `mod.rs` keeps the axes, `Grid`, the
  shared item helpers and `layout_grid_children` (225 / 288 lines). Moves only.
- 2026-10-05 — C7-GRID-RERESOLVE (part 1 follow-up; row added to the Phase 7 table with the other two):
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
- 2026-10-05 — C7-SUBGRID, part 1 of 2 (the value; the layout is part 2): `grid-template-columns` /
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
- 2026-10-05 — C7-SUBGRID, part 2 of 2: the subgrid layout (CSS Grid 2 §9, §9.5). Structure — the
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
- 2026-10-05 — C7-ABSPOS-PADDING-EDGE (part 1 follow-up): checked first whether the grid fallback alone
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
- 2026-10-05 — C7-SPLIT (no behaviour change, no test changed): the file-size pass over
  `layout_pass/grid/*` and every production file Phase 7 touched. The grid directory was kept under
  500 lines a file as it grew (`size.rs` out of `mod.rs` for C7-GRID-RERESOLVE, `places.rs` and
  `subgrid.rs` for C7-SUBGRID, `tracks_of` into `track.rs`); largest now `subgrid.rs` 484,
  `placement.rs` 471. Two touched files were past the bar: `property_dispatch/table.rs` 566 (the grid
  fields and shorthands) → `table.rs` 337 + `fields.rs` 237 (`fields_of` and `all_fields`, the
  property → field map; `table` re-exports `fields_of`, so its callers are unchanged), and
  `layout_pass/mod.rs` 533 → 482 + `flow.rs` 63 (`flow_axis` / `resolve_gap` / `gap_along`, the cut
  `SIZE-1` named; re-exported). Moves only. TECH_DEBT `SIZE-1` drops both and lists what is left (none
  of it touched by Phase 7).
- 2026-10-05 — Phase 7 items complete (C7-GRID-CORE, -AUTO, -PLACE, -AREAS, -ALIGN, C7-SUBGRID and the
  part 1 follow-ups C7-GRID-RERESOLVE, C7-ABSPOS-PADDING-EDGE, C7-SPLIT); the phase's architect and API
  gates are next. CSS-COVERAGE §3.9: 9 of 10 rows *Supported* (`masonry` / `grid-lanes` the decided
  exclusion). ACID.md: grid is now usable for a tile — proposed as a new tile 18 (Grid layout), not an
  extension of tile 7 (the reasons are in its coverage note).
- 2026-10-05 — Phase 7 gates (with the C6G re-review: all 28 at the root). Architect: 1 blocking —
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
- 2026-10-05 — C7G-LINES-SHIFT (architect B1): a grid's lines (`TuiExt::grid_lines`, CSS Grid 2
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
- 2026-10-05 — C7G-INITIAL-ALLOC (architect N1): `ComputedStyle::initial()` — the start of every
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
- 2026-10-05 — C7G-TRACK-VALIDITY (architect N2, API N1): layout reads only valid grid values. Where
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
- 2026-10-05 — C7G-MEMO-PURITY (architect N3): the per-pass intrinsic memo (`intrinsic::memo`)
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
- 2026-10-05 — C7G-SUBGRID-COST (architect N4). (1) The subgrid size memo's key was
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
- 2026-10-05 — C7G-FOCUS-FIXUP (architect N6, API upgrade-guide finding): `draw_if_dirty` ran the
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
- 2026-10-05 — C7G-INLINE-ATOM-MAX (architect N7): max-content inline sizes summed the text of an
  inline subtree (`intrinsic/inline.rs::inline_content_width`), atoms' text included, and a block
  container whose inline content is text and atoms only — not an IFC block by `is_ifc_block`, so laid
  out as an anonymous block box (CSS 2.1 §9.2.1.1) — was measured as its element children stacked and
  its text runs apart (`children.rs`). Decision, at the root: intrinsic inline sizes come from the
  packer layout uses (CSS Sizing 3 §5.1: the widest line at no soft wrap / at every one), as
  min-content already did: `inline::widest_line` (an IFC block) and `inline::widest_run_line` (one
  anonymous block box's run, the partition `block::inline_runs` shares with `layout_block_children`),
  each with a `LinePacker::measuring` packer whose atoms are their own max-content width with
  percentages against no basis (§5.2.1) and whose atom rows are not measured. The block container's
  Column estimate packs the same runs at its content width (its text was one unwrapped row a run, its
  atoms stacked) — needed, not only truer: with the wider, right Row widths the old estimate measured
  each nested atom at a width layout never gives it, and `nested_inline_blocks_measure_each_subtree_once`
  went quadratic (3, 6, 10, 15, 45, 91 Column walks at 1–12 levels); packing as layout does, its keys
  coincide with layout's and the pin holds unchanged. The sum walk and `pseudo_beside_inline` are gone
  (the pack holds the static pseudo-elements). Split: `inline/mod.rs` 563 → 365 + `feed.rs` (the box
  tree fed to the packer, now shared by `compute_inline_layout`, `pack_run` and `measure.rs`); TECH_DEBT
  `SIZE-1` updated. Red: `css_phase7/container.rs::an_atoms_box_counts_in_its_lines_max_content` —
  `x ` beside an `inline-grid` of `5 5`, a `width: max-content` paragraph `(10, 2)` for `(12, 1)`
  (checked against the unchanged source by stashing it); likewise `inline-flex` / `inline-block`
  `width: 10`, and `x y ` in an IFC 14 wide. Green after. Mutation: the IFC path measured at
  min-content → `(10, 2)` for `(14, 1)` (reverted, touched). One snapshot changed, justified:
  `rdom-showcase`'s `ua_chrome.snap` — its "Buttons" `<section>` (an `<h3>` and three `<button>`s, a
  flex item of the root, sized by this estimate) was 2 rows too tall, the old Column estimate stacking
  the three inline-block buttons; it is now its `<h3>` and one line, so the three blank rows after the
  buttons are one, as between every other section (glyphs moved up two rows, no cell changed
  otherwise). No other test changed.
- 2026-10-05 — C7G-STACKING-ONE (architect N8): `is_layered` folded z-indexed flex and grid items in
  (CSS Flexbox §5.4, CSS Grid 2 §6.5) but `creates_stacking_context` did not, so each caller patched the
  gap — `collect_layers` with `|| !is_positioned(c)`, and the paint and hit walks by asking
  `is_layered` first. Decision: one rule, `is_z_indexed_item`, that both read; `creates_stacking_context`
  now takes `(dom, parent, c)` like `is_layered` and answers for the items, so every layered box is
  positioned or a context, and the patch in `collect_layers` is gone. All users grepped and updated:
  `render/stacking.rs` (`collect_layers` twice, `atom_shadows_in`), `paint_pass/stacking_walk.rs::
  paint_in_flow`, `hit_test/descend.rs::hit_in_flow_element`. Red: `render/stacking_tests.rs` (new) —
  `a_z_indexed_item_is_layered_and_a_stacking_context` did not compile against the one-argument
  predicate (it could not know the item rule); green after, with `the_rule_is_the_items` pinning the
  block-flow, positioned and opacity answers. No behaviour change: no other test and no snapshot
  changed.
- 2026-10-05 — C7G-MINOR (architect N9 and the gate's two ACCEPT records). (1) `tree::clear_box_state`
  reset every box value but the new `grid_lines`, against C6G-CONTENTS-STATE's one reset; it now
  clears them. Red: `layout_pass::tests::a_grid_turned_contents_keeps_no_grid_lines` (the lines kept);
  green after (not observable outside: a box-less element is never a containing block). (2) The
  restyle guard of C6G-BLOCKIFY recascaded a `display: contents` element's whole subtree whenever it
  was restyled, its style unchanged or not. Decision: keep the subtree when the style is unchanged
  and no element between it and its box parent, nor the box parent, changed in this restyle the
  answer its children's blockification reads (`children_are_items`: their `contents`-ness or their
  flex / grid flow) — the walk is top-down, so such an element was restyled first and noted in the
  pass's `Scratch::items_changed` (empty unless one did). A stored per-element bit was tried first and
  dropped: it grew `TuiExt` past its size tripwire (448 B for 440), a cost every element pays for a
  restyle-only saving. Red: `cascade::cost_tests::
  an_unchanged_contents_element_keeps_its_subtree_in_a_restyle` — 101 nodes visited for 1; green
  after. Mutation: the guard never firing → `apply_tests::
  a_restyle_unblockifies_the_children_of_a_contents_item` fails (`Block`; reverted, touched). (3) `blockify::children_are_items` stopped at a `Fragment`,
  `stacking::is_item_of` walked through one. A `Fragment` is only ever a tree's root (rdom-core
  unwraps one on insertion), so the two never disagreed on a reachable tree; made one answer by
  construction: `is_item_of` is `children_are_items` (re-exported from `cascade`). Not behavioural, no
  test. (4) Step 3 of §11.5 rescanned all items for each span length up to the widest (O(widest ×
  items); `span 10000` is reachable): the spanning items are sorted by span once and taken in
  `chunk_by` groups. Not behavioural; the sizing tests pin the order. (5) `auto_repetitions` counted a
  `minmax(5, 2)` track by its max, 2: a definite max is now floored by a definite min (a growth limit
  is never below its base size, §11.4). Red: `css_phase7/tracks.rs::
  a_max_below_the_min_counts_as_the_min_in_an_auto_repetition` — three columns of 5 in a 12-wide grid,
  the third at x 10 (checked against the unchanged `template.rs` by stashing it); green after. (6)
  DIVERGENCES §2 records the accepted exception: grid re-resolution detects aspect-ratio transfers
  only, a `column wrap` flex item's height-dependent min-content width not; and, found with
  C7G-MEMO-PURITY, that a subgrid stays out of its parent's baseline groups on its non-subgridded axis
  too. No snapshot changed.
- 2026-10-05 — C7G-README-GRID (API B1): the doctested rdom-tui README had no grid example, and two
  docs still said grid did not exist. The README gains a "Grid layout" section: what grid supports,
  and a complete page layout built from CSS (`rdom_css::from_css_strict`) — named areas, `auto 1fr
  auto` rows, a two-value `gap`, a `main` that is itself a grid of `repeat(auto-fill, minmax(4, 1fr))`
  cards, one `grid-column: 1 / -1` — cascaded, laid out and painted into a 20 × 5 `Buffer` whose rows
  it asserts, so `cargo test --workspace` checks the paint. The Rust-builder form joins it with
  C7G-GRID-SETTERS, once `grid-area: head` is one call. Stale lines: CSS-COVERAGE's `fr` row
  (`grid's fr does not exist`) is *Supported* — grid's `<flex>` and the rdom flex weight — with §1's
  counts moved (§3.3 16 / 1, total 144 / 27, 117 rows Partial / Missing); DIVERGENCES' masonry entry
  no longer calls Grid 1 / 2 "scheduled". Check: the doctest's first build failed to compile — its
  `main` returned `Box<dyn Error>`, and `rdom_css::ParseError` does not implement `std::error::Error`
  (the example returns `ParseError` instead; the missing impl is a separate API finding, reported, not
  fixed here); then green, its painted rows as written. No test expectation or snapshot changed.
- 2026-10-05 — C7G-DESIGN-TYPES (API B2): DESIGN's `#[non_exhaustive]` section classified none of
  the Phase 7 public types. Now: the grid values `TrackBreadth`, `TrackSize`, `RepeatCount`,
  `TrackRepeat`, `TrackListItem`, `TrackList`, `LineNameItem`, `LineNameList`, `GridLine`,
  `GridAutoFlow` and `NamedArea` join the closed CSS value types (a layout must size or place each
  whole); `GridTemplateShorthand` / `GridShorthand` join the shorthand records beside
  `FlexShorthand`, with `SpannedTokens`; `GridTemplateAreas` (only `new` builds one, rectangular by
  construction) and `ScrollRange` (accessors and `new`) are sealed by private fields. Batch A added no
  public type (`TrackSize::AUTO_LIST` is a const). Decision on `GridTemplate`, the one
  `#[non_exhaustive]` CSS value: kept open — a consumer reading it through `tracks()` / `subgrid()` /
  `is_valid()` needs no arm for CSS Grid 3's masonry axis — and the silent-`none` risk is closed by a
  rule instead: rdom-tui never matches it (it reads the accessors only), and every match on it in
  rdom-style names each variant, under `#[deny(clippy::wildcard_enum_match_arm)]` on the accessors'
  `impl`, `serialize_grid_template`, `serialize_grid_template_shorthand` and `absolutize_template`,
  so a new variant fails to compile at each read and the accessors' answer is decided there. The rule
  is in DESIGN and on the type's doc. Red: the lint added first, `cargo clippy` failed at the three
  wildcard arms (`subgrid()`, `absolutize_template`, the shorthand serializer); green after naming
  the variants. Not behavioural (the arms return what the wildcard did). The C6G-FRONTEND-API entry,
  which said DESIGN listed `SpannedTokens` as closed, is corrected in place. No test or snapshot
  changed.
- 2026-10-05 — C7G-GRID-SETTERS (API N2). (1) Every grid node setter now says what it does with a
  value outside the grammar: `set_grid_template_columns` / `-rows` and `set_grid_auto_*` refuse it
  (a debug build panics, a release build keeps the earlier declaration, as CSSOM ignores an invalid
  `setProperty`) after the builders' clamps (`fr` / `%`, `span 0`, an empty list as `none`);
  `set_grid_row` / `-column` / `-area` refuse line 0 and a name `span` / `auto`;
  `set_grid_template_areas` and `set_grid_auto_flow` are never refused (valid by construction); and
  `set_justify_content`, whose doc the five other alignment setters point to, names the debug panic.
  (2) New node setters: `set_grid()` / `set_inline_grid()` (`display: grid` / `inline-grid`, through
  `TuiStyle::grid` / `inline_grid`; any other `display` stays a CSSOM write, which `set_grid`'s doc
  names — a general display setter would need a type for the whole `display` value, which rdom keeps
  as three fields), `set_grid_area(row_start, column_start, row_end, column_end)` and
  `set_grid_area_named(name)`. (3) New builders: `TuiStyle::place_content` / `place_items` /
  `place_self(align, justify)` (CSS Box Alignment 3 §5.5, §6.4, §6.5; each half checked by its
  longhand) and `TuiStyle::grid_area_named(name)` — `grid-area: head` in one call (§8.4: a lone
  `<custom-ident>` copied to all four lines); the four-line `grid_area` is unchanged. Not taken from
  the gate's list: `From<&str>` / `From<i32>` for `GridLine` and `TrackSize::auto()` / `min_content()`
  (the `TrackSize::AUTO` const and `TrackBreadth` variants cover them). The rdom-tui README's grid
  section gains the same page built from the builders, rect-checked. Red: the new tests did not
  compile (no `set_grid`, `set_grid_area`, `set_grid_area_named`, `set_inline_grid`, `place_*`,
  `grid_area_named`). Green after: `tui_style::tests::place_builders_set_both_longhands` /
  `grid_area_named_sets_all_four_lines`, and `css_phase7/setters.rs` (new) —
  `node_setters_drive_grid_layout` lays out a page built only through node setters (an area name,
  the four-line form with `nth_named(1, "main-start")`, `grid-row` / `grid-column`, `gap`; the rows
  1 / 3 / 1 and columns 6 / 13 of §11.7) and `display_grid_node_setters_compute_their_display`
  (`inline-grid` blockified to `grid` as a grid item, CSS Display 3 §2.7). No test expectation or
  snapshot changed.
- 2026-10-05 — C7G-UPGRADE-GUIDE (API N3). The CHANGELOG's "Upgrading from 0.5": (a) a silent-change
  entry for `focus()` refusing a hidden element (C6G-VISIBILITY-ONE-ANSWER), with the "open panel,
  focus input" case and a workaround that was checked, not assumed: the obvious one,
  `request_animation_frame`, does not work — the event loop runs a handler's animation frame
  callbacks in the same turn, before the frame cascades (`pump_scheduler` then `draw_if_dirty`) — so
  the guide gives an animation frame callback requested from inside one, which runs after the
  cascade. `css_phase6/visibility_answers.rs::focusing_a_just_shown_input_waits_for_the_next_frame`
  pins all three through `handle_event` (Enter on the opening button) and `advance`: in the handler
  and in one frame callback the focus stays on the button, in two it moves to the input. (A first
  draft drove the click through `AppHandle::inject`, which runs after the turn's timers, and one frame
  callback then took the focus — not what a key or mouse handler sees; the test uses the input path.)
  The `tab_index` doc lists the rendering condition, and DIVERGENCES' "Focusability reads the last
  cascade's styles" the frame-callback detail. (b) The padding-edge containing block moves from #16
  to #4 by impact, naming the title-on-the-border idiom and its migration — less the border width
  from each inset, `top: 0; left: 2` → `top: -1; left: 1`; `css_phase7/abspos.rs::
  a_title_on_the_border_moves_by_the_border_width` paints both (inside the panel, then on its top
  border over the glyphs). (d) The API table's rows for items that never shipped in 0.5 — checked
  with `git grep` at `v0.5.0`: `lookup_in` / `substitute` / `resolve_custom_properties` (its
  `ContentContext for HashMap<String, String>` half did ship and stays), `PropertyRegistration` /
  `PropertySyntax`, `parse_max_size`, `.justify_content(…into())` (no alignment builder existed),
  `set_from_source`, `SpannedTokens`, `App::register_property`, and `add_rule_in_layer`, which the
  gate did not list — move to a "Changes to APIs added after 0.5" table for git `main` consumers;
  the compile-break list drops them. (e) `TuiNodeExt::direction()` returned `Row` for `row-reverse`
  while documented as `flex-direction`. Decision: keep it, documented as the axis — the half
  `set_direction(Direction)` writes, so the setter / reader pairs stay symmetric and no 0.5 caller
  breaks — and add `flex_direction() -> Option<FlexDirection>`, the whole value, beside
  `set_flex_direction`. Red: `node::tests::flex_direction_reads_back_the_reversed_forms` did not
  compile (no `flex_direction`); green after, all four values and the axis for each. (f) The eight
  Phase 7 bullets past ~340 characters are trimmed (the gate's six and two more at 347 / 414). No
  test expectation or snapshot changed.
- 2026-10-05 — C7G-DOCS-TESTS (API N4, N5, N6, the ACID tile 18 additions). Stale docs (N4):
  DIVERGENCES §3's empty "Flexbox and box alignment" and "Grid" headings are gone (nothing of either
  is left unshipped; `masonry` is a §2 decision); the static-position entry names grid beside flex
  (CSS Grid 2 §10.2 places the child as the sole item of an area that is the content box; rdom takes
  its start corner in both, which `grid/mod.rs` already cites); CSS-COVERAGE's headline gaps are
  `line-height`, `text-align` and `@media` (grid placement is shipped), and its `row-gap` /
  `column-gap` row names the grid gutters instead of "ready for grid"; `Display::Block`'s doc says a
  flex or grid item. Tests (N5), `css_phase7/everyday.rs` (new): `1fr 1fr` holds a 10-cell word's
  column at 10 and leaves the other its 1 cell, where `minmax(0, 1fr)` and an `overflow: auto` item
  share 5 / 5 (§6.6, §7.2.4, §11.7); `place-items: center` centers `ab` at (4, 2) in a 10 × 5 area;
  the `auto 1fr auto` page with `gap: 1` is 5 rows under an `auto` height and fills `min-height: 9`
  with a 5-row `fr` row. These pin shipped behaviour, so they were green on first run; each pair of
  cases differs in its result, so a regression in either path fails one. Public track read (N6):
  `TuiAccessors::grid_tracks() -> Option<GridTracks>` — the analogue of §7.2.6's resolved value —
  each column and row a half-open cell range from the content box's left / top edge, before the
  container's scroll offset, in grid order (an `rtl` grid's first column the rightmost range),
  implicit tracks included; built by `GridLines::used_tracks` from the content-box-relative edges of
  C7G-LINES-SHIFT, so it needs no second copy and stays true after `shift_subtree`; `None` for a box
  that is not a laid-out grid (each dispatch clears the lines). `GridTracks` is sealed by private
  fields with `new` / `columns()` / `rows()` (DESIGN). Red: `css_phase7/used_tracks.rs` (new) did not
  compile (no `grid_tracks`); green after — a padded grid with gutters and an implicit row, an `rtl`
  grid, and a block (`None`). ACID.md tile 18 gains the gate's five cases (`1fr` against
  `minmax(0, 1fr)`, two items in one cell by `z-index`, `inline-grid` in text, `rtl`, `1 / -1`) with
  their expected results. No test expectation or snapshot changed.
- 2026-10-05 — C7G-SIZES (architect N10). TECH_DEBT `SIZE-1` was wrong against the tree: it said
  `render/inline/mod.rs` was untouched by Phase 7 (dcbe040 touched it; C7G-INLINE-ATOM-MAX has since
  split it), left out files Phase 7 grew past 500 (`intrinsic/mod.rs`, `layout/sizing.rs`,
  `layout/keywords.rs`, `cascade/walk.rs`), and gave `grid/subgrid.rs` as 484 (491 at the gate, 520
  after batch A). Measured, not estimated: every production `.rs` file over 500 lines, with one rule
  for "production" — a `crates/*/src` file's lines outside its inline `#[cfg(test)]` modules, test
  files (`tests.rs`, `*_tests.rs`, `test_*.rs`) not counted (CLAUDE.md: test files may run long), the
  generated `entities.rs` table exempt. By that rule three files were past 575, all split by pure
  moves (no code changed, callers unchanged through re-exports or the same paths):
  `style/cascade/walk.rs` 590 → 526 + `root_vars.rs` 61 (`merge_root_vars`, the `:root`
  custom-property seed; `walk` re-exports it); `rdom-core/src/dom.rs` 665 → 443 +
  `interaction_state.rs` 234 (the hover / focus / active / `:focus-visible` / pointer capture /
  selection getters and setters, an `impl Dom` over the same `pub(crate)` fields); and
  `rdom-parser/src/parser.rs` 800 → `parser/mod.rs` 522 + `parser/attr.rs` 137 (attribute names and
  values, a child module so it keeps the parser's private state) + `char_refs.rs` 158 (character
  references; the scanner tests stay in `parser`). `wc -l` alone would have flagged ten more rdom-core
  / rdom-tui files whose size is an inline test module; they are listed with both numbers. `SIZE-1`
  now lists every file between 500 and 600. Mechanical check: `rdom-showcase/tests/integration/
  file_sizes.rs` (in the one unpublished crate, as it reads every sibling's `src/`) fails
  `cargo test --workspace` past 600 lines, its counting rule tested on its own cases; the rule is
  recorded in CLAUDE.md §Architecture Hygiene. Red: the same rule run before the splits counted
  `parser.rs` 800 and `dom.rs` 665 past 600 (and `walk.rs` 590 past the 575 split bar); after them the
  check is green, and with its limit lowered to 560 it fails on exactly the three files at 561–564
  (`mouse/mod.rs`, `dirty_tracker.rs`, `event_detail.rs`; restored). No test expectation or snapshot
  changed.
- 2026-10-05 — Phase 7 closed: both gates run, 15 gate fixes (`C7G-*`: batch A — LINES-SHIFT,
  INITIAL-ALLOC, TRACK-VALIDITY, MEMO-PURITY, SUBGRID-COST, FOCUS-FIXUP, INLINE-ATOM-MAX,
  STACKING-ONE, MINOR; batch B — README-GRID, DESIGN-TYPES, GRID-SETTERS, UPGRADE-GUIDE, DOCS-TESTS,
  SIZES). Carried: C8-CB-COMPLETE (the padding-edge containing block's remaining cases) and
  C12-FOCUS-FLUSH (`focus()` flushing style, TECH_DEBT `FOCUS-FLUSH-1`), both scheduled. The `C7G-*`
  re-review rides with the Phase 8 gate.
- 2026-10-05 — C8-PARSE-ERROR (found by C7G-README-GRID). Audit of every public error type in the
  workspace (`pub struct|enum *Error`): `DomError`, rdom-core's selector `ParseError`, rdom-parser's
  `ParseError`, `StyleError`, `PropertySyntaxError`, `RegisterPropertyError` and `SetPropertyError`
  implemented `std::error::Error`; three did not — `rdom_css::ParseError` (no `Display` either),
  `rdom_style::parse::token::TokenizerError` and `property_dispatch::DispatchError` (re-exported as
  `rdom_tui::cssom::DispatchError`). Each now has `Display` (`line:column: problem` for the two with
  a position, as rdom-parser's does; `ParseErrorKind` / `TokenizerErrorKind` display the problem) and
  `Error`. `SetPropertyError` wrapped a `DispatchError` / `DomError` without reporting it: its
  `source()` now returns the inner error, and its `Display` uses theirs instead of `{:?}`. The
  rdom-tui README's grid example returns `Box<dyn std::error::Error>`, as a consumer's `main` would.
  Red: `strict.rs::a_parse_error_is_a_std_error_with_a_position`, `token_tests::
  a_tokenizer_error_is_a_std_error`, `property_dispatch::tests::a_dispatch_error_is_a_std_error` and
  `declaration::tests::write::set_property_error_sources_its_inner_error` did not compile (no
  `Display` / `Error`); green after. No test expectation or snapshot changed.
- 2026-10-05 — C8-CB-COMPLETE (completing C7-ABSPOS-PADDING-EDGE; CSS 2.1 §10.1, CSS Position 3
  §2.1, CSS Overflow 3 §2.2 / §3, CSS Grid 2 §9.1). One walk: `positioning/containing.rs::
  absolute_containing_block(dom, from, style, viewport)` — `fixed` the viewport; `absolute` the
  nearest ancestor whose `position` is not `static`, from the box parent for an element and from
  the host for a `::before` / `::after` (a pseudo-element is its host's child), the viewport on a
  miss. `positioned_pseudos::resolve_containing_block`'s copy of the walk is gone. The ancestor
  gives (1) its padding box less its scrollbar gutters — the column on the bar's side, the bottom
  row — reserved under the rule scrollbar paint uses (`gutter_axes` with an `auto` axis forced when
  it overflows its scrollport); (2) within it, for a grid container, the grid area the box's own
  placement properties name (`grid::abspos_area` now takes the box's `ComputedStyle`, not a
  `NodeId`, so a pseudo-element's `grid-row` / `grid-column` place it); (3) for a scroll container,
  that rect moved by its `scroll_x` / `scroll_y`, so a contained box scrolls with the content as in
  every browser. `sticky` now establishes the containing block (the predicate was `relative |
  absolute | fixed` in both walks). Found while testing `fixed`: `sticky::place_one` shifted the
  stuck box's whole subtree after phase 2 had placed its positioned descendants, so a `fixed`
  descendant moved with it; `tree::shift_subtree` (only `sticky` uses it) now keeps a `fixed`
  subtree in place, while `shift_content` (`align-content`, phase 1) still moves everything (phase
  2 places `fixed` boxes again from the moved static positions). Red: `css_phase8/
  containing_block.rs` (new) — a sticky ancestor's `top: 0; left: 0` child at `(0, 0)` for `(3,
  2)`, the same for a sticky host's and a sticky ancestor's `::after`; `inset: 0` in an
  `overflow-y: scroll` box 10 wide for 9; `top: 4` in a positioned scroller scrolled by 2 at row 4
  for 2; a `fixed` box inside a stuck sticky at row 9 for 6; a positioned grid's `::after` with
  `grid-column: 2 / 4; grid-row: 2` the whole padding box `(0, 0, 20, 3)` for `(2, 1, 7, 2)`. The
  pin that a scroller below the containing block does not move the box was green before and after.
  Mutation (each restored and touched): `sticky` excluded from the predicate → the two sticky
  tests; no scroll offset and no gutter → the scroll and gutter tests. Decided, not changed:
  `scrollbar-gutter: stable` under `overflow: auto` reserves the horizontal row too (CSS Overflow 3
  limits the property to the inline-axis bar) — C8-SCROLLBAR's; the test reads `overflow-y`.
  Found, then left out of C8-OVERFLOW-TEXT (TECH_DEBT `ABSPOS-OVERFLOW-1`): an absolutely positioned box does not count in its
  containing scroll container's scrollable overflow (§2.2), so it cannot be scrolled to past the
  in-flow content. No other test expectation and no snapshot changed.
- 2026-10-05 — C8-Z-INDEX (CSS 2.1 §9.9.1, CSS Values 4 §5.1 / §3.2). `ZIndex::Value` holds an `i32`
  (was `i16`, a literal past ±32 767 dropped as invalid): `parse_z_index` reads any `<integer>` —
  literal or math function — and clamps a value past `i32` to it, as engines do (the tokenizer
  already clamps a literal past `i32`, C4G-NUMBER-RANGE); the two branches (a literal checked, a
  math result clamped) are one. The stacking layers' key (`LayerEntry::z`) and the positioned
  pseudo-elements' sort key are `i32`; both sort by `(z, tree order)`, an integer comparison, so
  paint and hit-test order stay total and deterministic. A `z-index` transition interpolates through
  the existing `lerp_i32` (`f64`, which holds every `i32` exactly, a saturating cast) instead of an
  `f32` that would have rounded large levels. Red: `positioning.rs::
  z_index_takes_the_full_integer_range` and `animation::tests::
  z_index_interpolates_over_the_full_integer_range` did not compile (`40_000` and `100_000` out of
  range for `i16`); `css_phase8/z_index.rs` — `z-index: 40000` rejected by the strict parse
  (`ExpectedToken("valid declaration")`); green after, `40000` over `39999`, `i32::MAX` over
  `i32::MIN`, a tie in tree order. Changed expectation, justified:
  `calc::semantics_tests::integer_properties_take_math_functions` — `calc(infinity)` clamps to
  `i32::MAX` (was `i16::MAX`), the range this item widens. DIVERGENCES' `i16` entry and its §3 line
  are gone; CSS-COVERAGE's `z-index` row is *Supported* (§3.10 5 / 0, total 145 / 26, 116 rows
  Partial / Missing). No snapshot changed.
- 2026-10-05 — C8-OVERFLOW-CLIP (CSS Overflow 3 §3.1–§3.3). rdom-style: `Overflow::Clip`, moved with
  `Overflow` into `layout/overflow.rs` beside the new `OverflowClipMargin { visual_box, margin }`
  (`<visual-box> || <length [0,∞]>`, whole cells, the box `padding-box` when omitted;
  `keywords.rs` was 511 lines and Phase 8 adds more overflow types). `overflow` takes one or two
  keywords (`parse_overflow_shorthand`) and serializes `<x> <y>` when they differ;
  `overflow-block` / `overflow-inline` are block-axis aliases in `logical.rs` (`overflow-y` /
  `overflow-x`, one storage, as `inline-size` is). Two predicates replace the `!= Visible` tests:
  `ComputedStyle::is_scroll_container` (`hidden` / `scroll` / `auto` on an axis) and
  `clips_overflow`; `normalize_overflow` is §3.1's computed value (beside a scrolling axis
  `visible` → `auto`, `clip` → `hidden`), run in the element and pseudo-element cascades before
  the BFC rule. rdom-tui, every reader audited (19 sites): a scroll container is what the BFC
  rule (`clip` forms none), the flex / grid automatic minimum, baselines (§9.1 of Box Alignment),
  `align-content`'s overflow, sticky's scrollport, scroll-into-view, the scrollbar geometry and
  the containing block's scroll offset read; `record_scroll_content_size` records an extent and
  `clamp_scroll_offset` keeps an offset only for one — any other box's are 0, so `clip` "forbids
  all scrolling". One clip rule, `layout_pass/clip_edge.rs::ClipEdges` (a scroll container's
  padding box; a `clip` axis's `overflow-clip-margin` box outset by its margin; nothing on a
  `visible` axis), read by `stacking::children_clip` (paint and hit-testing), the tree guides
  (their copy of the rule is gone) and the scrollable-overflow walk, which cuts a `clip`
  descendant's content to its edges per axis instead of dropping all of it. The ladder's note that
  skipped §3.1's rule ("an `auto` axis always reserves a gutter") had gone stale — `auto` reserves
  only on overflow — except for `scrollbar-gutter: stable`, which reserved the horizontal row too:
  it now reserves the vertical bar's gutter only (§3.3: the inline-start / inline-end edges), and
  `layout_node`'s second pass always settles an `auto` horizontal bar. Found by the rule: scrollbar
  hit-testing and thumb drags took the corner cell from the vertical track whenever
  `overflow-x` was `auto`, paint only while that bar showed — `bars_shown` is now the one answer
  for paint, hit, drag and the tree guides. Red: `property_dispatch/overflow_tests.rs` (4) and
  `css_phase8/overflow_clip.rs` did not compile (no `Overflow::Clip`, `OverflowClipMargin`,
  `overflow_clip_margin`); green after — the computed pairs, the logical longhands, per-axis
  clipping (`overflow-x: clip` paints the row below the box, cut at 3), the margin (one and two
  cells; none for `hidden`; `content-box` inside the right padding), a refused and a dropped
  scroll offset, the collapsing margin (2 under `clip`, 0 under `hidden`). Added after (green,
  mutation-checked): a `clip` descendant's 20-wide, 5-tall content counts `(4, 5)` in its
  scroller's extent (`(4, 1)` with the old "a clipping box contributes its border box").
  Mutation (each restored and touched): no normalization and `clip` forming a BFC → the computed
  and formatting-context tests; no clip margin and both axes clipping → the margin and per-axis
  tests. Changed expectations, justified: `cross_axis_independence_v1` pinned the skipped rule — now
  `a_visible_axis_beside_a_scrolling_one_computes_to_auto`; `overflow_auto_with_scrollbar_gutter_
  stable_reserves_gutter` expected the bottom row (5 rows for 4); `scroll_y_offsets_children_
  negative`, `negative_layout_rect_partially_visible` and the showcase's
  `sidebar_scroll_end_regression` scrolled boxes that are not scroll containers (the first two
  now give `c` `overflow: hidden` and a height, the third scrolls `.sidebar-tree`, the sidebar's
  real scroller); `apply_tests`, `canonical_values` and `every_property_has_important_setter` list
  the new property. The scrollbar runtime tests that failed under the computed rule (track, drag,
  `rtl` and `column-reverse` thumbs, the no-cascade press) pass
  unchanged with `bars_shown`. No snapshot changed. DIVERGENCES: `overflow-clip-margin` in whole
  cells, a viewport-relative length rejected. CSS-COVERAGE §3.11: 5 / 1 / 8 (total 149 / 24 / 88,
  112 Partial / Missing).
- 2026-10-05 — C8-OVERFLOW-TEXT (from ACID's gap list; CSS Overflow 3 §2.2). The scrollable overflow
  walk (`scroll_extent::extend_scrollable_overflow`) counted a descendant's border box and its
  anonymous boxes' border boxes, never its line boxes, so text overflowing a non-clipping box — a
  `nowrap` line wider than it, lines below a fixed height — could not be scrolled to. Now each line
  box of a descendant's `inline_layout` (at its content box) and of every anonymous block box (at
  its `rect`), the scroll container's own anonymous boxes included, counts as the rect from its
  leftmost to its rightmost fragment or generated run over its rows (`line_rects`), cut by the
  overflow clip edges of any `clip` box between (C8-OVERFLOW-CLIP's `ClipEdges`); a descendant
  scroll container still contributes its border box alone. Found while testing: that made the
  text scrollable to but not visible — `inline_paint::paint_inline_layout` cut every line at its
  own content box on both axes (a "band" of the box's rows, a right edge at its content width, and
  for a scroll container's own lines a right edge that moved left with `scroll_x`), so text past a
  box never painted whatever its `overflow`. CSS Overflow 3 §3.1: `visible` content "is not
  clipped". Lines and atoms now paint inside `clip` alone — the caller's `children_clip`, which a
  clipping box narrows to its padding box or overflow clip edge — and the band is gone. Red:
  `css_phase8/overflow_text.rs` — a 10-cell `nowrap` line in a 4-wide box painted `abcd` for
  `abcdef` (up to the port's clip), `pre` lines below a 1-row box not at all; green after, the
  scrolled port showing `efghij`. Red (extent): `css_phase8/overflow_text.rs` —
  a 10-cell `nowrap` line in a 4-wide box gave the port `scrollWidth` 4 for 10, three `pre` lines in
  a 1-row box `scrollHeight` 1 for 3, `scrollLeft` 4 clamped to 0; the scroller's own anonymous
  box's line 6 for 10 (checked by stashing the source after green). Added after (green,
  mutation-checked): a descendant's anonymous block box's line (`4` for `10` with its lines left
  out); the pin that a descendant scroll container keeps its lines was green before and after.
  Changed expectations, justified: `inline_flow.rs::ifc_clips_overflow_at_content_width` and
  `fixed_height_ifc_clips_overflowing_lines` pinned the cut for boxes with `overflow: visible`;
  they now set `overflow: hidden`, which is what clips in CSS, and keep their claims. Two
  snapshots changed, glyphs only (backgrounds identical): `selectable_text.snap` — the code line's
  `;` past its box now paints; `tab_form.snap` — the hint paragraph's wrapped second line
  `Ctrl-C: quit` paints in the blank row under its 1-row box, as in a browser.
  `rdom-tui` README's "Fixed height clips overflowing lines" is corrected. Also `rdom-showcase`'s `chrome_layout_contract::
  source_disclosure_when_open_has_fixed_height_12` — the open Source panel (`overflow: auto`) now
  shows a horizontal scrollbar for the `<pre>` lines wider than it, as a browser does, so its
  content area is 10 rows; the test keeps its box-model claim (11 = outer − border-top) less the
  bar's row when the content is wider (the snapshots show the panel closed).
  Not done here, recorded as TECH_DEBT `ABSPOS-OVERFLOW-1` and DIVERGENCES §3: an absolutely
  positioned box in its scroll container's scrollable overflow — the extent and the clamp are
  phase-1 work and positioned boxes are placed in phase 2 (C8-CB-COMPLETE's entry had scheduled it
  here). ACID's overflow gap is struck through.
- 2026-10-05 — C8-TEXT-OVERFLOW (CSS Overflow 4 §3, checked against the editor's draft). rdom-style:
  `TextOverflow` (sealed: `one(end)` / `two(left, right)`, `values()`, `line_sides(rtl)`,
  `is_clip()`) and `TextOverflowSide { Clip, Ellipsis, Str }` in `layout/overflow.rs`;
  `parse_text_overflow` takes one or two of `clip` / `ellipsis` / `<string>` (`fade` / `fade()`
  rejected, DIVERGENCES §2); serialized as written, a string through the new
  `serialize_css_string` (CSSOM §2.1); not inherited. The spec's mapping, followed over the brief's
  "start and end": one value is the end line box edge (the start edge clips), two values the
  line-left then the line-right edge. rdom-tui: paint only — `inline_paint/text_overflow.rs`.
  `Marking::of(block)` applies when the block's inline axis clips (§3: "overflow other than
  visible") and a side has a marker; its window is the block's content box columns, unscrolled, so
  a scrolled line is marked at the scrollport's edges. `cut_line` splits the line into pieces —
  each grapheme of its text and generated runs at its UAX #11 width, each atom whole — and on an
  overflowing marked edge keeps the pieces that end by the edge less the marker's cell width (so a
  wide character is hidden whole, never split), the marker painting right after them in the
  block's glyph style; when even the line's first piece does not fit, that edge clips (§3's first
  character rule). `paint_inline_layout` narrows each line's clip to the cut (text, generated
  runs, the selection overlay), skips an atom outside it, and paints the markers; the IFC, the
  text-leaf and the anonymous-block paths share it (an anonymous box's lines take its container's
  marking). Layout, hit-testing and the clipboard serializer are untouched, so a copy is the whole
  text. rtl: rdom starts an overflowing `rtl` line at the left edge (fragment columns are
  unsigned), so it overflows the right — its start — edge; a one-value `ellipsis` marks the end
  (left) edge, which has nothing to hide, and the right is clipped — recorded in DIVERGENCES §4;
  `text-overflow: clip ellipsis` marks the right. Red: `css_phase8/text_overflow.rs` — 5 of 7
  failed (`abcdef` for `abcde…`, `ab中文` for `ab中…`, `cdefgh` for `…defg…`; the `visible` and
  copy pins passed before and after); `overflow_tests.rs` did not compile (no `TextOverflow`;
  written with the types, not run red on its own). Green after. Mutation: the first-character
  rule off → that test (`…` for `a`; restored, touched). `apply_tests`, `canonical_values` and the
  important-setter test list the property. No snapshot changed.
- 2026-10-05 — C8-LINE-CLAMP (CSS Overflow 4 §4, the editor's draft read for the longhand mapping).
  rdom-style: `max-lines` (`none | <integer [1,∞]>`), `block-ellipsis` (`no-ellipsis | auto |
  <string>`, inherited), `continue` (`auto | discard | collapse | -webkit-legacy`), the `line-clamp`
  shorthand (`none` → `none` / `no-ellipsis` / `auto`; an integer → `block-ellipsis: auto` unless
  given, `continue: collapse` or a trailing `-webkit-legacy`), `-webkit-line-clamp` (`block-ellipsis:
  auto` always, an integer's `continue` `-webkit-legacy`), `-webkit-box-orient`, and `display:
  -webkit-box` / `-webkit-inline-box` as the Compat Standard's `flex` / `inline-flex`; types
  `BlockEllipsis`, `Continue`, `BoxOrient` (`layout/line_clamp.rs`), parsers in
  `parse/values/line_clamp.rs`, set / serialize in `property_dispatch/line_clamp.rs`. Found by the
  `all: unset` table test: `unset` on a shorthand resolved by the shorthand's name, so
  `line-clamp: unset` reset the inherited `block-ellipsis`; `css_wide::set_css_wide` now resolves
  it per field by the longhand owning it (CSS Cascade 4 §7.3.3), and the test expects no single
  keyword for the two clamp shorthands (CSSOM gives the empty string). rdom-tui: the cascade's
  `line_clamp::finalize_line_clamp` decides `ComputedStyle::line_clamp_container` (a block container
  with `max-lines` and `continue: collapse` / `discard`) and the legacy form — `-webkit-legacy` on a
  `flex` / `inline-flex` box with a vertical `-webkit-box-orient`, which then computes to `flow-root`
  / `inline-block`, as engines lay it out (a plain `flex` box with those properties counts too,
  DIVERGENCES). `layout_pass/line_clamp.rs::clamp_point` walks the container's line boxes — its own,
  its anonymous boxes', its block-level in-flow descendants' that are not formatting contexts of
  their own — in block order; the Nth one's bottom is the clamp point when anything follows (a
  further line or a box reaching past it). Layout: `layout_node` cuts the measured content there
  before `resolve_auto_height` (both gutter passes), and `intrinsic::wrapped_rows` clamps a
  container's own lines (`clamped_lines_height`), so a clamped flex item is N lines tall; a flex or
  grid item whose Nth line is in a block descendant is measured unclamped (DIVERGENCES §4).
  Paint and hit-testing: `stacking::children_clip` ends the container's content rows at the clamp
  point, whatever its `overflow`; the `block-ellipsis` joins C8-TEXT-OVERFLOW's `Marking`
  (`Marking::of(dom, owner, anon)` finds the flow holding the Nth line by walking up to the
  container through its formatting context), placed after the line's content, which gives up whole
  pieces when the marker does not fit. The CSSOM alias generator (`build.rs`) met `continue`, a Rust
  keyword, and `-webkit-…`: a vendor prefix's hyphen is dropped (`webkit_line_clamp()`) and a
  keyword is a raw identifier (`r#continue()`). Red: `line_clamp_tests.rs` (3) did not compile (no
  types or fields); `css_phase8/line_clamp.rs` — 6 of 7 failed (heights 3 for 2 and 5 for 3,
  `three` for `three>`; the "fits" pin passed before and after). Green after. Mutation (each
  restored and touched): the clamp clip off → three paint tests; the legacy form never active → the
  `-webkit-box` test. `apply_tests`, `canonical_values`, the important-setter test and
  `cascade_inherits_exactly_the_style_crates_inherited_set` list the new properties. No snapshot
  changed. Split: `finalize_line_clamp` lives in `style/cascade/line_clamp.rs`, keeping `apply.rs`
  at 540; TECH_DEBT `SIZE-1` updated (`computed.rs` 557, `inline_paint/mod.rs` 561; `keywords.rs`
  below 500 after `overflow.rs` took `Overflow`).

- 2026-10-05 — C8-ABSPOS-OVERFLOW (TECH_DEBT `ABSPOS-OVERFLOW-1`; CSS Overflow 3 §2.2, CSS 2.1
  §10.1 / §11.1.1, CSSOM View §4). An absolutely positioned element's border box and its own
  scrollable overflow count in the scrollable overflow of the nearest scroll container at or above
  its containing block (a scroll container between the box and its containing block does not
  contain it; a `fixed` box counts nowhere), cut to the overflow clip edges of the `clip` boxes from
  its containing block up and to the reachable side of the scroll container's content box (what
  lies before the scroll origin — left of an `ltr` box, right of an `rtl` one — is unreachable).
  Ordering, fixed at the root: phase 1 records extents, clamps offsets and settles `auto` bars, and
  phase 2 places positioned boxes against what phase 1 laid out, so phase 1 now merges each scroll
  container's `Reach` — the rect its positioned boxes cover, from its border-box origin, unscrolled
  (translation-invariant) — from the last settle (document data, `layout_pass/positioned_overflow/`),
  and `positioned_overflow::settle` measures it again after placement; when it changed, `layout_dom`
  runs phases 1–2 once more (never a third time: a second change is kept for the next layout). So
  the extent, the clamp and the two-pass gutter all see the boxes with no new path through
  `layout_node`. `scroll_extent::extend_box_overflow` is the walk's entry for a box whatever its flow.
  Cost, pinned by `positioned_overflow/cost_tests.rs` (`LAYOUTS` and a `MEASURED` counter): no
  positioned box — one run, nothing measured; three appearing — two runs, each box measured once a
  run; laid out again unchanged — one run (the in-flow layout plus one placement per box), one
  measurement per box. Red: `css_phase8/abspos_overflow.rs` — 4 of 7 failed (`(6, 1)` for `(10,
  6)` and `(9, 4)`, `scrollTop` 0 for 4, content `(6, 2)` for `(5, 1)` under `overflow: auto`); the
  pins — a box contained above the scroller, a `fixed` box, the clamp after the box is removed —
  passed before and after. Added after (green, mutation-checked): a box at `left: -4` adds nothing,
  a `clip` containing block cuts the box. Mutation (each restored and touched): no clip or
  reachable-side cut → those two; no second run → four extent tests and the cost test. Not done
  (DIVERGENCES §2): positioned `::before` / `::after`, placed in a pass of their own after the
  extents settle. No test expectation or snapshot changed.
- 2026-10-05 — C8-RTL-LINE-OVERFLOW (DIVERGENCES §4, from C8-TEXT-OVERFLOW; CSS Text 3 §7.1, CSS
  Writing Modes 4 §2.1, CSS Overflow 4 §3, CSSOM View §4). A line wider than its `rtl` block now
  starts at the right (inline-start) edge and overflows the left (end) edge, as browsers lay it out:
  `inline::align::start_lines_at_inline_start` shifts every `rtl` line by `content width − line
  width`, negative for an overflowing one. At the root, a fragment's column is signed:
  `InlineFragment::x` and `GeneratedFragment::x` are `i32` (were `u16`; Breaking — rdom-tui, with
  the constructors' `x`), so a fragment can sit left of its content box. Readers re-checked: paint
  (`inline_paint`, text and generated runs; the selection highlight skips a cell left of the screen
  instead of wrapping its `u16` column), hit-testing (`hit_test::fragment` / `descend` lost their
  "left of the content box → no hit" guard: a point on the overflowing part of the line hits the
  character there), the caret (`cell_of_position` already took the signed sum), the scrollable
  overflow's `line_rects` (signed spans; the `rtl` scroll origin already reaches negative offsets,
  C5G-RTL-SCROLL) and the static position after an inline run. One-value `text-overflow` now marks
  the left (end) edge where the line overflows (`…fgh`), with no change in `text_overflow.rs`: its
  marking already named the end edge. Red: `css_phase8/rtl_line_overflow.rs` — all 5 failed (`abcd`
  for `efgh`, `    abcdef` for `abcdefgh  `, `abcd` for `…fgh`, the scrolled port blank for `abcd`,
  the hit at byte 0 for 4); green after the shift and the type change. Added after (green,
  mutation-checked): the selection highlight (`e`, `f` lit in a box 2 cells in, `d` clipped) and a
  hit left of the box (byte 2 at column 0). Mutation (each restored and touched): the shift floored
  at 0 → all six layout tests; the hit-test guard back → the left-of-box hit (byte 8 for 2). Changed
  expectation, justified: `text_overflow.rs::rtl_marks_the_named_line_edge` pinned the divergence
  (`abcdef` / `abcde…`); now `…fghij` / `efghij` — the one value marks the overflowing left edge,
  `clip ellipsis`'s line-right edge hides nothing. `migration_hints::line_box_construction_hints`
  builds an atom at `x = -2`. DIVERGENCES §4's entry is gone (and the line-clamp entry's reference
  to it). No snapshot changed.
- 2026-10-05 — C8-FLOAT part 1 (CSS 2.1 §9.5.1 / §9.5.2 / §9.7, CSS Logical 1 §2.3): the values.
  rdom-style: `Float` (`none | left | right | inline-start | inline-end`, `side(rtl)` → `FloatSide`)
  and `Clear` (`none | left | right | both | inline-start | inline-end`, `sides(rtl)`) in
  `layout/float.rs`, parsers in `parse/values/float.rs`, set / serialize in
  `property_dispatch/float.rs`; neither inherited, serialized as written; the flow-relative
  keywords stay as specified and resolve against the containing block's `direction` in layout.
  rdom-tui cascade: `blockify::finalize_float` — an absolutely positioned box's `float` computes
  to `none`, a floated box is blockified (§9.7's table, `blockify`'s mapping); a float establishes
  a BFC (`finalize_bfc_formation`, §9.4.1); `float` / `clear` changes relayout. Red:
  `float_tests.rs` (3) and `css_phase8/float/computed.rs` (2) did not compile (no `Float`, `Clear`,
  fields or builders); green after. Changed expectation, justified: `apply_tests::
  initial_keyword_yields_the_initial_computed_value_for_every_property` cannot perturb `float`
  beside its `display: inline-flex` (a float blockifies it), so `float` is bound `_` there and
  `float_initial_is_none_and_unblockifies` covers it. Until the layout lands (part 2) a floated box
  lays out in flow as the block it computes to (DIVERGENCES §3). No snapshot changed.
- 2026-10-05 — C8-FLOAT part 2 (CSS 2.1 §9.5, §9.5.1 rules 1–9, §9.5.2, §9.4.1, §10.6.7, Appendix E
  step 5; CSS Logical 1 §2.3; CSS Display 3 §2.5): layout, paint and hit-testing. Decided, one
  module: `layout_pass/float/` — `area::ExclusionArea` (one BFC's float margin boxes; `place` is
  §9.5.1: not above an earlier float's top, as high as it fits beside the earlier floats over its
  whole height — else below the first that ends — and as far left / right as it goes; `band` the
  free columns over some rows; `opening` the first top where a box fits; `clearance`, `lowest`),
  pure and unit-tested; a stack of areas as document data (`enter` / `leave` around a block
  container that establishes a BFC — `block::establishes_bfc`, the margin-collapse predicate —
  `with_area` lending the innermost one out for a placement or a packing, never across a
  `layout_node`; `mark` / `rewind` so a box that lays its children out again — its gutter settled,
  a stale offset dropped — places their floats once); `size::FloatBox` (shrink-to-fit width,
  §10.3.5, or the declared one through `box-sizing`, clamped; the content height at it; margins,
  `auto` 0); `lines::LineExclusions`, the API the inline packer consumes — `band(top)`,
  `next_change(top)`, `place_float(id, top, used)` — implemented by `InlineFloats` in the IFC's
  coordinates; `flow::beside_floats` (clearance — the greater of the hypothetical top and the
  floats' bottom — and a BFC root beside the floats, an `auto` width shrinking down to its
  min-content contribution, else below them). Floats are out of flow (`tree::is_in_flow`, §9.3;
  `float_side`: a box parent that is a block container — a flex or grid item, and the root's
  children, rdom's viewport-column items, do not float). Block layout: a float joins an open
  inline run (the packer places it), otherwise stands in a `RunKind::Float` run placed at the
  cursor past the collapsed margins and laid out at once (`settle_height` corrects its exclusion
  to the height it got); a lineless inline run keeps its floats as a float run; the host's
  pseudo-elements and margin-trim / collapse read the first / last non-float run. The packer
  (no ad-hoc offsets): each line takes its band at its top (`open_line`), wraps at the band's
  width, an empty line too narrow for its first word moves down to the next float bottom (§9.5),
  a float met mid-line goes on the line if the band has room after the line's content (the
  content then shifts past a left float when the line settles) else at the next line's top
  (rule 6), and `break_line` aligns the line in its band — `inline/align.rs` is gone, its `rtl`
  rule there (C8-RTL-LINE-OVERFLOW's negative start included). `InlineLayout::line_at_row`
  allows the gap a moved-down line leaves. The IFC, text-leaf and anonymous-box paths pack
  around the floats and lay the floats they placed out; a BFC root's automatic height reaches
  its lowest float (§10.6.7, `dispatch::layout_children`). Paint and hit-testing: a fourth layer,
  `Layers::floats`, painted after the in-flow content and before `z-index: auto` positioned boxes,
  each float atomically (its positioned descendants this context's), hit-tested in reverse —
  DIVERGENCES §2 records that inline content overflowing into a float is under it. The scrollable
  overflow walk counts floats. Red: `css_phase8/float/place.rs` — 8 of 10 failed with the
  implementation stashed (the floats laid out in flow: `XYZ` above `aa bb cc`, floats stacked
  `[(0,0),(0,1),(0,2),(0,3)]`, `aa` / `XY` on two rows); green after. Added after (green,
  mutation-checked): `clear.rs` (5), `bfc.rs` (10), `paint.rs` (2), `area.rs` unit tests (5).
  Mutation (each restored and touched): the packer's line width ignoring the band → four
  line-shortening tests; no clearance and no BFC avoidance → four; no §10.6.7 height → one; no
  `rewind` → the re-layout test (`x` 8 for 0); no float paint layer → eleven; no float hit layer
  → the hit test. Split: the run partition moved to `block/runs.rs` (`partition`, with the float
  rule; `block/mod.rs` 607 → 565). No existing test expectation or snapshot changed.
- 2026-10-05 — C8-FLOAT parts 3–4 (CSS Sizing 3 §5.1 / §5.2, CSS 2.1 §10.6.7 / §9.4.3, CSS Box 4 §3,
  CSS Overflow 4 §3 / §4): intrinsic sizes and the interactions; the planned parts 3 (intrinsic) and
  4 (interactions) landed together. Intrinsic — `float/measure.rs`, the layout's rules on a
  scratch area, the measured box a formatting context of its own: `inline_rows` (an IFC block's or
  text leaf's lines beside its own floats, and the floats' rows, for `wrapped_rows`);
  `block_height` (a block container whose flow holds a float: its runs in order — a float placed
  at the cursor, an inline run packed beside the floats, a block child below the floats it clears
  and, a BFC root, where they leave it room, a non-root block child laid out in the same area so
  its lines go beside the parent's floats — to the lowest float); `block_width` (max-content: a
  float run's floats side by side and beside the in-flow content right after them, an inline run
  packing its own floats as boxes in its line through the measuring packer's `push_float`;
  min-content: the widest piece, a float whole). Interactions: `margin-trim` (CSS Box 4 §3) drops
  a float's inline-start (inline-end) margin when its margin box would abut that content edge —
  `float::place_box` tries the trimmed box and keeps it only flush (`area::position` then `push`)
  — and its block-start margin at the content top (`Placement::content_top`); line clamping
  needed nothing (the clamp container's content clip already cuts the float layer, and its height
  ends at the clamp point after the §10.6.7 extension) — pinned; `text-overflow` marks the end line
  box edge: a line shortened by floats keeps its band (`LineBox::band`, crate-private, only when a
  float shortened it) and `cut_line` narrows the block's window to it. Found and fixed at the root
  (CSS 2.1 §9.4.3): a relatively positioned box laid its subtree out in the shifted box, so a
  float in it excluded at the shifted position — `layout_node` now lays the box out in flow and
  moves it with its subtree afterwards (`tree::shift_box`, layout being translation-invariant;
  `fixed` descendants too, phase 2 placing them again). Red: `float/intrinsic.rs` — 4 of 4 (`1`
  for 3, `2` for 3 twice, `4` for 7; the first draft measured root children, which resolve their
  height from layout, so they passed — the tests now size a row flex container's item);
  `float/interactions.rs` — 2 of 3 (`[(2,1),(7,1)]` for the trimmed `[(0,1),(5,1)]`, `ab cdefRRR`
  for `ab cde…RRR`; the line-clamp pin passed before and after; a first draft of the
  `text-overflow` test expected a `nowrap` word wider than the band beside the float, which §9.5
  moves below it — corrected); `float/bfc.rs::a_relative_offset_does_not_move_a_floats_exclusion`
  (`      aa  ` for `  aa bb   `). Green after. Added after: the grid-item case. Mutation (each
  restored and touched): no float rows → the taller-float test; no block measurement → the
  block-child test; no float width → the width test; trimming off → the trim test; the band
  window off → the `text-overflow` test. Changed expectation: none. No
  snapshot changed.
- 2026-10-05 — C8-SCROLLBAR (CSS Overflow 3 §3.3, CSS Scrollbars 1 §2–§3). rdom-style: the scrollbar
  values in `layout/scrollbar.rs` — `ScrollbarGutter` (moved from `keywords.rs`) gains
  `StableBothEdges` (`auto | stable && both-edges?`, `both-edges` only beside `stable`, either
  order; Breaking — rdom-style), `ScrollbarWidth` (`auto | thin | none`), `ScrollbarColor` (`auto |
  <color>{2}`, thumb then track, colors kept as specified, inherited), and
  `NATIVE_SCROLLBAR_TRACK` / `_THUMB`, which the UA's `::scrollbar*` rules now use too; parsers in
  `parse/values/scrollbar.rs`, set / serialize in `property_dispatch/scrollbar.rs`. rdom-tui
  layout: one helper, `gutter::gutters` (the vertical bar's column on its side, its twin on the
  opposite edge under `both-edges`, the horizontal bar's row), replaces the per-site
  `gutter_axes` arithmetic in the content area, the containing block, and the block, grid and
  wrapped-flex intrinsic sizes; `gutter_axes` reserves nothing under `scrollbar-width: none`, and
  `stable` now reserves on an `overflow: hidden` box too (§3.3 — it was `scroll` / `auto` only;
  Changed — rdom-tui). Paint: `Look` decides what styles a bar — rdom's pseudo-elements while
  both standard properties are `auto`, the standard properties alone otherwise (Chromium's
  precedence over `::-webkit-scrollbar`); `thin` (decided, DIVERGENCES §1): the one-cell bar, no
  track glyph, the thumb in the light line; `scrollbar-color`: the track cells filled with the
  track color, the thumb glyph in the thumb color on it, resolved against the element as
  `caret-color` is; `bars_shown` is false under `none`, so paint, hit-testing and dragging see no
  bar while the wheel, keys and script still scroll. Red: `scrollbar_tests.rs` (4) did not compile
  (no types or fields); `css_phase8/scrollbar.rs` — 8 of 8 failed (the strict parse rejecting the
  properties; the hidden box `(0, 10)` for `(0, 9)`); green after. Mutation (each restored and
  touched): no `both-edges` and no hidden-`stable` → four layout tests; the standard look off →
  the three paint tests. Changed expectations: `apply_tests`, `canonical_values`, the
  important-setter test and `cascade_inherits_exactly_the_style_crates_inherited_set` (which also
  probes `float` / `clear` now) list the new properties. No snapshot changed.
- 2026-10-05 — C8-OVERSCROLL (CSS Overscroll Behavior 1 §3). Found: rdom chained every wheel
  scroll a scroll container could not take to the next scrollable ancestor (CSS's `auto`), with no
  way to stop it; keyboard scrolling never chains (DIVERGENCES §2, now recorded). rdom-style:
  `OverscrollBehavior` (`auto | contain | none`, `chains()`) in the new `layout/scroll.rs`, the
  shorthand (one or two values, `x` then `y`, serialized as one when they match), `-x` / `-y`,
  and `-block` / `-inline` as block-axis aliases in `logical.rs` (`y` / `x` in `horizontal-tb`, one
  storage); not inherited. rdom-tui: `handle_wheel` stops the walk at a scroll container on the
  wheel's axis that could not move and whose value on that axis does not chain. Red:
  `scroll_tests.rs` did not compile (no type or fields); `overscroll_tests.rs` — 3 of 4 failed
  (the strict parse rejecting the properties; the `auto` pin passed before and after); green
  after. Mutation (restored, touched): chaining never stopped → the `contain` / `none` and the
  per-axis tests. Found while testing, recorded not fixed (TECH_DEBT `SCROLLPORT-1`): the
  runtime's clamp uses the padding box, layout's the content box, so with a scrollbar gutter a box
  layout leaves at its far end can be past the wheel's maximum (a forward tick moved it back);
  the tests scroll with `set_scroll_*`, the runtime's clamp. `apply_tests`, `canonical_values`,
  the important-setter test and the inheritance probes list the new properties. No snapshot
  changed.
- 2026-10-05 — C8-SCROLL-PADDING (CSS Scroll Snap 1 §4.1 / §4.2, CSSOM View §5.1, HTML focusing
  steps). rdom-style: `ScrollPadding` (`auto | <length-percentage [0,∞]>`, `resolve(port)`: a
  percentage of the scrollport's size on the side's axis, `auto` 0) in `layout/scroll.rs`;
  `scroll-margin` sides are `i16` cells (a `<length>` of either sign; a percentage is none, a
  viewport-relative length rejected, DIVERGENCES §2); the shorthands take one to four sides and
  serialize in their shortest form; the physical longhands are fields (`scroll_padding_top` …
  `scroll_margin_left`), the block-axis logicals `logical.rs` aliases and the inline-axis ones
  directional mappings (replayed by the cascade with the element's `direction`); viewport units in
  a `scroll-padding` resolve at computed-value time (`absolute.rs`, the C2G gate). rdom-tui:
  `into_view::inset` (the optimal viewing region: the scrollport less `scroll-padding`) and
  `outset` (the scroll snap area: the border box plus `scroll-margin`) — `scroll_element_into_view`
  aligns the outset box once per call in each container's inset port. Found: rdom never scrolled a
  focused element into view; `focus::focus_node` (Tab, Shift+Tab, `focus()`) now does, `nearest`
  on both axes, as browsers reveal focus (Changed — rdom-tui); pointer focus does not. Red:
  `scroll_tests.rs` additions did not compile (no fields); `css_phase8/scroll_padding.rs` — 4 of 4
  failed (the strict parse rejecting the properties); green after. Mutation (each restored and
  touched): no inset, no outset and no focus scroll → all four. Changed expectations:
  `apply_tests`, `canonical_values`, the important-setter test and the inheritance probes list the
  new properties. No existing test or snapshot changed with the focus scroll. Splits (both files
  were at 597–599 lines and C8-SNAP adds to them): the builder's scrolling and scrollbar setters
  moved to `tui_style/builder/scroll.rs` (`builder/mod.rs` 599 → 488), and `Content` /
  `ContentContext` to the new `content.rs` (`computed.rs` 597 → 512; both still re-exported from
  the crate root).
- 2026-10-05 — C8-SNAP (CSS Scroll Snap 1 §4–§6). rdom-style: `ScrollSnapType` (`none | [x | y | block
  | inline | both] [mandatory | proximity]?`, strictness `proximity` when omitted and in the
  shortest serialization), `ScrollSnapAxis::physical` (`block` / `inline` as `y` / `x` in
  `horizontal-tb`), `ScrollSnapAlign { block, inline }` of `SnapAlign` (one value is both; one value
  serialized when they match), `ScrollSnapStop`; not inherited. rdom-tui, decided, one module:
  `runtime/scroll_snap/` — `points` (a container's snap positions on an axis: each box it is the
  snap container of, not inside a nested scroll container, its snap area — the border box outset
  by `scroll-margin`, `scrollbar::outset` — aligned `start` / `end` / `center` with the snapport —
  the padding box inset by `scroll-padding`, `scrollbar::inset` — offset by the scroll the last
  layout placed it at, clamped to the range), `select` (pure, unit-tested: `choose(positions,
  dest, intent, mandatory)` — `Nearest`: the position nearest the destination; `Directional {
  from }`: past the start in the scroll's direction, the one nearest the destination, else under
  `mandatory` the one nearest the start; a `scroll-snap-stop: always` position passed on the way
  — or beyond the destination on the way to the chosen one — is chosen instead; `proximity` within
  `PROXIMITY_CELLS` = 2, decided), and `mod` (`snap(element, to, Motion)` for a scroll, recording
  the boxes snapped to in `ScrollState::snapped`; `resnap`, §5.4: back to the box last snapped to,
  at its new position, or under `mandatory` the position nearest the offset). Wired:
  `smooth_scroll::perform_scroll` takes a `Motion` (`To` for `scrollTo` / `scrollTop =` /
  `scrollIntoView` / Home / End, `By { from }` for `scrollBy` and the arrow / page keys) and snaps
  the clamped destination before it jumps or animates — so a smooth scroll settles snapped; the
  wheel snaps each tick `By` its start (a terminal has no gesture end); a track click pages `By`;
  `end_drag` snaps a released thumb drag `To` (it now takes the dom); the frame re-snaps after its
  layout and lays out again when an offset moved. `into_view` carries the element's box by the
  scroll's real destination (`smooth_scroll::destination`), snapped or not. Red: the
  `scroll_tests.rs` additions were written before the types (not run red on their own);
  `scroll_snap/tests.rs` against a stub `resnap` — 9 of 11 failed (`1` for 3 after a tick, `5` for
  6 after `scrollTo(5)`, the drag left at 7, …; the no-snap pin and the padding test — whose
  unsnapped destination happened to be the expected position — passed); green after; the frame
  test (added after) failed with the frame's `resnap` call disabled (6 for 9). Mutation (each
  restored and touched): the wheel's snap off → the wheel and proximity tests; the drag's → the
  drag test; the snapport's `scroll-padding` off → the padding test. Changed expectations:
  `apply_tests`, `canonical_values`, the important-setter test and the inheritance probes list the
  properties. No snapshot changed.
- 2026-10-05 — Phase 8 items complete (part 2: C8-ABSPOS-OVERFLOW, C8-RTL-LINE-OVERFLOW, C8-FLOAT,
  C8-SCROLLBAR, C8-OVERSCROLL, C8-SCROLL-PADDING, C8-SNAP). CSS-COVERAGE §3.10 is 6 / 0 / 0 / 1 and
  §3.11 13 / 0 / 1 / 0 (the one left, scroll-driven animations, is Phase 12's); total 158 / 23 / 80
  / 46, 103 rows Partial / Missing. ACID: tile 13 lists the new overflow and scrollbar features, a
  tile 19 for floats is proposed, and an interactive step I11 for snapping, overscroll and focus
  scrolling. Open: TECH_DEBT `SCROLLPORT-1` (found by C8-OVERSCROLL). The phase's gates are next.
- 2026-10-05 — Phase 8 gates (with the C7G re-review: all 15 at the root). Architect: 3 blocking —
  clearance does not stop parent / first-child top-margin collapsing (§8.3.1, §9.5.2); re-snap after
  layout undoes every scroll that bypasses `snap()` (smooth steps, thumb drag, autoscroll, caret
  reveal); SCROLLPORT-1 makes a scroller's end unreachable with block padding or a horizontal bar (four
  port notions; recommended one `scrollport` + one `scrollable_overflow` in `layout_pass`). API: 4
  blocking — clearfix (`::after{content:"";display:block;clear:both}`) makes no box (pseudos are inline
  text only, empty content dropped, clearance elements only); `-webkit-box` + `-webkit-line-clamp`
  followed by `line-clamp` resets `continue` and lays out as a flex row; a scroller holding only
  abspos content measures its extent from the box's own top (`scrollHeight` 1 vs 21); snap areas
  taller than the snapport skip their middle (§6.2.3). Non-blocking: abspos-overflow rerun can paint an
  unconverged frame (and up to six layouts per frame with resnap / reveal); focus scrolling fires on
  pointer paths (tree row, dialog return), no `preventScroll`, stale rects in handlers; `float/measure.rs`
  a third block-flow model (adds margins, ignores box-sizing / clamp); per-element cost of floats /
  resnap / text-overflow marking / clamp_point for content that uses none; line-clamp missing from
  `ClipEdges` and the extent, `clamp_point` counts nested padding; float paint order needs a two-pass
  in-flow paint (backgrounds → floats → inline content); FC boxes dodge floats with a pre-layout
  height, packer-placed floats never settle, `float::rewind` dead?; duplicate `is_float` /
  `containing_scroller`; rtl caret off-screen clamps to column 0; files (`router/mouse/mod.rs` 592,
  `block/mod.rs` 577, `apply.rs` 565); atom max-content without shrink-to-fit undocumented; upgrade
  guide misses five silent changes (focus scroll, abspos widening scrollers, stable gutter on hidden,
  relative in-flow-then-shift, rtl overflow); flex container `text-overflow` ellipsis browsers do not
  draw; `-webkit-box` without clamp reads back `flex`; `contain` on non-overflowing box blocks chaining;
  a non-moving mandatory snap chains; inherited `scrollbar-color` disabling `::scrollbar` undocumented;
  DESIGN misses 8 types, two wildcard arms on closed `Overflow`, no root re-exports, scroll padding /
  margin loose fields not `Sides`; the focus README snippet needs `TuiTimers`; stale counts / docs /
  README; ACID gaps; README floats + ellipsis example wanted. Accepted: four float simplifications
  (tell consumers to wrap content in `<body>`), keyboard not chaining, abspos stale-state handling.
  Full reports: `target/claude-logs/c8_gate_{architect,api}.md`. Fix as `C8G-*`, two batches.
- 2026-10-05 — C8G-SCROLLPORT (architect B3; closes TECH_DEBT `SCROLLPORT-1`) with C8G-ABSPOS-EXTENT's
  extent half (API B3). Found: four notions of the scrollport (layout's clamp the content box, the
  runtime's the padding box with the gutters in it, `ClipEdges` the padding box, the containing block the
  padding box less the gutters) and an extent of the content union only, measured `max − min` — so the
  end padding and the row under a horizontal bar were never reachable (wheel stopped at 15 of 17 with
  `padding: 1 0`), `End` / an `end` snap / `scrollIntoView({block: end})` left the target under the bar,
  and a scroller holding only `top: 20` content measured 1 row. Decision, one module
  (`layout_pass/scrollport.rs`): the scrollport is the padding box less the gutters layout reserved (now
  recorded, `ScrollState::gutters`, boxed with the other scroll bookkeeping so `TuiExt` does not grow),
  the gutters at the padding edge as CSS Overflow 3 §5.2 says (the bar moved out of the padding; the
  content box is the same rect); the scrollable overflow area (`scroll_content_*`, `scrollWidth` /
  `scrollHeight`) is the scrollport ∪ the in-flow content extended by the end padding ∪ the contained
  absolutely positioned boxes, grown only away from the scroll origin (§2.2, CSSOM View §4: content on
  the origin's far side is unreachable); the range is area − scrollport. Every reader takes it: layout's
  clamp and pass-2 bar decision, the runtime's bounds and metrics (wheel, keys, `scrollTo`, drag, track
  click, autoscroll, caret reveal, `scrollIntoView`, focus scrolling), the snapport, the sticky view
  rectangle, the containing block, `ClipEdges` (content is clipped at the scrollport — out of a gutter
  with no bar in it), and one `paint_pass::scrollbar::tracks` that paint, hit-testing and the thumb drag
  share (the track spans the scrollport's side; the duplicated bar geometry in `hit.rs` / `drag.rs` and
  `gutter::vertical_bar_column` are gone). Its own `rtl` line boxes now count by their fragments, not
  from the content box's left. Red: `runtime/scrollbar/scrollport_tests.rs` — 9 of 9 failed (padding:
  `scrollHeight` 20 for 22; both bars: range `0..=15` for 16; rtl `-20..=0` for `-23..=0`;
  `column-reverse` `-15..=0` for `-17..=0`; the `end` snap 10 for 11; `scrollIntoView` 6 for 7; the bar at
  column 8 inside the padding; the gutter showing `abcdef`), and `abspos_overflow.rs`'s two new tests
  (`scrollHeight` 1 for 21, 4 for 7). Green after. Mutation (restored, touched): no end padding → the
  padding, rtl, `column-reverse` and both-bars tests. Changed expectations, each now CSSOM's (the area is
  never smaller than the scrollport): `scrollable_overflow_stops_at_a_clipping_descendant` (3 → 5), five
  `css_phase8` extents (`overflow_text`, `overflow_clip`, `abspos_overflow` — the scrollport's 6 × 2
  floor), `nested_scroll_overflow` (4 → 10). No snapshot changed. DIVERGENCES: the "gutter between the
  content box and the padding" entry is gone (now CSS's); new: a flex / grid item's margin is not in the
  area (§2.2 counts it).
- 2026-10-05 — C8G-ABSPOS-EXTENT (API B3, architect N1). The extent half — a scroller holding only
  `top: 20` content measures from its scroll origin (`scrollHeight` 21, scrollable to 16) — landed with
  C8G-SCROLLPORT, whose scrollable overflow area grows only away from the origin (its two tests are
  `abspos_overflow.rs`'s). This entry is the convergence half. Found: `layout_dom` ran phases 1–2 at most
  twice, so a reach whose own scrollbar changed it was painted unconverged — a positioned 20 × 10
  `overflow: auto` box holding `left: 0; right: 0; height: 30`: run 2 adds the vertical bar, the box is
  then 19 wide, the settle's "changed" was dropped, and the frame showed a 20-wide area in a 19-wide
  scrollport (`scroll_range().x()` `0..=1`) until something else laid out. Decision: the minimal fix the
  gate named — request a relayout — paints the bad frame first; the root fix — placing the boxes a
  scroller contains before it records its extent — makes phase 1 depend on phase 2's containing blocks,
  a restructuring out of proportion with a one-column error. Chosen: a third run
  (`positioned_overflow::MAX_ROUNDS` = 3) — run 2 sees the new reach, run 3 the reach run 2's scrollbar
  moved; a reach still changing after that flips a bar on and off (a cycle a browser also cuts) and is
  kept for the next layout. `reachable` (the side of a scroller's content that counts) now starts at its
  scrollport, as the area does. Cost pin: `runtime/app/layout_runs_tests.rs` — a frame with a new
  positioned box that narrows itself, a snap target moved by an insertion and a typed line to reveal
  runs phases 1–2 five times (3 rounds, the re-snap's, the reveal's), and at most 3 × `MAX_ROUNDS`. Red:
  `cost_tests::a_reach_its_own_scrollbar_changes_converges_in_one_layout` — `0..=1` for `0..=0` (2 runs);
  green after (3, then 1 on the next layout). No test expectation or snapshot changed.
- 2026-10-05 — C8G-RESNAP (architect B2, N4, N10; split first, N9). Found: `resnap` ran after every
  layout and wrote the recorded target's position whenever the offset differed from it, while only
  `snap()` set the record and nothing cleared it — so a smooth PageDown under `y mandatory` jumped to its
  destination on the first frame (the animation never showed; `scroll` fired twice a frame, back and
  forth), and every thumb-drag move after a snap was pulled back on the next frame; `resnap` also walked
  the whole tree each laid-out frame and dispatched `scroll` between layout and paint. Decision, two
  rules: (1) a snap records the box and the offset of its snap position (`ScrollState::snapped`,
  `SnapRecord`); `resnap` moves the container only when layout moved that position (§5.4: "re-snapped to
  that same snap position"), retargeting a smooth scroll in flight instead of cutting it; (2) every
  runtime scroll write goes through one funnel (`scrollbar::scroll::write`), whose `WriteKind` says
  whether it is a snap's (`Snap`: the snapped destination, a smooth step towards it, the wheel's
  snapped tick, a track page, a drag release) or not (`Free`: thumb-drag moves, autoscroll, caret /
  node reveal), and a `Free` write that moves the box clears the record — the wheel's inline copy of the
  write is gone; `TuiNodeMutExt::set_scroll` clears it too. (1) also covers writes that bypass the
  funnel (`TuiExt::scroll_y` is public). `resnap` visits only the containers a snap recorded a target in
  (document data `Snapped`, pruned as records clear), and queues its `scroll` events
  (`scroll::write_offsets_queued`); the `App` fires them after painting (`fire_queued_scroll_events`,
  in `draw_if_dirty` and the off-frame `cascade_and_layout`), HTML's rendering-update timing. Split
  first (own commit): `handle_wheel` to `router/mouse/wheel.rs` (`mod.rs` 592 → 462). Red:
  `scroll_snap/resnap_tests.rs` — 4 of 5 failed (the smooth mid-frame at 9 for between 0 and 9; the drag
  pulled back to 6; `resnap` dispatched mid-pipeline, 2 events for 1; 1 container visited for 0; the
  after-paint pin passed before and after). Green after; two tests added while fixing:
  `a_layout_change_mid_drag_keeps_the_drag` (mutation: the funnel not clearing → 9 for 13) and
  `a_layout_that_does_not_move_the_target_leaves_the_offset` (mutation: re-snap regardless of the
  position → 6 for 8); each restored and touched. The frame layout-run pin (C8G-ABSPOS-EXTENT) is
  unchanged: 5. No existing test expectation or snapshot changed.
- 2026-10-05 — C8G-SNAP-TALL (API B4, API N10). Found: `points` emitted each box's aligned position only
  and `select` jumped between them, so 30-row cards in a 10-row `y mandatory` list went 0 → 30 on a wheel
  tick or PageDown and rows 10–29 were never shown (a `scrollTo(15)` went back to 0). Decision: CSS Scroll
  Snap 1 §6.2.3 — each `SnapPoint` of an area longer than the snapport carries its covering range (from
  its start aligned with the snapport's to its end aligned, clamped to the scroll range); `pick` rests at
  the destination itself when it is in a covering range and a directional scroll would pass no other snap
  position on the way, else chooses as before. The snap record keeps the box's aligned position, and
  `resnap` now moves the container by how far that position moved — at the aligned position as before,
  inside a tall box as far into it as before. §6.2.3's spacing condition on the neighbouring positions is
  not checked for a scroll to a destination (DIVERGENCES §2). Chaining, decided: (a) a tick a mandatory
  snap holds in place (no snap position ahead, room to scroll) was read as "at the end" and chained to the
  parent; the wheel now chains only from a box at its boundary in the tick's direction (Overscroll
  Behavior 1 §3). (b) `contain` on a box that cannot scroll: the spec applies `overscroll-behavior` to
  every scroll container "regardless of whether those elements currently have overflowing content or are
  user scrollable", and Chromium does since 144 (blink-dev "Respect overscroll-behavior on non-scrollable
  scroll containers"; the gate's "Chromium chains there" was Chromium before 144) — so rdom keeps stopping
  at a non-overflowing `overflow: auto` box and now also at an `overflow: hidden` one, which the wheel
  used to skip (the modal-backdrop case). Red: `scroll_snap/tall_tests.rs` — 3 of 3 failed (PageDown
  `[30, 60, 90]` for `[10, 20, 30]`; the wheel 30 for 1; `scrollTo(15)` 0); `overscroll_tests.rs` — the
  held mandatory tick chained (outer 1 for 0), `hidden` + `contain` chained (1 for 0). Green after; added
  while fixing: `a_resnap_keeps_the_place_inside_a_tall_card` (15 → 17 after a 2-row insertion). No
  existing test expectation or snapshot changed.
- 2026-10-06 — C8G-CLEARANCE-COLLAPSE (architect B1). Found: `parent_collapses_top_with_first_child`
  had no clearance condition (its doc still said "`clear` isn't a property we model", and DIVERGENCES
  "there is no `float` / `clear` yet"), so in `<body><div><div style="float:left;height:3">F</div><div
  style="clear:left;margin-top:2">x</div></div></body>` the cleared div — the first in-flow child, the
  float being out of flow — let its margin escape: the parent moved down 2, F painted at row 2 and x at
  row 5 (browsers: 0 and 3). Decision: CSS 2.1 §8.3.1 ("…and the child has no clearance") with §9.5.2's
  hypothetical position — the child's margin collapsed through the parent puts it at the parent's top,
  where a float ahead of it in the parent sits too, so a first in-flow child whose `clear` names the side
  of such a float has clearance (`margin_collapse::first_child_has_clearance`); the parent then keeps the
  child's margin inside, and the existing clearance (`float::beside_floats`, max of the hypothetical
  position and the float's bottom) puts it at 3. The sibling and collapse-through halves were already
  right (`place.rs`'s `cleared`). Not weighed: a float from outside the parent reaching below its top
  (recorded in DIVERGENCES' margin-collapse entry, which no longer says floats are missing). Bottom
  edge unchanged: §8.3.1's bottom rule concerns collapse-through children with clearance, which
  `place.rs` already stops. Red: `css_phase8/float/clear.rs::clearance_stops_the_parent_and_first_child_margins_collapsing`
  — `(2, 2, 5)` for `(0, 0, 3)`; the no-`clear` pin passed before and after. Green after; no other test
  or snapshot changed.
- 2026-10-06 — C8G-PSEUDO-BOXES (API B1) — partial: block-level done; `inline-block` / `inline-flex`
  atoms and floated pseudo-elements remain (DIVERGENCES §2). Found: a static `::before` / `::after` was
  always inline text — `static_pseudo_text` ignored `display` (even `none` showed), empty content made no
  box (`visible_inline_pseudos` drops it), clearance applied to elements only — so the clearfix
  (`.c::after { content: ""; display: block; clear: both }`) left `.c` 0 rows tall and the next paragraph
  wrapped beside the float; and a pseudo-element's computed `display` started from rdom's element
  initial `block`, not CSS's `inline`. Decision, on the box tree rather than a fourth pseudo path: a
  block-level static pseudo-element (`inline::generated::is_block_pseudo`: `display` block-level, not
  floated, with `content` — `""` included — in a block-flow host) is its host's first / last item in
  `box_tree::box_sequence` (`BoxItem::Generated(host, slot)`, the item C6G's flex path and box-less
  children already use), a block-level run member (`runs::child_level`), so the host is a block
  container (not an IFC or pure-text leaf; `box_index` accounts for the leading item); the block pass
  lays it out (`block/generated.rs`: margins in the collapsing accumulator, width its own or the
  containing block's less its margins, `min-*` / `max-*`, `clear` against the formatting context's
  floats, height its own or its packed text's, padding and border) into an `AnonymousIfc` with a
  `GeneratedBox` — the record C6G-PSEUDO-FLEX-ITEMS paints and hit-tests — and intrinsic sizing adds it
  (`intrinsic::measure_content`). It never packs into a line (`own_inline_pseudo_text` excludes it), its
  margins do not collapse through the host (both parent / child predicates stop at it), and a `flex` /
  `grid` / `flow-root` one packs its text as one inline formatting context. `display: none` now generates
  nothing (`static_pseudo_text`), the pseudo cascade starts from `display: inline` (CSS Display 3 §2),
  and the intrinsic width counts only inline pseudo text (`pseudo_content_width` — positioned, `none` and
  block-level ones excluded). Positioned pseudo-elements, counters and the flex / grid item path are
  untouched; Phase 10 has no row for pseudo-element `display` (C10-CONTENT, -LIST-ITEM and -FIRST are
  about content, markers and `::first-*`). `display: table` (Bootstrap's clearfix) is a Phase 13 value,
  still rejected. Split (SIZE-1): `block/mod.rs` would have reached 606 — the collapsed-border sibling
  overlap moved to `place::borders_overlap`, the generated arm to `block/generated.rs` (578). Red:
  `css_phase8/pseudo_boxes.rs` — 5 of 6 failed (the clearfix `.c` 0 rows for 3; `"Titlebody"` for a line
  each; the empty `height: 2` box absent; `flex` / `grid` / `flow-root` inline; `display: none` showing
  `"Tbody"`); green after, with two tests added (the background at the box; a flex item host 5 × 2 —
  mutation: no intrinsic hook → 2 × 1). CSS-COVERAGE: the `::before` / `::after` row Supported → Partial
  (it had claimed what `display` never did), §3.16 3 / 2 / 5 / 6, total 157 / 24 / 80 / 46. No existing
  test expectation or snapshot changed.
- 2026-10-06 — C8G-WEBKIT-CLAMP (API B2, API N8 / N9, architect N5). Found: (1) `display: -webkit-box;
  -webkit-box-orient: vertical; -webkit-line-clamp: 3; line-clamp: 3` — what autoprefixers and Tailwind
  emit — was not clamped: the later `line-clamp` set `continue: collapse`, the legacy test needed
  `-webkit-legacy`, and the box stayed a flex row; (2) `-webkit-box` parsed as plain `flex`, so a vertical
  one without a clamp was a row and `display` read back `flex`; (3) a flex container with `text-overflow:
  ellipsis` marked its anonymous items' lines; (4) the clamp clipped only in `stacking::children_clip`, so
  a clamped paragraph's hidden lines counted in its scroller's overflow (`scrollHeight` 10 in a 3-row
  box); (5) `clamp_point` took a nested block's border-box bottom as content after the Nth line, so
  bottom padding drew an ellipsis. Decisions: rdom-style keeps the legacy keyword
  (`TuiStyle::webkit_box` / `ComputedStyle::webkit_box`, a `display`-owned field beside `list_item`,
  `parse::values::is_legacy_box`), and `display` serializes it as written (`-webkit-box` /
  `-webkit-inline-box`, as browsers' `getComputedStyle` does). The cascade rule
  (`style/cascade/line_clamp.rs`): a `-webkit-box` with a vertical orient and `max-lines` is the legacy
  clamp whatever `continue` says (a `flex` box with the orient no longer is — the DIVERGENCES caveat is
  gone); a `-webkit-box` without a clamp is a flex container whose direction is its orient (Chromium's
  legacy flexbox ignores `flex-direction`). `text-overflow` marks only a block container's lines (§3:
  "applies to block containers"). The clamp joins the clip edges: `ClipEdges::of_element` adds a
  line-clamp container's clamp point on the block axis and serves paint / hit-testing
  (`children_clip`, whose spans now narrow the incoming clip), the scrollable-overflow walk and
  `positioned_overflow`. `clamp_point` reads a recursed descendant's content box. Red:
  `display_tests::the_legacy_webkit_box_keywords_read_back` (`flex` for `-webkit-box`);
  `css_phase8/line_clamp.rs` — the pair 3 rows for 2, the vertical box `abcd` for a column, the
  scroller `scrollHeight` 10 for 3, `b…` for `b`; `text_overflow.rs::a_flex_container_draws_no_ellipsis`
  `abcde…` for `abcdef`. Green after. Changed expectations: `apply_tests`'s exhaustive initial test skips
  `webkit_box` (PERTURB's vertical orient and `max-lines` would make it the legacy clamp and undo its
  `flow`; covered by the tests above), and `property_dispatch::tests`' `display` mask gains
  `WEBKIT_BOX`. No snapshot changed.
- 2026-10-06 — C8G-CARET-RTL (architect N8). Found: `cell_of_position` clamped the caret's column and
  row with `.max(0) as u16`, so a caret on the part of an `rtl` line overflowing left of the screen
  painted in column 0, and a caret scrolled above the screen read as row 0 — the caret reveal then
  scrolled a textarea at row 2 scrolled to 5 only by 2 (to 3), and Home / End / Up / Down computed their
  line from the clamped row. Decision: one signed answer, `inline::caret::caret_cell` (`(i32, i32)`), read
  by the reveal and the line movements; the public `cell_of_position` keeps its `(u16, u16)` and returns
  `None` for a caret on no cell (documented; the caret painter then draws nothing). Not breaking: the
  signature is unchanged, the off-screen answer was wrong. Duplicates removed: `stacking::is_float`
  (which read the passed-in parent, the DOM parent at the hit-test call, not the box parent) is now
  `float::float_side`'s answer for the element, so `is_layered` takes the element; the containing-block
  ancestor walk is one function, `positioning::containing_ancestor`, used by placement and by
  `positioned_overflow` (whose copy C8-CB-COMPLETE's walk had drifted from). Red:
  `css_phase8/rtl_line_overflow.rs::a_caret_left_of_the_screen_is_on_no_cell` — `Some((0, 0))` for
  `None`; `runtime/scrollbar/reveal_tests.rs` — `scrollTop` 3 for 0. Green after; no other test or
  snapshot changed.
- 2026-10-06 — C8G-PSEUDO-ATOMS (API B1, the rest of C8G-PSEUDO-BOXES). Found: an `inline-block` /
  `inline flow-root` / `inline-flex` / `inline-grid` `::before` / `::after` was packed as inline text (no box:
  its `width`, padding, border and background never drawn), a floated one stayed inline text, and a `flex` /
  `grid` one — block-level or not — packed its text as an inline formatting context, ignoring its
  container properties. Decision, on the box tree throughout (no fourth pseudo path): a pseudo-element with a
  box of its own is the box tree's generated item, `items::AnonymousItem` — the record C6G-PSEUDO-FLEX-ITEMS
  made for pseudo flex items, now built for any pseudo (`AnonymousItem::pseudo`, `Item::of_box`) and measured
  through the same `Keywords` / `contribution` doors elements use. (1) Atoms: `inline::generated::InlinePseudo`
  classifies a static pseudo once (`Text`, `Atom` by `box_tree::is_atomic_inline`, `Float`), and every place
  that feeds a pseudo to the packer goes through it (`feed::push_pseudo_box`); the packer places an atom as an
  element's (`open_atom` / `close_atom`, shared), as one `GeneratedFragment` holding its box (`is_atom`,
  `atom_rows`; `vertical::AtomRows::of` is now the one row geometry for element and pseudo atoms, `AtomAt`
  settles both), and the layout pass lays its content out inside it after packing
  (`layout_pass::generated_atoms`, at the three packing sites). (2) Floats: the float module's API takes a box
  item (`float_side_of`, `is_float_item`, `clear_sides_of`, `place*`, `clearance_floor`, `FloatBox::of`,
  `outer_contribution`, `LineExclusions::place_float`, the packer's pending floats); a host's floated pseudo is
  an item of its box sequence (`generated::sequence_pseudos`, beside the block-level ones), placed by the block
  pass or the packer, laid out by `float::lay_out` and kept on the box whose run placed it
  (`TuiExt::floated_pseudos`, one thin `Box`: the size tripwire 440 → 448) — shifted with it, counted in its
  scroller's overflow and its `opacity` layer, painted in its stacking context's float layer
  (`LayerEntry::generated`) and hit-tested there to its host. (3) A `flex` / `grid` pseudo (block-level or
  atomic) lays out its one anonymous item (`AnonymousItem::content_item`, the anonymous box style of the
  pseudo) by `flex::layout_flex_children` — which already took items — and by grid layout, whose container is
  now `grid::GridBox` (an element, or a generated container's one item: `layout_generated_grid`,
  `generated_content_size`). A generated flex container's content size is its text's: its one `flex: 0 1
  auto` item with no box of its own is as wide as its text up to the content width and wraps to the same
  rows (documented at `AnonymousItem::content_size`). The block-level pseudo (C8G-PSEUDO-BOXES) now sizes and
  lays its content out through the same item. A pseudo's text packs in its own `white-space` and `direction`
  (`inline::pack_generated`). `GeneratedFragment` is `#[non_exhaustive]` (Breaking, `GeneratedFragment::text`;
  also re-exported at `render::`). Split (SIZE-1): `block/mod.rs` 579 → 522 + `block/inline_run.rs`;
  `inline_paint/mod.rs` would have reached 641 → 519 + `inline_paint/generated.rs`. Red:
  `css_phase8/pseudo_atoms.rs` — 11 of 12 failed (`Xbody` for ` X body`; `aa bbbody` for a two-row atom;
  `abbody` for `    abbody` / `  ab  body` / `ab  body`; `T` at column 0 for 4; `ab cd` on one row for two;
  `Fbody text` for `F body` / `  text`; `F` / `para` on two rows for one; cleared `y` 1 for 3; intrinsic 2 for
  4), the floated `::after` test passed by coincidence and was rewritten (`bodyR` → `body     R`, red after);
  `a_click_on_a_floated_pseudo_targets_its_host` added while fixing. Green after. Mutations (restored,
  touched): no content item → the four flex / grid tests; the packer dropping atoms and floats → eight;
  hit-testing skipping generated floats → the click test. CSS-COVERAGE: `::before` / `::after` Partial →
  Supported, §3.16 4 / 1 / 5 / 6, total 158 / 23 / 80 / 46. DIVERGENCES: the "never an atom or a container"
  entry is now the two departures left (a block-level pseudo's margins do not collapse through its host;
  `display: table` waits for Phase 13); the floats entry no longer says pseudo-elements do not float. No
  existing test expectation or snapshot changed.
- 2026-10-06 — C8G-PAINT-PHASES (architect N6; DIVERGENCES §2's older "a later block's background covers
  earlier overflowing text", C4G-SHADOW-ORDER's gap). Found: each in-flow box painted whole — background,
  border, text, children — and the floats after the whole in-flow content, so a float covered inline content
  overflowing into it and a later block's background covered an earlier block's overflowing line; and a float
  inside a `z-index: auto` positioned box was painted (in the context's float layer) before that box, whose
  background then hid it. Decision: Appendix E's phases per *paint unit* — a context root, a `z-index: auto`
  positioned box (step 8), a float (step 5) and an atomic box (7.2.1.4.1.1; flex / grid items, Flexbox §5.4) all
  paint "as if [they] created a stacking context": the unit's box, its in-flow block-level boxes' shadows,
  backgrounds and borders in tree order (step 4), its floats (step 5), then its inline content (step 7), an
  atomic box whole at its turn. The walk that collects a context's layers (`stacking::collect`, the old walk
  moved, `stacking.rs` 552 → `stacking/` — `mod.rs` 254, `collect.rs` 295, `unit.rs` 167) also gathers the background-phase
  boxes (`BoxEntry`, every in-flow block-level box where it gathered the shadowed ones; a block-level pseudo's
  box too) and the floats with the unit they belong to (`LayerEntry::owner`), for the root's unit and each
  `z-index: auto` unit; a float or an atomic box gathers its own when it paints (`for_each_unit_box`, the old
  atom-shadow walk generalized, allocating only for a float). The inline-content phase is the existing content
  recursion, which no longer paints an in-flow box's own box (`box_paint::box_frame` split from `paint_box`).
  The two-stroke shadow (backdrop phase + under-text at the box's turn, `Shadows`, `tint_bg`) is gone: a shadow
  paints once, with its background. Cost pin (`paint_pass/phase_cost_tests.rs`): a paint looks at each node
  twice (layers + content, as before) and paints each block box once, for 10 / 40 siblings and a 20-deep chain.
  Hit-testing still tries floats before in-flow content (DIVERGENCES §2). Red: `css_phase8/paint_phases.rs` —
  3 of 5 failed (`abcdefgF  ` for `abcdefghij`; row 1 blank for `line2`; `  text` for `F text`); the inline-block
  float and atom-turn tests were guards (green before and after). Green after. Mutation (restored, touched):
  repainting a box whole in the content phase → the later-block test. No existing test expectation or snapshot
  changed.
- 2026-10-06 — C8G-FLOAT-MEASURE (architect N3, N7, N10). Found: intrinsic block sizes of block containers
  were two models beside layout's — `children_size` summed the children's outer contributions (no margin
  collapsing at all: two `margin: 1 0` paragraphs measured 6 for a laid-out 5, with floats or without) and
  `float/measure.rs` re-implemented the flow when a float was in it (its own chrome arithmetic: no
  `min-height` — 2 for 4 —, no `box-sizing`, no clamp); a line-clamp container whose lines were in block
  descendants measured unclamped (DIVERGENCES); a formatting context root dodged floats at its pre-layout
  height (`overflow: hidden` text checked as one row, laid out as three, overlapping a lower float: `x` 4 for
  6); floats placed by the packer were never settled to their laid-out height; and an inline block took its
  max-content width (no shrink-to-fit, min-content contribution 7 for `aaa bbb`). Decision — one model:
  block layout's run loop moved to `block/flow.rs` (`prepare`, `inset`, `run`) behind a `FlowSink` — what a
  float, a block child, a block-level pseudo-element and an inline run are placed *as* — with two sinks:
  layout's (`block/mod.rs` `LayoutSink`, the document) and measurement's (`block/measure.rs`, a scratch area,
  the measured box its own formatting context's root as before). The arithmetic is shared, not duplicated:
  `place::place_block_child` (width, height through `resolve_block_height` — `box-sizing`, `min-*` /
  `max-*` —, the outer margin chains, collapse-through, clearance and `float::beside_floats_in`) and
  `place::advance`; `generated::place` for block-level pseudo-elements. Measurement takes a child's own
  (memoized) height unless floats are in the area or a clamp is counting lines; then it measures the child's
  content in the same area, by the same `dispatch::children_layout` decision layout reads (now one function:
  `Inline` / `TextLeaf` / `Block` / `Flex` / `Grid`), counting line boxes for the clamp (§4.4).
  `intrinsic::measure_content` sends a block container's block size there; `children_size`'s block sum and
  `float/measure.rs`'s block flow (`block_height`, `Chrome`, `in_same_context`, `outer_height`,
  `block_top`) are gone, and so is the generated pseudo-elements' separate row count. N7: a formatting
  context root is placed again at its laid-out height when taller (`place::replace_beside_floats`, in both
  sinks, once — DIVERGENCES §2); every float is settled by its index in the area (`PlacedFloat`,
  `ExclusionArea::grow` replacing `grow_last`), the packer's too (`float::lay_out` lays out and settles);
  `float::rewind` is live on one path — a block that is no scroll container dropping a stale offset relays
  its children in its parent's context (`laying_a_block_out_again_places_its_floats_once` fails without it)
  — and was dead after a scroll container's gutter pass (a formatting context root: the call is gone, the
  reason documented). N10, fixed: an inline block's `auto` width is shrink-to-fit (§10.3.9), the float's
  rule (`float::size::FloatBox::of`, for element and pseudo-element atoms), and an intrinsic measurement takes
  an atom's min- or max-content contribution. Splits: `block/mod.rs` 522 → 287 + `flow.rs` 385 +
  `measure.rs` 253; `intrinsic/mod.rs` 579 → 336 + `intrinsic/content.rs` 259. Red:
  `css_phase8/float/measure.rs` — 6 of 6 failed (6 for 5 with and without a float; 2 for 4; 2 for 1 under
  `line-clamp: 1`; `x` 4 for 6; the settle test panicked slicing a multi-byte row and was fixed to chars;
  `aaa b` for `aaa` / `bbb`). Green after. Mutations (restored, touched): no settle → the settle test (`lo`
  for `hello`); no same-context measurement → the clamp test; no re-place → the formatting-context test; no
  stale-offset rewind → the re-layout test; the gutter-pass rewind removed → nothing fails (dead).
  DIVERGENCES: the clamped-item entry is gone (its `rtl` ellipsis note kept), simplification 2 says the
  exclusion settles after its run, and the once-only re-place is recorded. No existing test expectation or
  snapshot changed.
- 2026-10-06 — C8G-IDLE-COST (architect N4, the remainder after C8G-RESNAP's snap walk). Found, by counting:
  every block container's inline-size measurement built its box sequence and float-run partition to learn it
  held no float (`float::measure::block_width`: 16 partitions for eight float-less flex items' paragraphs);
  every inline flow on every paint walked up its ancestors to the formatting root looking for a line-clamp
  container (`text_overflow::block_line`: 390 steps over three paints of a tree with none); and a line-clamp
  container's clamp point — a walk of its descendants' lines, gathered into a `Vec` and sorted — was
  computed again by `ClipEdges::of_element` and by each flow's marking on every paint (24 walks over three).
  Decisions: (1) `block_width` asks `may_hold_floats` first — an allocation-free scan of the children
  (through box-less ones) and of the floated pseudo-elements, a superset of the partition's answer, which
  still decides; the block size path is the flow itself since C8G-FLOAT-MEASURE (layout's model, its
  partition inherent). (2)–(3) the clamp points the layout pass finds are document data
  (`line_clamp::ClampPoints`, cleared at `layout_dom`'s start, written by `line_clamp::clamped` when layout
  cuts a container's height), each row kept from the container's scrolled content top so a later shift of
  the subtree (relative offset, sticky, `align-content`) or of its scroll offset keeps it true;
  `clamp_point` reads it (walking only a container no pass laid out), and `line_clamp::any` — whether the
  pass laid one out — gates `block_line`'s walk. The gate's "only when `text-overflow` is set somewhere" is
  this walk's condition read as what the walk is for: the `block-ellipsis` of a clamped line; a box's own
  `text-overflow` is an O(1) read of its style. Pins: `layout_pass/idle_cost_tests.rs` (0 partitions without
  a float, some with one), `inline_paint/cost_tests.rs` (no walk step without a clamp; no clamp walk at
  paint with one). Red: 16 for 0; `(390, 0)` for `(0, 0)`; 24 for 0. Green after. Added while fixing:
  `css_phase8/line_clamp.rs::a_shifted_clamped_box_keeps_its_clamp_point` (mutation: rows kept absolute → it
  fails, the second line clipped). No existing test expectation or snapshot changed.
- 2026-10-06 — C8G-FOCUS-SCROLL (architect N2, API N7). Found: `focus_node` scrolled the newly focused
  element into view on every caller — the tree builtin's row click (a pointer press focuses a focusable row,
  the click then focuses the tree: `nearest` moved a tall tree from 4 to 3), a closing dialog returning focus
  (HTML §4.11.4 "close the dialog": "the viewport should not be scrolled by doing this step"; it scrolled to 6
  from 0) — `focus()` took no options (HTML `FocusOptions.preventScroll`), and in a handler it aligned the rect
  of the last layout, so an element the handler had just moved was revealed at its old place (`scrollTop` 0 for
  4). Decision: `FocusOptions { prevent_scroll }` (`#[non_exhaustive]`, `new()` / `prevent_scroll(bool)`),
  `TuiAccessorsMut::focus_with(options)` beside `focus()`, `runtime::focus::focus_node_with_options` beside
  `focus_node` (now its default form); the dialog's return and the tree's pointer click pass
  `prevent_scroll` (a keyboard-synthesized click still scrolls: keyboard focus); pointer focus already did not
  scroll. Fresh rects: rdom cannot flush layout from a handler (TECH_DEBT `FOCUS-FLUSH-1`), so under a
  running `App` (`timers::in_app`: its scheduler installed around every user-code entry point) the scroll is
  recorded (document data `PendingFocusScroll`) and done after the `App`'s next layout, the one the focus is
  shown in (`frame::style_and_layout`, after the re-snap, laying out again when it moved an offset); on a bare
  document it is done at once — DIVERGENCES §2 records the timing. Red: `runtime/focus/scroll_tests.rs` — 4
  of 4 failed (the API stubbed: `prevent_scroll` 6 for 0, the dialog 6 for 0, the handler 0 for 4; the tree
  click passed at first — the press focused the tree, so the click's focus was a no-op — and was rewritten
  with focusable rows, 3 for 4). Green after. Mutation (restored, touched): never deferring → the handler
  test. No existing test expectation or snapshot changed.
- 2026-10-06 — C8G-API-TYPES (API N12, N13, N6). Found: DESIGN's closed / open list did not name the eight
  Phase 8 scroll types; two `Overflow` matches ended in a wildcard arm (`paint_pass/scrollbar.rs`
  `should_paint`, rdom-style `ComputedStyle::normalize_overflow`) on a closed type, where a new variant must
  fail to compile; the Phase 8 value types were reached only through `rdom_tui::layout`; `scroll-padding-*` /
  `scroll-margin-*` were four loose fields each on `TuiStyle` and `ComputedStyle` where `margin` / `padding`
  are `Sides` (C6-MARGIN-SIDES); and the upgrade guide's focus snippet called `request_animation_frame`
  without the `TuiTimers` trait, in neither the root nor the prelude. Decisions: DESIGN classifies
  `OverscrollBehavior`, `ScrollPadding`, `ScrollSnapType`, `ScrollSnapAxis`, `ScrollSnapStrictness`,
  `SnapAlign`, `ScrollSnapAlign`, `ScrollSnapStop` as closed CSS values (each a decision the wheel, the
  scroll-into-view region or the snap selection must make), and `FocusOptions` (C8G-FOCUS-SCROLL) as an
  options bag, `GeneratedFragment` (C8G-PSEUDO-ATOMS) as a growing layout output; the two matches name every
  variant, under `#[deny(clippy::wildcard_enum_match_arm)]`; the root re-exports the Phase 8 values
  (`Float` … `ZIndex`), `FocusOptions` and `TuiTimers`, the prelude `FocusOptions` and `TuiTimers` (the
  re-export rather than a snippet change: a consumer needs the trait to call the method); `TuiStyle::
  scroll_padding` / `scroll_margin` are `Sides<Option<Value<…>>>` and `ComputedStyle::scroll_padding` /
  `scroll_margin` `Sides<…>` — the dispatch table, the cascade's field list and `absolute.rs` address
  `.top` … `.left`; the per-side builders keep their names (`side_setter!`, moved to the builder module and
  given an `exact` arm so `scroll_margin_left(2)` still infers its `i16`). Breaking for git-main consumers only
  (the fields were added after 0.5): CHANGELOG Breaking bullet and the "added after 0.5" table row. Red:
  `migration_hints.rs::scroll_sides_and_root_hints` — 23 compile errors (the root types, `FocusOptions`,
  `TuiTimers`, the `Sides` fields); `cargo clippy` with the `deny` on the two functions — the wildcard arm.
  Green after. No test expectation or snapshot changed.
- 2026-10-06 — C8G-DOCS (API N5, N11, N14, N15, N16; architect N9, N10, the accepted `<body>` note). Upgrade
  guide: the silent changes it missed, ranked by impact into the list — focus scrolling into view (#6),
  an absolutely positioned box widening its scroller (#7), a `stable` gutter on `overflow: hidden` (#22),
  relative positioning laid out in flow and then shifted (#27), the `rtl` line overflowing left (#29) — and
  this batch's own: block content measured with collapsed margins and shrink-to-fit inline blocks (#4,
  C8G-FLOAT-MEASURE), overflowing text painting over what follows it (#28, C8G-PAINT-PHASES); #21 now says
  a stylesheet's `overflow: clip`, dropped in 0.5, clips. DIVERGENCES: `scrollbar-color` is inherited, so
  an ancestor's switches `::scrollbar` styling off below it, the UA accent included. Stale docs: CSS-COVERAGE
  counts 227 property names (159 + 68 flow-relative); its roadmap row 47 no longer promises keyboard
  chaining; the root README's Unreleased list has a positioning / floats / overflow / scrolling bullet;
  `reserve_scrollbar_gutter`'s doc names `hidden` under `stable` (the `_forced` doc was already true);
  DIVERGENCES' "no `float` / `clear` yet" had gone with C8G-CLEARANCE-COLLAPSE. The `<body>` note, decided:
  its home is the rdom-tui README's new "Floats and text overflow" section, where a consumer meets floats,
  beside DIVERGENCES' simplifications entry (which already says it) — a doctested example of a `.media`
  float with `margin-right` beside its text, the clearfix, a `.truncate` line and a `line-clamp: 2` block,
  its painted rows asserted. ACID: tile 13 gains the autoprefixed `-webkit` + `line-clamp` pair; tile 19
  the clearfix (and with a cleared first child's margin), the `.media` float, the paint phases and the
  pseudo-element atoms and floats; I11 PageDown under `mandatory`, a card taller than the snapport, a key at
  the end not chaining and a smooth PageDown. File sizes, settled in CLAUDE.md §Architecture Hygiene: 500
  is the bar (past it a file is listed in `SIZE-1`), 575 splits on touch (a change leaving a touched file
  past it splits it), 600 is the gate's hard limit (`file_sizes.rs`); no file this batch touched ends past
  575 (`apply.rs` 566). Test gaps (API N16), `css_phase8/gate_gaps.rs`, all green as written but one: a
  `scrollbar-width: none` scroller's last column is content — painted, hit, dragged as such — and it
  scrolls by the wheel; Shift+Tab within `scroll-padding-top`; focus scrolling nested scrollers; pointer
  focus not scrolling; `overflow: clip` not containing its float — written expecting the float painted,
  red, and rewritten: `.c` clips it away at its 0-row clip edge while it still excludes (CSS Overflow 3
  §3.1, the code right); a stuck sticky box carrying its absolutely positioned child; a `.truncate` flex
  item. No test expectation or snapshot changed.
- 2026-10-06 — Phase 8 closed: both gates run, 15 gate fixes `C8G-*` (batch A: SCROLLPORT, ABSPOS-EXTENT,
  RESNAP, SNAP-TALL, CLEARANCE-COLLAPSE, PSEUDO-BOXES, WEBKIT-CLAMP, CARET-RTL; batch B: PSEUDO-ATOMS,
  PAINT-PHASES, FLOAT-MEASURE, IDLE-COST, FOCUS-SCROLL, API-TYPES, DOCS); their re-review rides with the
  Phase 9 gate. Open for that review: the re-place of a formatting context root beside floats is done once;
  a float's settled height reaches content after its run only; focus scrolling under an `App` is at the
  next layout (`FOCUS-FLUSH-1`).
- 2026-10-06 — C9-WHITE-SPACE (CSS Text 4 §3, §4.1, §6.1; CSS Text 3 §3, §4.1.1–§4.1.3, §5.1; CSSOM
  §6.7.2's shortest form). rdom-style: `white-space` is the shorthand of `white-space-collapse` (`collapse |
  preserve | preserve-breaks | preserve-spaces | break-spaces`) and `text-wrap-mode` (`wrap | nowrap`):
  `layout/text.rs` (`WhiteSpaceCollapse`, `TextWrapMode`, `WhiteSpace` moved there with `PreLine` /
  `BreakSpaces`, `longhands` / `from_longhands`), the CSS Text properties grouped — `TuiStyle::text`
  (`TextDeclarations`) and `ComputedStyle::text` (`TextStyle`, inherited whole: every CSS Text property
  inherits, so later items add fields there and the cascade copies one group); `parse/values/text.rs`,
  `property_dispatch/text.rs`, `tui_style/builder/text.rs`; the shorthand takes the four keywords and the
  longhand pair in either order (an omitted one initial) and serializes as the keyword a pair spells, else
  the non-initial longhands (`preserve-breaks nowrap`). `white-space-trim` is not parsed (DIVERGENCES §2).
  Breaking — rdom-style (the fields, `ImportantMask::WHITE_SPACE` → two bits, the new variants). rdom-tui:
  decided at the root — the CSS Text properties apply to text, so the packer reads each text's own values
  (`inline/run_style.rs::RunStyle`, the text node's parent element's or the pseudo-element's computed style)
  instead of the block's `white-space` for the whole flow; a `<span style="white-space: pre">` in a normal
  paragraph keeps its spaces. White space processing is `inline/white_space.rs` (`classify`: collapsible,
  forced break, preserved space / tab, control, text; `segment_break_removed`, the UA-defined
  transformation: removed next to U+200B or between two East Asian F / W / H non-Hangul characters, Gecko's
  rule — DIVERGENCES §2; `is_collapsible_white_space`, shared with `bears_line` and the copy serializer).
  The packer's intake moved to `packer/intake.rs` (packer `mod.rs` 470 → 327 + `intake.rs` 229) and is
  rebuilt on three rules: (1) a soft wrap opportunity commits the word buffer only when its text wraps — a
  `nowrap` collapsible space sits inside the buffer as one space, so `nowrap` content is measured and placed
  as one piece (it was placed word by word, each beside the last); the opportunity takes the
  `text-wrap-mode` of the text before it (DIVERGENCES §2); after an atom, the next text's; (2) `pre-wrap`'s
  opportunity is at the end of a sequence of preserved spaces (was before and after each), `break-spaces`'
  after each; (3) Phase II — a word's trailing `pre-wrap` spaces are not measured for fit and hang
  (`LineBox::hang`, crate-private: unconditionally at a soft wrap, at a forced break or the end only the
  cells that overflow, `LineEnd`), placement (`rtl` start, later `text-align`) and intrinsic sizes read
  `width - hang`, so they count for max-content and not min-content. A buffer's trailing collapsed space
  becomes the separator after the word is placed (`commit_word` / `place_word`; the first draft made it
  pending before placing and lost it — found by the float test below). The copy serializer collapses per
  `white-space-collapse` (`pre-line` keeps its line feeds, `preserve-spaces` copies them as spaces).
  Red: `css_phase9/white_space.rs` — 7 of 10 failed against the new syntax cascaded into the old packer (a
  temporary mapping: `pre-line` as `normal`, `break-spaces` as `pre-wrap`): `a b c d` for `a b` / `c d`, `ab`
  / ` cd` for `ab` / `cd` (the space wrapped), `ab` left in an `rtl` box for `  ab`, `a b c` for `a  b c`,
  `aaa b b c` for `aaa b  b c`, `中文 字` for `中文字`, min-content 2 for `break-spaces` 3; the
  `break-spaces` test passed on the old `pre-wrap` path (its rule), the longhand-combination one by the
  mapping, the copy test by the serializer change made with the types. Green after; `white_space.rs` unit
  tests and `property_dispatch/text_tests.rs` (written with the types, compile-red). Mutation (restored,
  touched): no hang → two tests; the segment break kept → its test; every run styled initial → six.
  Changed expectation, justified: `css_phase8/float/interactions.rs::text_overflow_marks_the_line_box_edge_
  beside_a_float` laid out `ab cdefghij` (`nowrap`) beside a float because its first word fit; `nowrap` text
  is one piece now, so CSS 2.1 §9.5 moves it below the float (`a_nowrap_line_too_wide_for_the_band_moves_
  below_the_float` pins that) — the marking test now reaches the band window through an atom and text after
  it that cannot wrap (`ab …   RRR`; mutation: the band window off → `ab XYZWcde`). No snapshot changed.
- 2026-10-06 — C9-BREAKING (CSS Text 3 §5.2, §5.3, §5.5, §6.1; UAX #14 LB8, LB11–LB14, LB21, LB25, LB29–LB31;
  HTML `<wbr>`). rdom-style: `word-break`, `overflow-wrap` (+ `word-wrap`, a legacy name alias: the same field,
  `fields_of` and set / serialize arms), `line-break`, `hyphens` in the `text` groups (`WordBreak`,
  `OverflowWrap`, `LineBreak`, `Hyphens`), all inherited; the cascade's CSS Text applicator moved to
  `style/cascade/text.rs` (`apply.rs` 567 → 566). rdom-tui — what rdom used: white space, any two-cell
  grapheme on both sides, after `-`. Decided extent (DIVERGENCES §2): a UAX #14 subset without dictionaries
  in `render/inline/breaking.rs` — `BreakClass` by character (ID: CJK ideographs, kana, Hangul, fullwidth
  letters and any other two-cell grapheme; CJ small kana; iteration marks; IN; the CJK hyphens; HY / BA; SHY;
  ZW; GL / WJ; CL / CP / EX / IS / NS; CJK centered punctuation; OP) and `break_between(before, after,
  BreakRules)`, the one rule the packer asks before each text grapheme (`intake::take_opportunity`, replacing
  the width-2 / hyphen special cases): no break before closing punctuation or after opening punctuation,
  `break-all` reads letters as ID, `keep-all` refuses letter–letter pairs whatever `line-break` says, `line-break`'s
  §5.3 rules apply where both sides are CJK (no `lang`); `anywhere` breaks everywhere. `strict` vs `normal`: the
  spec now forbids breaks before small kana for both, so they differ only before `〜` `゠` — followed as
  written. BK / NL characters force a break (`white_space::classify`). Soft hyphens are kept in the word buffer,
  zero cells wide (`GraphemeKind::SoftHyphen`); a word ending in one reserves the hyphen's cell when placed
  (a conservative rule, DIVERGENCES), and a line broken there (soft, not at a following space) shows `-`
  (`packer/fragments.rs::show_hyphen`). That rendering is the first where a fragment's painted text is not its
  source, so at the root: `InlineFragment` carries a crate-private `SourceMap` (`inline/source_map.rs`: per
  source grapheme, its source and text bytes; `None` when the text is the source), with `source_len()`
  (public), `units()`, `cells_before_source`, `source_at_cell`; every cell↔source reader moved onto it — the
  caret (`caret.rs`, whose `cells_before_byte` is gone), hit-testing (`hit_test/fragment.rs`, whose
  `cells_to_bytes` is gone), the selection highlight, word / line movement and multi-click (`text.len()` →
  `source_len()`); the packer builds maps per placed word and merges them with a contiguous fragment's
  (`PendingGrapheme` holds `source_len`, a `Cow` text and `mapped`). C9-TAB-SIZE, C9-TEXT-TRANSFORM and
  justification reuse it. `overflow-wrap` (`packer/emit.rs::split_word`): a word that starts an empty line and
  is wider than it places as many graphemes as fit, breaking only after a grapheme whose text allows it; a
  min-content measurement (`measuring` at width 0) takes `anywhere`'s breaks only (§5.5); `word-break:
  break-word` is `normal` + `anywhere` (`RunStyle::of`). The packer split again: `packer/fragments.rs` (fragment
  building, maps, the hyphen; `emit.rs` 373 → 346). Red: `css_phase9/breaking.rs` — 10 of 10 failed (`abcdef`
  unbroken under `break-all`; `日本語の` / `文章` under `keep-all`; `。` starting a line; `bbbbbbbb` overflowing
  under each `overflow-wrap` form; min-content 6 for 1; `hyphen` for `hy-` / `phen`; the caret at 4 on row 0;
  `abc` / `def` unbroken at `line-break: anywhere`, a zero-width space and U+2028; `〜` breaking under
  `strict`); unit tests `breaking.rs` and `source_map.rs` written with the code. Green after two corrections
  to the tests themselves, against the spec: `break-all` puts `g` beside `ef` (`ef g` / `hi`, greedy), and a
  caret at the byte between two lines broken with no space takes the first line's end (existing behaviour
  between ideographs, now documented) — the test asserts the next byte on line 2. Mutation (restored,
  touched): no emergency split → two tests; no shown hyphen → two; closing punctuation breakable → one;
  `anywhere` off → one. No existing test expectation or snapshot changed.
- 2026-10-06 — C9-TAB-SIZE (CSS Text 3 §4.2, §4.1.1, §4.1.2). rdom-style: `tab-size: <number [0,∞]> | <length
  [0,∞]>` (`TabSize::Number` / `Length`, `cells()` rounding onto the grid half to even — a space is one cell,
  so both are cells), inherited, initial 8; a percentage or a viewport length has no basis and is invalid;
  serialized as a number or `<n>ch`. rdom-tui — what rdom did: a preserved tab in `pre` / `pre-wrap` /
  `<textarea>` was one space (DIVERGENCES §3), and the caret counted it as one cell. Now a preserved tab is a
  `GraphemeKind::Preserved { tab: Some(size) }` piece, laid at placement (`packer/emit.rs::layout_tabs`): the
  word starts at a known cell of a line whose start is `line_origin()` cells from the block's starting content
  edge (the floats' share of that side; the right edge under `rtl`), and each tab takes `size - pos % size`
  cells (§4.2: "lines up the start edge of the next glyph with the next tab stop ... multiples of the tab
  size from the starting content edge"; the 0.5ch rule cannot apply to whole cells), 0 at `tab-size: 0`
  ("preserved tabs are not rendered"). It renders as that many spaces through C9-BREAKING's `SourceMap` (one
  source byte, N text bytes), so the caret, hit-testing, selection and the textarea's editing caret step over
  it as one character (`the_caret_moves_over_a_tab_as_one_character`). A tab is preserved white space: it
  hangs at a soft wrap under `pre-wrap` and is a soft wrap opportunity at the end of its sequence (each one
  under `break-spaces`), as C9-WHITE-SPACE's spaces. Tabs are placed again where the word lands — after a
  wrap, and in `fit_empty_line` when a line moves below a float (found by the test: the first draft placed
  them only on a wrap, so a line moved down kept the tab width of the band beside the float, ` abcdefg` for
  `    abcdefg`). Red: `css_phase9/tab_size.rs` — 4 of 4 failed (`a b` for `a       b`, `ab cd` for `ab  cd`,
  `a b` for `ab` at `tab-size: 0`, the caret at 2 for 4); green after; the float test added after (red on the
  first draft as above, green after the fix). Mutation (restored, touched): no tab layout → five; no re-layout
  in `fit_empty_line` or after a wrap → the float test; the line origin at 0 → the float test. Changed
  expectation: none. Collapsible tabs (`normal`) are still spaces (§4.1.1).
- 2026-10-06 — C9-TEXT-TRANSFORM (CSS Text 3 §2.1 and its ordering note, CSS Text 4 §2.1, MathML Core §4.2 /
  Appendix C.1). rdom-style: `text-transform: none | [capitalize | uppercase | lowercase] || full-width ||
  full-size-kana | math-auto` as `TextTransform { case: TextCase, full_width, full_size_kana, math_auto }`
  (closed record, `NONE`), inherited, serialized in the grammar's order. rdom-tui: `render/inline/transform.rs`
  — what a text grapheme renders as, applied in the packer's intake after white space collapsing and before
  line breaking (§2.1: "after Phase I ... before Phase II"; Appendix A: it affects line breaking), so the
  breaking class is read off the rendered text (a full-width letter breaks like an ideograph) and the
  rendered width is what is measured. Case: `char::to_uppercase` / `to_lowercase` (full mappings incl.
  SpecialCasing's unconditional ones), Final_Sigma for `Σ` from a `CaseContext` (the packer's: the character
  before is cased) and a one-character lookahead within the text node, titlecase through a table of the
  characters whose titlecase is not their uppercase (`ǅ` `ǈ` `ǋ` `ǲ`, `ß` → `Ss`, the Latin and Armenian
  ligatures). `capitalize`'s word rule (DIVERGENCES §2): a letter or digit not preceded by a letter, digit or
  word-internal apostrophe. `full-width`: ASCII `!`–`~` → U+FF01–FF5E, the seven Latin-1 signs, the halfwidth
  katakana block, and a *preserved* space → U+3000 (collapsible spaces untouched, §2.1's note). `full-size-
  kana`: Text 4's table. `math-auto`: a text node of exactly one character in C.1's table (any element — rdom
  has no MathML). The rendering goes through C9-BREAKING's `SourceMap`, so `ß` → `SS` is one source unit two
  cells wide. Copy: verified against the spec (§2.1 "must not affect the content of a plain text copy & paste
  operation") and the engines — Gecko copies the source; Blink and WebKit copy the transformed text, an open
  bug since 2013 (crbug.com/325231). Decided: the spec and Gecko — the clipboard serializer reads the DOM, so
  nothing changed there; recorded in DIVERGENCES §2. Found while implementing: a cargo-fmt-reflowed call was
  not rewritten by the edit script, so the first build pushed the source grapheme — caught by the red tests
  staying red. Red: `css_phase9/text_transform.rs` — 6 of 6 failed (`Straße` for `STRASSE`, `ab 12` for
  `ａｂ １２`, `ぁっャ`, `x+cosh`, the caret at 5 for 6, `hello …` uncapitalized); green after; the
  leading-apostrophe case added after a surviving mutation. Mutation (restored, touched): Final_Sigma off,
  the titlecase table entry gone, U+3000 off, an apostrophe always word-internal — each fails its test. No
  existing test expectation or snapshot changed.
- 2026-10-06 — C9-TEXT-INDENT (CSS Text 3 §8.1, §4.2; CSS Sizing 3 §5.1 / §5.2.1; CSS Values 4 §6.1.2). rdom-style:
  `text-indent: <length-percentage> && hanging? && each-line?` as `TextIndent { length: Length, hanging,
  each_line }` (initial `Cells(0)`; `Length::Auto` is outside the grammar and indents nothing, documented on the
  field), inherited; its viewport units resolve at computed-value time with the insets' (`absolute.rs` — found
  by C2G-VIEWPORT-FIELDS' table-driven gate test, which caught `text-indent: 10vw` left unresolved). rdom-tui:
  `render/inline/indent.rs::LineIndent` decides which lines: the first formatted line, the lines after a
  forced break with `each-line`, inverted by `hanging`; the packer holds the current line's indent
  (`indented()`, then per `break_line` from its `LineEnd`) and treats it as a margin at the line box's start
  edge — `line_width()` less it (a negative one lengthens the line), the line placed past it (`start + indent`,
  or under `rtl` `band end - indent - width`), `line_origin()` past it for tab stops ("from the starting
  content edge", so a tab after an indent still lands on a stop counted from the edge). "Only lines that are
  the first formatted line of an element": an IFC block's first line; of a block container's anonymous block
  boxes, the one holding its first line-bearing content (`pack_run` reads the run's `pseudos.before`, which
  `generated::run_pseudos` computes from that same child — the block pass already computes it per run, so no
  extra walk); a block child's own lines take its own inherited indent. Intrinsic sizes add each line's
  indent (`LineBox::indent`, crate-private; `measure::widest`), a percentage against 0 (the size measured is
  its basis); `widest_run_line` takes whether the run holds the first formatted line, computed for the first
  run only. Not done (DIVERGENCES §2): anonymous flex / grid items are packed unindented. Red:
  `css_phase9/text_indent.rs` — 7 of 7 failed (no indent anywhere; `     aaa` for `   aaa  ` under `rtl`;
  intrinsic 3 for 6); green after. Mutation (restored, touched): no `each-line` after a forced break → one;
  the `rtl` indent dropped → one; every run first-formatted → one; no indent in intrinsic sizes → one; tab
  origin without the indent → one. No existing test expectation or snapshot changed.
- 2026-10-06 — C9-TEXT-ALIGN (CSS Text 3 §6.1–§6.4, §8.1; CSS Writing Modes 4 §2.1; CSS 2.1 §9.5). rdom-style: CSS
  Text 3 makes `text-align` a shorthand of `text-align-all` and `text-align-last` ("values other than
  justify-all or match-parent are assigned to text-align-all and reset text-align-last to auto") — followed
  as written, though engines keep `text-align` a longhand (DIVERGENCES §2): `TextAlign` (7 keywords,
  `physical(rtl)`), `TextAlignLast` (`auto` + those), `TextJustify` (`distribute` a parse-time alias of
  `inter-character`), the shorthand serialized when its longhands are one of its forms; all inherit;
  `match-parent` computes in the cascade (`cascade/text.rs::finalize_text_align`: the parent's value with
  `start` / `end` made physical by the parent's `direction`, `start` on the root — an element with no element
  parent — for elements and `::before` / `::after`). rdom-tui — what rdom had: no `text-align`; lines started
  at the start edge (the `rtl` shift in `break_line`, C8-RTL-LINE-OVERFLOW). At the root: that shift is
  replaced by `render/inline/align.rs::place_line`, the one placement of a settled line — the line box is the
  band floats leave it (C8-FLOAT's `LineExclusions`) less the `text-indent` at its start edge; the line's
  alignment is `text-align-all`'s, or for its last line and a line before a forced break `text-align-last`'s
  (`auto`: `start` for `justify`); `start` / `end` follow `direction` (closing C5-WRITING's note); an
  overflowing line is start-aligned (§6.1), so the C8-RTL-LINE-OVERFLOW behaviour is the `start` case.
  `center` rounds the leading space down; `justify` collects the line's opportunities in visual order across
  fragments, generated runs and atoms (`Opportunities`), hands each `free / n` cells and the first `free % n`
  one more, and widens the unit before each opportunity with trailing spaces — through C9-BREAKING's
  `SourceMap`, so the caret and hit-testing map a justified cell to its separator
  (`justified_cells_map_back_to_the_source`). `auto` = separators + gaps beside a two-cell character; a line
  with a preserved tab is not justified (§6.1's allowance; tab stops stay aligned); an intrinsic measurement
  skips placement. The conditional hang before a forced break now measures overflow against the line box
  (band less indent). Red: `css_phase9/text_align.rs` — 7 of 8 failed (every line at the start edge;
  `abc de` unjustified; `ab` left in the `match-parent` child); the overflow test passed before and after (the
  old start placement); the source-map test added after (green; mutation-checked). Mutation (restored,
  touched): `center` rounding up → two; no remainder → two; `text-align-last: auto` not falling back to
  `start` → two; `auto` without CJK gaps → one; overflow not start-aligned → one; no justified source map →
  one. No existing test expectation or snapshot changed.
- 2026-10-06 — C9-TEXT-WRAP (CSS Text 4 "Joint Wrapping Control: the text-wrap shorthand", "Selecting How to
  Wrap: the text-wrap-style property"; the current draft adds `avoid-short-last-line`, implemented with
  `pretty`'s rule). rdom-style: `text-wrap: <'text-wrap-mode'> || <'text-wrap-style'>` (omitted longhand
  initial, shortest serialization — `nowrap pretty`, `balance`), `TextWrapStyle`, inherited. rdom-tui —
  decided at the root: the packer streams (it places each word as its soft wrap opportunity arrives), so a
  wrap style that compares layouts needs the content again; instead of walking and measuring the tree again
  the packer records its intake (`packer/replay.rs::Op`: each text borrowed from the DOM with its `RunStyle`,
  each atom with its measured width and rows, forced breaks, `<wbr>`, floats) and a replica — same width,
  direction, indent, alignment — replays it with `WidthCaps` on its lines (per group of lines between forced
  breaks, and on one line), which `line_width()` reads; the packer tracks each line's group. `render/inline/
  wrap.rs`: `auto` / `stable` are the greedy packing (it never looks ahead — `stable`'s definition — pinned
  by `stable_keeps_earlier_lines_when_text_is_added`, which `balance` fails as expected); `balance` bisects,
  for every group of 2–6 lines at once (`MAX_BALANCED_LINES`, Chromium's limit; the spec: "may treat this
  value as auto if there are more than ten lines"), the narrowest cap keeping the group's greedy line count
  ("must not change the number of line boxes" for ≤5 lines), one replay per step plus the final one; `pretty`
  / `avoid-short-last-line` cap the line before a one-word last line one cell short of its content, kept only
  when the count holds (the exact rule in DIVERGENCES §2). Balanced lines keep their alignment in the whole
  line box (`align.rs` reads the band, not the cap). Floats: a paragraph that met a float or had a line beside
  one keeps its greedy breaks (a replay would place the floats into the area again). Cost, pinned by
  `inline/wrap_cost_tests.rs` (a test-only grapheme counter in the intake): a paragraph past six lines is
  packed once (the same count as `auto`); a short one at most (1 + ⌈log2 20⌉ + 1) times its text; 60 groups
  between forced breaks the same bound — the groups are bisected together, so it is not multiplied by their
  number; `pretty` at most twice. Red: `css_phase9/text_wrap.rs` — 4 of 5 failed (`aa bb cc dd ee` / `ff` for
  the balanced lines, alone and per group; `ddd` alone under `pretty`; the first row unchanged under
  `balance`); the `nowrap balance` test passed before and after. Green after. Mutation (restored, touched):
  no six-line limit → the long-paragraph cost test; `pretty` keeping a layout with more lines → the revert
  test (added after this mutation survived); bisection accepting one extra line → two. No existing test
  expectation or snapshot changed.
- 2026-10-06 — Phase 9 part 1 complete (C9-WHITE-SPACE, C9-BREAKING, C9-TAB-SIZE, C9-TEXT-TRANSFORM,
  C9-TEXT-INDENT, C9-TEXT-ALIGN, C9-TEXT-WRAP). The inline packer now has a module per concern: white space
  processing (`white_space.rs`, `run_style.rs`), breaking (`breaking.rs`), text-transform (`transform.rs`),
  the indent (`indent.rs`), alignment and justification (`align.rs`), wrap styles (`wrap.rs`), and the
  source map every rendering that is not its source goes through (`source_map.rs`); `packer/` split into
  `intake.rs`, `emit.rs`, `fragments.rs`, `replay.rs`. Part 2 (C9-LINE-HEIGHT, C9-VERTICAL-ALIGN,
  C9-DECORATION, C9-FONT) is next; the Phase 9 gates run after it.
- 2026-10-06 — C9-LINE-HEIGHT (CSS Inline 3 §5.1; CSS 2.1 §10.8, §10.8.1; CSS Values 4 §6.1.1), closing C2-LH.
  rdom-style: `line-height: normal | <number [0,∞]> | <length-percentage [0,∞]>` as `LineHeight` (`Normal`,
  `Number`, `Rows` — a length known at parse time, serialized in `ch` — and `Calc`, a percentage or a length in
  a context unit, which computes to `Rows`), in the CSS Text group (`TextStyle::line_height`, inherited whole —
  a number as the number, a length as its rows), `parse/values/inline.rs`. Decided mapping: the font is one row,
  so `normal` and `1` are one row, a number that many rows, a percentage that share of one; `rows()` rounds
  ties to even (every fractional length's rule, DIVERGENCES §1) and is at least one — the glyph's own row, where
  a browser lets lines overlap (DIVERGENCES §2). Half-leading (§10.8.1): `half_leading()` puts `floor((L − 1) /
  2)` rows above the glyph row and the rest below — the odd row below. C2-LH at the root: `lh` / `rlh` are
  context units like the viewport units (`CalcUnit::needs_context`), so `length_percentage` keeps them and the
  cascade makes them absolute; `UnitContext` (viewport, `lh`, `rlh`) generalizes the viewport resolution
  (`CalcExpr::absolutize_in`, `ComputedStyle::resolve_context_units`; the old entry points keep their meaning
  with one-row line heights). The cascade computes `line-height` first (`cascade/text.rs::finalize_line_height`:
  its `lh` the parent's, its `rlh` the root's — the initial one row on the root), then resolves every other
  length with the element's own `lh` and the root's `rlh` (`root_line_height`, the document element's). A
  registered custom property's `lh`, computed before `line-height`, is one row (documented). `tab-size`, which
  takes only parse-time lengths, no longer takes `lh` (as it takes no viewport unit). rdom-tui — decided at the
  root: the packer tracks the inline boxes (`packer/frames.rs`): an arena of frames for the formatting context,
  each an inline box's rows above and below its baseline row (`BoxRows::of(line_height)`), the strut frame 0
  (`with_strut`, the block's line height, `packer_for` / `pack_generated`); the feed enters a frame per inline
  element (`walk_inline_box`), per `::before` / `::after` text and per anonymous inline box of a `display:
  contents` element's text (`Op::Enter` / `Op::Leave`, so `text-wrap-style` replays them); every grapheme's
  `Origin` names its frame, and placing content on a line marks the frame and its ancestors; `settle_line`
  takes the marked frames' extent beside the atoms', and the next line starts with the strut. Baselines:
  `inline/baselines.rs` — an inline block's (and a flex / grid item's) first / last baseline is its first /
  last line's glyph row: the content rows less the leading around them (`insets`: from the inline layout of
  inline content, through the first / last in-flow block child's padding and border in a block container;
  skipped where no line can be taller than a row, `has_tall_lines`, so default content costs one subtree walk
  more, no pack); an anonymous flex item takes its packed lines' (`InlineLayout::baselines`). Consumers already
  read the line geometry (C5G-ATOM-BOX): intrinsic heights, `line-clamp`, scroll extents, the caret's row. Hit
  testing: a click on a leading row resolves to that line's text at its column (`fragment_at_layout`: a text
  fragment covers its line's rows, an atom its own), but targets the block — an inline element's box is its
  glyph row (`descend::hit_fragment` unchanged), as browsers. Not done: vertical-align (next item). Red:
  `css_phase9/line_height.rs` — 8 of 8 failed on HEAD (`line-height` an unknown property, the strict sheet
  rejected); with the cascade in and `settle_line` ignoring the line heights (the old layout) 8 of 9 failed
  (one-row lines everywhere; the inline block's `y` on its content's last row, `["   ", " y ", "x z"]`; the caret
  at row 1), the `lh` / `rlh` test passing (cascade only). Green after; then the flex-item and scroll-extent
  tests. Found while testing: a click on a leading row fell to the clamp's "between fragments" fallback (the
  line's end) — fixed in `fragment_at_layout` as above. Mutation (each alone, restored, touched): `settle_line`
  ignoring line heights → 8; no ancestor marking → 1; the odd row above → 2; no baseline insets → 1; `lh` in
  `line-height` not the parent's → 1; `rlh` the element's own → 1; leading rows not hitting text → 1; no frame
  for pseudo text → 1; none for `display: contents` text → 1; anonymous items' baselines from content rows → 1.
  Changed expectation: `numeric_tests::lh_units_parse_as_one_row` became `lh_units_wait_for_the_line_height`
  (`2lh` is an expression until the cascade). No snapshot changed.
- 2026-10-06 — C9-VERTICAL-ALIGN (CSS 2.1 §10.8, §10.8.1; CSS Inline 3 §4; HTML §15.3.4). rdom-style:
  `vertical-align: baseline | sub | super | text-top | text-bottom | middle | top | bottom |
  <length-percentage>` as `VerticalAlign` (`Rows` a length of either sign, `Calc` a percentage or a context
  length, which computes to `Rows` against the element's own line height — `cascade/text.rs` after
  `line-height`), a top-level field (not inherited), dispatched with `line-height` in
  `property_dispatch/inline.rs` (new; `line-height`'s arms moved there from `text.rs`); the UA sheet gives
  `sub` / `sup` their alignment and `line-height: normal`. Decided: CSS 2.1's single property — Inline 3's
  shorthand split (`alignment-baseline`, `baseline-shift`, `baseline-source`) stays N/A, one baseline per row
  (DIVERGENCES §1 / §2). rdom-tui — at the root, the line-height frames carry the alignment
  (`packer/frames.rs`): each frame resolves its `BoxAlign` against its own rows when it is entered — a raise
  from its parent's baseline (`sub` / `super` one row, the parent's sub/superscript position on a grid;
  `middle`: its middle row — the upper of two — on the parent's baseline row; `text-top` / `text-bottom`: its
  top / bottom on the parent's glyph row, the content area) accumulated down the tree, or a new aligned subtree
  for `top` / `bottom`; an atom is a frame too (`atom_frame`: its margin box's rows, its own alignment). Marking
  a frame adds its extent to its subtree's; `settle` makes the line from the strut's subtree and grows it for a
  taller `top` (downward) / `bottom` (upward) subtree — CSS 2.1 leaves the placement to minimize the height;
  Blink's choice — and gives each subtree's baseline row; `vertical::settle_line` places every fragment at its
  frame's row (`InlineFragment::frame`, `GeneratedFragment::frame`, crate-private; `GeneratedFragment::y`, new,
  public). Consumers that read the line's baseline row for text moved to the fragment's: paint (text, generated
  text, links, the selection overlay), the caret; hit-testing already read `line.top + fragment.y`. Table
  cells: the table builtin does not read it (no table formatting context yet) — C13-TABLE-PROPS. Red:
  `css_phase9/vertical_align.rs` — 6 of 6 failed with the property parsed but not applied (every run on the
  baseline: `["abc", "   ", "   "]` for the super / sub / length shifts, the atom's `B` on row 2 for row 0
  under `top`, …); green after. The `sub` / `sup` UA test after (red `["x2i", …]`, green with the UA rules).
  Mutation (each alone, restored, touched): `super` no shift → 2; shifts not accumulated → 2; `middle`
  unshifted → 1; `text-top`'s sign → 2; a `bottom` subtree growing the line downward → 1; a `top` subtree on the
  root's baseline → 2; a percentage of one row → 1; text painted on the line's baseline row → 4; the caret on
  it → 1; generated text unplaced → 1; atoms ignoring their alignment → 2; the UA `sup` rule gone → 1. Changed
  expectation: `vertical.rs`'s unit test now places fragments from frame rows (the line arithmetic moved to
  `frames.rs`'s tests). No snapshot changed.
- 2026-10-06 — C9-DECORATION (CSS Text Decoration 4 §2.1–§2.6, §3.2, §4.1, §4.2; Text Decoration 3 §2; ECMA-48 /
  ITU T.416 SGR; kitty's underline extensions). rdom-style: `text-decoration` is the Level 4 shorthand of
  `text-decoration-line` (`TextDecorationLine`: `underline || overline || line-through || blink`),
  `-style` (`TextDecorationStyle`), `-color` (a `TuiColor`, initial `currentcolor`) and `-thickness`
  (`TextDecorationThickness`, a `PaintLength` so `2px` / `0.1em` parse as the decorating properties' pixel
  lengths do), any order, serialized shortest; `TuiStyle::text_decoration` is their group
  (`TextDecorationDeclarations`), not inherited; `text-underline-offset`, `text-underline-position` and
  `text-decoration-skip-ink` are inherited, in the CSS Text group — all four placement properties parse,
  cascade, resolve their viewport units and change nothing drawn (DIVERGENCES §1). Breaking — the old
  single-keyword field and `parse_text_decoration`'s pair (CHANGELOG, API table, `text_decoration_hints`). The
  `TextDecoration` enum stays as the builder's one-line form. rdom-tui — decided at the root: the computed
  decoration is a `TextDecorations` group (the color resolved with the other colors, `cascade/colors.rs`, an
  undeclared one `currentcolor`), and propagation (§2.1: "propagated to all in-flow children", "not ... to
  the contents of atomic inline-level descendants", "nor to out-of-flow descendants", drawn "with the
  decorating box's color and style") is a derived used value, `ComputedStyle::applied_decorations`, computed
  once per element and pseudo-element in the cascade (`cascade/text_decoration.rs::finalize_applied_
  decorations`, after `display` / `float` / `position` are final) from the parent's — so paint reads one field
  per fragment and the old `UNDERLINED` / `CROSSED_OUT` modifier bits on the computed style are gone (they made
  `text-decoration` reach only the element's own text, the opposite of §2.1). A cell holds one line of each
  kind: the innermost decorating box's wins (DIVERGENCES §2). Paint (`paint_pass/text.rs::text_modifiers`): the
  underline's `Modifier::UNDERLINED` plus its style bit (`UNDERLINE_DOUBLE` / `_CURLY` / `_DOTTED` / `_DASHED`,
  new) and, where it is not the text's color, `Cell::underline_color` (new; `Style::underline_color`, the
  translucent composite and the background fill carry it); `OVERLINED` (new), `CROSSED_OUT`, `SLOW_BLINK`.
  Emission stays in the backend layer: `render/sgr_capabilities.rs::SgrCapabilities` (`styled_underline`,
  `underline_color`, `overline`; `BASIC`, `EXTENDED`, `from_env` / `detect` over `TERM`, `TERM_PROGRAM`,
  `VTE_VERSION`, `KITTY_WINDOW_ID`) — rdom had no capability model (Phase 3 queries the background, OSC 11,
  and the theme, mode 2031, but nothing about SGR); `sgr::emit_sgr_transition_for` writes `4:n`, `58:2::r:g:b`
  / `58:5:n` / `59` and `53` / `55` only where the capabilities allow, else a plain `4` (the colon forms are
  what an older terminal misreads); the backends hold their capabilities (`with_sgr_capabilities`, `BASIC`
  by default; `App` detects), `BackendState` stays plain. `VirtualScreen` reads the colon and semicolon forms.
  The UA rules keep their `.text_decoration(…)` builder calls. Red: `css_phase9/text_decoration.rs` — 3 of 3
  failed with the properties cascaded and paint not reading them (`modifier: Modifier(0)` on every cell); the
  `VirtualScreen` test (`virtual_screen/tests/terminal.rs::decorations_emit_by_the_terminals_capabilities`)
  written with the emitter. Green after. Found by the C2G viewport gate test: `text-decoration-thickness:
  10vw` (and the offset) kept its viewport unit — now resolved in `absolute.rs`. Mutation (each alone,
  restored, touched): nothing propagated → 1; atoms or out-of-flow boxes receiving decorations → 1 each; no
  underline color → 2; `wavy` as `double` → 1; the styled underline, the underline color or the overline
  ungated, or the basic underline not emitted → 1 each (the `VirtualScreen` test); the outermost line winning
  → 1 (the `AppliedDecorations` unit test, extended after this mutation survived); `text-decoration-color`'s
  initial value not `currentcolor` → 1 (the propagation test now declares the line alone, after this mutation
  survived). Changed expectations: the cascade tests that read `UNDERLINED` / `CROSSED_OUT` on the computed
  style read `applied_decorations`; `text_decoration_does_not_inherit_to_children` became
  `text_decoration_does_not_inherit_but_propagates` (the child's own line stays `none`, its text is
  underlined — CSS §2.1, where the old test pinned the opposite); `text_decoration_inherit_copies_parents_
  bits` reads the line; rdom-css's property tests read `text_decoration.line`; the inherited-set probe covers
  the eight names. No snapshot changed.
- 2026-10-06 — C9-FONT (CSS Fonts 4 §2.1–§2.5, §3.7, §6.11's CSS 2.1 form). rdom-style: the font properties are a
  group, `TuiStyle::font` (`FontDeclarations`) / `ComputedStyle::font` (`Font`), inherited whole: `font-weight`
  (`FontWeight`: `normal`, `bold`, `bolder`, `lighter`, `<number [1,1000]>` — a literal outside the range invalid,
  a math function clamped; computed to a number), `font-style` (`FontStyle`, `oblique <angle [-90deg,90deg]>?`),
  and, parsed, cascaded and serialized but drawing nothing, `font-size` (`FontSize`: the keywords, `math`, a
  length — a `PaintLength`, so `16px` / `1.2em` take the decorating properties' pixel rule, `em` = 16px, not
  `font-size`), `font-family` (`FontFamily`: names as written, or a system font), `font-stretch` with its new
  name `font-width` (`FontStretch`), `font-variant` in CSS 2.1's form (`normal | small-caps`, the `font`
  shorthand's). The `font` shorthand takes the full grammar — the prefix in any order with `normal`, the size,
  `/ line-height`, the family list, or a system font keyword — and sets the six longhands and `line-height` (reset
  to `normal` when omitted: §3.7 "resets ... line-height"); it serializes shortest, or as its system keyword.
  Breaking — `TuiStyle::bold` / `italic` (`bool`) are gone (CHANGELOG, API table, `font_hints`); the `bold()` /
  `italic()` builders and `ImportantMask::BOLD` / `ITALIC` (now aliases of `FONT_WEIGHT` / `FONT_STYLE`) stay.
  Not parsed (N/A, DIVERGENCES §1): `font-variant-*`, `font-synthesis*`, `font-kerning`, `font-feature-settings`,
  the other font-tuning properties — the coverage audit classes them N/A, not Missing. rdom-tui:
  `cascade/font.rs` applies the group (out of `apply.rs`, which lost its modifier-bit applicator — 511 → 472, after C9-DECORATION's 566 → 511) and
  `finalize_font` computes the weight against the parent's (§2.2's relative-weight table) and derives
  `ComputedStyle::modifiers`' bold and italic from the font. Decided (DIVERGENCES §2): bold (SGR 1) from 600 —
  `bold`, `bolder` from 400, and 600–1000; lighter weights draw normal, not faint: SGR 2 dims the color rather
  than thinning strokes (body text at `300` would read as disabled), and rdom dropped SGR 2 for that reason
  (T8); `oblique` is italic except at `0deg`. Red: `css_phase9/font.rs` — 5 of 5 failed on HEAD (the strict sheet
  rejected `font-weight: 100`, `font-style: oblique`, `font`, `font-size`); green after; the dispatch tests
  (`property_dispatch/font_tests.rs`) written with the parsers. Found by the C2G viewport gate test:
  `font-size: 10vw` kept its unit — resolved in `absolute.rs`. Mutation (each alone, restored, touched): bold from
  700 → 1; relative weights against 400 → 1; `oblique` upright → 1; `lighter` than 800 at 400 → 1; `oblique 0deg`
  italic → 1; the shorthand keeping `line-height` → 2; the weight's range unchecked → 1; the shorthand serialized
  without its line height → 1. Changed expectations: the tests that read `style.bold` / `italic` read the font
  group (rdom-style UA and stylesheet tests, rdom-tui node tests, rdom-css property tests); the inherited-set probe
  and the `initial` test cover the group. No snapshot changed.
- 2026-10-06 — Phase 9 part 2 complete (C9-LINE-HEIGHT closing C2-LH, C9-VERTICAL-ALIGN, C9-DECORATION, C9-FONT):
  all eleven Phase 9 rows done. The inline packer gained `packer/frames.rs` (the inline boxes: line heights,
  alignment, aligned subtrees) and `inline/baselines.rs`; the decorations are a derived used value in the cascade
  and capability-gated SGR in the backend (`render/sgr_capabilities.rs`); the font is a cascade group
  (`cascade/font.rs`). CSS-COVERAGE §3.12–§3.14 have no Partial or Missing row left; ACID tile 8 covers the
  Phase 9 text features. The Phase 9 gates (architect + API, with the C8G re-review) are next.
- 2026-10-06 — Phase 9 gates (with the C8G re-review: 14 of 15 at the root; C8G-SNAP-TALL incomplete).
  Architect: 3 blocking — length-changing transforms classify the rendered grapheme (`ß`→`SS` taken as
  an ideograph, so uppercase words break mid-word and min-content shrinks); a covering snap range's end
  is never offered (25-row cards in a 10-row list skip rows 20–24); two baseline models (`content_rows`
  + `insets`, gated on line-height only, missing `vertical-align` and leading anonymous text). API: 2
  blocking — UA form controls do not reset `line-height`, so a page-level `line-height` breaks one-row
  inputs and textareas (inferred); the 575 split rule broken in range (`layout/text.rs` 586,
  `builder/mod.rs` unlisted). Non-blocking: allocation in hot paths (`FontFamily` Vec cloned per
  element, per-word `units`, per-line Vecs, boxed `SourceMap::units`, per-grapheme `map_chars`,
  quadratic `source_len` on merge, quadratic `balance` count); `first_child_has_clearance` builds a
  `box_sequence` per block and misses floated `::before`; floated pseudos never settle; `SgrCapabilities`
  (KITTY_WINDOW_ID overrides a `screen` TERM, no builder, no `App` override, undocumented);
  VirtualScreen plain `4` keeps style bits; stale `rlh` under var restyle; focus deferral keyed on a
  thread-local, layout-run pin misses the focus relayout (6); intrinsic widths of pseudo text and bare
  text ignore transform / collapsing / tabs; `line-height: 1.5` rounds to 2 rows (preflight
  double-spaces an app); upgrade guide not ranked, three silent changes missing (0.5-dropped text
  properties now apply, `computed.modifiers` no longer carries UNDERLINED / CROSSED_OUT, `InlineFragment::text`
  is rendered text); `<sup>` doubling a line deserves a README note; DESIGN misses the Phase 9 part 2
  types, `FontVariant` open vocabulary, `FontStretch::Keyword(&'static str)`; `ImportantMask::WHITE_SPACE`
  removed needlessly; no `justify-all` builder, prelude lacks the new types; stale docs; `letter-spacing`
  not parsed; cascade depends on render code (`cascade/text_decoration.rs`). Decided: a fractional
  `line-height` floors to whole rows (leading under one row cannot be drawn; a terminal row already
  carries the font's line gap), so `1.5` is one row, `2.5` two — DIVERGENCES §1; `letter-spacing` is
  implemented in whole cells. Full reports: `target/claude-logs/c9_gate_{architect,api}.md`. Fix as
  `C9G-*`, two batches (A correctness and cost, B API and docs).
- 2026-10-06 — C9G-TRANSFORM-BREAK (architect B1). Found: `push_text_grapheme` classed a grapheme for
  line breaking by its rendered text's first character and *total* width (`class_of(first, w == 2)`), so a
  transform that renders one source grapheme as several — `ß` → `SS` under `uppercase` — read as a two-cell
  ideograph (ID) and opened a break on both sides: `<p style="width:5;text-transform:uppercase">straße</p>`
  painted `STRA` / `SSE`, min-content 4. Decision: the transform applies before line breaking (§2.1, the
  module's documented order, so full-width letters keep breaking as ID), and UAX #14 classes the rendered
  text's own graphemes (LB9) — a rendered piece takes its first grapheme's class by that grapheme's width
  (`breaking::class_of_rendered`); an untransformed grapheme is classed as before. Not classed by the source
  grapheme, which would stop full-width text breaking (`full_width_uses_the_fullwidth_forms` pins that).
  Red: `css_phase9/text_transform.rs::a_lengthened_letter_stays_a_letter_for_line_breaking` — failed
  (`["STRA    ", "SSE     "]` for `["STRASSE ", "        "]`); its Final_Sigma (`ΟΔΟΣ ΣΑ` lowercased
  breaks only at the space), `capitalize` digraph (`ǆungla` → `ǅungla`, min-content 6) and min-content 7
  assertions ride with it. `full_width_text_breaks_like_ideographs_whatever_its_length` (`aß` under
  `uppercase full-width` → `Ａ` / `ＳＳ`, min-content 4) passed on HEAD — an ID neighbour breaks either way —
  and is kept as the pin for the multi-grapheme full-width piece. Green after. Mutation (restored,
  touched): `class_of_rendered` reading the whole rendering's width → the red test fails again. No
  existing expectation or snapshot changed.
- 2026-10-06 — C9G-SNAP-COVER (architect B2; completes C8G-SNAP-TALL). Found: `pick` rested at a
  destination only inside a covering range, and `choose` saw only the aligned positions, so a page from
  inside a tall card to a destination past its range snapped to the next card: 25-row cards in a 10-row
  `y mandatory` list paged 0 → 10 → 25, rows 20–24 never shown (`tall_tests.rs` passed only because 30 is a
  multiple of the page). Decision: §6.2.3 makes every covering offset valid, the end-aligned one included —
  `points::positions` offers each covering range's two ends (no stop) beside the aligned positions, to
  `pick` and to `follow`'s mandatory fallback (the record keeps the box's aligned offset); and, decided
  beyond the spec's letter, a directional scroll from inside a covering range that would leave it rests
  first at the range's end in its direction (unless an aligned position lies between), so a 23-row card —
  whose end (13) is farther from a page's destination (20) than the next card (23) — still shows rows 20–22.
  DIVERGENCES §2's snap entry says so. Red: `scroll_snap/tall_tests.rs` —
  `page_down_reveals_every_row_of_cards_off_the_page_size` failed (rows `[20..24, 45..49, 70..74]` unseen
  for 25-row cards) and `scroll_to_just_past_a_tall_card_rests_at_its_end` failed (`scrollTo(15)` with
  23-row cards: 23 for 13); `the_wheel_reveals_every_row_of_cards_off_the_page_size` passed on HEAD (a
  one-row tick from the range's end lands on the next card with nothing skipped) and is kept as the
  wheel's pin. Green after. Mutation (each alone, restored, touched): no leaving-edge rest → the page test
  fails for 23-row cards (rows 20–22, 43–45, 66–68 unseen); no range ends among the positions → the
  `scrollTo` test fails (23). No existing expectation or snapshot changed.
- 2026-10-06 — C9G-ONE-BASELINE (architect B3). Found: two baseline models — an atom's or a flex / grid
  item's baselines were its content's first and last rows (`vertical::content_rows`, from the content
  height) moved by `baselines::insets`, which re-packed the content only when `has_tall_lines` saw a
  `line-height` above one row, and recursed through the first and last *element* children. A `sub` /
  `super` also makes a two-row line, so `a <span style="display:inline-block">H<sub>2</sub>O</span> b`
  took the atom's last row (the `2`'s) as its baseline and painted `H O` a row above `a b`; and a leading
  anonymous line (`t<div style="line-height:3">u</div>`) was skipped, so `align-items: baseline` put a
  sibling on the block child's glyph row. Decision: one model, the packed lines — new
  `layout_pass::baselines::content_rows` (memoized for the pass beside the content sizes,
  `intrinsic::memo`) measures a block container through `block::measure`'s flow (the C8G-FLOAT-MEASURE
  model; new `block::measure::baselines`): each inline run's and inline formatting context's lines
  recorded by their `text_row()`, each block-level child's baselines in turn at its placed row (or its
  lines beside this flow's floats where the flow measures it in place), each block-level `::before` /
  `::after` by its packed lines (`AnonymousItem::content_rows`, as C5G-ATOM-BOX's atomic pseudos already
  were). The inline block's last baseline (`vertical::atom_rows`), C6-ALIGN's flex / grid
  `BaselineBox` and the atomic pseudo read it; `vertical::content_rows`, `baselines::insets` and
  `has_tall_lines` are deleted. A flex or grid container's baselines stay its first and last content rows.
  DIVERGENCES §2: the inline-block entry lost its "margins and heights below the last line are not
  counted" caveat (the flow places them now), and "a box's baselines are its first and last content
  rows" is now a flex / grid container's only. Red: `css_phase9/vertical_align.rs` —
  `an_inline_blocks_baseline_is_its_last_lines_text_row` (`["  H O   ", "a  2  b ", …]` for `["a H O b ",
  "   2    ", …]`) and `a_leading_anonymous_line_holds_the_first_baseline` (`z` on row 1 for row 0) failed;
  green after. Cost pin: `intrinsic/memo_tests.rs::nested_inline_blocks_measure_each_baseline_once`
  (≤ 2 × (depth + 1) baseline walks at 4 and 12 levels). Mutation (each alone, restored, touched): no
  line recorded in inline formatting contexts or runs → 3 fail (the two red tests and
  `an_inline_block_aligns_by_each_keyword`); none in anonymous runs → the leading-line test; the
  baseline memo bypassed → the cost pin (20 walks at 4 levels). No existing expectation or snapshot
  changed.
- 2026-10-06 — C9G-UA-LINE-HEIGHT (API B1). Confirmed first (the gate inferred it): with `html {
  line-height: 2 }` a text `<input>` (one row, `overflow-x: hidden` pairing `overflow-y: auto`) had a
  `scrollHeight` of 2 — a scrollable second row — and a `<textarea>` of fixed height 4 showed two lines (`a`,
  blank, `b`, blank). Decision: HTML's UA sheets keep a control's own text layout — Chromium's `input,
  textarea, select, button { … letter-spacing: normal; word-spacing: normal; line-height: normal;
  text-transform: none; text-indent: 0; text-shadow: none; text-align: start }` with `font:
  -webkit-small-control`, Gecko's `input { line-height: normal }` — so rdom's UA sheet
  (`ua/controls.rs`) gains the rule with the properties rdom has: `line-height: normal`, `text-transform:
  none`, `text-indent: 0`, `text-align: start`, and from the font reset the weight and style (`normal`;
  size and family are inert); the button rules after it keep buttons bold. `letter-spacing` /
  `word-spacing` join the rule with C9G-LETTER-SPACING. Silent change from 0.5 (`body { font-weight:
  bold }` no longer reaches a field) — upgrade item 37. Red: `css_phase9/line_height.rs::
  form_controls_keep_a_normal_line_height` failed (input `scrollHeight` 2 for 1; the textarea `[" a", "",
  " b", ""]`); green after, with the other resets asserted (`uppercase`, `text-indent: 2`, `text-align:
  right`, `bold`, `italic` on `html` leave the controls alone). Mutation (each declaration dropped alone,
  restored, touched): each of the six fails the test. Changed expectation: `ua_total_rule_count` 163 → 167
  (the new 4-selector rule). No snapshot changed.
- 2026-10-06 — C9G-LINE-HEIGHT-FLOOR (the Phase 9 gate's decision; architect N11, API N2). Found:
  `LineHeight::rows()` rounded ties to even, as fractional lengths do, so `line-height: 1.5` — Tailwind
  preflight's `html` value, and most resets' — was two rows and double-spaced a whole app. Decision (the
  gate's): a fractional used line height floors to whole rows, at least one — number, percentage and
  length alike (`1.5`, `150%`, `1.9ch` one row; `2.5` two): leading under a row cannot be drawn, and a
  terminal row already carries the font's line gap. `rows()` is the one place (`lh` / `rlh`, the packer's
  half-leading, `vertical-align` percentages all read it); new `line_height::floor_rows` takes a value
  within a millionth below a whole row as that row (`f32` and math-function arithmetic). DIVERGENCES §1's
  `line-height` entry, the coverage row, the rdom-tui README, CHANGELOG (Changed — rdom-style; upgrade item
  33, and item 36's `font` example now `14px/2`) say so. Red: rdom-style `rows_floor_onto_the_grid_at_least_one`
  (renamed from `rows_round_…`; `1.5` → 2 for 1), rdom-tui `css_phase9/line_height.rs::
  a_reset_line_height_does_not_double_space` (`html { line-height: 1.5 }`: `["aa", "  ", "bb", "  "]` for
  `["aa", "bb", "cc", "  "]`) and `the_values_map_onto_whole_rows` (`1.5`: 4 rows for 2) failed; green
  after. Mutation (restored, touched): no epsilon → `a_hair_below_a_whole_row_is_that_row` fails. Changed
  expectations: `rows_round_onto_the_grid_at_least_one` pinned `1.5` → 2 (now 1); `the_values_map_onto_whole_rows`
  pinned `1.5` / `150%` at 4 rows for two lines (now 2), with `1.9ch`, `2.5`, `250%` added. No snapshot
  changed (no demo uses a fractional line height).
- 2026-10-06 — C9G-LETTER-SPACING (the Phase 9 gate's decision; API N8). Split first (`900571a`):
  rdom-style `layout/text.rs` (586 lines, past the split-on-touch bar) into `white_space.rs` (white space
  processing, wrapping, line breaking, `tab-size`), `text_align.rs` (transform, indent, alignment) and
  `text.rs` (`TextStyle`). Then: `letter-spacing` / `word-spacing: normal | <length>` (CSS Text 3 §9.2,
  §9.1) — rdom-style `layout/spacing.rs` `Spacing` (`Normal`, `Cells`, `Calc`; closed, DESIGN's list),
  in the inherited text group, parsed (`parse_spacing`), serialized (`ch`), absolutized with the other
  context lengths (`absolute.rs`), `cells()` the used whole cells — floored by `calc::floor_cells`, now
  shared with `line-height` (C9G-LINE-HEIGHT-FLOOR) — and never negative. Pixels, decided: **rejected**,
  with every font-relative length — spacing is geometry, and DESIGN "Pixel lengths select, cells
  measure" makes such a length invalid (`length_percentage` never yields one); a percentage (Text 4) is
  not taken either. DIVERGENCES §1 has the entry (and the sub-cell list lost the two names). rdom-tui:
  `RunStyle` carries both; new `packer/spacing.rs` — a piece's spacing is blank cells appended to its
  rendered text (`PendingGrapheme::spacing`, mapped to its source unit, so the `SourceMap`, caret,
  hit-testing, selection and copy see a unit and its spacing as one, copy the DOM text): letter spacing
  after every grapheme of text, a preserved space and a collapsed separator (`separator_width` /
  `push_separator`), word spacing after each §9.1 word separator; none after a tab, a zero-width unit, an
  atom, or inside a cursive script (Arabic, Syriac, N'Ko, Mandaic, Mongolian, Phags-pa). §9.2's "not at the
  end of a line": a word is fitted without its last grapheme's spacing (`word_fit`, `split_word`), and the
  spacing ending a line is dropped when it is settled (`drop_trailing_spacing`, unless spaces hang). The
  justification opportunities read a unit by its first grapheme (`align::unit_of`), so a widened separator
  is still one. Intrinsic sizes and `text-overflow` follow through the packer; bare text and pseudo
  content measured outside it are C9G-MISC-CORRECTNESS's. The UA form-control reset (C9G-UA-LINE-HEIGHT)
  gains `letter-spacing` / `word-spacing: normal`, as Chromium's has. Coverage: the 3.12 row N/A →
  Supported (16 / 0 / 0 / 5; totals 182 / 18 / 62 / 45), 260 property names. Upgrade item 38 (silent: the
  properties were dropped). Red: `css_phase9/letter_spacing.rs` — 10 of 10 failed (the strict sheets
  rejected both properties); green after. Mutation (each alone, restored, touched): no line-end drop → 4
  fail; no fit without the last spacing → 1 (the `a bc` fit, added after this mutation survived); no
  spacing in `split_word`'s fit → 1 (the `overflow-wrap` case, added with it); `unit_of` reading the whole
  unit → the justification test; no cursive exemption → the cursive test; no separator spacing → 4; no
  preserved-space spacing → 1; no UA `letter-spacing` reset → the form-control test. Changed expectations:
  the inherited-set probe and `PERTURB` / `initial` probe, the canonical-values table and the
  `!important`-setter test cover the two properties. No snapshot changed.
- 2026-10-06 — C9G-PACKER-ALLOC (architect N1, N2, N3). Found, by counting (red first, each): (1) an
  inherited `font-family` list (`FontFamily::Names(Vec<String>)`) was cloned into every descendant's
  computed style — 12 more allocations per plain element under `.p { font-family: system-ui,
  -apple-system, "Segoe UI", Roboto, sans-serif }` (540 for 300 per 20 elements); (2) a `units` Vec built
  for every word, mapped or not, and a temporary text `String` copied into each new fragment (81
  allocations per 40 words on a line); (3) `Frames::settle` allocated a fresh extents Vec and a baselines
  Vec per line (6 allocations a line with the above); (4) `SourceMap::units()` boxed a `dyn Iterator` — 3
  allocations for a caret, hit and unit-walk query on one fragment; (5) `map_chars` allocated per grapheme
  under a case transform even when unchanged (468 for 198 on already-upper text); (6) `append_fragment`'s
  merge summed the whole map for `source_len()` — 89 700 units for a 600-unit `pre-wrap` line of tabs; (7)
  `balance` counted each group's lines with a pass per group — 4 000 000 visits for a 2000-line `pre-line`
  log. Fixed at each root: (1) `Names(Arc<[String]>)` (rdom-style; API table "Changes to APIs added after
  0.5"); (2) a group's units built only when a grapheme in it is mapped, and `append_fragment` takes a
  `Cow` and moves an owned text into a new fragment; (3) `settle` reuses the extents buffer and `Settled`
  keeps the strut's row apart from the other subtrees' (empty, unallocated, on a plain line); (4) a
  concrete `source_map::Units` iterator; (5) `map_chars` maps through a three-char `Chars` buffer and
  allocates from the first changed character only; (6) `SourceMap` keeps its source length (the text-byte
  adjusters, `text_bytes_mut`, cannot change it); (7) one `histogram` pass per pack. Tests:
  `style/cascade/cost_tests.rs::an_inherited_family_list_is_shared_not_copied` (equal per-element
  allocations with and without the list) and `render/inline/alloc_tests.rs` (six: ≤ 48 allocations per 40
  words, ≤ 88 per 40 lines, equal allocations under `uppercase` on upper text, 0 per unit walk, ≤ 4 units
  summed per unit, ≤ 4 line visits per line); all red on HEAD as quoted, green after (40 words: 41; 40
  lines: 82). Mutation (each alone, restored, touched): units built for every group → the word and line
  tests fail; a fresh extents Vec per line → the line test. Not done: a `Calc` inside `TextIndent` still
  clones a box per element when a `calc()` indent is inherited (rare; tracked in the gate report). No
  existing expectation or snapshot changed.
- 2026-10-06 — C9G-CLEARANCE-COST (architect N4, N5). Found: (1) `first_child_has_clearance` built a
  `box_sequence` for every collapsible parent — 22 for six nested blocks with no float anywhere; (2) it
  returned `false` at a generated item, so a floated `::before` never gave the first child clearance; and,
  found while testing it, (3) `outer_edge_margin` stopped the parent / first-child chain at *any* generated
  item, a floated `::before` included, while the flow still suppressed the child's top margin inside the
  parent — `.p::before { float: left } .x { margin-top: 2 }` lost the margin (parent and `x` at row 0, for
  2). Fixed at the root: the predicate runs only where `float::measure::may_hold_floats` (the scan that
  allocates nothing, now `pub(in layout_pass)`) says a float may be, and classifies each box item by
  `float_side_of` — elements and pseudo-elements alike; the margin chain skips a floated pseudo-element as
  it skips a floated element (out of flow) and still stops at a block-level one. N5, decided after a probe
  of eleven styles (line heights, padding, `display: flex` / `grid`, `pre`, `balance`, `vertical-align`):
  a floated pseudo-element is measured (`FloatBox::of` → `AnonymousItem::contribution`) and laid out
  (`AnonymousItem::lay_out` at that border box) by the same packing, so its laid-out height is its placed
  height and its exclusion needs no settling — no code change; the Phase 8 Log's "every float is settled
  by its index" holds for elements, whose automatic height layout resolves, and pseudo-elements keep the
  one measured box. Red: `idle_cost_tests.rs::clearance_is_not_scanned_without_floats` (22 for 0) and
  `css_phase8/float/clear.rs::a_floated_before_does_not_stop_the_collapse` (`(0, 0)` for `(2, 2)`) failed;
  `a_floated_before_gives_the_first_child_clearance_too` passed on HEAD by accident (the lost margin and
  the clearance land `x` at row 3 either way) and now holds for the right reason. Green after; pin added:
  `css_phase8/pseudo_atoms.rs::a_floated_pseudos_exclusion_is_its_laid_out_box`. Mutation (each alone,
  restored, touched): no `may_hold_floats` gate → the scan test; floated pseudos ignored for clearance →
  the clearance test; the chain not skipping them → the collapse test. No existing expectation or snapshot
  changed.
- 2026-10-06 — C9G-MISC-CORRECTNESS (architect N7, N8, N9, N10, N13's layering). Five fixes, each red
  first. (1) Stale `rlh` (N8): a restyle (`walk::Mode::Restyle`) kept an element whose own style stayed and
  skipped its subtree, but `rlh` below reads the root's line height — `html { line-height: 1 → 3 }` over
  `.card { line-height: 1 } .card p { margin-top: 1rlh }` left `p` at 1. Fixed as viewport units are: a
  restyle that moves the root element's used line height sets `Scratch::root_line_height_moved`, and no
  element keeps its subtree for the rest of that pass. Red: `var_tests.rs::
  a_restyle_of_the_roots_line_height_reaches_rlh_below_a_kept_element` (1 for 3). (2) Focus deferral (N9):
  keyed on the `timers::in_app` thread-local, so a second, bare document focused from an `App` handler
  recorded a `PendingFocusScroll` nothing serviced. Now document data the `App` sets on its own document
  (`focus::laid_out_by_app`, at construction); `in_app` is gone (its only reader). Red:
  `focus/scroll_tests.rs::a_bare_document_focused_inside_an_app_handler_scrolls_at_once` (0 for 7). The
  frame pin (`app/layout_runs_tests.rs`) now has a focus scroll too: it ran 6 `layout_dom` rounds (3 + the
  re-snap's + the focus scroll's + the reveal's). Brought down: the three post-layout services run against
  the first layout — each already corrects for offsets moved since (`scrollbar::state::laid_out`) — and
  share one relayout, so 4 (≤ 2 × `MAX_ROUNDS`, was 3 ×); red 6 for 4, every scroll still landing (the
  snap at 5, the textarea at 1, the list at 4). (3) Intrinsic widths (N10): `pseudo_content_width` and
  `intrinsic_text` took raw `unicode-width` widths. Now the packer's measuring mode: new
  `inline::widest_pseudo_line` (a pseudo-element's own inline content packed alone, under the measurement's
  constraint) and `inline::text_node_extent` (a text node packed alone by its parent's values: widest line,
  rows at the budget). Red: `css_phase9/letter_spacing.rs::generated_text_is_measured_as_it_is_laid_out`
  (`straße` uppercased 6 for 7; letter-spaced, collapsed and tabbed cases with it) and
  `layout_pass/tests.rs::a_text_node_is_measured_through_the_packer` (12 for 13). List markers riding a
  descendant's line keep their raw width (Phase 10's markers). (4) `VirtualScreen` (N7): a plain `4` kept
  the underline style bits; it now clears them (ECMA-48: a single underline). Red: `virtual_screen/tests/
  parser.rs::a_plain_4_resets_the_underline_style`. (5) Layering (N13): `cascade/text_decoration.rs` called
  `render::box_tree::is_atomic_inline`, and `cascade/walk.rs` `render::box_tree::is_contents`. The predicate
  is now `ComputedStyle::is_atomic_inline` (rdom-style; every caller uses it, the render copy deleted), and
  the walk reads `display: contents` off the computed style. Red: `style/layering_tests.rs::
  the_cascade_does_not_depend_on_render_code` (a source scan of `style/`'s production modules: 2 hits).
  Mutation (each alone, restored, touched): no root-line-height flag → the `rlh` test; deferral on every
  document → 3 focus tests; the `App` not marking its document →
  `focus_in_a_handler_scrolls_against_the_next_layout`. Changed expectation: the frame pin's 5 runs → 4
  with a focus scroll added (`a_frame_runs_layout_at_most_twice_the_round_cap`). DIVERGENCES §2's focus entry
  says the deferral is per document. No snapshot changed.
- 2026-10-06 — C9G-PSEUDO-CLAMP (batch B; a probe of batch A's: a pseudo-element's `line-clamp: 2` showed
  all five lines). Found: a `::before` / `::after` box packs its generated text through its box-tree item
  (`items::AnonymousItem`), which never read the clamp; an element's clamp is a walk keyed by `NodeId`
  (`line_clamp::clamp_point`) that a pseudo-element has none of. Decision: the clamp at the generated
  box's root — `AnonymousItem::pack_clamped` (new `line_clamp::clamp_lines`) keeps the first N line boxes of
  a pseudo-element that is a line-clamp container and marks the last (`LineBox::ends_clamp`), for its
  block size, its baselines and its laid-out lines alike (block-level, inline-block, float and flex-item
  pseudos all lay out through it); its inline sizes read the whole content (min- / max-content are not
  clamped). The cut lines are not kept — generated content has no DOM position for a caret or a copy to
  reach, so nothing reads them — and a host's own clamp counts only the lines kept. Paint takes the
  pseudo-element's own marking (`Marking::of_generated`): the `block-ellipsis` of the marked line, or of
  the line its host's clamp ends at, and — a fix riding with it — its own `text-overflow` (a block
  pseudo's lines were marked by its host's, which does not apply to them, §3). Red:
  `css_phase8/line_clamp.rs` — `a_block_pseudo_element_clamps_its_lines` and
  `an_atomic_or_floated_pseudo_element_clamps_its_lines` failed (`["one two ", "three   ", "four    ", "X
  …"]`); green after (the float case first expected `X` below the float — it wraps beside it, correctly,
  and the test was corrected to assert that and the clamped box's clearance). Mutation (restored,
  touched): an atom's or float's lines painted with no marking → the atomic test fails. List markers'
  raw widths go to C10-LIST-ITEM (its row says so). No existing expectation or snapshot changed.
- 2026-10-06 — C9G-SGR-CAPS (API N1, architect N6). Found: `App::new` hard-coded
  `SgrCapabilities::from_env()` and the backends' `with_sgr_capabilities` takes them by value, so an app could
  not force `BASIC` (a log, a recording) or give a terminal the detection missed its extensions; the
  `#[non_exhaustive]` type had no way to build a custom set; and detection let an inherited
  `KITTY_WINDOW_ID` / `TERM_PROGRAM` win over a multiplexer — GNU screen started from kitty got `EXTENDED`,
  and screen splits `58:2::255:0:0` into faint and resets. Decisions: (1) the override follows the `App`
  builder pattern (`with_caret_blink`, `with_clipboard`): `App::with_sgr_capabilities(caps)` (the next frame
  drawn whole) and `App::sgr_capabilities()`, carried by new *provided* `Backend::set_sgr_capabilities` /
  `sgr_capabilities` methods (a no-op and `BASIC` by default, so a consumer's backend still compiles; the
  two built-in backends implement them); `SgrCapabilities` is re-exported at the crate root. (2) `const`
  builders `with_styled_underline` / `with_underline_color` / `with_overline`, so
  `SgrCapabilities::BASIC.with_styled_underline(true)` is a set. (3) Detection, conservative, checked
  against the terminals' own changelogs / the vtdn.dev SGR tables: a multiplexer wins — `STY` (GNU screen,
  no colon forms) → `BASIC`; `TERM_PROGRAM=tmux` with a version ≥ 3.2 (tmux exports both from 3.2, its
  CHANGES; it has parsed `4:x` since 2.9, `58` and `53` since 3.0, and passes each on where the outer terminfo
  has `Smulx` / `Setulc` / `Smol`, a plain underline or nothing elsewhere) → `EXTENDED`; any other sign (`TMUX`, `TERM`
  `screen*` / `tmux*`) → `BASIC`. Then all three for kitty / foot / WezTerm / Ghostty / mintty /
  `KITTY_WINDOW_ID` / VTE ≥ 0.60; styles and color for Alacritty (`TERM=alacritty` or
  `ALACRITTY_WINDOW_ID`; 0.11, no overline), VS Code ≥ 1.60 (xterm.js 4.10+) and VTE 0.52–0.59; the
  styles alone for iTerm2 ≥ 3.4 (its SGR 58 is nightly-only). Not detected, documented: Windows Terminal
  (all three from 1.20, but `WT_SESSION` has no version) and `COLORTERM` (24-bit color says nothing about
  these). README "Terminal notes" documents the table and the override (a doctest); DIVERGENCES §2's
  decoration entry says so. Red: rdom-tui `render::sgr_capabilities::tests` —
  `detection_knows_the_partial_terminals` (Alacritty `BASIC` for styles + color) and
  `a_multiplexer_wins_over_the_outer_terminal` (screen + `KITTY_WINDOW_ID`: `EXTENDED` for `BASIC`) failed
  against the old `detect` (environment injected through `detect`'s `var`, no process environment
  touched); `the_builders_make_a_custom_set` and `css_phase9/text_decoration.rs::
  the_app_takes_a_capability_override` did not compile (no builders, no `App` method, no root export).
  Green after. No existing expectation or snapshot changed.
- 2026-10-06 — C9G-UPGRADE-RANK (API N2, N3; docs). The upgrade guide's silent-change list was ranked by
  impact only to item 29, items 30–38 appended in implementation order. Re-ranked whole (41 items), widest
  first: `line-height` (4) after the three flex / box-model changes — `line-height: 2` on `body` doubles every
  paragraph; the 0.5-dropped CSS Text properties now applying (5, new); decoration propagation (6) — the UA
  `a[href]` underline now reaches a link's children; `<sub>` / `<sup>` taking a row (9, from 34); per-element
  `white-space` (12) and the form-control resets (13) beside the other layout changes; the two reader-facing
  changes (15, 16, new) after the focus rules; the font, UAX #14, tab and spacing items (24–27) with the
  other "a dropped property now applies" items of smaller reach. Added, each with what to read instead: the
  six CSS Text properties dropped in 0.5 (`text-align` with `-last` / `text-justify`, `text-transform`,
  `text-indent`, `word-break`, `overflow-wrap`, `text-wrap`) now apply; `ComputedStyle::modifiers` no longer
  carries `UNDERLINED` / `CROSSED_OUT` (`applied_decorations`); `InlineFragment::text` is the rendered text
  (`source_len()`), and `LineBox::text_row()` is the line's baseline row, not every run's. rdom-tui README:
  a note under "Inline formatting" on `<sup>` doubling a line, with the opt-out `sub, sup { vertical-align:
  baseline }` as a doctest (` 2 ` / `x  ` under the UA sheet, `x2 ` with the opt-out). No code changed; no
  expectation or snapshot changed.
- 2026-10-06 — C9G-TYPES (API N4, N5, N6; architect N13's classification). (1) DESIGN's `#[non_exhaustive]`
  classification names every Phase 9 type. Closed (a value layout, paint or a serializer must handle whole):
  `LineHeight`, `VerticalAlign`, `FontWeight`, `FontStyle`, the inert `FontSize` / `FontSizeKeyword` /
  `FontFamily` / `SystemFont` / `FontStretch` / new `FontStretchKeyword` (fixed sets a serializer writes back
  whole), the decoration values, `TextDecorations`, `AppliedDecorations` / `AppliedLine`, the groups
  `Font` / `FontDeclarations` / `TextDecorationDeclarations`, the parser records `TextDecorationShorthand` /
  `FontShorthand`, new `TextAlignKeyword`; open: `SgrCapabilities` (an options bag), and — decided —
  `FontVariant`, now `#[non_exhaustive]`: CSS Fonts 4 §6.11 extends CSS 2.1's `normal | small-caps`, rdom
  draws none of it, and a consumer can treat an unknown value as `normal` (a `compile_fail` doctest pins
  it). (2) `FontStretch::Keyword(&'static str)` → `Keyword(FontStretchKeyword)`, the eight §2.3 keywords
  typed (`keyword()`, `percent()`, `from_keyword`, `ALL` replacing `FONT_STRETCH_KEYWORDS`), so no string
  outside the grammar can be stored. (3) `ImportantMask::WHITE_SPACE` restored as `WHITE_SPACE_COLLAPSE |
  TEXT_WRAP_MODE`, like `BOLD` / `ITALIC`; the white-space Breaking bullet and API row no longer list it.
  (4) `justify-all`: not a `text-align-all` value, so not a `TextAlign` variant — new `TextAlignKeyword`
  (`All(TextAlign)` | `JustifyAll`, `longhands()`), the `text-align` shorthand's value, which
  `TuiStyle::text_align` / `_important` now take as `impl Into<TextAlignKeyword>` (a `TextAlign` as before)
  and `parse_text_align` builds through, one mapping. (5) The prelude gains `TextAlign`, `LineHeight`,
  `VerticalAlign`, `TextDecoration`, `TextDecorationLine`, `TextDecorationStyle`, `TextTransform`, `TextCase`,
  `FontWeight`, `FontStyle` (what a typical app's text builders take); `Spacing`, `TextAlignKeyword` and
  `FontStretchKeyword` are exported at the root (`Spacing` was missing there). The changed after-0.5 items
  are API-table rows with a `font_type_hints` migration group; `white_space_hints` reads
  `ImportantMask::WHITE_SPACE`. Red: `css_phase9/api_types.rs` (the prelude-only module, `justify-all`
  through the builder, the typed keywords, the mask) did not compile (21 errors: the prelude names, the
  new types, the mask); green after. No existing expectation or snapshot changed.
- 2026-10-06 — C9G-DOCS (API N7, B2's `SIZE-1` refresh; architect N12, N13's comment). CSS-COVERAGE: priority
  rows 23 (`text-transform`) and 24 (`text-indent`) marked Shipped, row 23's "copy keeps the DOM text, as
  browsers do" corrected (§2.1 and Gecko keep it; Blink and WebKit copy the transformed text — DIVERGENCES
  §1); the "Inherited-property set" row lists the whole `inherits()` table (it named 7 and called that
  complete). READMEs: rdom-tui's "Inline formatting" lists `text-align` (with `-all` / `-last`,
  `text-justify`), `text-indent`, `text-transform`, `tab-size`, `text-wrap`; rdom-style's property list gains
  the same; the root README's layout line gains `text-wrap` and the spacing properties.
  `InlineFragment::text`'s doc says the line's top row (`y` 0), not its baseline row. The stale comments:
  `inherit.rs` (children of an underlined element "render without an underline" — they are underlined by
  propagation) and the UA `abbr` comment ("solid underline, closest fidelity"), which was a stale rule too —
  decided to follow HTML §15.3.4: `abbr[title] { text-decoration: dotted underline }` (a new UA rule; `abbr`
  keeps rdom's muted color and loses the underline when it has no title). Red: rdom-style
  `ua::tests::an_abbreviation_with_a_title_is_underlined_dotted` (no `abbr[title]` rule); green after;
  `ua_total_rule_count` 167 → 168. `SIZE-1` recounted by the gate test's own rule over every production
  file: 21 between 500 and 575 (`tui_style/builder/mod.rs` 511 and `render/virtual_screen/tests/terminal.rs`
  524 added, `apply.rs` gone at 472, every figure current), none past 575, so no split. No snapshot changed
  (no demo has an `<abbr>`).
- 2026-10-06 — Phase 9 closed. The 14 gate fixes landed in two batches — A (correctness and cost):
  C9G-TRANSFORM-BREAK, -SNAP-COVER, -ONE-BASELINE, -UA-LINE-HEIGHT, -LINE-HEIGHT-FLOOR, -LETTER-SPACING,
  -PACKER-ALLOC, -CLEARANCE-COST, -MISC-CORRECTNESS; B (API and docs): C9G-PSEUDO-CLAMP, -SGR-CAPS,
  -UPGRADE-RANK, -TYPES, -DOCS. Open for the Phase 10 gate's re-review: list markers riding a descendant's
  line are measured by raw width (C10-LIST-ITEM's row); a `calc()` `text-indent` still clones per inheriting
  element; Windows Terminal is not detected (no version in its environment), so its SGR extensions need
  `App::with_sgr_capabilities`.

- 2026-10-06 — C10-LEGACY-COLON. Found: `p:before` reached rdom-core's selector parser as an unknown
  pseudo-class, so the rule was dropped with a warning; `::first-line` / `::first-letter` were rejected as
  unsupported pseudo-elements; suffixes matched case-sensitively (`::BEFORE` dropped). Decision: the
  stylesheet's suffix stripper (`selector_text::extract_pseudo_suffix`) owns the mapping — Selectors 4 §15's
  four legacy spellings strip to their targets after the double-colon forms (an escaped colon, `.a\:before`,
  stays an identifier), every name compares ASCII case-insensitively, and `PseudoElementTarget` gains
  `FirstLine` / `FirstLetter`: their rules are stored and match nothing until C10-FIRST (DIVERGENCES §3).
  rdom-core's `querySelector` still rejects every pseudo-element (unchanged). Red: rdom-style
  `stylesheet::tests` — `legacy_single_colon_pseudo_elements_are_pseudo_elements` (`("p:before", None)`),
  `first_line_and_first_letter_parse_and_names_ignore_case` (`Err`) and
  `a_legacy_pseudo_element_has_pseudo_element_specificity` failed; `an_escaped_colon_is_no_legacy_pseudo_element`
  passed before (a guard); `css_phase10/legacy_colon.rs` — both failed (the strict sheet warned). Green after.
  CSS-COVERAGE: the legacy row Missing → Supported, §3.16 5 / 1 / 4 / 6, total 183 / 18 / 61 / 45. No
  existing expectation changed (`extract_rejects_unsupported_pseudo_element` now uses `::grammar-error`).
- 2026-10-06 — C10-CONTENT. Found: `content` took strings, `counter()` and (substituted) `attr()` only;
  `counters()`, the `<quote>` keywords and alt text dropped the declaration; `var()` already worked (the
  general substitution runs first — the coverage row was stale; pinned now). Checked the engines for
  `content` on an element: none implements §2's `<content-list>` replacement (Chromium, Gecko and WebKit
  replace an element only with an image; MDN: "you can't use it to replace a string in an element with
  another string"), so the task's premise that Chromium paints strings there does not hold — rdom follows
  the engines (computed, nothing generated; DIVERGENCES §2). Decisions: `Content` gains `Counters`,
  `Quote(QuoteKind)` and `WithAlt { content, alt }` and becomes `#[non_exhaustive]` (Breaking);
  `ContentContext` gets provided `counters()` / `quote()`. The quote depth is document-wide tree-order
  state, so it lives in the counter walk (`CounterState::quote`, a `Cell`): a generated box's `<quote>`
  items are kept on its computed style (`content_quotes`), count as counter ops (`has_ops`, `note_ops`, so
  a moved depth recomputes its readers) and are replayed for kept subtrees (`replay_element`) — one
  mechanism for both. An element's own `content` resolves with `generates: false` (no marks, no depth).
  Marks are the English `quotes: auto` pair until C10-QUOTES. The alt text resolves to
  `ComputedStyle::content_alt`, not painted (copy excludes generated content, as browsers do; rdom has
  no accessibility tree to hand it to — consumers read the computed style). The UA sheet gains HTML's
  `q::before` / `q::after`; a `content` string now serializes escaped. Red: rdom-style
  `property_dispatch::content_tests` (3, new API — compile-red); `css_phase10/content.rs` — 6 of 8
  failed (`["a b c"]` for `["“a ‘b’ c”"]`; `1 c` / `II d` for `2.1 c` / `II-II d`; no `‘3`; no depth
  across elements; `content_alt` `None`), `var_substitutes_inside_content` and the element case passed
  before (guards); `counter_tests::partial_cascades_replay_the_quote_depth` added. Green after. Mutations
  (restored, touched): no quote replay → the replay test; quotes left out of `note_ops` → the replay test.
  Changed expectation: `ua_total_rule_count` 168 → 170. CSS-COVERAGE: `content` Partial → Supported,
  `counters()` Missing → Supported, §3.15 3 / 2 / 5 / 2, total 185 / 17 / 60 / 45.
- 2026-10-06 — C10-QUOTES. Found: `quotes` was an unknown property; C10-CONTENT's `<quote>` items used
  fixed English marks. Decisions: `rdom_style::Quotes` (`Auto` / `None` / `MatchParent` / `Pairs`, an
  `Arc` so inheriting is a refcount) with `pair(level, lang)` — a level past the last pair repeats it — and
  the table `auto_quotes(lang)` (CLDR's quotation / alternate delimiters for 21 primary subtags, English
  otherwise; documented, DIVERGENCES §2). The content language is the host's nearest `lang` / `xml:lang`
  (`cascade::quotes::content_language`, HTML §3.2.6.2; crate-local — C11-LINK-LANG's `:lang()` may move it
  into rdom-core), read only when a pseudo's `content` holds a quote. `match-parent` is computed away
  (`finalize_quotes`): the parent's value, an `auto` parent's made explicit in the parent's language, so a
  `lang="fr"` child of German text keeps „ “; the root's is `auto`. `CounterState::quote` takes the pair
  lookup, so the depth logic is unchanged. Red: rdom-style `content_tests` (two, compile-red: no `Quotes`);
  `css_phase10/quotes.rs` — 4 of 5 failed (three strict sheets warned on `quotes`; every language painted
  English), `auto_quotes_use_the_quoting_elements_own_language` passed before (a guard). Green after.
  Mutations (restored, touched): no `match-parent` computation → the match-parent test; no language →
  the language test. Changed expectations: `canonical_values` and `every_property_has_important_setter`
  gain `quotes`. CSS-COVERAGE: `quotes` Missing → Supported, §3.15 4 / 2 / 4 / 2, total 186 / 17 / 59 / 45.
- 2026-10-06 — C10-COUNTERS, part 1 of 2 (the counter styles). Found: `CounterStyle` was a closed enum of
  five styles formatted by hand-written arms; any other name dropped the declaration. Decision, table-driven
  and shared with C10-COUNTER-STYLE: every style is a `CounterStyleRule` — the descriptors of an
  `@counter-style` rule (`System`, symbols, additive symbols, `negative`, `prefix`, `suffix`, `range`, `pad`,
  `fallback`, `speak-as`) — and the 44 simple predefined styles of §6 are a table of such rules
  (`counters/predefined.rs`, built once behind a `OnceLock`); one generator (`counters/generate.rs`, §3.1)
  resolves `extends` chains (a cycle or a missing base: `decimal`), checks the range (the system's `auto`
  range by default), runs the system's algorithm in `i64` (so `i32::MIN` has a magnitude), pads (the
  negative sign counting, §3.6), signs, and falls back (§3.7; a cycle or `MAX_FALLBACK_DEPTH` = 8 ends at
  `decimal`). Bound: `MAX_REPRESENTATION_CHARS` = 60 (§3.1's floor for what a UA must support) — `symbolic`
  and `additive` check the length before building anything, `pad` never pads past it, and a longer result
  goes to the fallback, so no value allocates more than the cap. `CounterStyle` is now a name
  (`Name(Arc<str>)`, `#[non_exhaustive]`; Breaking): predefined names lowercase on parse (§3), others keep
  their case, the CSS-wide keywords and `default` are rejected, an undefined name formats as `decimal`, `none`
  as nothing. Lookup goes through `CounterStyleLookup` (`Predefined` here; the cascade's author rules next
  item) and `ContentContext::format_counter`, which the cascade overrides with the box's direction
  (`disclosure-closed` is `◂` under `rtl`). §7's complex styles stay undefined (DIVERGENCES §2). Red:
  `css_phase10/counters.rs` — both failed on HEAD (the strict sheet rejected `lower-greek`,
  `disclosure-closed`, …); rdom-style `counters::tests` (7, new API — compile-red). Green after. Mutation
  (restored, touched): `pad` not counting the negative sign → `numeric_styles_write_positional_digits`
  (`-07`). No existing expectation changed (two cascade tests build `CounterStyle::named("upper-roman")` /
  `decimal()` for the old variants).
- 2026-10-06 — C10-COUNTERS, part 2 of 2 (counter ops, scoping, the list-item counter). Found: `reversed()`
  and `counter-set` dropped the declaration; a sibling's reset nested in the previous sibling's counter
  instead of replacing it (§4.5), so `counters()` of a second list read `2.1`; `li` counted through an
  explicit UA `counter-increment`, `display: list-item` incremented nothing (§4.6), and `<ol start>`,
  `<ol reversed>` and `<li value>` did nothing; pseudo-elements other than `::before` / `::after`
  (`::backdrop`, `::selection`, the scrollbar parts) applied counter ops no replay accounted for.
  Decisions: (1) `CounterState::enter` takes the box's style and applies §4.4's order — reset (instantiate,
  replacing a same-parent counter), increment (plus the implicit `list-item` one: −1 when the innermost
  `list-item` is reversed), set — and only `::before` / `::after` hold ops; `has_ops` / `note_ops` count
  `counter-set` and `list-item`. (2) The reversed counter's initial value (§4.2's algorithm, current ED:
  negated increments plus the last non-zero one, a set stopping it) depends on boxes the top-down walk has
  not reached. It is computed at the instantiating box by replaying, in a scratch `CounterState` that traces
  the new counter, the scope's ops as last cascaded (`counters::reversed::initial_value`, stored in the
  computed op so replays reuse it); every value a walk used is checked after the walk against the boxes as
  they now are (`stale_reversed`), and `cascade` / `subtrees` re-run from the stale owners once — ops never
  depend on counter values, so it settles (the first cascade of an `<ol reversed>` and an appended item both
  take that path). Chose this over a post-cascade counter pass, which would have replaced the incremental
  replay machinery. (3) HTML's list attributes are presentational hints (CSS Cascade 4 §6.4.4): a new
  ladder step `Source::Hint` after UA normal (`Plan::add_hints`, `Declarations::with_hints`), built per
  element by `cascade::hints` (only `ol` / `li`; HTML §2.3.4.1's integer rules); `ol[reversed]` is a UA
  rule and `li` is `display: list-item` in the UA sheet. `CounterOp` gains `reversed` / `value_given`
  (`#[non_exhaustive]`, Breaking). Split (SIZE-1): `cascade/counters.rs` → `counters/{mod,reversed,tests}.rs`.
  Red: `css_phase10/counters.rs` — 6 of 7 new tests failed (three strict sheets rejected `counter-set` /
  `reversed()`; `2.1 z` for `1 z`; `0 x` for list items; `1. x` for `<ol start=5>`);
  `an_author_rule_beats_the_start_hint` passed before (a guard); rdom-style `content_tests` (counter ops)
  and the cascade tests `appending_to_a_reversed_list_renumbers_it`, `counters::tests` (new API). Green
  after. Mutations (restored, touched): no stale check → three integration tests and the append test; no
  sibling replacement → the sibling test; no hint step → the `<ol start>` test. Changed expectations:
  `ua_total_rule_count` 170 → 171; `nested_lists_scope_and_resume` now asserts the second list replaces
  the first's counter (`values == [0]`). CSS-COVERAGE: `counter-reset` Partial → Supported, `counter-set`
  Missing → Supported, §3.15 7 / 0 / 3 / 2, total 189 / 15 / 58 / 45. No snapshot changed.
- 2026-10-06 — C10-COUNTER-STYLE. Found: `@counter-style` was an unsupported at-rule (consumed and warned),
  and `symbols()` dropped the `content` declaration. Decisions: (1) Storage, as `@property`: the sheet keeps
  its definitions in source order (`Stylesheet::counter_styles`, `CounterStyleDefinition { name, rule,
  layer }`; `append` maps their layers), and the cascade orders all sheets' definitions once per run —
  layer rank (unlayered last), then sheet, then source — into a `CounterStyleRegistry` (built lazily by
  `Sheets::counter_styles`), so the last definition of a name wins (CSS Cascade 5 §6.4.3 for name-defining
  at-rules; Counter Styles 3 §3). The registry is a `CounterStyleLookup` over the predefined table, so the
  one generator of C10-COUNTERS serves author styles unchanged; `ContentContext::format_counter` resolves
  through it. (2) The grammar lives in rdom-style (`counters::descriptors`: every descriptor of §3.1–§3.9,
  `<symbol>` as string or identifier — images N/A, DIVERGENCES §2 — `additive-symbols` strictly
  descending, `range` bounds ordered, `speak-as` kept inert); rdom-css only reads the prelude and block
  (`counter_style.rs`, beside `property.rs`, whose `read_body` it shares). An invalid descriptor is dropped
  and reported; a rule that defines nothing (a protected or reserved name, symbols that do not suit the
  system, `extends` with symbols) is reported once (`WarningKind::InvalidCounterStyleRule`). (3)
  `symbols()` is `CounterStyle::Symbols(Arc<CounterStyleRule>)`, an anonymous rule (suffix `" "`,
  fallback `decimal`), which the generator starts from directly. Bounds (C10-COUNTERS', exercised here
  with author styles): `symbolic` / `additive` refuse to build past 60 code points before allocating, the
  final text is capped again, a fallback chain stops at a repeat or after 8 styles, an `extends` cycle
  extends `decimal`. Red: rdom-css `counter_style.rs` (4, compile-red: no `counter_styles()` /
  `InvalidCounterStyleRule`); `css_phase10/counter_style.rs` — all 5 failed (the strict sheets warned on
  `@counter-style` / `symbols()`); rdom-style `counter_takes_symbols_and_author_names`. Green after (a
  layer assertion added to the last-wins test). Mutations (restored, touched): definitions not ordered by
  layer → the last-wins test; both representation caps removed → the bounds test (either cap alone keeps
  it green — the early one bounds the allocation, the final one the text). No existing expectation
  changed. CSS-COVERAGE: `@counter-style` / `symbols()` Missing → Supported, §3.15 8 / 0 / 2 / 2, total
  190 / 15 / 57 / 45.
- 2026-10-06 — C10-LIST-ITEM, part 1 of 2 (the list properties and the `::marker` cascade). Found:
  `list-style-*`, `list-style` and `marker-side` were unknown properties; `::marker` an unsupported
  pseudo-element; `display: list-item` set a flag nothing read but the counters (C10-COUNTERS). Decisions:
  (1) The four longhands and the shorthand in rdom-style (`layout/list.rs`, `parse/values/list.rs`; the
  shorthand's `none` fills whichever of image / type is unset, §3.6), all inherited; `list-style-image`
  reuses the `background-image` `<image>` parser and stays inert. (2) `::marker` is a pseudo-element target
  whose rules are cut at rule-build time to §3.2's properties (`TuiStyle::marker_subset`, sharing
  `restricted_to` with `first_line_subset`), so no layout property can reach a marker. (3) The cascade
  computes a list item's marker before its `::before` (CSS Pseudo 4 §3.1's order, so counter and quote
  reads come first): `content: normal` — now `Content::Normal`, distinct from `none` — gives `list-style-type`'s
  text (`CounterStyle::marker_text_with` through the sheets' registry: prefix, the `list-item` value,
  suffix), `none` no marker; `text-transform` is forced `none` (the Lists UA sheet's rule, which no author
  rule can reach since the property does not apply); a marker's quotes count in the replays (`OpBox::Marker`).
  Right to left the marker text is given in visual order (runs reversed: `1. ` → ` .1`), as UAX #9 lays an
  isolated marker out — rdom draws text left to right. `TuiExt::computed_marker` (tripwire 448 → 456).
  Red: compile-red (new API: `computed_marker`, the list properties, `PseudoElementTarget::Marker`) for
  `css_phase10/list_item.rs` (5), rdom-style `list_tests` (2) and `marker_rules_keep_only_the_marker_properties`.
  Green after. Mutations (restored, touched): no `marker_subset` → the subset test and
  `the_marker_pseudo_element` (padding 3 reached the marker). Changed expectation: `ua_total_rule_count`
  171 → 176. The gate's `the_initial_style_allocates_nothing_for_grid` caught the initial `disc` allocating
  per element (a lowercased `String` and an `Arc`): predefined counter style names are now `'static`
  (`CounterStyleName`, matched case-insensitively against the table without allocating), `CounterStyle::disc()`
  / `decimal()` are `const`. The marker is not laid out yet; `li::before` still draws it (part 2).
- 2026-10-08 — C10-LIST-ITEM, part 2 of 2 (laying the marker out). Found: the marker was the UA's
  `li::before`, packed inside the line (never hung), riding a descendant's line through an `li`-only
  path (`marker_line_holder` / `deferred_markers`, keyed on the tag) that also kept an `li`'s `::before`
  from CSS 2.1 §9.2.1.1's own line; its intrinsic width was its raw `unicode-width` (C9G-MISC-CORRECTNESS's
  leftover). Decisions: (1) One pseudo path for the marker: `PseudoSlot::Marker` / `StyleSlot::Marker`, and
  the slot → style matches across layout, paint, hit-testing and visibility (19 sites, each `Before` /
  `After` by hand) collapse into `TuiExt::computed_pseudo` / `computed_for`. (2) `inline::markers` decides
  where a marker goes for any `display: list-item` box of block flow: it rides the item's first line box —
  the item's own when its `::before` or first line-bearing content is inline, else a block descendant's,
  else (no line reachable) a line the marker makes — and is pushed with that block's `::before`
  (`feed::push_marker`); `line_markers` lists the markers riding a line, outermost first. (3) `inside`
  packs as generated text (the `::before` path). `outside` packs through the packer too — the marker text
  alone in its run style (transform `none`, letter spacing, `white-space: pre`) — into fragments the line
  keeps apart (`LinePacker::push_outside_marker`, `cur_outside`, an `Op` for `text-wrap` replays): settled on
  the line's rows, untouched by its alignment and width. The layout pass gives them their column once the
  line's block is packed (`markers::place_outside`, at the two real packing sites): end at the item's
  border edge — read from the item's rect, written before its children are laid out — or start at its
  right edge when `marker-side` says right, the text then in visual order (`visual_rtl`; DIVERGENCES §2).
  Paint uses the uncut clip for them; static positions skip them. (4) Intrinsic sizes pack the markers
  riding a line (`widest_marker_line`) — an outside one takes no room — as followed by content: packed
  alone, a marker's last letter spacing would be dropped as at a line's end (7 cells measured 6), so it is
  measured as markers plus a one-cell probe, less the probe. (5) UA: the two `li::before` rules
  and `[role=treeitem]::before` go; `ul` / `menu` `padding-inline-start: 2`, `ol` `3` (room for "1. ").
  Caught on the way: `visible_inline_pseudos` asking for the line markers of every host recursed through
  `box_sequence` (stack overflow in two suites) — it now asks only whether the host's own marker makes its
  line (`makes_own_line`), which climbs nothing. Red: `css_phase10/list_item.rs` part 2 — 10 of 12 failed
  on HEAD (`  • a` for `• a`; the marker on the item's text row instead of hung; no rtl side; no marker on an
  empty item; `li::before` riding the paragraph), `inside_markers_are_the_first_inline_box` passed (the old
  inside placement agrees) and the intrinsic test was reshaped to a mixed-content paragraph so it measures
  the riding marker's term. Green after. Mutations (restored, touched): no `place_outside` and no right
  side → 11 tests; markers dropped from the intrinsic term → the measurement test (5 for 9). Changed
  expectations (the marker is `::marker`, hung): `ua_ul_renders_bullet_before_each_li` /
  `ua_ol_renders_numbered_markers` (`  • first` → `• first`), four cascade / app tests read
  `computed_marker` for `computed_before` (`nested_ul_does_not_advance_the_enclosing_ol_numbering`'s inner
  bullet is now HTML's `◦`), `ua_ul_ol_menu_have_left_padding` (`ol` 3), `ua_tree_aria_rules`
  (no `[role=treeitem]::before`), `ua_total_rule_count` 176 → 173, C10-COUNTERS' `<ol>` integration tests
  (a padded wrapper so markers past the viewport edge stay visible). Snapshots: `lists_generated` and
  `ua_chrome` — markers hang in the padding, a second paragraph aligns under the first: what a browser
  draws. DIVERGENCES: the `li::before` entry is gone; the outside-marker approximations are recorded.
  Not done: positioned `::before` / `::after` still lay out and paint on their own path
  (`positioned_pseudos`) — the one separate pseudo path left; the Phase 10 gate decides its unification.
- 2026-10-08 — Phase 10 part 1 closed (C10-LEGACY-COLON, -CONTENT, -QUOTES, -COUNTERS, -COUNTER-STYLE,
  -LIST-ITEM): CSS-COVERAGE §3.15 10 / 0 / 0 / 2, §3.16 6 / 1 / 3 / 6, §3.7 8 / 0 / 1 / 2, total
  194 / 14 / 54 / 45; ACID tile 9 extended. Part 2 (C10-FIRST, -HIGHLIGHT, -DETAILS-CONTENT, -PSEUDO-CHAINS)
  is next.
- 2026-10-08 — C10-PSEUDO-UNIFY (new row, from C10-LIST-ITEM's note). Found: positioned `::before` / `::after`
  had a layout and paint path of their own (`positioned_pseudos`, two modules): one string on one line (every
  intrinsic keyword its width), painted in a flat pass after every stacking context ordered by the host's
  `z-index`, not hit-tested, not in any scroll container's overflow, an axis with both insets `auto` at the
  containing block's edge; a relative one taken out of flow and anchored at the host's edge, a sticky one placed
  as an absolute one against its host. Decisions (CSS Pseudo 4 §2: positioned as an element): (1) An absolutely
  or fixed positioned pseudo-element is the generated box every other one is (`items::AnonymousItem::pseudo`),
  placed by phase 2 with the elements — `place::compute_placed_rect` is now over a `Placed` (an element, or a
  generated item: its `Keywords::for_run`, its content size as the shrink-to-fit size) — in document order, its
  containing block from its host up (C8-CB-COMPLETE's walk, grid areas included), its static position computed
  from its host's laid-out content (`positioning::pseudo::static_position`: a `::before` at the content start,
  a `::after` after the last line — on it when inline-level — or below the last block; content start in a flex
  or grid container; an inline host's first / last fragment). Its box is kept on the host
  (`TuiExt::positioned_pseudos`, one thin `Box` like `floated_pseudos`; `before_layout` / `after_layout` /
  `PseudoLayout` gone, Breaking), shifted with a sticky ancestor (a `fixed` one kept), counted by
  `positioned_overflow::settle` (now over `BoxItem`s), and laid into its stacking context's layers as a positioned
  child of its host (`stacking::Generated::Positioned`, `collect::Walk::positioned_pseudo`: its own `z-index` and
  `opacity` make it a context, its clip by `position`), painted (`paint_positioned_pseudo`) and hit-tested to its
  host there (`hit_generated`, shared with floats). (2) A relatively positioned or sticky one is in flow
  (`inline::generated::static_pseudo_text` excludes only `absolute` / `fixed`) and moved after layout
  (`positioning::pseudo_offsets`, pass 3, the flagged subtrees only): a box of its own moves its border box and
  lines; a run or atom of a line keeps its packed place and records the move (`GeneratedFragment::offset`), which
  paint (a moved run after the line's text) and hit-testing add. The relative offset is `relative_offset` against
  the content box of the block container whose flow holds it; the sticky one `sticky::sticky_offset`, now shared
  with sticky elements, in the nearest scrollport at or above the host. The moved one paints with its flow, not on
  the positioned layer (DIVERGENCES §2). (3) `pack_generated` packs the pseudo-element's own `content`, in flow or
  not. Caught on the way: every box-form pseudo-element (float, atom, block, flex item, now positioned) painted a
  translucent background twice under its text (the box, then the text's style) — C3G-PSEUDO-TINT had fixed only
  the old positioned path; its lines now paint in a glyph style (`FlowPlacement::boxed`,
  `text::pseudo_glyph_style`) — and the single-row painter, a host with no content's fallback, drew such a
  pseudo-element's text a second time at the host's first row (it now draws only inline-text pseudo-elements,
  `generated::is_inline_text`). Red: `css_phase10/pseudo_unify.rs` — all 8 failed on HEAD (`ab cd` on one row for
  a `min-content` box; `Zb` for `abZ`; `abcd` over the host's text for `XYcd`; the pseudo over a higher sibling
  context; `None` for the click; `scrollHeight` 3 for 7; the relative `::before` out of flow; the sticky one
  scrolled away); `color_tests::floated_pseudo_translucent_background_composites_once_under_its_text` red before
  the glyph style and the single-row fix. Green after. Mutations (restored, touched): no glyph style → both
  translucent tests; the single-row painter drawing floats again → the floated one; no
  positioned layer entries → the z, click, static and min-content tests; no offsets pass → the relative and
  sticky tests; overflow ignoring generated items → the overflow test; no static position → the static test.
  Changed expectations: `a_relative_pseudo_with_both_insets_only_shifts` (now in flow: the `::after` is the host's
  only content, at its start, moved by `(1, 1)`; under `rtl` at its end, moved left) and the two placement tests
  that read `after_layout` read `positioned_pseudos()`. DIVERGENCES: the flat-pass, not-in-overflow,
  not-hit-tested and one-string-width entries are gone; one entry records the moved pseudo-element's paint order.
  No snapshot changed.
- 2026-10-08 — C10-FIRST, part 1 of 3 (the cascade). Found: `::first-line` / `::first-letter` rules were stored
  whole and matched nothing (C10-LEGACY-COLON). Decisions: (1) rdom-style cuts them when the rule is built, as
  `::marker`'s: `first_line_subset` (CSS Pseudo 4 §2.2.1 — the font, color, `opacity`, background and text
  decoration properties, `text-transform`, `letter-spacing`, `word-spacing`; `line-height`, which §2.2.1 also
  lists, is left out until a first line can grow — DIVERGENCES §3) — shared with `::placeholder` (§4.3 gives it
  the same list, so it gains the transform, the spacings and every background longhand) — and
  `first_letter_subset` (§2.3.1: those plus `line-height`, `vertical-align`, `float`, margins, padding and
  borders; `restricted_to` gains `flow_relative` so `margin-inline-end` keeps its declaration and maps by
  direction, the rule getting direction overlays). (2) rdom-tui cascades them onto block containers (block flow,
  not inline, box-less or flex / grid; `walk::is_block_container`) as `TuiExt::computed_first_line` /
  `computed_first_letter`, the letter inheriting from the line (§2.3.1's fictional tag sequence), matched only
  when a sheet has such rules (`Sheets::styles_first`) so other documents pay no matching pass;
  `PseudoSlot::FirstLetter` / `StyleSlot::FirstLetter` name the letter's box (`computed_pseudo`). Red: rdom-style
  `first_line_rules_keep_only_the_first_line_properties` / `first_letter_rules_keep_only_the_first_letter_properties`
  (padding, `display`, sizes kept); `css_phase10/first.rs::first_line_and_first_letter_are_cascaded`
  (compile-red: no `computed_first_line`). Green after. Mutation (restored, touched): the letter inheriting from
  the block → the cascade test (green for red). No existing expectation changed. Split (SIZE-1): the subsets left
  `tui_style/mod.rs` (615) for `tui_style/subsets.rs`; the early pseudo-element styles left `cascade/walk.rs` (629) for
  `cascade/early_pseudos.rs` (a named struct for the nine-style tuple).
- 2026-10-08 — C10-FIRST, part 2 of 3 (`::first-line` laid out and painted). Decisions: (1) Which line
  (`inline/first_line.rs::hosts`): a flow packed for block `b` that holds its first formatted line is the first
  formatted line of `b` and of each ancestor whose first line-bearing box item is the block below it, with no
  inline `::before` line of its own before it (CSS Pseudo 4 §2.2; `generated::line_bearing_child` /
  `own_line_pseudos`, the predicates margin collapsing and markers use), stopping at a non-block container; the walk
  climbs only as far as the outermost ancestor with a `::first-line` / `::first-letter`, so a document without one
  pays an ext lookup per ancestor and allocates nothing. (2) Layout — the brief's "style switch at the line
  boundary in the packer": the packer (`packer/first.rs`) maps each run's `text-transform` / `letter-spacing` /
  `word-spacing` while the first line is packed (`FirstLineRun`: a value the run shares with the block is
  inherited, so it takes the `::first-line`'s — rdom has computed values only, DIVERGENCES §2), keeping each
  buffered grapheme's source and unmapped run; when line 0 is settled (`break_line` → `end_first_line`) the mapping
  ends and the buffered word — the next line's start, taken in under the first line's style — and the pending
  separator are reshaped in their own run (`split_word` now hands its rest back before the break, so the cut word's
  tail is reshaped too). Replicas (`text-wrap` replays) carry the first-line packing. The DOM text is not split:
  fragments map to the source through the existing `SourceMap`. Intrinsic sizes measure with the same packing.
  (3) Paint: the settled line keeps its hosts (`LineBox::first_line`); `first_line::effective` overlays, for a
  run on it, each host's `::first-line` color, font weight / style and applied decorations where a rule set them
  and the run's value is its host's, and the line's background behind a run with none of its own (painted with the
  run); `::before` / `::after` text on the line takes it too, markers do not. Red: `css_phase10/first.rs` — the
  seven layout / paint tests failed on HEAD (`Reset` for red on the first line, the descendant block's line, the
  `::before`; `aaa bbb` for `AAA BBB`; `Reset` for the blue background; the clamped line not uppercased; the
  spaced cell mapping to offset 2 for 1). Green after. Mutations (restored, touched): no reshape at the line's end
  → the transform / spacing and line-clamp tests (`CCC` / `c d`); no climb past the block → the descendant-block
  test; no paint overlay → the four color / background tests. No existing expectation or snapshot changed.
  CSS-COVERAGE: the row Missing → Partial (`::first-letter` remains), §3.16 6 / 2 / 2 / 6, total 194 / 15 / 53 / 45.
- 2026-10-08 — C10-FIRST, part 3 of 3 (`::first-letter`). Decisions: (1) The letter is found before packing, in
  the content (`inline/first_letter.rs::letter`): a walk of the flow's block in packing order — its inline
  `::before` text, its text and the inline boxes it holds (their `::before` / `::after`), out-of-flow and floated
  boxes skipped, an atom, a block or `<br>` ending it — scanning graphemes: leading white space skipped, then
  punctuation, one letter or number, the punctuation after it (§2.3.2; punctuation is a table, documented); the
  result is source spans (`LetterSpan`: a text node or a host's generated content, byte range), so the DOM text is
  never split. The innermost host of the first line's chain with a `::first-letter` styles it (`host_of`).
  (2) The packer takes a grapheme in a span under the letter's run mapping (`LetterRun`: its own
  `text-transform` / spacings where its rules set them) with an origin of its own (`Origin::letter`), so it makes
  fragments of its own (`InlineFragment::first_letter` / `GeneratedFragment::first_letter`), which paint styles
  over the first-line style (`first_letter::effective`: color, font, decorations, background). (3) A floated
  letter's graphemes are left out of the line and its float pushed where they were (`BoxItem::Generated(host,
  PseudoSlot::FirstLetter)`): the floated-pseudo machinery of C8G-PSEUDO-ATOMS places, lays out
  (`AnonymousItem::pseudo` — `pack_generated` packs the letter's source slices for this slot), paints and hit-tests
  it; `float_side_of` takes the slot without a `content` check. Red: `css_phase10/first.rs` — the five letter tests
  failed on HEAD (`Reset` for blue on the letter, the punctuation, the `::before`'s letter; `hello` for `Hello`;
  the drop cap absent). Green after. Mutations (restored, touched): no trailing punctuation → the punctuation test;
  the letter never floating → the drop-cap test. No existing expectation or snapshot changed. CSS-COVERAGE: the row
  Partial → Supported, §3.16 7 / 1 / 2 / 6, total 195 / 14 / 53 / 45. DIVERGENCES: the §3 entry is gone; §2 records
  the non-floated letter's box properties, the punctuation table, nested letters and intrinsic sizes.
- 2026-10-08 — C10-HIGHLIGHT, part 1 of 2 (the data model, rdom-core). Found: rdom-core had no live-range machinery
  — `Range` is a value, the selection is cleared when its node leaves the tree (DIVERGENCES §2) and text edits do not
  move it (the editor sets it after each edit) — so the brief's "existing Range boundary-update machinery" had to be
  built. Decisions: (1) `rdom_core::highlight` (renderer-free, it holds only `Range`s): `Highlight` (a set of ranges in
  insertion order, `priority`, `kind: HighlightType`; `#[non_exhaustive]`, built with `new` / `with_priority` /
  `with_type`) and `HighlightRegistry` (CSS Custom Highlight API 1 §4: a map in registration order — re-setting a
  name keeps its place, as a JS `Map` does — the tie-break of equal priorities, §5.2). `Dom::highlights` reads it;
  `Dom::highlights_mut` fires the new `Mutation::HighlightsChanged` first (a renderer cannot tell what a `&mut`
  borrower changed; rdom-tui's dirty tracker repaints on it, no cascade). (2) Every registered range is live (DOM
  §5.3), the updates in the mutators: "replace data" (§4.10 steps 8–11) in `NodeMut::write_data` — precise for
  `edit_text`, the whole data for `set_node_value` / `set_data`, as a browser's `.data =`; "insert" (§4.2.3 step 6)
  after `append_child` / `insert_before` link a node; "remove" (steps 4–7) in `detach_from_parent` before the unlink
  (the registry taken out so the subtree test can borrow the DOM). Each is a no-op with no highlight registered
  (`is_empty`), so documents without highlights pay nothing; the child index is computed only then. `StaticRange` is
  not modelled (DIVERGENCES §2). Red: rdom-core `highlight_tests` (4, compile-red: no `Highlight`), and the
  registration test's record count first expected 6 for the 5 accesses it made (a test bug, fixed). Green after. No
  existing expectation changed.
- 2026-10-08 — C10-HIGHLIGHT, part 2 of 2 (`::highlight()` styled and painted). Decisions: (1) rdom-style parses
  `::highlight(<custom-ident>)` in the suffix stripper (`selector_text::highlight_suffix`: the argument one
  identifier, through rdom-core's `css_syntax`); the name rides the target, `PseudoElementTarget::Highlight(Arc<str>)`,
  so the cascade's per-target matching needs no side channel — the enum loses `Copy` (Breaking). The highlight
  pseudo-elements' rules — `::selection`'s too, which had kept every property — are cut to CSS Pseudo 4 §3.2's
  properties rdom can draw (`highlight_subset`: `color`, `background-color`, the decorations, custom properties).
  (2) rdom-tui cascades `::highlight(name)` for each name the sheets style (`Sheets::highlight_names`), matched
  afresh — one per name, not a fixed slot — onto `TuiExt::computed_highlights` (a thin `Box`, keeping the size
  tripwire at 456). (3) One overlay path (`inline_paint/highlight_overlay.rs`, from `selection_overlay.rs`):
  `Overlays::of` builds the layers once per inline formatting context's paint — each registered highlight's
  ranges by priority, then registration (§5.2), the selection last, the topmost (Pseudo 4 §3.5) — and paints each
  over a fragment's cells in its range: the nearest ancestor's style for that layer, applying its background and
  the color and decorations that differ from that element's (computed values, DIVERGENCES §2); `user-select`
  excludes text from the selection's layer only. Red: rdom-style `highlight_pseudo_elements_carry_their_name` /
  `highlight_rules_keep_only_the_highlight_properties` (compile-red: no `Highlight` target); `css_phase10/highlight.rs`
  — all 4 failed (`Reset` for the highlight backgrounds; the yellow layer missing under the selection). Green after.
  Mutations (restored, touched): priority order reversed and the selection painted first → the overlap and the
  selection tests. No existing expectation or snapshot changed (no test styled `::selection` with a property the
  subset drops). TECH_DEBT: `HIGHLIGHT-COST-1` (fragments × ranges). CSS-COVERAGE: `::highlight()` Missing →
  Supported, `::selection`'s row rewritten, §3.16 8 / 1 / 1 / 6, total 196 / 14 / 52 / 45.
- 2026-10-08 — C10-DETAILS-CONTENT — partial. Found: a closed `<details>` was hidden by a UA rule
  `details:not([open]) > *:not(summary) { display: none }` — elements only, so text directly inside showed, and any
  `<summary>` (not just the first) stayed — and `::details-content` was rejected as unsupported. Decisions: (1)
  rdom-style parses `::details-content` (`PseudoElementTarget::DetailsContent`); the UA rule becomes
  `details::details-content { display: block }`. (2) rdom-tui cascades it onto every `<details>`
  (`TuiExt::computed_details_content`, early pseudo-elements). The slot (`cascade/details.rs`): a child is slotted
  unless it is the first `<summary>` element child (HTML §15.5.20); slotted children inherit from the slot (the three
  places a child's parent style is chosen: the walk, a kept element's replay, a partial cascade's root); the slot
  hides its content while the element lacks `open` (the UA's `content-visibility: hidden`) or the slot is
  `display: none` — hidden elements compute `display: none` (the mechanism the UA rule used, so layout, paint,
  hit-testing and focus already agree), hidden text is skipped where text joins the box tree
  (`box_tree::is_hidden_text`: the box and item sequences, `holds_loose_text`, the packer's feed, the first-letter
  scan). Not done: the slot is no box — rdom's box tree has no generated block holding element content, and adding
  one reaches every walk that pairs a box with its children — so its background, border, padding, sizes and
  `overflow` draw nothing and it cannot be animated (DIVERGENCES §2); `content-visibility` itself waits for
  C14-CONTAIN. Red: `css_phase10/details_content.rs` — all 3 failed on HEAD (`loose` under a closed summary; the
  strict sheet rejected `details::details-content`); the API assertion was compile-red (no
  `computed_details_content`) and was left out of the behaviour red run. Green after. Mutations (restored,
  touched): no slot inheritance and closed not hiding → all three. Changed expectation: the `TuiExt` size bound 456 →
  464 (one `Rc`; the item's other fields fit in what the removed `PseudoLayout`s freed). No snapshot changed.
  CSS-COVERAGE: Missing → Partial, §3.16 8 / 2 / 0 / 6, total 196 / 15 / 51 / 45.
- 2026-10-08 — C10-PSEUDO-CHAINS — partial. Found: a selector with anything after its pseudo-element was
  rejected (`::before:hover` an "unsupported pseudo-element"), save rdom's own `::scrollbar-thumb:vertical`;
  rdom-core's selectors reject every pseudo-element; nothing knew which pseudo-element the pointer was over
  (the hit test names a pseudo-element's host, C10-PSEUDO-UNIFY). The C5-BOX-SIZING entry's
  `*, ::before, ::after` note, recorded under this item, was settled by C5G-BARE-PSEUDO (a bare
  pseudo-element attaches to the implicit `*`; `the_bare_pseudo_reset_list_parses`) — nothing left of it.
  Decisions: (1) Representation, in rdom-style beside the pseudo-element: a pseudo-class after a
  pseudo-element describes the pseudo-element, which rdom-core's element AST cannot name (it rejects
  pseudo-elements and stays renderer-free), so the trailing compound is not an rdom-core compound but a set on
  the rule, `Rule::pseudo_state: UserActionState` (`:hover`, `:active`, `:focus`, `:focus-visible`,
  `:focus-within`); its parse reuses rdom-core's: `selector_text::extract_pseudo_chain` peels trailing
  `:x` segments off the end, each parsed by `rdom_core::selectors::parse("*:x")` and kept when it is a
  user-action pseudo-class (names, case and escapes by the element grammar), then hands the rest to
  `extract_pseudo_suffix`; when the rest ends in no pseudo-element they were the element's own (`a:hover`,
  left whole). Allowed after `::before`, `::after`, `::marker`, `::first-letter` (legacy spellings too);
  after another pseudo-element, a logical combination or a descendant (`::before :hover`) the selector is
  invalid. Specificity: each counts in B (§17). (2) The state, in rdom-tui as document data
  (`style::pseudo_pointer`: hovered and active `(host, PseudoSlot)`, the hosts to restyle): the focus
  pseudo-classes never match. The query: `HitTestExt::hit_test_pseudo` (`hit_test/pseudo.rs`) — given the
  element the hit test found, the generated box under the point among those layout recorded: the host's
  positioned boxes, the floated and block-level ones on a box at or above it, runs and atoms on the lines of
  the blocks at or above it, and the line a list item's marker rides; a marker or a first letter may belong to
  an ancestor of the hit; the innermost wins. The router writes hovered on every move and active on press /
  release, only while the App's sheets have a chained rule (`set_tracked`, synced with the sheet set as the
  sibling triggers are), so other documents pay no second lookup; the frame adds the changed hosts to the dirty
  roots (live and connected only). (3) Matching (`cascade/matching.rs`): a rule whose selector matches is
  recorded whatever its state, and applies only while `pseudo_pointer::matches` holds — on a match and on a
  cached reload alike; `Scratch::gated` says a gated rule matched, which makes a `::first-letter` exist unstyled
  so it can be hovered. Not done — nested pseudo-elements (`::before::marker` / `::after::marker`, CSS Lists 3
  §3.1): a marker on a generated box needs a marker slot per pseudo-element, the `list-item` counter on a
  `::before` / `::after` and marker placement in a generated box's lines (the marker path is element-keyed:
  `computed_marker`, `inline::markers`), a two-commit item of its own; DIVERGENCES §3 lists it. Red: rdom-style
  `user_action_tests` — `a_pseudo_element_takes_trailing_user_action_pseudo_classes` and
  `trailing_pseudo_classes_count_as_pseudo_classes` failed (`Err("unsupported pseudo-element…")`), the
  invalid-selector and element-pseudo-class tests passed before (guards); `css_phase10/pseudo_chains.rs` — all 6
  failed with a stub `hit_test_pseudo` (the rules applied unhovered: red without the pointer; `None` from the
  query). Green after, with `pseudo_pointer` unit tests (stale hosts dropped, untracking restyles) and
  `pseudo_chain_tests` (no tracking without a chained rule). Mutations (restored, touched): the frame not
  draining the changed hosts → the three hover tests (the `:active` one passes: the element's `:active` already
  restyles the host); no unstyled first letter → the first-letter test. The marker test hovers an `inside`
  marker: an outside one is not in the hit-test set (DIVERGENCES §2, extended). No existing expectation or
  snapshot changed. CSS-COVERAGE: the row stays Partial (nested pseudo-elements), counts unchanged.
- 2026-10-08 — C9-CARRY-INDENT (new row, from the Phase 9 close's open list). Found: `text-indent` is inherited
  (CSS Text 3 §8.1) and a `calc()` one keeps its percentage to the used value (CSS Values 4 §10.9), so every
  descendant's computed style held a copy of the expression tree — `Length::Calc(Box<CalcExpr>)` cloned per
  inheriting element (six allocations each for `calc(50% + 2)`). Decision: `Length::Calc` holds an
  `Arc<CalcExpr>`, as `Quotes` and the font family list are shared — one change for the indent and the insets
  (`Length`'s other users), every `Length` clone allocation-free; `Length::calc(expr)` builds one (Breaking —
  the variant's payload). The other `Calc(Box<…>)` payloads stay boxed; of them `Spacing` (`letter-spacing` /
  `word-spacing`) and `LineHeight` are inherited, and whether a `calc()` of theirs reaches the computed style
  unresolved — and so clones per element — is open for the Phase 10 gate. The viewport absolutizer wraps its
  result (`|e| Length::Calc(e.into())`). Red: rdom-tui
  `cost_tests::an_inherited_calc_text_indent_is_shared_not_copied` — 420 allocations for 300 over 20 elements
  (`text-indent: calc(50% + 2)` against `text-indent: 2`, `cascade_allocations` as the family-list test). Green
  after. Mutation: HEAD's `Box` is the reverse change (the red run). Changed expectations: three tests build a
  `Length::Calc` (`Length::calc(…)` / `.into()`); no behaviour or snapshot changed.
- 2026-10-08 — Phase 10 part 2 closed (C10-PSEUDO-UNIFY, -FIRST in three parts, -HIGHLIGHT in two,
  -DETAILS-CONTENT, -PSEUDO-CHAINS, and C9-CARRY-INDENT from the Phase 9 close): every Phase 10 row is done but
  two, honestly partial — C10-DETAILS-CONTENT (the slot is no box) and C10-PSEUDO-CHAINS (nested
  pseudo-elements). CSS-COVERAGE §3.15 10 / 0 / 0 / 2, §3.16 8 / 2 / 0 / 6, total 196 / 15 / 51 / 45; ACID
  tiles 9 and 17 extended and an interactive step I12 added. Open for the Phase 10 gate: the
  `::details-content` slot box (a generated block holding element content; DIVERGENCES §2); `::before::marker`
  / `::after::marker` (DIVERGENCES §3); `HIGHLIGHT-COST-1` (TECH_DEBT: overlay cost fragments × ranges);
  outside markers outside the hit-test set, so `::marker:hover` reaches inside markers only (DIVERGENCES §2);
  whether an inherited `calc()` `letter-spacing` / `word-spacing` / `line-height` clones per element as
  `text-indent` did (C9-CARRY-INDENT); and the re-review of the Phase 9 gate fixes (`C9G-*`) that rides with
  this gate.
- 2026-10-08 — Phase 10 gates (with the C9G re-review: all 14 at the root). Architect: 2 blocking —
  marker placement is quadratic in sibling count in every document (`line_markers` climbs and rebuilds
  the parent's `box_sequence` per flow; ~4M box visits for 2000 rows); two production files past 575
  (`inline_paint/mod.rs` 577, `computed.rs` 589) with SIZE-1 stale. API: 4 blocking — six reshaped
  items (`CounterStyle`, `CounterOp`, `Content`, `PseudoElementTarget`, `Length::Calc`,
  `parse_counter_ops`) lack API-table rows and migration groups, `CounterStyle` / `CounterOp` not at
  the root; DESIGN lists `Content` closed (it is open), ~17 new types unclassified, `ListStylePosition`
  / `MarkerSide` / `QuoteKind` wrongly `#[non_exhaustive]`; upgrade guide misses seven silent changes
  (`<q>` quotes, closed `<details>` hides loose text, `::selection` subset, `::placeholder` subset,
  nested `◦` / `▪`, `abbr[title]` dotted, `li{counter-increment:none}`) and item 10 lacks the
  `li::before` colour and `ul{padding:0}` consequences; outside markers clip at column 0 (UA list
  padding 2 / 3 cells; "10. " paints "0. "). Non-blocking: highlight cost (O(N²) appends while a
  highlight is registered, fragment insertion bypasses the remove hook, ranges cloned per IFC per paint,
  `::highlight()` re-matched and allocated per element per name on every cascade; HIGHLIGHT-COST-1);
  `first_line::hosts` climbs for every flow with no sheet using it; outside markers not hit-testable
  (`::marker:hover` only for inside), the flex / grid list-item DIVERGENCES line describes invalid CSS;
  `<ol type>` / `<li type>` unmapped; inherited values that allocate per element (`font-size` %,
  `color-scheme`, `ListStyleImage(String)`) — want one table-driven cost test and `Arc` payloads;
  `TuiExt` holds 14 mostly-`None` pseudo style slots (~130 B / node) — move to one boxed side record;
  pseudo hit test scans every ancestor line per pointer move; sheets rescanned per cascade
  (`styles_first`, `highlight_names`, the counter-style registry); positioned `::before` static
  position ignores `text-align`; layering test evadable; counter cap claim holds only for symbolic /
  additive; `::selection{background}` paints white text (paired cascade); highlight API friction (no
  descendant iterator, no checked `Range` builder, `HighlightsChanged` fires before the change on every
  `highlights_mut()`, `kind` / `with_type`, `size` / `len`); `GeneratedFragment::offset` / `outside`
  crate-private, `positioned_pseudos()` returns `AnonymousIfc`; `Calc` payloads inconsistent (`Arc` vs
  `Box`); `InvalidCounterStyleRule` overloaded with an untyped reason; READMEs (rdom-core record list,
  rdom-tui Phase 10, list and highlight doctests); Zellij not a multiplexer; ACID tile 9 too big;
  `Backend` doc should say wrappers forward the capability methods. Also open from the phase:
  `::details-content` as a real box, `::before::marker` / `::after::marker`. Full reports:
  `target/claude-logs/c10_gate_{architect,api}.md`. Fix as `C10G-*`, three batches: A correctness and
  cost, B finish the two partial items, C API and docs.
- 2026-10-08 — C10G-SPLITS (architect B2). Two production files past the 575 split-on-touch bar, split by
  concern with no code change: `rdom-tui/src/render/paint_pass/inline_paint/mod.rs` (578) → `mod.rs` 325
  (the three entry points and the anchor tagging) + `inline_paint/flow.rs` 271 (`FlowPlacement` and the
  shared line walker `paint_inline_layout`, which C10G-HIGHLIGHT-COST changes next); `rdom-style/src/computed.rs`
  (590) → `computed.rs` 398 (the `ComputedStyle` record) + `computed/initial.rs` 159 (`initial()` and
  `Default`) + `computed/queries.rs` 55 (`flex_direction`, `is_scroll_container`, `is_atomic_inline`,
  `clips_overflow`, `normalize_overflow`). TECH_DEBT `SIZE-1` recounted against the tree (21 files between
  500 and 575, none past). Public paths unchanged; no test or snapshot changed.
- 2026-10-08 — C10G-MARKER-COST (architect B1). Found: `line_markers` (CSS Lists 3 §3.5: a marker rides the
  first line of the block holding the item's first line box) climbed from every packed and measured flow,
  in every document, and each step's `line_bearing_child` built the parent's whole `box_sequence` Vec — a
  row among N siblings visited all N. Fixed at the root, two parts: (1) `box_tree::find_in_sequence` finds
  the first (last) accepted item of a box sequence by walking the child list in place from that end — the
  same items `box_sequence` yields, through box-less children (`find_in_sequence_agrees_with_box_sequence`
  pins it against `box_sequence` both ways) — so `line_bearing_child` and its callers (the climb,
  `first_line_holder`, `inline_content_at_edge`, `run_pseudos`, `own_line_pseudos`) stop at the first
  line-bearing item and allocate nothing; (2) the climb runs only in a document with a list item: a new
  document flag (`style::doc_flags`, document data, the first of the cascade's document-wide facts) is set
  when an element computes `list_item`, and is conservative like the `tree_has_*` flags (never cleared).
  Decision: a document flag, not a per-node "inside a list item" bit — the bit would need its own
  invalidation when an ancestor's `display` changes in a partial cascade, and with (1) a climb step is O(1)
  anyway. Red: `box_tree_tests::sibling_rows_lay_out_in_linear_box_tree_visits` (≤ 16 visits per row
  for 2000 sibling `<div>row</div>` in a `<div>`) failed with 8 018 000 visits (81 800 for 200). Green:
  18 000 with no list item, 26 000 with every row a list item, 22 002 with one list item elsewhere. Pin
  added: `markers_tests::the_marker_climb_runs_only_in_a_document_with_list_items` (0 steps without, > 0
  with). Mutation (each alone, restored, touched): the eager `box_sequence` lookup → the visit test fails
  (8 022 000 for the list-item rows); the gate removed → the climb test (6 steps for 0). No existing
  expectation or snapshot changed.
- 2026-10-08 — C10G-MARKER-CLIP (API B4). Found: the UA gave `ul` / `menu` two cells of
  `padding-inline-start` and `ol` three, so an outside marker (CSS Lists 3 §3.5: its end at the item's
  border edge) wider than that started left of a list at column 0 — "10. " painted "0. ", "III. " "I. ".
  Decision: HTML §15.3.8's 40px is 2.5em at the 16px default font, about five digits of text, so all three
  lists get four cells — "10. ", "iv. " and a bullet with room fit, as in a browser at the page's edge; a
  wider marker (upper-roman "III. ", a 100th item) overflows the list's box into what is beside it, as a
  browser's does, and is cut only by the viewport or a clipping ancestor. Sizing the padding to the widest
  marker (scanning the item count) was rejected: a browser does not, and an author `padding` must keep its
  meaning. DIVERGENCES §2's marker entry now says so, with the column-0 case and its remedy (a
  `margin-left`). Red: `list_item.rs::the_lists_padding_holds_ten_and_overflow_is_clipped_only_by_the_viewport`
  failed at its first row (`"1. a   "` for `" 1. a  "`: three cells); green after, with "10. j" whole, "III. "
  cut to "II. " at column 0, whole beside a `margin-left: 2`, and cut at an `overflow: hidden` ancestor's
  edge. Changed expectations (the padding is two cells wider for `ul`, one for `ol`; nothing else moved):
  eleven `list_item.rs` rows and the measured width (`4 + 7`), `counters.rs` (the `start` / `reversed` /
  `value` rows no longer need their wrapper's padding — the B4 workaround — and `101.` sits a cell in),
  `pseudo_chains.rs` (the inside marker is two cells further right), three `paint_pass` list rows and the
  UA padding test; snapshots `lists_generated.snap` and `ua_chrome.snap` — the list rows only, one or two
  cells right; no background changed.
- 2026-10-08 — C10G-MARKER-HIT (architect N5, N6, N12; API B4's DIVERGENCES line). Four parts, each red
  first. (1) Outside markers are hit as their item (CSS Lists 3 §3.5: the marker is a box of the list item
  outside its principal box; browsers hit-test it to the `li`): the element hit test missed the item's box
  and stopped at the list whose padding the marker hangs in, so a click on a bullet targeted the `ul` and
  `::marker:hover` reached inside markers only. Fixed where the walk visits the item in reverse paint
  order (`descend_children_reverse`): a point that misses an element's box is tested against its outside
  marker (`hit_test::pseudo::on_outside_marker` — the marker's line holder's lines, which `place_outside`
  already placed; free without a list item, `doc_flags`), and the item goes on the path; the pseudo hit
  test then names the marker as before. (2) The DIVERGENCES §2 sentence about a flex or grid list item
  went (CSS Display 3 §2.3 makes `list-item flex` invalid, and rdom rejects it), as did its comment in
  `markers.rs`, and the "not in the hit-test set" clauses (§2 marker entry, §2 pseudo-element `:hover`).
  (3) `<ol type>` / `<li type>` (`1 a A i I`, case-sensitive) and `<ul type>` / `<li type>` (`none disc
  circle square`, ASCII case-insensitive) map to `list-style-type` (HTML §15.3.8) in `cascade/hints.rs`.
  Decision: presentational hints, not UA rules as HTML writes them — rdom matches `type` values
  case-insensitively, so a UA selector cannot tell `a` from `A`; recorded in DIVERGENCES §2 (an author
  `revert` removes a hint). (4) An absolutely positioned inline `::before`'s static position (CSS 2.1
  §10.3.7: the hypothetical box with `position: static`) is the start of its host's first line — after
  `text-align` and `text-indent` — not the content box's edge (`positioning::pseudo::first_line_start`; a
  block-level one keeps the content start). Red: `list_item.rs::an_outside_marker_is_hit_as_its_item`
  (`(2, 0)` hit the `ul`), `pseudo_chains.rs::outside_marker_hover_matches_over_the_marker` (`Reset` for
  red), `list_item.rs::the_type_attribute_sets_the_list_style_type` (`1. ` for `a. `) and
  `pseudo_unify.rs::an_absolute_before_starts_where_its_hosts_first_line_starts` (`"*  ab   "` for
  `"   *b   "` centered); green after. Mutation (the three code changes disabled together, each caught by its
  own tests, restored, touched): the marker branch → both hit tests; the `type` hint → the type test; the
  first-line start → the static-position test. No existing expectation or snapshot changed.
- 2026-10-08 — C10G-SELECTION-PAIRED (API N2). Found: the UA's `*::selection { background-color: Highlight;
  color: white }` supplied `color` whatever the author set, and the overlay applies a `color` that differs
  from the text's, so `::selection { background-color: yellow }` painted white text on yellow. CSS
  Pseudo-Elements 4 §3.4's paired defaults: the UA's highlight colors are used as a pair — an author value
  for either drops the UA's other. Fixed in the cascade, where origins are known
  (`style/cascade/paired.rs`, run on the computed `::selection` style): when the declarations above the UA
  origin (author layers, hints, inline) set exactly one of `color` / `background-color` — directly, or
  through a pending `var()` declaration — the other is reset to what an unset highlight value paints: the
  originating element's `color`, a transparent background. Decision: only `::selection` — the UA styles no
  `::highlight()`, so the pair has nothing to drop there. DIVERGENCES §2's highlight entry says so. Red:
  `highlight.rs::an_author_selection_color_drops_the_uas_other_half` failed with `(white, yellow)` for
  `(red, yellow)`; green after (also `(red, blue)` with only `color` set, and the UA pair untouched with
  neither). Mutation (the background reset dropped, restored, touched): the `color`-only row fails with the
  UA's `Rgb(57, 75, 126)` background. No existing expectation or snapshot changed.
- 2026-10-08 — C10G-HIGHLIGHT-COST (architect N1, N2, N3; API N3.5; closes TECH_DEBT `HIGHLIGHT-COST-1`).
  Found, by counting (red first, each): (1) rdom-core walked every earlier sibling (`child_index`) on each
  insertion while a highlight was registered — 1 999 000 hops for 2000 appends under one search highlight
  — and each removal too; (2) inserting a `DocumentFragment` unlinked its children by hand, so a live range
  inside them stayed there (DOM §4.2.3 "insert" step 4 removes them first: it moves to `(fragment, 0)`);
  (3) `Overlays::of` copied every range of every highlight into a layer list for each inline formatting
  context's paint (15 000 copies for 50 paragraphs, 100 hits, 3 paints) and every fragment tested every
  range; (4) a `*::highlight(search)` rule allocated a style and a list per element per cascade (420
  allocations for 300 over 20 elements) and a restyle matched it again. Fixed at each root: (1) an append
  moves no boundary (its index is the old child count, which no offset exceeds) and is skipped; an
  insertion or removal elsewhere walks the siblings only when a boundary sits in the parent or inside the
  removed node; (2) the fragment branches of `append_child` / `insert_before` insert each child in turn,
  so each leaves the fragment through `detach_from_parent` and its remove hook; (3) a new
  `render::highlight_index` keys an `OverlayIndex` by `HighlightRegistry::generation()` (new, rdom-core:
  moves at every `highlights_mut()` and every live-range move) and the selection, indexes the layers by
  text node (ranges within one node by that node, ranges spanning nodes apart, merged in paint order per
  fragment), is built once at the end of `layout_dom` (document data), and a paint after a change no
  layout saw builds its own; (4) the `::highlight()` matches are recorded in `MatchedRules` per name
  (shared across elements through the pass's `Scratch`) and reloaded by a restyle, a style equal to the
  parent's for that name is the parent's `Rc`, and an equal list is the parent's
  (`TuiExt::computed_highlights` is an `Rc<Vec<…>>`). Getting (4) to zero per element found two
  allocations every element and pseudo-element paid: `Dom::class_list` boxed its iterator (now a concrete
  one; rule matching reads it instead of a `DomTokenList` snapshot), and `ComputedStyle::initial()` built
  a fresh empty `var()` map (now one per thread, shared — immutable, so no test-order dependence; the
  allocation test helper builds it before counting). Decision: build the index at layout, not at paint —
  paint takes `&Dom`, layout is the `&mut` step before every paint, and the key makes a stale index
  harmless. Tests: rdom-core `highlight_tests.rs::appending_under_a_highlight_walks_no_siblings` (0 hops for
  2000 appends, an insertion and 100 removals with no boundary in the log, and 500 appends with one) and
  `inserting_a_fragment_removes_its_children_from_it_first`; rdom-tui
  `inline_paint/cost_tests.rs::highlight_ranges_are_indexed_not_copied_per_flow` (0 copies, ≤ 300 range
  tests for 3 paints), `cascade/cost_tests.rs::a_universal_highlight_rule_shares_its_style` (equal
  per-element allocations with and without the rule: 160 = 160) and
  `a_restyle_reuses_the_highlight_matches` (0 selector matches). All red on HEAD as quoted, green after;
  the plain baseline itself fell from 300 to 160 allocations per 20 elements. Mutation (restored,
  touched): no append fast path → 1 075 250 hops; no removal gate → 195 050 hops; the prepared index
  ignored → 15 000 copies; no node index → 15 000 range tests; no style sharing → 220 allocations for
  160; highlight matches never reloaded → 1 match. No existing expectation or snapshot changed.
- 2026-10-08 — C10G-IDLE-SCANS (architect N4, N10, N11). Found, by counting (red first, each): (1)
  `first_line::hosts` climbed every ancestor of every packed and measured flow looking for a
  `::first-line` / `::first-letter` host — 84 steps for six nested blocks of text with no such rule
  anywhere; (2) every cascade run rescanned its sheets for `styles_first`, `highlight_names` and the
  `@counter-style` registry (rebuilt by cloning every rule) — 9 scans for two full cascades and a partial
  one over one sheet set; (3) the pseudo hit test (`hit_test_pseudo`, what `::before:hover` reads) looked
  at every line of every block above the hit — 2001 lines for a point in a 2000-line `<pre>`. Fixed: (1)
  a second document flag (`style::doc_flags::has_first_rules`), set by every cascade run (full or
  partial) from its sheets, gates the climb; unlike the list-item flag it follows the sheets both ways;
  (2) the three facts move from the per-run `Sheets` to `SheetFacts` on the sheet set's
  `PropertyRegistry` — the set's identity (`Sheets::stamp`), which an `App` rebuilds exactly when its
  sheets change and the stateless forms keep per unchanged sheet set — so they are found once per set;
  (3) the lines are read by row (`InlineLayout::line_at_row`), all of them only when the block's subtree
  holds a positioned `::before` / `::after` (`tree_has_positioned_pseudo`: a relatively positioned one is
  drawn off its line). The router already asks the pseudo hit test only while a sheet has a chained rule
  (`pseudo_pointer::is_tracked`). Decision: cache the facts on the registry rather than precompute them on
  each `Stylesheet` (the gate's suggestion) — the counter-style registry orders definitions across sheets
  by layer, so it is a fact of the set, and one place keeps all three. Tests:
  `idle_cost_tests.rs::no_first_line_rule_no_climb` (0 steps), `cascade/cost_tests.rs::a_sheet_set_is_scanned_once`
  (3 scans: one per fact) and `hit_test/tests.rs::the_pseudo_hit_test_reads_the_line_at_the_row` (≤ 8 lines;
  and a relatively positioned `::before` a row below its line still found there). Red on HEAD as quoted,
  green after. Mutation (restored, touched): no first-rules gate → 84 steps; every line read → 2001
  lines; never every line → the moved `::before` is missed (`None`). No existing expectation or snapshot
  changed.
- 2026-10-08 — C10G-INHERIT-COST (architect N7; API N5). One table-driven test now covers the inherited set:
  `cascade/cost_tests.rs::every_inherited_value_is_shared_not_copied` gives every property
  `property_dispatch::inherits` names a non-initial value — one holding heap data where the property can (a
  percentage, an expression, a string, a list, a custom counter-style name) — asserts the table covers the
  set and that each value parses, and compares the per-element allocations of 20 plain elements under a
  root setting it against a root setting none. Red on HEAD, seven copied per element (200 allocations for
  160, `color-scheme` 280): `font-size: 62.5%` and the `font` shorthand carrying it
  (`PaintLength::Calc(Box)`), `text-underline-offset: 10%` (the same), `list-style-type: '→ '`
  (`String`), `list-style-image` (`String`), `color-scheme: light dark` (`Vec<String>`) and
  `block-ellipsis` (`String`). Fixed at the payloads: `ListStyleType::String`, `ListStyleImage::Image` and
  `BlockEllipsis::Str` hold `Arc<str>`; `ColorSchemeList` keeps its names as `Option<Arc<[String]>>` (`None`
  for `normal`, so `initial()` allocates nothing); and every `calc()` payload is an `Arc<CalcExpr>`, as
  `Length`'s became in C9-CARRY-INDENT — `Size`, `MinSize`, `MaxSize`, `GapValue`, `PaddingValue`,
  `MarginValue`, `FlexBasis`, `Spacing`, `LineHeight`, `VerticalAlign`, `PaintLength`, `TrackBreadth` and
  `IntrinsicSize::FitContentLimit` — each with a `calc()` constructor (API N5: no consumer spells the
  pointer type). Green after: every row 160 = 160. Breaking: the CHANGELOG bullet with its migration
  (`Size::Calc(Box::new(e))` → `Size::calc(e)`), API-table rows (from 0.5: `Size`, `MinSize`, `GapValue`,
  `PaddingValue`, `MarginValue`; after 0.5: the rest and the three `Arc<str>` variants) and the hint group
  `migration_hints.rs::calc_payload_hints`. Mutation: HEAD's payload types are the reverse change (the red
  run). Changed expectations: construction sites in rdom-style's own tests (`Box::new` → `Arc::new` or
  `calc()`); no behaviour or snapshot changed.
- 2026-10-08 — C10G-TUIEXT-SIDE (architect N8). Found: every element's `TuiExt` held twelve pseudo-element
  slots almost always `None` — `::marker`, `::first-line`, `::first-letter`, `::details-content`,
  `::backdrop`, the three scrollbar parts, and the `::before` / `::after` previous styles and transition
  overrides — 464 bytes per node. Fixed: they move into one `PseudoStyles` record
  (`ext/pseudo_styles.rs`, `#[non_exhaustive]`, DESIGN's classification extended), boxed on `TuiExt` only
  while one is set: the cascade writes its eight through `update_pseudo` (no box for an element that sets
  none, the box dropped when the last goes), the transition engine its four (`snapshot_pseudo_prev`, the
  override slots of `presentation_for_mut`, `release_empty_presentation` dropping an emptied record).
  `::before` / `::after` / `::selection` stay inline (the UA's `*::selection` gives every element one), and
  so do the `::highlight()` styles — decided by the gate's own test: moved into the record, a
  `*::highlight()` rule boxed one per element (`a_universal_highlight_rule_shares_its_style`: 180
  allocations for 160), while inline they stay shared from the parent.
  Accessors stay: `computed_pseudo(slot)`, `computed_for(slot)`, `presentation_for(slot)`,
  `computed_highlight(name)` and the `TuiNodeExt` reads are unchanged, and each moved field has an accessor
  of its name (`computed_backdrop()` …) — the public fields of 0.5 among them are a Breaking bullet, an
  API-table row and the hint group `tui_ext_pseudo_hints`. Red: the size tripwire lowered to 368 (all
  thirteen moved) failed on HEAD (`size_of::<TuiExt>() = 464, bound 368`); green after at 376 with the
  highlights kept inline (−88 B per node), the bound set there. Pin added:
  `ext/tests.rs::the_pseudo_side_record_is_boxed_only_when_used` (none on a plain `div`, one on an `li`,
  gone when it stops being a list item). No other expectation or snapshot changed (the test reads rewrote
  field reads as accessor calls).
- 2026-10-08 — C10G-MINOR (architect N13: the layering scan and the counter cap). Two fixes, each red first.
  (1) The layering test (`style/layering_tests.rs`, CLAUDE.md §Architecture Hygiene: `style/` reaches no
  `crate::render`) scanned lines for the text `crate::render`, so a grouped import — `use
  crate::{render::Rect, style::X};` — or a `super::super::super::render` path evaded it. It now reads paths
  as Rust does: comments and string / character literals blanked, every `use` tree flattened (groups,
  nested groups, `self`, `as`, `*`), every path in code collected, and `crate::` / `super::` / `self::`
  resolved against the file's module path (`module_of`: `style/cascade/walk.rs` is `style::cascade::walk`).
  Red: `every_spelling_of_a_render_path_is_caught` failed at the grouped import under the old scan
  (refactored first into the same `offenders(module, source)` shape); green after, with the nested group,
  the multi-line `pub(crate) use crate::{ render, };`, both `super` paths (three levels up is
  `crate::render`, two is `style::render` and passes), a path in code, and a `render_x` name, a comment
  and a string not caught. The tree itself is clean. (2) The counter cap (`MAX_REPRESENTATION_CHARS`, 60
  code points, §3.1's "at least 60"): `numeric` and `alphabetic` built the whole representation —
  `symbols("0"×20 "1"×20)` built 630 code points for `i32::MAX` — before the final check threw it away;
  the Phase 10 log's "no value allocates more than the cap" held only for `symbolic` and `additive`. Both
  now share `positional`, which counts each digit's code points as it finds the digits (indices on the
  stack, at most 64) and returns `None` — the fallback writes the value — before building past the cap; so
  the claim holds for every system. Red: `counters/tests.rs::numeric_and_alphabetic_check_the_cap_before_building`
  (630 built for ≤ 60); green after (the fallback `2147483647`, nothing built, and two-digit values of 40
  code points still written). Mutation (each, restored, touched): the `use` trees ignored → the grouped
  import passes the scan; the cap check removed → 630 built. No existing expectation or snapshot changed.
- 2026-10-08 — C10G-INHERIT-COST split. C10G-INHERIT-COST's `calc()` constructors left
  `rdom-style/src/layout/sizing.rs` at 600 production lines, past the 575 split-on-touch bar (found by the
  batch's closing recount; the gate's limit is 600, so the workspace check still passed): `aspect-ratio`
  (`AspectRatio`) moves to `layout/aspect_ratio.rs` (71) and `gap` (`GapValue`) to `layout/gap.rs` (65),
  `sizing.rs` 468; re-exports unchanged. `Size`'s doc no longer says its `Calc` clones the tree (it is an
  `Arc`). TECH_DEBT `SIZE-1` recounted (none past 575). No code or test changed.
- 2026-10-08 — C10G-DETAILS-CONTENT-BOX (finishes C10-DETAILS-CONTENT). Found: `::details-content` was a
  style with no box — its background, border, padding, sizes and `overflow` drew nothing, a closed element
  kept no box, and loose text in the slot took the `<details>`'s style. Decision (HTML §15.5.20: the second
  slot of the element's shadow tree is a block box): the slot's box is a node outside the document, kept by
  the cascade (`style::cascade::details::sync_content_box`: created with the first slot style, given each new
  one as the same `Rc`, dropped with the last; linked both ways through `PseudoStyles`'
  `ContentBoxLink`; the bottom-up `tree_has_*` flags mirrored onto it; a box whose `<details>` left the
  arena reclaimed by each cascade, document data), and the box tree puts it between the element and its
  content (`render::box_tree::slot`): `box_tree::children` — the one child walk, `PaintOrder::tree` — gives
  a `<details>` its first `<summary>` and the box, in that order wherever the summary is, and the box the
  other children in tree order; `slot::parent` climbs from slotted content to the box and from the box to
  the `<details>`, and `box_parent` reads it. Every walk that read `child_nodes()` / `first_child()` for
  boxes now reads `children` (layout's element children, IFC detection, shifts, positioned and sticky
  collection, static positions, scroll extents, float detection, the inline feed and first letter, paint's
  row bounds and text, the scroll / snap / smooth-scroll / animation walks, the nearest-flow search) and every
  climb for a box reads `slot::parent` (inline flow lookup, hit-test chains, scroll containers for the
  wheel / reveal / `scrollIntoView`, sticky scrollports, margin-collapse's formatting context, border
  priority, highlight styles). Rejected: a `BoxItem::Generated` slot laid out by its own code — every element
  path keyed by `NodeId` (rects, scroll offsets, gutters, line boxes, `layout_node`'s two-pass scrollbar,
  transitions) would have needed a second copy. What faces the DOM never names the node: `hit_test_path`
  (and `hit_test`, `elements_from_point`) leave it out — a hit on its border or padding is the `<details>`'s
  — while `position_at` reads the internal path, whose line boxes are the box's; transition events of the
  box fire on the `<details>` with `pseudoElement` `"::details-content"` (CSS Transitions 1 §6.1). Closed:
  the box stays, empty (the UA's `content-visibility: hidden`, whose content-skipping is still the content
  computing `display: none` — DIVERGENCES §2 rewritten to that). What animates: the box transitions as an
  element — paint properties interpolate; a `height` transition runs and fires events but layout reads the
  end value, as for every element (found here: geometry transitions never reached layout; DIVERGENCES §3
  under C12-ANIMATABLE), and there is no `interpolate-size`. Red: `css_phase10/details_content.rs` — 8 new
  tests failed on HEAD (no border drawn, `after` under an unclipped `two`, no closed box, (5, 3) for the
  inline-block, a border hit naming the `<p>`, no wheel scroll, no transition event); the margin test's
  first half (collapse through an unstyled slot) passed before, a guard. Green after, with
  `the_slots_content_keeps_its_tab_order_and_caret` (Tab order summary → slotted button → after, caret cell
  inside border and padding), `a_dropped_details_takes_its_slots_box_with_it` (the arena back to its size)
  and `box_tree_tests::a_details_box_tree_is_its_summary_and_its_slots_box`. Mutations (restored,
  touched): no reclaim → the drop test; `hit_test_path` keeping the box → the hit test; events left on the
  box → the transition test (together, each caught by its own); `slot::parent` ignoring the box → the
  scroll and caret tests; `children` ignoring the link → 9 of 13. No existing expectation or snapshot
  changed. CSS-COVERAGE `::details-content` Partial → Supported, §3.16 9 / 1 / 0 / 6, total 197 / 14 / 51
  / 45. TECH_DEBT `SIZE-1` recounted for the touched files (none past 575).
- 2026-10-08 — C10G-PSEUDO-MARKER (finishes C10-PSEUDO-CHAINS). Found: `li::before::marker` was an invalid
  selector ("at most one pseudo-element"), and a `display: list-item` `::before` / `::after` — which already
  incremented `list-item` (CSS Lists 3 §4.6: `counters::enter` reads the flag on any box) — drew no marker.
  Decisions (CSS Pseudo-Elements 4 §4, CSS Lists 3 §3.1): (1) rdom-style parses the one nesting CSS defines,
  `::before::marker` / `::after::marker` (also after the legacy `:before`), as `PseudoElementTarget::BeforeMarker`
  / `AfterMarker`: two pseudo-elements in the specificity (Selectors 4 §17), cut to `marker_subset`, taking
  trailing user-action pseudo-classes; any other nesting stays invalid. The UA's `::marker { white-space: pre }`
  now also names them (`*::marker` does not select a pseudo-element's marker; UA rules 173 → 175). (2) rdom-tui
  computes the marker right after its pseudo-element — `::before`'s in `early_pseudos`, `::after`'s after the
  children with the `::after` (`early_pseudos::after_marker`) — when that box is a list item, inheriting from it,
  its `content: normal` the `list-style-type` text of the `list-item` value the box just set; matched rules
  cached per slot (`Slot::BeforeMarker` / `AfterMarker`, 14 slots), the styles kept in `PseudoStyles`
  (`before_marker` / `after_marker`, accessors `computed_before_marker` / `computed_after_marker`), the slots
  `PseudoSlot` / `StyleSlot::BeforeMarker` / `AfterMarker` (no transitions, as `::marker`). (3) Layout: the
  marker rides the pseudo-element's own first line — `inline::pack_generated`, which packs the lines of every
  box a `::before` / `::after` makes (block-level, flex or grid item, float, positioned), pushes it first
  (`markers::pseudo_marker`, the element path's `feed::push_marker` with the marker's slot) — inside as the first
  inline box, outside hung beside the pseudo-element's border box (`markers::place_outside_of_box`, the element
  path's placement over a given box; `AnonymousItem::lay_out_content` takes the border box), `marker-side`
  reading the pseudo-element's direction and its host's (`hangs_right(item, slot)`). (4) The pseudo hit test
  looks for the marker on a generated box's lines before naming the box (innermost wins), so
  `::before::marker:hover` applies. Same predicate as an element's marker: a block container — an inline list
  item gets none, element or pseudo-element (found here: CSS-COVERAGE claimed a marker for `inline list-item`;
  DIVERGENCES §4 entry added, the row corrected). Red: rdom-style `a_before_or_after_takes_a_nested_marker` and
  `a_nested_marker_takes_trailing_user_action_pseudo_classes` failed ("at most one pseudo-element suffix allowed
  per selector, found 2"; `no_other_pseudo_element_nests` passed before, a guard); `css_phase10/pseudo_marker.rs`
  — 6 of 7 failed with the slot variants stubbed (`"x"` for `"1. x"`, `(h, Before)` from the pseudo hit test),
  the no-list-item guard passed. Green after, plus `every_box_a_before_makes_carries_its_marker` (flex item,
  grid item, float, absolutely positioned). On the way: the outside marker landed a cell right (`" 1.x"`) until
  the UA `white-space: pre` reached nested markers (its space was collapsed off the marker's width). Mutations
  (restored, touched): no marker pushed in `pack_generated` → 6 of 7; no outside placement and no nested pseudo
  hit together → the outside test and the hover test, each. Changed expectation: the UA rule count test (173 →
  175). No snapshot changed. CSS-COVERAGE: the pseudo-element-chain row Partial → Supported, `::marker` and
  `display: list-item` rows extended, §3.16 10 / 0 / 0 / 6, total 198 / 13 / 51 / 45; DIVERGENCES §3's
  pseudo-element line removed. TECH_DEBT `SIZE-1`: `walk.rs` 527 / 541.
- 2026-10-08 — C10G batch B closed (C10G-DETAILS-CONTENT-BOX, C10G-PSEUDO-MARKER): both Phase 10 partial
  items are done; §3.16 has no Partial row. Found on the way and recorded: geometry transitions never move
  layout (DIVERGENCES §3, C12-ANIMATABLE); an inline list item has no marker (DIVERGENCES §4).
- 2026-10-08 — C10G-INLINE-LIST-ITEM (found by C10G-PSEUDO-MARKER). Found: `display: inline list-item` — on an
  element, or on a `::before` / `::after` — incremented `list-item` but drew no marker, because markers were
  placed only on a block container's first line. CSS Display 3 §2.3 gives every list item a `::marker`, and
  CSS Lists 3 §3.5 says `outside` "is equivalent to `inside`" when the list item is an inline box. Fixed in the
  inline feed: `walk_inline_box` pushes an inline list item's marker (`markers::inline_marker`) as the box's first
  inline box, ahead of its `::before` (Pseudo-Elements 4 §3.1); `push_pseudo_text` does the same for an inline
  list-item `::before` / `::after` (`markers::inline_pseudo_marker`, the nested marker slot). Both are always
  inside. Measurement shares the feed, so shrink-to-fit widths count the marker. The block predicates
  (`marker`, `pseudo_marker`) are unchanged. Red: `css_phase10/inline_list_item.rs`, all 5 failed on HEAD
  (`"ab        "` for `"1. a2. b  "`, `"   a    "` for `"   ▪ a  "`, `"xa    "` for `"- xa  "`, no `→` marker,
  `"a       "` unmeasured); green after (the `::marker` rule's colour and the pseudo hit test naming
  `(span, Marker)` included). On the way: a `<span>` that is the root element is blockified (CSS Display 3 §2.7)
  and so hangs its marker off-screen, which is correct, so the test wraps it in a `<div>`. DIVERGENCES §4's
  "inline list item has no marker" entry removed; CSS-COVERAGE `display: list-item` and `::marker` rows
  corrected (counts unchanged, both rows were already Supported). No existing expectation or snapshot changed.
- 2026-10-08 — C10G-MIGRATION (API B1, N1). The six items Phase 10 reshaped from 0.5 had Breaking prose but no
  API-table row and no hint group. All six existed in `v0.5.0` (checked against the tag), so their rows go in
  the from-0.5 rdom-style table: `Length::Calc` (C9-CARRY-INDENT; `Box` → `Arc`, `Length::calc`), `CounterStyle`
  (a name, not a `Copy` enum), `CounterOp` (`#[non_exhaustive]`, `new` / `reversed`), `parse_counter_ops` (third
  argument), `Content` (`#[non_exhaustive]`, `Normal` for `normal`) and `PseudoElementTarget` (not `Copy`). The
  "Compile breaks" summary gains a counters-and-content line. Hint groups in `migration_hints.rs`:
  `counter_style_hints`, `counter_op_hints`, `generated_content_hints`, `pseudo_element_target_hints`; batch A's
  `calc_payload_hints` already built `Length::calc`, but through `layout::Length`, so it now names `Length` at
  the root and matches `Length::Calc`. Root re-exports added (N1): `CounterStyle`, `CounterStyleName`, `CounterOp`,
  `ListStyleType`, `ListStylePosition`, `ListStyleImage`, `MarkerSide`, `Length`; the prelude adds `Highlight`,
  `CounterStyle`, `ListStyleType` and `ListStylePosition`. Red: the new groups failed to compile under `use
  rdom_tui::*;` (20 errors, `cannot find type CounterStyle` …); green after (49 hint tests). Decision: the
  `@counter-style` rule types (`System`, `CounterStyleRule`, …) stay off the root (generic names) and are reached
  as `rdom_tui::style::counters::…` (rdom-style's module, re-exported beside `style::parse`), so
  `CounterStyle::symbols(System::Cyclic, …)` works with `rdom-tui` alone; the hint builds one. No existing expectation changed.
- 2026-10-08 — C10G-DESIGN-TYPES (API B2; the architect's DESIGN item). DESIGN listed `Content` as closed, but
  it is `#[non_exhaustive]` (C10-CONTENT), and it classified none of Phase 10's ~20 new public types. Every type
  is now decided. Open (`#[non_exhaustive]`): `Content` (Generated Content 3 / GCPM keep adding items; resolution
  lives in rdom-style, `Content::resolve`, where an unknown item resolves to nothing), `Quotes` (read through
  `pair()`; a WD grammar that already grew `match-parent`), the `@counter-style` descriptors (`CounterStyleRule`,
  `System`, `CounterRange`, `SpeakAs`, `CounterStyleDefinition`: an invalid descriptor is ignored, and formatting
  falls back to `decimal` inside rdom-style), `CounterOp` (grew `reversed()`; applied by the cascade),
  `HighlightType` (changes nothing painted) and `PseudoElementTarget`'s new variants. Options bag: `Highlight`.
  Closed: `ListStyleType`, `ListStyleImage`, `ListStylePosition`, `MarkerSide`, `QuoteKind`, `QuotePair`. Sealed
  by private fields: `CounterStyleName`, `CounterStyleRegistry`, `UserActionState`, and `HighlightRegistry` (with
  batch A's `generation()`); `Predefined` is a field-less handle. Crate-private: `ContentBoxLink`. Batch A/B's
  `PseudoStyles` was already classified; its new marker fields and the `BeforeMarker` / `AfterMarker` slots are
  noted there. Decision: closed means dropping `#[non_exhaustive]` from `ListStylePosition` and `MarkerSide`
  (placements layout must make, as `TextAlign` / `Float`), from `QuoteKind` (four keywords, each moving the depth
  its own way), and also from `ListStyleType` and `ListStyleImage`, whose grammars are fixed (`<counter-style> |
  <string> | none`, `<image> | none`) and whose growth lands inside a variant's payload. No spec reason was found
  for any of the five to grow. All five are new since 0.5, so relaxing them breaks nothing. Red:
  `css_phase10/list_item.rs::marker_and_quote_values_are_closed` (an exhaustive match outside rdom-style) failed
  to compile with 4 × E0004 (`_` not covered); green after. No wildcard arm in the workspace became unreachable
  (clippy clean). No existing expectation changed.
- 2026-10-08 — C10G-UPGRADE (API B3). The upgrade guide's silent changes from Phase 10 were item 10 (no ids)
  and four batch-A/B items appended unranked at the end (43–46). Nine missing changes were added and Phase 10's
  entries ranked by impact among the rest (55 items). Item 10 now carries its ids (C10-LIST-ITEM,
  C10G-MARKER-CLIP) and both consequences: an `li::before { color }` no longer colours the bullet (use
  `li::marker`), and `ul { padding: 0 }` hangs the bullets off-screen (use `list-style-position: inside`, or keep
  the padding). Item 11, after it: nested `◦` / `▪` bullets. After the 0.5-era layout items comes the more
  visible Phase 10 group: positioned pseudo-elements (43 → 24, trimmed from 939 to about 420 characters, dropping
  the overflow, min-content and static-position-after-`text-align` clauses the Log keeps); the `<details>` content
  box (46 → 25); a closed `<details>` hiding loose text (new); `<q>` quotes (new); the `type` hints (44 → 28); the
  `::selection` paired defaults (45 → 29); the `::selection` highlight subset, with its bold dropped (new); and the
  `::placeholder` subset (new). At the tail: `abbr[title]` dotted (moved from Fixed — rdom-style, where its bullet
  is removed), outside-marker hit-testing, `li { counter-increment: none }`, and this batch's inline list-item
  marker. The wider list padding is item 10's "four cells (two in 0.5)", so it gets no separate entry. "The
  357-character bullet" matched no bullet in the guide by characters or bytes, so the trim went to the longest
  Phase 10 one, item 43. Docs only, so no code or test changed.
- 2026-10-08 — C10G-HIGHLIGHT-API (API N3). The search-highlight use case from Rust had four rough edges, each
  fixed. (1) No descendant walk existed under any name: rdom-core had only the crate-private
  `walk_descendants` / `walk_subtree` callbacks, and `query_selector_all` returns elements only. The new
  `Dom::descendants(root)` (`traversal.rs`, the `Descendants` iterator) walks in tree order, includes text nodes,
  and uses no stack; it shares `next_in_subtree` with the private walks. (2) A checked range builder:
  `Dom::range_between(a, b)` (`range.rs`) checks each point as DOM §5.5 "set the start or end" does (an offset
  past `Dom::node_length`, or inside a UTF-8 character, is `InvalidOffset`) and orders the two by §5.2.
  Decision: two points in different trees are `InvalidState` rather than a collapsed range, because no range
  can hold them. (3) `HighlightsChanged` fired at every `highlights_mut()`, before the change. `highlights_mut`
  now returns a `HighlightsMut` guard (`Deref` / `DerefMut` to the registry). On drop it moves the generation
  and fires one record, but only when the registry marked a change: `set`, a `delete` that removed something,
  a `clear` of a non-empty registry, or a registered `Highlight` changed through `get_mut`. To make that last
  one knowable, `Highlight`'s members became private, with `add` / `delete` / `clear` / `set_priority` /
  `set_kind` marking real changes. Decision: a guard, not explicit registry methods, so `get_mut` and the map
  API keep the web's shape. It skips the observer call while unwinding from a panic. (4) Names: the web's
  `type` is `kind` throughout (`kind()`, `set_kind`, `with_kind`, was `with_type`). The web's `size` (setlike
  and maplike) is `len()` / `is_empty()` on both types, the Rust collection convention, documented on the
  module. `highlight.rs` became `highlight/{mod, registry, live}.rs` (173 / 234 / 85). Red: rdom-core
  `highlight_tests.rs` — the new `highlights_changed_fires_after_a_change_and_only_then`,
  `a_checked_range_validates_and_orders_its_points` and `descendants_walk_a_subtree_in_tree_order`, plus the
  renamed accessors, did not compile (18 × E0599); green after (9 tests). Mutations (restored, touched):
  firing before the change at every call → `[[]]` for `[["a"]]` ("after the change"); firing at every guard
  drop → 5 records for 1 ("nothing changed"). Docs: the rdom-tui README's new "Custom highlights: search
  results" doctest walks three paragraphs, highlights every "error", and asserts the text and the yellow cells
  of each row and the black text; `HighlightsMut` re-exported at the rdom-tui root; hint group
  `migration_hints.rs::highlight_hints` with an after-0.5 API-table row. Changed expectation: the existing
  registry test reads `len()` / `priority()` / `kind()` and `with_kind` (it still counts five records for
  five changes).
- 2026-10-08 — C10G-API-SMALL (API N4, N6, N8; the architect's `Backend` note). Five parts, red first where code
  changed. (1) `GeneratedFragment` gains `offset()` (a relative / sticky pseudo-element's shift, CSS 2.1 §9.4.3),
  `drawn_at()` (`(x, y)` moved by it — where paint draws and hit-testing finds it) and `is_outside_marker()`
  (CSS Lists 3 §3.5); the fields stay crate-private. Decision: accessors, not public fields, as the record's other
  reads (`is_atom`, `atom_rows`). (2) `TuiExt::positioned_pseudos()` returns an iterator of `PositionedPseudo {
  slot, border_box, content_box, lines }` (`ext/layout_cache.rs`, `#[non_exhaustive]`); the slice of
  `AnonymousIfc` it returned is the crate-private `positioned_pseudo_boxes()`, which paint, stacking,
  hit-testing and layout index (eight call sites renamed). (3) rdom-css's `InvalidCounterStyleRule` now means only
  "the rule defines nothing", `reason: CounterStyleRuleReason` (`MissingBlock`, `Unterminated`,
  `NotOneIdentifier`, `Rule(CounterStyleRuleError)`); a declaration dropped from a defined rule is the new
  `CounterStyleDescriptorDropped { name, descriptor, reason: CounterStyleDescriptorReason }` (`Malformed`,
  `Descriptor(DescriptorError)`); rdom-style's `check_rule` / `apply_descriptor` return the typed
  `CounterStyleRuleError` (`ReservedName`, `SymbolsDoNotSuitSystem`) / `DescriptorError` (`Unknown`,
  `InvalidValue`), each with `Display` and `Error`. Decision: the parse-level reasons live in rdom-css and wrap
  rdom-style's, so neither crate restates the other's. (4) SGR detection treats `ZELLIJ` as a multiplexer
  (`BASIC`), beside `STY`: Zellij leaves the outer `TERM` and terminal variables, and is not known to pass every
  extension on. (5) The `Backend` trait doc says a wrapping backend must forward `set_sgr_capabilities` /
  `sgr_capabilities` (provided methods, so a wrapper that omits them drops `App::with_sgr_capabilities`). Red:
  `css_phase10/layout_reads.rs` (3 tests) and rdom-css `counter_style.rs::invalid_rules_define_nothing` (now
  asserting each reason) / `a_dropped_descriptor_is_its_own_warning` failed to compile (the accessors, the
  iterator, the typed reasons and the new variant missing); `sgr_capabilities::tests::zellij_is_a_multiplexer`
  failed with the kitty `EXTENDED` set for `BASIC`. Green after. DESIGN classifies `PositionedPseudo`, the four
  reason enums, and C10G-HIGHLIGHT-API's `HighlightsMut` / `Descendants`. Changed expectations: the rdom-css
  invalid-rule test names each rule's reason (it matched any `InvalidCounterStyleRule`), and
  `css_phase8/containing_block.rs` reads `positioned_pseudos().next()` for the border box. CHANGELOG: the from-0.5
  `before_layout` row and two after-0.5 rows, hint group `generated_box_read_hints`.
- 2026-10-08 — C10G-DOCS (API N7, N9). (1) The rdom-core README's mutation record list gains `HighlightsChanged`,
  and `PreDetach`, which was also missing. A short "Custom highlights" section points to the rdom-tui README's
  example. (2) The rdom-tui README gains a "Lists, counters and generated content" section: what Phase 10 ships
  (markers, every predefined style, `@counter-style`, `counter()` / `counters()`, `::marker`, the HTML list
  attributes, quotes, `::first-line` / `::first-letter`), and a doctest painting an `upper-roman` `ol`, a nested
  `counters(list-item, ".")` outline on `::marker`, and a custom `@counter-style` bullet. It asserts every row
  and the `::marker` colour on the marker cell but not the text. The roman list keeps the UA padding and sets a
  `margin-left`, so the rows pin the four-cell list padding. Mutation (the UA padding set back to 0.5's 3 / 2
  cells, restored, touched): the doctest fails with `"  I. one"` / `"✓ milk"` for `"   I. one"` /
  `"  ✓ milk"`, so it would have caught the marker-clip bug. (3) ACID tile 9 is now three tiles: 9a counters and
  generated content (with the positioned pseudo-elements and `<details>`); 9b lists and markers, gaining the
  column-0 cases (a ten-item `ol`, an `upper-roman` list at the edge and beside a `margin-left`), nested
  bullets, `<ol type>`, an inline list item and an outside `::marker:hover`; 9c first line, first letter and a
  search highlight built from Rust (overlap with the selection stays in tile 17). I12 points at 9a–9c. (4) The
  C12-ANIMATABLE row of the Phase 12 table now says, in bold, that geometry transitions never reach layout
  (found by C10G-DETAILS-CONTENT-BOX). Docs and a doctest only; no production code changed.
- 2026-10-08 — Phase 10 closed: 19 gate fixes, all at the root — batch A (correctness and cost: C10G-SPLITS,
  -MARKER-COST, -MARKER-CLIP, -MARKER-HIT, -SELECTION-PAIRED, -HIGHLIGHT-COST, -IDLE-SCANS, -INHERIT-COST and its
  split, -TUIEXT-SIDE, -MINOR), batch B (the two partial items: -DETAILS-CONTENT-BOX, -PSEUDO-MARKER) and batch C
  (API, migration and docs: -INLINE-LIST-ITEM, -MIGRATION, -DESIGN-TYPES, -UPGRADE, -HIGHLIGHT-API, -API-SMALL,
  -DOCS). Every Phase 10 row is done and §3.16 has no Partial row; the fixes' re-review rides with the Phase 11
  gate. Carried forward: geometry transitions reaching layout (C12-ANIMATABLE, flagged on its row),
  `interpolate-size` for `::details-content`, and closed `<details>` content computing `display: none` until
  C14-CONTAIN brings `content-visibility`.
- 2026-10-08 — C11-SPLIT (before Phase 11): `rdom-core/src/query_selector.rs` (519 production lines) split by
  concern — `query_selector/mod.rs` (the query APIs), `matcher.rs`, `pseudo.rs`, `attribute.rs`, `tests.rs`.
  Pure move.
- 2026-10-08 — C11-ATTR-FLAGS: attribute selector case flags (Selectors 4 §6.3). `SimpleSelector::Attribute`
  gains `case: AttrCase` (`Default` / `AsciiInsensitive` / `Sensitive`, `#[non_exhaustive]`; a Breaking
  bullet); the parser reads an `i` / `s` identifier (ASCII case-insensitive) after the value — after a string
  or an identifier, white space optional between a string and the flag — and rejects any other identifier
  there and a flag without a value (`[a i]`). The flag overrides HTML §4.16.2's case-insensitive list either
  way, for every operator. Decided — the `<ol type>` / `<ul type>` / `<li type>` mapping stays a
  presentational hint: HTML §15.3.8 writes it as `ol[type=a s]` rules "expected to apply, as presentational
  hints", not as UA-sheet rules, so C10G-MARKER-HIT's DIVERGENCES §2 sentence (a browser's UA rule would
  survive an author `revert`) was wrong — a browser's hint is author-level too. The sentence went; the
  §2 list-attribute entry now names `type` among the hints and how its values compare, and `hints.rs` cites
  HTML's text. Red: `query_selector::tests::attribute_case_flags_override_the_default_case` failed with
  `Err("expected `]` in attribute selector, got `i`")`; the parse test failed to compile (`case`,
  `AttrCase`); green after, with `css_phase11/attr_flags.rs` through a sheet (green as written — the
  implementation was in). Mutation (both flags made to defer to the HTML list, restored, touched): the
  matcher test and the sheet test fail. Changed tests: the two parser tests that build or destructure
  `Attribute` gained `case` / `..` (API shape, no expectation). No snapshot changed.
- 2026-10-08 — C11-NTH (Phase 11 cites Selectors 4 by the W3C WD of 2022-11-11, where child-indexed
  pseudo-classes are §13.3, typed ones §13.4, specificity §15; older citations in the code follow an earlier
  draft's numbering — §17 specificity, §14.x the structural pseudo-classes): `:nth-child(An+B [of S])`, `:nth-last-child()`, `:nth-of-type()`, `:nth-last-of-type()`
  (Selectors 4 §13.3–§13.4; `SimpleSelector::Nth(Box<NthSelector>)`, `NthKind`, specificity one pseudo-class
  plus `S`'s most specific, §15) and `:first-of-type` / `:last-of-type` / `:only-of-type` (`PseudoClass` variants).
  An+B (CSS Syntax 3 §6.2) is scanned from the text in `selectors/anb.rs`, token shapes kept: white space
  allowed around a binary sign and before `of`, none between a sign or coefficient and its `n` (`+ n`, `2 n`
  rejected), `odd` / `even` / `n` ASCII case-insensitive, terms clamped to `i32`. The pseudo-class parsing
  moved to `selectors/pseudo_parser.rs` (`parser.rs` would have passed 500). Matching cost: a per-pass
  nth-index cache (`rdom_core::SelectorCaches`, `query_selector/caches.rs`), as Blink's / Servo's — the first
  match under a parent indexes all its children for that kind of count (every element, per type, or per `of
  S` list keyed by the list's address), later matches read it. `Dom::matches_list_with(id, list, scope,
  &mut caches)` is the pass API (`matches_list_in_scope` uses fresh caches); the query APIs share one per
  call; rdom-tui's `matching::Scratch` holds one per cascade pass, threaded through `scope::match_rule`.
  Safety: `Dom::mutation_epoch` moves with every mutation record (observed or not), and caches built under
  another epoch are dropped before use. Invalidation: a child-list change already marks every child of the
  parent (once per drain), which covers indices moving at either end — not the document; an `of S` reads
  its siblings' state, so `SiblingTriggers` records `S`'s compounds as left-of-sibling compounds wherever the
  `:nth-*()` sits, and a class / attribute change `S` reads marks the parent's children. Decided — Selectors
  4 §13.3 (Level 4) lets a parentless element match: it is index 1 of 1, and `:first-child` / `:last-child` /
  `:only-child`, which required a parent, now match one too (a Fixed bullet). Red: the rdom-core matching
  tests failed with "unsupported pseudo-class `:nth-child`", the counting test to compile (no
  `caches::probe`), the rdom-tui integration tests on `selector_walk`'s unknown-selector assert, the
  parentless test with `:first-child` → `Ok(false)`; green after. Mutations (each alone, restored, touched):
  no cache reuse in rdom-core → `nth_matching_over_a_long_sibling_list_is_linear` (2000 siblings); no epoch
  guard → `selector_caches_do_not_outlive_a_mutation`; fresh caches per rule in rdom-tui →
  `cost_tests::nth_child_over_a_long_list_indexes_it_once_per_pass`; no `of S` trigger →
  `nth_of_s_reads_what_s_reads` and `css_phase11/nth.rs::a_sibling_matching_of_s_or_not_restyles_the_others`.
  The counting test's bound counts child nodes (the list interleaves text nodes): 4002 steps for 2000
  elements on first run, bound set to 3N. No existing expectation or snapshot changed.
- 2026-10-08 — C11-LINK-LANG (with `:dir()`, deferred here by C5-WRITING). `:any-link` / `:link` (Selectors 4
  §8.1–§8.2) match `a` / `area` with `href` (HTML §4.16.3); `:visited` parses and never matches — rdom keeps
  no history (DIVERGENCES §1) — so `a:link, a:visited` keeps its `:link` half instead of dropping the rule.
  `:lang()` (§7.2; `SimpleSelector::Lang(Vec<String>)`) takes `<ident>` / `<string>` ranges (a bare `*` must
  be escaped or quoted) and matches by RFC 4647 §3.3.2 extended filtering, ASCII case-insensitively, against
  the new `Dom::language(id)` (HTML §3.2.6.2: the nearest `xml:lang` / `lang`, `Some("")` when empty) — the
  lookup `quotes: auto` now shares. Decided — an unknown language (empty or absent) is matched by the empty
  range only, never by `*`. `:dir()` (§7.1; `PseudoClass::Dir(Option<Directionality>)`, another argument
  valid and matching nothing) matches the new `Dom::directionality(id)` (`directionality.rs`, HTML §3.2.6.4:
  `dir` `ltr` / `rtl`, `auto` and a `<bdi>` without a valid `dir` by the first strong character of the
  value or contained text — skipping `<bdi>`, `<script>`, `<style>`, `<textarea>` and elements with a valid
  `dir` — `ltr` without one; `<input type=tel>` `ltr`; else the parent's; the root `ltr`). Decided — `:dir()`
  follows HTML directionality, not the CSS `direction` property, as Selectors 4 §7.1 says (pinned: a
  `direction: rtl` ancestor does not make `:dir(rtl)` match). The strong-character classes are approximated
  without a Bidi_Class table (DIVERGENCES §2, new entry). A pass memoizes directionality in
  `SelectorCaches`. With `:dir()` in place the UA's `dir` rules became HTML §15.3.5's own
  (`[dir]:dir(ltr), bdi:dir(ltr), input[type=tel i]:dir(ltr)` / `[dir]:dir(rtl), bdi:dir(rtl)`), so `dir=auto`
  sets `direction` (the DIVERGENCES §1 "treated as no `dir`" sentence went). Invalidation: `lang` / `dir`
  changes mark the subtree already; a text edit or child-list change marks the nearest `dir=auto` / `<bdi>`
  host whose contained text it is part of (`dirty_tracker::marks::mark_auto_direction_host`, O(depth), every
  frame — the UA sheet reads `:dir()`); `SiblingTriggers` records `lang` / `xml:lang` for a `:lang()` left
  of `+` / `~`. Split: `style/dirty_tracker.rs` (567 production lines, past 575 with this change) became
  `dirty_tracker/{mod,observe,marks,tests}.rs`. Section numbers here and in the new tests follow the 2022
  WD (§7.1, §7.2, §8.1, §8.2). Red: `linguistic_tests` failed to compile (`Directionality`,
  `Dom::language`, `Dom::directionality`); with those tests set aside, the link and `:lang()` tests failed on
  "unsupported pseudo-class `:any-link`" / "`:lang`"; `css_phase11/link_lang.rs::dir_auto_sets_direction_from_its_text`
  (`Ltr` for `Rtl`) and `editing_text_under_dir_auto_restyles_it` (`Reset` for red after the edit); green
  after. Mutation (the auto host left unmarked, restored, touched): the editing test fails. Changed
  expectation: the UA rule count 175 → 178 (`ua_total_rule_count`: the two `dir` rules now hold five selectors).
  No snapshot changed.
- 2026-10-08 — C11-SCOPE: the query methods match with the node they were called on as the scoping root
  (DOM §4.2.6 "scope-match a selectors string", Selectors 4 §8.4): `query_selector_in` /
  `query_selector_all_in` with their root (the document's `query_selector` / `query_selector_all` with the
  document node, so `:scope > div` finds the top-level elements and `:scope` alone none), `matches` and
  `closest` with their element. Checked, no change needed: `:scope` in `@scope` (the scoping root, also
  inside `:is()` / `:not()` arguments) and at a sheet's top level (`:root`, the root element in a tree whose
  root is one) — two cascade tests pin them (`scope_tests.rs`). Red:
  `query_methods_scope_to_the_node_they_are_called_on` (`[]` for the two `span`s) and
  `matches_and_closest_scope_to_their_element` (`matches(em, ":scope")` was `false`); green after; the two
  cascade checks green as written. No existing expectation or snapshot changed.
- 2026-10-08 — C11-COMBINATORS (found while designing C11-HAS, whose anchored matching needs it): two matcher
  bugs. (1) `matches_complex` took the nearest candidate for each compound and never backtracked, so `div > p
  span` failed for a `span` whose nearest `p` sits in a `section` under the `div`'s `p` (Selectors 4 §3.1:
  some assignment must satisfy every combinator). Rewritten as `match_chain` with Servo's outcomes
  (`Matched`, `NotMatchedGlobally`, restart from the closest descendant / later-sibling combinator), which
  bound the backtracking; it also takes an optional anchor (the `:has()` element) as the chain's last step.
  (2) `+` / `~` stepped to the previous *node*, so a text node or comment between `h1` and `p` broke `h1 + p`;
  they step to the previous element sibling (§14.3 / §14.4 in the earlier draft's numbering). Red:
  `complex_selectors_backtrack_past_the_nearest_candidate` and `sibling_combinators_skip_text_and_comments`
  (`Ok(false)` for `Ok(true)`); green after. No existing expectation or snapshot changed.
- 2026-10-08 — C11-HAS: `:has(<relative-selector-list>)` (Selectors 4 §4.5). rdom-core: `SimpleSelector::Has(Vec<
  RelativeSelector>)` (`combinator` — `Descendant` without a leading one — and `selector`; `#[non_exhaustive]`),
  parsed unforgiving, `:has()` invalid anywhere inside another (`Parser::in_has`; inside a forgiving `:is()`
  the inner `:has()` argument is dropped, as for any invalid argument), specificity its most specific
  argument (§15). Matching (`query_selector/has.rs`) tries each element the relative selector can reach as its
  subject — the anchor's descendants (children only for `>` without a descendant step), its later siblings
  (the next only for `+` without a sibling step) and their descendants when the selector steps down — and
  reads the rest with C11-COMBINATORS' `match_chain`, the anchor as its last step. Cost: answers per
  (relative selector, anchor) in `SelectorCaches`; the plain `:has(<compound>)` computes, bottom-up, whether
  each element's subtree holds a match and records them all, so nested anchors cost one walk
  (`has_over_a_deep_chain_is_linear`: 400 anchors, ≤ 3N visits; the rdom-tui pass shares it,
  `cost_tests::has_over_a_deep_chain_walks_it_once_per_pass`). The other forms search once per anchor per pass
  (TECH_DEBT `HAS-COST-1`). Invalidation, in Blink's spirit: (1) the caches list every anchor a `:has()` was
  evaluated for; each cascade pass flags them (`TuiExt::has_anchor`, sticky; `doc_flags::note_has_anchor`);
  (2) `style::has_triggers::HasTriggers::of_sheets` records what the arguments read — attribute names
  (`class` / `id` / `[name]` / `lang`), state and attribute-reading pseudo-classes, sibling reach — and the
  App hands it to the tracker with the sibling triggers; (3) the tracker, on a change a trigger fires for
  (`mark_state_dirty`, which interaction chains, attribute / class changes and text-emptiness flips go
  through) and on every child-list change, walks the changed element's ancestors (the parent inclusive for a
  child-list change) and, with sibling reach, each one's earlier siblings, marking flagged anchors only
  (`Cause::Has`, which a `:has()` left of `+` / `~` turns into a sibling mark via `SiblingTriggers`). Zero
  work without a `:has()` rule (empty triggers) or before any anchor was flagged
  (`has_invalidation_costs_nothing_without_a_has_rule`: 0 steps; ≤ depth + 1 with one). Found by mutation and
  fixed before commit: with the tracker's default "every change" triggers, `Cause::Has` fired too, so marking
  an anchor walked again from it — exponential in the depth of nested anchors (a 30-deep chain hung); `fires`
  now never fires for `Cause::Has` whatever the sheets (pinned:
  `has_invalidation_with_unknown_sheets_walks_once_per_change`, which hung before). `selector_walk::argument`
  became `arguments` (an `:nth-*()`'s `of S` and a `:has()`'s relative selectors are argument lists too), so
  `uses_validity` sees `form:has(:invalid)`. Red: the rdom-core matching / specificity tests failed on
  "unsupported pseudo-class `:has`" and the counting test to compile (`has_anchors`, `has_nodes`); the rdom-tui
  integration tests (`css_phase11/has.rs`) failed with `Reset` for the anchors' colors after a class change
  inside, a child inserted into a wrapped `ul`, and `:hover` / `:checked` / `:focus` (two passed at first only
  because the test's own insertions into the root marked everything — restructured so each case reaches the
  anchor only through `:has()` invalidation); green after. Mutations (each alone, restored, touched): no
  subtree memo → the two linear tests; the tracker walk off → the four invalidation tests; anchors never
  flagged → the same; the sibling walk off → `a_change_in_a_later_sibling_restyles_the_anchor`; walking
  without the trigger check → the zero-cost test (it hung: the recursion above). Changed expectation:
  `tui_ext_size_tripwire` 376 → 384 (the anchor flag; the small fields had no padding left). No snapshot
  changed.
- 2026-10-08 — Phase 11 part 1 docs: ACID tile 4 lists the selectors part 1 shipped (case flags, the `:nth-*`
  family with `of S`, `:link` / `:any-link` / `:visited`, `:lang()`, `:dir()` on `dir=auto`, `:has()` in its
  four relations, `:scope`, the backtracking and text-skipping combinator cases) and a stage-2 step I13
  exercises their invalidation. C11-COLUMN moved to Phase 13 as C13-COLUMN (the column combinator selects
  the cells a column spans; rdom has no column model until C13-TFC); DIVERGENCES §3 and the coverage row
  say so. §3.17 now: 25 Supported, 1 Partial (`:indeterminate`, part 2), 8 Missing (the part 2 form and
  display states, `:blank`, the column combinator), 4 N/A. Part 2 (C11-FORM-STATES, C11-MODAL-POPOVER) is
  not started.
- 2026-10-08 — C11-FORM-STATES, part 1 of 4: `:read-only` / `:read-write` (Selectors 4 §14.3.1, HTML §4.16.3;
  `PseudoClass::ReadOnly` / `ReadWrite`). rdom-core answers it from attributes and tree shape
  (`form_pseudo.rs`, `Dom::is_read_write`): a mutable `<input>` that `readonly` applies to (the list
  `will_validate` already used), a mutable `<textarea>` (mutable: no `readonly`, not actually disabled — a
  `<fieldset disabled>` counts), and any other element that is an editing host or editable
  (`Dom::is_editable_or_editing_host`, HTML §6.8.1: the nearest explicit `contenteditable` state up the
  tree decides). `:read-only` is every other element. Invalidation needs nothing new: `readonly`,
  `disabled` and `contenteditable` are attributes, whose changes mark the subtree, and `:has()` treats the
  two as attribute-reading. Red: `form_state_tests` failed on "unsupported pseudo-class `:read-write`";
  green after, with `css_phase11/form_states.rs` through a sheet and an App (green as written — the
  implementation was in). Mutation (the inherited `contenteditable` walk cut to the element itself,
  restored, touched): `read_write_follows_htmls_mutability_rules` fails. No snapshot or existing
  expectation changed.
- 2026-10-08 — C11-FORM-STATES, part 2 of 4: `:indeterminate` (Selectors 4 §14.4.3, HTML §4.16.3) on checkboxes
  and radio groups (the `<progress>` case was in). Decided — the checkbox's indeterminate flag stays reflected
  into an `indeterminate` attribute, the storage `TuiAccessorsMut::set_indeterminate` already used: it is
  rdom's model for live control state (`checked`, `value`, `selected` — DIVERGENCES §2, extended), and
  attribute invalidation, `:has()` and serialization need nothing new. Activation clears it with the flip
  and a canceled click restores both (HTML §4.10.5.1.15, `ToggleUndo::Checkbox { was_indeterminate }`); the
  UA draws `[-] ` (one rule after `:checked`, UA count 178 → 179). A radio matches while its group has no
  checked member (`Dom::is_indeterminate`; `SelectorCaches::radio_unchecked` answers a whole group per pass,
  `CacheWork::radio_group_walks`). Invalidation: checking one radio changes its siblings' match with no
  mutation on them, so the per-frame validity marks became `FormStateMarks` (bits per element: validity,
  radio indeterminate), tracking radio groups only when an author sheet reads `:indeterminate` — the UA's
  rule is checkbox-only (pinned: `the_uas_indeterminate_rules_are_checkbox_only`), so no App pays a walk for
  it. Found and fixed on the way: the marks marked a flipped element with `mark_dirty`, around the
  tracker's `:has()` walk, so `div:has(:invalid)` missed `set_custom_validity`; they now mark through
  `DirtyTracker::mark_state_changed` (the state path). Red: the core tests failed to compile
  (`is_indeterminate`, `radio_group_walks`) and, with those lines set aside, on `false` for an indeterminate
  checkbox and the radio-pass assert; the toggle tests on the flag left set and `[ ]` painted; the App tests
  with `Reset` for the group and the checkbox; the `:has(:invalid)` test with `Reset` (run against the old
  `mark_dirty` path). Green after. Mutations (each alone, restored, touched): marks via `mark_dirty` →
  `checking_a_radio_restyles_its_whole_group` (its `:has()` anchor) and the custom-validity test; no radio
  tracking → the group test. Changed expectations: the UA rule count (one rule); the marks' one-walk test
  reads the validity bit. No snapshot changed.
- 2026-10-08 — C11-FORM-STATES, part 3 of 4: `:default` (Selectors 4 §14.4.2) and `:in-range` / `:out-of-range`
  (§14.3.3–§14.3.4), with HTML §4.16.3's definitions. The defaults (`defaultChecked`, `defaultSelected`) and
  the range states live in rdom-tui, so rdom-core asks through a new hook in the validity hook's pattern:
  `ControlState` (`#[non_exhaustive]`: `DefaultChecked`, `DefaultSelected`, `RangeLimited`, `OutOfRange`),
  `ControlStateHook`, `Dom::set_control_state_hook`, `Dom::control_state` (without a hook the defaults are
  the content attributes and nothing has range limitations); `validation::install` installs it with the
  validity hook (`form_state::control_state`). The default button is the form's first submit button in tree
  order (HTML §4.10.21.2), found once per form per pass (`SelectorCaches::default_buttons`,
  `CacheWork::default_button_walks`). Range limitations: `min` / `max` on `number` and the date-like
  states, and every `range` input. The date-like states parsed nothing before: `validation/dates.rs`
  implements HTML §2.3.5's date, month, week, time and local date-time microsyntaxes (an unparsable value is
  no value), and those states now suffer `rangeUnderflow` / `rangeOverflow` — a `time` whose `max` precedes
  its `min` is a reversed range across midnight (HTML §4.10.5.3.7) — reading the `value` attribute, as they
  are not text fields (DIVERGENCES §2 entry rewritten; `step` still unchecked for them). Invalidation:
  value / `min` / `max` are attributes; `:default` changes far from the element (an earlier submit button
  inserted elsewhere in the form, `set_default_checked`) — `FormStateMarks` gained a `:default` bit,
  computed when a sheet uses `:default` with one pass's caches, and the default setters note a state write.
  Red: the core tests failed to compile (`ControlState`, `set_control_state_hook`, `default_button_walks`);
  the App tests on "invalid selector `:in-range`" / "`:default`" and the date validity on
  `range_underflow == false`; green after (test fixes on the way: buttons and options have UA colors, so
  "unstyled" became "not the rule's color"; the range test runs in an App, which seeds number fields from
  `value`). Mutations (each alone, restored, touched): no `:default` bit → both `:default` App tests; no
  reversed time range → the range test ("inside a reversed time range"). No snapshot or existing
  expectation changed.
- 2026-10-08 — C11-FORM-STATES, part 4 of 4 (item done): `:user-valid` / `:user-invalid` (Selectors 4
  §14.4.4–§14.4.5, HTML §4.16.3: an `<input>` / `<textarea>` / `<select>` with its *user validity* set that is
  a candidate and valid / invalid; `Dom::user_validity_state`, `ControlState::UserValidity`). The flag lives
  in the form builtins (`FormControlState::user_validity`), set by HTML's rules: wherever a builtin fires
  `change` (`form_state::fire_change` sets it first — the toggle, number step, slider and `<select>` paths
  now share it), and on a submission attempt for every submittable element the form owns, before the
  no-validate check (HTML §4.10.21.3); the reset algorithm clears it. Text controls had no commit at all
  (DIVERGENCES: "`change` … not on text-input blur"): a user edit or undo / redo now marks a pending change
  (`change_pending`, beside `value_user_edited`), and losing focus commits it — `change` before `blur`, as
  browsers order them (`focus::change_focus` → `form_state::commit_pending_change`); a programmatic value
  cancels it. Decided — Enter in a single-line field does not commit (browsers do); the DIVERGENCES line now
  says exactly that. Invalidation: `FormStateMarks` gained a user-validity bit (and computes validity when a
  sheet reads only `:user-*`), marked through the state path so `:has(:user-invalid)` follows. Red: the App
  tests failed on "invalid selector `input:user-invalid`" (the core test was written before the core change
  but first run after it — its red was shown by mutation: `user_validity_state` returning `None` fails it);
  green after (test fixes: the anchor's color inherited into the fields, so it is checked by background; a
  re-focused field's caret is at its start, so the test presses End). Mutations (each alone, restored,
  touched): no commit on blur → the blur test; no submission flag → the submission test; no reset clear →
  the same; no user-validity bit in the marks → only the new `a_novalidate_submission_still_sets_user_validity`
  (the other tests' fields were restyled by focus moves) — added for it. No snapshot or existing expectation
  changed.
- 2026-10-08 — C11-MODAL-POPOVER, part 1 of 3: the top layer and `:modal`. Found: rdom had no top layer — a
  modal dialog was marked with an rdom-internal `data-rdom-modal` attribute, painted in flow and again
  after a backdrop post-pass, and clicks outside it reached the page (DIVERGENCES). rdom-core now keeps the
  document's top layer (`top_layer.rs`: an ordered set with a `TopLayerKind` per member — `ModalDialog`,
  `Popover` — beside the focus state; changes fire `InteractionChanged { kind: TopLayer }`, which the dirty
  tracker's state path already handles; the removing steps take a removed subtree's members out, reported
  while connected). `:modal` is a member as `ModalDialog` (Selectors 4 §11, HTML §4.16.3; no fullscreen).
  rdom-tui: `dialog::show_modal` adds the dialog, `close` / `show` remove it (`is_modal`, `top_modal` read
  the top layer). Rendering, per CSS Position 4: a member whose `position` is not `absolute` / `fixed`
  computes to `absolute` (`cascade::blockify::finalize_top_layer`) with the viewport as containing block
  (`positioning::containing_block`; not part of a scroller's overflow); `stacking::collect_layers` leaves
  members and their subtrees out, and `paint_pass::top_layer` (replacing `backdrop.rs`) paints each rendered
  member after the document, bottom to top, its `::backdrop` first, as a stacking context clipped by the
  viewport only. Hit-testing tries the top layer first, topmost first; below a modal dialog a miss lands on
  its `::backdrop`, so the dialog is the target and the page is inert (HTML §6.3). UA: HTML's `dialog:modal`
  placement (fixed, inset 0, auto margins, fit-content, capped at `calc(100% - 2)`; UA count 179 → 180).
  Decided — the non-modal dialog stays in flow and the UA's backdrop untinted (DIVERGENCES §2 entry
  rewritten): moving every dialog out of flow would re-lay every existing `<dialog open>` page. Red: the
  core tests failed to compile (`TopLayerKind`, `InteractionKind::TopLayer`, `add_to_top_layer`) and then on
  "unsupported pseudo-class `:modal`"; the App tests with an empty top layer, `Static` for `Absolute`, the
  dialog in flow (`x: 0, y: 1, width: 20`), the page under the point and no backdrop / top-layer paint;
  green after. Mutations (each alone, restored, touched): no stacking-walk exclusion →
  `a_top_layer_element_paints_once` (a translucent dialog blended twice, 192 for 127); no inertness → the
  hit test; no viewport containing block → `position_computes_to_absolute_against_the_viewport`; no
  position forcing → the same. Changed tests: the three backdrop paint tests and the translucent-backdrop
  color test put the dialog in the top layer instead of setting the marker, and check cells around the
  centred dialog (expectations moved with the UA's centring); three dialog form tests click the submit
  button where it is laid out — two of them clicked the dialog's border before and passed without
  submitting. No snapshot changed (the showcase's dialog is non-modal).
- 2026-10-08 — C11-MODAL-POPOVER, part 2 of 3: the `popover` attribute and `:popover-open` (HTML §6.12).
  rdom-core: `:popover-open` is a top-layer member as `TopLayerKind::Popover`; `DomError::NotSupported`
  (`NotSupportedError`); `ToggleDetail::source` (`ToggleEvent.source`). rdom-tui: `runtime::builtins::popover`
  (`mod.rs` the API and contract, `algorithms.rs` HTML §6.12.2's check popover validity, show / hide popover,
  hide all popovers until, hide popover stack until, close entire popover list, topmost popover ancestor,
  nearest inclusive open popover; `invoker.rs` the `popovertarget` activation behavior; `attribute.rs` the
  attribute change steps). Decided — visibility is top-layer membership and the showing auto / hint lists
  are the top layer's popovers by the mode each was shown in (`Popovers::opened`, document data), so the
  removing steps rdom-core already runs keep the lists right; the per-element state (opened mode, invoker,
  previously focused element, the showing-or-hiding flag) is document data, not `TuiExt` (no size cost).
  Decided — `toggle` fires synchronously like `<details>` / `<dialog>`'s (DIVERGENCES §2, new entry with the
  Tab order: no invoker-adjacent navigation), and the attribute change steps run at the App's next boundary
  (`PopoverAttributes`, beside `Selectedness`; mutation observers may not mutate). The hint rules follow
  §6.12.2 step 12: a hint with no hint ancestor but an auto ancestor joins the auto stack. A stack walk
  whose hide leaves its popover showing (a listener took the attribute away) drops it from the top layer,
  so the walk always ends. The dialog focusing steps became `dialog::focusing_steps`, which a
  `<dialog popover>` shares. UA: HTML's three popover rules (UA count 180 → 183; `padding: 0.25em` is
  under a cell). Red: the core `:popover-open` test failed on "unsupported pseudo-class"; the rdom-tui tests
  were written before the module but first run after it (their red was the missing module), so each one's
  red is shown by a mutation (each alone, restored, touched): `beforetoggle` uncancelable → the cancel test;
  an auto popover hiding nothing → the stack and hint tests; no invoker nesting → the stack test; no hiding
  of nested popovers first → the order test; no focus restore → the focus test; `show` toggling → the
  invoker test; no attribute watch → the attribute test; hints ignoring the stack rules → the hint test; the
  UA hide rule broken → the integration test. No snapshot or existing expectation changed besides the UA
  count.
- 2026-10-08 — C11-MODAL-POPOVER, part 3 of 3 (item done): popover light dismiss (HTML §6.12.2 "light dismiss
  open popovers", the close watchers of auto / hint popovers). `popover/light_dismiss.rs`: the mouse router
  reports every button's press and release (`pointer_down` / `pointer_up`, before the `mousedown` /
  `mouseup` dispatch, as Blink runs it with the pointer events); the release hides the popovers above the
  *topmost clicked popover* (the open popover the point is in or whose invoker it is on, by stack position)
  when the press began in the same one. Esc: `topmost_close_watcher` is the topmost modal dialog or auto /
  hint popover in the top layer; the popover builtin hides a popover there, the dialog builtin defers to
  it and otherwise cancels its dialog — each claims the key (`prevent_default`), so one press closes one
  watcher. `dialog::show_modal` now runs HTML §4.11.4's popover step (hide the auto / hint popovers not
  holding the dialog, `popover::hide_unrelated_to`). Decided — no focus-based light dismiss: HTML has none
  (focus leaving a popover leaves it open, pinned by `focus_moving_out_does_not_dismiss`). Red: the light
  dismiss tests failed with the popovers left showing and `under` nested wrongly — the latter was the
  test's error (HTML: an auto popover inside a modal dialog nests in no popover, and `showModal()` hides
  the unrelated ones), rewritten to the spec before the code; green after (a click point moved off the
  nested popover's middle). Mutations (each alone, restored, touched): release hides nothing → the
  outside and nested tests; no same-target check → the drag-out test; invokers not counted → the invoker
  test; the dialog not deferring → the Esc order test; the dialog not claiming its Esc →
  `one_esc_closes_one_watcher` (added for it); `showModal()` hiding nothing → the Esc order test; no
  release hook for the other buttons → `a_right_click_outside_dismisses_too`. Phase 11's items are done;
  its gates are pending.
- 2026-10-08 — Phase 11 part 2 docs: ACID tile 15 lists the form states, the modal dialog's top layer and a
  popover; stage-2 steps I14 (user validity: the blur commit, a submission attempt, a reset, a checkbox) and
  I15 (popover light dismiss: invokers, nested stacks, a drag out, a click outside, Esc against a modal
  dialog, a manual popover) exercise them. CSS-COVERAGE §3.17: 32 Supported, 0 Partial, 2 Missing (`:blank`,
  a decided exclusion; the column combinator, C13-COLUMN), 4 N/A. Phase 11: items done, gates pending (the
  Phase 10 fixes' re-review rides with them).
- 2026-10-08 — Phase 11 gates (with the C10G re-review: all 19 at the root but one parent climb).
  Architect: 2 blocking — a modal dialog does not make the page inert to the keyboard (focusing steps
  focus nothing when the dialog has no focusable content; Enter activates the button underneath);
  popover hide loops (`close_entire_list`, `hide_stack_until`) have no progress bound, so ping-ponging
  `beforetoggle` handlers hang the App in raw mode. API: 3 blocking — no Phase 11 silent change is in
  the upgrade guide (modal centring / inertness, `change` on blur, `dir=auto`, backtracking, `+` / `~`
  skip text, `:scope` in queries, `showModal` hides popovers, `[-]` glyph, parentless `:first-child`)
  and "Compile breaks" lacks `SimpleSelector::Attribute`; top-layer elements paint no fill (`Canvas`
  resolves to `Reset`, which `fills` treats as none; the initial background is `Reset`; `dialog` has
  no UA background — a modal over text shows the page through); DESIGN classifies no Phase 11 type.
  Non-blocking: the pseudo hit test and `children_are_items` skip the `::details-content` box (hover on
  generated content in `<details>` never matches); `:has(p div)` climbs past the anchor (~N³);
  the `:has()` sibling walk is global and unbounded; `dir=auto` restyles its subtree on every edit;
  `showModal()` misses HTML's guards and swallows `add_to_top_layer` errors; popover removing steps
  (nested-through-invoker, unpruned maps, stale auto ancestor), an unrendered modal leaves the page
  clickable, a popover above a modal stays hit-testable; a mid-pass mutation drops `:has()` anchors;
  selector cache keys ignore the scoping root; `NthKind` / `AttrCase` / `Directionality` closed by spec
  but `#[non_exhaustive]`; SIZE-1 stale (`popover/algorithms.rs` 523 unlisted); highlighted-node
  removal cost; `":default"` re-parsed per flush; a panic in `HighlightsMut` schedules no repaint; no
  cost pin on `match_chain`; re-exports and a `selector_hints` group; `ControlStateHook` returns `bool`
  (want `Option<bool>`); Enter does not commit a text field; `input:user-invalid{border-color}` paints
  nothing (no UA text-field border); READMEs lack popover / top layer / form states (and say 175 UA
  rules); popover placement recipe until C15; `SelectorCaches` docs must say it goes stale across
  frames for hook-backed state; stale rustdoc on three `PseudoClass` variants. Process: the brief now
  requires silent changes, DESIGN classification, re-exports and migration rows in the same commit.
  Full reports: `target/claude-logs/c11_gate_{architect,api}.md`. Fix as `C11G-*`, two batches
  (A correctness and cost, B API and docs).
- 2026-10-08 — C11G-MODAL-INERT (architect B1, N6's inertness gaps). Found: a modal dialog made the page
  inert to the pointer only — the dialog focusing steps focused nothing when the dialog had nothing
  focusable, `focus_node` accepted any target, and the `inert` attribute was reflected but did nothing.
  Decided — one computed notion, in rdom-core (renderer-free: attributes and the top layer):
  `Dom::is_inert(id)` (HTML §6.3) is true outside the dialog the document is *blocked by*
  (`Dom::blocking_modal()`: the topmost *modal* dialog in the top layer, as the engines read §6.3.2's
  "topmost dialog") or under an `inert` attribute that no modal dialog escapes (§6.3.1's "such as modal
  dialogs"; CSS UI 4's `[inert] { interactivity: inert } dialog:modal { interactivity: auto }`) — one
  ancestor walk. rdom-tui reads it everywhere a focusable area or a hit is decided: `tab_index` (so
  `is_focusable`, `focus()` and the click-focus climb), `change_focus` (`focus_node` and the pointer
  path refuse an inert target, changing nothing — the focusing steps find no focusable area), the focus
  fixup (an element made inert is blurred at the next frame, as one made hidden is), sequential navigation
  (scoped to the blocking modal, pruning `inert` subtrees), `is_unselectable` (HTML: selection "as if
  `user-select: none`") and the hit test (top-layer members that are inert are passed over; the stacking
  walk prunes `inert` subtrees, checks layered boxes and pseudo hosts with the full walk, and resolves an
  inert inline to its block). The dialog focusing steps follow §4.11.4 step 4: with no autofocus and no
  delegate the dialog itself is the control — focused when rendered. Checked the gate's two readings
  against the spec text: an unrendered modal still blocks (§6.3.2 names no rendering), so the hit test
  returns nothing instead of skipping to the page; and a popover shown above a modal from outside it is
  inert (only the subject's flat-tree descendants are excepted) — the gate was right, the hit test now
  passes it over. Keyboard activation needs no guard of its own: the builtins act on the focus, which can
  no longer be inert after the focusing steps or a frame. `dialog::top_modal` is `blocking_modal`. Red:
  rdom-core's five `inert_tests` failed on the stubbed `false` / `None`; rdom-tui's six `inert_tests`
  failed for the stated reasons (the focus stayed on `del`; `focus_node` moved into the inert page; the
  unrendered modal's page was hit; the outside popover was hit; Tab and the pointer reached the `inert`
  button; the inert focus stayed). Green after. Mutations (each alone, restored, touched): no inert
  check in `is_unselectable` → `inert_text_is_not_selectable` (added for it); no subtree pruning in the
  descent → the `inert` subtree test; no `change_focus` guard → the `focus_node` test. No existing
  expectation changed.
- 2026-10-08 — C11G-POPOVER-BOUND (architect B2, N5, N6; API N2). Found: `close_entire_list` and
  `hide_stack_until` looped while their list was non-empty, so two auto popovers whose closing
  `beforetoggle` listeners show each other hung the App (reproduced through light dismiss: the cap in the
  test's listener tripped) — the C11-MODAL-POPOVER log's "the walk always ends" was wrong. Decided —
  follow the current HTML text (Living Standard of 2026-10-07), which has changed since C11-MODAL-POPOVER
  was written: show popover step 2 refuses a show while the document's *showing popover* flag is set or
  its *hiding popover nesting count* is non-zero; "hide popover stack until" (which replaces "close entire
  popover list") hides a slice of the list fixed at its start, top down, then one pass with `fireEvents`
  false over whatever is not in the remaining slice; "hide popovers until" keeps a hint endpoint's *hint
  stack parent*; "topmost popover ancestor" is the last popover of the auto list then the hint list that
  holds the new popover or its source, and an auto popover under a hint ancestor is *downgraded* to a hint
  (15.2) — replacing the old step-12 rule. `popover/algorithms.rs` (523 lines) is split: `algorithms.rs`
  (407) keeps validity, show / hide and the removing steps, `state.rs` (118) the per-element and
  per-document state, `stack.rs` (144) the walks. The flags are reset
  on every path, a panicking listener included (`guarded`: catch, clean up, re-raise). Because rdom fires
  `toggle` synchronously inside the algorithm, a `toggle` listener cannot show a popover either — written
  into DIVERGENCES §2 with the timer recipe. The stale ancestor (architect N6): the spec computes the
  ancestor once, before hiding the hints, as the gate's text proposed the opposite of — but under the new
  walk an endpoint no longer in the list gives `lastHideIndex` 0, so a hint listener that hid the ancestor
  leaves no unrelated auto popover showing, which was the bug; pinned. The removing steps: rdom-core drops a
  removed popover from the top layer at once, and an observer may not run the walk, so the popover state
  keeps each auto / hint popover's place in its list until `flush_removed` (with the attribute change
  steps, at the `App`'s next event or frame) runs "hide popover" for it — no focus, no events — hiding the
  popovers above it (nested through an invoker or not) and dropping every map entry of a popover that no
  longer shows (the maps were never pruned). Hide's events carry the invoker as `source` (the activation
  passes the button). `dialog::show` / `show_modal` return `Result` with HTML §4.11.4's guards: a modal
  dialog's `showModal()` returns at once (it was moved to the top, making the dialog above it inert); an
  open non-modal, a disconnected or a popover-showing dialog is `InvalidState` (the last silently swapped
  the top-layer kind); `show()` on a modal dialog is `InvalidState` (it made it non-modal), and `show()` now
  runs its steps 7–10 — previous focus, hiding unrelated popovers, the dialog focusing steps. Not done: a
  dialog's `beforetoggle` (HTML's newer show steps), written into DIVERGENCES §2. Red: the five popover
  tests failed (both ping-pongs: "nothing reopened" and "did not terminate"; the nested show was allowed;
  the stale ancestor left `m1`; the removed popover's nested one stayed; 50 entries tracked after churn);
  the dialog guards were red as compile errors (`()` returned). Green after. Changed expectations:
  `show_after_show_modal_clears_modal_marker` is removed (HTML throws; replaced by
  `show_on_a_modal_dialog_throws`); `dialog_show_on_already_open_does_not_refire_toggle` asserts the
  refusal; `esc_closes_popovers_and_modal_dialogs_in_order` closes the dialog before its second
  `showModal()` (step 1 makes a repeat a no-op). Mutations (each alone, restored, touched): no step-2 guard →
  both direct ping-pong / nested-show tests (light dismiss alone stays bounded by the slice); no cleanup on
  a panic and no removal flush (two disjoint mutations in one run) → the panic test, and the removal and
  churn tests.
- 2026-10-08 — C11G-CANVAS-FILL (API B2). Found: `Color::Reset` stood for two things — the terminal's
  default background, which `Canvas` and `reset` resolve to, and "no background", the initial value —
  and `fills()` treated it as the second everywhere, so the UA's `[popover] { background-color: Canvas }`
  painted nothing and a modal dialog over text showed the page's glyphs in its padding and beside its
  lines (a regression of C11-MODAL-POPOVER: the 0.5 dialog sat in flow). Decided — split them once, at
  the root: `background-color`'s initial value is the real `transparent` (CSS Backgrounds 3 §3.2;
  `ComputedStyle::initial().bg` is `Color::TRANSPARENT`), and `fills(bg)` is `alpha > 0` — so `Canvas` /
  `reset` *paint*: the box's cells are blanked (space) in SGR 49, "the default background, painted", while
  `transparent` is "no paint". `Color::Reset` keeps meaning the terminal's default colour; only the
  initial value moved. Every reader that took `Reset` as "no fill" was audited: `fills` (background,
  text, `::first-letter` / `::first-line`, highlight overlays, tree-row highlights) now follows from the
  root; the scrollbar's track / thumb tested `bg != Reset` (now `fills`); `Buffer::tint` (a `::backdrop`)
  skipped `Reset` (now only `transparent`, so a `::backdrop { background-color: Canvas }` tints);
  `::selection`'s paired default dropped the UA's half to `Reset`, which would now paint — it drops it to
  `transparent` (pinned by C10G-SELECTION-PAIRED's test, which fails under the mutation). The canvas model
  (`compose`, the translucent composite, the caret) already resolved `Reset` through the scheme's canvas.
  The UA's `dialog` rule takes HTML's `background-color: Canvas; color: CanvasText` (every dialog, as
  HTML's; a non-modal one in flow blanks only its own box); `[popover]` already had it. Red: the four
  `canvas_fill` tests failed (the initial was `Reset`; `Canvas` / `reset` over a red parent left it red;
  the modal dialog's and the popover menu's boxes held the page's `x`s). Green after. Changed
  expectations (all "an unstyled element's `bg` is the initial value", now `TRANSPARENT`):
  `initial_is_safe_defaults`, `bg_does_not_inherit`, `focus_tint_does_not_fill_containers`,
  `ua_aria_tree_selectors_match`, `a_sheets_case_flags_decide_the_value_comparison`,
  `nth_pseudo_classes_style_through_a_sheet`, `focused_canvas_is_clean_by_default`,
  `focused_container_gets_no_tint`. No paint snapshot changed (rdom-showcase's and ACID's included):
  nothing in them specifies `Canvas` or `reset` as a background. Mutations (each alone, restored,
  touched): `fills` excluding `Reset` again → the three paint tests; no `background-color` in the UA's
  `dialog` rule → the modal dialog test; the paired default back to `Reset` → C10G's selection test.
- 2026-10-08 — C11G-HAS-COST (architect N2, N3, N7, N8, N11; corrects TECH_DEBT `HAS-COST-1`). Found, by
  count: `div:has(p div)` over 1000 nested `div`s visited 499 500 candidates and took 333 333 000 chain
  steps (every candidate's climb ran to the root); a `div:has(+ i p)` miss under 1000 ancestors climbed
  1002 steps; one `li:has(+ .on)` / `li:has(~ .on)` rule made toggling a class on 5000 rows walk
  12 512 496 / 12 512 500 siblings; a mutation in a pass's middle cleared the anchors recorded before it;
  and `:scope` inside `:has()` or `of S` read another `@scope` root's cached answer. Decided — (1) a
  *downward* argument (descendant and child combinators only) is answered by a bottom-up memo per
  (element, compound): "does an element below this one, related as the combinator left of compound k
  says, match it and the rest to its right?" — each computed once per pass, the anchor's answer is k = 0
  — generalising C11-HAS's `:has(.x)` memo to every downward form, O(N · compounds) for nested anchors
  and no chain climb at all; (2) an argument with a sibling combinator keeps the search, but `match_chain`
  stops a compound's climb at the anchor (descendant / child lead) or the anchor's parent (sibling lead) —
  no compound of the argument can sit there (§4.5) — returning the step's not-found outcome, which keeps
  Servo's bounds; (3) the invalidation walk's sibling reach comes from the sheets
  (`HasTriggers::sibling_reach`): the length of the `+` run leading a relative selector, unbounded with a
  `~` in it or a sibling step nested in `:is()` / `:not()`, 0 without — and each (element, parent) is
  walked once per drain (`DirtyState::has_walked`; with unbounded reach a run's earlier siblings are
  marked walked too, their walks being the run's suffixes; keyed by the parent so a node moved in the
  drain walks again); (4) the anchors are facts, not cached answers: `sync` keeps them; (5) the `:has()`
  key (`HasKey`) and the `of S` nth key carry the scoping root. `CacheWork::chain_steps` (new field of a
  `#[non_exhaustive]` struct) counts the matcher's candidate steps, which pins the backtracking bound
  (N11). TECH_DEBT `HAS-COST-1` now states the bounds the tests pin (a `~` argument over a long list is
  still quadratic per pass; the sticky flag and subtree restyle stay accepted). Red → green:
  `a_downward_has_chain_over_nested_anchors_is_linear` (499 500 / 333 333 000 → under 3N and 0),
  `a_sibling_has_search_stops_at_the_anchors_parent` (1002 → ≤ 4), `has_anchors_survive_a_mutation_mid_pass`,
  `cached_answers_are_kept_per_scoping_root`, `an_adjacent_has_walk_checks_one_earlier_sibling` and
  `a_subsequent_sibling_has_walk_is_deduplicated_within_a_drain` (12.5M → under 6N);
  `backtracking_is_bounded_by_the_depth_per_compound` is a pin (green before). Mutations (each alone,
  restored, touched): no anchor bound → the sibling search test; the nth key without the scope → the scope
  test's `of :scope` half; `sync` clearing the anchors → the anchors test; a descendant step's not-found
  outcome as a restart → the backtracking pin; no sibling-run dedupe → the `~` test; unbounded reach
  always → the `+` test (rewritten to one drain per change, since the dedupe alone also made the
  one-drain version linear). Existing expectation changed: `the_arguments_reads_fire` asserts reaches
  (`HasTriggers::siblings()` became `sibling_reach()`).
- 2026-10-08 — C11G-DIR-AUTO-COST (architect N4). Found, by count: 100 appended rows (and clock ticks)
  under `<main dir=auto>` restyled `main`'s subtree 100 times, though its first strong character never
  moved — `mark_auto_direction_host` marked the host on every text or child-list change below it.
  Decided — a host remembers the directionality it was last styled with (`TuiExt::auto_direction`, set by
  the cascade's write-back for every `dir=auto` / valid-`dir`-less `<bdi>` element, `None` for others;
  `style::dir_auto` holds the one "is an auto host" predicate the cascade and the tracker share) and the
  tracker marks it only when `Dom::directionality` now differs, recording the new one. The check costs the
  host's text up to its first strong character per change below it — what the restyle would have read
  anyway — and no allocation. Red: `a_dir_auto_host_restyles_only_when_its_direction_flips` counted 100
  restyles; green after (0, then one per flip ltr → rtl → ltr, none for an rtl → rtl edit). Mutations (each
  alone, restored, touched): marking unconditionally → the count; no cascade baseline (the first edit sees
  `None`) → the count (1).
- 2026-10-08 — C11G-DETAILS-PARENT (architect N1). Found: two climbs that look for a box took DOM parents
  and skipped the `::details-content` box (C10G-DETAILS-CONTENT-BOX's `slot::parent` is the box tree's
  view): the pseudo hit test (`hit_test::pseudo::ancestors_or_self`) read the `<details>`'s line boxes
  for a slotted `<span>`'s `::before`, which are the slot box's, and found nothing (so `:hover` never
  applied); and blockification's `children_are_items` was called with the slotted child's DOM parent (the
  `<details>`) but the slot's style, then climbed DOM parents — under `details { display: flex }
  details::details-content { display: contents }` it went from the `<details>` to its parent and left the
  content inline. Both now climb the box tree, and the element cascade passes the box parent. The cascade
  may not depend on render code (`the_cascade_does_not_depend_on_render_code`), so the one answer moved:
  `style::cascade::details::box_parent` (it reads the cascade's own slot links) and `slot::parent`
  delegates to it. The audit
  of every other `parent_node()` climb in render, layout, the hit test, scrolling, selection and the
  cascade: `walk::items_changed_above` (the restyle's version of the same `display: contents` climb)
  now climbs the box tree too — no case tells the two apart today, as the slot box is never itself in the
  restyle's `items_changed` list, but it is the same question; the rest are DOM questions and stay —
  `<a href>` lookup, table-row parents, closed-`<details>` text hiding (asks for the `<details>` itself),
  the top layer's event path, clipboard serialisation, `@scope`, counters (the element tree), highlight
  inheritance (the slot box keeps no highlight styles), `user-select`'s used value and hosts (selection
  hosts are DOM elements whose text a selection spans; the slot box has no children of its own), and the
  subtree cascade's flag bubbling (the slot box takes its `<details>`'s flags through `mirror_flags`; pinned
  by `a_restyle_inside_details_flags_the_slot_box`, green before and after). Red:
  `a_slotted_elements_before_is_hit` (no pseudo-element hit) and
  `slotted_content_of_a_flex_details_through_a_contents_slot_blockifies` (`Inline`); green after, with
  `a_restyled_contents_slot_reblockifies_its_content` for the restyle path. Mutations (each alone,
  restored, touched): a DOM climb in the hit test → the hit test; the DOM parent passed to
  `children_are_items` → both blockify tests; a DOM climb inside it → both blockify tests.
- 2026-10-08 — C11G-ENTER-COMMIT (API N4). Found: Enter in a single-line field fired no `change` and set no
  user validity — a one-field TUI prompt (`<input required pattern=…>`) never showed `:user-invalid` until
  a Tab it may never get — and implicit submission counted only rdom's text-family inputs as fields that
  block it. Decided — HTML §4.10.5.5 leaves the moment of a text control's commit to the user agent;
  every engine commits a single-line field on Enter, before the implicit submission (§4.10.21.2) the
  key starts. The form builtin's Enter listener (moved with its helper into `form/implicit.rs`; `form/mod.rs`
  573 → 522 lines) now runs `form_state::commit_pending_change` first — `change` when the user edited the
  value since the last commit, and user validity, with or without a form owner — then, if the field is
  still in the tree, implicit submission. The trigger and the count use HTML's exact list of fields that
  block implicit submission (`blocks_implicit_submission`: Text, Search, Telephone, URL, Email, Password,
  Date, Month, Week, Time, Local Date and Time, Number), so a date field counts against a form's one-field
  submission and submits from Enter itself. The DIVERGENCES §2 entry "A text control commits its value on
  losing focus only" is removed. Red: `enter_commits_a_single_line_field` (0 `change`s),
  `enter_commits_before_implicit_submission` (`["submit"]`, no `change`),
  `date_fields_block_implicit_submission` (a text + date form submitted); green after. Mutation (restored,
  touched): no commit → both Enter tests.
- 2026-10-08 — C11G-MINOR (architect N11's remainder). Three parts. (1) Highlighted-node removal cost: the
  removal hook (DOM §4.2.3 "remove" steps 4–7) asked of every boundary point whether it is inside the
  removed node with an ancestor walk each, twice (once to decide whether any boundary moves, once to move
  them) — 2 511 690 parent hops for 10 removed lines of a 60-deep log under a 1000-hit search. The answers
  now share their ancestors (`InsideMemo`: a walk stops at the removed node, the root or a node already
  answered, and records its answer for the nodes it passed; one memo per removal, both passes) — 60 400
  hops. (2) `FormStateMarks::current` parsed `:default` on every flush to match it through the selector
  engine; rdom-core gains `Dom::is_default_with(id, &mut SelectorCaches)` (the cached path `:default`
  matching already used, now public beside `is_default` and `matches_list_with`, syncing the caches to
  the epoch as they do) and the walk calls it — no selector at all. (3) A panic while a `HighlightsMut`
  guard is held: the guard's drop moves the generation but, unwinding, runs no observer (a second panic
  would abort), so the dirty tracker never heard and no repaint was scheduled. Decided — not a deferred
  record in rdom-core (a host that catches the panic and mutates nothing would still never hear it) but
  the frame's own check: the `App` notes the registry generation it painted (`after_paint`, document data)
  and a prelude stage repaints when the registry's differs — the scroll offsets' pattern (stage 8). Red:
  `removing_under_a_large_highlight_walks_each_ancestor_once` (2 511 690 hops; bound 81 200) and
  `a_highlight_changed_by_a_panicking_caller_repaints` (no frame drawn); green after.
  `is_default_with_shares_the_pass_caches` pins the new API (one default-button walk for 50 buttons);
  part (2) changes no behaviour — the existing `:default` tests cover it. Mutation (restored, touched):
  memo lookups off → the hop count.
- 2026-10-08 — C11G batch A split and SIZE-1 recount. C11G-MODAL-INERT left `runtime/hit_test/descend.rs` at
  592 production lines, past the 575 split-on-touch bar (it should have split in that item; done here,
  before closing the batch): the inline formatting context's half — the fragment owner under a point,
  atomic / transparent / hidden / inert inlines, the inline ancestors on the path — moves to
  `hit_test/inline_hit.rs` (215), `descend.rs` keeps the stacking-context and box walk (407). No behaviour
  changes; the hit-test tests are the evidence. TECH_DEBT `SIZE-1` recounted: none past 575; the batch's
  splits (`popover/algorithms.rs` → + `state.rs` + `stack.rs`; `form/mod.rs` → + `implicit.rs`; this one)
  recorded there. C11G batch A (C11G-MODAL-INERT, -POPOVER-BOUND, -CANVAS-FILL, -HAS-COST, -DIR-AUTO-COST,
  -DETAILS-PARENT, -ENTER-COMMIT, -MINOR) is done; batch B (API and docs: C11G-UPGRADE, -DESIGN-TYPES,
  -API, -DOCS) remains.
- 2026-10-08 — C11G-UPGRADE (API B1). The upgrade guide held four batch-A items unranked in the middle
  (16–20: inertness, `Canvas`, `show()` focusing, Enter) and none of Phase 11's own silent changes. Ranked
  now (68 items), by the gate's order with batch A fitted in: 16 the modal dialog — centred in the top layer,
  covering content where it sat in flow, the page inert (the C11G-MODAL-INERT item merged into it, as one
  consequence of `showModal()`); 17 `Canvas` / `reset` painting and `dialog`'s HTML background (the reader
  half of that item split out); 18 the hidden-focus refusal (C6G, kept in place); 19 `change` on blur; 20
  Enter committing; 21 `dir=auto` / `<bdi>` setting `direction`; 22 backtracking combinators and 23 `+` / `~`
  skipping text and comments (two items: different selectors start matching, for different reasons); then
  the reader changes, with 26 a computed `bg` starting `Color::TRANSPARENT` (code testing `bg ==
  Color::Reset` for "no background" tests `bg.alpha() == 0`). At the tail, narrowest last: `:scope` in the
  query methods, `showModal()` hiding open popovers, `dialog::show()` focusing into the dialog (19 → 66: it
  moves focus only for code that called `show()`), the `[-]` glyph, parentless `:first-child`. "Compile
  breaks" gains a Selectors line for `SimpleSelector::Attribute`'s `case`. Docs only: no test.
- 2026-10-08 — C11G-DESIGN-TYPES (API B3, architect N9). DESIGN classified no Phase 11 type — the fourth
  phase running the rule was missed by review. Decided — make it mechanical, reading DESIGN as it is written:
  the classification section is wrapped in `<!-- type-classification: begin / end -->` markers, and
  `rdom-showcase`'s `design_types` test (the unpublished crate, as `file_sizes`, since it reads `specs/` and
  every sibling's sources) collects every name each published crate's `lib.rs` exports (`pub use` items'
  last segments and aliases; its own `pub struct` / `pub enum`), keeps those that a production source
  defines as a struct or enum, and fails on any not named in backticks between the markers. A name in a
  bullet is a classification because each bullet is a kind with its reason; a separate index would be a
  second copy to drift. The rule is in CLAUDE.md §Contract First. Run against the old DESIGN, it named 67
  unclassified types — Phase 11's `ControlState`, `Directionality`, `TopLayerKind`, `SelectorCaches`,
  `CacheWork`, and 62 older ones (the DOM handles, the ids, the runtime and terminal types, the registration
  syntax, `SystemColor`, `Flow` / `Visibility` / `UserSelect`, the sheet records…), each now in its kind.
  Decisions: `Directionality`, `AttrCase` and `NthKind` closed — `#[non_exhaustive]` dropped (HTML
  §3.2.6.4's two values; Selectors 4 §6.3's no flag / `i` / `s`; the four child-indexed forms —
  `:nth-col()` indexes columns and lands as its own selector with C13-COLUMN, so the gate's "keep `NthKind`
  open for it" does not apply); `TopLayerKind` (fullscreen), `PopoverState` (`hint` came once) and
  `ControlState` (a hook answering `None` for an unknown question, C11G-API) open; `RelativeSelector` /
  `NthSelector` open parser outputs with no constructor; `SelectorCaches` sealed by private fields;
  `CacheWork` an outcome record. Types reached only through a public module (`selectors::NthKind`,
  `popover::PopoverState`) are outside the test's surface and classified by review; C11G-API's root
  re-exports bring `AttrCase` and `PopoverState` into it. Red: the exhaustive matches of the three enums in
  `closed_by_spec_enums_match_exhaustively` (an external crate) failed to compile (`_` not covered); the
  check failed on the missing markers, then on 67 names. Green after both changes.
- 2026-10-08 — C11G-API (API N1, N3, N8, N9). (1) Root re-exports: `TopLayerKind`, `Directionality` and
  `ControlState` from rdom-core's root, `AttrCase` from `rdom_core::selectors`, and
  `PopoverState` from the popover builtin — the types `top_layer_kind`, `directionality`, `control_state`,
  an attribute selector's `case` and `popover_state` return; all now inside `design_types`' surface and
  classified (C11G-DESIGN-TYPES). (2) The `SimpleSelector::Attribute` API row had no hint test though its
  migration builds one: `selector_hints` builds it with `case: AttrCase::Default`, destructures it with
  `..` and matches `type=CheckBox` through `Dom::matches_list`. (3) `ControlStateHook` returns
  `Option<bool>`: `Dom::control_state` takes the hook's `Some`, else the substrate's default — so a sibling
  backend's `_ => None` arm defers a question added later instead of answering it `false` (the
  `DefaultChecked` default is an attribute, which a `false` silently overrode). rdom-tui's hook answers every
  question it knows, `None` (with its `debug_assert!`) for one it does not. `ControlState` is new since 0.5,
  so the row is in "Changes to APIs added after 0.5", with `control_state_hook_hints`. (4) `SelectorCaches`
  says it is one pass's: the epoch moves only on mutation records, and user validity, default checkedness
  and constraint validity are hook-backed, so caches kept across frames answer `:has(:user-invalid)` /
  `:default` from before such a change. (5) Stale rustdoc: `PseudoClass::Indeterminate` (checkboxes and
  radio groups since C11-FORM-STATES, not "`<progress>` only"), `Open` (no `data-rdom-open`; a modal dialog
  has `open`), `Scope` (the query methods' scoping roots of DOM §4.2.6; `:root` only with no scoping root).
  Red: `a_hook_answering_none_defers_to_the_substrate_default` and the updated hooks failed to compile
  (`expected bool, found Option<bool>`); the three hint groups failed to compile without the re-exports
  (`TopLayerKind` … `ControlState` not found). Green after. Changed expectations: the three hooks in
  `form_state_tests.rs` return `Option<bool>` (`_ => false` → `_ => None`), with the same assertions.
- 2026-10-08 — C11G-DIALOG-BEFORETOGGLE (left open by C11G-POPOVER-BOUND). HTML §4.11.4's current text (the
  Living Standard of 2026-10-07) gives `<dialog>` a `beforetoggle`, which rdom did not fire. Followed: `show()`
  steps 3–4 — a cancelable `beforetoggle` (`closed` → `open`), then return when it was canceled or a listener
  opened the dialog; `showModal()` steps 6–9 — the same event (with the step-1–5 guards before it, so a refused
  call fires nothing), then return when canceled or when a listener opened it, disconnected it or showed it as
  a popover; "close the dialog" steps 2–3 — a non-cancelable `beforetoggle` (`open` → `closed`), then return
  when a listener closed it first (its return value stands). These returns are not errors, as in HTML. The
  dialog's `toggle` was dispatched cancelable (the `TuiEvent` default), which a `ToggleEvent` never is; one
  helper (`fire_toggle_event`) now fires both, cancelable only for an opening `beforetoggle`. Step comments
  renumbered to the current text. The DIVERGENCES §2 sentence is removed; `toggle` still fires synchronously,
  uncoalesced (the same entry). Ranked into the upgrade guide (67: a dialog `beforetoggle` listener — one
  written for popovers on a `<dialog popover>` — now also runs for `show()` / `showModal()` / `close()`). Red:
  the four tests of the new section failed (no `beforetoggle` in the log and a cancelable `toggle`; the
  canceled show opened the dialog; the listener's `show()` and disconnection did not end `showModal()`; the
  outer close overwrote the inner's return value). Green after; no existing expectation changed. Mutations
  (each alone, restored, touched): no step-7 re-check → `show_modal_rechecks_after_beforetoggle`; no close
  step-3 re-check → `close_rechecks_after_beforetoggle`.
- 2026-10-08 — C11G-HAS-IS-SIBLING (batch A's found gap: a sibling step nested in `:is()` inside `:has(+ …)`
  can reach an element before the anchor, and the `:has()` walk visits only earlier siblings). Wrote the red
  test first — `a_nested_sibling_step_before_the_anchor_restyles_it`: under `.a:has(+ :is(.x ~ *))` (and
  `:hover` / `:empty` left compounds, `:not()`, `:nth-child(2 of .x)`, and an inserted `.x`), a change to an
  `<i>` two elements before the anchor, with both triggers computed from the sheet as the App computes them
  — and it was green. The reach is bounded, and already handled, by the sibling marking: every combinator
  nested in an argument leads up or back from its subject, so an element an anchor's match reads is in its
  subtree or a later sibling's (the walk's), an ancestor of it (whose change restyles its subtree), inside it,
  or reached by a back step — and such an element matches a compound left of a sibling combinator, which
  `SiblingTriggers` records inside `:has()` arguments (`selector_walk::arguments` walks them), so its change
  marks every one of its siblings: the anchor or one of its ancestors. `of S` is recorded the same way. Decided
  — keep one mechanism per direction, and write the argument where the next reader looks: the `has_triggers`
  module doc ("Why earlier siblings are enough"), `mark_has_anchors`' doc, TECH_DEBT `HAS-COST-1` (the
  forward case costs the sibling marking's O(siblings)). The test is a pin; it fails when the sibling triggers
  stop walking `:has()` arguments (mutation, restored, touched: `arguments` returning nothing for `Has` → the
  `.x` case fails). No behaviour change, so no CHANGELOG bullet.
- 2026-10-08 — C11G-FORM-BORDER (API N5). `input:user-invalid { border-color: red }` paints nothing: the UA text
  field has no border (`padding: 0 1` and a `Field` background), so there is no border style for the colour to
  show on. Options: (a) a UA border — 2 rows and 2 columns on every field, a one-row prompt becoming three, and
  the user kept the input's `padding: 0 1` look; (b) document that authors add `border`; (c) map `border-color`
  on a borderless field to something visible — a used-value rule no spec has, which would surprise the author
  who sets `border-color` ahead of a `:focus { border-style: solid }`. Decided (b): the terminal cost of (a)
  falls on every form whether or not it shows states, and (c) invents CSS. DIVERGENCES §1 gains "A text field
  has no UA border" with both working forms; the rdom-tui README gains "Form states and the field border", a
  doctest of three required fields under `:invalid` — `border-color` alone (one row, nothing red), a
  background cue (one row, red), and `border: solid` + `border-color` (three rows, a red border) — and the
  root README's built-ins paragraph says so. Docs only: the doctest documents existing behaviour, green when
  written (rows and colours asserted exactly).
- 2026-10-08 — C11G-DOCS (API N6, N7). The READMEs were silent on Phase 11's part 2. rdom-tui README: "Form
  states" (the section C11G-FORM-BORDER started, its border half now a subsection) lists the input
  pseudo-classes and when user validity is set, with a doctest driving a headless `App` with key events —
  an empty required field `:invalid` but not `:user-invalid`; an email field typed into stays unjudged until
  Tab leaves it, then `:user-invalid`; a click on the submit button judges the untouched field too (the
  blocked submission focusing the first invalid field), and the next frame's computed `bg` is the author's red
  (the focused field keeps the UA's `!important` `:focus-visible` tint — the assertion first written on it
  failed for that reason, which the comment now says). "Popovers and the top layer": the placement recipe
  until C15-ANCHOR — in a `beforetoggle` listener the event's `source` is the invoker (`invoker_of` is set
  only once the popover shows: HTML's step 21 runs after the event, so the gate's "`invoker_of` in
  `beforetoggle`" would read `None`), its `bounding_rect` gives `top` / `left` after `inset: auto` — and
  `autofocus` for Tab; its doctest opens a `popovertarget` menu by a click on the button, checks it sits
  under the button, paints it over six lines of `x`s and finds none in its box (the `Canvas` fill), then
  closes it with a press and release outside. Root README: the UA rule count is 183 (it said 175; the
  `ua_sheet` test pins 183); the top-layer bullet replaces "modal `<dialog>` focus trap"; the pseudo-class list
  gains Phase 11's and the Selectors 4 structure; the built-ins list gains the `popover` attribute. ACID tile
  15 shows the dialog and the popover over page text, their boxes blank in `Canvas`. Docs only: the two
  doctests pass against existing behaviour.
- 2026-10-08 — Phase 11 closed. Batch A (C11G-MODAL-INERT, -POPOVER-BOUND, -CANVAS-FILL, -HAS-COST,
  -DIR-AUTO-COST, -DETAILS-PARENT, -ENTER-COMMIT, -MINOR) and batch B (C11G-UPGRADE, -DESIGN-TYPES, -API,
  -DIALOG-BEFORETOGGLE, -HAS-IS-SIBLING, -FORM-BORDER, -DOCS): 15 gate fixes, every architect and API
  finding addressed (API N2 by C11G-POPOVER-BOUND, architect N10 by batch A's splits) or recorded (TECH_DEBT
  `HAS-COST-1`); API N7's "consider" — moving `<select>`'s dropdown (`data-rdom-open`) into the top layer —
  is not taken in Phase 11 and is left to C12's form-control work. The DESIGN classification of public types is now checked mechanically (`design_types`).
  The fixes' re-review rides with the Phase 12 gate.
- 2026-10-08 — C12-ANIMATABLE (1/2). Found (C10G-DETAILS-CONTENT-BOX): the transition engine wrote running
  values into a sparse `PresentationStyle` that paint read field by field, so layout took a geometry
  transition's end value at once. Fixed at the root, as browsers composite the effect stack onto the
  computed value (CSS Transitions 1 §3, Web Animations 1 §5.4.5): each frame the running values are
  composited onto the cascade's style and that composite *is* `TuiExt::computed` (`computed_before` /
  `computed_after` for the pseudo-elements, the `::details-content` box's own), so layout, paint, hit-testing,
  `getComputedStyle` and inheritance all read one style. The cascade's own style — the after-change style the
  next change is diffed against — is kept beside it (`TuiExt::cascaded_for`); the cascade's write-back
  re-applies the running values over a fresh style (`TuiExt::overlay`), so an unrelated restyle mid-run keeps
  them and a change to another property applies at once. A running longhand that inherits or propagates
  restyles the element's children (the same restyle registered custom properties use), so they inherit it.
  Interpolation moved to rdom-style's new `animation` module: a table of every longhand of the dispatch table
  with its spec "Animation type" (by computed value, repeatable list, shadow list, discrete, not animatable),
  and per type the `Animate` rule — cells exactly and rounded half to even (DIVERGENCES §1), length +
  percentage as `calc()` mixes (the old midpoint snap of every `calc()` pair is gone), Oklab colors, integers,
  ratios through their logarithm, track lists item by item, shadow lists padded; a pair that does not
  interpolate is not transitioned (Transitions 2 §2 leaves it to `allow-discrete`). Transitions run per
  longhand, so events name longhands (`padding` → `padding-top` …), and `transition-property` names any
  property (`TransitionProperty::Named(&'static str)`, shorthands expanded, flow-relative names by
  `direction`, the last entry naming a property wins). Frames run on the app's clock when `App::advance`
  drives it (wall clock otherwise), which made App-level transition tests deterministic; three
  `registered_transition_tests` that slept on the wall clock now advance it. Red: `geometry_transition_tests`
  — `a_height_transition_moves_the_box_frame_by_frame` (`left: 10, right: 2` — the end value at once),
  width / padding / margin / inset / gap (`24` vs `17`), `::details-content` height (`4` vs `2`), inherited
  color, idle cost; green after. `longhand_tests::every_longhand_drives_one_interpolation` drives one
  interpolation per longhand from cascaded values (`rdom_style::animation::tests` checks the spec table over
  `property_names()`). Mutation-checked: skipping the composite fails 8 tests, dropping the cascade overlay
  fails `a_change_mid_transition_applies_to_the_other_properties`, dropping the children's restyle fails
  `descendants_inherit_the_running_value`. Cost: no running transition, no frame; one running, one whole-tree
  layout per frame (TECH_DEBT `ANIM-RELAYOUT-1`, no partial relayout exists), no cascade. `walk.rs` split
  (`finish.rs`). Remaining: `interpolate-size` / `calc-size()` (part 2/2).
- 2026-10-08 — C12-ANIMATABLE (2/2) done. `interpolate-size: numeric-only | allow-keywords` (CSS Values 5
  §11, inherited, not animatable) and `calc-size(<basis>, <sum>)` (§10) on `width` / `height`: `Size` gains
  `CalcSize(Arc<CalcSize>)` — `size * factor + offset` over an `auto` or intrinsic basis; an `any` or length
  basis folds to a plain size, a nested one composes. The parser takes sums linear in `size` (checked by
  parsing at `size` = 0, 1, 2 against two percentage bases), which every interpolation produces
  (DIVERGENCES §2). Interpolation (`rdom_style::animation::size`): a keyword is `calc-size(k, size)` and a
  length `calc-size(any, L)` under `allow-keywords` read on the after-change style; two `calc-size()`s of
  one basis, or one with a length, interpolate whatever it says; two different keywords never do. Layout
  (`layout_pass::calc_size`): a document that ever computed a `calc-size()` (`doc_flags`) lays out once with
  each such property at its basis, sizes it from the box that pass gave (`box-sizing`'s box; a percentage in
  the offset against the box parent's content box), and lays out again — every layout mode and every parent
  measuring the box sees a length; the computed styles are put back. Red: `interpolate_size_animates_
  details_content_to_auto`, `allow_keywords_animates_an_auto_width`, `calc_size_sizes_from_its_basis` (all
  three: `interpolate-size` unknown / `calc-size()` invalid); `numeric_only_keeps_auto_discrete` pins the
  initial value. Mutation-checked: skipping the second pass fails the three. Closing a `<details>` does not
  animate: its content computes `display: none` at once, so the `auto` basis is 0 (a browser transitions
  `content-visibility` with `allow-discrete`; DIVERGENCES §2, C14-CONTAIN). `min-*` / `max-*` /
  `flex-basis` do not take `calc-size()` yet (DIVERGENCES §2). Item done.
- 2026-10-08 — C12-TIMING. CSS Easing 2 §2.1 `linear(<linear-stop-list>)`: parsed and canonicalized
  (§2.1.1 — missing first / last inputs 0% / 100%, an input below an earlier one raised to it, runs of
  missing inputs spread evenly, two percentages a hold; fewer than two stops invalid) into
  `TimingFunction::LinearStops(Arc<[LinearStop]>)`, evaluated by §2.1.2's algorithm exactly (point A the
  last at or before the input, the pair before the last at the end; equal inputs give B). `TimingFunction`
  is no longer `Copy`. Easing 1 §2.3.1's before flag: `ease_before`, used during a transition's delay, so
  `steps(n, jump-start)` shows the start value until the delay ends (it showed the first jump).
  Transitions 1 §2.4: a negative `transition-delay` is valid and starts the transition part-way —
  `rule::Clock` sets the start back by the part skipped and the combined duration (`max(duration, 0) +
  delay`) must be positive for a transition to start; delays are `i32` end to end (`parse_time_list`
  signed, `parse_duration_list` for durations, a negative duration still invalid). §6: `transitionrun`
  fires when a transition is created (`TransitionEventKind::Run`), before its delay, with `transitionstart`
  after; both carry the skipped time as `elapsedTime`; custom-property transitions too. §2.1 / §2.5: `none`
  only alone in `transition-property` and in the shorthand. The shorthand serializer repeated a short
  list's last value; it now repeats the list (§2, as the cascade's lookup does). `cubic-bezier()` x outside
  [0, 1] and `steps(1, jump-none)` were already invalid; `steps()` positions were complete. Red: rdom-style
  `transition::timing_tests` (5 tests: `LinearStops` / `ease_before` / signed `parse_time_list` did not
  exist), rdom-tui `animation::timing_tests` (4: negative delay half-way at once, `-100ms` of `100ms`
  changing at once, `transitionrun` before the delay, the jump-start hold), the dispatch of
  `transitionrun`. Mutation-checked: starting the clock at registration fails the negative-delay test;
  `ease` for `ease_before` fails the jump-start one. Item done.
- 2026-10-08 — C12-BEHAVIOR. CSS Transitions 2 §3.1 `transition-behavior: normal | allow-discrete` — a list
  longhand like the other `transition-*` (cycled to `transition-property`'s length) and a keyword of each
  `transition` piece (`TransitionBehavior`; the `transitions_important` group and the `::marker` subset
  carry it). The engine starts a transition for a pair that does not interpolate only under
  `allow-discrete`; it then steps by the longhand's rule — Web Animations 1 §5.3.1's 50 % for a plain
  discrete one, `visibility`'s own, and for `display` the non-`none` value for every progress strictly
  inside (0, 1) (CSS Display 4's / Web Animations 2's display rule), so `display: block → none` keeps the
  box laid out and painted until the end and `none → block` shows at once. Overlay: CSS Position 4 §3.4
  `overlay: none | auto` (`Overlay`, not inherited, `auto !important` on `:modal` / `:popover-open` in the
  UA sheet, the same interpolation rule as `display`). The top layer could not defer removal; it can now:
  rdom-core's `request_remove_from_top_layer` (§3.3 "request an element to be removed from the top layer")
  keeps the element in the top layer in its place, drawn there, but with `top_layer_kind` `None` — no
  longer `:modal` / `:popover-open`, no longer blocking the document — and `runtime::top_layer` removes it
  at a style update once its computed `overlay` (the running value) is not `auto`: after each frame's
  transition step in an `App`, after `CascadeExt::cascade*` outside one. Dialog close and popover hide
  request the removal only when the element's `transition-property` takes `overlay` with `allow-discrete`
  (nothing else could keep it), so every other close still leaves at once, as before. Red: rdom-core
  `a_requested_removal_stays_pending_in_the_top_layer` (no API), rdom-tui `behavior_tests` (3 of 4: the
  `transition` shorthand rejected `allow-discrete`), `a_display_none_transition_keeps_the_box_until_it_ends`,
  `an_overlay_transition_keeps_a_hidden_popover_in_the_top_layer` (left at once). Mutation-checked: a plain
  50 % step for `display` fails the display test; ignoring `overlay` in `finish_removals` fails the popover
  one. `content-visibility` does not exist yet (C14-CONTAIN); its discrete transition comes with it. The easing parsers moved out of `parse/values/transition.rs` (503 production lines) into `easing.rs`. Done.
- 2026-10-08 — C12-STARTING. CSS Transitions 2 §3 `@starting-style`, stored as the C1 `@scope` / `@layer`
  contexts are — on the rule, not in a sheet registry like `@property` / `@counter-style`, since its
  contents are style rules: `RuleContext::in_starting_style` / `Rule::starting_style`. rdom-css parses it
  at the top level (a rule list; `parse_rule_list` now carries the whole `RuleContext`) and nested in a
  style rule (its block's declarations are the parent's, CSS Nesting 1 §3.2); a prelude is invalid. The
  cascade skips starting-style rules (`Sheets::applies`, in `collect`); `cascade::starting_style`
  computes an element's style with them applying, inheriting from its parent's computed style, and only
  when one matches it. The engine's `diff_and_register_with` takes it: an element not rendered at the last
  style update — never styled, or `display: none` itself or under a box ancestor (read for every element
  before the snapshots move on) — has no before-change style (Transitions 1 §3), so its values change at
  once unless it has a starting style, from which its transitions start. That is a behaviour change: an
  element out of `display: none` used to transition from its hidden values (CHANGELOG silent change).
  The App's frame passes its sheets and registry. Red: rdom-css `starting_style` (3: the at-rule was
  unsupported), rdom-tui `starting_style_tests` (3: parse warnings), plus
  `a_newly_rendered_element_without_a_starting_style_does_not_transition` (checked by mutation: treating a
  styled element as rendered fails it). Mutation-checked: no starting style in the frame fails the
  inserted-element and popover tests; applying starting rules in the normal cascade fails
  `starting_style_rules_do_not_apply_otherwise`. Pseudo-elements get no starting style, and a child
  inherits from its parent's computed style rather than its starting style (DIVERGENCES §4). ACID I6
  updated and I16 added (popover entry and exit). Phase 12 part 1 (C12-ANIMATABLE, -TIMING, -BEHAVIOR,
  -STARTING) done.
- 2026-10-08 — C12-KEYFRAMES (1/2), the syntax. A split first: `rdom-core/src/event_detail.rs` (573
  production lines) became `event_detail/` (`mod.rs`, `form.rs`, `ui.rs`, tests), so the animation event
  payload fits. CSS Animations 1 §3 `@keyframes`: rdom-css `keyframes.rs` reads the name (a
  `<custom-ident>` other than `none`, a CSS-wide keyword or `default`, or a `<string>`; else
  `InvalidAtRulePrelude`), each block's `<keyframe-selector>#` (`from`, `to`, percentages in [0%, 100%];
  an invalid one drops the block, `WarningKind::InvalidKeyframeSelector`) and its declarations, dropping
  `!important` ones (`ImportantInKeyframe`). rdom-style `keyframes`: `KeyframesRule { name, layer,
  keyframes }` kept per sheet in source order (`Stylesheet::keyframes` / `define_keyframes`, as
  `@counter-style` is), `Keyframe { selectors, style }` with its `animation-timing-function` (`easing()`)
  and `animation-composition`, and `KeyframesRule::resolve` — one keyframe per offset, ascending, the
  blocks naming it in source order (they cascade, §3). The ten longhands (§4, CSS Animations 2 §3:
  `animation-name` / `-duration` with `auto` / `-timing-function` / `-delay` signed / `-iteration-count`
  with `infinite` and fractions / `-direction` / `-fill-mode` / `-play-state` / `-composition` /
  `-timeline` `auto | none`) are lists in `TuiStyle` and `ComputedStyle` (empty is the initial value),
  not inherited, not animatable, and `animation` sets them per piece in any order — the first `<time>`
  the duration, a keyword going to the first unset component it fits, the name last — resetting
  `animation-composition` and `-timeline`; it serializes only with equal-length lists and those two
  initial. Red: rdom-style `property_dispatch::animation_tests` (4 of 5: `UnknownProperty`), rdom-css
  `keyframes` (8: no `Stylesheet::keyframes`; the at-rule warned `UnsupportedAtRule`); green after.
  `apply_tests::initial_keyword_yields_the_initial_computed_value_for_every_property` covers the new
  fields. The builder gains `animation_*` setters and `animations_important` (`ImportantMask::ANIMATIONS`);
  `tui_style/builder/mod.rs` (580) split first — the positioning setters to `position.rs`, the transition
  and animation ones to `motion.rs`. Nothing runs yet: the engine is part 2/2.
- 2026-10-08 — C12-KEYFRAMES (2/2) done, the engine. CSS animations run in the transitions' effect stack —
  no second engine: `runtime::animation::css` keeps a `CssAnimation` per `animation-name` entry in the
  `AnimationRegistry` beside the transitions, and `composite` writes each element style's transitions
  onto its cascaded style first, then its CSS animations in `animation-name` order (Web Animations 1
  §5.4.5, CSS Animations 2 §3: animations sort above transitions); the composite is the computed style
  layout, paint and inheritance read, and the cascade's write-back keeps it over a fresh style
  (`TuiExt::overlay`), as for transitions. The cascade hook (`diff_and_register_in`, the App's frame,
  under its sheets) matches an element style's new `animation-name` list to its animations by name
  (§4.1): a new name starts at the frame's time, a dropped one is cancelled (`animationcancel` from the
  before or active phase), a kept one takes the other longhands in place, its start time kept — a pause
  holds the local time, a resume restarts the clock from it; an element not rendered (`display: none`,
  itself or above) runs none, so hiding cancels and showing restarts; a changed sheet set re-resolves
  every rule. Keyframes (CSS Animations 1 §3): `cascade::keyframes_rule` resolves a name to the last
  rule by layer, sheet and source order (a `SheetFacts` map, built once per sheet set);
  `cascade::keyframe_style` computes a keyframe's values as the element's cascade (or its
  `::before` / `::after`'s) with the keyframe's blocks in a new ladder step, `Source::Animation`, between
  the normal and the important declarations (CSS Cascade 5 §6.1) — so `inherit`, `var()`,
  `currentcolor` and the relative units resolve as for the element, and an `!important` declaration
  keeps its value; computed when the animation starts or its element style changes, never per frame.
  `KeyframeEffect` keeps, per animated longhand (`Longhand::declared_in`, animatable ones only), its
  property-specific keyframes with implicit 0% / 100% ones at the underlying value, each with its easing
  (the keyframe's `animation-timing-function`, else the animation's) and composite operation; sampling
  (Web Animations 1 §5.3.3) eases the interval and interpolates through `Longhand::interpolate`, and
  under `add` / `accumulate` an endpoint is `underlying + value` (`Longhand::add`, new in rdom-style:
  numbers, integers, length-percentages — a `calc()` sum when not linear — and colors by sRGB channel;
  any other type replaces, DIVERGENCES §2). The timing model is Web Animations 1 §4 (`css::timing`):
  phases, active time by fill mode, overall / simple progress (the end is progress 1 of the last
  iteration), current iteration, directed progress for the four directions; `auto` durations are 0 on
  the clock. Events (CSS Animations 2 §4.2's table, `css::events`): `animationstart` /
  `animationiteration` / `animationend` / `animationcancel` with `elapsedTime` (interval start / end,
  iterations × duration, the active time cancelled) and `pseudoElement` (a `::details-content` box's to
  its `<details>`), payload `rdom_core::AnimationDetail` (`EventDetail::Animation`); a frame's events go
  out after its transition events, by the time each happened, then tree order, slot order and
  `animation-name` order — not cancelable. `App::get_animations(node)` lists the transitions and CSS
  animations (`AnimationInfo`: kind and name, pseudo-element, play state, current time). Frame pump
  (cost pinned by `an_infinite_animation_costs_a_composite_per_frame_and_layout_only_for_geometry`): a
  running transition or a playing animation before its end asks for a `Redraw::Paint` frame
  (`needs_frames`; a paused or finished one asks for none, the live loop polls at the animation frame
  rate while one does); such a frame steps every registered effect and composites only the element
  styles whose progress moved (one per animated element per frame) and lays out only when a composited
  longhand layout reads moved (`Longhand::affects_layout`: all but the colors, `opacity` and the
  shadows) — a background-color pulse paints 4 frames with 0 layouts, a `height` one with 4; the same
  rule now spares a color transition its layout (TECH_DEBT `ANIM-RELAYOUT-1` narrowed). Every
  transition test kept its expectations. Red: with the hook not running CSS animations (the
  `diff_and_register_with` path), 21 of the 22 `keyframes_tests` / `animation_event_tests` fail (the
  `!important` one passes vacuously); green after. Mutation-checked: the animation step after the
  important ones fails `an_important_declaration_beats_the_animation`; animations under transitions
  fail `an_animation_sorts_above_a_transition`; always laying out fails the cost test; ignoring the
  composite operation fails `composition_adds_keyframes_to_the_underlying_value`; no color addition
  fails `colors_add_by_channel`. `prefers-reduced-motion` waits for `@media` (C14-MEDIA, noted in
  DIVERGENCES §3 and in `runtime::animation::css`): it is a media feature, so no engine hook is needed.
  `style/cascade/ladder.rs` is now 552 production lines (TECH_DEBT `SIZE-1`). Item done.
- 2026-10-08 — C12-SCROLL-DRIVEN (1/2), the syntax (Scroll-driven Animations 1). rdom-style's
  `keyframes::timeline` holds the values — `TimelineAxis` (`block | inline | x | y`), `TimelineScroller`
  (`nearest | root | self`), `TimelineName` (`none | <dashed-ident>`), `TimelineInset` (start and end,
  each `auto` or a `<length-percentage>`), `TimelineScope` (`none | all | <dashed-ident>#`),
  `TimelineRangeName` (the six named ranges) and `RangeBoundary` (`normal`, or an offset into a named
  range or the whole timeline) — and `AnimationTimeline` gains `Named`, `Scroll { scroller, axis }` and
  `View { axis, inset }` (`scroll()` takes its two keywords in either order, `view()` an axis and insets
  in either order; the defaults serialize away). The longhands `scroll-timeline-name` / `-axis`,
  `view-timeline-name` / `-axis` / `-inset`, `timeline-scope`, `animation-range-start` / `-end` (a name
  alone is 0% at the start, 100% at the end) and the shorthands `scroll-timeline`, `view-timeline`,
  `animation-range` (an omitted end is the start's range at 100% when the start names one, else
  `normal`; §4.3) are computed lists, none inherited; `view-timeline-inset` animates by computed value
  (lists of one length pair their insets), the others are not animatable; `animation` resets the
  range. `ImportantMask::TIMELINES` and builders (`scroll_timeline_name`, `animation_range`, …).
  Their viewport units are absolute at computed-value time (`absolute.rs`, caught by
  `css_phase2_gates::every_length_property_resolves_viewport_units_in_the_cascade`).
  Red: `property_dispatch::timeline_tests` (7: `UnknownProperty`); green after. Nothing follows a
  timeline yet: the engine is part 2/2.
- 2026-10-08 — C12-SCROLL-DRIVEN (2/3), the timelines. `runtime::animation::css::timeline` resolves an
  animation's timeline each frame it steps: `scroll()` — the nearest box ancestor that is a scroll
  container, the document element for `root`, the element for `self` — `view()` — the element in its
  nearest scroll container — and a name — the nearest element at or above it declaring it (a scroll
  timeline before a view timeline of one name), or the single descendant declaring it of the nearest
  ancestor whose `timeline-scope` takes it (none or several: inactive; TECH_DEBT
  `TIMELINE-SCOPE-COST-1`). Progress is measured in cells from the scroll origin along the axis (`block`
  is `y` in `horizontal-tb`): the current offset against the unified scrollport and scroll range
  (`layout_pass::scrollport`, `scroll_bounds`; under `rtl` or a reversed flex axis `scrollLeft` runs
  0 → −range and counts from the right), a range of 0 or no scroll container making the timeline
  inactive — the animation idle, no effect, no events. A view timeline places its subject from the
  last layout (its offset then added back) and builds §3.4's named ranges from the subject's near edge,
  size and the scrollport shrunk by `view-timeline-inset` (`auto`: the scroller's `scroll-padding`);
  `animation-range` attaches the animation to `[start, end]` in offsets (a name's range, or the whole
  timeline; a percentage of that range). The animation's local time is the offset's place in that
  range mapped onto its timing scaled to fill it (`auto`: the iterations share it), the range's end
  still active (Web Animations 2's boundary rule); events follow the phase changes as on the clock.
  A scroll-driven animation needs no clock frames (`needs_frames` is false): a scroll asks for the
  frame, which steps it with the new offset; after that frame's layout the scroll-driven animations
  step again and the page lays out once more if they moved geometry (§5, a timeline the layout made
  stale — the first frame of a scroller included). `AnimationInfo::timeline_progress`. Red:
  `scroll_timeline_tests` (7: the animation ran on the clock with a zero duration, or stayed at the
  element's width); green after, plus `the_first_frame_shows_a_timeline_its_layout_made`, which fails
  without the post-layout step. Mutation-checked: ignoring the origin's side fails the `rtl` test;
  dropping the end-boundary rule fails `a_scroll_timeline_follows_the_scroll_offset`. `css/mod.rs` (585
  lines) split: the registry side — matching, retiming, building effects, stepping — to `css/update.rs`.
  Remaining (3/3): keyframe selectors naming a timeline range (§4.4).
- 2026-10-08 — C12-SCROLL-DRIVEN (3/3) done: keyframe selectors naming a timeline range
  (Scroll-driven Animations 1 §4.4). `KeyframeSelector` gains its range (`in_range(name, fraction)`,
  any finite fraction; `range()`), rdom-css reads `<timeline-range-name> <percentage>` (a negative one
  too), and `KeyframesRule::resolve` keeps one keyframe per (range, offset). A longhand with such a
  keyframe keeps its keyframes unplaced in `KeyframeEffect`; each sample places them in the
  animation's attachment range through its progress timeline (`ProgressTimeline::place`: the range's
  point against `[range-start, range-end]`, outside [0, 1] allowed), drops them on a timeline without
  named ranges, and adds the implicit 0% / 100% keyframes only where no keyframe reaches them; the
  interval search reads the last keyframe's offset rather than 1. Red: rdom-css
  `keyframe_selectors_may_name_a_timeline_range` (no `KeyframeSelector::range`; the selectors were
  invalid), `scroll_timeline_tests::keyframes_can_sit_on_a_named_range` (the sheet warned on them);
  green after. Mutation-checked: placing nothing fails the engine test. Item done.
- 2026-10-08 — C12-ANIMATABLE leftover: `calc-size()` on `min-*` / `max-*` / `flex-basis` (CSS Values 5 §10)
  and their keywords under `interpolate-size: allow-keywords` (§11). `MinSize`, `MaxSize` and `FlexBasis`
  gain `CalcSize(Arc<CalcSize>)`, `CalcSizeBasis` gains `Content`; the parser takes, per property, the
  bases it allows (`min-*`: `auto` and the intrinsic keywords; `max-*`: the intrinsic keywords;
  `flex-basis`: `auto`, `content` and the intrinsic keywords — an `any` or length basis folding as
  before). No second layout pass is needed for them: `intrinsic::Keywords::min` / `max` measure the
  basis keyword where an intrinsic `min-*` / `max-*` keyword is measured, and apply the sum in the box
  `box-sizing` measures (`auto` on `min-*`: the automatic minimum of a box that is no flex or grid item,
  0 — DIVERGENCES §2); a `flex-basis` one is the sum over the base its keyword gives (§9.2 step 3,
  `main_axis`). Interpolation (`rdom_style::animation::size`) is one generic `Sizing` rule for the four
  types, so a keyword of each interpolates with a length through `calc-size()`. Red: the four
  `runtime::app::calc_size_tests` (the values were invalid; `min-width: 0` → `max-content` did not
  interpolate); green after. Mutation-checked: ignoring a `calc-size()` min fails the floor and
  transition tests. Breaking (rdom-style): the three enums' new variant (CHANGELOG, `calc_size_hints`).
- 2026-10-08 — C12-STARTING leftover: starting styles for `::before` / `::after` (CSS Transitions 2 §3).
  `cascade::starting_style` takes the slot: a pseudo-element's is computed through
  `compute_pseudo_style` over its originating element's computed style with the `@starting-style` rules
  applying, and exists when one of them matched it. The cascade hook treats a `::before` / `::after`
  that generated no box at the last style update (no previous style) as newly rendered, as it does an
  element out of `display: none`, so its transitions start from that style (and without one its
  values change at once, as before). The public `diff_and_register_with` keeps its element-only
  closure. Red: `starting_style_tests::a_new_pseudo_element_transitions_from_its_starting_style` (the
  `::before` showed opacity 1 at once); green after. DIVERGENCES §4 rewritten: the pseudo-elements are
  no longer excluded; a starting style still inherits from the parent's computed style. The `<details>`
  closing animation still waits for C14's `content-visibility`. Phase 12 part 2 (C12-KEYFRAMES,
  C12-SCROLL-DRIVEN, the part 1 leftovers) done.
- 2026-10-08 — Phase 12 part 2 docs: CSS-COVERAGE's priority table row 21 (`@keyframes` + `animation-*`)
  marked shipped (the §3.18 and §3.11 rows and the §1 counts moved with each item: 3.18 is 10 / 0 / 0,
  3.11 14 / 0 / 0, the total 222 Supported, 8 Partial, 32 Missing). ACID gains I17 (a keyframe
  animation at fixed clock times: keyframe easing, directions, fills, `!important`, the order above
  transitions, `add`, the four events) and I18 (a scroll-driven progress bar: `scroll()` with no clock
  tick, `rtl`, a `view()` item with a range and a range keyframe, `timeline-scope`, an inactive
  timeline). Docs only.
- 2026-10-08 — C12-FOCUS-FLUSH (TECH_DEBT `FOCUS-FLUSH-1` closed; HTML §6.6.3 focusing steps, §6.6.6
  `focus()`). Found: `focus()` decided focusability on the last frame's styles, so "show the panel, focus its
  input" in one handler was refused (the upgrade guide's nested-rAF workaround). Decided — the cascade's
  inputs as document data, as the focus deferral is (C9G): `runtime::style_flush` keeps `StyleInputs` (the
  sheets in cascade order as shared `Rc<Stylesheet>` handles — the App's author sheets, the `<style>`
  sheets and the `CSS.registerProperty` sheet now live behind `Rc` — the property registry, the dirty
  tracker) published by the App at every sheet-set change (`FramePrelude::sync_sheet_set`); no rdom-core
  hook was needed, since `focus()` itself is rdom-tui's. `flush_style(dom, id)` (public) takes from the
  tracker only the roots that hold `id` (`DirtyTracker::take_roots_holding`: `id` and its ancestors) and
  cascades their subtrees as the frame would; the rest stays dirty. `TuiAccessorsMut::focus` /
  `focus_with` call it first. The transition registry is *not* reached: the before-change style is kept per
  element (`computed_prev`) until the frame's transition hook, so the flush leaves a `flushed` flag on the
  tracker and the next frame runs its hook (and lays out) even with no root left — transitions start at the
  frame's time, and two flushes in one handler are one style change (DIVERGENCES §2). Scope: style only —
  layout stays whole-tree and per frame, so `bounding_rect`, the scroll metrics and the focus scroll read the
  last layout; a whole-tree restyle the App has pending and `<style>` text edited in the handler are not
  flushed; `&self` readers (`computed()`, Tab) read the last style update. Red:
  `runtime/style_flush/tests.rs` (3 of 4 failed: the focus refused, `None` for `Some(input)`; the frame test's
  "the flush took them"), and `css_phase6/visibility_answers.rs`'s pin, rewritten as
  `focusing_a_just_shown_input_flushes_its_style` (the handler and one-rAF cases now focus the input — the
  changed expectation this item exists for). Green after. The counting test measures the panel subtree's
  cost from a frame with only it dirty, then flushes with an unrelated section dirty too: the flush matches
  the panel's cost (13 matching passes, the panel and input with their pseudo-elements), the section stays
  dirty, and a second `focus()` with nothing dirty matches 0. Mutation-checked (restored, touched): the
  frame ignoring `flushed` fails the transition test; taking every root fails the count (68 for 13).
  Docs: DIVERGENCES §2's "Focusability reads the last cascade's styles" replaced by what a flush covers;
  the focus-scroll entry no longer names the debt; upgrade-guide item 20 loses the workaround.
- 2026-10-08 — C12-OUTLINE (CSS UI 4 §5, CSS 2.1 Appendix E step 10). rdom-style: `outline-style` (`auto |
  <outline-line-style>` — every `<line-style>` but `hidden`, rdom's `half-block` not taken), `outline-width` (a
  `<line-width>`, so a pixel width selects the glyph weight, DESIGN's rule), `outline-color` (`auto | <color>`,
  kept as a `TuiColor` and resolved at paint like `caret-color`), `outline-offset` (a `<length>` of either sign,
  a pixel offset one cell its way through `PaintLength::offset_cells`) and the `outline` shorthand (any order,
  omitted parts reset; a lone `auto` is the style, as browsers read it); serialized shortest. Stored in a new
  group, `TuiStyle::ui` / `ComputedStyle::ui` (`UiDeclarations` / `UiStyle`, closed like the text and font
  groups), which the rest of Phase 12 part 3 fills; none inherit; widths and offsets absolutize their viewport
  units; the animation table types them (style discrete, the rest by computed value — two colors interpolate,
  `auto` does not) and marks all four paint-only (`affects_layout` false: an outline takes no room). rdom-tui:
  `paint_pass/outline.rs`. A box paint (`box_paint::paint_box`, and a generated `::before` / `::after` box)
  records its outline on the buffer — the ring rect (border box grown by offset + 1), the clip it painted into,
  the line, weight, rounded flag and resolved color (`auto`: the accent under `outline-style: auto`, else
  `currentcolor`); each stacking context's walk notes the buffer's outline count on entry and draws the ones
  recorded since on exit, so a context's outlines come after all it painted, its positioned descendants
  included, and nested contexts draw their own first. A ring cell joins the ring cells beside it and takes its
  glyph from the border tables (`border_join::outline_glyph`: `double`, the dash runs, light / heavy, rounded
  corners for `auto`), clears the cell's border state as content does, and is clipped by the box's own clip —
  its `overflow` ancestors. Nothing in layout reads the outline, so it neither moves boxes nor adds scrollable
  overflow. The UA focus cue was weighed against moving to `:focus-visible { outline: auto }`, as browsers draw
  it: not taken — a one-cell ring around a one-row control covers the rows above and below and a column each
  side, i.e. the neighbouring controls, where a browser's ring covers a pixel of margin; the tint stays
  (DIVERGENCES `FOCUS-VOCAB-1`, which now says so and points authors at `outline: auto`), so no paint or
  snapshot changes. Red: rdom-style `outline_tests` did not compile (no `TuiStyle::ui`); the paint tests
  `css_phase12/outline.rs`, run with the ring never drawn, failed 7 of 9 (the two passing pin no room taken and
  `none` drawing nothing); green after. Mutation-checked: drawing at box paint instead of the context's end
  fails the over-the-next-box, after-positioned and negative-offset tests. Changed tables: the property and
  important-setter contracts, the `initial` perturbation and the longhand interpolation samples gain the four
  longhands. CHANGELOG silent change 20 (outline declarations now draw).
- 2026-10-08 — C12-CURSOR (CSS UI 4 §4.1; HTML §15.3.4). rdom-style: `cursor` — `[<url> [<x> <y>]?,]*
  <cursor-predefined>`, the keyword required last — as `Cursor { images, keyword }` (`CursorImage`, the 36
  keywords and `auto` / `default` / `none` in `CursorKeyword`) in the `ui` group; inherited, discrete,
  paint-only for the animation table. The UA's `a[href]` takes `cursor: pointer` (HTML §15.3.4's `:link,
  :visited`). rdom-tui: `runtime::pointer_shape` — `PointerShapes { None, Osc22 }` (`#[non_exhaustive]`)
  detected as `SgrCapabilities` is (C9G-SGR-CAPS): a multiplexer gets `None` (it would have to pass OSC 22
  through), kitty / foot / WezTerm / Ghostty (`TERM`, `TERM_PROGRAM`, `KITTY_WINDOW_ID`) get `Osc22` — the
  terminals that take the CSS names; xterm's OSC 22 takes X cursor-font names and is left out. `App::new`
  detects, `App::with_pointer_shapes` overrides, a `with_backend` app sends nothing. The `App` keeps the
  pointer's last position and the shape it last sent (`app/pointer.rs`) and, after each mouse event and each
  drawn frame (a still pointer's element may change, or its `:hover` style), sends `OSC 22 ; <name> ST` when
  the shape under the pointer changed: the hit element's computed keyword, `auto` the text pointer over an
  editable control or a line of selectable inline content (`hit_test::over_selectable_text`), `none`
  `default` (no protocol hides it). Restoration: the first shape sent sets a process-wide flag (the panic hook
  has no `App` to ask), and `restore_terminal` — `leave_tui_mode`, so the `TerminalGuard`'s drop, a normal
  exit and the panic hook — sends `OSC 22 ; default ST` while it is set (read, not taken, so concurrent
  restores cannot race). Red (by mutation, the tests written first against the finished API): with `auto`
  never the text pointer, `the_pointer_follows_the_element_under_it` fails at the paragraph; with no restore
  bytes, `leaving_tui_mode_restores_the_pointer` fails; with no update after a frame, the `:hover` style's
  `grab` is never sent (only the pre-hover `pointer`). Detection pinned by `detection_is_conservative`; the
  dispatch by `ui_tests::cursor_takes_image_fallbacks_and_a_keyword`. `property_dispatch/outline.rs` became
  `ui.rs` (with its tests), the home of the CSS UI 4 arms. CHANGELOG silent change: the pointer changes shape
  in those terminals.
- 2026-10-08 — C12-CARET (CSS UI 4 §6.1–§6.2). Checked first: rdom paints its own caret (the hardware cursor
  is hidden at startup and never shown; `inline_paint/caret.rs`, a cell in `caret-color` / rdom's
  `caret-text-color`, its blink a phase the runtime writes, `runtime::caret_blink`), so `caret-shape` maps to
  the painted caret, not DECSCUSR. `caret-color` was already complete (`auto | transparent | <color>`,
  inherited, interpolated, `currentcolor` / `var()` / `light-dark()` resolved at paint under the element's
  scheme); its parser moved to `parse_caret_color`, shared with the shorthand. rdom-style: `caret-shape: auto |
  bar | block | underscore`, `caret-animation: auto | manual` (both inherited, discrete, paint-only) in the
  `ui` group, and `caret: <'caret-color'> || <'caret-animation'> || <'caret-shape'>` (an `auto` to the first
  component still free, omitted ones reset; shortest serialization). Paint, per cell: `auto` and `block` as
  before (no existing caret paint changes); `underscore` keeps the glyph, its colors and background and
  underlines the cell with `caret-color` as the underline color (SGR 58 where the terminal has it); `bar` is
  `▏` (U+258F) in `caret-color` on a blank cell, the underscore's underline over a glyph — a cell holds one
  glyph (DIVERGENCES §2). `caret-animation: manual`: the blink controller keeps the phase on and schedules no
  flip for a focused element with it (an unfocused terminal still hides the caret), and the painter ignores
  an off phase under it. Red: `editing/caret/shape_tests.rs` 4 of 5 failed (underscore and bar painted the
  block; the manual caret hidden by the off phase; the shorthand's underline missing — `block_and_auto`
  passing, pinning the unchanged default), `caret_blink/tests.rs::caret_animation_manual_does_not_blink`
  failed (a flip still scheduled); green after. Dispatch: `ui_tests::the_caret_longhands_and_shorthand`.
- 2026-10-08 — C12-CONTROLS (1/4), `accent-color` (CSS UI 4 §6.3). rdom-style: `auto | <color>`, inherited,
  by computed value between two colors (`AccentColor`, kept as a `TuiColor` like `caret-color`, in the `ui`
  group). rdom-tui: `style/accent.rs` resolves an element's used accent (against its color and used scheme;
  `None` for `auto`), read by the chrome that a browser fills with it — the progress bar (`gauge`'s fg
  override; `<meter>` keeps its zone colors, as CSS UI 4 leaves it out), the range slider's track and thumb
  (its canvas paint), and a checked or indeterminate checkbox's or checked radio's mark: the cascade sets
  the toggle's `::before` color to the accent right after computing it (`early_pseudos`), so the mark is
  restyled with `:checked` and inherits nothing odd; an unchecked toggle keeps the text color. No default
  paint changes (`auto` everywhere). Red, by mutation of the finished code (an accent never resolving):
  both `css_phase12/controls.rs` tests fail at the checked checkbox and the progress bar; dispatch pinned by
  `ui_tests::accent_color_is_auto_or_a_color`. Remaining: `appearance`, `field-sizing`, `resize`.
- 2026-10-08 — C12-CONTROLS (2/4), `appearance` (CSS UI 4 §7.1). rdom-style: `none | auto | base |
  <compat-auto> | <compat-special>` (`Appearance`, not inherited, discrete) and the legacy alias
  `-webkit-appearance` on the same storage (as `word-wrap` is `overflow-wrap`'s). "Chrome", per built-in —
  defined because a browser's "native appearance" is a platform widget and rdom's is CSS and paint: the
  toggles' marks, the buttons' `[ ` / ` ]` and a drop-down `<select>`'s `▾` are the UA origin's `::before` /
  `::after` rules (`style::accent::draws_pseudo_chrome` names those controls), which a control with
  `appearance: none` does not take — `compute_pseudo_style` drops the UA rules from its gather
  (`Scratch::drop_user_agent`, `sorted` and `ranks` together; the ladder comes from the author rules, and
  what the gather recorded for reuse is untouched), so an author `::before` still draws; the gauges' bar
  (the inline chrome supplier gives empty text) and the range track (its canvas paint) are the painted
  chrome. A text field's `Field` background is ordinary UA CSS and stays (browsers keep the UA's
  declarations under `none`). `base` and the compat keywords are `auto`. Red, by mutation of the finished
  code (no control counted as chrome, no `none` read by paint): both appearance tests in
  `css_phase12/controls.rs` fail; dispatch pinned by `ui_tests::appearance_keywords_and_the_legacy_name`.
- 2026-10-08 — C12-CONTROLS (3/4), `field-sizing` (CSS UI 4 §7.2). rdom-style: `content | fixed` (`FieldSizing`,
  not inherited, discrete). The model: a browser's text field has an intrinsic size (`size`, `cols` /
  `rows`) that `content` replaces with the content's; rdom's UA writes fixed sizes instead, so the cascade's
  new `field_sizing::finalize` (after the ladder, with the element's matched rules and inline style in
  hand) sets a text `<input>`'s or `<textarea>`'s `width` to `max-content`, and a textarea's `height` to
  `auto`, wherever no author or inline declaration — a value, a `var()` one, or a flow-relative
  `inline-size` / `block-size` — sets them. Layout then sizes the field by its text child and padding (an
  empty one to its padding and placeholder). `<select>` is left out (its size is its options'). Red, by
  mutation (the finalizer a no-op): `field_sizing_content_sizes_a_field_to_its_value` reads `(22, 1)` for
  `(7, 1)`; the fixed and author-width cases pin the rest. Dispatch: `ui_tests::field_sizing_is_content_or_fixed`.
- 2026-10-08 — C12-CONTROLS (4/4) done, `resize` (CSS UI 4 §4.2). rdom-style: `none | both | horizontal |
  vertical | block | inline` (`Resize`, not inherited, discrete; `axes()` maps the flow-relative keywords onto
  the physical axes of a horizontal writing mode, closing CSS-COVERAGE §3.22's logical-keywords row); the UA
  sheet's `textarea` takes `both`, as HTML's rendering section and the engines' UA sheets do. Feasible, so
  done rather than left inert: `runtime/resize.rs` — a left press (before the scrollbar's own hit test, as
  browsers put the resizer over the scrollbar's end) on the bottom-right cell of a scroll container on the
  hit path whose `resize` allows an axis starts a drag that takes the pointer (`Router::resize_drag`); each
  captured move writes the border box the pointer gives, less padding and border under `content-box`, as
  `width` / `height` through the reflecting setters (so into the `style` attribute, as browsers write the
  resized size), the content box never below one cell; the release ends it, a lost release is cancelled by
  the next press, a button-less move ends it, as for the thumb. No grip is drawn (no paint change).
  Red, by mutation (no box counted resizable): `dragging_the_corner_resizes_the_box` reads `(6, 2)` for
  `(9, 4)` and the textarea test `(10, 4)` for `(6, 6)`. Dispatch: `ui_tests::resize_keywords_and_their_axes`.
  CHANGELOG silent change: a textarea's corner press now resizes it. C12-CONTROLS done.
- 2026-10-08 — C12-SELECT-TOP-LAYER (Phase 11 API N7, left to C12 at Phase 11's close; HTML's select picker,
  CSS Position 4 top layer, HTML §6.12.2 light dismiss). Found: an open drop-down set `data-rdom-open`, which
  the UA sheet answered with `height: auto` — the select grew in flow to list its options, pushing the page
  down, clipped by an `overflow` ancestor and under later stacking contexts. Decided — keep the look (the
  option list from the select's row) but render it as a browser's picker: rdom-core's top layer gains
  `TopLayerKind::Picker` (`#[non_exhaustive]` enum, additive), which `select::open` adds the select as and
  `close` removes (a select not in the document does not open, as `showPicker()` throws there). The
  select's box stays in flow (`finalize_top_layer` skips a picker, so its `position` is not forced) at its
  one row (`select[data-rdom-open]` keeps `height: 1`, `overflow: visible`); its options overflow below it on
  the field background (`:where(select[data-rdom-open]) > option`, below the selected / highlighted options'
  own; UA count 185 → 186); being a top-layer member, the stacking walk leaves it out and
  `paint_pass::top_layer` draws it after the document, clipped by the viewport only; the hit test, which
  tries the top layer first, descends into a picker's content outside the select's own box; the scrollable
  overflow walk counts the select's row but not its options. Light dismiss shares the popover router hooks
  (`popover::light_dismiss::pointer_down` / `pointer_up` call `select::light_dismiss_down` / `_up`): a
  release closes each open picker that neither the press nor the release was inside — the same press /
  release rule as an auto popover's; it is not a popover otherwise (not `:popover-open`, no close watcher:
  Esc stays the select's key; opening hides no popover — DIVERGENCES §2). Red:
  `select/top_layer_tests.rs` 3 of 3 failed (not in the top layer; the select 3 rows tall; the option click
  missed — the overflowing options were clipped); green after. Mutation-checked: no light dismiss fails
  the dismiss test. Changed test: `open_dropdown_renders_options_inline_without_chrome` opens its select
  after inserting it (its comment said no top layer); its expectations stand. No snapshot changed.
  CHANGELOG silent change 21.
- 2026-10-08 — Phase 12 part 3 docs and close: every Phase 12 row is done (C12-FOCUS-FLUSH, -OUTLINE, -CURSOR,
  -CARET, -CONTROLS in four commits, -SELECT-TOP-LAYER added as a row); the Phases table reads "items done,
  gates pending". CSS-COVERAGE: §3.19 is 9 Supported / 1 Partial (`pointer-events`) / 1 Missing (`nav-*`, the
  decided exclusion) / 1 N/A; §3.22's logical-keywords row closed with `resize: block / inline`; the total
  230 / 8 / 24 / 45; the priority rows 10, 42–45, 67, 68 marked shipped. ACID: tile 15 gains the controls
  (`accent-color`, `appearance: none`, `field-sizing`), the open picker over the page and the outline
  rings; I3 the `outline: auto` focus ring and the flushed "open panel, focus input"; I9 the caret shapes,
  `caret-animation: manual` and the pointer shapes. TECH_DEBT `SIZE-1` recount of the files part 3 grew:
  `runtime/app/mod.rs` 562, `style/cascade/apply.rs` 518 and `runtime/router/mouse/mod.rs` 506 (new to the
  list), `property_dispatch/set.rs` 500 — none past 575. Docs only.
- 2026-10-08 — Phase 12 gates (with the C11G re-review: all 15 hold; `Dom::is_inert` climbs DOM
  parents, misjudging a positioned `::details-content` box). Architect: 2 blocking — an element
  detached by `remove_child` (not dropped) animates forever (pruning uses `contains`;
  `cancel_for_node` never called; 60 fps idle, no cancel events); a `::before` / `::after` that stops
  generating mid-animation keeps its presentation base, so it stays painted and frames run forever.
  API: 2 blocking — `appearance: none` (the widest Phase 12 silent change) is not in the ranked
  upgrade list; DIVERGENCES contradicts itself on scroll-driven animations and leaves an empty §3
  heading. Non-blocking: before-change and rendered state read from the cascaded, not composited,
  style (reopening during an exit transition misbehaves); layout decided by which longhands animate
  rather than whether values changed (a `visibility` blink lays out every frame); the transition diff
  builds two N-entry maps per cascade; worst case 18 (36) pass-1 layouts per frame, the `calc_sizes`
  flag never cleared, ANIM-RELAYOUT-1 understated; an invisible resize hot spot on every textarea, resize
  ignores the gutter and rewrites both axes; removing an open `<select>` leaves it open; outline not
  drawn on inline elements; timing / event spec gaps and scroll-driven restyle lag; infinite animations
  request every frame (a `steps(1)` blink, an inert `transform` spinner); `computed` now holding running
  values is undocumented and the cascade-only accessor (`cascaded_for`) is TuiExt-only and misnamed;
  uneven re-exports (transition types, `AnimationInfo`, `Longhand`); dead / leaky surface
  (`AnimationEventKind`, `ActiveAnimation`, vestigial `effective_*` with a `Reset` fallback);
  `TransitionProperty::Named` accepts any string; `TimingFunction` / `Appearance` should be
  `#[non_exhaustive]`; builder gaps (`overlay`, view-timeline axis / inset, `timeline_scope`,
  `.cursor()` needs `.into()`, two `Cursor` types, `LinearStop::new` argument order); upgrade-guide
  placement (`display:none` fade-in break at 74) and missing items (negative delays, `linear()`,
  `@starting-style`), rows 141 / 151 wrong; `App` configuration scattered and its module doc stale;
  READMEs lag (no animation doctest, porting recipes, stale rdom-style lists, a stale UA comment);
  `design_types.rs` weaker than its rule (globs, macro types, any backticked name, no
  `#[non_exhaustive]` check); SIZE-1 stale (`ladder.rs` 571, `rdom-css/src/block.rs` unlisted).
  Full reports: `target/claude-logs/c12_gate_{architect,api}.md`. Fix as `C12G-*`, two batches.
- 2026-10-08 — C12G-DETACHED (architect B1, N2b; CSS Animations 1 §4.1, CSS Transitions 1 §3, Web
  Animations 1 §5.6). Found: the registry pruned by `dom.contains`, true for a node `remove_child`
  detached and kept, and `cancel_for_node` had no caller — a removed spinner pumped 60 fps forever and its
  transitions ended with `transitionend`; a re-inserted element diffed against its pre-removal
  `computed_prev`. Decided — a removal is a disconnection even when the element comes back in the same
  task (browsers cancel and restart; `moveBefore` exists to avoid exactly that): the dirty tracker's
  observer records each removed element (`take_detached`, the existing mutation path, no per-frame tree
  scan); the frame, before its cascade, cancels every transition and animation in each removed subtree
  still in the arena, with their events, and forgets its before-change styles and composited values
  (`runtime/animation/teardown.rs`, `TuiExt::forget_rendering`), so a subtree inserted again is newly
  rendered (its `@starting-style`, its animations from the start). A backstop in `advance_frame`
  cancels the entries of any node not connected (a `::details-content` box counts as its `<details>`),
  O(entries × depth), and drops a dropped node's silently, as before. Red: `app/teardown_tests.rs` 4 of 4
  (no `animationcancel`, `transitionend` instead of cancel, width 10 for 2 on re-insertion, 6 for 2 on
  restart) and `animation::tests::advancing_cancels_the_transition_of_a_detached_element`; green after.
  Mutation-checked: the removal hook off fails the two re-insertion tests; the backstop off fails the
  registry test. Found and recorded (TECH_DEBT `MOVE-RECORD-1`): moving an attached node with
  `append_child` fires no removal record, so a moved element keeps its animations. CHANGELOG silent
  change 77.
- 2026-10-08 — C12G-PSEUDO-GONE (architect B2; CSS Pseudo-Elements 4 §2, CSS Transitions 1 §3, CSS
  Animations 1 §4.1, CSS Lists 3 §4.5). Found: a slot whose cascade gave `None` cleared only
  `computed_before` / `computed_after`; `presentation_*.base` stayed, `cascaded_for` preferred it, so the
  diff saw no change, the transitions and animations ran on and `composite` put the stale style back.
  And rdom kept a style for a rule-matched `::before` / `::after` whose content was `none` / `normal` — a
  pseudo-element with no box, on which a transition still ran (the architect's hover tooltip, its
  content only on `:hover`, kept a style that reversed its color). Decided, at the roots: (1) the
  cascade gives such a pseudo-element no style at all — `compute_pseudo_style` returns `None` before its
  counter ops when the declared content is `none` / `normal` (or undeclared without legacy text), so it
  also stops counting (CSS Lists 3 §4.5); a `var()` / `attr()` content that resolves to nothing keeps
  its style, as before; (2) `write_pseudo(None)` drops the slot's presentation record
  (`TuiExt::drop_pseudo`), and `cascaded_for` is `None` whenever `computed_for` is — no style of either
  kind comes back; (3) the diff notes a slot that had a previous style and has none
  (`stopped_generating`): its transitions are cancelled (`transitioncancel`) and its CSS animations
  updated with no style (`animationcancel`). Red: `app/teardown_tests.rs` — the spinner `::after` with
  `.s` removed kept animating (no `animationcancel`), the tooltip `::after` kept a style; green after.
  Mutation-checked, each alone: the content rule off fails the tooltip and both cascade tests; the
  presentation drop and `cascaded_for` guard off fail both teardown tests; the diff's cancel off fails
  both. Changed test: `cascade::tests::content_none_suppresses_pseudo` expected a style with no content
  (the old model); it now expects none — the suppression it pins stands. New
  `content_normal_before_generates_no_box_and_no_counter_ops`. Checked: `::marker` takes no animation
  overrides (no presentation slot), so it cannot keep a base; a `::details-content` box always has the
  UA's style, and its `<details>` leaving the document is C12G-DETACHED's (the box is matched by its
  host); a box dropped by `sync_content_box` (a sheet set without the UA's rule) drops its effects
  silently with the node. CHANGELOG silent change 78.
- 2026-10-08 — C12G-BEFORE-CHANGE (architect N2a; CSS Transitions 1 §3, the before-change style "with any
  styles derived from declarative animations ... updated to the current time"). Found: `was_rendered`
  read the previous *cascaded* `display`, so a popover reopened halfway through its fade-out — its
  `display: none` held as `block` by `allow-discrete` — counted as newly rendered: the starting-style
  branch ran, which names only `opacity`, and the running `color` transition went on to the closed color
  and snapped back at its end. Decided: the rendered state reads the before-change `display` — the
  running value while a transition or animation holds it, which the cascade's write-back
  (`TuiExt::overlay`) carried into the computed style, else the previous cascade's
  (`diff::before_change_display`). The longhand values already start from the composite: a transition
  replacing a running one starts from its current value (`register`). Remaining nuance, recorded here: a
  longhand a CSS animation drives is compared cascade to cascade, so a cascade change to it can start a
  transition under the animation (the spec's before- and after-change styles both carry the animation's
  value, so none starts); it is masked while the animation applies. Red:
  `starting_style_tests::a_popover_reopened_during_its_fade_out_reverses_from_where_it_is` — red 33 after
  76 (fading on to black); green after (it rises, ends at 200 with nothing running). N2b (re-insertion)
  was C12G-DETACHED's.
- 2026-10-08 — C12G-FRAME-COST (architect N3, N4; API N9; Web Animations 1 §4–§5, CSS Easing 1 §2.3,
  Scroll-driven Animations 1 §5). Six parts, one counting test each in `app/frame_cost_tests.rs`, all red
  first. (1) Layout by value: `composite` compares each layout-read longhand (animated now or before) of
  the previous composited style with the new one (`Longhand::differs`) — not "is one animating"; a
  continuous color pulse beside a `step-end` `visibility` blink painted 63 frames and laid out 63, now 2
  (red 63). `visibility` and `z-index` stay layout longhands (`collapse` is layout; the value test
  covers the blink). (2) Next-change scheduling (`css/schedule.rs`, `animation/schedule.rs`): an
  animation whose every keyframe interval is `steps()` (`KeyframeEffect::change_points`: each interval's
  n divisions and its ends) wakes at the next such point after its last step — mirrored in a backwards
  iteration (§4.9.1) — or the iteration's end; in its delay at the active start; continuous ones every
  frame. `needs_frames` is "a frame is due now"; the App notes the frame at a frame's start
  (`service_animation_clock`) and the live loop's poll timeout wakes at `next_wake`. A `blink 1s
  step-end infinite` over one second: 2 paints and 2 composites (red 63 / 63); the alternate test pins the
  mirroring (mutation-checked: the forward mapping leaves it hidden at 1760 ms). (3) An empty effect (its
  keyframes name only what rdom does not render — `transform` until Phase 15): no frames; its events
  (start, iterations, end) are due at their local times (`next_event`) and stepped without a frame
  (`step_events`) — 0 paints, events at 0 / 100 / 200 / 300 ms (red: `needs_frames` true). (4) The
  transition hook visits only what the cascade recomputed: `cascade_subtrees_all_with` returns its roots
  (the widened ones too), a style flush records its roots (`DirtyTracker::note_flushed`, was a flag),
  and `diff_and_register_in` walks those subtrees (`scoped_element_ids`: connected, outermost), the
  rendered maps answering a parent outside them by climbing its box ancestors; a full cascade or a changed
  sheet set walks the tree. A class change on one of 300 rows: 1 visit (red 300). (5) The `calc_sizes`
  flag clears when layout's collecting walk finds no `calc-size()` box; to keep a re-inserted subtree
  from hiding one, C12G-DETACHED's forgetting now drops the computed styles of a subtree still out of
  the document at the frame (one re-inserted in the same task keeps them, its running values put back to
  the cascade's), so its insertion is styled afresh and noted. (6) The scroll-driven re-step shares the
  services' relayout (re-snap, focus scroll, caret reveal — the timelines read the offsets as the
  services left them, against the first layout): at most 2 `layout_dom` a frame; a frame needing a focus
  scroll, a re-step and a `calc-size()` box ran 6 phase runs, now 4. TECH_DEBT `ANIM-RELAYOUT-1`
  rewritten with the bound: 2 × 2 × `MAX_ROUNDS` = 12 per frame, 24 with an autoscroll tick's off-frame
  pass (was 18 / 36). Combined mutation run (all six reverted): the six tests fail. Changed tests:
  `animation_event_tests::an_infinite_animation_costs_a_composite_per_frame_and_layout_only_for_geometry`
  expected a layout on every frame of a whole-cell `height` animation; it now expects one on the frames
  whose height moved (2 of 4), the contract changed; `teardown_tests::a_detached_spinner…` asserted
  `needs_frames` for a `steps(4)` spinner between steps — it now asserts the spinner wakes the app
  (`next_wake`), and no longer does once removed. CHANGELOG silent change 79. Not done here: transitions
  with `steps()` still run every frame (their cost is bounded by their duration); `keep_cascaded`'s
  per-restyle `Rc` (architect N3's last line) and nested `calc-size()` (N4) are left for batch B.
- 2026-10-08 — C12G-RESIZE-PICKER (architect N5, N6; CSS UI 4 §4.2, HTML's select picker and removing
  steps). Resize — decided: draw the grip (the first option), since a resizer browsers show is the
  faithful affordance and excluding the hot spot where a box has no room would have made `resize`
  unusable on every padding-less box (the C12-CONTROLS tests drag exactly such a box). One answer for
  the hot spot and the glyph, `render::resizer::cell`: the bottom-right cell of the padding box (inside
  the border; a bordered box's corner glyph stays), of a scroll container whose `resize` allows an axis —
  `overflow: clip` no longer counts (§4.2: scroll containers only); paint draws `◢` there in the box's
  `color` after its scrollbars (`paint_scrollbars` → `resizer::paint`), so the press that starts a drag
  is always on the visible grip, over the content or a bar's end. The drag's `extra` (what a
  `content-box` size leaves out) is measured from the layout — border box less content box less the
  gutters, which the content box gives up inside the size — so a percentage padding resolves against
  the containing block (it used the box's own width: a one-cell drag grew a `padding-left: 50%` box by
  three). A move writes only an axis `resize` allows whose size it changed (`ResizeDrag::written`):
  a horizontal drag no longer rewrites `height`, an unchanged move writes nothing (each write is a
  mutation, a restyle and a layout). The architect's "2 cells per 1" with a gutter did not reproduce —
  rdom's gutter is inside the size, as in browsers; the stable-gutter case is pinned. Picker: `open`
  records the select in document data, `close` forgets it, and `select::settle_pickers` — run with the
  selectedness flush at the App's next event or frame, as popovers settle — closes a recorded select
  that left the top layer (rdom-core's removing steps took it out) or is disabled
  (`Dom::is_actually_disabled`): no `data-rdom-open` left, so inserted again it is a closed drop-down.
  Red: `css_phase12/controls.rs` `a_resizable_scroll_container_draws_its_grip` (no grip),
  `a_one_cell_drag_is_a_one_cell_resize` (19 for 17), `a_horizontal_drag_writes_only_the_width`
  (`height: 2` written); `select/top_layer_tests.rs` `removing_an_open_select_closes_its_picker`,
  `disabling_an_open_select_closes_its_picker` (still open); green after. Snapshot changed:
  `rdom-showcase/tests/snapshots/tab_form.snap` — the Notes textarea's bottom-right cell shows `◢`
  (the text layer only; the background section is unchanged: the grip takes the cell's background).
  DIVERGENCES §2's resize entry rewritten (grip drawn; the corner inside the border); CHANGELOG silent
  change 76 updated.
- 2026-10-08 — C12G-OUTLINE-INLINE (architect N7; CSS UI 4 §5, §5.1). Found: `outline::defer` was called
  from the block and generated-box paints only, so an inline element's outline — the
  `a:focus-visible { outline: auto }` DIVERGENCES `FOCUS-VOCAB-1` recommends — drew nothing. Decided:
  per-fragment rectangles (§5.1 allows one non-rectangular outline or one per fragment; whole-cell
  rectangles keep each ring a box-drawing ring). The line walker, after a flow's lines, records a ring
  for each inline element with an outline on each line it has a fragment on
  (`inline_paint/outline.rs`): the cells its text fragments, its unmoved or moved `::before` /
  `::after` runs and the atoms inside it take on that line, its rows the rows they span; the climb from
  a fragment's element goes through `display: contents` ancestors and stops at the first box that is not
  inline (the flow's block), so text directly in a block costs one style read. Rings are deferred to the
  stacking context's end with the flow's clip, as a block's are. Red: `css_phase12/outline.rs`
  `an_inline_element_draws_its_outline` and `a_wrapped_inline_element_rings_each_line_fragment` (no
  ring); green after. DIVERGENCES §1's outline entry gains the per-fragment sentence; CHANGELOG silent
  change 20 names inline elements. No snapshot changed.
- 2026-10-08 — C12G-MISC (the C11G re-review's `is_inert`; architect N8, N9's restyle lag and recursion;
  the stale comment; SIZE-1). (1) Inertness: `Dom::is_inert` climbs DOM parents, and a positioned
  `::details-content` box is a parentless layer entry; rdom-core cannot know the box, so the one caller
  that meets it — `hit_test::descend::hit_layers` — asks for its `<details>` (`slot::host_of`); the
  other readers (focus, Tab, selection, the top layer, generated hosts) are handed DOM elements. Red:
  `inert_tests` — in a modal dialog the click was swallowed (0 for 1), under `[inert]` it landed (1 for
  0). (2) Timing and events (each red first): reversing (CSS Transitions 1 §3) — `reversing.rs` keeps, for
  a transition a reversal started, its reversing-adjusted start value and shortening factor; a change
  back to a running transition's adjusted start shortens the new one's duration (and a negative delay)
  by |output · factor + 1 − factor| — red: `0 → 10` reversed at 30 ms was still at 2 after 31 ms, now 0
  with a 30 ms `transitionend`; `transitioncancel`'s `elapsedTime` is the active time (delay excluded,
  clamped to the duration), for registered custom properties too — red 50 ms for 0 inside the delay; one
  frame's events merge by time — transitions and custom properties carry the time they happened
  (`take_timed_events`, the public `take_pending_events` unchanged), the dispatch moved to
  `app/animation_events.rs` and sorts the three streams by time, transitions before animations at one
  time — red `transitionend` (30 ms) before `animationstart` (10 ms); keyframe step easings take the
  before flag in the before phase (`KeyframeEffect::apply` gets it) — red width 6 for 2 in a
  backwards-filled delay; rdom-style: `<integer>` interpolation rounds a half toward +∞ (red: `order`
  0 → 1 half-way was 0), a `<time>` may carry `+` (red: `+1s` rejected, in lists and both shorthands).
  (3) Scroll-driven: the post-layout re-step now drains the restyle queue (`restyle_animated`, shared
  with the main step) and its relayout counts it — red: a scroll-driven `color` reached `#bar`'s span a
  frame late; `descendants_declaring` walks with an explicit stack. Left: a `::before` `view()`
  timeline still takes its host as the subject (N9's third bullet). (4) `tree_guides.rs`'s comment no
  longer says `Reset`. (5) SIZE-1 recounted (2026-10-08; the old date was one not yet reached):
  `animation/mod.rs` 558 joins the list, `rdom-css/src/block.rs` 543 is listed, `cascade/ladder.rs` is
  553 by the rule (untouched, so not split), `walk.rs` left it. CHANGELOG silent change 80.
- 2026-10-08 — C12G-MOVE-RECORD (TECH_DEBT `MOVE-RECORD-1`; DOM §4.2.3 "insert" steps 4 and 7.1, "adopt"
  step 2, "replace", "pre-insert" step 3). Found: `append_child` / `insert_before` of an attached node
  unlinked it (`detach_from_parent`, which already ran focus / hover / selection purging, the top
  layer's removing steps and the live ranges' "remove" steps) but fired only the insertion's record, so
  an observer — the dirty tracker's C12G-DETACHED teardown among them — never saw the old parent lose
  it; a fragment's emptying fired nothing; `replace_child`'s record did not name the replaced child;
  and replacing a child with its own next sibling (or inserting a node before itself) linked it before
  a node it had just unlinked, a cycle in the sibling list. Decided: one insertion path —
  `take_for_insertion` (a fragment's children out of it with one record for the fragment, as step 4
  queues; else the node out of its parent with that parent's removal record, as adopt's unsuppressed
  "remove" does) then `link` (structure and highlight bookkeeping only) and the caller's record:
  `append_child` / `insert_before` one insertion record per node, `replace_child` one record with the
  arrivals and the replaced child. Nothing in rdom-tui special-cased the missing record; the
  teardown now sees moves, so a moved element's transitions and animations are cancelled and restart
  (CSS Animations 1 §4.1: removal cancels, insertion starts afresh — what browsers do on a DOM move).
  `moveBefore()` (the state-preserving move) not added: it needs a record kind a renderer tells from a
  removal plus the "move" algorithm's focus, top-layer and range steps — not cheap; DIVERGENCES §2
  "DOM API shape" says so. Red: `rdom-core/src/tree_move_tests.rs` — 4 of 6 failed (no removal record,
  no fragment record, `replace_child` without the replaced child), the next-sibling replace looped
  forever in `children()` (the before-itself case was added with the fix); rdom-tui `teardown_tests::a_spinner_moved_with_one_append_restarts` width 6
  for 2 (mutation-checked with the removal record turned off); green after. CHANGELOG silent change 78.
- 2026-10-08 — C12G-UPGRADE (API B1, N7; docs only). The upgrade guide's silent-change list re-ranked
  by impact, now 83 items: `appearance: none` / `-webkit-appearance` (dropped in 0.5, now stripping the
  UA's `::before` / `::after` chrome) is item 4 — a reset's `button, select { appearance: none }`
  loses the brackets and the `▾` on every page that uses one — with the web's custom-checkbox
  consequence (no mark; `em` sizes dropped, so an empty 2×2 border box) and the port: cells and a
  `::before` mark. The `display: none` fade-in break moved from 74 to 20, beside the "running
  transition is the computed value" item (19), and batch A's reversed-transition change follows it
  (21); a new item 23 says negative `transition-delay`, `linear()` and `@starting-style` were dropped
  in 0.5 and now apply. Batch A's other silent changes folded in by impact instead of appended: the
  textarea grip (26) and removal / move cancelling animations (27, 28) after the `<select>` overlay,
  the `computed_pseudo` / removed-element reads (38, 39) with the other reader-facing changes after the
  `bg` initial value. API table: `ImportantMask::TRANSITIONS` is the union of five bits
  (`TRANSITION_BEHAVIOR` included); the `TuiStyle` / `ComputedStyle` new-fields row names
  `interpolate_size`, `ui`, `overlay` and the timeline fields, with their items.
- 2026-10-08 — C12G-DIVERGENCES (API B2; docs only). DIVERGENCES §2 "Timers & animations" no longer
  says scroll-driven animations are scheduled (they shipped in C12-SCROLL-DRIVEN); §3 "Transitions and
  animations" carries "(none)" like its siblings. Found while checking the section: its "Animation
  events follow transition events" entry was made false by C12G-MISC, which merged the streams by time
  (transitions first at one time, as Web Animations 1 §4.4's composite order puts them); rewritten to
  the divergence that remains — transitions tied at one time keep their creation order, not tree
  order then property name.
- 2026-10-08 — C12G-COMPUTED-DOCS (API N1; Web Animations 1 §5.4.5, CSS Transitions 1 §3). The docs where
  consumers read — `TuiExt::computed`, `computed_for`, `node.computed()` / `computed_rc()` — now say the
  value holds the running transitions' and CSS animations' values at the last frame (mid-flight a
  `height: 2 → 10` transition reads 6), and point to the style without them. Decided on the name: CSS
  Cascade's "cascaded value" is the pre-computed winning declaration, so the accessor is named for Web
  Animations' *base value* — `TuiExt::cascaded_for` → `base_computed_for` (its doc: "transitions and
  CSS animations", not just transitions), `PresentationStyle::cascaded()` → `base()`, and the node
  handle gains `TuiNodeExt::base_computed()` (the host slot). Both were added after 0.5, so the rename
  is a row in the API table's post-0.5 section, not a Breaking bullet. Red:
  `geometry_transition_tests::base_computed_is_the_style_under_the_running_values` did not compile (no
  `base_computed`); green after — mid-flight `computed` height / width 6 / 6 (a transition and an
  animation), the base 10 / `auto`; after both end the two are one style.
- 2026-10-08 — C12G-API-HYGIENE (API N2–N6). (1) Re-exports: the `transition-*` vocabulary
  (`TransitionProperty`, the new `PropertyName`, `TimingFunction`, `LinearStop`, `StepPosition`,
  `TransitionBehavior`, `TransitionRule`) and `App::get_animations`' `AnimationInfo` / `AnimationKind`
  with `Longhand` are at the `rdom_tui` root. (2) Dead surface — decided: `ActiveAnimation` is
  crate-private rather than `#[non_exhaustive]` with accessors, because no public API ever built or
  returned one (`register` is private; `AnimationInfo` is the inspection record), so accessors would
  have been a second dead surface; `AnimationEventKind` likewise (no public signature); the vestigial
  `effective_fg` / `_bg` / `_border_color` / `_padding` are removed (they only read `computed`, and
  `effective_bg`'s `Reset` fallback contradicted the `TRANSPARENT` initial value). Both classified in
  DESIGN's crate-private records. (3) `TransitionProperty::Named` holds a `PropertyName`, an opaque
  `&'static str` built only by `PropertyName::new` (a dispatch-table name, ASCII case-insensitive):
  `Named("colour")` no longer compiles and `named("colour")` is `Other`, as CSS keeps an unknown
  `<custom-ident>` (CSS Transitions 1 §2.1). (4) `TimingFunction` and `Appearance` are
  `#[non_exhaustive]`, moved to DESIGN's open vocabularies (Easing and CSS Forms keep growing; both are
  evaluated inside rdom-style). (5) Builders: `view_timeline_axis`, `view_timeline_inset`,
  `timeline_scope` added — `overlay` already had one (a `setter!` macro the review's grep missed); the
  `ui_setter!` setters take `impl Into` of their value (`.cursor(CursorKeyword::Pointer)`, the UA
  sheet's `.into()` dropped); the tokenizer's `parse::Cursor` is `parse::SourceCursor` (it is a
  character cursor, and the name collided with the `cursor` property's `layout::Cursor` under a glob);
  `LinearStop::new(output, input)` in CSS's `linear(<output> <input>%)` order, the fields declared in
  that order too. Breaking bullets for the 0.5 items (`TimingFunction`, `TransitionProperty`,
  `parse::Cursor`, `effective_*`, `ActiveAnimation`), post-0.5 rows for the rest; hint group
  `animation_api_hygiene_hints`. Red: rdom-style's four new tests did not compile (no `PropertyName`,
  no `SourceCursor`, no `view_timeline_axis`, `.cursor` refused a `CursorKeyword`); green after. The
  rdom-tui re-export test was written before the re-exports but its red was not run separately.
  Changed test: `timing_tests::linear_stop_builds` pinned the old argument order — replaced by
  `a_linear_stop_is_built_output_first`.
- 2026-10-08 — C12G-APP-CONFIG (API N8). Decided: an option an app sets once, when it builds the `App`,
  is a consuming `with_*` builder; a `&mut self` `set_*` stays only for what an app changes while it
  runs (`set_color_scheme`, a theme toggle, beside `with_color_scheme`; the stylesheet stack and
  `register_property` are actions, not options). Renamed: `tick_rate` → `with_tick_rate`, `on_tick` →
  `with_tick_handler`, `set_animation_frame_rate(&mut self)` → `with_animation_frame_rate` (consuming),
  `set_import_loader(&mut self)` → `with_import_loader` (consuming; the `<style>` sheets are re-parsed
  whenever it is called, so a test that sets it after construction reassigns `app`). The `App` module
  doc's stale "Public API surface" (only `with_sgr_capabilities`; `handle_event` / `draw_if_dirty`
  called pub-crate) is rewritten, with a `## Configuration` table of every option and its `App::new` /
  `App::with_backend` default (tick 50 ms both; frames 60 fps both; caret 530 ms vs steady; SGR from the
  environment vs the backend's; pointer shapes from the environment vs `None`; color scheme asked at
  `run` vs dark; system clipboard and opener both; no import loader), mirrored in the README.
  `with_url_opener`'s doc no longer names a nonexistent `build`. Prose that named `on_tick` says "tick
  handler" (DIVERGENCES' `P7G-TICK-TOUCHED-1` entry and `@import` entry included). Red:
  `config_tests::every_construction_option_is_a_with_builder` did not compile (no `with_tick_rate`);
  green after, with `with_backend_defaults_match_the_table` pinning the table's `with_backend` column.
  Breaking bullet and API rows; hint group `app_config_hints`.
- 2026-10-08 — C12G-README-ANIM (API N10). The rdom-tui README gains "Transitions and animations": a
  doctest on `App::with_backend` + `App::advance` that paints at fixed times — a `width: 2 → 10`
  transition at 50 ms (6 cells, `computed()` 6, `base_computed()` 10) and at 125 ms (10), a
  `pulse 100ms linear 2 alternate` animation at 50 ms (4), 125 ms (5, running back, one
  `animationiteration`) and 225 ms (2, `animationend`). "Porting web patterns that do not carry over"
  has two more doctests — a scroll-progress bar (a named `scroll-timeline` hoisted by `timeline-scope`,
  `width` keyframes; half the range scrolled is 10 of 20 cells) and an `appearance: none` checkbox
  drawn with `::before` (`( ) ` → `(•) ` on click) — and the `em` → cells rule (`1em` square is
  `width: 2; height: 1`). Found while writing the first doctest: `Stylesheet::append` dropped
  `@keyframes`, so a sheet built by `rdom_css::from_css_strict` (which appends its parse) never
  animated — every in-tree test pushed `rdom_css::parse`'s sheet instead. Fixed in rdom-style (each
  rule's layer mapped as the rules' are); red: `layers::tests::append_carries_keyframes` 0 for 1 (and
  the doctest: the pulse 2 for 4, the progress bar 20 for 0, no animation registered); green after.
  The rdom-style README's lists: `calc-size()` on `min-*`, `max-*` and `flex-basis` too, `overlay`,
  a "User interface" line (outline, cursor, caret, accent, appearance, field sizing, resize), the
  easing forms and `@starting-style`, and a "Scroll-driven animations" line. `ua/controls.rs`'s focus
  comment no longer says a TUI cannot draw a ring — it says why the UA still tints (a cell-wide ring
  covers the neighbours; DIVERGENCES `FOCUS-VOCAB-1`).
- 2026-10-08 — C12G-DESIGN-TEST (API N11). `rdom-showcase/tests/integration/design_types.rs` hardened,
  each rule with a self-test: (a) a glob `pub use path::*` in a `lib.rs` is followed — the module's
  file is read (a published crate's path, or the crate's own) and its surface taken recursively; a
  glob inside a `pub use` group fails; (b) macro-defined types: every `macro_rules!` whose body
  defines `struct $…` / `enum $…` must be in the test's `TYPE_MACROS` (today `bitflags_like`, whose
  invocation spells `pub struct DocumentPosition`, read by the source scan with its attributes), so a
  new type-defining macro fails until listed; (c) a type is classified only by an *entry* — a backtick
  span that is exactly its name (`Name`, `Name<T>`) inside a kind's bullet; `Name::Variant`, calls and
  mentions outside the bullets do not count; (d) the kind fixes the attribute — open kinds (web
  vocabularies, error / outcome sets, options bags, `GridTemplate`) ⇔ `#[non_exhaustive]`, closed kinds
  (CSS values and style records, sets fixed by definition) without it, one polarity per type; handles,
  sealed and crate-private bullets are outside the attribute rule, as DESIGN says. Decided against a
  "not inside parentheses" entry rule: DESIGN's bullets group entries in parentheses (`geometry (`Rect`,
  …)`, the grid values), so depth cannot tell an entry from a reason. Red: the hardened check failed
  on five types with no entry (`Dom`, `InputType`, `MouseButton`, `Style`, `TransitionProperty` —
  named only in paths such as `MouseButton::Other` and `render::Style`, or in a call) and, once those were
  listed, would fail on five reason mentions under the opposite kind (the probe found them first:
  `AccentColor` in `SystemColor`'s reason, `Align` / `AlignProperty` / `CounterStyle` / `GridTemplate`
  in other types' reasons); DESIGN fixed — entries added, the reason mentions reworded. Mutation-checked:
  restoring `` `AccentColor` `` in the open bullet fails the attribute rule. DESIGN's rule paragraph and
  CLAUDE.md's checklist line describe the stricter check.
- 2026-10-08 — C12G-CARRYOVER (batch A's "not done"). (1) `steps()` transitions (CSS Easing 1 §2.3, CSS
  Transitions 1 §3): an `ActiveAnimation` remembers the frame that last composited it (`stepped_at`)
  and `next_change` answers `now` for a continuous easing, the end of its delay while in it
  (`transitionstart`, a `jump-start`'s first step), else the next of its `n` equal divisions after the
  last frame — the last one its end (`transitionend`); a boundary passed since is due at once.
  `next_frame` takes the minimum over transitions and CSS animations (registered custom-property
  transitions still every frame). Red: `frame_cost_tests::a_stepped_transition_paints_only_at_its_steps`
  25 paints for 4; green after. (2) `keep_cascaded`'s per-restyle `Rc`: measurable as a full re-diff
  of every element restyled under a running transition — a new base allocation defeats the hook's
  `Rc::ptr_eq` skip. Both write-backs (`keep_cascaded`, which now takes the style by value and
  allocates only when it differs, and `set_cascaded`) keep the old base when the cascade's style is
  equal. Red: `geometry_transition_tests::an_unchanged_restyle_keeps_the_base_style` (a `data-x`
  write mid-transition replaced the base); fixing `keep_cascaded` alone stayed red — the restyle went
  through `set_cascaded` — green with both. (3) Nested `calc-size()` (architect N4; CSS Values 5 §10:
  a basis is the box's size with its content as it is): `layout_pass::calc_size` records each box's
  count of `calc-size()`d ancestors and resolves level by level, innermost first, each level in a pass
  after the one that measured it — `d + 2` passes for nesting `d` deep, 2 without nesting as before;
  TECH_DEBT `ANIM-RELAYOUT-1`'s bound restated. Red:
  `calc_size_tests::a_nested_calc_size_resolves_inside_out` outer 4 for 2; green after. (4) Not done:
  a `::before` / `::after` `view()` timeline still takes its originating element as the subject
  (DIVERGENCES §2, "`scroll(root)` follows the document element"). rdom has no single rect for a
  pseudo-element — an inline run's fragments, a generated block box, a float or a positioned box each
  keep their geometry in a different place — so a subject rect means a pseudo-element bounding-box
  accessor first; recorded here, the divergence entry stands.
- 2026-10-08 — C12G-SPLITS (CLAUDE.md §Architecture Hygiene: 575 split on touch; moves only). The SIZE-1
  recount at the end of batch B found two files batch B took past 575: `runtime/app/mod.rs` 596
  (C12G-APP-CONFIG's configuration table in the module doc) — its `with_*` option builders moved to
  `app/config.rs` (104), leaving 508, the table staying in the `app` module doc; and
  `rdom-core/src/tree.rs` 587 production lines (C12G-MOVE-RECORD's insertion helpers) — now
  `tree/mod.rs` 421 (the public mutations and the insertion path) and `tree/detach.rs` 179 (the
  unlink, the interaction-state purge and the removing steps). No behaviour or public path changed.
  TECH_DEBT `SIZE-1` recounted.
- 2026-10-08 — C12G-DATES (docs only). The Log and the Phases table carried dates the commits do not:
  entries ran to 2026-10-17 while the work was committed 2026-10-04 … 10-08 (the program's first commit
  is 2026-10-04's), and earlier ones drifted a day or more ahead. Every Log entry's date and every
  Phases-table "done" date is now the author date (date part) of the commit that introduced its line:
  `target/claude-logs/fix_dates.py` finds it with `git log --reverse -S` on the line's head as first
  written (its date and opening words, so a tail edited later does not move it), falling back to the
  date-free text for a line whose head changed — 332 lines corrected, none unresolved (Phase 2's row,
  whose tail gained a clause on 10-06, keeps its 10-04). Checked by hand against the items' own
  commits: C1G-README 10-04, C5G-DOCS-AND-SHOWCASE 10-05, C6-FLEX-DIRECTION-INITIAL 10-05,
  C8G-PSEUDO-BOXES 10-06, C10-COUNTERS 10-06, C11-HAS 10-08, C12-CONTROLS 10-08. The Phase 12 row's
  "gates run" date is 10-08. Elsewhere: TECH_DEBT's `SIZE-1` recount date was corrected with C12G-SPLITS'
  recount; CHANGELOG, DIVERGENCES, ACID and the READMEs carry no date past today. Left as written: the
  status line's "started 2026-10-03" (the audit commits of 10-03 that the program grew from) and the
  "HTML Living Standard of 2026-10-07" citations (a spec snapshot's date, not a commit's).
- 2026-10-08 — Phase 12 closed. Both gates' findings are fixed (18 `C12G-*` commits: batch A's seven,
  batch B's ten and the C12G-SPLITS hygiene split) or recorded: the `::before` / `::after` `view()`
  subject (C12G-CARRYOVER, DIVERGENCES §2) and `moveBefore()` (C12G-MOVE-RECORD, DIVERGENCES §2) stay
  documented divergences; TECH_DEBT `MOVE-RECORD-1` is closed. The re-review of the `C12G-*` fixes
  rides with the Phase 13 gate.
- 2026-10-08 — C13-TFC, part 1: the table grid's slot assignment (HTML §4.9.12.1 "forming a table",
  the algorithm for processing rows; CSS Tables 3 §3.3), as pure DOM-free logic in rdom-core
  (`rdom_core::table`): `assign_slots` takes row groups of rows of `CellSpan`s and places each cell in
  the first slot of its row no earlier rowspan covers, the grid as wide as its widest row. Decided:
  a rowspan ends at its row group's last row (CSS Tables 3 §3.3 and every browser; HTML's algorithm
  adds empty rows, which nothing renders), `rowspan="0"` spans to it. `CellSpan::from_attributes` and
  `column_span` read `colspan` / `rowspan` / `span` by HTML §2.3.4.2's rules for parsing non-negative
  integers (`" +4px"` is 4) and §4.9.11's clamps (1000, 65534). rdom-core owns it because both
  consumers need one placement: C13-TFC's table formatting context and C13-COLUMN's column
  combinator, which rdom-core matches. DESIGN: the three records are closed (geometry). Red:
  `table/tests.rs` 8 of 8 failed against a stub; green after. Mutation: rowspans not clamped to the
  group → 1 fails.
- 2026-10-08 — C13-TFC, part 2: the data model. rdom-style: `display: table | inline-table` (`Flow::Table`,
  the inner type, with `block` / `inline` as the outer one — `block table`, `table inline` parse too) and
  CSS Display 3 §2.4's layout-internal keywords as `Display::TablePart(TablePart)` — standalone keywords,
  a cell or caption a `flow-root` block container inside, the others `flow` (`TablePart::is_block_container`);
  `table` takes no `list-item` (§2.3). Decided: one variant holding the part rather than eight `Display`
  variants — every reader that is not the table layout treats them alike (block-level in a block flow,
  blockified as an item). `table-layout: auto | fixed` (not inherited) and `caption-side: top | bottom`
  (inherited; `block-start` / `block-end` parse as the same two — CSS Tables 3 makes `top` / `bottom` the
  table's block-start / block-end sides; `inline-start` / `inline-end`, which no browser ships, are
  invalid) in a new style group, `TableStyle` / `TableDeclarations` (`ComputedStyle::table`,
  `TuiStyle::table`), dispatched by `property_dispatch/tables.rs`, discrete in the animation table, with
  builders and `!important` bits. rdom-tui: blockification (CSS Display 3 §2.7) makes a table part a
  `block flow` box and `inline-table` a `table` — and, by CSS 2.1 §9.7, a floated or absolutely positioned
  table part; `layout_differs` reads the table group and `border-spacing` (both move boxes now); a table is a
  formatting context root. Interim, until part 3: a table lays out as a
  `flow-root` and a table part as a block box. Red: `table_tests.rs` (rdom-style) failed to compile
  (no `TablePart` / `TableLayout` / `CaptionSide`); `display_tests` listed `table` as invalid (expectation
  removed: it is valid now); `css_phase13/display.rs` 2 of 2 failed with the blockification arm a no-op
  (`TablePart(Cell)` / `FlowRoot` kept in a flex container and on a float); green after. Breaking
  (rdom-style): the two variants and the `table` field — CHANGELOG, the API table rows, migration hints
  `table_display_hints`; DESIGN lists the new types (closed).
- 2026-10-08 — C13-TFC, part 3: the table formatting context, `layout_pass/table/` (CSS 2.1 §17 with CSS
  Tables 3 where it is more precise), for tables built from `display` values. Module tree: `structure`
  (§17.2.1's fixup over `box_tree::item_sequence` — captions; columns from `table-column` /
  `-column-group` boxes, `<col span>` / `<colgroup span>` read for those elements; row groups in display
  order, the first header group first and the first footer group last (§17.2); anonymous rows around runs
  of cells and other content in a table or row group, anonymous cells around runs of non-cells in a row;
  white space alone between proper children, `display: none` and out-of-flow boxes dropped), `grid`
  (rdom-core's `assign_slots`; `colspan` / `rowspan` on `<td>` / `<th>` only — CSS has no span
  property; `visibility: collapse` rows and columns, §17.5.5), `lines` (the separated model's
  `border-spacing` before each column and after the last and between rows, the collapsing model's
  shared lines one cell wide wherever a table, group, row, column or cell border lies on them; a
  collapsed track's two lines merged), `columns` (CSS Tables 3's cell measures — outer min-content
  `max(min-width, min-content)`, outer max-content with a length `max(min-width, width, min-content)`
  — less the borders on lines in the collapsing model; column boxes' widths as floors; spanning cells
  fewest columns first, their shortfall spread by growth room, then max-content, then equally;
  percentages), `width` (the automatic algorithm's four guesses — min-content, percent,
  constrained, max-content — interpolated, the excess past max-content to unconstrained columns, then
  constrained, then percent; the fixed algorithm from column boxes and the first row, the rest
  shared equally, when `table-layout: fixed` and the width is not `auto`), `rows` (the tallest cell
  of each row, at least the row's `height`; rowspans' excess by row heights; a taller table box's
  extra over the rows, §17.5.3), `place` (captions above / below by `caption-side`, the table box
  between them; groups, rows, columns and cells at their areas — a box covers a collapsed line when it
  has a border on it — each cell laid out with `layout_node`, an anonymous cell's inline runs as
  `AnonymousIfc`s on the box whose children they are), `anonymous` (an anonymous cell's inline runs
  packed, its block children stacked). Decided: one `solve(width)` serves the layout and the table's
  intrinsic height, and `content_size` the min- / max-content widths without distributing (MIN / MAX
  and CAPMIN, §17.5.2.2); every cell is measured through `intrinsic`, so the per-pass memo serves the
  repeats — the counting test (`table/cost_tests.rs`) pins ≤ 3 solves a pass, ≤ 2 Row walks and ≤ 1
  Column walk a cell, at 3 rows and at 20. The element's `layout` is the table wrapper box (captions
  included); its border and background paint on the table box (`table::table_box`: the content box
  grown by the chrome, read by `paint_pass::box_paint`), no stored field (the `TuiExt` size bound).
  The collapsing model has no table padding (§17.6.2). Interactions: `children_layout` gives tables
  their own arm (before the IFC test — a table of inline children is no IFC), `is_ifc_block` is false
  for tables and for rows / groups / columns; a cell's height is its rows' (`auto_height` leaves it);
  a block-level `auto`-width table is shrink-to-fit and never narrower than its min-content
  (`block::width`); an `inline-table` atom's baseline is its first row's (`inline::vertical`, §17.5.3),
  `baselines::content_rows` asks the table; column boxes are no hit-test targets. DIVERGENCES §2: the
  whole-cell entry for the table formatting context (lines, spanning distribution, caption margins,
  anonymous cells' block content, cells top-aligned until C13-TABLE-PROPS); §3 lists what remains.
  Red: `css_phase13/tfc.rs` 14 of 15 failed against part 2 (tables laid out as flow-roots: cells
  stacked, no columns, no spacing, no captions; `cells_are_hit_and_columns_are_not` passed there and pins
  the hit rule), the cost test did not exist; green after, with `display.rs`'s two. Two expectations
  corrected while writing (the reasoning, not the code, was wrong): anonymous cells are cells of the
  grid, so `loose` widens the column `a` sits in; and an inline table first sat on its last row — the
  atom baseline was an inline block's, fixed in `vertical::atom_rows`. Mutation (each alone, restored,
  touched): the inline table on its last baseline → 1 fails; no spanning spread → 1; the header group
  kept in place → 1; no collapsed line marked → 1; each cell's height walked twice → the cost test.
  No existing test expectation or snapshot changed.
- 2026-10-08 — C13-TFC, part 4: HTML tables on the table formatting context. The UA sheet gives the table
  elements HTML §15.3.8's `display` values (`table`, `table-caption`, `table-column(-group)`,
  `table-header-group` / `-row-group` / `-footer-group`, `table-row`, `table-cell`); `<tr>`'s flex row
  and its `height: 1` and `colgroup` / `col`'s `display: none` are gone; cells keep their one-cell inline
  padding and `<th>` its bold; HTML's `border-spacing: 2px` is not taken (no whole cell; the initial 0,
  DIVERGENCES §2). The flex-row model's pieces are deleted: `runtime::builtins::table` (`size_all_tables`,
  `size_columns`), its call in `App::build`, `TuiExt::table_used_width` and its two readers (flex's main
  size, the intrinsic contribution), `tree::is_collapsed_table_row` (a collapsed row is the table's
  business now, §17.5.5). This closes the `TABLE-TFC-1` divergence (STABILIZE-2026-09) and DIVERGENCES §2's
  "tables are flex rows" entry; C4-SPACING's layout part is done (separated borders space HTML tables
  too) and C6-VISIBILITY's `<col>` note with it. Found while migrating: (1) the document root's children
  are items of rdom's viewport column, which stretches them — a browser's `<body>` would not stretch a
  table (CSS 2.1 §17.5.2.2): `flex::cross::hugs_as_inline_level` now also keeps a root-level table at its
  content width, as it keeps an inline block; (2) copy recognised table parts by tag — HTML §3.2.7 reads
  the used `display` — so a `display: table-cell` copied no tab and a `<tr>`, no longer `block`, would
  have lost its line break: `clipboard::serialize` keys the tab on `table-cell` and the line breaks on
  `table-row` / `table-caption` / block-level (DIVERGENCES' clipboard entry loses its tag departure).
  Red: `css_phase13/html.rs` 5 of 5 failed against part 3 (`table` computed `block flow`, a `<col>`'s width
  ignored — 3 for 6, no spacing — `(0, 0)` for `(2, 1)`, a wrapped cell's row one high, `<tfoot>` first and
  `rowspan` ignored); `rendered_text_tests::css_table_cells_and_rows_copy_by_their_display` copied `abcd`;
  green after. Changed expectations: `ua_colgroup_and_col_are_none` → `ua_colgroup_and_col_are_table_columns`
  (HTML gives them their table-column display; `none` was the flex-row model's); `layout_pass::tests::
  table_in_a_horizontal_scroll_wrapper_…` cascades the UA sheet — with `Stylesheet::bare()` its `<table>`
  is nested blocks, and only the deleted builtin's stamped widths made it overflow; the stale-glyph app
  test gives its flex cells inline widths instead of stamping `table_used_width`; the C6 visibility test
  and `table_column_sync_makes_cells_align_across_rows` drop the builtin call — same assertions. CHANGELOG:
  the Breaking bullet and API row (`table_layout_hints`), and silent change 8 (HTML tables lay out as CSS
  tables: a shrink-to-fit width, rows as tall as their cells, rowspan / col / tfoot / caption placement).
  No showcase demo has a table; no snapshot changed. Mutation (each alone, restored, touched): a root-level
  table stretched → the C6 collapsed-row test fails (its cell 20 wide); rows without line breaks in copy → 2.
- 2026-10-08 — C13-TFC, part 5 (item done): the anonymous table around table parts outside a table (CSS 2.1
  §17.2.1 rule 3). Block flow partitions a run of them as a `RunKind::Table` (new; `RunKind::is_block_level`
  is true for it and `Block`, so it ends inline runs and a block-level edge pseudo still gets its own line
  box), white space alone between two such runs joining them (rule 1.4: no box; `runs::merge_table_runs`);
  `FlowSink::table_run` lays the run out (`table::layout_anonymous`) or measures it
  (`table::anonymous_height`) as a table whose style is an anonymous box's that is a `table` — `auto`
  width, shrink-to-fit in the containing block (§17.5.2.2), no margins. The table module takes a
  `TableBox` (an element, or `Anonymous { parent, items }`) through `Structure::of`, `solve`, `place` (which
  writes no rects for a box with no node). Intrinsic sizes: a block container's inline size counts each
  table run as its anonymous table (`intrinsic::children`, and `float::measure` for flows with floats),
  the flow partitioned only when a child is a table part (the idle-cost pin `block_inline_sizes_partition_
  floats_only_when_one_floats` caught the unconditional first try: 16 partitions for 0). Decided, and in
  DIVERGENCES §2: parts inside an inline box get no anonymous `inline-table` — rdom does not split inline
  boxes around block-level ones, and these follow the same packing. Red: three `css_phase13/tfc.rs` tests
  — stray cells stacked as blocks (`["before", "a", "bb"]` for `["before", "abbc", "after"]`), stray rows
  not sharing columns (`["a", "bbb"]` for `["a bbb", "ccd"]`), an inline block around two stray cells 2
  wide for 4 (`"ab"` for `"abcd|"`); green after. Mutation (each alone, restored, touched): white space not
  joining table runs → 1 fails; the anonymous tables left out of the inline size → 1. C13-TFC is done:
  CSS-COVERAGE's `display: table` row Supported (§3.7 9 / 0 / 0 / 2; total 234 / 7 / 21 / 45);
  DIVERGENCES §3's display list is empty.
- 2026-10-08 — C13-TABLE-PROPS (CSS 2.1 §17.5.3, §17.6.1.1). `caption-side` and `border-spacing` shipped with
  C13-TFC (parts 2–4); `table-layout` too. New: (1) `vertical-align` on cells — `table/align.rs`: `top`,
  `middle` (free rows halved, rounded down: the upper middle), `bottom`, and `baseline` for every other value
  (§17.5.3); a cell's baseline is its first line box's text row (`baselines::content_rows`, C9G-ONE-BASELINE's
  model) or the bottom of its content edge, counted from its track's top (a collapsed top border on the line
  above taken off); a row's baseline is the lowest of its `baseline` cells', and `rows::heights` grows a row
  to hold each shifted cell; `place` moves a laid-out cell's content down by the offset
  (`tree::shift_cell_content`: its lines when it packs its own, its children and anonymous boxes) and an
  anonymous cell's lines with it; the table's baselines (an `inline-table`'s, a table flex item's) are its
  rows' (a row with no `baseline` cell: its last row). HTML §15.3.8's UA rules: `thead`, `tbody`, `tfoot`,
  `tr { vertical-align: middle }`, `td, th { vertical-align: inherit }` — an HTML cell is centred in a taller
  row (CHANGELOG silent change 8 gains it). (2) `empty-cells: show | hide`, inherited (`EmptyCells`, the
  table group's third field, dispatched, discrete, builders, `!important` bit, inherited-set probes):
  `table::hides_empty_cell` — a cell with no in-flow box, no text but white space and no `::before` /
  `::after` in a separated table (its table's `border-collapse`; an anonymous table's always) — and
  `paint_pass::box_paint` skips its shadows, background and border. DESIGN lists `EmptyCells` (closed).
  Red: `css_phase13/props.rs` 4 of 4 failed (every cell top-aligned: `["Tamb", "", ""]`; `[" BC", "", "A"]`;
  the HTML cell at row 0; the empty cell's box drawn), `table_tests.rs`'s `empty-cells` test failed to
  compile; green after. Mutation (each alone, restored, touched): `middle` at the top → 2 fail; the
  baseline shift not growing the row → 1 (after the test gained a two-line cell — the first version's row was
  already tall enough and the mutation survived); the UA `inherit` gone → 1; the empty-cell check off → 1.
  Also pinned (no red: it already worked): `tfc.rs::points_over_cells_resolve_to_their_text` — `position_at`
  over a cell's text and an anonymous cell's. Split (SIZE-1): `table/mod.rs` would have reached 513 — the
  anonymous table's entry points moved to `table/stray.rs` (moves only), leaving 421.
- 2026-10-08 — C13-COLUMN (Selectors 4 §16.1–§16.3; was C11-COLUMN). rdom-core: `Combinator::Column` (`||`, white
  space optional) and `SimpleSelector::NthColumn(NthColumnSelector)` (`:nth-col(An+B)` / `:nth-last-col(An+B)`,
  no `of S`; a pseudo-class's specificity, `(0, 1, 0)`). The column model is HTML's, from the DOM alone —
  `table/html.rs` (crate-private): the `<colgroup>`s before the first row group or row make the columns (a
  `<col>` its `span`, an empty `<colgroup>` its own; a `<col>` outside a `<colgroup>` and a later
  `<colgroup>` none, as HTML §4.9.12.1 forms them), the rows are the table's `<tr>`s and its `<thead>` /
  `<tbody>`s' in tree order then its `<tfoot>`s', the cells placed by C13-TFC part 1's `assign_slots` — the
  same placement the table formatting context uses. Matching (`query_selector/column.rs`): a cell's column
  elements are each `<col>` / `<colgroup>` whose columns overlap the cell's; `match_chain` tries them in turn
  for a `||` step (no tree direction: a failure lets the next subject on the right try,
  `RestartFromClosestLaterSibling`); `:nth-col()` matches when any column the cell spans is `An+B` from the
  first (from the last: the table's width minus the column). The model is formed once per table per pass and
  kept in `SelectorCaches` (`CacheWork::table_models` counts it). Decided: (1) a `<colgroup>` represents its
  columns, so `colgroup.g || td` matches the cells of its columns — Selectors 4 speaks of "column elements"
  and HTML's column groups are made of columns; (2) column membership is the document language's (§16.1), so
  only HTML's `<td>` / `<th>` of a `<table>` belong to columns — a CSS `display: table-cell` does not, and
  neither does a `<td>` outside a table; this is the spec, not a divergence. rdom-tui: the dirty tracker
  restyles the whole `<table>` (`marks::mark_column_change`, walking up through the table elements) on a
  `span` / `colspan` / `rowspan` change or a child-list change of the table, a row group, a row or a
  `<colgroup>` — only while the sheets hold a column selector (`uses_column_selectors`, set by the App's
  prelude with the sibling and `:has()` triggers; true until it says otherwise); `selector_walk`,
  `has_triggers` and `sibling_triggers` read `:nth-col()` as reading the three span attributes. Red:
  `column_tests.rs` (rdom-core) failed to compile (no `Combinator::Column` / `NthColumn`);
  `css_phase13/columns.rs::changing_the_columns_restyles_the_cells` failed before the tracker change (a `span`
  on the first `<col>` left `b` and `d` red); green after, with the tracker's two unit tests. Mutation (each
  alone, restored, touched): no column elements for a cell → 1 fails; the model not cached → the cost test
  (150 models for 1); `:nth-last-col()` counted from the first → 2. DESIGN lists `NthColumnSelector` (open,
  a parser output). CSS-COVERAGE §3.17's column-combinator row Supported (33 / 0 / 1 / 4).
- 2026-10-08 — Phase 13 items done (C13-TFC in five parts, C13-TABLE-PROPS, C13-COLUMN); status line "items
  done, gates pending". ACID tile 14 covers the new table behaviour (row spans, separated borders and
  `border-spacing`, `empty-cells`, cell `vertical-align`, captions, the column selectors). TECH_DEBT `SIZE-1`
  recounted for the files Phase 13 touched. CSS-COVERAGE: §3.20 4 / 0 / 0 / 0, §3.17 33 / 0 / 1 / 4, §3.7 9 / 0 /
  0 / 2, §3.5 14 / 0 / 0 / 2; total 237 / 7 / 18 / 45.
- 2026-10-08 — Phase 13 gates (with the C12G re-review: all 18 hold; the 2-layouts-per-frame bound is
  real; `Stylesheet::append` dropped only `@keyframes`; slot assignment follows HTML §4.9.12.1 with
  HTML's span caps). Architect: 2 blocking — the `||` candidate loop stops at the first candidate
  that reports "no match anywhere" (a cell spanning two colgroups is missed); hostile column spans
  cost quadratic time (each column box's range found by scanning all columns; ~8·10⁸ comparisons for
  20 `<colgroup span=1000>` collapsed). API: 3 blocking — `<th>` / `<caption>` not centred (HTML
  §15.3.8, undocumented); the UA `table` lacks `box-sizing: border-box`, so the documented
  `width: 100%` migration overflows a bordered table; no public read of a table's used column widths
  (`table_used_width` removed, nothing like `grid_tracks()`). Non-blocking: paint-time table geometry
  disagrees with the scrollport / clip / resizer / hit test (captions clipped, scrollbars beside them);
  column invalidation too broad (any `<td>` text change) and missing under `:has()`; duplicated span
  logic with different tag case-sensitivity, `CellSpan` public fields skip the clamp,
  `columns: usize::MAX` overflows; spec gaps not in DIVERGENCES (fixed layout extra width, percentages
  over 100%, spanning baseline cells, a part misparented in an anonymous cell, "Three departures" lists
  four); per-pass table cost (structure rebuilt 4–5×, anonymous cells re-packed ~6×, quadratic group
  lookups; only solves pinned, at 20 rows); nested `calc-size` pays d+2 passes on every layout while
  authored; teardown O(roots × animations × depth); `after([fragment,x])` / `replace_with([self])` /
  `replace_children` corruption-shaped bugs; BEFORE-CHANGE nuance not in DIVERGENCES; the DESIGN type
  check ignores `pub mod` paths; `append` should destructure exhaustively; a bare `<col>` ignored by
  `||`; silent change #8 under-ranked with an incomplete migration (box-sizing, spread width, wrapping,
  cell width as minimum, no cell margins / gap / flex); UA `tr` sets `vertical-align` instead of
  inheriting, `table{text-indent:initial}` missing; missing tests (zebra, `width:100%`, `th`, wrapper);
  stale docs; a README data-table recipe. Design decision: make the document root a block container
  (`flow-root`) with a definite viewport height instead of a flex column — it removes four documented
  root special cases (additive margins, no floats, percentage-height definiteness, the no-stretch
  exception); breaking, so its own item C13-ROOT-BLOCK. rdom-virtualtable (0.3.14, read-only): compile
  breaks on `size_columns` and `table_used_width`; header widths become minimums (use
  `table-layout: fixed`); cell-margin pinning stops. Full reports:
  `target/claude-logs/c13_gate_{architect,api}.md`. Fix as `C13G-*`, three batches (A correctness and
  cost, B API / UA / docs, C the root block container).
- 2026-10-08 — C13G-COLUMN-MATCH (architect B1, N3; API N1; Selectors 4 §3.1, §16.1, HTML §4.9.11,
  §4.9.12.1, §13.2.6.4.9). Found: the `||` loop returned on a candidate's `NotMatchedGlobally`, but the
  candidates share no ancestor chain (columns of two `<colgroup>`s are cousins), so a cell spanning a
  plain group and `.hl` missed `.hl col || td`; a `<col>` child of the `<table>` (an HTML parser's
  implied `<colgroup>`, which rdom-parser does not insert) was a column to layout but not to `||`; the
  span readers were two — layout's exact-case `tag_name() == "td"`, the model's ASCII
  case-insensitive one — so a `create_element("TD")` cell spanned one column in layout and two for
  `:nth-col()`; `CellSpan`'s public fields skipped the clamp. Decided: (1) the `||` loop returns on
  `Matched` only and otherwise tries the next column element; (2) the model reads leading bare
  `<col>`s as the implied group's columns, in the shared model, not the parser; a `<col>` after the
  first row stays none (HTML forms columns from the groups before the rows only); (3) one reader:
  `rdom_core::table::{cell_span_of, column_span_of}` (HTML table elements ASCII case-insensitively —
  DIVERGENCES' case-sensitive-names entry names the table model as its exception), used by
  `table::html` and by rdom-tui's `grid` / `structure`; (4) `CellSpan`'s fields are private, read
  through `columns()` / `rows()` — every value made is within 1000 / 65534, so `assign_slots` cannot
  overflow (`CellSpan` and `table` are new since 0.5: no break). Red: `column_tests.rs`
  `a_cell_spanning_two_column_groups_matches_through_either` (`["right"]` for `["wide", "right"]`),
  `a_bare_col_before_the_rows_is_a_column` (`[]` for `["a"]`); `css_phase13/tfc.rs`
  `an_upper_case_td_spans_its_colspan` (width 9 for 10, run against the old `grid` / `structure`);
  green after, with `table/tests.rs`' `spans_are_read_from_html_elements_only` and
  `spans_are_clamped_however_made` and the upper-case column-model pin. Mutation: each red above is
  the old code path restored alone.
- 2026-10-08 — C13G-SPAN-COST (architect B2; CSS 2.1 §17.6.2, §17.5.1, HTML §4.9.3, §4.9.11). Found: the
  collapsing model's border marks (`lines.rs`) and the column boxes' rects (`place.rs`) found each box's
  columns by `position` / `rposition` over every column, for every column — quadratic in the columns, and
  a column count is ~1000× its attributes. Decided: (1) `Structure::column_boxes` holds each box with its
  range `start..end`, recorded as `push_column*` makes the columns (a group's patched after its `<col>`s);
  the marks and the rects read it, one visit per box; (2) the grid stops at `MAX_COLUMNS` = 65 535: HTML
  caps each span at 1000 but not their sum, and a terminal cell offset is a `u16`, so a column past it
  could never show — `Structure` makes no column past it, `Grid` cuts a cell reaching past it and keeps a
  cell starting past it in `Grid::beyond`, which `place` gives an empty rect (as a collapsed column's
  cell). DIVERGENCES §2's table entry says so. Red: `cost_tests.rs`
  `hostile_column_spans_cost_linear_time` — 1 520 540 040 column scans for 20 000 columns (20
  `<colgroup span=1000>`, collapsed, one layout; 27 s in a debug build); `the_grid_is_capped_at_u16_max_columns`
  — 70 000 columns for 65 535. Green after: ≤ 4 scans a column box, the grid 65 535 wide, the 67th
  `colspan=1000` cell empty. Mutation: the red runs are the old lookups and the uncapped grid.
- 2026-10-08 — C13G-TABLE-COST (architect N5; CSS 2.1 §17.5). Found: each of a table's questions in a
  pass — min- and max-content width (`size_of`, `min_width`), height at a width (the parent's measure),
  baselines, layout — rebuilt `Structure` / `Grid` / `Lines` and the column measures (5 builds a pass);
  an anonymous cell packed its runs for each of its min, max, height and baseline asks, per solve (19
  packs a cell); `lines.rs` and `place.rs` found each row group's rows by scanning every row (quadratic
  for a `<tbody>` per row). Decided: (1) `table/memo.rs` — a `TableMemo` in the pass's document data
  beside the intrinsic memo (`intrinsic::with_tables`): a table's `Skeleton` (structure, grid, lines,
  model and its column measures in a `OnceCell`) once a pass, keyed by its element or, for an anonymous
  table, its parent and first item; its `Sizes` (column widths, row heights, baselines) once per
  `(width, cb)`; an anonymous cell's min / max width and its height-and-baseline at a width (one pack
  gives both) once per question, keyed by container and first item (an anonymous cell has no node for
  the intrinsic memo). Pure for the reason the intrinsic memo is: styles and tree do not change while a
  pass runs; outside a pass nothing is kept. The memo is borrowed only to read or write, never while
  measuring (a cell may hold a nested table). `Solved` holds the `Rc<Skeleton>`. (2) `Grid::groups` lists
  each row group box with its rows `start..end` as the rows are listed; `GridRow::group` is gone. Red:
  `cost_tests.rs` `a_long_table_costs_linear_work_a_pass` — at 10 rows 5 structures, 650 group scans,
  190 anonymous packs; `a_long_table_allocates_a_bounded_amount_per_row` — 500 allocations a row (pin
  unset). Green after: 1 structure, 1 solve, 2 group scans a row, 4 packs an anonymous cell, 2 Row and 1
  Column walks a cell, at 10 and 1000 rows; 212 allocations a row at 100 and at 1000 rows, pinned at
  230. Mutation (restored, touched): the pass memo off (`with_tables` → `None`) fails both tests (5
  structures, 190 packs, 500 allocations a row); the old group lookups were the red group count.
- 2026-10-08 — C13G-TABLE-GEOMETRY (architect N1; CSS 2.1 §17.4: `overflow` and the border apply to the table
  box, not the wrapper around it and its captions). Found: paint recomputed the table box at paint time
  (`table::table_box`, the content box grown by the chrome), while the scrollport, the clip edges, the
  resizer, the scrollable extent and the hit test took `TuiExt::layout`, the wrapper — so a clipping table
  clipped its caption's first row behind the border inset and ran its scrollbars down beside the captions.
  Decided: (1) one source — layout keeps the table box as `TableInsets { above, below }` (the captions'
  rows, relative to the wrapper, so a subtree shift moves it) in a side record that replaces
  `TuiExt::grid_lines`: `kept: Option<Box<KeptLayout>>`, `KeptLayout::{Grid(GridLines), Table(TableInsets)}` —
  a box is a grid or a table, never both, so `TuiExt` stays 384 bytes and a box that is neither pays the
  pointer it already paid; set only for a table with captions. `TuiExt::border_box()` answers the table
  box for a table and `layout` for every other box; the box paint, `scrollport_of`, `ClipEdges::of`,
  the resizer and the positioned-overflow origin read it; `table::table_box` is gone. (2) Captions keep
  their table's own clip: `stacking::child_clip(parent, child, content, outer)` gives a caption of a
  table with captions `outer`, the clip the table paints in, and every other child `content` — asked by
  each walk (the paint-unit walk, the layer collection, the content recursion, the hit-test descent,
  each now carrying the parent's own clip beside its content clip), and the hit test descends into such a
  table where the point is within its own clip (`reach`); the scrollable extent leaves captions out
  (`stacking::outside_content_clip`). The wrapper stays the element's `layout_rect()` and its hit region
  (the table element generates the wrapper box). Red: `css_phase13/tfc.rs`
  `a_clipping_table_clips_its_table_box_not_its_captions` — row 0 `""` for `"Caption"` (`overflow: hidden`
  + border + caption); green after, with the `overflow-y: scroll` bar in the table box's rows and the
  caption hit at (2, 0). Mutation (each alone, restored, touched): `child_clip` never giving `outer` → the
  caption row blank again; the hit test's `reach` off → the caption point hits the table.
- 2026-10-08 — C13G-COLUMN-INVALIDATION (architect N2; Selectors 4 §16.1, §4.5; HTML §4.9.12.1). Found: a
  child-list change called `mark_column_change` from its parent whatever it was, and the climb passed
  through `td` / `th` / `col` — so any text or element appended inside a cell restyled its whole table
  while a sheet held a column selector (a live clock in one cell of 1000 rows re-cascaded every cell);
  and `has_triggers` read `:nth-col()` as reading the span attributes but not `||`, so
  `body:has(col.hl || td.x)` missed a `colspan` change. Decided: (1) `marks::moves_columns` — a
  child-list change moves columns only when its parent is a `<table>`, row group, `<tr>` or `<colgroup>`
  and an *element* came or went (rows, cells, columns; text and a cell's content never); the attribute
  path is unchanged (`span` / `colspan` / `rowspan` anywhere in the structure); the tag tests are ASCII
  case-insensitive, as the table model's (C13G-COLUMN-MATCH); (2) a complex selector with a `||` step
  in a `:has()` argument adds the three span attributes to the triggers. Red: `dirty_tracker/tests.rs`
  `only_structure_changes_restyle_the_table_for_columns` (failed at "text in a cell"),
  `has_triggers::tests::a_column_combinator_in_has_reads_the_spans` (failed at `span`); green after.
  Mutation: the reds are the old paths.
- 2026-10-08 — C13G-SPEC-GAPS (architect N4; CSS 2.1 §17.2.1 rule 3.2, §17.5.2.1, §17.5.3; CSS Tables 3
  "intrinsic percentage width of a column", "distributing excess width to columns"). All implemented,
  none left as a divergence: (1) fixed layout with every column fixed and the table wider — "the extra
  space should be distributed over the columns" — gives the extra to them by their widths, equally when
  all are 0 (`width::spread_extra`; the automatic algorithm's excess loop and it now share
  `width::spread_by`); (2) `columns::clamp_percentages` — each column's percentage at most what the
  columns before it leave of 100%, one left with nothing no percent column; (3) past every guess, the
  excess goes to the unconstrained columns by max-content, else the constrained ones, else the percent
  ones *by their percentages* (was by max-content), else every column; (4) `rows::heights` adds a
  `baseline` cell's shift down to its first row's baseline to its height whether it spans rows or not
  (the spanning one's need was its bare height, so `align::offset` capped its shift and it sat above the
  baseline); (5) an anonymous cell's run of table parts other than cells (rows, row groups, columns,
  captions, white space between them dropped) is a `Segment::Table`, measured and laid out as the
  anonymous table around them (`stray::anonymous_width` / `anonymous_height` / `layout_anonymous`, keyed
  in the pass memo by the cell's container and first part) — each row was laid out as a block of its own
  whose stray cells got a table each, so two rows inside a row did not share columns. DIVERGENCES' table
  entry said "Three departures" and listed four: now "Four departures". Red (`css_phase13/gaps.rs`, new):
  `fixed_layout_spreads_the_extra_width_over_fixed_columns` ([5, 15] for [10, 30]),
  `percentages_past_100_are_clamped_in_column_order` ([10, 10, 1] for [16, 4, 1]),
  `excess_width_goes_to_percent_columns_by_percentage` ([13, 17] for [10, 20]),
  `a_rowspanning_baseline_cell_gets_the_rows_it_needs` (x at row 1 for 2),
  `rows_inside_a_row_share_an_anonymous_table` (`["ab", "cccd"]` for `["a  b", "cccd"]`); green after.
  Mutation: each red is the old path.
- 2026-10-08 — C13G-CALC-SIZE-AUTHORED (architect N6; CSS Values 5 §10). Found: `calc_size::lay_out` ran a
  whole-tree pass per nesting level whenever the document flag said a `calc-size()` was authored — Chrome's
  documented accordion, `details[open]::details-content { height: calc-size(auto, size) }`, five deep, laid
  out 6 times on every layout, at rest. Decided: `resolved` returns `None` when every `calc-size()` length of
  a box equals the size the last pass laid it out at (an identity sum); such a box keeps its basis style
  and layout, and a level with no box changing size takes no pass. The passes are paid only on layouts
  where a sum moves a size — a running `interpolate-size` transition, or a factor / offset other than the
  identity — so the "frames where a basis could change" are exactly those. Behaviour: the box keeps its
  basis's layout, so percentages inside it resolve as against the basis (an `auto` height, indefinite)
  where the old extra pass had made them definite — DIVERGENCES' `calc-size()` entry says so. TECH_DEBT
  `ANIM-RELAYOUT-1` restated: the `d + 2` term applies only while a sum moves a size. Red:
  `runtime/app/calc_size_tests.rs` `an_authored_identity_calc_size_costs_no_extra_pass` — 6 layout rounds
  for 1; green after, the summaries at rows 0, 2, 4, 6, 8 both before and after (checked against the old
  code with the layout assertion first); `a_nested_calc_size_resolves_inside_out` (factor 0.5) still
  resolves inside out. Mutation: the old `lay_out` restored alone gives the red count.
- 2026-10-08 — C13G-TEARDOWN-COST (architect N7; CSS Animations 1 §4.1, DOM §4.2.3). Found: `detach` ran,
  for each removed root, `nodes()` (collect, sort, dedup) and a `contains` climb per running element —
  O(roots × entries × depth); since C12G-MOVE-RECORD a sort moves every row, so a 1000-row list with a
  spinner per row paid ~5·10⁵ climbs in one frame. Decided: the removed roots go in a `HashSet` once;
  each running element (`nodes()` once) climbs from its subject (a `::details-content` box from its
  `<details>`) to the root, cancelled at the first removed ancestor — O(entries × depth) whatever the
  number of roots; each root's subtree is then forgotten once (in `roots` order, duplicates skipped).
  Red: `app/teardown_tests.rs` `sorting_a_list_of_spinners_tears_down_in_linear_time` — 5050 steps at 100
  rows (bound 8 a row); green after at 100 and 1000 rows, the C12G-DETACHED tests unchanged. Mutation:
  the old loop is the red count.
- 2026-10-08 — C13G-DOM-CONVENIENCE (architect N8; DOM §4.2.6, §4.2.3 "ensure pre-insertion validity").
  Found: the variadic helpers inserted one item at a time — `after([fragment, x])` walked its cursor onto
  the emptied, parentless fragment and failed with `HierarchyRequest` after a partial insert;
  `replace_with([self])` inserted self before self, then removed it; `replace_children` cleared before an
  invalid node failed; `prepend([first child, x])` put `x` first; any invalid node after a valid one left
  a partial insert. Audit, step by step: `append` (convert, append), `prepend` (convert, before the first
  child *after* conversion), `before` (the viable previous sibling — first preceding sibling not in the
  list — then its next sibling, or the first child), `after` (the viable next sibling), `replaceWith`
  (the viable next sibling; replace when the receiver is not in the list, else pre-insert),
  `replaceChildren` (validity first, then replace all). Decided: `accessor/convert.rs` keeps the spec's
  result without its fragment — `check_insertable` validates every node first (exists, not an inclusive
  ancestor of the parent), `first_not_in` finds the viable reference among the siblings the list leaves in
  place, `insert_items` inserts in list order before it (each moved as it goes; the spec moves them all
  into a fragment first — the same tree, and no temporary fragment's records reach the observers, which
  would have counted fresh nodes as detached, C12G-DETACHED). `Dom::validate_insert` is `pub(crate)`.
  Red: `accessor/convert_tests.rs` (new, a section per method) — 10 of 15 failed (the four
  invalid-node partial inserts, `replace_children`'s clear, `after` with a fragment, `prepend` with the
  first child, `replace_with` with itself, alone and with another); the other five pin cases that already
  held (a fragment appended, a sibling in `before` / `after`, `replace_with([])`, a current child in
  `replace_children`); the existing `accessor/tests.rs` helpers' tests unchanged and green. CHANGELOG
  silent change 85.
- 2026-10-08 — C13G-MISC (architect N9, N10, N11). (1) `Stylesheet::append` destructures `other` with every
  field named and none behind `..` (the index, the source counter, the owner node and the version ignored
  with their reason; the scopes mapped by `append_scopes`) — a new collection fails to compile until
  `append` carries it, as `@keyframes` went missing silently. A compile-time guard: no runtime test, the
  workspace build is its check. (2) The DESIGN type check (`rdom-showcase/tests/integration/design_types.rs`)
  follows `pub mod`: `surface` takes the module path it reads and adds the surface of each `pub mod x;`
  it declares, recursively (an inline `pub mod x { … }` is already in the scanned source). Red:
  `the_surface_follows_pub_mod_paths` (`[]` for `["Deeper", "InMod", "Used"]`); then the full check
  found 37 public types reachable only through `pub mod` paths and unclassified — classified now in
  DESIGN: open vocabularies `EditKind`, `ScrollbarPart`, `StyleSlot`, `TransitionEventKind`; error sets
  `TokenizerError` / `TokenizerErrorKind` (the entry was the non-name `TokenizerError(Kind)`),
  `SubstitutionError`; options bags `SubstitutionContext`, `UnitContext` (new since 0.5, public fields,
  no attribute: made `#[non_exhaustive]`, built by `UnitContext::new` — API-table row and the
  `table_span_and_unit_context_hints` group, which also gives `CellSpan`'s C13G-COLUMN-MATCH reshape the
  row and hint it lacked); closed CSS keywords `CaretColor`, `CaretTextColor`, `PointerEvents`; sets fixed
  by definition `SelectorList`, `ComplexSelector`, `CompoundSelector` (Selectors 4 §3.1's structure);
  sealed `TransitionShorthandRule`, `SourceCursor`; runtime internals `AnimationRegistry`, `PendingEvent`,
  `PendingCustomEvent`, `PresentationStyle`, `EditorState`, `TypeaheadState`, `TimerId`, `CanvasPaint`,
  `InlineLayout`, `InlineFlow`, `SgrState`, `BorderContribution`, `BorderDirState`, `StyleDeclaration(Mut)`
  and the clipboard / URL-opener backends. It then found `NthColumnSelector` listed under both an open and
  a closed kind (a prose mention in the `NthKind` entry, invisible while the type was unreached): reworded.
  The cross-crate name collision and the multi-line `#[derive(…)]` lookback (N10's other two) stay as
  they are — no type trips them today. (3) DIVERGENCES §2 "Timers & animations" gains C12G-BEFORE-CHANGE's
  nuance: a cascade change to a longhand a CSS animation drives can start a transition under it, which
  outlives the animation (browsers snap).
- 2026-10-09 — C13G-TH-CAPTION (API B1; HTML §15.3.8). Found: the UA `th` only bolded and `caption` was italic
  and `TEXT_MUTED`, left-aligned — HTML has `caption { text-align: center }` and no font or colour for it,
  and a rule "that matches `th` elements that have a parent node whose computed value for the `text-align`
  property is its initial value" setting `text-align: center`; neither difference was in DIVERGENCES.
  Decided: (1) the conditional rule is a value, as in browsers (Blink's UA-only `-internal-center`, Gecko's
  `-moz-center-or-inherit`): `TextAlign::InternalCenter` (keyword `-internal-center`; the parser never
  produces it, pinned in `text_tests.rs`), set as the UA `th`'s `text-align-all` and computed away in
  `cascade::text::finalize_text_align` beside `match-parent` — `center` when the parent's `text-align-all`
  is `start` (or there is no parent element), else the parent's value as inherited (the rule does not
  match, so only inheritance sets it). An author `th` rule of any value, `inherit` included, wins by the
  cascade; a plain UA `th { text-align: center }` would have broken `table { text-align: right }`. Only
  `text-align-all` is read and set: browsers' `text-align` and `text-align-last` are separate properties,
  and the rule's reach is theirs. `TextAlign` is new since 0.5, so the variant is no break. (2) `caption`
  takes `text-align: center` and drops the italic and muted colour (HTML gives it neither). CHANGELOG
  silent change 8 says both, with `th { text-align: start }` to restore 0.5. Red (`css_phase13/html.rs`):
  `th_is_centred_unless_its_parent_aligns_text` (`Start` for `Center`),
  `header_cells_paint_centred_over_their_columns` (`" Name      Size"` for `"   Name    Size"`),
  `a_caption_is_centred_and_plain` (`"Cap"` at column 0 for column 4); green after. One existing test
  changed: `paint_pass/tests.rs`' `table_caption_uses_italic_dim_style` pinned the old caption look, which
  this item removes; it is now `table_caption_is_centred_and_plain`. No snapshot paints a `th` or a caption.
- 2026-10-09 — C13G-TABLE-UA (API B2, N4; HTML §15.3.8). Found: the UA `table` lacked HTML's `box-sizing:
  border-box`, so `table { width: 100%; border: solid }` was its container plus two (the documented
  `width: 100%` migration overflowed every bordered table, in both border models); `tr` set `vertical-align:
  middle` itself, so `tbody { vertical-align: top }` never reached its cells; `table { text-indent: initial }`
  was missing, so an indented page indented every cell. Checked against the section's whole table sheet:
  `table { box-sizing: border-box; border-spacing: 2px; border-collapse: separate; text-indent: initial }`,
  `td, th { padding: 1px }`, `th { font-weight: bold }`, `caption { text-align: center }`, `thead, tbody,
  tfoot, table > tr { vertical-align: middle }`, `tr, td, th { vertical-align: inherit }`, `thead, tbody,
  tfoot, tr { border-color: inherit }`, the `rules` / `frame` attributes' `border-color: black` (presentational
  attributes rdom does not map), and the quirks-mode `table` font / `line-height` / `white-space` /
  `text-align` reset (no quirks mode here; HTML has no `border-color: gray` today). Decided: the UA takes
  every rule above but the 2px spacing and the 1px padding — `table` `border-box` and `text-indent:
  initial`, the row groups `middle` and `border-color: inherit`, `tr` inheriting both, a new `table > tr {
  vertical-align: middle }` (UA rule count 186 → 187). The earlier decision on spacing and padding stands:
  a 2px gap rounds to 0 cells and the one-cell inline padding keeps text two blank cells apart, rows
  touching — not cramped at terminal scale; DIVERGENCES' table entry now says so. Red (`css_phase13/ua.rs`,
  new): `a_full_width_bordered_table_fits_its_container` (42 for 40), `rows_inherit_vertical_align_from_their_group`
  (`Middle` for `Top`), `a_table_resets_text_indent` (`"   abc"` for `" abc"`),
  `rows_and_groups_inherit_the_tables_border_color` (`Reset` for red); green after. `ua_total_rule_count`
  recounted. No snapshot changed.
- 2026-10-09 — C13G-TABLE-TRACKS (API B3; CSS 2.1 §17.4, §17.5, §17.6; CSS Writing Modes 4 §2.1). Found: removing
  `TuiExt::table_used_width` left no public read of a table's column widths (grid has `grid_tracks()`; `Solved`
  is private; a cell's `layout_rect` is its column only for a non-spanning cell in the separated model). And,
  writing the rtl case: an `rtl` table laid its first column on the left — the table formatting context never
  read `direction` (CSS 2.1 §17.5: the columns follow the table's direction), an undocumented gap. Decided:
  (1) `TuiAccessors::table_tracks() -> Option<TableTracks>`, `GridTracks`' shape (private fields, `new`,
  `columns()` / `rows()`, sealed by private fields in DESIGN, re-exported at the root): each track the cells
  between its two lines, from the table box's content edge (captions outside it, §17.4), unscrolled, in
  column / row order — an `rtl` table's first column its rightmost range, as an `rtl` grid's; separated:
  `border-spacing` the gaps and before the first range, the table's border and padding outside the edge (a
  column is the border box of a cell spanning only it); collapsing: no padding, the border the grid's outer
  line, so the edge is the border edge and each one-cell line a gap; a collapsed column / row an empty range.
  (2) Read, not solved: `place` keeps the tracks in the table's kept record — `KeptLayout::Table(TableKept {
  insets, columns, rows })`, now kept for every laid-out table element (it was only for one with captions;
  `TuiExt::has_captions` answers the stacking question that `matches!(kept, Table(_))` did), relative to the
  content edge so a subtree shift keeps it true; the accessor clones it. (3) `rtl` tables: `Lines::rtl` — a
  box's right border marks the line before its first column (`mark` swaps the sides) — and `place::Axis`
  lays the column lines and tracks right to left (`Axis::columns`, `span` mapping column order to physical
  order); one source, so cells, rows, column boxes, junctions and the kept tracks agree. CHANGELOG: Added
  bullet, the Fixed `rtl` bullet, silent change 8 gains it, and the `table_used_width` Breaking bullet and
  API-table row name `table_tracks()` as the migration (`table_layout_hints` asserts it). Red:
  `css_phase13/tracks.rs` (new) failed to compile without the accessor; with it,
  `an_rtl_tables_first_column_is_on_the_right` failed (`[(0, 3), (3, 8)]` for `[(5, 8), (0, 5)]`); green after,
  with the separated, spacing, collapsed, caption, collapsed-column and not-a-table cases and
  `cost_tests.rs`' `table_tracks_read_the_kept_layout_without_solving` (0 structures, 0 solves). Mutation:
  `mark` without the swap → the `td { border-right: solid }` rtl case gives `[(7, 10), (1, 6)]` for
  `[(6, 9), (0, 5)]`.
- 2026-10-09 — C13G-UPGRADE (API N2 and the rdom-virtualtable notes; docs and pins). Found: the tables entry was
  silent change 8, below four changes that only touch sheets declaring a property, though it re-renders every
  `<table>` with no author CSS; its `width: 100%` migration missed the border (fixed by C13G-TABLE-UA), the
  spread of extra width, wrapping, a cell `width` being a minimum, and the CSS that stops applying. Decided:
  (1) ranked 4th (after `display: flex` rows, `content-box` and flex base sizes, which reach more boxes; above
  `appearance: none`, the old 4th) — 4–7 move to 5–8; (2) its migration lists (a) `width: 100%` with the UA's
  new `border-box`, (b) the extra spread by max-content (0.5's content-wide left-packed columns do not come
  back), (c) `td, th { white-space: nowrap }` for one-line rows, (d) a cell `width` as a minimum in the
  automatic layout and exact only under `table-layout: fixed` with a table width, (e) no `flex` on cells,
  `gap` on the table or cell margins, and `table_tracks()` for column widths, and folds in C13G-TH-CAPTION's,
  C13G-TABLE-UA's and C13G-TABLE-TRACKS' table changes; (3) a "Porting a column-synced table" block after the
  silent list — rdom-virtualtable's shape (header widths exact under `table-layout: fixed; width: 100%`, the
  header being the first row so materializing rows cannot move the columns; `column_width(i)` as
  `table_tracks()?.columns()[i].len()`; the overflow chip out of the table, absolutely positioned in the
  pane). rdom-virtualtable itself is untouched. Pins (`css_phase13/upgrade.rs`, new; each claim held on first
  run, so no red — a docs item): `a_full_width_table_spreads_its_extra_width`, `cells_wrap_unless_nowrap`,
  `a_cell_width_is_a_minimum_unless_fixed` (10 auto, `[5, 3]` fixed), `flex_gap_and_cell_margins_do_not_apply`,
  `a_virtual_table_ports_to_css_tables` (columns `[12, 6, 12]`, the long cell clipped at its padding edge,
  the chip at (28, 0)). The first draft of the last expected the clip at the content edge; `overflow: hidden`
  clips at the padding box (CSS Overflow 3 §3), so the pin and the example say so.
- 2026-10-09 — C13G-DOCS (API N3, N5, N6; docs and pins). (1) DIVERGENCES' `border-collapse` entry said collapse
  reaches only a container's direct children and every container in the chain must declare it — false for a
  table, whose formatting context collapses the borders of cells under rows and row groups (CSS 2.1 §17.6.2):
  retitled "does not inherit, and outside a table it collapses a flex or block container's direct children",
  the table named as the one collapse context of its grid, and the inheritance axis now says a table nested
  in a collapsed table's cell is separated unless it declares collapse. (2) CSS-COVERAGE's `display: table`,
  `table-layout` and `caption-side` rows pointed at the deleted `runtime/builtins/table` / "table builtin":
  now `render/layout_pass/table/` (and its `place.rs` for captions). (3) The table builder's module doc names
  `empty-cells` and where `border-collapse` / `border-spacing` are set. (4) README.md's historical "0.1.0
  substrate" section is back to its wording (`<table>` family + column-width sync), which C13-TFC had
  rewritten; its UA rule count, kept current by convention, is 187 (C13G-TABLE-UA missed it). (5) Pins
  (`css_phase13/paint.rs`, new; held on first run): `zebra_rows_paint_across_their_cells` (the stripe on cells
  0–6 of the even row, padding included, not the odd rows, not past the table) and
  `a_wide_table_scrolls_in_an_overflow_auto_wrapper` (a block wrapper: the table shrinks to it, 16, and wraps;
  at `width: max-content` 25 wide, `scroll_width` 25, the row clipped at the wrapper). `width: 100%` and `th`
  defaults were pinned by C13G-TABLE-UA and C13G-TH-CAPTION. Found writing the wrapper pin: an `overflow:
  auto` box that is a direct child of the document root shows a vertical scrollbar it does not need
  (`scroll_height` 3 for a 2-row table, 5 for three lines of plain text — not table-specific), while the same
  box inside a block does not; the root's flex-column layout is the suspect, so it is left to C13-ROOT-BLOCK
  (batch C), and the pins and the README recipe nest the wrapper in a block. (6) rdom-tui README "Tables": a
  doctested data table — collapsed borders, zebra body rows, `col.num || td { text-align: right }`, `width:
  max-content` in an `overflow: auto` wrapper, `table_tracks()` for headers — asserting painted rows, the
  stripe and the column ranges `[1..19, 20..26]`. (7) ACID tile 14 adds zebra rows, unstyled `th` and caption
  defaults, a bordered `width: 100%` table, a `table-layout: fixed` table and an anonymous table, with what
  each verifies (ACID.md is still a proposal).
- 2026-10-09 — C13-ROOT-BLOCK (Phase 13 gate decision; architect "Design question: the root special cases"; CSS
  2.1 §10.1, §9.4.1, §8.3.1, §9.5, §9.2.1.1, §10.5, §10.6.3). Inventory — every place layout, cascade, paint or
  hit-test treated the document root or its children specially, and what each became: (1) `layout_pass/mod.rs`
  `layout_fragment_children`, the explicit viewport column (`Flow::Flex` + `Direction::Column`,
  C6-FLEX-DIRECTION-INITIAL) the root fragment laid its element children out in, its text not laid out → gone:
  `layout_pass/icb.rs` lays the children out in block flow in the initial containing block
  (`block::layout_block_children` with `box_tree::icb::style()`, `flow-root`), the viewport's size, its height
  definite; (2) an element root (`Dom::with_root_tag`) was laid out over the whole viewport → placed as a block in
  the ICB (`block::layout_root_element`: width less its margins, height its content's unless set); (3)
  `flex/cross.rs` `hugs_as_inline_level`, the no-stretch exception for root-level inline blocks (C6G-FLEX-SPEC) and
  tables (C13-TFC) → deleted; (4) `block/margin_collapse.rs`: every child of a Fragment root an independent
  formatting context (P6G-ROOT-MARGIN-1) → the root's children are ordinary blocks, the ICB the boundary; (5)
  `block/height.rs`: a root child's `auto` height definite only while it grew (`flex-grow`), a `fr` height under
  the root definite → removed (a `fr` height under the ICB is the non-flex case, indefinite); (6) `float/mod.rs`
  `float_side`: a box whose parent is no element did not float (C8-FLOAT) → the ICB is an `ltr` block container,
  its children float; (7) `cascade/blockify.rs` `children_are_items` (C6G-BLOCKIFY: root children not
  blockified), `flex::is_collapsed` (root children's `collapse` is `hidden`), `box_tree::is_flex_container` and
  `stacking::paints_atomically` (root children paint as blocks) — code unchanged, each justified by the column
  before and by the ICB now (docs rewritten); (8) paint (`stacking_walk::paint_unit`) and the hit test
  (`descend::hit_stacking_context`) recursed into the root's element children only → the ICB's anonymous boxes
  are kept as document data (`box_tree::icb::{anonymous_blocks, set_anonymous_blocks}`, the one read of an
  element's or the ICB's: paint, `nearest`, `inline_flow_for_text` / `inline_flow_layout`), painted after the
  children, and `in_a_line` treats the ICB as block flow (a root-level atom paints with its line); (9) the root's
  containing block for positioned children is the viewport — unchanged (it is the ICB). Docs that said otherwise:
  DIVERGENCES §2's margin entry (top-level margins add), percentage-height entry (definite only when growing),
  floats entry (fourth simplification) and `Dom::root()` entry; CSS-COVERAGE's float row; the rdom-tui README's
  floats paragraph; CHANGELOG silent change 41's "the document root's children stack in a column". Found: (a) the
  "spurious scrollbar" C13G-DOCS reported is the column flex-shrinking a root-level `overflow: auto` box taller
  than the viewport to fit it (its automatic minimum is 0), its content then overflowing — block flow keeps the
  `auto` height; (b) a hit on an inline in an anonymous block box's line (`<div><p>…</p>text <b>x</b></div>`)
  never reached the inline, for any block container — the column hid it at the root, where top-level `<a>` and
  `<span>` were flex items: `inline_hit::hit_anonymous_lines` searches those lines as an IFC block's, from
  `hit_content` and the root. Decided: no UA `height: 100%` for the root's children (browsers give `html` none,
  and a UA rule would make every top-level element viewport-tall; `:root` matches the fragment) — an app fills the
  screen with CSS, as on the web, and the root element's background fills it by propagating to the canvas
  (C13-ROOT-CANVAS, next). The showcase: `.app-shell` `flex: 1` → `height: 100%`; every demo root that relied on
  `flex: 1` (thirteen) and `scroll-list-demo` / `mo-demo` (`height: 100%` overflowing by their padding once
  nothing shrank them) declare `height: 100%; box-sizing: border-box` — the snapshots are unchanged; in the app
  the view pane already laid its demo out in block flow, where `flex: 1` did nothing, so the demos now fill the
  pane as written; the three rdom-tui examples likewise. Red (`css_phase13/root.rs`, new): sibling margins
  collapse (y 6 for 4), a root child floats (x 0 for 7), an `auto` root child that would grow is indefinite (12 for
  1), root-level inline blocks share a line (`(0, 1)` for `(3, 0)`), an element root is a block in the ICB
  (`(0, 0, 20, 1)` for `(1, 0, 18, 1)`), the README data table at the root in an 8-row viewport has no vertical bar
  (`scroll_height` 11 for 10), an inline in an anonymous line is hit (`None` for the `<a>`); guards that held
  before and after: `height: 50%` / `100%` of the viewport and definite below, a root-level table hugs its content
  and centres with `margin: 0 auto`, `flex: 1` on a root child grows nothing but a `height: 100%` flex shell
  does. Green after. Mutation (each alone, restored, touched): the ICB's children back to independent formatting
  contexts → the three `block_tests` root-margin tests; `float_side`'s ICB branch out → the float test; the old
  `flex-grow` definiteness → the indefinite test; `hit_anonymous_lines` out of `hit_content` → the element half of
  the hit test; the ICB's anonymous boxes not kept → the inline-block and hit tests; the root's anonymous-box paint
  out → the inline-block test. Existing tests changed: `block_tests`' P6G-ROOT-MARGIN-1 section pinned the column
  (margins inside a root child) — now the ICB contract (collapsing through it, stopping at the ICB); a `1fr`-high
  root flex row (`an_inline_block_flex_item_hugs…`) and a root-level aspect-ratio item read the column — given a
  fixed height and a column flex container; `css_phase3_colors`' bordered box shrank into an 8×3 viewport — now
  `border-box`; `css_values`' percentage margin now collapses through its block parent — `.cb` is `flow-root`;
  `css_phase8`'s floated-pseudo hit needs `<body>` to fill the screen — `body { height: 100% }`. CHANGELOG: silent
  change 2 (the old 2–85 move to 3–86, "Porting a column-synced table" cites 5), a Breaking bullet, two Fixed
  bullets, a showcase bullet; README "The document root and a full-screen app" with a doctested shell. DESIGN: the
  ICB paragraph in "Layout passes" and a decision-archive entry. TECH_DEBT `SIZE-1` recounted.
- 2026-10-09 — C13-ROOT-CANVAS (C13-ROOT-BLOCK's second half; CSS Backgrounds 3 §2.11.2). Found: paint had no
  canvas — the root element's background painted its own box only, which C13-ROOT-BLOCK made as tall as its
  content, so an app's background no longer reached the bottom of the screen. Decided: `paint_pass/canvas.rs` —
  `source` is the root element when its background paints (`fills`), else, the root element being an `html`, its
  first `body` child's (§2.11.2's HTML rule); `paint_dom` fills the viewport with it before the root stacking
  context (a translucent one composited once over the terminal default), and `paint_box` skips that element's own
  background ("the used value … is transparent"). The root element is `Dom::document_element` — rdom-core's one
  definition (an element root, else the root fragment's first element child), not a sole-child rule tried first:
  that made the canvas flicker off when a `<dialog>` was appended beside the app (DIVERGENCES' `Dom::root()` entry
  says so). It needs a box (`display: none` / `contents` paint nothing); `visibility` does not stop it (the canvas
  draws it, not the box). Red (`css_phase13/root.rs`, at C13-ROOT-BLOCK's head): the canvas below a content-high
  root element (`Reset` for the colour at (0, 3)), `body`'s background under a transparent `html` (`Reset` at
  (9, 3)); `the_propagated_background_paints_once` (a translucent root background as dark on its box as on the
  canvas) holds only with the box's own fill skipped. Green after. Mutation (each alone, restored, touched): the
  canvas fill out → both canvas tests; the `html` / `body` rule out → the `body` test; the box's own fill back →
  the paints-once test. Existing tests changed — each read a box's own background where its box is the document
  element, which the canvas now covers (a browser draws the same): `paint_pass/tests.rs`' `bg_fills_outer_rect…`,
  `higher_z_paints…`, `modal_dialog_repaints…`, `positioned_paints_above…`, `pseudo_background_under_opacity…`,
  `translucent_card…`, `css_phase4`'s background and `background-clip` tests, `css_phase6`'s `visibility: hidden`
  test, `css_phase7`'s negative-`z-index` test (a root element's negative layer paints over its canvas, as in a
  browser) and the README's transitions example put their boxes in a `<body>`. CHANGELOG silent change 3 (the old
  3–86 move to 4–87; the table port cites 6) and an Added bullet; CSS-COVERAGE's `background-color` row.
- 2026-10-09 — Phase 13 closed: both gates run, their findings fixed in three batches — A (correctness and cost:
  C13G-COLUMN-MATCH, -SPAN-COST, -TABLE-COST, -TABLE-GEOMETRY, -COLUMN-INVALIDATION, -SPEC-GAPS,
  -CALC-SIZE-AUTHORED, -TEARDOWN-COST, -DOM-CONVENIENCE, -MISC), B (API, UA and docs: C13G-TH-CAPTION, -TABLE-UA,
  -TABLE-TRACKS, -UPGRADE, -DOCS) and C (the document root as the initial containing block: C13-ROOT-BLOCK,
  C13-ROOT-CANVAS). Their re-review rides with the Phase 14 gate.
- 2026-10-09 — C14-MEDIA (part 1 of 2: `@media` in the parser and the cascade; Media Queries 4 §2–§4, §6–§7,
  Media Queries 5 §12, CSS Conditional 3 §2–§3, CSS Nesting 1 §3.2, CSS Cascade 5 §3). rdom-style gains
  `conditional/`: Kleene `Truth` and `Condition<L>` (`not` / `and` / `or` over leaves, `<general-enclosed>` kept as
  text and unknown) — the one grammar `@supports` and `@container` will reuse — over a component-value tree of
  the prelude's tokens (`syntax.rs`: blocks and functions, spans kept so a leaf is cut out as written and `>=` is
  one operator only without a space); `media.rs`: `MediaList` (named for CSSOM's `MediaList`; rdom-tui's
  `matchMedia` result takes `MediaQueryList`), each query `not all` when it does not parse (§3.2), features as
  boolean / plain / range tests (`min-` / `max-` normalized to ranges, `a < name <= b` both directions),
  evaluated by a terminal mapping; `media_env.rs`: `MediaEnvironment` (viewport, scheme, preferences) and
  `MediaPreferences` (`with_*` builders). Decided: lengths are cells — a number or `ch` — and a pixel or
  font-relative length is unknown, never a guessed scale (DESIGN's pixel rule): `(min-width: 600px)` and its `not`
  both stay off; an unknown result is false even under `not` (§3.2). `hover` / `pointer: fine` (the brief leaned
  `coarse`: a terminal mouse hits any cell exactly and targets are whole cells, so `coarse`'s "enlarge targets"
  would only cost cells); `grid: 1` (MQ4 §4.4's own example is a tty); `resolution` false (no pixel density —
  §2.4's "concept does not exist"); `color: 8` (rdom always emits 24-bit SGR; the backend has no colour-depth
  notion, so it is a preference an app may lower); `display-mode: standalone`; the rest in DIVERGENCES §2. Stored
  as rule context, as `@layer` / `@scope` are: `Stylesheet::declare_condition(ConditionRule { kind, parent })`,
  `RuleContext::in_condition`, `Rule::condition`, and `KeyframesRule::condition` / `CounterStyleDefinition::
  condition` (a definition inside `@media` names its rule only while it holds); `append` remaps them. rdom-css:
  `conditional.rs` opens `@media` (top level and `@layer` bodies a rule list, nested a block's contents with the
  parent's selector), `@import`'s media list wraps the imported rules in a condition; keyframes / counter-style
  parsers take the `RuleContext`. rdom-tui: `Sheets::new` takes a `MediaEnvironment` (viewport, scheme and the new
  document data `MediaPreferences`, `CascadeExt::set_media_preferences`); `cascade/conditions.rs` evaluates every
  sheet's conditions once per environment (`ConditionCache` on the sheet set's facts; an environment that flips
  nothing keeps the same `Rc`), `Sheets::applies(sheet, rule)` gates on them, and `MatchedRules` records the
  results it was matched under so a restyle never replays a match a flipped query invalidated; the counter-style
  and keyframes facts are built per run only when a definition is conditional. Red: `css_phase14/media.rs` — all
  11 failed on HEAD (the strict sheet rejected `@media` as unsupported; the import test applied the narrow sheet in
  a wide viewport); rdom-style `conditional::tests` (9) and rdom-css `media.rs` (6) were compile-red (no module /
  fields). Green after. Mutation (each alone, restored, touched): `applies` ignoring the conditions → 9 of the 11
  integration tests; `is_for` ignoring the results → `a_flipped_query_invalidates_recorded_matches`; the cache
  re-evaluating every run → `media_conditions_evaluate_once_per_environment`. Changed expectation: rdom-css
  `at_rules`' skipped-block test used `@media` as its unsupported at-rule — now `@page` with a nested margin rule.
  `media.rs` came out at 757 lines formatted: the feature (`media_feature.rs`: parse, values, evaluation) split
  from the list and query (`media.rs`). Silent change 26 (the old 26–87 move to 27–88). Part 2: the `App`'s preferences, `matchMedia`, resize restyle
  by flipped queries, `<style media>`.
- 2026-10-09 — C14-MEDIA (part 2 of 2: the `App`'s media environment; Media Queries 5 §12, CSSOM View §4.2, HTML
  §4.2.6, CSS Values 4 §6.1.2). `App::with_media_preferences` / `set_media_preferences` / `media_preferences`
  (config table row); `App::match_media(query) -> MediaQueryList` (`runtime::media_query`: a shared handle with
  `media()` — the serialization — `matches()`, `add_listener(FnMut(&mut TimerCtx, &MediaQueryListEvent))` /
  `remove_listener`; the App holds the lists weakly). Decided: the reports run in `draw_if_dirty` after the
  prelude, before the dirty roots are taken (HTML's "evaluate media queries and report changes" precedes the
  frame's style), against the backend's current size, and only when the environment moved; a listener reaches
  the document through a `TimerCtx`, as timer callbacks do; listeners added by a listener run from the next
  flip. Resize: `must_restyle` (`cascade/media.rs`) — the whole tree cascades again only when a computed style
  read a viewport-percentage length or a condition flipped; otherwise the frame lays out (the resize handler
  notes `Layout`, the frame's viewport check decides). Viewport reads are counted where every such length
  resolves (`ViewportUnit::percent_of`, `rdom_style::calc::viewport_reads()`, a monotonic per-thread count
  read as a difference, so it carries no state between cascades — chosen over threading a flag through
  `UnitContext`, which is `Copy`, and the line-height, `vertical-align` and registered-property resolvers that
  read the viewport outside it) and noted on the document (`MediaState`, sticky: a stale `true` costs a
  cascade, a lost one would leave stale lengths) by every cascade form, keyframe and starting styles included;
  the condition results are recorded by whole-tree cascades only (a subtree cascade leaves the rest of the tree
  under the old ones). A preference change cascades only when a query flips. `<style media>`: CSSOM
  `StyleSheet.media` as `Stylesheet::media` / `set_media` (rdom-style), evaluated with the sheet's conditions
  (`ConditionResults` per sheet: its media list, then its rules'); `StyleElements` passes the attribute and
  re-parses when it changes (the observer watches `media` on connected `<style>` elements); `append` turns the
  appended sheet's media list into a root condition. Red: `runtime/app/media_tests.rs` was compile-red (no
  `with_media_preferences`, `match_media`, `set_media`). Green: all 11. Mutation (each alone or in independent
  groups, restored, touched): always cascading on a resize → `a_resize_that_flips_nothing_only_lays_out`;
  `must_restyle` ignoring the viewport reads and the flipped results, and the lists reporting without a flip →
  six tests (the viewport-unit, flip, preference, `<style media>`, sheet-media and change-report ones). The
  reduced-motion test pins that an animation stops through CSS alone. CSS-COVERAGE: `@media` Supported (§3.21
  1 / 0 / 5 / 1, total 238 / 7 / 17 / 45). Silent change 26 gains `<style media>`. Item done.
- 2026-10-09 — C14-SUPPORTS (CSS Conditional 3 §6–§7.1, Conditional 4 §6.1, Conditional 5 §5, CSS Cascade 5 §3).
  rdom-style `conditional/supports.rs`: `SupportsCondition` over the shared `Condition<L>` grammar with
  `SupportsFeature` leaves — `(<property>: <value>)` (the value cut out as written by the component-value
  tree), `selector()`, `font-tech()`, `font-format()` — anything else `<general-enclosed>`, unknown. Decided:
  evaluated once, in `SupportsCondition::parse` (what rdom supports never changes while it runs), so the cascade
  only reads the stored result (`ConditionKind::Supports`); a declaration holds when it is a custom property or
  `property_dispatch::set` — the real value parser — takes it (`!important` stripped: it is part of a
  declaration; a `var()` value is kept for the cascade and so holds, as in a browser); `selector()` holds for
  exactly one complex selector `StyleSelector::parse` accepts (pseudo-elements included; a list is not one —
  `StyleSelector::len` added); the font functions are false (no fonts). `CSS.supports()` is two free functions,
  `rdom_style::supports(property, value)` (a value with `!important` is not one, Conditional 3 §7.1) and
  `supports_condition(text)` (a bare declaration retried parenthesized, as CSSOM does). rdom-css: `@media`'s
  opener generalized to `open_conditional_rule(name, …)` with `is_conditional`; an invalid `@supports` prelude
  drops the rule (`InvalidAtRulePrelude`, block skipped); `@import … supports(…)` takes a condition or a bare
  declaration (Cascade 5 §3) and conditions the imported rules, an unparseable one drops the import. Red:
  rdom-css `supports.rs` 4 of 4 and rdom-tui `css_phase14/supports.rs` 3 of 3 failed on HEAD (`@supports`
  unsupported, the strict sheet rejected it); rdom-style's four supports unit tests were compile-red. Green
  after. Mutation (restored, touched): the cascade ignoring the result → 2 integration tests; the declaration
  test skipping the value parser → 3 unit tests, 1 integration and the import test. Silent change 26 now
  "`@media` and `@supports` rules apply". CSS-COVERAGE §3.21 2 / 0 / 4 / 1, total 239 / 7 / 16 / 45.
