# DRAG-AUTOSCROLL — pointer capture + edge autoscroll for drags in scroll containers

**Status:** design **v2** (2026-06-05, revised after architect review). Not yet scheduled to a
version. No code until reviewed.

**v2 changes (what the review caught):** capture and autoscroll are now **decoupled** — capture is
the default, autoscroll is an explicit opt-in (v1 coupled them, which is wrong: a slider that
captures must not scroll). The **capture-precedence model is now a resolved decision**, not an open
question (a single unified pointer-capture owner). The **per-tick ordering and its virtualization
interaction are specified** (mutate → handlers → relayout → re-hit-test → synthetic event). The
**synthetic-event contract is specified**, including `mouseover`/`mouseout` and a `synthetic` flag.
Sequencing is **grid-first** (text selection adopted after, not before). Scroll **chaining and the
inside-edge band are cut from v1**. A **headless fake-clock pump** is a Phase-0 gate.

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

After a tick scrolls the container, the element under the held pointer changed. Re-hit-testing and
re-dispatching the pointer (not just firing a bare `scroll` event) means every drag handler that
already reacts to `mousemove` extends with **no new code** — the grid's `closest("td")` handler and
native text selection both. Rejected alternative: fire only `scroll` and make each consumer cache
the pointer + own a coords→cell map (re-deriving what the synthetic move gives for free).

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

**Arming.** A real pointer move that lands in an edge zone (below) arms an internal `set_interval`
(period ~50 ms; one internal constant). It is **disarmed** the instant any of: the pointer leaves
the zone, the drag ends, or the target container hits its scroll limit on the needed axis. No
poll-spin — when disarmed the loop blocks on input as usual. (rdom's loop already wakes for interval
timers; autoscroll registers/cancels one.)

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
- **Phase-0 gate (must pass before Phase 2 design is final):** confirm a **headless** test can pump
  the loop's timer phase with a fake clock — advance time, run due intervals, no real terminal/input.
  If not reachable, exposing that pump is a prerequisite sub-task. The whole autoscroll test story
  depends on it.
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
  the rectangle extends to revealed rows, with **no table code beyond** `set_pointer_capture` +
  `enable_drag_autoscroll` on mousedown.
- **Text selection:** drag-select past a scrollable `<div>`'s edge → the Range grows and the div
  scrolls.

## Implementation phases (re-sequenced grid-first — resolves S3)

0. **Gate:** verify / expose the headless fake-clock loop pump (test plan above). Blocks Phase 2.
1. **Pointer capture API + unified owner.** Surface `set_pointer_capture` / `release_pointer_capture`;
   collapse `selection_drag` / `scrollbar_drag` / consumer capture into the one `PointerCapture`
   owner with the decided precedence; guarantee `mouseup`-to-owner. **No autoscroll yet.**
   `DIVERGENCES` + tests. (Independently useful — sliders, drag handles.)
2. **Drag autoscroll** + `enable_drag_autoscroll`: the armed-interval tick (the 6-step contract),
   edge-zone dwell detection (innermost container, no chaining), synthetic-event contract incl.
   `MouseDetail.synthetic` + `mouseout`/`mouseover`, re-entrancy guards, fake-clock tests.
3. **Adopt in `rdom-virtualtable` first** (the driver, low risk): `set_pointer_capture` +
   `enable_drag_autoscroll` on the cell-drag `mousedown`; verify the rectangle extends across an
   autoscroll with no other table changes. This proves the public API.
4. **Adopt in native text selection** (after the API is proven). Start with the **minimal hook** —
   `selection::drag::begin` arms the same autoscroll (the existing extend runs on the re-dispatched
   move). A fuller cleanup/rewrite of `selection::drag` is a **separate** follow-up, not gating this
   work. (The "one primitive" rule forbids two long-term *copies*; it does not force the risky
   rewrite into this release.)
5. Ship as an `rdom-tui` minor bump; `DIVERGENCES.md` + `TECH_DEBT` (`DRAG-AUTOSCROLL-1`) + `STATE.md`.

## Open questions (genuinely open; the rest are decided above)
- **Horizontal autoscroll vs. `SCROLL-CROSS-AXIS-1`** (flex scroll only translates the main axis
  today): a container that can't scroll its cross axis makes cross-axis autoscroll a no-op —
  acceptable, but the two items may want to land together.
- **Exact period + step constants** — pick by feel during Phase 2; not a correctness question.
