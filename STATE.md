# STATE — project journal

Living progress ledger for `rdom`. Updated whenever a change records a meaningful decision, completes a milestone, opens a risk, or shifts direction.

For the durable architecture and roadmap, see [`specs/DESIGN.md`](specs/DESIGN.md). For the current major project plan (0.2.0 = `rdom-showcase` + event bundle + `calc()`), see [`specs/SHOWCASE.md`](specs/SHOWCASE.md). For accepted tech debt, see [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md).

## Current focus

**In flight:** `BORDER-MODEL-1` — border model refactor. Replaces the `border-collapse` heuristic stack with a layered model: layout owns positioning (direct-children-only collapse, gap-sacred), paint owns visual merging (per-direction style/color/priority, CSS Tables 3 §11.5 conflict resolution). Adds `BorderStyle` enum with `hidden` kill-switch. Makes `border-collapse` non-inheriting. Closes `M5-COLLAPSE-1`, `M5.5b-CELL-OWNERSHIP-1`, `BFC1-CODE-COLLAPSE-INSETS-1`. Branch: `border-model`. Plan + progress below under "BORDER-MODEL-1 initiative log."

**Just closed:** `BFC-1` — Block Formatting Context substrate. Semantic HTML now stacks per the web platform: `<div><h1><p></p></div>` produces a block-flow column at intrinsic heights with no CSS at all. CSS 2.1 §10 normal flow + §8.3.1 margin collapse + §10.5/§10.6.3 height resolution + CSS3 `gap` for block + atomic inline-block in IFC, all on top of the original flex pass. Plan: [`specs/BFC-1.md`](specs/BFC-1.md). Tasks #70–#78 + #80–#95 closed. See "2026-05-26 — BFC-1 closed" below for the full landing.

**Release in flight:** 0.2.0. Workstreams: `rdom-showcase`, event surface bundle, `calc()` value system, now BFC-1. Plan: [`specs/SHOWCASE.md`](specs/SHOWCASE.md).

**After BFC-1:** M8 — Coverage demos (the showcase becomes a complete tour of the substrate). Currently partially shipped (4 demos in animations, 3 in text + others); resumption blocked until BFC-1 closes the textual-content authoring gap.

**Status:** **M1 + M2 + M3 + M4 + M5 + M6 + M7 closed; M8 in flight.** M7 polished the showcase consumer-side: Source view tab (Demo / Source toggle), terminal resize integration, scroll-position indicator at the bottom of `<main>`, CLI deep-link via `--demo <slug>` / `--list` / `--help`. M8 (Coverage demos) is underway — first four demos shipped (MutationObserver + Animations: transition / interval / rAF), plus a substrate fix for inline-block in flex rows surfaced by the interval-counter demo. 2,481 workspace tests passing.

The seven fixes (in order of discovery):
1. `class` attribute ↔ `classList` round-trip per WHATWG (commit `a92aa6a`)
2. `%` units as first-class CSS sizing (commit `0b363db`)
3. Nested `border-collapse` + content-bearing children get proper content inset (commit `5b699c2`)
4. Off-viewport mask bits filtered at paint-side instead of joiner-side (commit `78e5060`) — architectural cleanup of the M2-D2-review patch
5. CSS `flex: <n>` shorthand parses + applies (commit `fdab1bb`)
6. Transparent intermediate container propagation for `border-collapse` sibling overlap (commit `29765b5`)
7. CSS-default `flex-shrink: 1` so `height: 100%` shrinks to fit instead of overflowing (commit `afed656`)

Plus `4b79b05` updates the showcase chrome to use canonical `flex: 1` for "fill remaining" — the modern CSS idiom that matches the layout mode.

One piece of architectural debt deferred with teeth: `EVT-DETACH-1` (implicit blur/focusout/mouseleave on detach) is tracked in [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md) and listed as a non-negotiable M5 deliverable in [`specs/SHOWCASE.md`](specs/SHOWCASE.md). M5 cannot ship `mouseleave` for explicit motion without closing this.

## 0.2.0 milestone status

- [x] **M1** — Substrate honesty *(closed 2026-05-22)*:
  - [x] D1 — Multi-slot stylesheet API (`App::push_stylesheet` / `remove_stylesheet` + `cascade_all` / `cascade_subtrees_all`). Commit `c585065`, plus grumpy-review follow-ups: `adf14be` (drain dirty tracker, not peek), `82a2dbe` (set_stylesheet returns id + empty-sheets test), `e5b4e89` (tuple-vec storage + per-pass vars merge).
  - [x] D2 — Subtree-replacement contract + integration tests. 15 contract tests under `crates/rdom-tui/tests/subtree_replacement_contract.rs`. Root-cause fix in `rdom-core::tree::detach_from_parent` adds a `purge_interaction_state_for_subtree` helper so every detach path cleans up focused/hovered/pointer_capture/selection. Commits `245c626`, `41f9f76` (review follow-ups + EVT-DETACH-1).
  - [x] D3 — Focus-on-detach specification. Folded into D2 (same fix surface): `dom.focused()` clears synchronously on detach (matches the web); the no-`blur`-event divergence documented in [`specs/DIVERGENCES.md`](specs/DIVERGENCES.md) §"Runtime & focus" and tracked as **`EVT-DETACH-1`** in [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md), blocking on M5.
