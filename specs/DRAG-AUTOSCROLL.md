# DRAG-AUTOSCROLL — pointer capture + edge autoscroll for drags in scroll containers

**Status:** design **v3** (2026-06-05, reviewed twice — ready to implement). No version scheduled
yet; Phase order below.

**v3 changes (second review — feasibility/honesty, surfaced by tracing the data flow):**
- **Capture state lives in `rdom-core` dispatch** (it's a DOM API), including a generic
  `autoscroll: bool` flag, so a `Dom`-only `EventCtx` can arm both without reaching the App/router —
  this is *how* the `SHOWCASE-EVT-1` wall is crossed. `rdom-tui` *reads* the flag and interprets it
  as "autoscroll scroll ancestors" (rdom-core stays renderer-agnostic). See N1.
- **Dropped the "zero new consumer code" claim.** The autoscroll pointer is *beyond the edge* → no
  cell under it → a target-node consumer (the grid) must extend via **clamped `client_x/client_y` →
  cell**, not `closest("td")`. The substrate's job is the tick + scroll + a faithful synthetic move
  carrying coords; the consumer maps+clamps. Text selection gets it ~free (runtime clamps to the
  nearest text position). See N2 + "Consumer autoscroll contract."
- **`event.target` under capture stays DOM-standard** (the captured node). v2 proposed a divergence
  (target = under-pointer); it's unnecessary since drags read coords, not `target`. See N2.
- **The autoscroll tick is a built-in event-loop phase woken by a timer, not a user `set_interval`**
  (it needs router + relayout + dispatch, which `TimerCtx` can't do). See N3.
- **Phase-0 gate sharpened** to a concrete `pump_one_autoscroll_tick()` headless hook. See N4.

**Implementation notes (after Phases 0–1):** the substrate was further along than the design
assumed — pointer capture already lived in `rdom-core` (used by scrollbar + text selection),
`Event.is_synthetic`/`with_synthetic` already existed, and the scheduler clock was already virtual.
So Phase 0 = a deterministic `App::advance(ms)`; Phase 1 = just the `Dom::drag_autoscroll` flag +
confirming the `prevent_default` precedence. No `EventCtx`/App reach-through was needed; capture +
precedence pre-existed. The phase descriptions below reflect this.

**v2 changes (first review):** decoupled capture from autoscroll (capture default, autoscroll
opt-in); unified single-owner capture + precedence; specified the per-tick mini-frame and its
virtualization interaction; specified the synthetic-event contract incl. `mouseover`/`mouseout` +
`synthetic` flag; grid-first sequencing; cut chaining + the inside-edge band; added the clock gate.

## Origin

Surfaced building `rdom-virtualtable`'s mouse cell-selection: dragging a range stops at the viewport
edge. Browsers autoscroll the scroll container during a drag (text selection, DnD, custom grids).
rdom does neither — `runtime::selection::drag::extend`
(`crates/rdom-tui/src/runtime/selection/drag.rs`) has no scroll logic, and a consumer drag can't
extend past what's materialized because the runtime doesn't know a consumer drag is happening and
`mousemove` doesn't fire while the pointer is held still past the edge. This is cross-cutting runtime
capability — it must serve text selection, the table, and any future drag (reorder, resize, DnD) in
**any** scroll container.

## Two decoupled pieces (the rule)

The review's headline correction: these are **orthogonal** and must not be conflated.

1. **Pointer capture** — "route all pointer events for this drag to node X, even outside its box,
   and guarantee X gets the `mouseup`." This is `setPointerCapture`. It does **not** scroll anything.
   A volume slider or drag-handle captures and must **never** autoscroll its scroll ancestor.

2. **Drag autoscroll** — "while *this* drag's pointer dwells at a scroll container's edge, scroll the
   container on a timer tick and re-evaluate the drag at the held pointer." This is an **opt-in
   behavior layered on a capture**, not implied by it.

> While a captured drag has **autoscroll enabled** and the pointer **dwells** at (or beyond) a
> scrollable ancestor's edge, the runtime scrolls that ancestor one step per tick; each tick then
> **re-evaluates the drag at the pointer's current position** (relayout → re-hit-test → synthetic
> `mouseout`/`mouseover`/`mousemove`, and native text-selection extends on that move).

There is exactly **one** implementation of autoscroll, in the runtime. Text selection and consumer
drags are **consumers** of it. **If `selection::drag` and the virtual table don't share this path,
the design is wrong** — the one thing we refuse to do is implement autoscroll twice.

### Why re-dispatch the pointer (mechanic)

After a tick scrolls the container, the content under the held pointer changed, so the runtime
re-hit-tests and re-dispatches the pointer (rather than firing a bare `scroll` event). The drag's
update logic — native text-selection extend, or the grid's range extend — runs on that synthetic
move, with the **same handler as a real move**. One tick mechanism; every drag type reuses it.

**What this does NOT do (corrected from v2):** it is *not* "zero new consumer code." The autoscroll
scenario is precisely *pointer at/beyond the container edge* — so the cell "under" the held pointer
is often **nothing** (the pointer sits below the last row, over empty space). A target-node mapping
(`closest("td")`) therefore finds no cell and the range wouldn't grow. So an autoscroll consumer
extends via the synthetic move's **`client_x/client_y`, clamped to the container's cell grid** (see
"Consumer autoscroll contract"). Native text selection gets this nearly free — the runtime clamps to
the nearest text position. The substrate's contract is: deliver the tick + the scroll + a faithful
synthetic move carrying coords; the consumer maps-and-clamps. Rejected alternative: a bare `scroll`
event with no pointer — that forces the consumer to *also* cache the pointer position the synthetic
move already carries.

## Pointer capture — unified single-owner model (resolves B2)

rdom already has two capture-like router states — `selection_drag` and `scrollbar_drag`
(`crates/rdom-tui/src/runtime/router/mod.rs`). Adding a third, parallel public capture would make
`mousedown` routing a minefield. **Unify them under one owner:**

```rust
enum PointerCapture {
    TextSelection,                              // armed by the router's selection::drag::begin
    Scrollbar,                                  // armed by scrollbar thumb/track drag
    Element { node: NodeId, autoscroll: bool }, // armed by a consumer's set_pointer_capture
}
// router holds: pointer_capture: Option<PointerCapture>
```

**Precedence (decided):** at most one owner at a time.
- A `mousedown` on a scrollbar thumb/track claims `Scrollbar` and never dispatches to content, so it
  can't collide with a consumer.
- For a `mousedown` on content: the router begins its implicit `TextSelection` capture, then
  dispatches the event. If a consumer handler calls `set_pointer_capture(node)` during that
  dispatch, it **supersedes** the text-selection capture (the consumer explicitly claimed the
  pointer — e.g. the grid claiming a cell drag instead of a text selection). One owner wins; the
  superseded text-selection is cancelled.
- `mouseup` releases the capture **and is guaranteed delivered to the owner** even if the release
  lands off any element — that's the point of capture. (Confirm the router delivers a release the
  terminal reports outside all element rects; if not, that's a Phase-1 fix, not an open question.)

**Where the state lives — IMPLEMENTED (N1, simpler than feared).** Pointer capture already lives in
`rdom-core` on the `Dom`: `set_pointer_capture` / `pointer_capture` / `release_pointer_capture` are
public (`dom.rs`), already used by scrollbar drag *and* text selection (`selection::drag::begin`
calls `set_pointer_capture`), and auto-released on `mouseup`. So `setPointerCapture` is already the
single capture slot, and a `Dom`-only `EventCtx` already reaches it (`ctx.dom.set_pointer_capture`).
Phase 1 adds one thing: a generic **`drag_autoscroll: bool`** on the `Dom` (+
`set_drag_autoscroll` / `drag_autoscroll`), reset on every capture change and on release.
`rdom-core` stays renderer-agnostic — it stores a bool; `rdom-tui`'s loop reads `dom.drag_autoscroll()`
to decide whether to run the autoscroll phase. No new `EventCtx` methods, no App reach-through — the
`SHOWCASE-EVT-1` wall was never actually in the way for capture.

**Precedence — IMPLEMENTED via the existing `prevent_default` gate (B2).** `handle_down` runs its
defaults (scrollbar hit → focus → `selection::drag::begin`) **only when the mousedown wasn't
`prevent_default`ed** (`router/mouse/mod.rs:124`). So a consumer that captures in its mousedown
handler **and calls `prevent_default`** owns the drag — the runtime's text-selection/scrollbar
defaults never run, and its capture stands. No router refactor and no `selection_drag` unification
needed: `dom.pointer_capture` is the one owner; `selection_drag`/`scrollbar_drag` are per-kind
extend-state riding alongside it. (Pinned by `consumer_capture_with_prevent_default_beats_text_selection`.)

**`event.target` under capture — stays DOM-standard (N2 revised).** The existing captured-move path
dispatches with `target = the captured node` (DOM-standard). The v2 doc proposed a divergence
(target = under-pointer); it's **unnecessary** — the consumer contract has drags read **coords**, not
`target`, so target semantics don't matter to a drag. Left as-is.

**N5 (moot here).** The grid sets `user-select: none` AND `prevent_default`s, so the runtime's
text-selection default never arms on its cells — the supersede path isn't exercised.

Public API (on `EventCtx`, `crates/rdom-core/src/dispatch.rs:63`):

```rust
impl EventCtx<'_, Ext> {
    /// Route all subsequent pointer events for the in-progress drag to `node`
    /// (even outside its box); the owner is guaranteed the matching mouseup.
    /// Does NOT autoscroll. Released on mouseup or release_pointer_capture.
    pub fn set_pointer_capture(&mut self, node: NodeId);
    pub fn release_pointer_capture(&mut self);

    /// Opt the in-progress captured drag into edge autoscroll (see below).
    /// No-op without an active Element capture. Must be called from the same
    /// mousedown handler that captured.
    pub fn enable_drag_autoscroll(&mut self);
}
```

(A `capture_for_drag_select(node)` convenience = `set_pointer_capture` + `enable_drag_autoscroll` may
be added once both exist; keep the primitives separate underneath.)

## Drag autoscroll — the per-tick contract (resolves B3)

Autoscroll is dwell-gated and tick-driven. While a capture with `autoscroll = true` is active:

**Arming.** A real pointer move that lands in an edge zone (below) arms a **built-in event-loop
autoscroll phase woken by a timer** (period ~50 ms; one internal constant). It is **not** a user
`set_interval`: the tick needs router state (the capture + held pointer), a scroll, a relayout, and
a dispatch — none of which a scheduler callback (`TimerCtx` = dom + scheduler) can do. It is
**disarmed** the instant any of: the pointer leaves the zone, the drag ends, or the target container
hits its scroll limit on the needed axis. No poll-spin — when disarmed the loop blocks on input as
usual. rdom's loop already wakes on a timer deadline for the scheduler, so the wake path exists;
autoscroll arms/clears its own deadline.

**Each tick runs, in this exact order** (a mini-frame — *not* a shortcut):
1. Compute the per-axis step from pointer-vs-edge (see speed model).
2. Apply the scroll offset to the target container (`set_scroll_top` / `set_scroll_left`).
3. Run the resulting `scroll` event's handlers — **the consumer may re-window here** (drop + rebuild
   rows). This is the virtualization interaction the review flagged.
