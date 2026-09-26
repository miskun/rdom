# STABILIZE-2026-09 — pay down every open debt item before 0.5.0

**Status:** IN PROGRESS (started 2026-09-23, the day after 0.4.0 shipped).

**Goal.** The next release, **0.5.0**, is the "stable and complete" line: it ships with an empty
`## Open` section in [`TECH_DEBT.md`](TECH_DEBT.md). Every item there is handled one of three ways,
and nothing is left as "later":

- **fix** — implement the web behavior (or the real refactor), TDD, in the crate that owns it;
- **divergence** — the behavior is a deliberate, permanent departure: it moves to
  [`DIVERGENCES.md`](DIVERGENCES.md) with its reason, and stops being called debt;
- **external** — the cause is outside rdom (a terminal emulator bug); it is documented under
  "Known limitations" in `DIVERGENCES.md` and the workaround is user-facing.

Routing (0.6.0) and async tasks (0.7.0) do not start before this program closes.

**Rules.** Same as [`HARDENING-2026-09.md`](HARDENING-2026-09.md): one item per commit, failing test
first, per-crate tests then the workspace gate, docs move with the code (`DIVERGENCES.md`,
`TECH_DEBT.md` row deleted in the same commit), grumpy architect + API gates at the end of each
phase, findings fixed or filed before the next phase starts. All five crates bump to 0.5.0 (the core
changes force it), so phases are ordered by dependency, not by publishability.

## 1. Triage (verified against source 2026-09-23)

Already resolved in 0.4.0 and deleted from the ledger in the program's first commit:

| Id | Why it is stale |
|---|---|
| `CSS-INHERIT-KEYWORD-1` | CSS-wide keywords ship since Batch 2 (`property_dispatch::css_wide_keyword`, `set_css_wide`). |
| `CSS-BG-SHORTHAND-1` | `background` is registered in the property table and dispatches to `bg` (color-only subset, documented in DIVERGENCES). |
| `BORDER-MODEL-1` | Landed in 0.2.0; `BorderStyle` enum, per-direction conflict resolution, no `border-model` branch exists. Its "closes" list was already deleted. |

Everything else is real. Disposition per item:

| Id | Disposition | Phase |
|---|---|---|
| `PROC-TOOLCHAIN-PIN-1` | fix — pin `channel = "1.95.0"` locally and in CI; bump deliberately | 1 |
| `PROC-TUI-DEV-DEP-1` | fix — self-contained examples in `rdom-tui`; showcase shims + snapshot tests move to `rdom-showcase` | 1 |
| `CORE-DOCPOS-ALLOC-1` | fix — allocation-free depth-walk comparison | 2 |
| `CORE-GEN-COLOCATE-1` | fix — `(generation, Option<Node>)` in one slot | 2 |
| `CORE-DROP-PANIC-LEAK-1` | fix — free under a drop guard | 2 |
| `PARSER-VOID-TAGS-1` | fix — `rdom_core::markup::VOID_TAGS` becomes the exported single source; `vr` decided | 2 |
| `PARSER-ENTITIES-1` | fix — full WHATWG named-reference table (generated, sorted static) incl. the legacy no-semicolon set | 2 |
| `STYLE-TRANSITION-VALUE-1` | fix — `Value<Vec<…>>` for the four transition longhands | 3 |
| `STYLE-PERCENT-FRACTION-1` | fix — `Size::Percent` carries the fraction; rounding happens once, at resolution | 3 |
| `CSS-WARNING-POSITION-1` | fix — token spans; declaration warnings point at the declaration | 3 |
| `STYLE-PROPERTY-TABLES-1` | fix — one `fields_of(name)` table, four derived functions, coverage test | 3 |
| `STYLE-INHERITS-TWO-SOURCES-1` | fix — `INHERITS_MASK` derived from `property_dispatch::inherits` | 3 |
| `D-M3-2` | fix — `cubic-bezier()` and `steps()` parsed | 3 |
| `CALC-GAP-1` | fix — `GapValue::{Cells, Calc}` resolved at layout | 3 |
| `SUB-4` | fix — `Length::Cells(i32)` | 3 |
| `SUB-3` | fix — UA `style { display: none }` in its section | 3 |
| `UA-CHECKBOX-INLINE-1` | fix — `inline-block`, matching `<button>` | 3 |
| `UA-SB-1` | fix — `::scrollbar-thumb:vertical` / `:horizontal` | 3 |
| `UA-OL-1` | fix — CSS counters (`counter-reset` / `counter-increment` / `counter()`), `list-item` auto counter, `ol > li::before { content: counter(list-item) ". " }` | 3 |
| `CSS-VARS-SCOPE-1` | fix — per-element custom properties across style / css / cascade | 3 + 4 |
| `CASCADE-INITIAL-ALLOC-1` | fix — hoisted initial constants + rightmost-selector rule index | 4 |
| `D-M3-3` | fix — pseudo-element transitions | 4 |
| `D-M3-5` | fix — one microtask drain per tick | 4 |
| `D-M3-6` | fix — discrete properties toggle at midpoint under `transition: all` | 4 |
| `D-M2-2` | fix — hypothetical static position for `auto` insets | 5 |
| `D-M2-3` / `D-M2-4` | fix — nested stacking contexts per CSS 2.1 Appendix E (negative z-index below in-flow content) | 5 |
| `DRY-1` / `DRY-2` | fix — one in-flow filter, one positioned-collection helper for paint and hit-test | 5 |
| `D-M5N-8` | fix — prefix shift on left-edge clip | 5 |
| `M5-MIN-CONTENT-2` | fix — strict longest-unbreakable-run min-content | 5 |
| `M5-STICKY-1` | fix — `right` / `bottom` sticky, nested scroller containing block | 5 |
| `CALC-PADMARG-1` / `BFC1-MARGIN-PERCENT-CHAIN-1` | fix — containing-block width threaded through padding / margin resolution and the collapse walkers | 5 |
| `BFC1-PERF-MARGIN-CHAIN-1` | fix — single pre-order margin pass or per-pass memo | 5 |
| `BFC1-PERF-INLINE-FLOW-LOOKUP-1` | fix — `text_node → flow` index | 5 |
| `BFC1-CODE-BLOCK-SPLIT-1` | fix — `block/{mod,margin_collapse,width,height}.rs` | 5 |
| `SCROLLBAR-AUTO-TWO-PASS-1` | fix — pass 2 re-resolves auto height and the scroll carry-back | 5 |
| `BFC1-AUTO-HEIGHT-ORDERING-1` | → DESIGN layout rule (nothing reads a pre-final auto height as a basis: relative percent insets follow §9.3.2, percent heights §10.5, sticky / scroll extents read after the pass) | 5 |
| `FLEX-BLOCK-MAIN-INTRINSIC-1` | fix — block items resolve flex base size via `max-content` | 5 |
| `FLEX-ITEM-MARGIN-MAIN-INTRINSIC-1` | fix — main-axis margins count toward the container's hypothetical main size | 5 |
| `FLEX-ITEM-NEGATIVE-MARGIN-1` | fix — `i32` placement, negative margins pull outward | 5 |
| `SCROLL-CROSS-AXIS-1` | fix — cross-axis scroll offsets and overflow cross size; horizontal autoscroll band | 5 |
| `TABLE-COLSPAN-1` | fix — `colspan` width spreading in the column-sync pass; `rowspan` is part of the `TABLE-TFC-1` divergence (rows are independent flex containers) | 5 |
| `TABLE-TFC-1` | **divergence** — tables are element-driven (`<table>` family), not `display: table` on arbitrary elements; anonymous table fixup is not implemented. Reason: no consumer needs it and it would import a second layout algorithm. Documented under Layout. | 5 |
| `FOCUS-THUMB-NEAREST-1` | fix — post-layout nearest-scroll-container marker | 5 |
| `OPACITY-1` | fix — subtree group rendering into an off-screen buffer | 6 |
| `TREE-BFC-PSEUDO-1` | fix — pseudo emitted into the first / last anonymous box | 6 |
| `PACKER-STRING-ALLOC-1` | fix — byte ranges into the source text; per-frame `(node, width)` cache | 6 |
| `PAINT-INLINE-LAYOUT-CLONE-1` | fix — borrow / `Rc` the inline layout | 6 |
| `SGR-ALLOC-1` | fix — allocation-free SGR emission | 6 |
| `CARET-REVEAL-STALE-LAYOUT-1` | fix — pending-reveal flag serviced after `layout_dom` | 6 |
| `POINTER-EVENTS-IFC-1` | fix — resolve `pointer-events` through the fragment owner chain | 6 |
| `FLEX-RS-SPLIT-1`, `SCROLLBAR-SPLIT-1`, `HIT-TEST-SPLIT-1`, `SELECT-SPLIT-1`, `APP-MOD-SPLIT-1`, `INLINE-PAINT-SPLIT-1`, `STYLE-DISPATCH-SPLIT-1`, `STYLE-VALUES-SPLIT-1` (+ `cssom/declaration.rs`, `render/buffer.rs`, `virtual_screen` behind `cfg(test)` / a `test-util` feature) | fix — split by concern, no behavior change; chrome substitution behind a runtime hook | 6 |
| `SHOWCASE-EVT-1` | fix — `AppHandle` gains stylesheet intents drained after the current event | 6 |
| `FORM-DEFAULTS-1` | fix — `defaultValue` / `defaultChecked` separate from the dirty value; reset restores | 6 |
| `EDIT-2` | fix — `user-select: contain` clamps to the nearest in-host fragment | 6 |
| `ITERM2-MOUSE-MOTION-1` | **external** — moves to DIVERGENCES "Known limitations" with the click-to-wake workaround; rdom-tui README notes it | 6 |