- [x] **M2** — Showcase scaffold *(closed 2026-05-22)*. `crates/rdom-showcase/` workspace member (`publish = false`). Public API: `Demo` trait, `Category` enum, `Source` struct, `DEMOS` registry, `build_shell(&mut TuiDom) -> ShellHandles`, `base_stylesheet()`. Shell structure is native HTML (`<header>`/`<aside>`/`<nav>`/`<h2>`/`<main>`). One demo (`HelloWorld`) wired end-to-end. Two stylesheets registered (base + demo, slot order pinned by test) exercising M1's multi-slot API. CSS authored as strings parsed via `rdom_css::from_css` — same shape consumers learn from + same source the M7 Source tab will surface. Commits `0c25920` (scaffold), `a92aa6a` (substrate: class attribute round-trip), `c6f5d34` (review follow-ups: heading, border-collapse, dep cleanup, CSS-as-string, slot-order test).
- [x] **M3** — Sidebar nav + per-demo subtree swap *(closed 2026-05-22)*. Six deliverables shipped:
  - D1 — two additional placeholder demos (`FlexRow`, `Hover`) registered alongside `HelloWorld`, each with its own class-scoped stylesheet.
  - D2 — sidebar rebuilt as a `<details>/<summary>` category tree grouped by `Category` enum, with `<li data-demo-slug="…" tabindex="0">` items.
  - D3 — `ShowcaseState` + `mount_demo` in `crates/rdom-showcase/src/nav.rs` clears `<main>`'s children + builds + appends the new demo's subtree. Per-demo sheets are pre-pushed at App startup; class-scoped selectors mean swapping is a pure subtree replacement, no sheet churn.
  - D4 — single `click` listener on the sidebar (event delegation) walks up the target's ancestor chain for `data-demo-slug`, looks up the demo, calls `mount_demo`.
  - D5 — `keydown` listener on the sidebar: ArrowUp/ArrowDown traverse `<li>`s in document order (wraps), Enter/Space activate the focused one. Tab/Shift+Tab traversal works for free via the runtime's built-in focus router because `<li>`s carry `tabindex="0"`.
  - D6 — 7 end-to-end integration tests in `crates/rdom-showcase/tests/subtree_swap_integration.rs` exercising the M1 D2 substrate purge contract through the showcase's `mount_demo` path (focus/hover/pointer-capture/selection in detached subtree all clear; same-idx swap is a no-op; multi-swap leaves `<main>` with exactly one child; full-viewport paint after multiple swaps survives without panic).
- [x] **M4** — Examples-to-demos refactor *(closed 2026-05-23)*. All 10 in-tree examples ported to `crates/rdom-showcase/src/demos/`. Each `rdom-tui/examples/*.rs` is now a 1-line shim calling `rdom_showcase::demos::X::run_standalone()`. Paint snapshots pin all 10 outputs at fixed viewports under `crates/rdom-tui/tests/snapshots/`. `OPS-4` retired. Showcase grew from 3 (M3) to 13 demos across 7 categories. Side fix: `sticky_demo`'s pre-existing rendering bug (every Nth item missing under `overflow: auto` due to CSS-default `flex-shrink: 1`) closed by adding `flex-shrink: 0` to demo items — same pattern applied to `scrollable_list`, `tab_form`, etc.
- [ ] **M4** — Examples-to-demos refactor; closes `OPS-4` (snapshot pinning for the seven older examples). *Showcase.*
- [x] **M5** — Event surface bundle *(closed 2026-05-23)*. Six new events + the implicit-detach ceremony:
  - D1 — **`keyup`** distinguishes `KeyEventKind::Release` from Press/Repeat. App enables `KeyboardEnhancementFlags::REPORT_EVENT_TYPES` + `DISAMBIGUATE_ESCAPE_CODES` on `enter_tui_mode`; supporting terminals (kitty/foot/WezTerm/alacritty 0.13+/recent xterm) fire Release, others silently no-op.
  - D2 — **`contextmenu`** fires on right-mouse-button down at the hit target; Shift+F10 fires on the focused element. Cancelable, bubbles.
  - D3 — **`dblclick`** synthesized on the second click of a 2-click sequence, dispatched after the regular click. Triple-click is selection-gesture territory.
  - D4 — **`resize`** dispatches on the document root (Window target per HTML §UIEvents) when `CtEvent::Resize` fires. Coalesced per crossterm signal.
  - D5 — **`scroll`** dispatches on elements whose `scroll_x`/`scroll_y` actually changed. Three mutation sites wired: wheel scroll, scrollbar drag, programmatic `set_scroll_*`/`scroll_to`. No event at-rail-end wheel ticks.
  - D6 — **Implicit-detach event ceremony** (closes `EVT-DETACH-1`). New `Mutation::PreDetach` variant fires BEFORE structural unlink. `runtime::implicit_events` module's App-level observer dispatches `blur` + `focusout` on focus loss, `mouseout` + `mouseleave` on hover loss. Tree intact at dispatch → bubbling works through live ancestor chain. 8 integration tests pin the contract. Two `DIVERGENCES.md` entries removed.
- [x] **M6** — `calc()` value system *(closed 2026-05-24)*. End-to-end shipped:
  - **Phase 1** — `CalcExpr` AST + recursive-descent parser. CSS-correct precedence (`+ -` < `* /`), parens, unary minus, nested `calc()`. Banker's-rounding resolver. Substrate types: `Size::Calc(Box<CalcExpr>)`, `Length::Calc(Box<CalcExpr>)` — `Size` / `Length` non-Copy, `.clone()` at move boundaries. `PresentationStyle::Eq` derive removed; `AnimatedValue` non-Copy.
  - **Phase 2** — Layout-time resolution: `apply_relative_shift` resolves `top`/`bottom` against parent height, `left`/`right` against parent width. `axis_size_from_edges` / `axis_position_anchored` / `axis_position_relative_shift` take `&Length` and resolve Calc via shared `length_to_cells` helper. `compute_placed_rect` (absolute positioning) resolves Size + Length Calc against the containing block. `compute_pseudo_layout_rect` (positioned pseudos) follows the same pattern. `layout_flex_children` resolves main-axis Calc against `main_budget` and cross-axis against `container_cross`.
  - **Phase 3** — `parse_unsigned` and `parse_padding_shorthand` accept constant-only `calc()` (e.g. `padding: calc(2 * 3)` → 6 cells). Percent-bearing calc on padding/margin/gap is rejected — narrow gap tracked as `CALC-PADMARG-1`.
  - **Phase 4** — End-to-end integration tests in `crates/rdom-tui/tests/calc_layout.rs` (10 tests): width / height / top / left / nested calc / negative-clamp / absolute positioning / relative shift / constant padding / paint-pipeline survival.
  - **Animation**: Calc-bearing transitions snap at midpoint (no layout context at interpolation time). Documented in DIVERGENCES.md.