4. **Re-run layout** so any nodes the scroll handler created have current rects.
5. Re-hit-test the **held pointer screen cell** → the element now under it.
6. Fire synthetic pointer events at the pointer (see contract): `mouseout` on the previous element +
   `mouseover` on the new one if it changed, then `mousemove`. Native text selection extends on this
   move; the grid's handler extends the rectangle on it.

Steps 3–4 **must complete before step 5**, or the re-hit-test targets a stale or just-dropped node —
straight back into `CASCADE-FREED-ROOT-1` territory. Re-use the normal frame's mutate→layout→dispatch
pipeline; do not invent a parallel ordering.

**Re-entrancy guards (resolves the feedback hazard):**
- A re-entrancy flag is set for the duration of a tick; `set_pointer_capture` /
  `enable_drag_autoscroll` / autoscroll arming are no-ops while it's set. One tick = at most one
  scroll step per container + one synthetic event burst. No recursion.
- The step is computed from absolute pointer-vs-edge each tick (idempotent), never from accumulated
  deltas. The consumer's move handler **must not** itself scroll the same container (documented
  contract); the runtime owns the scroll during autoscroll.
- All `NodeId`s the tick holds are re-validated after step 3 (a handler may have freed them); the
  substrate tolerates freed cascade roots now, but the tick must not deref a stale id.