## 2. Phases

| Phase | Crates | Items | Status |
|---|---|---|---|
| 0 | docs | this plan; stale rows deleted | done 2026-09-23 |
| 1 | process | toolchain pin, dev-dep inversion | done 2026-09-23 |
| 2 | rdom-core, rdom-parser | 3 core + 2 parser | done 2026-09-23 |
| 3 | rdom-style, rdom-css | 12 style / css / UA items incl. counters and custom-property storage | done 2026-09-24 |
| 4 | rdom-tui cascade + animation | custom-property cascade, inherits mask, initial hoist + rule index, three animation items | done 2026-09-24 |
| 5 | rdom-tui layout | 22 layout items incl. stacking contexts, static position, cross-axis scroll, spans | done 2026-09-24 (both gates) |
| 6 | rdom-tui paint + runtime + forms | 18 items incl. group opacity, splits, form defaults, app intents; 34 gate fixes (`P6G-*`) | done 2026-09-25 (both gates + re-review) |
| 7 | completeness | scope confirmed 2026-09-26: fieldset[disabled], form owner + button overrides, constraint validation, `:focus-visible`, `::placeholder`, undo coalescing, blinking caret, clipboard whitespace, `scroll-behavior`, live `<style>`, showcase demos | |
| 8 | release | 0.5.0 across all five crates, migration notes | |

Each phase ends with the two review gates; each commit carries the item id.

## 3. Log

- 2026-09-23 — Phase 2: `drop_subtree` frees under an observer panic (`catch_unwind` → free →
  `resume_unwind`); document position / boundary points / common ancestor are a depth walk with an
  oracle test over every pair of a 40-node tree; the slot generation is colocated with the node
  (`Slot { generation, node }`, `InvariantViolation::GenerationTableMismatch` removed); HTML §13.3's
  void list is exported from `rdom-core` and imported by the parser (`vr` dropped); the parser
  decodes the full WHATWG entity table (generated `entities.rs` + `tools/gen_entities.py`, legacy
  no-`;` names, attribute caveat). Phase 1 and 2 are gated together.
- 2026-09-23 — Phase 1+2 API gate: two blockers on the entity work fixed before commit — the C1
  0x80–0x9F → Windows-1252 remap (`&#146;` is `’`) and digit-only numeric scanning (`&#65abc;` is
  `Aabc;`); DIVERGENCES "in full" claim now earned. Also: `rust-version` 1.85 → 1.88 (let-chains),
  stale `rdom-tui/Cargo.toml` tarball comment removed, examples use HTML elements and no `let _ =`
  swallows, publish checklist notes the `rdom-tui` → `rdom-parser` dev-dep pin. Slip: the entity
  generator script landed in the Phase 1 commit instead of Phase 2's.
- 2026-09-23 — Phase 1+2 architect gate: blocker — the drop-path guard covered only the
  `ChildListChanged` record; the focus / selection purge inside `detach_from_parent` fires before it.
  Fixed by snapshotting the subtree first and running detach + fire under one guard, freeing only if
  the root ended up detached (a `PreDetach` panic leaves an attached, intact subtree); the two
  `*_dropping` wrappers got the same guard. Four tests. Also: legacy prefix loop bounded by
  `LONGEST_LEGACY_NAME` (6 bytes, generated), `RefContext` enum instead of bool flags, numeric /
  named scanner edge tests, `#[rustfmt::skip]` emitted by the generator, CI channel read hardened
  against CRLF (`tr -d '\r'`, `*.toml eol=lf`), stale CI comment fixed. Decisions recorded: the full
  entity table ships unconditionally (66 KB of source, no feature flag — correctness over size); the
  `render` / `buffer_to_snapshot` test helpers are duplicated between the `rdom-tui` and
  `rdom-showcase` suites on purpose (two test targets, no shared test crate).
- 2026-09-23 — Phase 3 start: `STYLE-MASK-COLLISION-1` (FLOW → bit 41, uniqueness test);
  `STYLE-INHERITS-TWO-SOURCES-1` closed by deleting the dead `PropMask` / `INHERITS_MASK` /
  `LAYOUT_MASK` exports and pinning `inherit_inheritable_from` to `property_dispatch::inherits`
  with a test that probes all 41 properties (every inherited name must be probed).
- 2026-09-23 — `STYLE-TRANSITION-VALUE-1`: the four transition longhands carry `Value`; the
  css-wide coverage test no longer skips them; `apply_transition_lists` takes the parent.
- 2026-09-24 — `STYLE-PERCENT-FRACTION-1`: `Size::Percent(f32)` + `Size::percent_of` as the one
  rounding site (six layout call sites); whole-millisecond times stay as a narrowed divergence.
- 2026-09-24 — `STYLE-PROPERTY-TABLES-1`: `define_fields!` + `fields_of(name)`; four functions
  collapse to folds; found and fixed the stale-`flow`-after-`removeProperty("display")` bug on the
  way.
- 2026-09-24 — `CSS-WARNING-POSITION-1` (`tokenize_at` with per-token positions; declaration warnings
  and in-block tokenizer errors are absolute) and `D-M3-2` (`cubic-bezier()` / `steps()` / step
  keywords parsed, evaluated per CSS Easing 1, serialized).
- 2026-09-24 — `SUB-4` (`Length::Cells(i32)`), `CALC-GAP-1` (`GapValue`, `resolve_gap` in layout with
  the indefinite-axis rule, percent-gap layout test), `UA-CHECKBOX-INLINE-1` + `SUB-3` (inline-block
  toggles; UA section list matches the code).
- 2026-09-24 — `CSS-VARS-SCOPE-1`: `TuiStyle::custom_properties`, any-selector parsing (warning
  variant removed), cascade pre-pass folding declarations into a copy-on-write inherited map (this
  also fixes the inheritance overwrite found during mapping), CSSOM `--x` round-trip.
- 2026-09-24 — `UA-SB-1` (per-axis thumb targets, layered pseudo computation, two `TuiExt` slots) and
  `UA-OL-1` (CSS counters: data model in `rdom_style::counters`, `Content::Counter`, tree-order
  `CounterState` in the cascade with `::after` moved after the children and subtree replay, UA `<ol>`
  numbering; `ua_chrome` snapshot regenerated).
- 2026-09-24 — Phase 4: `D-M3-5` (checkpoint per task), `D-M3-6` (wrong premise: Transitions L1 never
  transitions discrete properties; `transition-property: display` is now valid + inert,
  `transition-behavior` listed as not shipped), `CASCADE-INITIAL-ALLOC-1` (constants hoisted, lazily
  built `RuleIndex` per sheet drives candidate matching), `D-M3-3` (pseudo-element paint transitions in
  their own `StyleSlot`, events carry `pseudoElement`).
- 2026-09-24 — Phase 3+4 API gate: root `README.md` had been overwritten with `rdom-css`'s README by a
  commit-split script (restored from history); `ul` / `menu` reset `list-item` per HTML §15.3.8 so a
  nested bullet list no longer advances the enclosing `<ol>`; `RuleIndex` tag keys are case-exact like
  the matcher (`DIV` rule vs `div` element); DIVERGENCES swept (stale transitions / calc-gap entries
  removed, `::scrollbar*` family framed as a WebKit-modeled extension, z-index range, input-event
  checkpoint residual, counters entry lists `start` / `reversed` / `li[value]`); three displaced doc
  comments and the `INHERITS_MASK` / `:root-only` leftovers fixed; READMEs refreshed. Deferred to
  Phase 8: reorganize CHANGELOG "Unreleased" into one Breaking / Added / Changed / Fixed block per
  crate. Ergonomics filed: `gap(impl Into<GapValue>)`, a `Content::resolve` context type.
- 2026-09-24 — Phase 3+4 architect gate: two blockers fixed — subtree cascades with counters now run
  one tree-ordered walk that cascades each dirty root as it is reached (exact after insertions in any
  root order, O(N) per frame; `CascadeExt` and `App` insertion tests) and the pseudo-element styles
  live behind `Rc` with an identity short-circuit in the transition diff (no per-frame deep clones;
  paint borrows the presentation slots). Also: pseudo-elements apply their own counter ops, `--x:
  initial | inherit | unset`, any `<custom-ident>` transition name, one rounding helper
  (`calc::round_half_to_even`), one `push_rules` writer, `caret-color` inherits (CSS UI 4), CSSOM
  `--x` stores the rendered value once, two more D-M3-5 tests, stale `AnimatableProperty` doc. New
  rows: `STYLE-DISPATCH-SPLIT-1`, `STYLE-VALUES-SPLIT-1` (Phase 6 splits). DESIGN records the
  dual-published `:root` variables as deliberate.
