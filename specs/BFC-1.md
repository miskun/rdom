# BFC-1 — Block Formatting Context milestone

**Status:** in progress (started 2026-05-25)

**Goal:** rdom gets a real block layout pass alongside flex, so semantic HTML (`<h1>`, `<p>`, `<div>`, etc.) lays out the way the web platform specifies — block boxes stack vertically at natural heights, margins collapse, container overflows below.

**Premise:** Currently every container goes through the flex algorithm. `display: block` is parsed and stored but ignored by layout. The result: authors who write semantic HTML get flex semantics (distribution, shrink-to-fit, no margin collapsing). The user's complaint — "creating `<h1></h1><p></p><source>` should be trivial and isn't" — is the visible tip of this gap.

**Faithfulness target:** WHATWG-faithful for the supported subset. Spec references: CSS 2.1 §8 (Box Model), §9 (Visual formatting), §10 (Visual formatting model details), §8.3.1 (margin collapsing), §9.2.1.1 (anonymous block boxes).

---

## Locked decisions

1. **`Flow` enum name.** `Flow { Block, Flex }` — matches CSS3 Display Module's "inner display" concept. Lives alongside the existing `Display` (outer display).
2. **BFC formation tracking.** Add `establishes_new_bfc: bool` to `ComputedStyle`. Computed at cascade time from `overflow_x/y`, `position`, `display`, root identity, etc. Used by the margin-collapse pass.
3. **Anonymous boxes are layout-pass ephemera, not DOM nodes.** Allocated per layout pass into a `Vec<AnonymousBlockBox>` inside the block-layout state. No DOM mutation, no observer events.
4. **Inline-ancestor breaking around blocks is OUT OF SCOPE.** Per CSS 2.1 §9.2.1.1, `<p><span>before <h1>X</h1> after</span></p>` should break the `<span>` around the `<h1>`. rdom defers this — the HTML parser normally auto-corrects such inputs (closing the `<p>` before `<h1>`), so the scenario is rare in well-formed content. Documented divergence.
5. **`direction` without `display: flex`** — parse-accept-but-ignore + `WarningKind::DirectionWithoutFlex` lint. Backwards-compatible silently for now; lint surfaces authoring drift.
6. **`text-align` / `vertical-align`** — separate follow-up milestone (`TEXT-ALIGN-1`). Not gated by BFC-1 but the natural next item once block content stacks.
7. **Floats and `clear`** — parse-recognize the properties + `WarningKind::UnsupportedProperty`. Layout ignores them. Documented divergence.
8. **`<table>` and table-internal display values** — explicitly out of scope. Separate milestone if a real consumer needs it.

## Closes

- **`BFC-1`** in `TECH_DEBT.md` (the milestone itself).
- **`M5-MARGIN-1`** — margin collapsing.
- **`SUB-2`** — IFC requiring at least one inline-element child (anonymous box generation handles text-only paragraphs).
- **`IFC-MIXED-TEXT-INLINEBLOCK-1`** — text vanishing alongside inline-block siblings (same anonymous box fix).

## Estimated scope

2–3 weeks focused work, broken into 9 phases. Each phase is a shippable commit; phases 1–3 build infrastructure without changing user-visible behavior; phase 4 wires the dispatch and flips the contract.

---

## Phases

### Phase 1 — `Flow` enum + cascade + BFC formation predicate

Goal: cascade infrastructure for block layout. No layout dispatch change. No behavior change.

- Add `Flow { Block, Flex }` to `rdom-style/src/layout.rs`.
- Add `flow: Flow` to `ComputedStyle` with default `Flow::Block`.
- Add `establishes_new_bfc: bool` to `ComputedStyle`. Computed at cascade time when any of:
  - Element is `Dom::root()`.
  - `overflow_x` or `overflow_y` is non-visible.
  - `position` is `Absolute` or `Fixed`.
  - `display` is `InlineBlock`.
  - `flow` is `Flex` (flex containers establish a new BFC).
- Parser:
  - `display: flex` → `Display::Block` + `Flow::Flex`.
  - `display: inline-flex` → `Display::Inline` + `Flow::Flex`.
  - `display: block` → `Display::Block` + `Flow::Block` (current default).
  - `display: inline` / `inline-block` / `none` → unchanged outer, Flow doesn't apply (or carries `Block` for inline-block's inner).
- `direction` lint: emit `WarningKind::DirectionWithoutFlex` when `direction` is set but `flow` resolves to `Block`.
- Cascade tests pin the shape.
- **No layout change** — `Flow::Block` containers still route to flex via the unchanged dispatcher.

### Phase 2 — `layout_block_children` skeleton + width formula + auto margins + min/max

Goal: build the block layout pass. Not wired to dispatch yet.