- [x] **M7** — Showcase polish *(closed 2026-05-24)*. Four deliverables:
  - D1 — **Source view tab** in `<main>`. Demo / Source toggle; Source mounts a `<div class="source-view">` with the demo's `MARKUP` + `CSS` strings rendered into two `<pre>` blocks with `<h2>` labels. `ShowcaseState` gains `view: ViewMode`; switching demos auto-resets to Demo view; `.active` class flips between tabs.
  - D2 — **resize integration verified.** Substrate already wires resize (M5 D4); 3 integration tests pin that the showcase chrome adapts (main panel grows/shrinks) and that listeners on the document root see one resize event per crossterm signal.
  - D3 — **Scroll-position indicator** at the bottom of `<main>`. Empty when no scrollable element is in play; populates with "Row N/M — P%" on any `scroll` event from a descendant. Wired via `wire_scroll_indicator` listening on `dom.root()`.
  - D4 — **CLI deep-link:** `cargo run -p rdom-showcase -- --demo <slug>` opens directly to a named demo; `--list` prints every registered slug + title; `--help` prints usage. 7 unit tests cover `parse_args`.
- [ ] **M8** — Coverage demos (one per primitive in §0.1.0 + every new 0.2.0 addition). *Showcase.*
- [ ] **M9** — CI + snapshots + README + DESIGN.md decision archive + per-crate version bumps + `cargo publish` → **0.2.0 ships**.

## Semver release track

- [x] **0.1.0** — Initial release (2026-05-19): DOM substrate, cascade, flexbox, runtime, native built-ins, UA stylesheet, CSS parser, HTML parser.
- [x] **0.1.0 editing parity** (2026-05-20): selection, caret, contenteditable parity.
- [ ] **0.2.0** — In flight. `rdom-showcase` (headline) + event surface bundle + `calc()` value system. See [`specs/SHOWCASE.md`](specs/SHOWCASE.md).
- [ ] **0.3.0** — Client-side routing primitive.
- [ ] **0.4.0** — Async tasks during event handlers.

## Open risks

- **`EVT-DETACH-1`** — implicit `blur` / `focusout` / `mouseleave` / `mouseout` not dispatched on detach. Documented in [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md) as a non-negotiable M5 deliverable. Risk: if M5 scope grows and this slips, rdom-tui ships an internally inconsistent hover-event model. Mitigation: M5 exit criteria in [`specs/SHOWCASE.md`](specs/SHOWCASE.md) explicitly require closing `EVT-DETACH-1` + deleting the related DIVERGENCES.md entries.

## Recent decisions

### 2026-05-27 — BORDER-MODEL-1 initiative log

**Goal:** replace `border-collapse`'s heuristic stack with a layered, browser-faithful model so CSS authors get expected results without surprising substrate side effects.

**Architectural decisions:**
- **Layout vs paint split.** Layout positions borders (collapse → direct-children overlap by 1, gap honored verbatim). Paint composites per-direction `(BorderStyle, Color, priority)` and emits the winning glyph + color per CSS Tables 3 §11.5.
- **`border-collapse` is non-inheriting.** rdom divergence from CSS, documented in `DIVERGENCES.md`. Containers that want their direct children to participate declare collapse themselves. No subtree contamination.
- **Conflict resolution adopts CSS Tables 3 §11.5 wholesale.** Hidden kill-switch → none losses → style ranking → child wins ancestor → later-DOM-order wins. Winner contributes BOTH glyph and color.
- **`BorderStyle` enum** in `rdom-style` carries the full CSS keyword set; `dashed`/`dotted`/`ridge`/`outset`/`groove`/`inset` parse + rank correctly but render as `solid` (CSS-faithful degradation on terminal medium).
- **Scope:** single initiative. Block-flow collapse, flex-flow collapse, paint conflict resolution, cascade non-inheritance, BorderStyle enum, chrome migration, all dead-code removal — one branch, one push.

**Milestones:**
- [ ] M1 — Docs + contract pinning. `DIVERGENCES.md` rewritten under "Layout"; `TECH_DEBT.md` opens `BORDER-MODEL-1` and marks `M5-COLLAPSE-1` / `M5.5b-CELL-OWNERSHIP-1` / `BFC1-CODE-COLLAPSE-INSETS-1` as resolved by it; this log opened.
- [ ] M2 — Failing test scaffold pinning every contract row (4 layout outcomes, non-inheritance, hidden kill-switch, style ranking, color winner, DOM-order tiebreak).
- [ ] M3 — `BorderStyle` enum + parsing; replace per-side bool in `Border`; migrate every call site.
- [ ] M4 — `border-collapse` becomes non-inheriting.
- [ ] M5 — Flex layout: remove transparent-intermediate recursion; direct-children-only; `gap > 0` honored.
- [ ] M6 — Block-flow mirror of M5's rules.
- [ ] M7 — Paint: per-direction buffer + CSS Tables 3 §11.5 conflict-resolution algorithm.
- [ ] M8 — Showcase chrome migration to explicit per-container collapse declarations; demos audited.
- [ ] M9 — Cleanup; workspace gate green; visual smoke on every example; `BORDER-MODEL-1` closed.

### 2026-05-26 — BFC-1 closed: block-formatting-context substrate end-to-end

Shipped in 9 phases (commits `9e9af04` Phase 1 → `<phase-9-commit>` Phase 9):

