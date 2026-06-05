# DRAG-AUTOSCROLL — pointer capture + edge autoscroll for drags in scroll containers

**Status:** design (2026-06-05). Not yet scheduled to a version. No code until this doc is reviewed.

## Origin

Surfaced building `rdom-virtualtable`'s mouse cell-selection: dragging a range stops at the
viewport edge — you can't select beyond what's currently materialized. Browsers autoscroll the
scroll container during a drag (text selection, drag-and-drop, custom grids) so the selection keeps
growing as the viewport scrolls under the held pointer. rdom does none of this:

- `runtime::selection::drag::extend` (`crates/rdom-tui/src/runtime/selection/drag.rs`) extends the
  text Range to whatever's under the pointer and **has no scroll logic** — native text selection in
  an `overflow: scroll` div dead-stops at the edge.
- A consumer drag (the table's cell selection in `install_mouse`) only extends on `mousemove`, which
  doesn't fire when the pointer is held still past the edge, and the runtime has **no knowledge** a
  consumer drag is even happening.

This is a cross-cutting runtime capability, not a per-widget feature: it must work for text
selection, the virtual table, and any future drag (column reorder, drag-resize, drag-and-drop) in
**any** scroll container (`<div>`, `<tbody>`, anything `overflow: scroll | auto`).

## The one rule (non-negotiable)

> While a **captured pointer drag** is active and the pointer sits within (or beyond) a scrollable
> ancestor's **edge zone**, the runtime scrolls that ancestor on a timer tick; after each scroll it
> **re-evaluates the drag at the pointer's current position** (re-hit-test → re-dispatch
> `mousemove`, and extend native text selection if that's the active drag).

There is exactly **one** implementation of this, in the runtime. Text selection and consumer drags
are **consumers** of it, not parallel copies. **If `selection::drag` and the virtual table don't
share this code path, the design is wrong.** The single biggest failure mode of the naive plan
("substrate autoscrolls text; the table rolls its own") is two divergent edge-zone/speed
implementations and two bug surfaces. This doc exists to prevent that.

### Why re-dispatch `mousemove` (mechanic choice)

After a tick scrolls the container, the cell/text under the held pointer changed. Two ways to tell
the drag:

- **(A · chosen) Re-hit-test the held pointer and re-dispatch a synthetic `mousemove` at it.** Every
  drag handler that already reacts to `mousemove` then "just works" with **zero new code** —
  including `rdom-virtualtable`'s existing `install_mouse` handler, which hit-tests `closest("td")`;
  after the scroll the synthetic move targets the newly-revealed cell and the range extends. Native
  text selection re-extends the same way. Most browser-faithful (engines keep updating
  selection/`dragover` against the moving viewport).
- (B · rejected) Fire only a `scroll` event and make each consumer recompute the head. Forces every
  consumer to cache the last pointer position and own a coords→cell mapping — re-deriving what the
  synthetic move gives for free. More surface, guaranteed divergence.

## How the runtime knows a drag is happening: pointer capture

The runtime tracks *its own* drags (`router.selection_drag`, `router.scrollbar_drag` in
`crates/rdom-tui/src/runtime/router/mod.rs`). It has **no idea** a consumer is dragging — the cell
drag is just "left button held + moves the consumer interprets." Autoscrolling on *any*
left-hold-at-edge is wrong (it fires on plain presses and non-drag holds).

The web already named the fix: **`setPointerCapture` / `releasePointerCapture`.** A drag captures
the pointer on `mousedown`; while captured, the capturing element receives all moves (even outside
its box) **and** the engine autoscrolls. So the substrate addition is two layers:

1. **Public pointer capture.** Expose `set_pointer_capture(node)` / `release_pointer_capture()` on
   the event context. rdom already has *internal* capture state (the router's post-`mousedown`
   capture path, `router/mouse/mod.rs`), so this surfaces and generalizes existing machinery —
   independently useful (sliders, drag handles) beyond autoscroll.
2. **Autoscroll while a captured drag is in a scroll-container edge zone**, on the existing timer
   loop, with the (A) re-dispatch.

**Arming paths, one mechanism:**
- **Native text selection** arms capture internally (it already half-does in `selection::drag::begin`).
- **Consumers** arm it explicitly via `set_pointer_capture` from their `mousedown` handler.

This also closes the drag slice of `SHOWCASE-EVT-1` (an event handler can't reach App-level
machinery): capturing the pointer is the sanctioned way for a handler to say "I'm dragging; deliver
me moves and autoscroll my containers."

## Public API (proposed)

On the event context (`EventCtx`, `crates/rdom-core/src/dispatch.rs:63` — today only `event`, `dom`,
`request_redraw`):

```rust
impl EventCtx<'_, Ext> {
    /// Capture the pointer to `node` for the duration of the current drag.
    /// While captured: all pointer events route to `node`'s handlers (even
    /// outside its box), and the runtime autoscrolls the nearest scrollable
    /// ancestor of the pointer when it enters that ancestor's edge zone,
    /// re-dispatching `mousemove` at the held pointer after each scroll step.
    /// Released automatically on the next `mouseup`, or explicitly.
    pub fn set_pointer_capture(&mut self, node: NodeId);
    pub fn release_pointer_capture(&mut self);
}
```

Autoscroll is **implied by capture** (DOM-faithful: captured drags autoscroll). No separate
"enable autoscroll" flag — a captured drag in an edge zone scrolls; that's the contract. (If a
future consumer wants capture *without* autoscroll, add an opt-out then, not now.)