- New module `crates/rdom-tui/src/render/layout_pass/block.rs`.
- `layout_block_children(dom, id, container, computed)`:
  - Filter out-of-flow children (reuse the future `is_in_flow` helper or inline the filter — see `DRY-1`).
  - Walk in-flow children in document order.
  - For each child, compute width via CSS 2.1 §10.3.3:
    ```
    margin-left + border-left-width + padding-left + width + padding-right + border-right-width + margin-right = width of containing block
    ```
    Cases: `width: auto`, `width: <fixed>`, `width: <percent>`, with combinations of auto margins. Auto margins absorb leftover horizontal space (centering when both are auto).
  - Apply min-width / max-width clamping.
  - Compute height: `auto` → intrinsic; `fixed` → declared; `percent` → resolved (with the definite-parent rule from phase 6).
  - Apply min-height / max-height clamping.
  - Place at current vertical cursor; advance.
- No margin collapse yet (phase 5).
- No anonymous boxes yet (phase 3).
- Tests (named subset):
  - `block_width_auto_fills_container`
  - `block_width_fixed_respects_declared`
  - `block_width_percent_resolves_against_parent_content_width`
  - `auto_margin_left_pushes_block_to_right_edge`
  - `auto_margin_right_pushes_block_to_left_edge`
  - `both_auto_margins_center_block`
  - `over_constrained_widths_resolve_per_ltr_spec`
  - `min_width_floors_block`
  - `max_width_caps_block`

### Phase 3 — Anonymous block box generation

Goal: handle mixed inline+block children in a block-flow container. Closes `SUB-2` and `IFC-MIXED-TEXT-INLINEBLOCK-1` mechanics. Not wired to dispatch yet.

- In `block.rs`, when walking children, group consecutive inline-level children (text nodes, `Display::Inline` elements, `Display::InlineBlock` elements) into ephemeral `AnonymousBlockBox` entries.
- Each anonymous block establishes its own IFC. Width fills container. Height = packed inline content height.
- Layout pass returns a `Vec<BlockChild>` where each child is either `Real(NodeId)` or `Anonymous(range of node indices)` — used by both layout positioning and the eventual margin-collapse pass.
- Anonymous boxes participate in margin collapse like real blocks (phase 5).
- Tests (named subset):
  - `text_only_paragraph_wraps_in_anonymous_block`
  - `mixed_text_and_inline_block_wraps_in_anonymous_block`
  - `block_then_text_then_block_wraps_text_only`
  - `consecutive_inline_runs_share_one_anonymous_block`
  - `anonymous_block_inherits_styles_from_parent` (color, font-weight inheritance)
  - `hit_test_through_anonymous_block_routes_to_parent`
  - `selection_position_resolves_to_real_text_node_through_anonymous_box`

### Phase 4 — Dispatch wiring + UA sweep + chrome + demo migration

First user-visible phase. After this commit, semantic HTML stacks correctly.

- `layout_pass/mod.rs::layout_children`: dispatch on `flow`:
  ```rust
  if is_ifc_block(...) { /* IFC */ }
  else if has_text_only_children_and_no_elements(...) { /* pure-text-leaf */ }
  else {
      match flow {
          Flow::Flex => layout_flex_children(...),
          Flow::Block => layout_block_children(...),
      }
  }
  ```
- UA stylesheet sweep:
  - Elements that need flex behavior get explicit `display: flex` rules: probably nothing in the UA (chrome / authors opt-in).
  - Elements that are block-by-default get `display: block` (`<div>`, `<p>`, `<h1>`–`<h6>`, `<section>`, `<article>`, `<header>`, `<footer>`, `<main>`, `<nav>`, `<aside>`, `<ul>`, `<ol>`, `<li>`, `<blockquote>`, `<pre>`, `<form>`, `<fieldset>`, `<details>`, etc.). Most already set this; audit for completeness.
- Chrome migration:
  - `.app`, `.app-body`, `.main`, `.sidebar` get explicit `display: flex` (they need column / row flex behavior).
  - `.main .source-disclosure`, `.main .scroll-indicator` are block by default — no change.
  - Remove the `.page` workaround (no longer needed; default block layout does the right thing).
- Demo migration:
  - Demos with `flex-direction: row/column` or `flex: <n>` children → add `display: flex` to the container.
  - Pure-text demos (Hello World, Headings, etc.) → no opt-in needed.
- Snapshot regeneration: every snapshot file audited individually.
- Demo trait doc: drop the obsolete admonitions.

### Phase 5 — Margin collapsing (CSS 2.1 §8.3.1)

Goal: full §8.3.1 rule matrix. Closes `M5-MARGIN-1`.

- Adjacent sibling collapse:
  - Both positive → max.
  - Both negative → min (most negative).
  - Mixed → `max(positives) + min(negatives)`.
- Parent–first-child top margin collapse:
  - Conditions: parent has no top padding, no top border, parent doesn't establish a new BFC, no clearance.
  - When all conditions hold, the child's top margin "escapes" through the parent (and recurse to parent's own parent).
