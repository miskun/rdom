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
| C3-SCHEME | `color-scheme` and `light-dark()` (terminal background via OSC 11 / mode 2031) | partial — mode 2031 theme-change notifications are not listened to: crossterm 0.28's input parser cannot pass the report through (`App::set_color_scheme` is the hook meanwhile) |
| C3-ALPHA | Color alpha composited over the backdrop (shares the opacity compositor) | done |

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