### Edge zone + speed (cell-grained) — resolves S1

TUI is cell-grained; do not port the browser's pixel-accelerated curve.
- **Zone:** the container's **boundary cell on a scrollable axis, and everything beyond it.** The
  review argued "beyond-only" to avoid autoscrolling while selecting the last visible row — but a
  container flush with the terminal edge has no "beyond" (crossterm clamps coords), so beyond-only
  would never autoscroll there. **Resolution: include the boundary cell, but autoscroll is
  dwell-gated by the tick interval** — a quick drag *through* the last row (move in, move out within
  one period) never ticks; only *holding* at the edge scrolls. That's exactly the desired behavior
  and it works at the terminal edge too.
- **Step:** 1 cell/tick at the boundary; 2–3 cells/tick when the pointer is *beyond* the edge (cheap
  acceleration). Internal constants, tunable, **not** public config yet.

### No scroll chaining in v1 — resolves S2

Autoscroll the **innermost scrollable ancestor of the pointer that can move the needed axis**; if
it's at its limit, **stop** (do not chain to an outer container). The driver (the table) has one
scroll container; chaining is its own surprise surface ("inner hits limit → outer jumps"). Add it
when a real nested-scroller consumer exists.

## Synthetic-event contract (resolves B4)

The runtime synthesizes pointer events during autoscroll. Faithfulness is load-bearing — the grid's
move handler guards on `buttons & 1` and reads modifiers, so a thin/incorrect synthetic event would
be **rejected** and "zero new consumer code" would be a lie.