- Parent–last-child bottom margin collapse: symmetric.
- Empty-block collapse-through:
  - A block with no content, no padding, no border collapses its own top + bottom margins together.
- Out-of-flow children skip from adjacency: `position: absolute/fixed` doesn't break sibling-margin adjacency.
- Anonymous boxes participate as real blocks.
- Tests (named subset, ~15 scenarios per the WPT-style matrix):
  - `adjacent_positive_margins_collapse_to_max`
  - `adjacent_negative_margins_collapse_to_min`
  - `mixed_positive_negative_margins_sum`
  - `parent_top_margin_collapses_through_borderless_padding_less_parent`
  - `parent_top_margin_does_NOT_collapse_through_padding`
  - `parent_top_margin_does_NOT_collapse_when_parent_establishes_new_bfc`
  - `overflow_hidden_parent_blocks_collapse_out`
  - `empty_block_collapses_top_and_bottom_through`
  - `absolute_positioned_does_not_break_sibling_adjacency`
  - ... etc.

### Phase 6 — Block height formula + percent-height-needs-definite-parent

Goal: CSS 2.1 §10.6.3 + §10.5.

- Block container height when `height: auto`:
  - With `overflow: visible`: top of topmost in-flow content to bottom of bottommost in-flow content, accounting for margin collapse out of parent.
  - With `overflow != visible`: same, but margins don't escape (BFC formation).
- `height: <percent>` resolves to parent's height ONLY IF parent's height is definite. Otherwise resolves to `auto` (CSS 2.1 §10.5).
- Min-height / max-height clamping.
- Tests (named subset, ~14 scenarios):
  - `block_height_auto_sums_children`
  - `block_height_fixed_overflows_excess_children`
  - `block_height_percent_resolves_against_definite_parent`
  - `block_height_percent_falls_to_auto_when_parent_indefinite`
  - `min_height_floors_block_above_content`
  - `max_height_caps_block_below_content`
  - `min_max_clamping_with_margin_collapse_interaction`

### Phase 7 — Test matrix completion + snapshot audit

Goal: ~75 named scenario tests, snapshot files reviewed.

- Lift WPT subset where applicable (`web-platform-tests/css/CSS2/normal-flow/`).
- Test files organized under `crates/rdom-tui/src/render/layout_pass/block_tests.rs` or similar — keep tests near the code they exercise.
- Snapshot audit: per-file eyeball review for visual correctness. No blanket-accept.
- Performance bench `bench_block_layout` (100-paragraph document); target equal-or-better than equivalent flex cost.

### Phase 8 — Documentation

- `specs/DESIGN.md`: new top-level architectural section "Layout passes" explaining flex / block dispatch + IFC + pure-text-leaf carve-out.
- `specs/DIVERGENCES.md`:
  - Retire any entries obsoleted by BFC-1 (the implicit "everything is flex" footgun, IFC text-only divergence if SUB-2 is closed, inline-block + inline divergence if IFC-MIXED is closed).
  - Add new entries for deliberate divergences: inline-ancestor-breaking deferred, floats unsupported, etc.
- `specs/TECH_DEBT.md`: retire `BFC-1`, `M5-MARGIN-1`, `SUB-2`, `IFC-MIXED-TEXT-INLINEBLOCK-1`.
- Demo trait doc (`crates/rdom-showcase/src/demo.rs`): rewrite the authoring contract. Class-scoped CSS rule stays. Drop `.page` discussion. Document the "default is block, flex via `display: flex`" rule.

### Phase 9 — Grumpy review + perf characterization + close

- Grumpy chief architect review of the BFC-1 implementation. Findings logged + closed before milestone closure.
- Perf bench: confirm block layout ≤ flex cost on equivalent content.
- Anonymous box allocation cost: characterized. Switch to thread-local arena if hot.
- STATE.md updated with milestone close + decisions log.

---

## Migration breakage (intentional)

Any existing rdom consumer that relies on the implicit "every container is column-flex" default will see layout changes. Required migration:

- Containers with `flex-direction`, `flex-grow`/`flex-shrink`, `gap` (meant as flex gap), or relying on `flex: 1` children for fill → add `display: flex` to the container.
- Pure block content → no change.

Since rdom has no external consumers yet, this milestone ships the break without a deprecation period.

## Out of scope (do not roll in)

- `display: grid` and grid layout.
- `display: list-item` + `::marker` + counters. Tracked as `LIST-ITEM-1` for later.
- `text-align` / `vertical-align`. Tracked as `TEXT-ALIGN-1`.
- `<table>` and table-internal display values.
- CSS3 Display two-value syntax (`display: block flow`, etc.).
- Inline-ancestor breaking around blocks (CSS 2.1 §9.2.1.1 second half).
- Floats and `clear` (parse-recognize + warn only).
- `@media` queries.
- `@keyframes` / animation properties.
