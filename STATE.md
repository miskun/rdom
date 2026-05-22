# STATE — project journal

Living progress ledger for `rdom`. Updated whenever a change records a meaningful decision, completes a milestone, opens a risk, or shifts direction.

For the durable architecture and roadmap, see [`specs/DESIGN.md`](specs/DESIGN.md). For the current major project plan (0.2.0 = `rdom-showcase` + event bundle + `calc()`), see [`specs/SHOWCASE.md`](specs/SHOWCASE.md). For accepted tech debt, see [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md).

## Current focus

**Release in flight:** 0.2.0. Three workstreams bundled under one release — `rdom-showcase` (headline), event surface bundle, `calc()` value system. Plan: [`specs/SHOWCASE.md`](specs/SHOWCASE.md).

**Next milestone:** M3 — Sidebar navigation + per-demo subtree swap.

**Status:** **M1 + M2 closed.** M2 ships the static showcase scaffold at `crates/rdom-showcase/`; binary builds, integration tests pass, one demo (HelloWorld) wired through the Demo trait + shell layout. 2,320 workspace tests passing.

One piece of architectural debt deferred with teeth: `EVT-DETACH-1` (implicit blur/focusout/mouseleave on detach) is tracked in [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md) and listed as a non-negotiable M5 deliverable in [`specs/SHOWCASE.md`](specs/SHOWCASE.md). M5 cannot ship `mouseleave` for explicit motion without closing this.

## 0.2.0 milestone status

- [x] **M1** — Substrate honesty *(closed 2026-05-22)*:
  - [x] D1 — Multi-slot stylesheet API (`App::push_stylesheet` / `remove_stylesheet` + `cascade_all` / `cascade_subtrees_all`). Commit `c585065`, plus grumpy-review follow-ups: `adf14be` (drain dirty tracker, not peek), `82a2dbe` (set_stylesheet returns id + empty-sheets test), `e5b4e89` (tuple-vec storage + per-pass vars merge).
  - [x] D2 — Subtree-replacement contract + integration tests. 15 contract tests under `crates/rdom-tui/tests/subtree_replacement_contract.rs`. Root-cause fix in `rdom-core::tree::detach_from_parent` adds a `purge_interaction_state_for_subtree` helper so every detach path cleans up focused/hovered/pointer_capture/selection. Commits `245c626`, `41f9f76` (review follow-ups + EVT-DETACH-1).
  - [x] D3 — Focus-on-detach specification. Folded into D2 (same fix surface): `dom.focused()` clears synchronously on detach (matches the web); the no-`blur`-event divergence documented in [`specs/DIVERGENCES.md`](specs/DIVERGENCES.md) §"Runtime & focus" and tracked as **`EVT-DETACH-1`** in [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md), blocking on M5.
- [x] **M2** — Showcase scaffold *(closed 2026-05-22)*. `crates/rdom-showcase/` workspace member (`publish = false`). Public API: `Demo` trait, `Category` enum, `Source` struct, `DEMOS` registry, `build_shell(&mut TuiDom) -> ShellHandles`, `base_stylesheet()`. Shell structure is native HTML (`<header>`/`<aside>`/`<nav>`/`<main>`). One demo (`HelloWorld`) wired end-to-end. Two stylesheets registered (base + demo) exercising M1's multi-slot API. 9 tests (3 unit registry, 3 unit shell, 3 integration scaffold). Commit `0c25920`.
- [ ] **M3** — Sidebar nav + per-demo subtree swap + sheet stack push/pop. *Showcase.*
- [ ] **M4** — Examples-to-demos refactor; closes `OPS-4` (snapshot pinning for the seven older examples). *Showcase.*
- [ ] **M5** — Event surface bundle: `dblclick`, `contextmenu`, `keyup`, `mousemove`, `scroll`, `resize`. *Substrate.*
- [ ] **M6** — `calc()` value system in `rdom-style`/`rdom-css`, resolved at cascade/layout time. *Substrate.*
- [ ] **M7** — Showcase polish: source view + CLI deep-link + M5 event integration. *Showcase.*
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