No new API is needed on the consumer's *move* path: the re-dispatched synthetic `mousemove` flows
through the consumer's existing listener.

## The five blocking resolutions

### 1. One primitive, two first consumers
Implement capture + autoscroll + synthetic-move once. **Port `selection::drag` onto it** (rewrite
its extend to rely on the re-dispatched move; delete any bespoke edge handling). Wire
`rdom-virtualtable` as the second consumer to validate the public API. Ship order: primitive →
text-selection port → grid adoption. The text-selection port is the proof the primitive is real and
not grid-shaped.

### 2. Scroll chaining (nested containers)
When the pointer is over nested scroll containers, autoscroll the **innermost scrollable ancestor
that can still move in the needed direction**; if it's already at its travel limit on that axis,
chain outward to the next scrollable ancestor. Direction is per-axis (a container may autoscroll
vertically but be chained-past horizontally). This mirrors CSS scroll chaining and is the #1 source
of "feels wrong." Reuse the existing nearest-scrollable-ancestor walk
(`runtime::scrollbar::scroll_into_view` already finds scroll ancestors —
`runtime/builtins/tree/mod.rs:217` calls it).

### 3. Re-dispatch re-entrancy & feedback
The synthetic `mousemove` runs consumer handlers that may **mutate/re-window the DOM** mid-tick
(drop + rebuild rows). We *just* fixed a freed-node cascade crash in this exact territory
(`CASCADE-FREED-ROOT-1`). Requirements:
- The tick must be safe against handlers that `drop_subtree` during the synthetic move (snapshot ids
  before, re-validate after — the substrate now tolerates freed cascade roots, but the autoscroll
  loop must not hold a stale `NodeId` across the dispatch).
- **No feedback loop:** a handler that itself scrolls in response to the move must not compound with
  the autoscroll tick. Autoscroll computes its scroll step from the *pointer position vs. edge*, not
  from deltas, so it's idempotent per tick; document that the consumer's move handler must not also
  scroll the same container.
- One tick = at most one scroll step per container + one synthetic move. No recursion.

### 4. Idle CPU — arm/disarm precisely
The autoscroll timer ticks **only** while (capture active) ∧ (pointer in an edge zone) ∧ (the target
container can still scroll that direction). It is **armed** when a captured-drag move lands in an
edge zone and **disarmed** the instant any of those stops being true (pointer leaves the zone, drag
ends, container hits its limit). No poll-spin: when disarmed, the loop blocks on input as usual.
rdom's loop already wakes for `set_interval` timers (`runtime/timers.rs`), so the wake path exists —
autoscroll registers/cancels an internal interval, it does not add a new always-on poll.

### 5. Testability — deterministic time
Autoscroll is timer + loop driven; it must be testable without wall-clock sleeps. The scheduler
already supports a test clock (timers advance via an injected "now"); the autoscroll interval must
thread through it so a test can: capture → synthesize a held drag in the edge zone → advance the
clock N ticks → assert the container scrolled N steps and the selection/range grew accordingly.
**If it can't be unit-tested with a fake clock, the design is incomplete.**

## Edge zone + scroll-speed model

