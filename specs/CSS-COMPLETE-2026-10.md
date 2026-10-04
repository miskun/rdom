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
| 2 | Values, units, math functions | |
| 3 | Color | |
| 4 | Backgrounds and borders | |
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
| C2-VIEWPORT | `vw` / `vh` / `vmin` / `vmax` and the `sv*` / `lv*` / `dv*` / `vi` / `vb` variants (terminal size) | |
| C2-ANGLE | `<angle>` (`deg` / `grad` / `rad` / `turn`) | |
| C2-RATIO | Full `<ratio>` (bare numbers, decimals, `auto && <ratio>`) | |
| C2-ATTR | `attr()` with fallback and `type()` (Values 5) | |

(`cq*` units land with C14-CONTAINER.)

### Phase 3 — Color (audit §3.4)

| Id | Item | Status |
|---|---|---|
| C3-RGB | Modern `rgb()` / `rgba()`: space syntax, `/ alpha`, percentages, `none` | |
| C3-TRANSPARENT | `transparent` as a real fully transparent color (not `Reset`) | |
| C3-CURRENTCOLOR | `currentColor` | |
| C3-HSL-HWB | `hsl()` / `hsla()` / `hwb()` | |
| C3-LAB | `lab()` / `lch()` / `oklab()` / `oklch()` / `color()` with gamut mapping to sRGB | |
| C3-MIX | `color-mix()` | |
| C3-RELATIVE | Relative color syntax (`rgb(from …)`) | |
| C3-SYSTEM | System colors (`Canvas`, `CanvasText`, `LinkText`, `ButtonFace`, …) | |
| C3-SCHEME | `color-scheme` and `light-dark()` (terminal background via OSC 11 / mode 2031) | |
| C3-ALPHA | Color alpha composited over the backdrop (shares the opacity compositor) | |

### Phase 4 — Backgrounds and borders (audit §3.5)

| Id | Item | Status |
|---|---|---|
| C4-BACKGROUND | `background` shorthand (color layer; image layers parse and are inert, documented) | |
| C4-BG-CLIP | `background-clip` (`border-box` / `padding-box` / `content-box`) | |
| C4-BORDER-SHORTHAND | `border` / `border-top` … with width, style and color in any order | |
| C4-BORDER-SIDES | `border-style` / `border-color` / `border-width` 1–4 values; per-side longhands for style, color and width | |
| C4-BORDER-WIDTH | `border-width` mapping (`0` = none, thin / medium = light, thick = heavy glyphs) | |
| C4-RADIUS | `border-radius` and per-corner longhands → rounded corner glyphs | |
| C4-SHADOW | `box-shadow` (one-cell offset shade; blur / spread documented N/A) | |
| C4-SPACING | `border-spacing` (lands with the table phase if it needs the TFC) | |

### Phase 5 — Box model and sizing (audit §3.6, §3.22)

| Id | Item | Status |
|---|---|---|
| C5-BOX-SIZING | `box-sizing` (`content-box` is the CSS initial value — breaking default change, migration note) | |
| C5-INTRINSIC | `min-content` / `max-content` / `fit-content()` on width / height / min / max | |
| C5-MINMAX-SIZE | `min-*` / `max-*`: `none`, `%`, `calc()` | partial — `max-*: none` remains (`%` and `calc()` landed with C2-PERCENT) |
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
| C6-FLEX-LONGHANDS | `flex-grow` / `flex-basis` longhands; full `flex` shorthand (incl. basis) | |
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
  absent `clamp()` bound) and becomes `#[non_exhaustive]`; the parser recognizes a math function at
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