- 2026-09-24 — Workspace gate after the architect fixes caught a starvation bug in the new
  tree-ordered counter walk: a *detached* dirty root (the subtree a showcase demo swap removes)
  sorted first and was never reached, so no connected root behind it was cascaded — the showcase's
  stale-drag hover test hit stale layout. The walk now takes connected roots only (a detached
  subtree renders nothing); regression test `detached_dirty_root_does_not_starve_connected_roots`.
- 2026-09-24 — Phase 5, first batch: `DRY-1` / `DRY-2` (one `positioned_z_list`, `is_in_flow` in
  paint and hit-test), `D-M5N-8` (`paint_text_from` skips the clipped prefix at all four sites),
  `M5-STICKY-1` (`bottom` / `right`, calc insets), `FLEX-BLOCK-MAIN-INTRINSIC-1` (root cause was the
  UA `input` width, not flex-basis), `FLEX-ITEM-MARGIN-MAIN-INTRINSIC-1` (outer sizes in the
  container's intrinsic).
- 2026-09-24 — Phase 5: `D-M2-2` — phase-1 block, inline and flex layout record each positioned
  child's static position (`TuiExt::static_position`); phase 2 uses it for an axis with both insets
  `auto`. Flex ignores `justify-content` / `align-items` for it (DIVERGENCES).
- 2026-09-24 — Phase 5: `D-M2-3` / `D-M2-4` — `render::stacking` collects Appendix E layers per
  context with the §11.1.1 clip for each entry; paint walks them forward, hit-test backward.
  `relative` / `sticky` join the positioned layer; `opacity < 1` forms a context. Supersedes
  `positioned_z_list` (DRY-2).
- 2026-09-24 — Phase 5: `M5-MIN-CONTENT-2` — `content_min_size` measures min-content by packing
  the inline content at width 0 (the packer's own break rules), threaded through the recursion.
- 2026-09-24 — Phase 5: `CALC-PADMARG-1` + `BFC1-MARGIN-PERCENT-CHAIN-1` — `layout_node`,
  `intrinsic_size` and the margin-chain walkers take the containing-block width; the intrinsic
  recursion passes 0 on the measured axis (cyclic percentage).
- 2026-09-24 — Phase 5: `BFC1-PERF-INLINE-FLOW-LOOKUP-1` — anon-box lookup by `child_range`.
- 2026-09-24 — Phase 5: `SCROLLBAR-AUTO-TWO-PASS-1` — `resolve_auto_height` runs in both passes with
  the gutter rows; `BFC1-AUTO-HEIGHT-ORDERING-1` — recorded in DESIGN as the layout ordering rule.
- 2026-09-24 — Phase 5: `SCROLL-CROSS-AXIS-1` — cross-axis scroll in flex and block, descendant
  scrollable overflow, horizontal autoscroll band, `nearest_scroll_container` shared by keys /
  autoscroll.
- 2026-09-24 — Phase 5: `FOCUS-THUMB-NEAREST-1` — `App::mark_scroll_focus` keeps
  `data-rdom-scroll-focus` on the keyboard's scroll target; the UA rule matches the attribute.
- 2026-09-24 — Phase 5: `FLEX-ITEM-NEGATIVE-MARGIN-1` — signed margin math in the flex budget,
  placement and cross sizing.
- 2026-09-24 — Phase 5: `TABLE-COLSPAN-1` — `colspan` in `size_columns`; `TABLE-TFC-1` →
  DIVERGENCES (Layout: table model, `rowspan` included).
- 2026-09-24 — Phase 5: `BFC1-PERF-MARGIN-CHAIN-1` — per-pass chain memo on `TuiExt`, cleared by
  `layout_node`; 60-level bench shape.
- 2026-09-24 — Phase 5: `BFC1-CODE-BLOCK-SPLIT-1` — `block/{mod,margin_collapse,width,height}.rs`.
- 2026-09-24 — Phase 5 API gate (blocking, all fixed): four DIVERGENCES entries and the README
  positioning line had gone stale (autoscroll vertical-only, `min-width: auto` natural size,
  calc padding constant-only, `:focus-within::scrollbar-thumb`, flat stacking); the vertical-wins
  autoscroll precedence and the `opacity`-context clip approximation are now recorded; an
  inline-block atom's percent padding resolves against the IFC width instead of 0 (code fix +
  test); the permission-dialog demo docs no longer cite retired debt rows. Non-blocking, done:
  `margin_chain` / `MarginChainMemo` are crate-internal (pass-local scratch, not layout output);
  `SCROLL_FOCUS_ATTR` and the tree's `ACTIVE_ATTR` are public and the runtime-written attributes
  are a DIVERGENCES entry; the two UA changes moved to the `rdom-style` changelog section with the
  `compute_content_area_collapsed` migration clause; empty TECH_DEBT headings and stale "z-list"
  comments removed. Declined: a footer in the sticky demo (test-covered; demo stays minimal).
- 2026-09-24 — Phase 5 architect gate (blocking, both fixed with tests): B1 — the classic-scrollbar
  trigger compared the content against the pre-layout height estimate, so an `auto`-height
  `overflow-y: auto` block reserved a phantom column; it now compares against the final height, and
  intrinsic sizing counts a block's direct text runs next to element children. B2 — hits inside
  positioned boxes (now `relative` / `sticky` too) lost their ancestors from `hit_test_path`; layer
  hits insert the chain between the context root and the entry, and nested roots insert themselves.
  Non-blocking, done: relative percent `top` / `bottom` are `auto` under an indefinite containing
  block (§9.3.2; resolves the DESIGN ordering rule's own contradiction); static positions use the
  scrolled origin on both axes; a positioned inline-block atom paints from the layer only; the
  in-flow predicate copies are gone and the two collapse-through predicates and the two chain
  walkers are one each; the dead `margin_right` field went; the memo invariant is documented and
  checked in debug builds at the end of `layout_dom`; the scrollable-overflow walk no longer
  allocates per node and has a bench shape; the glued / stale doc comments are fixed; tests for the
  clipping-descendant stop, the fixed-in-nested-context clip, colspan over author-sized columns, a
  removed scroll-focus carrier and a transparent context root. Recorded as divergence:
  `elements_from_point` returns the ancestor chain. Not done: the `opacity`-context clip is a
  DIVERGENCES entry (API gate), not a fix.
- 2026-09-24 — Phase 6: `ITERM2-MOUSE-MOTION-1` → DIVERGENCES §4 (external: the terminal commits to
  motion tracking only on a real mouse event) + rdom-tui README "Terminal notes".
- 2026-09-24 — Phase 6: `SGR-ALLOC-1` — const modifier-code table, no `Vec` per cell.
- 2026-09-24 — Phase 6: `PAINT-INLINE-LAYOUT-CLONE-1` — paint borrows through `tui_ext()`'s
  `'a` lifetime.
- 2026-09-24 — Phase 6: `PACKER-STRING-ALLOC-1` — `PendingGrapheme<'a>` borrows the source text.
  Not done from that row: a per-frame `(node, width)` layout cache — the intrinsic measure and the
  layout pack the same text twice per frame by design (measure-then-place); a cache keyed on the
  previous frame's layout would go stale on content edits, so the double pack stays and is cheap
  now that a pack allocates per fragment only.
- 2026-09-24 — Phase 6: `POINTER-EVENTS-IFC-1` — fragment owner resolved through the
  `pointer-events` chain up to the block.
- 2026-09-24 — Phase 6: `SHOWCASE-EVT-1` — `AppContext` stylesheet intents, drained after the tick /
  injection; listeners use `AppHandle::inject`. The showcase keeps its pre-pushed sheets (a demo
  switch through intents is a showcase change, not substrate).
- 2026-09-24 — Phase 6: `TREE-BFC-PSEUDO-1` — `paint_inline_layout` owns pseudo emission for text
  leaves, IFCs and anonymous boxes; block-first / block-last hosts recorded in DIVERGENCES.
- 2026-09-24 — Phase 6: `CARET-REVEAL-STALE-LAYOUT-1` — `caret_reveal_pending` on the container,
  `service_caret_reveal` after `layout_dom` (a second layout only when the offset moved). Armed on
  `overflow-y`, not on the stale extent — the first overflowing line was otherwise never revealed.
- 2026-09-24 — Phase 6: `EDIT-2` — root cause was the host lookup: `user-select` inherits, so the
  nearest `contain` / `all` ancestor was the paragraph, not the host (`host_with` climbs the run).
  Contain clamp via `nearest_inline_target_in_subtree` + `resolve_in_target`; the edge rule stays
  as the no-flow fallback.
- 2026-09-24 — Phase 6: `FORM-DEFAULTS-1` — defaults captured on seed / first change, reset
  restores; the attributes stay live on purpose (`:checked` is attribute-matched in rdom-core; a
  separate live state would need a core hook for one selector). Found on the way: a toggle beside
  prose lost its click to the empty-space selection snap — toggles / range are now `user-select:
  none` in the UA sheet, like buttons.
- 2026-09-24 — Phase 6: `OPACITY-1` — group rendering (`Buffer::composite_group`); the per-write
  compose context and its `parent_bg` fallback are gone. Recorded in DESIGN.
- 2026-09-24 — Phase 6: `FLEX-RS-SPLIT-1` — `flex/{mod,main_axis,cross,placement,collapse}.rs`; the gap==0 && collapse overlap gate now lives
  in one predicate.
- 2026-09-24 — Phase 6: `SCROLLBAR-SPLIT-1` — `scrollbar/{hit,drag,keys,autoscroll,reveal,geometry,scroll}.rs`; largest piece 166
  lines; `nearest_scroll_container` narrowed to the module (no crate callers).
- 2026-09-24 — Phase 6: `HIT-TEST-SPLIT-1` — `hit_test/{descend,nearest,fragment}.rs` behind the unchanged `HitTestExt` surface.
- 2026-09-24 — Phase 6: `SELECT-SPLIT-1` — `select/{model,state,click,keyboard,typeahead,dropdown}.rs`; stale module doc corrected.
- 2026-09-24 — Phase 6: `APP-MOD-SPLIT-1` — `app/{stylesheets,keyboard_defaults,autoscroll,frame}.rs`; `mod.rs` 1423 → 734 lines,
  public API unchanged.
- 2026-09-24 — Phase 6: `INLINE-PAINT-SPLIT-1` — `inline_paint/{single_row,caret,selection_overlay,chrome}.rs`; chrome substitution behind
  `ChromeText` / `InlineChromeFn` (`builtins::inline_chrome::lookup`). Compiled-in supplier
  table, not per-`Dom` registration: hand-driven `paint_dom` consumers and `paint_pass/tests.rs`
  paint chrome without an `App`; a per-`Dom` slot would be a separate, behavior-changing item.
- 2026-09-24 — Phase 6: `STYLE-DISPATCH-SPLIT-1` — `property_dispatch/{table,css_wide,set,serialize,value_serializers}.rs`; public paths
  unchanged.
- 2026-09-24 — Phase 6: `STYLE-VALUES-SPLIT-1` — `parse/values/` one file per value family. The row's "also over the bar" list moves to
  its own row, `OVERSIZED-FILES-1`, for the Phase 6 architect gate to triage.
- 2026-09-24 — Phase 6: `DECLARATION-SPLIT-1` — `cssom/declaration.rs` → `cssom/declaration/{mod,error,read,write,serialize}.rs` + `tests/{mod,read,write,css_text}.rs`; public paths unchanged; rdom-tui lib tests 1661 → 1661.
- 2026-09-24 — Phase 6: `BUFFER-SPLIT-1` — `render/buffer.rs` → `render/buffer/{mod,border,write,composite,diff,tests}.rs`; public paths unchanged; rdom-tui lib tests 1661 → 1661.
- 2026-09-24 — Phase 6: `VIRTUAL-SCREEN-TEST-UTIL-1` — `VirtualScreen` behind `#[cfg(any(test, feature = "test-util"))]`; `virtual_screen/{mod,parse}.rs` + `tests/{mod,parser,terminal}.rs`; `tests/inline_flow.rs` is its own `[[test]]` with `required-features = ["test-util"]`, enabled in the workspace run by `rdom-showcase`'s dev-dependency; lib tests 1661 → 1661, integration 191 → 163 + 28 (`inline_flow`).
- 2026-09-24 — Phase 6 API gate (3 blocking): B1 — the UA `user-select: none` on toggles / range
  and `user-select` inheriting (CSS UI 4: not inherited; nested `contain` merges into the outer host)
  are undocumented divergences; B2 — CHANGELOG files new API, a breaking change and an rdom-style UA
  change under "Fixed", and `TuiAccessors` gained required methods unflagged; B3 — `<form>` reset
  skips `<select>` and range. Non-blocking: selection-drag capture retargets clicks (root cause of
  the toggle patch), block-first `::before` gap (`ol > li > p` shows no numbers), stale comments,
  `test-util` invisible to consumers, no default setters, showcase not migrated to the intents, no
  demo exercises the new features.
- 2026-09-24 — Phase 6 architect gate (3 blocking): B1 — group opacity misses borders (joiner runs
  after compositing at full fg) and `Reset` fg, and an `opacity: 0` glyph erases the backdrop; B2 —
  `composite_group` splits wide-glyph pairs; B3 — the toggle `user-select` rule is a symptom patch:
  the selection drag takes DOM-visible pointer capture and retargets `click`. Non-blocking: backdrop
  glyphs untinted and links inherited under a translucent box, full-screen layer cost, form reset
  gaps, nested `contain`, duplicated frame pipeline, stylesheet ids allocated twice, `::before`
  shift paint-only (packer / hit-test / caret disagree), `render/` helpers living in `runtime/`,
  duplicate `cells_before_byte`, `test-util` coverage depending on the showcase. OVERSIZED-FILES-1:
  split `layout.rs` (geometry out of the style crate), move `stylesheet.rs` tests, dedupe the
  `cascade/apply.rs` initials against `ComputedStyle::default()`, accept `ua.rs` / `tui_style.rs`,
  state a policy for test files.
  Decision: fix rather than record, one item per commit (`P6G-*` ids): OPACITY-COMPOSITE,
  OPACITY-LAYER-COST, SELECTION-CAPTURE (then drop the toggle / range UA rule), USER-SELECT-INHERIT,
  FORM-RESET (select, range, textarea capture, default setters), PSEUDO-SHIFT, BLOCK-FIRST-PSEUDO,
  FRAME-DEDUPE (+ carried stylesheet ids), RENDER-HELPERS, LAYOUT-RS-SPLIT, STYLESHEET-TESTS,
  APPLY-INITIALS, TEST-UTIL, SHOWCASE-INTENTS, DOCS (CHANGELOG reorganised per crate and kind,
  stale comments, DESIGN opacity paragraph). Demos for the new features go to the Phase 7 scope
  question.
- 2026-09-24 — Phase 6 gate fix: `P6G-OPACITY-COMPOSITE-1` — `composite_group` rewritten around a per-cell glyph contest (layer glyph wins at α ≥ 0.5 or over an empty cell), border contributions blended / tinted before the joiner, canvas fg model, wide pairs as a unit, α = 0 no-op. `opacity_zero_collapses_fg_to_parent_bg` now expects nothing painted.
- 2026-09-24 — Phase 6 gate fix: `P6G-OPACITY-LAYER-COST-1` — `paint_pass/group.rs`: `layer_region` (rows of every box / anonymous box / pseudo box / inline line in the subtree, ±1 row, full width) + `Buffer::copy_region`; `composite_group` walks only the layer's area. Under `cfg(test)` every group is also painted through a full-frame layer and asserted equal.
- 2026-09-24 — Phase 6 gate fix: `P6G-SELECTION-CAPTURE-1` — the selection drag is router state only (`router.selection_drag`); `drag::begin` no longer calls `set_pointer_capture` / `set_drag_autoscroll`. `handle_move` extends on every button-held move whatever the hit (mousemove / hover stay hit-targeted), a button-less move ends a stale drag; `App::autoscroll_drag_source` = opted-in capture, else the selection drag's anchor container. Expectations changed: `mousedown_on_text_starts_drag_and_sets_caret_selection` (capture None), `mousedown_alone_leaves_collapsed_caret_not_whole_paragraph` and `text_selection_drag_past_edge_autoscrolls_and_keeps_extending` (router state instead of `drag_autoscroll()`). Scrollbar-thumb drags keep their capture. DIVERGENCES §pointer capture / drag autoscroll updated.
- 2026-09-24 — Phase 6 gate fix: `P6G-TOGGLE-USER-SELECT-REVERT-1` — removed the FORM-DEFAULTS-1 UA `user-select: none` on checkbox / radio / range; new `ua_toggles_and_range_leave_user_select_alone`, `ua_buttons_are_unselectable` narrowed to the button family. Toggle test renamed `click_on_checkbox_beside_text_toggles_and_selects_no_text`: the mousedown's empty-space snap now collapses a caret in the prose (as a browser mousedown outside text does), so it asserts no *text* is selected instead of no selection. FORM-DEFAULTS-1's CHANGELOG bullet dropped; DIVERGENCES documents the button-family UA rule.
- 2026-09-24 — Phase 6 gate fix: `P6G-USER-SELECT-INHERIT-1` — user-select dropped from the inherited set (`inherits`, `inherit_inheritable_from`); `runtime/selection/user_select/{mod,tests}.rs`: `used_value` / `resolve_child` / `is_unselectable` / `all_host` / `contain_host` replace `ancestor_with` / `host_with` / `has_none_ancestor`; the clipboard walk carries the used value down (and a range end inside `none` text now stops the copy). Editable elements = text controls + contenteditable hosts (DIVERGENCES). Expectation changed: `user_select_inherits_to_child` → `user_select_is_not_inherited` (computed `auto`, used `none`); `position_at_user_select_none_inherits_to_subtree` renamed. rdom-tui lib 1686 → 1701, rdom-style 256 → 257 (incl. 2 green-on-arrival pins: `::before` highlight, explicit-text hit).
- 2026-09-24 — Phase 6 gate fix: `P6G-FORM-RESET-1` — `input::keeps_default_value` (textarea, text-family, range) gates `note_default_value` / `reset_to_default`; range captures in `range::set_value` and the `set_value` accessor, textarea in the accessor; `select::state::{note_default_selected, reset_to_default}` called by every runtime `selected` writer (single pick, toggle, extend, Ctrl+A, the `set_value` accessor); `reset_controls` gains the `<select>` branch. `default_value()`: an `<input>` falls back to its `value` attribute. DIVERGENCES entry rewritten (no dirty flags; no selectedness setting algorithm). rdom-tui lib 1701 → 1708.
- 2026-09-24 — Phase 6 gate fix: `P6G-PSEUDO-SHIFT-1` — packer `push_generated` → `LineBox::generated` (`GeneratedFragment { host, slot, x, width, text }`), kept out of `fragments` so hit-test / caret / selection / clipboard never see generated cells (a click on one clamps to the text). `compute_inline_layout` packs the host's pseudos; `compute_inline_layout_for_run` the first / last in-flow run's (`pack_run` + `RunPseudos` crate-internal). Paint's `Pseudos` shift removed; min-content no longer adds the pseudo width on top of a pack that holds it; static positions start after a `::before`. The empty-editable caret stays at the block start (the `:empty::before` placeholder idiom); placeholder paint and caret pinned. 7 red → green + 3 pins (packer contract, placeholder paint, placeholder caret).
- 2026-09-24 — Phase 6 gate fix: `P6G-BLOCK-FIRST-PSEUDO-1` — `render/inline/generated.rs` owns the placement predicates (`own_inline_pseudo_text`, `own_line_pseudos`, `marker_line_holder`, `deferred_markers`), shared by the packer, the block pass (pseudo-only anonymous runs at a block edge; the Inline arm packs with explicit `RunPseudos`), intrinsic sizing (own-line rows on the Column axis; marker width counted on its holder), the margin-collapse predicates (an own-line pseudo separates parent / child margins) and paint (a block-flow container with in-flow children no longer paints its pseudos at its first row — that drew them under the first child, "blockd"). Choice: `li::before` follows `::marker` placement (browser-visible "1. Step"), not `::before`'s own line; the DIVERGENCES block-first entry is replaced by the marker entry. 5 red → green (the whitespace-surrounded case was already green after PSEUDO-SHIFT) + 4 pins written after the fix (margin, flex intrinsic height, nested markers, flex fallback).
- 2026-09-24 — Phase 6 gate fix: `P6G-INLINE-PSEUDO-1` — `render/inline`: `walk_inline_box` packs every in-flow inline element's static `::before` / `::after` around its content (the cascade already computed them; nothing consumed them). Paint tags generated cells with the host's `<a href>`; `hit_fragment` routes a generated cell of an inline descendant to its host (unless the pseudo is `pointer-events: none`); `inline_content_width` adds descendants' pseudo widths; `static_position_in_ifc` orders generated runs with their host's top-level child and takes the furthest end (a later `::before` is no longer ahead of every child). 9 red → green (packer, paint ×4 incl. link + counter + max-content, hit offsets, hit target, caret, static position).
- 2026-09-24 — Phase 6 gate fix: `P6G-TEXT-BEFORE-BLOCK-MARGIN-1` — `parent_collapses_{top,bottom}_with_*` also require no inline content at that edge (`generated::inline_content_at_edge`, the `line_bearing_child` walk the pseudo placement uses: collapsible whitespace-only text holds no line), so the local suppression in `layout_block_children` and the outer-margin chain walkers agree. 2 red → green + 1 guard (whitespace-only edges, green on arrival). Noted, not fixed: a margin that escapes through the root's first child is dropped at the document root (outer.y = 0).
- 2026-09-24 — Phase 6 gate fix: `P6G-SELECT-SELECTEDNESS-1` — `select/selectedness.rs`: `run` (the algorithm; captures `defaultSelected` of each option it flips), `seed_all` (App::build), `reset_to_default` ends with it; `Selectedness` = a mutation observer queueing `OptionsChanged` (option / optgroup child-list changes, via `enclosing_select`) and `Picked` (`selected` added to an option) in record order, muted while flushing, flushed at the top of `handle_event`, `draw_if_dirty` and `cascade_and_layout` (the observer cannot mutate the tree — DIVERGENCES documents the deferral); the last `Picked` per select wins. `model::option_disabled` (own attribute or disabled `<optgroup>` parent; keyboard / click still read the own attribute only). Accessors `default_selected` / `set_default_selected`. Expectations changed: `value_returns_empty_when_nothing_selected` now uses a multi-select; `value_returns_single_selected_option_value`, `space_in_single_select_does_not_trigger_type_ahead` and `keyboard_pick_captures_default_selected_and_reset_restores_it` author their `selected` before mount (`select_fixture_marked`) — marked after mount, the algorithm's first-option pick sat beside it until the next flush. 7 red → green + accessor test (compiled with the new methods) + exclusivity test (verified red with `make_exclusive` disabled) + list box / multi guard. DIVERGENCES: the no-algorithm clause replaced by the algorithm, its deferral and the accessors.
- 2026-09-24 — Phase 6 gate fix: `P6G-SELECT-OPTGROUP-DISABLED-1` — every pickability check in `select/{keyboard,click,state,typeahead}.rs` reads `model::option_disabled` (own attribute or disabled `<optgroup>` parent) instead of the option's own attribute. 6 red → green (arrows ×2, Home / End, click, type-ahead, multi Ctrl+A / Space / shift-extend). Noted, not fixed: form `collect` still submits a selected disabled option (HTML's entry list skips disabled options).
- 2026-09-24 — Phase 6 gate fix: `P6G-ROOT-MARGIN-1` — `block/margin_collapse.rs`: `establishes_independent_formatting_context` (own `establishes_new_bfc`, the root itself, or a flex item — layout parent `flow: flex`, or the Fragment root whose children `layout_fragment_children` lays out as a column) gates `parent_collapses_{top,bottom}_with_*`, so layout and the chain walkers stop at it. Root cause was general: flex items were never collapse boundaries, and the Fragment root lays its children out as flex items, so an escaped margin reached a flex placement that never reads the chain. 4 red → green (root child top / nested / bottom, flex item) + 1 guard (root child's own margin, green on arrival). Fixtures moved under a plain block (expectations unchanged): `parent_top_margin_collapses_with_first_child_when_no_padding_or_border`, `parent_with_only_empty_collapse_through_children_has_zero_content`. No showcase snapshot changed. DIVERGENCES §`Dom::root()` documents the top-level-element-as-root model.
- 2026-09-24 — Phase 6 gate fix: `P6G-FRAME-DEDUPE-1` — `app/frame.rs`: one pipeline, `style_and_layout` (cascade → transition diff / advance → layout → caret-reveal servicing; a free fn over split borrows because `draw_if_dirty` runs it inside `Terminal::draw`), and `take_dirty_roots` (drain + sort + dedup); `draw_if_dirty` and `cascade_and_layout` both call them (scroll-focus marking stays draw-only, as before). Stylesheet ids: one `StylesheetIdAllocator` owned by the App and borrowed by every `AppContext` (`&mut`, split from `dom`), so the context no longer predicts ids from a snapshot; `Set` / `Push` intents carry the id and the App registers the sheet under it (`replace_stylesheets` / `append_stylesheet`), allocating nothing — the `debug_assert_eq!` tie is gone, and a registered-at-most-once `debug_assert!` guards the stack. Interleaving is structurally impossible (a live context holds `&mut` to the allocator and the DOM); pin `stylesheet_ids_from_contexts_and_direct_calls_are_the_registered_ids` (set + push + push + remove in one handler, then a direct push, then another handler) green before and after. `StylesheetId`'s field is now private. Behavior unchanged.
- 2026-09-24 — Phase 6 gate fix: `P6G-RENDER-HELPERS-1` — `cell_of_position` (+ `fragment_for_position` / `phantom_line_and_column`) moved to `render/inline/caret.rs` (`render::inline::cell_of_position`, public; `runtime::editing::caret::cell_of_position` stays as a re-export); the one `cells_before_byte` is `render::inline`'s, used by caret and the selection overlay (the paint copy is gone). The `user-select` used value (`used_value` / `resolve_child` / `is_unselectable` / `all_host` / `contain_host`, with their tests) moved to `style/user_select`; `runtime/selection/user_select.rs` keeps the behaviors (`span_all_text`, `clamp_to_contain_host`) and re-exports the rest. Runtime callers (movement, caret reveal) use the render path. Stale `child_range` comment in `layout_block_children` fixed (raw child-node indices, which `inline_flow_for_text` relies on; `AnonymousIfc` doc likewise). Non-test `crate::runtime::` in `render/`: only `builtins::inline_chrome::lookup` and `builtins::canvas::RenderContext`. No behavior change.
- 2026-09-24 — Phase 6 gate fix: `P6G-FORM-COLLECT-DISABLED-1` — `form/mod.rs` `walk_collect`: the `<select>` arm filters `selected_options` through `select::option_disabled` (now `pub(crate)`, re-exported from `select`). 2 red → green: `collect_skips_selected_options_that_are_disabled` (left `[m=kept, m=own-disabled, m=group-disabled]`), `collect_single_select_with_disabled_selected_option_submits_nothing`. `fieldset[disabled]` is not implemented anywhere (focus, activation, collect all read the control's own attribute) — DIVERGENCES §3 lists it; `collect` doc's stale "<select> not supported" line fixed.
- 2026-09-24 — Phase 6 gate fix: `P6G-LAYOUT-RS-SPLIT-1` — `rdom-style/src/layout.rs` (1340 lines) split into `layout/{mod,rect,keywords,sizing,box_model}.rs` (33 / 119 / 324 / 190 / 474); every type keeps its `rdom_style::layout::` path through `mod.rs` re-exports. The geometry (`compute_content_area{,_collapsed}`, `compute_padding_box`, `clamp_size`) — every caller is in rdom-tui (layout pass, paint, stacking, scrollbar / focus / router geometry), none in rdom-css or rdom-style — moved to `rdom-tui/src/render/layout_pass/geometry.rs`; `rdom_tui::layout` is now a module (`pub use rdom_style::layout::*` + the geometry), so `rdom_tui::layout::compute_padding_box` keeps working; the 14 `rdom_style::layout::compute_padding_box` call sites use `crate::layout::`. Breaking for rdom-style (CHANGELOG). Tests moved with their code: 23 = 8 (rect 6 + box_model 2) in rdom-style + 15 in geometry; rdom-style lib 242, rdom-tui lib 1780 (1765 + 15), all green.
- 2026-09-24 — Phase 6 gate fix: `P6G-STYLESHEET-TESTS-1` — `rdom-style/src/stylesheet.rs` (1020 lines) split into `stylesheet/{mod,index,selector_text,tests}.rs` (370 / 90 / 134 / 444): `RuleIndex` in `index.rs` (`build` now `pub(super)`), `extract_pseudo_suffix` / `split_top_level_commas` in `selector_text.rs` (`pub(super)`), all 42 tests in `tests.rs` under their section headers. Public paths unchanged. No behavior change; rdom-style lib 242 green.
- 2026-09-24 — Phase 6 gate fix: `P6G-APPLY-INITIALS-1` — `cascade/apply.rs` (1090 → 581 lines): the ~20 per-type wrappers and `apply_simple!` are one generic `apply_value` (+ `apply_optional` for the `Option` fields) driven by a `value!(field: MASK, …)` macro; `initial` reads the field of `ComputedStyle::initial()` through `Initials` (a per-element `OnceCell`, built only when an `initial` keyword is met), so the starting style and the keyword share one table. Hand-rolled applicators (colors, modifier bits, `text-decoration`, `opacity`, `border-collapse`) read `Initials` too; `border-color`'s initial stays `currentColor`. Transition lists join the generic path (`apply_transition_lists` gone). New `cascade/apply_tests.rs`: `initial_keyword_yields_the_initial_computed_value_for_every_property` (every `property_names()` entry set to `initial` over a perturbation that moves every field; exhaustive `ComputedStyle` destructuring) — green on arrival, verified red by mutating opacity's initial to 0.0 (`initial` for `opacity`: left 0.0, right 1.0). Found by the unification: `flow`'s `inherit` slot was hard-coded to `Block`, so `display: inherit` dropped the parent's inner display — `display_inherit_takes_the_parents_inner_display_too` red (left `(Block, Block)`, right `(Block, Flex)`) → green. rdom-tui lib 1782.
- 2026-09-24 — Phase 6 gate fix: `OVERSIZED-FILES-1` — resolved (`P6G-TEST-FILE-POLICY-1`). Split: `rdom-style` `layout.rs` (`P6G-LAYOUT-RS-SPLIT-1`), `stylesheet.rs` (`P6G-STYLESHEET-TESTS-1`); `rdom-tui` `cascade/apply.rs` 1090 → 587 (`P6G-APPLY-INITIALS-1`). Accepted: `rdom-style/src/ua.rs` (1329) is one data table — the UA rule list, grouped by element family and pinned by `ua_total_rule_count` — and `tui_style.rs` (1113) is one macro-generated builder (the `TuiStyle` struct and its `setter!` methods); splitting either would scatter one concern. Test files (`cascade/tests.rs` 2784, `layout_pass/tests.rs` 4710, `paint_pass/tests.rs` 5292, `app/tests.rs` 3836) are covered by the new CLAUDE.md §Architecture Hygiene policy: the few-hundred-line bar is for production modules; a test file organized by per-behavior section headers may run long and is split when a section outgrows its module. Also fixed the stale DIVERGENCES §Layout "No margin collapsing" line (block-flow collapsing has shipped since BFC-1; the entry now states what collapses and what does not) and the matching `ComputedStyle::margin` doc comment.
- 2026-09-24 — Phase 6 gate fix: `P6G-FORM-SUBMITTER-1` — `form/mod.rs`: `collect_with_submitter` (`collect` = no submitter); `walk_collect` threads the submitter — a button contributes only as the submitter and only a submit button can be one (`<input type=reset|button>`, `<button type=reset|button>` never; `<button>` submitters now collected, value default `""`). `default_button` + the keydown path: Enter clicks the default button (`TuiAccessorsMut::click`, so the click listener submits with it and handles `method=dialog`), a disabled one blocks, the one-field rule only without a submit button. `button_action`: invalid `<button type>` = Submit (HTML §4.10.6). `SubmitDetail` doc updated; DIVERGENCES §omissions: form owner = nearest `<form>` ancestor (no `form=`, no `formaction` family). 7 red → green (`collect_includes_only_the_submitter_among_buttons` left `[q, a=A, r=R, x=X]`; `enter_with_a_default_button_clicks_it_and_submits_with_it`, `enter_with_a_disabled_default_button_does_not_submit`, `button_with_an_invalid_type_is_a_submit_button`, `click_on_button_with_invalid_type_submits`, `clicked_submit_button_is_the_submitter_and_the_only_button_entry`, `submitter_without_value_contributes_empty_string_and_without_name_nothing`). No existing expectation changed. rdom-tui lib 1789.
- 2026-09-24 — Phase 6 gate fix: `P6G-TEST-UTIL-1` — `crates/rdom-tui/Cargo.toml`: self dev-dependency `rdom-tui = { path = ".", features = ["test-util"] }` (cargo accepts it); `[package.metadata.docs.rs] features = ["test-util"]` (not `all-features`, which would also document the `no-synchronized-output` build); `rdom-showcase`'s dev-dependency on rdom-tui dropped (it does not use `VirtualScreen`, and its normal dependency covers everything else). Red: `cargo test -p rdom-tui --test inline_flow` → "target `inline_flow` in package `rdom-tui` requires the features: `test-util`"; green: `cargo test -p rdom-tui` runs lib 1789 + `inline_flow` 28 + integration 163 + doc 6, no flags. `cargo publish --dry-run -p rdom-tui --allow-dirty` packages (239 files); the packaged manifest drops the path-only self dev-dependency (cargo strips versionless dev-deps), so a tarball test run still needs `--features test-util` for `inline_flow` — documented in the Cargo.toml comment and CLAUDE.md. The dry-run's verify step fails for an unrelated, pre-existing reason: it builds against the published rdom-style 0.4.0, which lacks items rdom-tui now imports (`ContentContext`, `CounterOp`, `GapValue`, …) — a release needs the rdom-style bump first. README §Features, CLAUDE.md §Testing Commands updated.
- 2026-09-24 — Phase 6 gate fix: `P6G-SHOWCASE-INTENTS-1` — `rdom-showcase/src/demo_sheet.rs`: `DemoSheet` (`AppHandle` + `Arc<Mutex<Option<StylesheetId>>>`); `ShowcaseState::attach_sheet` registers the mounted demo's sheet on the App before the first frame, and `mount_demo` (run from the sidebar `click` listener) queues `switch_to` through `AppHandle::inject` — the closure removes the previous sheet and pushes the new one with the `AppContext` intents. `main.rs` no longer pre-pushes all 21 demo sheets (each of which also carried a full UA copy via `from_css`); the stack is base + mounted demo. Substrate gap found and fixed: `App::advance` (the headless driver, "exactly as the loop would") never drained injections, so no test could observe a listener's intents — it now calls `drain_handle_injections` before drawing. Red → green: `advance_runs_injected_closures_before_drawing` (left 1 sheet, right 2); `keyboard_nav::switching_demos_swaps_the_mounted_demos_stylesheet` (after Down + Enter + `advance(0)`: still `.hello`'s sheet, expected `.flex-row-demo`'s). `keyboard_nav::build_app` ("exactly as main::run") and `swap_renders_clean_at_full_viewport` use `attach_sheet` (the latter asserts base + one sheet after every swap); other integration tests keep pushing every sheet (worst case for the class-scoped convention). No snapshot changed. Stale comments fixed (registry test: no TECH_DEBT row, no "file a substrate gap"; `demo.rs`, `nav.rs`, `shell.rs`, `main.rs`, `chrome_layout_contract.rs`). The class-scoped selector convention stays: demos no longer collide with each other, but the mounted sheet still applies to the chrome.
- 2026-09-24 — Phase 6 gate fix: `P6G-RUSTDOC-1` — `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` (with and without `--features rdom-tui/test-util`) was red with 107 warnings (rdom-core 6, rdom-style 12, rdom-tui 91 — the first failing crate stops the `-D` run); now clean. Fixes: ~65 public module docs linking private items (Phase 6 module splits: `scrollbar`, `select`, `cascade`, `layout_pass`, `paint_pass`, `hit_test`, `selection`, `router`, …) → plain code spans; unresolved links repointed (`Flow` → `crate::layout::Flow`, `ImportantMask`, `LayoutRect`, `CrosstermBackend`, `App::with_url_opener`, `rdom_trace!`, `Self::copy`, `TuiDocAccessors::*_from_point`) or rewritten where the target never existed or moved (`Range::new` → `Dom::selection_range` / `Range::ordered_unchecked`; `contextmenu_keyboard`; rdom-style's link into rdom-css's `apply_declaration`; `runtime` `pointer_capture` / `abort` → rdom-core; `selection::paint` → `paint_pass`; `render` `color` / `modifier` → the `rdom_style` re-exports; the empty `crate::editing` placeholder now says where editing lives); CSS grammar `<ident>` etc. in code spans; bare URLs in `<>`; redundant explicit link targets dropped; a non-Rust `…` in an `ignore` block. Doc comments only. CI does not run rustdoc (build / test / clippy / fmt only); not added here.
- 2026-09-24 — Phase 6 gate fix: `P6G-DOCS-1` — CHANGELOG Unreleased reorganised per crate and kind, breaking changes flagged with migration hints; stale comments fixed.
- 2026-09-24 — Phase 6 gate fix: `P6G-DRAG-RESET-1` — `handle_down` calls `drag::end` first, before hit-testing. 3 red → green (`press_on_scrollbar_thumb_after_lost_mouseup_…`, `press_on_nothing_after_lost_mouseup_…`, `cancelled_press_after_lost_mouseup_…`; each left `Some(Ifc { block: NodeId(2) })`, right `None`). DIVERGENCES §pointer capture notes the lost-mouseup rule. rdom-tui lib 1793.
- 2026-09-24 — Phase 6 gate fix: `P6G-RUN-LAYOUT-VIS-1` — `compute_inline_layout_for_run` (public since rdom-tui 0.3.14, so kept rather than made crate-private) now picks the run's pseudos with `generated::run_pseudos` — the run holding the host's first / last `line_bearing_child` — so whitespace-only text and comments at the edges no longer drop them; the block pass is unchanged. `GeneratedFragment::slot`: `StyleSlot` → new `ext::PseudoSlot` (paint's dead `Host` arm gone; `From<PseudoSlot> for StyleSlot`). Red → green: `run_layout_packs_host_pseudos_past_whitespace_only_edges` (the span-alone run: left `[]`, right `[(NodeId(2), Before, "<"), (NodeId(2), After, ">")]`). Tests `static_pseudos_pack_as_generated_fragments` / `inline_element_pseudos_pack_at_its_start_and_end` compare `PseudoSlot` values (type change only).
- 2026-09-24 — Phase 6 gate fix: `P6G-SUBMIT-DEFAULT-LABEL-1` — kept `""`: HTML's entry list appends "the value of the field element" (§4.10.21.4) and a submit input's value (value mode default) is its `value` attribute or `""`; the "Submit" default is only the label (§4.10.5.1.19, both checked against the live spec). Browsers send the label (Chrome `Submit`, Firefox `Submit Query`), but rdom paints no label for `<input type=submit|reset|button>` at all — probe: `[  ]` with and without `value=Go` — so there is no rdom label to reuse. DIVERGENCES: the submit-value entry (§2) and the no-label limitation (§4). Pin `value_less_submit_input_submitter_contributes_empty_string` (green on arrival, the decision).
- 2026-09-24 — Phase 6 gate fix: `P6G-OVERLAY-CHEAP-FIRST-1` — `apply_selection_overlay` runs the `is_unselectable` ancestor walk after the byte-range / cell-range checks, so fragments outside the selection skip it. No behavior change; covered by `selection_spanning_user_select_none_skips_its_highlight` and `selection_highlights_explicit_text_inside_a_user_select_none_block`.
- 2026-09-24 — Phase 6 gate fix: `P6G-DOCS-2` — DIVERGENCES: the opacity same-glyph rule (layer's modifiers / link win at any alpha when it repaints the backdrop's own symbol — selection overlays) and top-level / flex-item margins adding (2 + 3 = 5 rows; `<html><body>` for browser behavior) in both margin entries. DESIGN: the layer is a bounded row band, not a frame copy; same-glyph bullet. `paint_pass/group.rs` doc: `cfg(test)` paints every group twice (2^depth for nested groups, canvas callbacks twice). CHANGELOG: Added `GeneratedFragment` / `LineBox::generated` / `ext::PseudoSlot` / `render::inline::cell_of_position`. TECH_DEBT: empty Open subsection headers removed. CI: `rustdoc` job on ubuntu (toolchain from `rust-toolchain.toml`, rust-cache), command added to the CLAUDE.md gate, §Pre-commit and `/commit` (now four commands).
- 2026-09-24 — Phase 6 gate fix: `P6G-INPUT-BUTTON-LABEL-1` — the label was never generated: nothing in the UA sheet, `seed_all` (text family only) or the chrome seam produced it, and the bare `input { width: 20 }` rule also sized these inputs to 20 cells. Fix in the UA sheet (rdom-style): `input[type=button|submit|reset]::before { content: "[ " attr(value) }`, `:not([value])` defaults `"[ Submit"` / `"[ Reset"`, `width: auto` on the button-family input rule; `button::before` keeps `"[ "`. Generated content rather than a chrome supplier: the seam is paint-only (the select it serves has a fixed width), while pseudo content is already measured by intrinsic sizing and hit-tests to the host. 8 red → green in `button::tests` (e.g. `submit_input_paints_its_value_as_the_label`: left `"[  ]"`, right `"[ Go ]"`; `submit_input_box_is_as_wide_as_its_label`: left 20, right 6; `a_click_on_the_label_activates_the_submit_input`: click at x=6 counted, box too wide). `ua_total_rule_count` 142 → 144. DIVERGENCES: §4 limitation removed, new §Selection & editing entry (label is `::before` content), §2 submit-value entry reworded.
- 2026-09-24 — Phase 6 gate fix: `P6G-PRESS-RESET-2` — `handle_down` now also calls new `scrollbar::cancel_drag` (after `selection::drag::end`, before hit-testing): it clears `router.scrollbar_drag` and releases the capture only while it is still on the drag's scrollbar owner (the capture `begin_drag` took). Author capture is not released by a press (browsers keep implicit capture until pointerup / pointercancel; rdom releases it on mouseup or the next button-less move). Red → green: `press_on_text_after_lost_thumb_mouseup_does_not_drag_the_thumb` (scroll_y left 24, right 14); pin `press_after_lost_mouseup_keeps_an_author_pointer_capture` (green on arrival). DIVERGENCES §pointer capture extended. rdom-tui lib 1808.
- 2026-09-25 — Phase 6 closed. Re-review of the gate fixes (`5ae9ffd..b03fbf1`): no blocking
  findings; its follow-ups fixed (`P6G-DRAG-RESET-1`, `P6G-RUN-LAYOUT-VIS-1`, `P6G-OVERLAY-CHEAP-FIRST-1`,
  `P6G-DOCS-2` incl. a rustdoc CI job, `P6G-INPUT-BUTTON-LABEL-1`, `P6G-PRESS-RESET-2`;
  `P6G-SUBMIT-DEFAULT-LABEL-1` kept `""` per WHATWG, recorded in DIVERGENCES). TECH_DEBT Open: none.
  Carried to the Phase 7 scope question: `fieldset[disabled]`, demos for the Phase 6 features.
  Carried to Phase 8: rdom-style must bump before rdom-tui can verify a publish; a tarball test run
  needs `--features test-util` for `inline_flow` (path dev-dependency stripped on publish).
- 2026-09-26 — Phase 7 scope confirmed with Miska: everything offered. Items, in order:
  `P7-FIELDSET-DISABLED-1`, `P7-FORM-OWNER-1` (`form=` + `formaction` / `formmethod` / `formenctype` /
  `formtarget` / `formnovalidate`), `P7-VALIDATION-1` (constraint validation: `required`, `pattern`,
  `minlength` / `maxlength`, `min` / `max` / `step`, type mismatch, `ValidityState`, `checkValidity` /
  `reportValidity` / `setCustomValidity`, `invalid` event, `:valid` / `:invalid` / `:required` /
  `:optional`, submission blocked unless `novalidate` / `formnovalidate`), `P7-FOCUS-VISIBLE-1`,
  `P7-PLACEHOLDER-PSEUDO-1`, `P7-UNDO-COALESCE-1`, `P7-CARET-BLINK-1`, `P7-CLIPBOARD-WS-1`,
  `P7-SCROLL-BEHAVIOR-1`, `P7-LIVE-STYLE-1`, `P7-DEMOS-1`. Each ends its DIVERGENCES §3 line or
  README open-polish mention.
- 2026-09-26 — Phase 7: `P7-FIELDSET-DISABLED-1` — HTML §4.10.18.5 "actually disabled" lives in one place, `Dom::is_actually_disabled` (rdom-core `form_control.rs`): own `disabled`, or inside a `<fieldset disabled>` outside its first `<legend>` child; `<optgroup>` / `<option>` rules folded in. `:disabled` / `:enabled` added to the selector engine; the UA `[disabled]` rule is now `:disabled`. Focus (`tab_index`, `has_focusable_descendant`), `is_editable`, toggle / select / range / number activation, the form click and implicit-submission gates and `collect` all use it; `select::option_disabled` delegates. Expectations changed: `disabled_overrides_explicit_tabindex` and `ua_rule_loses_to_author` / `ua_disabled_rule_mutes_by_default` use an `<input>` (a `<div disabled>` is never disabled in HTML); new `disabled_attribute_on_a_non_control_does_not_block_focus`, `ua_disabled_rule_ignores_a_disabled_attribute_on_a_non_control`. DIVERGENCES §3 `<fieldset disabled>` line removed; new §cascade entry on the UA muting inheriting into the fieldset.
- 2026-09-26 — Phase 7: `P7-FORM-OWNER-1` — HTML §4.10.17.3 form owner and §4.10.19.6 submitter overrides, in rdom-core `form_assoc.rs`: `Dom::form_owner` (`form="id"` naming a connected `<form>`, else nearest ancestor form; a `form` attribute naming no form means no owner, even nested), `Dom::form_listed_elements` (one DFS over the form's whole tree, tree order), `Dom::is_submit_button`, `Dom::submit_detail`. `SubmitDetail` gains `action` / `method: FormMethod` / `enctype: FormEnctype` / `target` / `no_validate` and becomes `#[non_exhaustive]` with `SubmitDetail::new`. rdom-tui `form::elements` / `collect_with_submitter` / reset / default button / implicit-submission field count and the `input_form` / `textarea_form` / `select_form` / `button_form` accessors use the owner; `enclosing_form`, `nearest_form_ancestor` and `is_dialog_form` are gone — the dialog close reads the effective method, so a submitter's `formmethod="dialog"` closes the dialog. No existing expectation changed. DIVERGENCES §3 `form` attribute line removed; new §2 entry: submission navigates nowhere, `action` / `target` are raw strings.
- 2026-09-26 — Phase 7: `P7-FORM-ENUM-CASE-1` — HTML §2.3.3 enumerated keywords: `Dom::is_submit_button` and every rdom-tui `type` read (text family, toggle, radio, range, number, password, hidden, button family, form collect / reset / implicit submission, focus, label) go through new rdom-core `InputTypeState` / `Dom::input_type_state` (case-insensitive, Text for missing / invalid); the four copies of the text-family list are one `node::is_text_input`. The selector matcher treats the values of HTML §4.16.2's attribute list (`type`, `method`, `enctype`, `lang`, …) as ASCII case-insensitive, so UA `input[type=checkbox]` rules match `type="CheckBox"`. Chosen: both, each where HTML puts it — keyword reads parse through one enum, selectors follow §4.16.2 (a read-side fix cannot reach author / UA selectors, a matcher fix cannot reach Rust reads). `method` / `enctype` were already case-insensitive. Red → green: `submit_button_type_matches_ascii_case_insensitively`, `html_case_insensitive_attribute_values_match_regardless_of_case`, `input_type_keywords_are_ascii_case_insensitive`, `input_type_is_the_canonical_keyword_of_the_state`, `uppercase_checkbox_type_toggles_and_gets_the_ua_glyph`, `button_and_input_type_keywords_are_case_insensitive`. No expectation changed.
- 2026-09-26 — Phase 7: `P7-REQUEST-SUBMIT-1` — HTML §4.10.3 `requestSubmit`: `form::submit` is now the one submission path — click activation, implicit submission and `form_request_submit` all fire `submit` through it and it closes a method-`dialog` dialog keyed on the effective method from `submit_detail` (the per-caller close blocks and `fire_submit` are gone). `form::request_submit` checks the submitter first: not a submit button → new `DomError::TypeError`; form owner not this form → `DomError::NotFound`. `form_request_submit` returns new `SubmitOutcome { Submitted, Canceled, NotAForm }` (`#[non_exhaustive]`, re-exported). Red (compile: `SubmitOutcome` / `DomError::TypeError` missing; the old path closed no dialog) → green: `form_request_submit_rejects_a_submitter_that_is_not_a_submit_button`, `form_request_submit_rejects_a_submitter_owned_by_another_form`, `form_request_submit_closes_a_method_dialog_dialog`, `form_request_submit_canceled_does_not_close_the_dialog`. Expectations changed with the signature: `form_request_submit_fires_submit_with_submitter_detail`, `form_request_submit_returns_true_when_prevented` (now `…_reports_a_canceled_submit`), `form_request_submit_no_op_on_wrong_tag`, and the showcase `dom_api` snapshot line (`prevented=false` → `Submitted`).
- 2026-09-26 — Phase 7: `P7-VALIDATION-1` — HTML §4.10.20 constraint validation. rdom-core `constraint.rs`: `Dom::will_validate` (candidacy / barring: disabled, `readonly` where it applies, hidden / reset / button inputs, non-submit buttons, `<datalist>`). rdom-tui `runtime::builtins::validation` (`mod` API + `states` + `syntax` + `pattern` + `messages`): `ValidityState`, `validity` / `is_invalid` / `validation_message` / `set_custom_validity` / `check_validity` / `report_validity` / `interactively_validate`; `TuiExt` gains `custom_validity`, `value_user_edited` (set by `perform_edit` and undo / redo, cleared by programmatic values and reset — HTML's dirty value flag, so `maxlength` / `minlength` apply to user edits only, counted in UTF-16 code units) and a per-control compiled-`pattern` cache; `regex` added to rdom-tui (already in the lockfile). `form::submit` validates unless `SubmitDetail::no_validate`; `SubmitOutcome::Invalid`. Radio groups reuse `toggle::radio_group` (name-keyed, now in DIVERGENCES). Red → green: `an_invalid_form_blocks_click_submission`, `an_invalid_form_blocks_implicit_submission`, `a_canceled_invalid_event_still_blocks_submission_without_focusing` (submit fired: `log.borrow().is_empty()` failed); the API tests (`validation::tests`, 18; `constraint::tests`, 2; `syntax` 2; `an_invalid_form_blocks_request_submit`, `novalidate_and_formnovalidate_skip_validation`) were red by compilation. No expectation changed. DIVERGENCES: §3 line narrowed to the pseudo-classes and the unrendered input types; new §2 entries for `pattern` (Rust regex vs JS `v` flag), the URL syntax check, no validation bubble / fixed English messages, name-keyed radio groups.
- Found while mapping Phase 3 (not on the ledger): `ImportantMask::FLOW` and `POINTER_EVENTS` share
  bit 39 (`tui_style.rs`), custom-property inheritance in the cascade is overwritten by the merged
  root map (`walk.rs`), and tokenizer errors inside a block are body-relative (`declarations.rs`).
  All three are Phase 3 items.

- 2026-09-23 — Phase 1: `rust-toolchain.toml` pins `1.95.0` and CI reads the channel from it
  (`PROC-TOOLCHAIN-PIN-1`). The ten showcase-backed example shims, the twelve demo snapshot tests and
  the sixteen goldens moved into `rdom-showcase/{examples,tests}/`; `rdom-tui/examples/` has three
  self-contained programs, the `rdom-showcase` dev-dep and the tarball `exclude` are gone
  (`PROC-TUI-DEV-DEP-1`).

- 2026-09-23 — Program opened. Triage: 3 stale rows deleted (`CSS-INHERIT-KEYWORD-1`,
  `CSS-BG-SHORTHAND-1`, `BORDER-MODEL-1`), 52 remain: 50 fix, 1 divergence (`TABLE-TFC-1`),
  1 external (`ITERM2-MOUSE-MOTION-1`).
