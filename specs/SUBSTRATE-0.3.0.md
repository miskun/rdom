# SUBSTRATE-0.3.0 — substrate fixes from the first downstream consumer

**Status:** in flight (started 2026-06-02). Target: **0.3.0**.

## Origin

The first real downstream consumer of the published substrate — `rdom-extensions` (a
data-visualization component crate: charts, sparklines, gauges, a virtual table, built on
`rdom-tui 0.2`) — surfaced seven points of friction while being built. They were captured in that
repo's `RDOM_SUBSTRATE_FINDINGS.md` (grumpy-architect review) and confirmed against rdom source
before this plan. This document is the rdom-side fix plan; each finding maps to a `TECH_DEBT` ID.

The headline lesson: **the substrate is good, but two gaps make it lie to a programmatic
consumer** — geometry node setters that don't affect layout, and no way to request a repaint from
an event listener. Those are the High items. The rest are ergonomics/hygiene.

## Milestones

Sequenced by severity. A and B are High; C is quick/additive; D is medium. Each is independently
shippable; B/C/D don't depend on A. Each ends with the two review passes and a `TECH_DEBT`/
`DIVERGENCES` update. Workspace gate (`fmt` / `clippy -D warnings` / `test`) green per commit.

### Milestone A — `EXT-LAYOUT-SETTERS-1`: make geometry node setters drive layout (High · breaking)

**Confirmed root cause.** `TuiNodeMutExt::set_width/set_height/set_min_*/set_max_*/set_direction/
set_gap/set_padding/set_border/set_overflow` write **raw `ext.*` fields** (`node.rs:285-349`). The
cascade reads only `inline_style` (a `TuiStyle`) + stylesheet rules (`cascade/walk.rs:215`); the
flex/block layout reads only `ComputedStyle` (`flex.rs:210` `parent.direction`, `:264` child
`.computed()`). The raw `ext.*` geometry fields reach **no** layout path. Their only readers:
the accessors (`node.rs:41-61` — which therefore mislead), tests, and one live consumer —
`accessors/helpers.rs:181` reads `ext.overflow` for scroll detection.

**Fix.** Rewire the setters to the cascade path:
1. Each setter writes `ext.inline_style.<prop> = Some(Value::Specified(v))` and sets
   `style_dirty = true`. (`TuiStyle` already carries every one of these fields.)
2. Accessors read the specified value from `inline_style` (preserves "you get back what you set"
   without needing a cascade; same values as before).
3. Redirect `accessors/helpers.rs:181` to read `computed.overflow`.
4. Remove the now-dead raw geometry fields from `TuiExt` (`ext.rs:146-157`). If test churn is
   large, gate removal behind a follow-up and `#[deprecated]` the fields meanwhile — decision after
   the audit.
5. Audit `rdom-showcase` + workspace tests for setter-as-layout usage; fixing this may activate
   latent bugs (geometry set via setters that silently did nothing). That's the point — but eyes.

**TDD.** Failing test first: a flex container built *only* with node setters (no CSS) currently
lays out wrong; assert it lays out correctly after the fix. Plus accessor round-trips and a scroll
regression via `computed.overflow`.

**Breaking:** setter→layout behavior changes (correct direction); accessor source changes (same
values). Pre-1.0 minor, acceptable.

### Milestone B — `EVENT-REDRAW-1`: repaint request from event listeners (High)