Each synthetic event carries:
- `buttons`: the held bitmask captured at drag start (left = bit 0). Non-zero, or guards drop it.
- `modifiers`: the modifiers in effect at the last real move.
- `client_x` / `client_y`: the **held pointer screen cell** (unchanged across ticks — the *content*
  moved, not the pointer).
- A new `synthetic: bool` on `MouseDetail` (`crates/rdom-core/src/...`), `true` for these, so apps
  that count/analyze input can distinguish them. (Small additive rdom-core change.)

Because the element under the pointer changes as content scrolls, a tick fires the faithful
**`mouseout`(old) + `mouseover`(new)** pair before `mousemove` — otherwise `:hover` styles go stale
mid-autoscroll. (The first tick may have no "old" element.)

## Consumer autoscroll contract (N2)

A drag consumer that wants its selection to follow an autoscroll does **not** map the synthetic
move's *target node* (it's beyond the edge — empty space). It maps the move's **`client_x`/
`client_y`, clamped to the scroll container's cell grid**, to a logical position. For
`rdom-virtualtable`:

```text
on the (real or synthetic) drag mousemove while captured:
    let (cx, cy) = (detail.client_x, detail.client_y)
    clamp cy into [tbody.top, tbody.bottom-1]  → window row → logical row (via window_start)
    clamp cx into the column band               → column
    extend_selection_to(row, col)
```

