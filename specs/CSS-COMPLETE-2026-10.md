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
| 4 | Backgrounds and borders | gates run 2026-10-06; `C4G-*` fixes in progress (C4-SPACING layout with C13-TFC) |
| 5 | Box model and sizing (incl. logical properties) | |
| 6 | Display, visibility, flexbox, box alignment | |
| 7 | Grid | |
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
| C5-BOX-SIZING | `box-sizing` (`content-box` is the CSS initial value — breaking default change, migration note) | |
| C5-INTRINSIC | `min-content` / `max-content` / `fit-content()` on width / height / min / max | |
| C5-MINMAX-SIZE | `min-*` / `max-*`: `none`, `%`, `calc()` | done (`%` / `calc()` with C2-PERCENT, `none` with C2G-MAX-NONE) |
| C5-MARGIN-TRIM | `margin-trim` | |
| C5-CONTAIN-SIZE | `contain-intrinsic-size` (+ longhands) | |
| C5-LOGICAL | Logical properties: `inline-size` / `block-size` / `min-*` / `max-*`, `margin-*` / `padding-*` / `border-*` / `inset-*` / radius logical forms (horizontal-tb ltr mapping) | |
| C5-WRITING | `direction` and `writing-mode` for the values a terminal can render (rtl lines; vertical documented N/A if not) | |

### Phase 6 — Display, visibility, flexbox, box alignment (audit §3.7, §3.8)

| Id | Item | Status |
|---|---|---|
| C6-DISPLAY-KEYWORDS | `display: contents` / `flow-root` / multi-keyword syntax | |
| C6-VISIBILITY | `visibility: visible / hidden / collapse` | |
| C6-ORDER | `order` | |
| C6-DIRECTION-REVERSE | `flex-direction: row-reverse / column-reverse` | |
| C6-FLEX-LONGHANDS | `flex-grow` / `flex-basis` longhands; full `flex` shorthand (incl. basis) | partial — the shorthand's grammar, shrink and stored basis landed with C2G-FLEX-SHORTHAND; remain the longhands and the basis in layout (`ComputedStyle::flex_basis` is cascaded, unread) |
| C6-WRAP | `flex-wrap` / `flex-flow`, multi-line flex containers | |
| C6-JUSTIFY | `justify-content` (all distribution values) | |
| C6-ALIGN | `align-items` / `align-self` (incl. `baseline` where meaningful) | |
| C6-ALIGN-CONTENT | `align-content` | |
| C6-PLACE | `place-content` / `place-items` / `place-self`, `justify-items` / `justify-self` (block-level) | |
| C6-GAP | `row-gap` / `column-gap` and two-value `gap` | |
| C6-SPLIT | File-size pass on `layout_pass/flex/*` after the above | |

### Phase 7 — Grid (audit §3.9)

| Id | Item | Status |
|---|---|---|
| C7-GRID-CORE | `display: grid` / `inline-grid`, `grid-template-columns` / `-rows` with cells / `%` / `fr` / `auto` / `minmax()` / `repeat()` | |
| C7-GRID-PLACE | `grid-row` / `grid-column` (+ start / end), `grid-area`, auto-placement, `grid-auto-flow` (`dense`) | |
| C7-GRID-AREAS | `grid-template-areas`, `grid-template`, `grid` shorthands | |
| C7-GRID-AUTO | `grid-auto-columns` / `grid-auto-rows` | |
| C7-GRID-ALIGN | Box Alignment in grid (`justify-*` / `align-*` / `place-*`) | |
| C7-SUBGRID | `subgrid` | |

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
| C8-POS-MINMAX | Positioned boxes (elements and pseudo-elements) honour `min-*` / `max-*` (CSS 2.1 §10.4 / §10.7 with §10.3.7 / §10.6.4); a `position: relative` pseudo with both insets takes its declared width | |

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
- 2026-10-04 — C1G-TYPED-ERRORS: `rdom_style::{RegisterPropertyError, PropertySyntaxError}`
  (`#[non_exhaustive]`, `Display` keeps the old messages, so `@property` warnings read the same);
  `PropertyRegistration::new`, `PropertySyntax::parse`, `App::register_property` return them. A test
  per variant (rdom-style) and for `AlreadyRegistered` through the `App`.
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
- 2026-10-04 — C2-TRIG: math expressions are type-checked (`CalcExpr::kind` → `CalcKind::{Number,
  Length, Angle}`, Values 4 §10.9) with rdom's number-is-a-cell relaxation (a number unifies with a
  length); `parse_calc` rejects an ill-typed tree and each property checks the kind it takes. Newly
  rejected as ill-typed: a product of two lengths (`calc(50% * 10%)`), a division by a length.
  Angles are radians inside the evaluator. `<number>` properties (`opacity`, flex factors) take math
  functions of type `<number>`. Constants are numbers once parsed (`pi` serializes as its value).
  `calc/mod.rs` split into `functions.rs` (the functions and their evaluation) and `types.rs`.
- 2026-10-04 — C2-CH: `CalcExpr::Dimension { value, unit: CalcUnit }` (`calc/units.rs`) is the leaf for
  every unit Phase 2 adds; `ch` folds to cells outside a percent-bearing expression. A registered
  `<length>` (`@property`) takes unit dimensions too.
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