**Confirmed root cause.** `EventCtx { event, dom }` (`rdom-core/dispatch.rs:63`) has no redraw
request; only `AppContext::request_redraw` (`app/context.rs:68`, sets `redraw_requested`) and
`AppHandle` do. The loop repaints on DirtyTracker DOM mutations or `needs_redraw`
(`app/mod.rs` `draw_if_dirty`). A listener mutating non-DOM state (a `<canvas>` reading external
`Rc<RefCell>` state — the pattern rdom's own canvas docs recommend) produces no DOM mutation, so it
can't trigger a frame. Key events dispatch in `handle_event`; mouse via `Router` →
`RouteOutcome.redraw_requested`.

**Fix (general — Option A).**
1. `rdom-core`: `EventCtx` gains `request_redraw(&mut self)` + a backing flag; `dispatch_event`
   surfaces "redraw requested" to the caller.
2. `rdom-tui`: OR that into `needs_redraw` for both dispatch paths (key in `handle_event`; mouse by
   folding into `RouteOutcome.redraw_requested`).
3. Sugar: `canvas::request_repaint(dom, node)` for discoverability (thin wrapper).

**TDD.** A `keydown`/`click` listener that calls `ctx.request_redraw()` and mutates no DOM forces a
repaint (assert via a paint counter on `TestBackend`).

**Risk:** low-medium; additive to a core type, backend-agnostic, generic over `Ext`. Unblocks
interactive canvas components downstream.

### Milestone C — ergonomics & hygiene (Medium/Low · mostly additive)

- **`TUISTYLE-FLEX-BUILDER-1` (#2).** `TuiStyle::flow()` already exists; add `flex()` / `flex_row()`
  / `flex_column()` / `inline_flex()` mirroring the `property_dispatch.rs:402` `flex`-keyword
  mapping (`Display::Block` + `Flow::Flex`). Document the `.display()`-resets-`.flow()` ordering
  trap (`tui_style.rs:427`). Additive.
- **`RENDERCTX-DEDUP-1` (#3).** `render::RenderContext` (`render/render_context.rs`) is **dead code**
  — used only by its own tests, yet re-exported at the crate root (`lib.rs:80`) where it collides
  with the real one (`runtime::builtins::canvas::RenderContext`, the `PaintFn` type). Delete the
  dead type + its re-export; re-export the canvas `RenderContext` at the crate root as canonical
  (keep the name). Low breakage (unused name).
- **`CANVAS-TEST-CTOR-1` (#6).** Canvas `RenderContext::new` is `pub(crate)` — downstream can't
  build one over a scratch `Buffer` to unit-test paint. Add a public constructor
  (`pub fn for_test(buffer, rect)` or make `new` pub). Additive.

### Milestone D — `ARENA-RECLAIM-1`: detach-vs-free ergonomics (Medium)

**Confirmed.** `remove_child` / `clear_children` / `replace_child(ren)` detach but never free; only
`drop_subtree` frees (`tree.rs`). The detach-not-free is **deliberate** — observers read removed
nodes synchronously (`ChildListChanged { removed }`) and detached nodes can be re-attached. So
immediate-free would break those contracts.

**Fix (additive + docs; keep primitives detaching).**
1. Add `clear_children_dropping` / `remove_child_dropping` (detach + `drop_subtree`).
2. Document orphan/leak semantics loudly on the three detaching methods, pointing at `drop_subtree`
   and the new variants.
3. `DIVERGENCES.md`: add an entry — DOM relies on GC to reclaim detached nodes; rdom's arena does
   not, so reclamation is explicit. (Currently only implied.)
4. Open question: leave `replace_children` detaching (architect's lean — silent free surprises
   re-attachers); offer `replace_children_dropping` instead.

## Cross-cutting / release

- **Version 0.3.0** (A and C-#3 are breaking). Bump workspace + inter-crate pins; `CHANGELOG.md`
  `[0.3.0]`; README install lines.
- **`DIVERGENCES.md`:** A removes an undocumented divergence (set-property-affects-layout); note in
  decision archive. D adds the arena-reclamation divergence.
- **Downstream payoff (`rdom-extensions`):** after 0.3.0 — delete the `flex()` field-poking helper
  (use `TuiStyle::flex_row()`); wire real keyboard/mouse interaction via `ctx.request_redraw()`;
  unit-test paint directly. Bump it to depend on `rdom-tui 0.3`.

## Decision archive

- **2026-06-02 — repurposed the 0.3.0 slot.** 0.3.0 was pencilled as "client-side routing
  primitive"; routing slides to 0.4.0. 0.3.0 becomes the substrate-honesty release driven by the
  first downstream consumer's findings, because shipping a confirmed lie (setters that don't affect
  layout) and a blocker for interactive canvas components outweighs a new feature.