TUI is cell-grained and low-resolution; do **not** port the browser's pixel-accelerated curve.
- **Edge zone:** the outermost **1 cell** on each scrollable axis, plus everything *beyond* the
  container (a drag dragged off the bottom is "in the bottom zone").
- **Step:** **1 cell per tick** while in the 1-cell zone; **2–3 cells per tick** when the pointer is
  *beyond* the edge (cheap acceleration so "drag way past" feels responsive). Constants in code,
  tunable, **not** a public config knob yet.
- **Interval:** a single fixed period (start ~50ms; tune by feel). One knob, internal.

## Boundary: virtualization without a real scroll container

`rdom-virtualtable` can window via `show_window` with **no** `overflow: scroll` element (keyboard
mode). The substrate autoscroll acts on **real scroll containers only** (it scrolls a DOM node's
scroll offset). In pure-windowed mode there's no container to scroll, so autoscroll does nothing and
the *table* must advance its own window — a consumer concern, explicitly out of scope here. The
common interactive setup (`enable_scrollbar` → a real scrollable `<tbody>`) is fully covered: the
runtime scrolls the `<tbody>`, the synthetic move re-targets the revealed cell, the range extends.
State this boundary in the table's docs.

## Divergences from the web (for `DIVERGENCES.md` when shipped)
- Pointer capture is **mouse-only** (rdom has no touch/pen pointers); `setPointerCapture` captures
  "the pointer" = the mouse. No `pointerId`.
- Edge zone + speed are cell-grained and fixed, not the pixel-accelerated browser curve.
- Autoscroll is implied by capture (no separate opt-in), which is stricter than the web (where
  autoscroll also happens during native text-selection/DnD without an explicit capture call — rdom
  arms those internally, so the observable behavior matches).

## Test plan
- **Unit (rdom-tui, fake clock):** capture a node; synthesize a drag held in the bottom edge zone of
  a scrollable `<div>`; advance the clock; assert `scroll_top` increased by the expected steps and a
  synthetic `mousemove` was dispatched at the held position each tick. Repeat for top/left/right and
  for "pointer beyond the edge" (faster step).
- **Scroll chaining:** nested scrollers; inner at its limit chains to the outer.
- **Re-entrancy:** a move handler that `drop_subtree`s the row under the pointer mid-autoscroll must
  not panic (guards #3).
- **Disarm:** pointer leaves the zone / drag ends → no further ticks (assert the interval is
  cancelled; no scroll on subsequent clock advances).
- **Text selection (integration):** drag-select text in an `overflow: scroll` `<div>` past the
  bottom; the Range grows and the div scrolls.
- **Grid (rdom-virtualtable):** drag a cell range past the `<tbody>` edge; the window scrolls and the
  rectangle extends to the newly-revealed rows — with **no new code** in the table beyond
  `set_pointer_capture` on mousedown.

## Implementation phases
1. **Pointer capture API** — surface `set_pointer_capture`/`release_pointer_capture` on `EventCtx`,
   backed by the router's existing capture state; route captured moves to the capturing node; release
   on `mouseup`. (Independently shippable; no autoscroll yet.) `DIVERGENCES` + tests.
2. **Edge autoscroll** — the armed-interval tick: edge-zone detection on the nearest scrollable
   ancestor (with chaining), scroll step, synthetic-move re-dispatch, re-entrancy guards, fake-clock
   tests.
3. **Port `selection::drag`** onto the primitive (text selection autoscrolls; delete bespoke paths).
4. **Adopt in `rdom-virtualtable`** — `set_pointer_capture` on the cell-drag `mousedown`; verify the
   range extends across an autoscroll with no other table changes.
5. Ship as an `rdom-tui` minor bump; `DIVERGENCES.md` + `TECH_DEBT` updates; record in `STATE.md`.

## Open questions (resolve during review)
- **`mouseup` outside the viewport** while captured: capture guarantees we still get the `mouseup`
  (it routes to the capturing node) — confirm the router delivers it when the terminal reports the
  release off any element.
- **Horizontal autoscroll** interplay with `SCROLL-CROSS-AXIS-1` (flex scroll only translates the
  main axis today). If a container can't scroll its cross axis, autoscroll on that axis is a no-op —
  acceptable, but note it; the two items may want to land together.
- **Capture vs. the existing `selection_drag`/`scrollbar_drag`** router state: unify them under one
  capture concept, or let capture coexist as a third kind? Prefer unification, but scope the refactor
  in Phase 1.
