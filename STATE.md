# STATE — project journal

Living progress ledger for `rdom`. Updated whenever a change records a meaningful decision, completes a milestone, opens a risk, or shifts direction.

For the durable architecture and roadmap, see [`specs/DESIGN.md`](specs/DESIGN.md). For the current major project plan (0.2.0 = `rdom-showcase` + event bundle + `calc()`), see [`specs/SHOWCASE.md`](specs/SHOWCASE.md). For accepted tech debt, see [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md).

## Current focus

**Release in flight:** 0.2.0. Three workstreams bundled under one release — `rdom-showcase` (headline), event surface bundle, `calc()` value system. Plan: [`specs/SHOWCASE.md`](specs/SHOWCASE.md).

**Next milestone:** M1 — Substrate honesty. Three blocking deliverables; **D1 done**, D2 and D3 next.

**Status:** M1 D1 (multi-slot stylesheet API) landed in `c585065`, with a follow-up fix in `adf14be` after the grumpy architect review found `invalidate_cascade` peeked instead of draining the dirty tracker. 2,295 workspace tests passing. M1 D2 (subtree-replacement contract + tests) is next.

## 0.2.0 milestone status

- [ ] **M1** — Substrate honesty *(in progress)*:
  - [x] D1 — Multi-slot stylesheet API (`App::push_stylesheet` / `remove_stylesheet` + `cascade_all` / `cascade_subtrees_all`). Commit `c585065`, with `adf14be` fixing a latent `invalidate_cascade` bug surfaced by grumpy architect review.
  - [ ] D2 — Subtree-replacement contract + integration tests.
  - [ ] D3 — Focus-on-detach specification.
- [ ] **M2** — Showcase scaffold (`crates/rdom-showcase/`, `Demo` trait, static first demo). *Showcase.*
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

(none recorded yet)

## Recent decisions

### 2026-05-22 — M1 D1 grumpy architect review found dirty-tracker peek-vs-drain bug

`App::invalidate_cascade` (and the v0.1.0 `set_stylesheet` it was extracted from) called `DirtyTracker::roots_snapshot` — a peek — when the intent was to drain. Effect: when the DOM had pending dirty subtrees at the moment of `push/remove/set_stylesheet`, the next paint did a partial subtree cascade instead of the full re-cascade the API contract promises. Elements outside the dirty subtree kept stale computed styles from the previous sheet stack. Pre-existing latent bug in v0.1.0; surfaced by the M1 D1 design review because multi-sheet mutation makes the violation observable.

Regression test added (two siblings, mutate one, swap a sheet, assert the un-mutated sibling re-cascaded). One-line fix: `roots_snapshot` → `take_roots`. Commit `adf14be`.

Non-blocking findings from the same review, recorded as follow-up work (not in this milestone):
- Parallel-vec storage (`stylesheets` + `stylesheet_ids`) is a hidden invariant; cleaner shape is `Vec<(StylesheetId, Stylesheet)>` with `style_sheets()` returning an iterator. Acceptable churn for 0.2.0 if we want to address.
- `merge_root_vars` allocates per element (matches pre-existing `root_vars_rc` allocation pattern in single-sheet code; absolute cost worse under multi-sheet). Compute once per cascade pass, thread `&VarMap` down — probably 5× faster on the vars step. Tech debt entry candidate.
- `set_stylesheet` doesn't return a `StylesheetId` (asymmetric with `push_stylesheet`); minor API smell.
- Empty-stylesheets edge case is reachable and untested.

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
