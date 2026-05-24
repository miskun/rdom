# SHOWCASE — the 0.2.0 plan

`rdom-showcase` is a permanent, in-tree TUI app that mounts every rdom primitive in a single browsable binary — sidebar tree (collapsible categories + demos), main view (renders the selected demo), built using rdom itself. It ships as the headline feature of **release 0.2.0**, which bundles three workstreams under one release:

1. **`rdom-showcase`** — the showcase app itself (headline). Headline because it's the largest piece, the most user-visible, and the forcing function for the other two.
2. **Event surface bundle** — `dblclick`, `contextmenu`, `keyup`, `mousemove`, `scroll`, `resize`. Substrate addition; the showcase depends on `resize`/`scroll`/`mousemove`, and `keyup`/`dblclick`/`contextmenu` round out web-platform parity.
3. **`calc()` value system** — substrate addition; lets the showcase's layout coverage demos exercise the full CSS value language.

This document is the durable plan and milestone reference for the whole release. Progress tracked in [`../STATE.md`](../STATE.md).

## Why the showcase

1. **Dogfooding.** Building a non-trivial app with rdom finds substrate gaps single-purpose examples don't.
2. **Single-binary tour.** Newcomers run one binary and see every native element, every cascade behavior, every event, every selection mode, in context.
3. **Forcing function for substrate completeness.** Anything the showcase needs that doesn't exist in the substrate is either a substrate addition we'd ship anyway, or evidence the substrate is missing something real.
4. **CI signal.** Living in-tree means showcase regressions block CI — `rdom-tui` cannot ship a release that quietly breaks the showcase.

## What it is (and isn't)

**Is:**
- An in-tree workspace member at `crates/rdom-showcase/`, marked `publish = false`.
- A consumer of the substrate, identical in shape to any downstream rdom app.
- A permanent fixture maintained alongside `rdom-tui`.

**Is not:**
- A Storybook clone (no React component-isolation model — rdom has elements, not components).
- A component library (substrate-first invariant: zero opinionated components in any in-tree crate).
- A published crate (not on crates.io; not part of the public API surface).
- An "example" (it lives outside `crates/rdom-tui/examples/`, has its own crate, and is too large to fit the example shape).

## Architectural placement

The showcase is a downstream consumer. It adds **zero** new types to `rdom-core`, `rdom-style`, `rdom-css`, `rdom-parser`, or `rdom-tui`. Anything required to build it that doesn't already exist in the substrate gets added to the substrate first (M1, M5, M6), with browser-faithful semantics, tests, and [`DIVERGENCES.md`](DIVERGENCES.md) entries for any deliberate departure.

## Demo authoring contract

Each demo is a value implementing a small trait:

```rust
pub trait Demo {
    fn slug(&self) -> &'static str;          // e.g. "forms/checkbox"
    fn title(&self) -> &'static str;         // human title in nav
    fn category(&self) -> Category;          // taxonomy enum
    fn build(&self, dom: &mut TuiDom) -> NodeId;
    fn stylesheet(&self) -> Stylesheet;
    fn source(&self) -> Source;              // markup + css strings for the Source tab
}
```

The registry is a hardcoded `pub const DEMOS: &[&dyn Demo]` array — explicit, boring, grep-able. No build.rs scanning, no macros, no inventory crate.

Existing `crates/rdom-tui/examples/*.rs` are refactored to expose their demo via `pub fn build(...)` + `pub fn stylesheet()`. A thin `fn main()` shim wraps the exposed pieces for standalone `cargo run --example` use. Single source of truth per demo.

## 0.2.0 milestone plan

Nine internal milestones. Each ends with the two-pass review gate (grumpy chief architect + grumpy chief API) per [`../CLAUDE.md`](../CLAUDE.md). Do not start the next milestone until blocking findings are addressed or recorded as accepted risks in [`../STATE.md`](../STATE.md) and [`TECH_DEBT.md`](TECH_DEBT.md).

The substrate work (M1, M5, M6) and the showcase work (M2–M4, M7–M9) interleave deliberately: substrate honesty (M1) lands before showcase code starts, then the showcase grows the surface that tests the new substrate as it lands.