- **Phase 1** — `Flow` enum (Block | Flex) cascades alongside `Display`; CSS3 Display Module two-value mapping (`display: block` → outer Block + inner Block, `display: flex` → outer Block + inner Flex). `establishes_new_bfc` cascade field gates margin-collapse + height-trap behavior.
- **Phase 2** — `layout_block_children` width formula (CSS 2.1 §10.3.3 seven-term equation), auto-margin centering with odd-cell distribution to right, min/max-width clamping.
- **Phase 3** — Anonymous block boxes (CSS 2.1 §9.2.1.1) around inline-level child runs in mixed content. Atomic `Display::InlineBlock` packing in IFC per CSS 2.1 §10.8 (Phase 3.5b). Hit-test, paint, selection, drag-routing all walk anon boxes.
- **Phase 4** — Dispatch wired in `flex::layout_children`: IFC → pure-text-leaf → `match Flow`. UA sweep added explicit `flow: Flex` to `<tr>`. Showcase chrome migration + 49-file demo sweep + snapshot regeneration. Substrate fixes uncovered during the sweep: block.rs passed wrong axis budget to intrinsic measurement (now `resolved_width` for cross-budget), block lacked border-collapse parent-edge inset (shared `flex::collapse_parent_edge_insets`), block lacked `parent_scroll` offset, atomic inline-block fragments didn't get layout rects (now `layout_atomic_inline_blocks` writes them in both anon-box and singular-IFC paths), `selection::drag::begin` engaged for atomic-inline-block sentinel positions (now gated on `node_type() == Text`).
- **Phase 5** — Full CSS 2.1 §8.3.1: adjacent-sibling collapse via `MarginAccumulator` (`max(positives) + min(negatives)`), parent-first/last-child collapse with new-BFC/padding/border blockers, empty-block collapse-through, full upward propagation via `accumulate_outer_top_margin` / `accumulate_outer_bottom_margin` (the chain walk that the grandparent uses, not just local suppression).
- **Phase 6** — CSS 2.1 §10.5 + §10.6.3 height. `BlockMeasurement` returned from block-flow dispatch only (`Option<BlockMeasurement>` after Phase 9 review); `layout_node` applies margin-collapse-aware content extent for `Auto` height (gated on parent's flow being Block, not flex, and element not being out-of-flow positioned). Percent height walks `nearest_block_ancestor_height_is_definite` (iterative after Phase 9 review). CSS3 Box Alignment Module `row-gap` between block-level element children.
- **Phase 7** — 62 named block-flow tests in `block_tests.rs`, snapshot audit of all 17 .snap files.
- **Phase 8** — DESIGN.md "Layout passes" section, DIVERGENCES.md retired the BFC-1-obsoleted entries (anonymous-inline-boxes / inline-block-disqualifies-IFC) and added explicit no-floats entry, TECH_DEBT.md retired `BFC-1` + `SUB-2` + `IFC-MIXED-TEXT-INLINEBLOCK-1`, Demo trait gained "Default flow is block; opt into flex with `display: flex`" authoring rule.
- **Phase 9** — Grumpy chief architect review surfaced 5 blocking findings + 7 non-blocking + 5 test-coverage gaps. Three blocking items addressed in this commit: iterative `nearest_block_ancestor_height_is_definite` (closes the unbounded-recursion concern), `layout_children` returns `Option<BlockMeasurement>` (only the block path produces one — moves the "valid for Block flow only" invariant into the type), explicit regression tests for vertical-auto-margin (resolves to 0 per §8.3) and anon-block-box-margin-boundary. Perf bench `benches/block_layout.rs` confirms block layout is on par with flex on equivalent content (254µs vs 253µs on a 100-paragraph document); margin-collapse-deep chains complete in 14µs. Remaining grumpy findings filed as `BFC1-PERF-MARGIN-CHAIN-1`, `BFC1-PERF-INLINE-FLOW-LOOKUP-1`, `BFC1-CODE-BLOCK-SPLIT-1`, `BFC1-CODE-ATOMIC-IB-DUP-1`, `BFC1-CODE-COLLAPSE-INSETS-1`, `BFC1-MARGIN-PERCENT-1`, `BFC1-AUTO-HEIGHT-ORDERING-1` in TECH_DEBT.md with characterization for each.

Workspace state: 2574 tests pass; clippy + fmt clean across the workspace. Block layout fully matches the relevant CSS specs (§9, §10, §17.6.3, Box Alignment Module) within the rdom TUI substrate scope. Authors can write semantic HTML now and get web-platform behavior.

### 2026-05-25 — Substrate fix: M5-MIN-CONTENT-1 retired (flex items default to content-min floor)

User-reported sidebar nav UX bug surfaced the deeper substrate gap that the project had been documenting for milestones: CSS-default `flex-shrink: 1` plus no `min-height: auto` floor let the Bresenham allocator squish flex items to zero cells when the container overflowed. The original symptom was "every-other-row highlight disappears" in the showcase sidebar; the actual cause was 0-height items stacking under their visible siblings.

The first fix attempt was chrome-side workarounds (`flex-shrink: 0` everywhere). Grumpy architect review correctly called that out: "App should work OOTB if you create it like any HTML app — this is our most important contract." Reverted the chrome workaround. Implemented CSS Flexbox §4.5 in the flex layout:

- Every flex item now has an auto-min floor along the main axis: `min(content_size_suggestion, specified_size_suggestion)`, dropped to 0 when overflow on the axis is non-visible. Items can no longer silently vanish in overflowing containers.
- `intrinsic_size` got a sibling `content_min_size` (skips the `Size::Fixed` short-circuit) so the content size suggestion measures actual content, not declared box size — `<a style="width:100; max-width:30">` with no children correctly resolves auto-min to 0 and max-width clamps to 30.
- `Size::Flex(_)` (the `flex: <N>` shorthand) maps to `specified_suggestion = 0`, matching CSS's `flex-basis: 0%` default. So `flex: 1` items still shrink freely (chrome wants this for fitting panes); authors who want content-protection opt in via explicit `min-*: auto`.
- Explicit `min-*: auto` is **more protective than CSS strict** (documented divergence in DIVERGENCES.md): always equals content size suggestion, regardless of specified cap. Gives authors a single-property "protect my content" without computing intrinsic themselves.

Chrome opts: `.app`, `.app-body`, `.main` now declare `min-width: 0` / `min-height: 0` to participate as fit-the-viewport shells — web-faithful (the same opt-in real CSS authors use for app-shell flex panes).

`autofocus` on the first sidebar `<li>` rounds out the OOTB UX so the app boots keyboard-navigable.

Tests: new `keyboard_nav.rs` (3 tests pinning autofocus + ArrowDown advancement + no-zero-height-squish), updated `parse_and_render` snapshot (cards now show all their content as they should). 2,487 workspace tests pass.

Follow-up: `SHRINK-CLEANUP-1` in TECH_DEBT — remove the ~60 `flex-shrink: 0` declarations across showcase demos. They're now no-ops but mislead future authors.

### 2026-05-24 — Substrate fix: inline-block in flex row paints UA pseudos

M8 demo `interval_counter` surfaced a substrate gap: `<button>` (inline-block) + `<span>` (inline) siblings in a flex row rendered as `Start0` — the button's UA `[ … ]` bracket pseudos were silently dropped. Root cause: `is_ifc_block` (`crates/rdom-tui/src/render/layout_pass/ifc.rs`) routed any container with an inline-element child through the IFC paint path, which doesn't synthesize pseudo fragments for inline-block children. The original behavior was deliberate ("inline-block doesn't flip the parent into IFC mode") but ignored *sibling inline elements* flipping it — exactly the failing case.

**Fix:** `Display::InlineBlock` now disqualifies the parent from IFC even with inline-element siblings. The container falls through to flex layout, where the inline-block child gets a proper rect and renders with full pseudo chrome. Closer to CSS Flexbox §3 step 7 (flex containers blockify their children) than the previous IFC opt-in. The companion paint-side change loosens `recurse_children` so a `display: inline` child *with its own `inline_layout`* (the new flex-laid case) paints normally instead of being swept into the legacy "cascade error" suppression bucket.

**Tests:** new `crates/rdom-tui/tests/button_flex_repro.rs` — four regression assertions (button alone, button + inline sibling, two buttons, plain inline-only IFC negative case). 2,481 workspace tests pass.

**Documentation:**
- `specs/DIVERGENCES.md` — new entry under §Layout calling out the inline-block ↔ IFC behavior and pointing at the residual gap.
- `specs/TECH_DEBT.md` — `BUTTON-FLEX-ROW-1` retired; the narrower remainder (`IFC-MIXED-TEXT-INLINEBLOCK-1`: mixed raw-text + inline-block in the same container) tracked as a separate, smaller debt with a workaround (wrap text in `<span>`).

The demo (`crates/rdom-showcase/src/demos/interval_counter.rs`) keeps its stacked counter layout — `<button>` then `<p>Counter: <span>0</span></p>` — which reads cleaner than the original button-and-value-in-a-row pattern. The substrate fix means either layout works now; the choice is now UX, not workaround.

### 2026-05-24 — M6 closed: calc() value system end-to-end

Shipped the full `calc()` value system. Width / height / top / right / bottom / left support layout-time percentage-bearing calc through cascade → layout → paint. Padding / margin / gap support constant-only calc (narrow gap tracked as `CALC-PADMARG-1`).

**Initial scope reduction was the wrong call.** The first M6 attempt shipped parse-time constant-eval only and deferred the layout-time work as `CALC-PCT-1`. Grumpy review (correctly) flagged that as accumulated debt + "scope reduction" framing burying real work. Reopened M6, powered through the refactor.

**What the refactor actually entailed:**
- `Size` + `Length` get `Calc(Box<CalcExpr>)` variants — both lose `Copy` / `Eq`, gain Clone-only semantics. The `.clone()` is O(1) for the simple variants; only `Calc` walks an AST.
- 48 compile errors after the variant addition, deduping to ~15 unique sites. Mechanical fix per site: either `.clone()` at move boundaries OR change `match c.field` to `match &c.field` and dereference simple variants inline.
- `apply_simple` macro in `cascade/apply.rs` changed from `*x` to `x.clone()` — works for both Copy and Clone-only types (Copy's Clone impl is memcpy).
- `apply_size` / `apply_length` callers pass `parent.{field}.clone()` for inherited values.
- `ResolveCtx` threaded into layout via new helpers `length_to_cells` (Length → Option<i32>) and `resolve_size_axis` (Size → u16). Each layout site picks the correct percentage basis: width sites use parent.width, height sites use parent.height, etc.
- `apply_relative_shift` gained a parent-rect parameter — the caller (in `layout_node`) reads `parent_node().tui_ext().content_layout` and passes it through. CSS 2.1 §9.4.3: relative offsets resolve against the parent's content box.
- Animation engine snaps Calc-bearing transitions at midpoint instead of tweening. Without resolved-pixel snapshotting at transition start (which requires layout context the engine doesn't currently have), smooth interpolation is impossible. Documented divergence; resolved-value snapshotting is a polish item.
- `parse_unsigned` and `parse_padding_shorthand` accept constant-only `calc()` for the u16-backed properties. Avoiding the Padding/Margin field-type refactor in this milestone — those types stay `u16` per side. Percent-bearing calc on padding/margin requires changing those types (rippling through paint/layout/cascade reads), which would be a separate milestone.

**Tests:**
- 10 calc layout integration tests in `crates/rdom-tui/tests/calc_layout.rs` — width/height/top/left/nested/clamp/absolute/relative/constant-padding/paint-pipeline.
- 6 existing end-to-end CSS tests retained.
- 16 existing AST + parser tests retained.

**`CALC-PCT-1` retired** from TECH_DEBT.md. Replaced with the narrower `CALC-PADMARG-1` for padding/margin/gap percent-calc support — that's a clean follow-up requiring a Padding field-type change.

2,448 workspace tests passing.

### 2026-05-23 — M5 closed: event surface bundle + implicit detach ceremony

Shipped the full 0.2.0 event surface in 6 per-deliverable commits.

**Additive events (D1–D5)** were straightforward — each wires one new dispatch path through the existing 3-phase pipeline:
- `keyup` distinguishes `KeyEventKind::Release` from Press/Repeat. Enabling `KeyboardEnhancementFlags::REPORT_EVENT_TYPES` + `DISAMBIGUATE_ESCAPE_CODES` on terminal init lets kitty-protocol terminals deliver Release events; non-supporting terminals stay silent — documented as a `DIVERGENCES.md` entry.
- `contextmenu` on right-mouse-down + Shift+F10. Two entry points, one event factory.
- `dblclick` reused the router's existing `register_click` count.
- `resize` dispatches on the document root when `CtEvent::Resize` fires — single dispatch site.
- `scroll` was three mutation sites consolidated: wheel scroll in `handle_wheel`, scrollbar drag in `runtime::scrollbar::set_scroll`, programmatic API funneled through `write_scroll_clamped`. All gate on "did the offset actually change" so at-rail-end ticks don't fire spurious events.

**The architectural deliverable was D6 — implicit detach events** (`EVT-DETACH-1` closure). The challenge: when the focused / hovered element is removed from the tree, browsers dispatch `blur`/`focusout`/`mouseout`/`mouseleave` BEFORE the actual removal, so bubbling works through the still-intact ancestor chain.

The shape: new `Mutation::PreDetach { detached_root, focused, hovered }` variant in `rdom-core::observer`. `detach_from_parent` fires it BEFORE the structural unlink, but only when the focused/hovered node is actually inside the subtree being detached (cheap short-circuit otherwise). The runtime's new `runtime::implicit_events` module installs an App-level `MutationObserver` that listens for `PreDetach` and dispatches the four events via the normal `TuiDispatchExt` pipeline. Because the tree is still intact at dispatch time, normal parent_node-walking bubbling works.

This keeps `rdom-core` renderer-free — it knows about Mutation records but not about events. The event-pipeline knowledge lives entirely in `rdom-tui`. The substrate emits a "here's a hook" record; the runtime decides what to do with it.

Two `DIVERGENCES.md` entries deleted ("Implicit focus loss on detach does not fire `blur` / `focusout`" + the hover counterpart) — no longer divergent. `EVT-DETACH-1` retired from `TECH_DEBT.md`.

Coverage: 8 integration tests in `crates/rdom-tui/tests/implicit_detach_events.rs` pin the order (blur → focusout → mouseout → mouseleave), the bubbling/non-bubbling distinctions, the synthetic flag, and the negative case (unrelated detach doesn't fire). 28 new tests across M5; 2,397 total workspace tests passing.

### 2026-05-23 — M4 closed: examples-to-demos refactor

All 10 in-tree examples now live as showcase demos at
`crates/rdom-showcase/src/demos/`. The `rdom-tui/examples/*.rs`
binaries are one-line shims calling `run_standalone()`. This
collapses three previously-distinct sources of truth (example
binary, snapshot test inline DOM construction, eventual showcase
demo) into one. The snapshot tests now build via
`rdom_showcase::demos::X::build(dom)` — no chance of test/example
drift.

Per-example design pattern (the M4 canonical port):

- `const MARKUP: &str` — HTML-ish reference for the M7 Source tab.
- `const CSS: &str` — class-scoped CSS string (passes the M3
  convention test from registry.rs).
- `pub fn build(dom: &mut TuiDom) -> NodeId` — constructs the
  subtree, registers any listeners, returns the root.
- `pub fn stylesheet() -> Stylesheet` — re-parses CSS via
  `rdom_css::from_css`.
- `pub fn run_standalone() -> io::Result<()>` — standalone-example
  entry point: build a one-off App, run it.
- `pub struct X` + `impl Demo for X` — registry entry.

Required Cargo change: `rdom-tui` adds `rdom-showcase` to its
`[dev-dependencies]`. Cargo accepts the cycle because dev-deps
are separate from runtime deps. `rdom-showcase` also gained
`rdom-parser` as a direct dep (for the `parse_and_render` demo).

**Substrate fixes shaken out during the port:**

- `sticky_demo` was rendering wrong all along — every Nth item
  disappeared under `overflow: auto` due to CSS-default
  `flex-shrink: 1` (shipped in M2) shrinking `height: 1` items
  via Bresenham to zero height when content overflowed. A
  diagnostic test confirmed the bug was pre-existing (the
  original programmatic-stylesheet shape produced the same
  scrambled output). Fix is author-side: `flex-shrink: 0` on
  items inside an `overflow: auto` container — the canonical
  CSS idiom for scrollable-list patterns. Applied to
  `sticky_demo`, `scrollable_list`, `tab_form`, `selectable_text`,
  and others as appropriate.

- `parse_and_render`'s original CSS used `:root { --accent: …; }`
  custom-property declarations, which the new M3 class-scoped
  convention test (added in the post-M3 review pass) correctly
  flagged as bleeding to other demos. Moved to `.par-demo { … }`
  so the vars only cascade under the demo's subtree. CSS-correct
  for the showcase's multi-demo-sheet-pre-pushed model.

**Coverage now pinned:** 10 snapshot tests at fixed viewports
cover every shipped example. Visual regressions in cascade,
layout, paint, UA chrome, scrollbar gutter, border collapse,
sticky positioning, form chrome, parser composition, or DOM
accessor surface flag immediately. `OPS-4` retired.

### 2026-05-22 — M3 closed: interactive demo navigation

Sidebar is now a real interactive surface, not just a static label. The structural shape: `<aside class="sidebar"><nav><details open><summary>Category</summary><ul><li data-demo-slug="…" tabindex="0">…</li></ul></details>…</nav></aside>` — every element is a standard HTML primitive, no opinionated component shows up.

Two demos were added alongside `HelloWorld` (`FlexRow`, `Hover`) so the navigation actually has more than one target — picking the simplest "shows that flex works" + "shows that :hover cascade works" demos gives the user something to click between without inflating M3's scope into M8 territory (full coverage demos).

The mount mechanism is deliberately boring: clear `<main>`'s children, build the next demo's subtree, append. That's it. All the interesting work (interaction-state cleanup, mutation records, dirty-tracking) is done by the substrate via M1 D2's `purge_interaction_state_for_subtree`. The integration tests in `tests/subtree_swap_integration.rs` validate end-to-end that the substrate contract survives the trip through the showcase's actual entry point — not just the unit-level `detach_from_parent` tests.

One architectural choice worth keeping: **per-demo stylesheets are pre-pushed at App startup**, not push/popped on each swap. This avoids a re-entrancy problem (the click handler runs inside the event dispatch loop and doesn't have mutable App access) and works because every demo's CSS is class-scoped (e.g. `.flex-row-demo`, `.hover-demo`). The convention is enforced by review, not by code — but since it's only the showcase that loads multiple demo sheets at once, the convention has exactly one consumer. If we later add a demo that needs to override a chrome rule, we'll need a real push/pop API; for now we don't.

Event handling uses **single-listener delegation** for both click and keyboard — the listener sits on the sidebar, walks up from `event.target` to find the demo `<li>`. This is the same pattern web devs reach for; rdom's three-phase dispatch makes it work the same way it does on the web.

Keyboard nav: Tab/Shift+Tab traversal is free because the runtime's focus router already handles `tabindex="0"` elements. ArrowUp/ArrowDown + Enter/Space are wired explicitly because they're application-level conventions, not generic focus mechanics — the W3C ARIA tree-view authoring practice is the reference.

### 2026-05-22 — Four more substrate gaps closed: paint-side filter, `flex:`, transparent collapse, flex-shrink

After the first three substrate gaps closed (class round-trip, `%` units, nested-collapse content inset), the M2 chrome dump exposed three stray border glyphs at viewport corners — the immediate fix moved the filter to paint-side architecturally (Finding 3 / commit `78e5060`). But the dump also revealed that the header's bottom border and sidebar/main's top borders weren't sharing despite collapse — the chrome rendered with two adjacent horizontal rules instead of one. The user pushed hard on framing: rdom must work for canonical CSS in the first minute, not require rdom-specific idioms.

Three more findings emerged from that frame and all landed at root cause:

- **Finding 1 — CSS `flex: <n>` shorthand** (`fdab1bb`). The canonical "fill remaining flex space" idiom every modern CSS author reaches for. Previously dropped silently because the parser didn't know the shorthand, forcing authors to learn the rdom `1fr` syntax (CSS Grid) or write programmatic `Size::Flex(1)`. Now parses with full grammar (`flex: <n>` / `flex: auto` / `flex: none`) and sets width + height (cross-axis Flex stretches by default).

- **Finding 2 — transparent intermediate propagation for `border-collapse`** (`29765b5`). The user-observed bug: a layout `<outer border collapse> > <header border> + <body no-border> > <sidebar border> + <main border>` should share `<header>`'s bottom with `<sidebar>` / `<main>`'s tops through the transparent `<body>`. CSS tables do this natively (`<tbody>`, `<tr>` are transparent). rdom's extension of `border-collapse` to flex now propagates the same way via a recursive `has_effective_border_on_edge` helper that walks through borderless container intermediates. Unifies the concept with the content-inset path (`collapse_parent_edge_insets`) from the original M2 D2 review.

- **Finding 4 — flex-shrink for overflow** (`afed656`). `height: 100%` on a flex child alongside a fixed-size sibling previously overflowed silently. CSS-default `flex-shrink: 1` distributes overflow proportionally; rdom didn't model it. Added `flex_shrink: u16` field across TuiStyle / ComputedStyle, integrated into the flex algorithm via Bresenham-style accumulation, respecting min-* clamps. 5 pre-existing tests updated to opt non-shrinking fixed-size items into `flex-shrink: 0` (canonical CSS for scrollable-container row patterns).

Final showcase update (`4b79b05`): updated the chrome stylesheet to use `flex: 1` for "fill remaining" instead of `width: 100%`. The latter still works (CSS-correct) but causes proportional shrinking; the former is the modern canonical CSS idiom for flex layouts.

User-level frame validated: with all seven substrate gaps closed, the chrome's CSS is what a modern web developer would write — no rdom-isms, no workarounds, just canonical Flexbox. The first-minute experience now matches "browser DOM in terminal."

### 2026-05-22 — Three substrate gaps surfaced by M2 visual review, all fixed at root cause

Visual review of the rendered showcase chrome surfaced two more substrate gaps after the class-attribute fix:

1. **`%` units silently dropped** (commit `0b363db`). My `width: 100%; height: 100%` declarations were tokenized + warned + dropped because `%` was grouped with `px`/`em`/`rem`/`ch` as "non-cell units." That grouping was wrong: those four need a pixel/font-size concept the terminal grid doesn't have, but `%` is *relative to parent dimensions* — which the layout pass already knows. Fixed by adding `Token::Percentage`, `Size::Percent`, layout resolution in flex (main + cross axis) and positioned-element placement. DIVERGENCES.md updated.

2. **Nested `border-collapse: collapse` + bordered child + content-bearing grandchild** (commit `5b699c2`). The chrome's `<header>` rendered as an empty box because the `<h1>` was being positioned at the same row as the shared parent-child border, then the border glyph painted over the text. Root cause: `compute_content_area_collapsed`'s flatten behavior assumes the touching child shares a border with the parent (the table-cell model); when the child is content-bearing (no own border), its content lands on the parent's painted border row. Fixed in `layout_flex_children` with per-edge insets that distinguish "child shares a border" vs "child is content-bearing leaf" — borderless container children remain transparent (the 3-sibling-nested-grid test still passes).

User-level lesson: **when CSS that should work doesn't, the default action is to investigate the substrate, not the showcase code.** Twice in a row I papered over the symptom; the user pushed back hard and was right. Both gaps were real substrate honesty issues that would have rotted into permanent divergence if deferred.

### 2026-05-22 — M2 grumpy review surfaced class-attribute round-trip bug; fixed

Visual smoke test of the M2 binary showed the chrome wasn't rendering — every `.foo` class selector was silently failing to match. Root cause: `dom.set_attribute(node, "class", "x")` wrote the attribute string but didn't sync the `classes` BTreeSet or the per-class selector indexes that selector matching consults. The reverse direction (`add_class("x")` didn't write back to `attrs["class"]`) was also broken. The `dom_api_demo` example documented the footgun as a "use `add_class` rather than `set_attribute`" comment — a clear smell.

Per CLAUDE.md "Real Fixes Only": root-cause fix in `rdom-core::tree::attrs::set_attribute` + `add_class` / `remove_class` / `toggle_class` / `replace_class`. Two new private helpers (`sync_class_list_from_attribute_value`, `sync_class_attribute_from_class_list`) keep the three sources of truth (attrs / classes / indexes) in sync regardless of which entry point is used. Tokens iterate alphabetically per the pre-existing BTreeSet-order divergence. 7 new round-trip tests; the workaround comment in `dom_api_demo` removed.

Surfaced by visual review precisely because no automated test exercised both directions — every existing class-related test used either `set_attribute` OR `add_class`, never both. Lesson: M2's shell-structure tests should have asserted computed-style as well; pinning visual output (per M9's snapshot harness) would have caught this without a visual review.

### 2026-05-22 — M2 closed: showcase scaffold runnable end-to-end

`crates/rdom-showcase/` shipped as a workspace member, `publish = false`. The `Demo` trait + `DEMOS` registry pattern is hardcoded (no build.rs scanning, no macros). Shell layout is pure native HTML (`<header>` / `<aside>` / `<nav>` / `<main>`) + CSS via `base_stylesheet()` — zero opinionated components, holding the substrate-first invariant.

The binary `cargo run -p rdom-showcase` enters TUI mode and renders the static HelloWorld demo into the shell's `<main>`. Two stylesheets are pushed (chrome + demo), exercising M1's multi-slot stylesheet API as a real downstream consumer for the first time.

Note: visual verification of the running binary requires interactive testing — integration tests cover cascade + layout + paint against a TestBackend at both 80×24 and 20×5 (tiny-viewport regression class).

### 2026-05-22 — M1 closed; EVT-DETACH-1 deferred to M5 with teeth

Second grumpy architect pass (covering D2 + D3) found five items: one undocumented divergence (selection-collapses-to-None instead of relocating boundary points per WHATWG), one cheap perf nit, one observability note about mutation-record ordering, two missing test paths (`drop_subtree`, `replace_with`), and the headline architectural question — should implicit `blur` / `focusout` / `mouseleave` events fire on detach now or later?

User pushback: "if we don't do it now, we should have reason to postpone it forever." Counter: M5 (event surface bundle) has a forcing function and the architecturally correct shape (pre-detach ancestor path capture + new Mutation variant + App-level translation observer) is event-pipeline work, not tree-mutation work — doing it in M1 means inventing rdom-tui pipeline in rdom-core. Defer to M5 with a stable id (`EVT-DETACH-1`), an explicit non-negotiable line in [`specs/SHOWCASE.md`](specs/SHOWCASE.md) M5 scope, and exit criteria that require closing this before M5 itself closes. Defer with teeth, not defer with hope.

All five items addressed in commit `41f9f76`. M1 milestone closed.

### 2026-05-22 — M1 D2 + D3 closed: subtree-replacement contract

13 integration tests in `crates/rdom-tui/tests/subtree_replacement_contract.rs` codify the contract `rdom-showcase` (and any consumer that swaps a subtree's children) needs. Cascade reset, MutationObserver records, DirtyTracker — all already worked. The four interaction-state pointers (`focused`, `hovered`, `pointer_capture`, `selection`) leaked stale references at detach — fixed by centralizing cleanup in `rdom-core::tree::detach_from_parent` via a new private `purge_interaction_state_for_subtree` helper.

One divergence captured: implicit focus loss on detach updates `dom.focused()` synchronously (matches web) but does NOT fire a synthetic `blur` / `focusout` event. Same for `hovered`/`mouseleave` and `selection`/`selectionchange`. Documented in [`specs/DIVERGENCES.md`](specs/DIVERGENCES.md) §"Runtime & focus".

D3 (focus-on-detach spec) folds into D2 because the cleanup surface is the same — D3 is the focus axis of a four-axis contract.

### 2026-05-22 — M1 D1 grumpy architect review found dirty-tracker peek-vs-drain bug

`App::invalidate_cascade` (and the v0.1.0 `set_stylesheet` it was extracted from) called `DirtyTracker::roots_snapshot` — a peek — when the intent was to drain. Effect: when the DOM had pending dirty subtrees at the moment of `push/remove/set_stylesheet`, the next paint did a partial subtree cascade instead of the full re-cascade the API contract promises. Elements outside the dirty subtree kept stale computed styles from the previous sheet stack. Pre-existing latent bug in v0.1.0; surfaced by the M1 D1 design review because multi-sheet mutation makes the violation observable.

Regression test added (two siblings, mutate one, swap a sheet, assert the un-mutated sibling re-cascaded). One-line fix: `roots_snapshot` → `take_roots`. Commit `adf14be`.

Non-blocking findings from the same review — all addressed in the same session, leaving M1 D1 fully closed with no lingering debt:
- Parallel-vec storage (`stylesheets` + `stylesheet_ids`) — collapsed to `Vec<(StylesheetId, Stylesheet)>` in `e5b4e89`. `style_sheets()` now returns `Vec<&Stylesheet>`; cascade signature changed to `&[&Stylesheet]`; existing tests unchanged.
- `merge_root_vars` allocated per element — moved to per-pass in `e5b4e89`. `cascade_all` / `cascade_subtrees_all` compute the merged `VarMap` once, thread `&VarMap` down, per-element work is `Rc::clone`. Saves O(elements × sheets) allocations per cascade.
- `set_stylesheet` didn't return a `StylesheetId` — now returns one in `82a2dbe`, symmetric with `push_stylesheet`.
- Empty-stylesheets edge case — covered by new test in `82a2dbe`.

### 2026-05-22 — M1 D1 landed: multi-slot stylesheet API

`App::push_stylesheet` / `remove_stylesheet` / `StylesheetId` shipped, with `cascade_all` / `cascade_subtrees_all` taking `&[Stylesheet]` so the cascade is honestly multi-sheet (push order is the third tiebreaker after specificity + source_idx; vars merge across sheets with later-wins per name). Existing `CascadeExt::cascade(&Stylesheet)` kept as a one-element-slice wrapper so 80+ cascade tests didn't churn. `set_stylesheet` semantics tightened to wholesale-replace (clear + push). Commit `c585065`.

### 2026-05-22 — 0.2.0 payload expanded to bundle the showcase

Originally 0.2.0 was `calc()` + event bundle, and `rdom-showcase` was a parallel track. Folded into one release because the showcase is the largest single consumer of the new events; shipping events without a real consumer risks substrate-design choices that miss in practice; and the showcase's M1 prerequisites (multi-stylesheet, subtree-replacement contract) are substrate completion 0.2.0 wants to ship anyway. Full rationale in [`specs/SHOWCASE.md`](specs/SHOWCASE.md) decision archive.

### 2026-05-22 — `rdom-showcase` planned

Decided to build `rdom-showcase` as a permanent in-tree TUI app for dogfooding and demonstrating every rdom primitive. Plan committed at [`specs/SHOWCASE.md`](specs/SHOWCASE.md).

Durable decisions in the plan's decision archive:
- Named `rdom-showcase`, not `rdom-storybook` (avoids React component-model expectations).
- Lives in-tree at `crates/rdom-showcase/` with `publish = false` (keeps CI signal, keeps public API surface minimal).
- Substrate work blocks showcase work, not the reverse (M1 lands before any showcase code).
- Demos rebuild on nav, not hidden-and-restored (harder substrate test, matches gallery intuition, bounded memory).

### 2026-05-22 — Grumpy chief architect pass on the showcase idea

Pre-implementation review identified three blocking substrate findings: stylesheets are single-slot (not honest for "shell + per-demo CSS"); subtree replacement is not tested as a contract; focus disposition on detach is unspecified. All three become M1 deliverables.

### 2026-05-20 — Editing parity shipped

`feat: editing parity for 0.1.0 — selection, caret, contenteditable` (c4b4eba). Editing tech debt items `EDIT-1` (cross-node undo) and `EDIT-2` (`user-select: contain` clamp) tracked in [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md).

### 2026-05-19 — 0.1.0 initial release

Five workspace crates published. Architectural decisions from M1–M5 internal milestones archived in [`specs/DESIGN.md`](specs/DESIGN.md#decision-archive).

## Decision archive

Older decisions worth preserving past their immediate context. New decisions land in §Recent decisions; rotate down here when they're no longer "recent."

(empty — will populate as 0.2.0 milestones land)