So adopting autoscroll in a consumer means: (1) `set_pointer_capture` + `enable_drag_autoscroll` on
mousedown, and (2) extend on the move via **clamped coords** instead of `closest("td")`. Modest, not
zero — but the edge-zone math, the scroll, the tick cadence, and the synthetic re-dispatch all live
in the substrate, used identically by text selection. (The grid's *non-drag* click mapping can keep
using `closest("td")`; only the drag-extend path needs coords.)

## Boundary: virtualization without a real scroll container

`rdom-virtualtable` can window via `show_window` with **no** `overflow:scroll` element. Autoscroll
acts on **real scroll containers only** (it changes a node's scroll offset). In pure-windowed mode
there's nothing to scroll, so autoscroll is inert and the table must advance its own window — a
consumer concern, out of scope here. The common interactive setup (`enable_scrollbar` → a real
scrollable `<tbody>`) is fully covered.

## Divergences from the web (for `DIVERGENCES.md` when shipped)
- Pointer capture is **mouse-only** (no touch/pen, no `pointerId`).
- Autoscroll is **explicit opt-in** layered on capture; the web autoscrolls implicitly during native
  text selection / DnD. rdom arms those internally, so observable behavior matches for text
  selection, but a custom captured drag must opt in (stricter, and avoids the slider-scroll trap).
- Edge zone + speed are cell-grained, dwell-gated, and fixed — not the browser's pixel curve.
- No scroll chaining (v1).

## Test plan
- **Phase-0 gate (must pass before Phase 2):** a deterministic, headless **`pump_one_autoscroll_tick()`**
  hook (or equivalent on the test `App`) that runs the full tick once — scroll → scroll-handlers →
  relayout → re-hit-test → synthetic dispatch — against a `TestBackend`, no wall clock, no real
  input. Every autoscroll test below drives this. If the loop can't be pumped one tick at a time
  headlessly today, **building that hook is the first sub-task** (it's also generally useful for
  testing any timer-driven runtime behavior).
- **Capture (Phase 1):** `set_pointer_capture` routes moves outside the node to it; `mouseup` off
  any element still reaches the owner; a consumer capture supersedes the implicit text-selection
  capture for the same mousedown.
- **Autoscroll tick (Phase 2, fake clock):** capture + `enable_drag_autoscroll`; synthesize a drag
  held at a scrollable `<div>`'s bottom edge; advance N ticks; assert `scroll_top` advanced the
  expected steps and a synthetic `mousemove` (with `buttons & 1`, `synthetic = true`) fired at the
  held cell each tick. Repeat top/left/right and "beyond the edge" (faster step).
- **Dwell gating:** a move that enters then leaves the zone within one period → zero ticks.
- **Disarm:** pointer leaves zone / drag ends / container at limit → interval cancelled; no scroll on
  later clock advances.
- **Re-entrancy:** a move handler that `drop_subtree`s the row under the pointer mid-tick must not
  panic; a handler that scrolls the same container must not compound.
- **Hover fidelity:** as content scrolls under a held pointer, `mouseout`/`mouseover` fire and a
  `:hover` rule tracks the newly-revealed element.
- **Grid (rdom-virtualtable):** drag a cell range past the `<tbody>` edge → the window scrolls and
  the rectangle extends to revealed rows. Table changes are exactly: `set_pointer_capture` +
  `enable_drag_autoscroll` on mousedown, and a drag-extend path that maps **clamped coords → cell**.
- **Text selection:** drag-select past a scrollable `<div>`'s edge → the Range grows and the div
  scrolls.

## Implementation phases (re-sequenced grid-first — resolves S3)

0. **Gate — DONE.** `App::advance(ms)` advances the virtual scheduler clock + services due
   timers/microtasks/rAF headless and deterministically (the scheduler clock was already virtual).
   Pinned by `advance_drives_a_scheduler_interval_deterministically`.
1. **Pointer capture — DONE.** Capture already lived in `rdom-core` on the `Dom`; added the generic
   `drag_autoscroll: bool` (+ `set_drag_autoscroll`/`drag_autoscroll`, reset on capture change +
   release). Precedence is the existing `prevent_default` gate — a consumer that captures +
   `prevent_default`s its mousedown owns the drag. `event.target` stays DOM-standard. No router
   refactor, no new `EventCtx` methods. Pinned by `drag_autoscroll_*` (rdom-core) +
   `consumer_capture_with_prevent_default_beats_text_selection` (rdom-tui).
2. **Drag autoscroll — DONE (vertical).** A built-in loop phase (`service_autoscroll`, keyed on the
   scheduler clock so it fires under both the live loop and `advance`): when an autoscroll-armed
   captured drag dwells in a scroll container's vertical edge zone, each ~50ms tick scrolls the
   nearest scroll container one step toward the pointer (`scrollbar::autoscroll_target` clamps the
   pointer into the captured box → hit-test → walk up; `autoscroll_step` scrolls + dispatches the
   `scroll` event so the consumer re-windows), then re-dispatches a synthetic left-button `Drag` at
   the held pointer through the router so the consumer extends. `compute_poll_timeout` floors to the
   period while armed; the tick's `while`-loop has an 8-iter guard; re-entrancy is avoided because
   the re-dispatch routes (not `handle_event`), so `note_autoscroll` isn't re-invoked. Consumers opt
   in with `dom.set_drag_autoscroll(true)`. Pinned by
   `drag_autoscroll_scrolls_a_container_held_at_the_edge`.
   **Scoped out of v1 (deferred):** *horizontal* autoscroll (pairs with `SCROLL-CROSS-AXIS-1`); the
   `MouseDetail.synthetic` flag (informational only — the re-dispatched move is faithful in
   buttons/coords/modifiers, which is what consumers need); and a *mid-tick relayout* before the
   synthetic move (only matters for a layout-dependent consumer like text selection's `position_at`
   — added in Phase 4 if needed; the grid maps `window_start` + coords, which the `scroll` event
   already updated, so it needs no mid-tick layout). Hover (`mouseout`/`mouseover`) stays suppressed
   during a captured drag, matching the existing capture path.
3. **Adopt in `rdom-virtualtable` first** (the driver, low risk): `set_pointer_capture` +
   `enable_drag_autoscroll` on the cell-drag `mousedown`, and switch the drag-extend path to
   **clamped `client_x/client_y` → cell** (per the consumer contract; the click path keeps
   `closest("td")`). Verify the rectangle extends across an autoscroll. This proves the public API.
4. **Native text selection — DONE (minimal hook).** `selection::drag::begin` now calls
   `dom.set_drag_autoscroll(true)` (it already captured the pointer), so a text-selection drag
   autoscrolls like a browser. The autoscroll tick re-lays-out before the synthetic move, so text
   selection's layout-dependent `position_at` re-extends at the revealed content. Pinned by the
   `drag_autoscroll` assertion in `mousedown_alone_leaves_collapsed_caret_not_whole_paragraph`. No
   `selection::drag` rewrite was needed — it rides the one primitive unchanged.
5. Ship as an `rdom-tui` minor bump; `DIVERGENCES.md` + `TECH_DEBT` (`DRAG-AUTOSCROLL-1`) + `STATE.md`.

## Open questions (genuinely open; the rest are decided above)
- **Horizontal autoscroll vs. `SCROLL-CROSS-AXIS-1`** (flex scroll only translates the main axis
  today): a container that can't scroll its cross axis makes cross-axis autoscroll a no-op —
  acceptable, but the two items may want to land together.
- **Exact period + step constants** — pick by feel during Phase 2; not a correctness question.