### M1 — Substrate honesty (showcase prerequisite)

Resolves the three blocking findings from the grumpy architect pass. No showcase code yet.

**Deliverables:**

1. **Multi-slot stylesheet API.** Finish the seam already promised in `App::set_stylesheet`'s doc comment and `App::style_sheets`:
   - `App::push_stylesheet(Stylesheet) -> StylesheetId`
   - `App::remove_stylesheet(StylesheetId)`
   - `App::style_sheets()` returns the real ordered list.
   - Cascade order is push-order (later sheets override earlier, same-specificity).
   - Tests: stack push/pop, removal mid-stack, re-cascade on mutation.
2. **Subtree-replacement contract.** Integration tests under `crates/rdom-tui/tests/` exercising atomic replacement of a subtree's children. Cover cascade reset/reapplication, `MutationObserver` records (removed + added), `DirtyTracker` reset for old / dirty for new, focus disposition on focused-element detach, active selection collapse on anchor/focus detach, scrollbar state cleared on the new subtree.
3. **Focus-on-detach specification.** Either ship browser-faithful behavior (detaching focused element moves focus to `body`, fires `blur`) with a test, or add a [`DIVERGENCES.md`](DIVERGENCES.md) entry with rationale.

**Exit criteria:** all three deliverables landed; workspace gate clean (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`); review gate run.

### M2 — Showcase scaffold

First runnable binary. No navigation yet.

**Deliverables:**

1. `crates/rdom-showcase/` workspace member, `publish = false`, root `Cargo.toml` updated.
2. `Demo` trait + `Category` enum + hardcoded registry stub (one demo: "Hello world").
3. Shell layout: sidebar (fixed width, list of demo titles, non-interactive), main view, header. Native HTML + CSS only.
4. Main binary mounts demo at index 0 statically.
5. Smoke test: `cargo run -p rdom-showcase` renders shell + hello-world demo.
6. Unit test: shell mounts without panic; first demo's root node is attached.

**Exit criteria:** binary runs; shell renders; one demo visible; review gate run.

### M3 — Sidebar navigation + per-demo subtree swap

The showcase becomes browsable.

**Deliverables:**

1. Sidebar tree using `<details>`/`<summary>` for categories (UA-rendered disclosure triangles for free).
2. Mouse click on a demo entry → swap main-view subtree to that demo + push the demo's stylesheet on the App's sheet stack (pop previous on leave). Uses M1 multi-stylesheet + subtree-replacement.
3. Keyboard navigation: `Tab`/`Shift+Tab` between sidebar and main, arrow keys within the tree, `Enter` to load.
4. Demo categories taxonomy (final form): `Layout`, `Forms`, `Text`, `Events`, `Cascade`, `Selection`, `Editing`, `Built-ins`, `Pseudo-elements`, `Positioning`, `Animations`.
5. Subtree-swap correctness verified using M1's tested contract — focus resets cleanly, selection collapses, no cascade leaks from the previous demo.

**Exit criteria:** can browse between three placeholder demos using both mouse and keyboard; sheets stack correctly; review gate run.

### M4 — Examples-to-demos refactor (closes `OPS-4`) — **CLOSED 2026-05-23**

The 10 existing `crates/rdom-tui/examples/*.rs` (originally planned as 7; two more had been added in the interim, plus `app_shell` which already had a snapshot) became showcase demos without losing their standalone-run shape. The canonical implementation now lives in `rdom_showcase::demos::*`; each example file is a 1-line shim calling `run_standalone()`.

**Shipped:**

1. Each example exposes `pub fn build(dom: &mut TuiDom) -> NodeId` + `pub fn stylesheet() -> Stylesheet` + `pub fn source() -> Source` + `pub fn run_standalone() -> io::Result<()>` + a `Demo` impl on a unit struct.
2. Standalone `cargo run -p rdom-tui --example <name>` still works for every one. Requires `rdom-showcase` as a `[dev-dependencies]` of `rdom-tui` (dev-deps don't participate in the runtime graph, so the coupling-direction inversion is bounded to test/example builds).
3. All 10 demos registered in `DEMOS`. Showcase grew from 3 → 13 demos across 7 categories.
4. Paint snapshots in `crates/rdom-tui/tests/snapshots/` pin every demo's output at fixed viewports. `OPS-4` retired.

**Side-fix:** the M2 default `flex-shrink: 1` (CSS-correct) interacted badly with terminal integer-cell allocation, dropping `height: 1` items to zero cells under overflow. Pre-existing bug surfaced by porting `sticky_demo`. Author-side fix (`flex-shrink: 0` on fixed-height children) applied across affected demos; root-cause substrate fix tracked as expanded `M5-MIN-CONTENT-1` in [`TECH_DEBT.md`](TECH_DEBT.md) (height-axis `min-height: auto` floor).

### M5 — Event surface bundle — **CLOSED 2026-05-23**

Substrate work. Six new events lift `rdom-tui`'s event coverage to the 0.2.0 set. Also closes the implicit-detach-events gap left open in M1 (`EVT-DETACH-1`).

**Deliverables:**

1. **`dblclick`** — synthesized on the nearest common ancestor of two consecutive `click`s within the platform double-click window. Matches HTML.
2. **`contextmenu`** — fires on right-mouse-button down (and on `Shift+F10` / context-menu key on platforms that send it). Cancelable.
3. **`keyup`** — fires for every key release the terminal reports. Pairs with existing `keydown`.
4. **`mousemove`** — fires while the terminal reports motion events. Respects pointer capture.
5. **`scroll`** — fires on any scrollable element when its `scroll_top` or `scroll_left` changes. Coalesced per rendering step (matches HTML).
6. **`resize`** — fires on `Window` (mapped to the runtime's resize signal) when the terminal grid changes size. Coalesced per rendering step.
7. **`EVT-DETACH-1` closure** — implicit `blur` / `focusout` / `mouseleave` / `mouseout` dispatch when the focused or hovered element (or any ancestor) is detached from the tree. Capture the pre-detach ancestor path in `rdom-core::tree::detach_from_parent`, plumb through a new `Mutation` shape, install an `App`-level observer that translates those records into `dispatch_tui_event` calls. See `EVT-DETACH-1` in [`TECH_DEBT.md`](TECH_DEBT.md) for the full rationale. **Non-negotiable** for M5 — shipping `mouseleave` for explicit motion without closing the implicit-detach path leaves `rdom-tui` with two inconsistent hover-event models.

Each event needs: a public event type, dispatch wired through the existing 3-phase pipeline, integration tests covering cancellation / propagation / `AbortSignal` removal, and (where applicable) a [`DIVERGENCES.md`](DIVERGENCES.md) entry for terminal-specific behavior.

**Exit criteria:** all six events implemented + tested; `add_event_listener` accepts the new names; `EVT-DETACH-1` closed (entry retired from `TECH_DEBT.md`); the implicit-detach DIVERGENCES.md entries deleted (no longer divergent); review gate run.

### M6 — `calc()` value system — **CLOSED 2026-05-24**

Substrate work. Lets CSS values combine `<length>`s and percentages.

**Deliverables:**

1. **Parser support in `rdom-style`/`rdom-css`** for `calc(<expr>)` where `<expr>` allows `+`, `-`, `*`, `/`, `<length>`, `<percentage>`, `<number>`, nested `calc()`, and grouping with parens. Operator precedence per CSS Values L3.
2. **Resolved value model.** `Length` (and any value type that admits `calc()`) gains a `Calc` variant carrying the unresolved expression. Resolution happens at cascade/layout time when the containing-block dimension is known.
3. **Property coverage.** `width`, `height`, `min-*`, `max-*`, `padding-*`, `margin-*`, `top`/`right`/`bottom`/`left`, `inset`, `flex-basis`, `gap`, `border-*-width`, `font-size` (in `<length>` positions only — no `<angle>` / `<time>` / colors in this milestone).
4. **Tests.** Unit tests in `rdom-style` for parsing + resolution. Integration tests in `rdom-tui` for layout against `calc()`-expressed lengths.
5. **Divergence audit.** Anything we don't ship (e.g. `calc()` in colors, type-checking strictness around units) lands a [`DIVERGENCES.md`](DIVERGENCES.md) entry.

**Exit criteria:** `calc()` parses + resolves for the listed property surface; layout passes use resolved values; review gate run.

### M7 — Showcase polish: source view + CLI deep-link + event integration

Polish to the level a real consumer expects. Uses M5 events.

**Deliverables:**

1. Source tab in the main view. Toggles between "Demo" (live) and "Source" (markup + CSS strings, plain `<pre>`, no syntax highlighting in v1).
2. `resize` integration: shell re-lays-out on terminal resize without artifacts.
3. `scroll` integration: scrolled-position indicator on long demos.
4. CLI deep-link: `cargo run -p rdom-showcase -- --demo forms/checkbox` opens directly to that demo.

**Exit criteria:** source view works for all M4 demos; resize re-layouts without artifacts; deep-link CLI lands user on the named demo; review gate run.

### M8 — Coverage demos

The showcase becomes a *complete* tour of the substrate, not just a wrapper around the existing seven examples. Uses M5 events + M6 calc.

**Deliverables (one demo per bullet, minimum):**

- **Layout:** flex direction (row/column), `justify-content` matrix, `align-items` matrix, `flex-wrap`, `inline-block` chrome, `min-width: auto` and intrinsic sizing, `overflow: auto` with scrollbars, **`calc()` showcase** (mixed `<length>` + `%` expressions).
- **Positioning:** `relative`, `absolute`, `fixed`, `sticky` (`top`/`left` v1), z-index stacking, `inset` shorthand.
- **Forms:** every `<input>` type (text, password, number, checkbox, radio, range, submit, button, reset, hidden, color, search, email, tel, url), `<textarea>`, `<select>`/`<option>`, `<form>` submission.
- **Text & inline:** word wrap, CJK breaks, `<br>`, `white-space: normal|pre|nowrap`, per-grapheme selection.
- **Editing:** single-text-node `contenteditable`, cross-text-node `contenteditable`, `readonly` and `beforeinput` cancellation, caret colors.
- **Selection:** mouse drag, keyboard extension (`Shift+arrow`, `Shift+Home`/`End`, `Ctrl-A`), `user-select: none|all|contain`, clipboard copy.
- **Cascade:** specificity ladder demo, `!important` inversion, custom properties (`var()`), `:hover`/`:focus`/`:checked`/`:open` interactions, `::before`/`::after`/`::selection`/`::backdrop`.
- **Events:** capture/target/bubble visualization, `stopPropagation`/`stopImmediatePropagation`/`preventDefault` toggles, `AbortSignal`-based listener removal, **full M5 event bundle on display** — a "draw your mouse path" demo for `mousemove`, a "right-click anywhere" demo for `contextmenu`, a "resize-aware layout" demo for `resize`, etc.
- **Animations:** CSS `transition` with cubic-bezier timing, `setTimeout`/`setInterval`, `requestAnimationFrame`.
- **Built-ins:** `<details>`/`<summary>`, `<dialog>` (modal + non-modal), `<progress>`, `<meter>`, `<table>` family, `<canvas>` + `RenderContext`, `<a href>` with scheme dispatch + OSC 8.
- **MutationObserver:** live demo where mutating the tree produces visible observer records.

**Exit criteria:** every primitive listed in [`../README.md`](../README.md) §"What's in 0.1.0" and every new 0.2.0 substrate addition (events, `calc()`) has at least one demo; review gate run.

### M9 — CI + snapshots + docs → 0.2.0 release

The showcase becomes a permanent fixture; 0.2.0 ships.

**Deliverables:**

1. `rdom-showcase` added to the workspace `cargo test --workspace` gate.
2. Paint snapshot tests for the shell at three terminal sizes (small/medium/large).
3. Paint snapshot tests for every demo in the registry (generated, not hand-curated).
4. CI runs the showcase test suite on all three platforms.
5. [`../README.md`](../README.md) gains a "Showcase" section pointing to `cargo run -p rdom-showcase`, and the "What's in 0.2.0" section listing events + `calc()` + showcase.
6. [`DESIGN.md`](DESIGN.md) decision archive entries: why the showcase lives in-tree; the M1 multi-stylesheet API; the `calc()` value-resolution timing.
7. Per-crate version bumps, [`CHANGELOG.md`](../CHANGELOG.md), `cargo publish` for the substrate crates that changed (per [`.claude/skills/publish.md`](../.claude/skills/publish.md)). `rdom-showcase` itself stays unpublished.

**Exit criteria:** showcase regressions break CI; docs updated; substrate crates published; 0.2.0 tagged; review gate run.

## Out of scope for 0.2.0

By design, things this release will not include:

- **No component model.** No "args panel," no "controls," no "stories." rdom has elements; the showcase shows elements.
- **No syntax highlighting in source view.** Plain `<pre>`. Can be added later as a separate concern.
- **No URL routing.** Deep-linking is a CLI flag (`--demo path/to/demo`). The 0.3.0 routing primitive lands separately.
- **No async demos** (deferred until 0.4.0 ships async tasks during event handlers).
- **No drag-resizable splitter.** Sidebar width is fixed.
- **No persistent demo state across nav.** Demos rebuild on every visit.
- **No `calc()` in non-`<length>` positions** (no `calc()` in colors, no `calc()` in `<time>` / `<angle>`). Length-and-percentage only in M6.
- **No touch, IME, drag-and-drop, long-press, or other non-terminal input models.** Out of scope for the project, not just this release.

## Verification per milestone

The two-pass gate at the end of each milestone, per [`../CLAUDE.md`](../CLAUDE.md):

1. **Grumpy chief architect pass** — correctness, performance, coupling, god objects, boundaries, duplicated logic, weak contracts, missing tests, operational risk.
2. **Grumpy chief API pass** — does this advance the project toward a browser-faithful, usable DOM for terminals; is the public API something a consumer can build on without surprise; is any divergence deliberate and documented.

Findings recorded in [`../STATE.md`](../STATE.md). Accepted risks go in [`TECH_DEBT.md`](TECH_DEBT.md) with stable IDs.

## Decision archive (0.2.0-specific)

Architectural decisions for this release worth preserving past their original context. Updated as decisions land.

### 2026-05-22: Named `rdom-showcase`, not `rdom-storybook`

Storybook is a React-era tool with an isolated-component model and args/controls UI. rdom has elements, not components. Naming this "Storybook" would invite expectations (parameter knobs, isolation containers) that violate the substrate-first / zero-opinionated-components invariant. "Showcase" / "gallery" carries no such freight.

### 2026-05-22: In-tree at `crates/rdom-showcase/`, not a sibling repo

Sibling repo would lose the CI signal — `rdom-tui` could ship a release that quietly breaks the showcase. In-tree means the showcase is a permanent regression detector and a permanent dogfooding fixture. `publish = false` keeps it off crates.io so the public API surface stays minimal.

### 2026-05-22: Substrate work blocks showcase work, not the reverse

The three blocking findings from the grumpy architect pass (multi-stylesheet API, subtree-replacement contract + tests, focus-on-detach spec) land in M1 before any showcase code is written. Building the showcase first would have the showcase discover substrate bugs that should have been tested-as-contracts already.

### 2026-05-22: Demos rebuild on nav, not hidden-and-restored

When the user navigates from demo A to demo B and back, demo A rebuilds from scratch — no `display: none` persistence. Reasons: (1) it's the harder substrate test (subtree-replacement contract gets exercised constantly), (2) it matches user intuition for a gallery (demos start in a known state), (3) memory stays bounded regardless of how many demos a user clicks through.

### 2026-05-22: 0.2.0 = showcase + events + `calc()` as one release

Originally `calc()` and the event bundle were 0.2.0; the showcase was a parallel track. Folded the showcase in because (a) it's the largest single consumer of the new events, (b) shipping events + `calc()` without a real consumer to dogfood them risks substrate-design choices that look fine in isolation but miss in practice, (c) the showcase's M-prerequisite work (multi-stylesheet, subtree-replacement) is itself substrate completion 0.2.0 wants to ship anyway.
