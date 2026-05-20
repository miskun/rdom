# rdom Agent Guide

This file defines how AI agents should work in this repository.

`rdom` is a DOM for terminal applications, in Rust. It brings the architecture of the browser DOM — arena-backed nodes, CSS-style cascade, flexbox layout, capture/bubble events, mutation observers, selection ranges — to text-mode UIs. The project needs to stay boring, correct, testable, and explicit.

Keep this file current. If the project makes a durable process, architecture, or quality decision, update `CLAUDE.md` in the same change.

`AGENTS.md` at the repo root is a symlink to this file — Claude Code, Codex, Cursor, Aider, and any other agent honoring the [agents.md](https://agents.md) convention all read the same content. Edit `CLAUDE.md`; `AGENTS.md` follows automatically.

## Where to look first

- [`specs/DESIGN.md`](specs/DESIGN.md) — architectural overview: crate map, non-negotiable invariants, roadmap, decision archive.
- [`specs/DIVERGENCES.md`](specs/DIVERGENCES.md) — every deliberate departure from the web platform.
- [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md) — open debt + accepted simplifications.

Detailed behavior lives in the code: each module has a top-level doc comment, and tests document the contracts.

## Non-Negotiable Project Principles

- The browser DOM is the reference model. Naming, semantics, and event/cascade behavior should track the web platform unless there is a documented reason to diverge. Every deliberate divergence lives in `specs/DIVERGENCES.md`.
- `rdom-core` is renderer-agnostic. It must not know about styles, layout, terminals, or paint.
- Styling, layout, paint, and the runtime event loop live in `rdom-tui` (or sibling backends, when those exist).
- The parser is a separate concern from the DOM and from the renderer. Parsing must not require a runtime or a backend.
- The crate set ships native HTML elements and zero opinionated components. Browser-faithful built-ins live in `rdom-tui::runtime::builtins`; higher-level component libraries are downstream projects, not in-tree.
- Public behavior is contract-first: domain types and tests describe behavior before implementation lands.

## Substrate First, Backend Second

`rdom-core` is the **shared substrate**: arena, `NodeId`, attributes, classes, tree mutation, CSS selectors, 3-phase event dispatch, `MutationObserver`, `AbortSignal`, and the `Selection` / `Range` / `Position` data model. It is generic over an extension type `Ext` and has zero rendering dependencies.

Every other crate is a **consumer** of that substrate:

- **`rdom-style`** — CSS data model + property dispatch + value parsers. Leaf crate; consumed by `rdom-css` (the parser) and `rdom-tui` (the renderer).
- **`rdom-css`** — CSS parser. Tokenizer + block parser + `<style>`-tag extraction + inline-style seeding. Produces `Stylesheet` / `TuiStyle` via `rdom-style`'s property dispatch.
- **`rdom-tui`** — the terminal backend. Owns CSS cascade (specificity, custom properties), layout (flexbox, inline formatting), paint (canvas + ANSI emission), and the runtime (event loop, hit test, keyboard/mouse routing, focus, text selection + clipboard), plus the native HTML element behaviors (`<button>`, `<input>` family incl. `type="range"`, `<select>`, `<form>`, `<details>`, `<dialog>`, `<progress>`, `<meter>`, `<table>` family, `<canvas>`). Defines `TuiExt` and operates on `Dom<TuiExt>`.
- **`rdom-parser`** — HTML-ish template strings → `Dom<Ext>`. Hand-rolled, no external parser deps. Equivalent in role to `parseFromString`.

Durable rules:

- Never add styling, layout, color, or terminal types to `rdom-core`. If the substrate needs to expose a hook for a backend, it does so through `Ext` or through a trait the backend implements — never by importing backend types.
- Never ship an opinionated component library in-tree. Native HTML elements live in `rdom-tui::runtime::builtins`; higher-level component libraries belong in downstream consumer projects, not in any crate this workspace publishes.
- Never let the parser depend on `rdom-tui`. The parser produces a tree; it does not style, lay out, or render it.
- Browser-DOM semantics are the default. New behavior should reference the relevant web spec (DOM, CSSOM, UI Events, CSS Flexible Box, Selection API, etc.) and call out deliberate divergence by adding an entry to `specs/DIVERGENCES.md`.
- Sibling backends (headless renderers, alternate terminal backends, GPU canvas, etc.) are siblings of `rdom-tui`, not specializations of it. The split is "which substrate primitive does this consume" — not "extend `rdom-tui` to cover another surface."

## Engineering Principles

### TDD Always

Write tests before implementation.

Expected loop:

1. Add or update a failing test that describes the desired behavior.
2. Run the smallest relevant test command and confirm the failure.
3. Implement the smallest change that makes the test pass.
4. Run the relevant tests again.
5. Refactor only after tests are green.

Do not skip the failing-test step for production behavior. Documentation-only changes are the main exception.

### Real Fixes Only

Do not paper over bugs with quick fixes.

When a bug appears:

1. Reproduce it with a failing test (unit test in the crate that owns the behavior, integration test if it crosses crates).
2. Identify the root cause.
3. Fix the root cause, not just the symptom.
4. Keep or add regression coverage.
5. Update `DIVERGENCES.md` or `TECH_DEBT.md` if the bug revealed unclear behavior.

Avoid:

- special-case patches that only satisfy the current fixture
- silent fallbacks that hide invalid tree state, broken selectors, or layout NaNs
- broad `unwrap_or_default` / `let _ =` that swallow real failures
- duplicating logic across crates instead of fixing the shared source
- weakening tests to make a failure disappear

### Contract First

Public behavior is represented as contracts before implementation:

- Rust domain types and unit tests for `rdom-core` (arena, selectors, events, mutation observers, `Selection`).
- Rust domain types and unit tests for `rdom-tui` (cascade, layout, paint, runtime).
- Public API surface lives in each crate's `lib.rs` re-exports; nothing user-facing leaks through `pub(crate)` accidents.
- New deliberate divergence from the web platform: add an entry to `specs/DIVERGENCES.md` in the same commit.

Avoid letting the parser, the cascade, the runtime, and the built-ins each invent their own version of the same concept (e.g. node identity, attribute lookup, focus rules).

### `rdom-core` Owns Truth

Pure DOM behavior — arena lifecycle, tree mutation, selector matching, event dispatch, mutation observers, selection model — belongs in `crates/rdom-core`.

`rdom-core` must avoid:

- terminal or rendering concerns
- color, style, or layout types
- crossterm, arboard, or any I/O dependency
- runtime state (event loops, schedulers)

The core should be testable with ordinary `cargo test -p rdom-core`, no backend required. If a feature seems to need a backend to test, the feature is probably layered wrong.

### Architecture Hygiene

Keep modules small, composable, and easy to reason about.

Watch for:

- god objects (a `Document`, `Runtime`, or `LayoutEngine` that absorbs unrelated responsibilities)
- files that mix DOM mutation, styling, layout, paint, and event routing
- modules that know too much about unrelated concepts (e.g. layout reaching into ANSI emission)
- duplicated cascade or layout logic
- hidden global state (statics, thread-locals) that make tests order-dependent
- APIs that are hard to test without spinning up a full terminal
- files growing past a few hundred lines because multiple responsibilities are accumulating

If a god object or oversized module is emerging, split it earlier rather than later. Prefer small domain types, explicit interfaces, and narrow modules over clever central objects.

At regular intervals, stop and inspect the codebase organization before adding more surface area.

### Browser-DOM Fidelity

When in doubt, do what the browser does.

- Tree mutation, attribute reflection, and class list semantics follow DOM Living Standard.
- Event dispatch is capture → target → bubble, with `stopPropagation` / `stopImmediatePropagation` / `preventDefault` semantics matching UI Events.
- Selectors follow Selectors Level 4 (within the supported subset). Specificity follows CSS specificity rules.
- Layout follows CSS Flexible Box Layout for flex containers, with documented terminal-specific divergences.
- `MutationObserver`, `AbortSignal`, `Selection`, `Range`, and `Position` follow their web counterparts.

Document any deliberate divergence in `specs/DIVERGENCES.md` and call it out at the API boundary.

### Safety By Default

High-impact operations must be designed with safety controls from the beginning.

Examples:

- arena reuse / id recycling (must not hand out a stale `NodeId` as a live one)
- mutation observer batching (must not deliver records that reference dropped nodes)
- runtime tear-down (must restore terminal state, even on panic)
- clipboard / OS integration (must degrade cleanly when the platform refuses)
- panics in user-supplied event handlers (must not corrupt the arena or leave the terminal in raw mode)

Prefer:

- typed handles over raw indices where lifetime confusion is possible
- explicit `Drop` and `defer`-style guards for terminal state
- deterministic ordering for observer and event delivery
- tests that exercise panic and early-return paths, not just the happy path

### Keep `rdom-tui` Honest

The terminal backend should stay focused.

It should:

- own cascade, layout, paint, and the runtime event loop
- expose a small, documented API surface (mount, render, dispatch, query, focus, selection)
- restore terminal state on shutdown and on panic
- surface failures (paint errors, hit-test ambiguity, missing cascade inputs) instead of swallowing them

It should not:

- become a kitchen-sink "framework"
- hide its own bugs behind retries or fallbacks
- ship opinionated composed components — those belong in downstream consumer projects, not in this substrate
- depend on `rdom-parser` (the runtime takes a `Dom<TuiExt>`; how it was built is not its concern)

## Testing Commands

Run the smallest relevant command first, then broaden before finishing.

Per-crate:

```bash
cargo test -p rdom-core
cargo test -p rdom-tui
cargo test -p rdom-parser
cargo test -p rdom-style
cargo test -p rdom-css
```

Workspace gate (same set CI runs, and the same set `/commit` enforces):

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Examples (smoke, when touching `rdom-tui`):

```bash
cargo run -p rdom-tui --example counter_button
cargo run -p rdom-tui --example scrollable_list
cargo run -p rdom-tui --example selectable_text
cargo run -p rdom-tui --example tab_form
cargo run -p rdom-tui --example parse_and_render
cargo run -p rdom-tui --example ua_chrome
cargo run -p rdom-tui --example app_shell
```

CI (`.github/workflows/ci.yml`) runs all three gates on `[ubuntu-latest, macos-latest, windows-latest]` for every push and PR against `main`. The toolchain is pinned via `rust-toolchain.toml` so local dev and CI use the same `rustfmt` / `clippy` versions.

If a command cannot run because dependencies or the environment are wrong, say that clearly in the final response — do not commit.

## Commit Discipline

Commit after each completed implementation or documentation step once the relevant tests or checks are passing.

Guidelines:

- Keep commits scoped to the completed step.
- Do not leave finished green work uncommitted for the user to notice.
- Do not commit if relevant tests are failing.
- If tests cannot run for an environmental reason, report that clearly and ask before committing.
- Do not mix unrelated dirty worktree changes into the commit.

### Pre-commit hygiene gate (mandatory)

Before every commit destined for push, the three-command gate (`cargo fmt` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test --workspace`) must pass clean. Doc-only commits skip the test pass.

**Rule:** if any gate command fails, fix and re-run before committing. Do not ship `fix: drop unused …` follow-up commits — those are evidence the gate was skipped.

The operational checklist lives in `.claude/skills/commit.md` (also invokable as `/commit`).

### Clean-entry-point rule after push

After every `git push`, the local repo must be in a state where `/clear` is safe — working tree clean, branch synced, no half-built artifacts.

If a clean entry point isn't reachable in this turn, say so explicitly and let the user decide whether to push wip or hold.

The operational checklist lives in `.claude/skills/push.md` (also invokable as `/push`).

### Releasing to crates.io (mandatory)

Every `cargo publish` is irreversible — a crate/version pair on crates.io can never be reused, only superseded by a higher version. Before any release, walk the full pre-publish checklist: bump-decision audit (which crates changed, which consumers must bump too), per-crate metadata audit (`description`, `readme`, `repository`, `keywords`, `categories`, path deps with version pins), README sanity (shipped features only — no marketing of unshipped work), gate sweep, leaf dry-run, then commit + push the prep before any actual publish.

**Rule:** never run `cargo publish` with a dirty worktree (`--allow-dirty` is dry-run only), and never re-use a version number — bump anything whose source changed, including transitive consumers when their dep version bumps.

The operational checklist lives in `.claude/skills/publish.md` (also invokable as `/publish`).

## Code and Docs Move Together

A behavior change that affects [`DIVERGENCES.md`](specs/DIVERGENCES.md) or [`DESIGN.md`](specs/DESIGN.md) updates them in the same commit. A change that opens (or pays down) tech debt updates [`TECH_DEBT.md`](specs/TECH_DEBT.md) in the same commit.

The default rule for `DIVERGENCES.md`: anything not listed there matches the web platform. So any deliberate departure must land an entry — otherwise downstream consumers (and downstream agents) will mistakenly assume rdom matches the web spec where it doesn't.

## Milestone Review Gates

At the end of every implementation milestone (any unit of work the project treats as a milestone), run two review passes before moving to the next milestone.

### Grumpy Chief Architect Pass

Review for:

- correctness and root-cause quality
- performance risks (allocation in the hot paint path, redundant cascade work, layout thrash)
- coupling and modularity (`rdom-core` staying renderer-free, `rdom-tui` not absorbing opinionated component logic)
- god objects or oversized files
- unclear boundaries between cascade / layout / paint / runtime
- duplicated logic across crates
- weak contracts (public APIs that paper over invariants)
- missing tests, especially for panic and error paths
- hidden operational risk (terminal state restoration, arena reuse, observer batching)

Output: what is strong, what should be improved, blocking findings, non-blocking findings, required follow-up work.

### Grumpy Chief API Pass

Review for:

- whether the milestone advances rdom toward a usable, browser-faithful DOM for terminals
- whether the public API is something a real consumer can build on without surprise
- whether divergence from the web platform is deliberate and documented in `DIVERGENCES.md`
- whether examples still work and still demonstrate the behavior they claim to
- whether the work creates real consumer value or just internal machinery

Output: what is strong, what should be improved, blocking findings, non-blocking findings, required follow-up work.

### Gate Rule

Do not start the next milestone until key findings are addressed or explicitly tracked in `TECH_DEBT.md` as accepted risks.

## Repository Boundaries

- `crates/` — the five workspace crates. Roles and durable rules in §Substrate First, Backend Second.
- `specs/` — three short docs: `DESIGN.md`, `DIVERGENCES.md`, `TECH_DEBT.md`.
- `.claude/skills/` — operational checklists (`/commit`, `/push`, `/publish`).
- `target/` — build output. Never commit.

## Agent Workflow

Before editing:

- Read the relevant code and tests. Check `DIVERGENCES.md` for any documented departures relevant to the change.
- Identify the smallest behavior change.
- Add or update tests first unless the change is docs-only.

While editing:

- Keep changes scoped.
- Prefer existing patterns in the same crate.
- Keep `rdom-core` renderer-free.
- Keep public contracts (re-exports, trait signatures, event/observer semantics) synchronized.
- When diverging from browser-DOM behavior, document the divergence at the API and add an entry to `specs/DIVERGENCES.md`.

Before final response:

- Run relevant tests (`cargo test -p <crate>` first, then workspace if the change crosses crates).
- Run `cargo clippy` and `cargo fmt --check` for any non-trivial change.
- Report what changed.
- Report verification results.
- Call out anything not run.
