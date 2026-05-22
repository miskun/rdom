# STATE — project journal

Living progress ledger for `rdom`. Updated whenever a change records a meaningful decision, completes a milestone, opens a risk, or shifts direction.

For the durable architecture and roadmap, see [`specs/DESIGN.md`](specs/DESIGN.md). For the current major project plan (0.2.0 = `rdom-showcase` + event bundle + `calc()`), see [`specs/SHOWCASE.md`](specs/SHOWCASE.md). For accepted tech debt, see [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md).

## Current focus

**Release in flight:** 0.2.0. Three workstreams bundled under one release — `rdom-showcase` (headline), event surface bundle, `calc()` value system. Plan: [`specs/SHOWCASE.md`](specs/SHOWCASE.md).

**Next milestone:** M3 — Sidebar navigation + per-demo subtree swap.

**Status:** **M1 + M2 closed.** M2 visual review surfaced SEVEN substrate-honesty gaps over multiple passes; all fixed at root cause per CLAUDE.md "Real Fixes Only" rather than worked around in the showcase. Substrate now provides the canonical CSS author experience: standard CSS chrome layouts render correctly with no rdom-specific idioms required. 2,347 workspace tests passing.

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
