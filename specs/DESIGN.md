# DESIGN — rdom architectural overview

rdom is a DOM for terminal applications, in Rust. It brings the architecture of the browser DOM — arena-backed nodes, CSS-style cascade, flexbox layout, capture/bubble events, mutation observers, selection ranges — to text-mode UIs.

This document is the durable architectural reference. For where rdom departs from the web platform, see [`DIVERGENCES.md`](DIVERGENCES.md). For the operational guide humans and AI agents follow when working on the code, see [`../CLAUDE.md`](../CLAUDE.md) (a.k.a. `AGENTS.md`).

## Crate map

Five workspace crates. Each publishes independently to crates.io at a shared version pinned in the root `Cargo.toml`.

```
rdom-core      pure DOM — arena, NodeId, attrs, classes, mutation,
               selectors, 3-phase events, MutationObserver, Selection.
               Generic over an Ext type. Zero rendering deps.

rdom-style     CSS data model + property dispatch + value parsers.
               Leaf crate; consumed by rdom-css and rdom-tui.

rdom-css       CSS string → Stylesheet / TuiStyle. Tokenizer, block
               parser, <style>-tag extraction, inline-style seeding.
               No external parser deps.

rdom-parser    HTML-ish template → Dom<Ext>. parseFromString
               equivalent. No external parser deps.

rdom-tui       Terminal backend. Cascade (specificity, custom
               properties), layout (flexbox, IFC), paint (canvas +
               ANSI), runtime (event loop, hit test, keyboard/mouse,
               focus, selection + clipboard), and native HTML
               element built-ins.
```

Dependency DAG: `rdom-core → {rdom-style, rdom-parser} → rdom-css → rdom-tui`.

## Non-negotiable invariants

The project keeps its shape by holding a small number of architectural lines.

### 1. The browser DOM is the reference model

Naming, semantics, and event/cascade behavior track the web platform unless there is a documented reason to diverge. When in doubt, do what the browser does. Every deliberate divergence lives in [`DIVERGENCES.md`](DIVERGENCES.md).

### 2. `rdom-core` is renderer-agnostic

The substrate has no styles, no layout, no terminal types, no paint. It exposes hooks for a backend through an `Ext` type parameter and through traits the backend implements. Never by importing backend types.

### 3. Styling, layout, paint, and the runtime event loop live in `rdom-tui`

Or in sibling backends, when those exist. Sibling backends (headless renderers, alternate terminal backends, GPU canvas) are *siblings* of `rdom-tui`, not specializations of it. The split is "which substrate primitive does this consume" — not "extend `rdom-tui` to cover another surface."

### 4. The parser is a separate concern

`rdom-parser` produces a tree. It does not style, lay out, or render. The parser must not depend on `rdom-tui`.

### 5. Native HTML elements only, zero opinionated components

The crate set ships native HTML element behaviors (`<button>`, `<input>` family incl. `type="range"`, `<select>`, `<form>`, `<details>`, `<dialog>`, `<progress>`, `<meter>`, `<table>` family, `<canvas>`). Higher-level component libraries (virtualized tables, custom widgets) belong in downstream consumer projects, not in this workspace. Same shape as the browser.

### 6. Public behavior is contract-first

Domain types and tests describe behavior before implementation lands. The public API surface lives in each crate's `lib.rs` re-exports; nothing user-facing leaks through `pub(crate)` accidents.

### 7. Real fixes only

Special-case patches that only satisfy the current fixture, silent fallbacks that hide invalid state, broad `unwrap_or_default` / `let _ =` that swallow real failures, and duplicated logic across crates are not acceptable. Fix the root cause, not the symptom.

## Layout passes

`rdom-tui` runs three formatting contexts off the same `layout_pass::layout_node` entry point. Selection happens in `flex::layout_children`:

1. **Inline Formatting Context (IFC)** — fires when the node has at least one `Display::Inline` child. The `LinePacker` greedily packs every descendant grapheme (plus atomic `Display::InlineBlock` fragments per CSS 2.1 §10.8) into `LineBox`es at the container's content width. Children get zero-sized layout rects; paint reads the parent's `inline_layout`. Single-fragment containers (just one inline element) still go through this path. Lives in `render/inline/`.

2. **Pure-text leaf** — fires when the node has direct text children and no element children (`<p>only text</p>`, `<input>`, `<textarea>`). Same line-packing as IFC, but reached through a structural shortcut so the predicate stays narrow ("at least one inline ELEMENT child").

3. **Flow dispatch** — for elements with element children that aren't an IFC. The cascaded `Flow` (Phase 1 of BFC-1) picks between:
   - `Flow::Block` → `block::layout_block_children`: CSS 2.1 §10 normal flow. Children stack vertically in document order; mixed inline+block content folds inline runs into **anonymous block boxes** (CSS 2.1 §9.2.1.1) that establish their own IFC. Margin collapse per §8.3.1 (adjacent siblings, parent-first/last-child, empty-block collapse-through, full upward propagation). Height: `Auto` resolves from the actual measured content extent; `Percent` resolves only against a *definite* containing block, walking the ancestor chain (§10.5). `row-gap` from CSS3 Box Alignment applies between adjacent block-level element children.
   - `Flow::Flex` → `flex::layout_flex_children`: CSS Flexible Box L1 distribution along `direction`. Main-axis grow / shrink / `flex-basis` per §9; cross-axis stretch; `gap` between items; auto-min content floor per §4.5 (`M5-MIN-CONTENT-1`). Establishes a new BFC.

CSS3 Display Module two-value mapping is the source of truth: `display: block` → outer `Block` + inner `Block`; `display: flex` → outer `Block` + inner `Flex`. The parser writes both fields atomically (`tui_style::display()` setter).

**Atomic inline-block in IFC** (Phase 3.5b): when an inline-block element appears alongside `Display::Inline` siblings, the IFC packer emits a single fragment for the inline-block carrying its intrinsic width (including UA pseudo content like `<button>`'s `[ ]`). After IFC layout, `layout_atomic_inline_blocks` (block.rs) recursively calls `layout_node` on each atom so its own subtree (text wrap, pseudos, descendants) lays out and so hit-test descends into it.

**Border-collapse parent-edge inset**: shared between block and flex via `flex::collapse_parent_edge_insets` — when `border-collapse: collapse` extends the parent's content area to include its border ring, the first/last in-flow child needs an inset if it lacks its own border to share the cell with. Same per-edge logic for both layout modes.

**`establishes_new_bfc`** (Phase 1 cascade field): true for `display: flex`, `display: inline-block`, `overflow != visible`, `position: absolute|fixed`. Used by margin-collapse to gate parent-child collapse + by parent-bottom/last-child trapping (a BFC traps its children's margins inside its content height instead of letting them escape upward).

## Roadmap

0.1.0 ships the DOM substrate, the cascade, flexbox layout, the runtime, native HTML built-ins, the UA stylesheet, the CSS string parser, and the HTML template parser. See the root [`README.md`](../README.md#whats-in-010) for the shipped feature list.

The work that fed into 0.1.0 was organized in five internal milestones (M1 CSS parser, M2 positioning, M3 timers + transitions, M4 DOM API completeness, M5 layout primitives bundle). Going forward, releases are numbered by semver only.

| Version | Scope |
|---|---|
| **0.2.0** | Three workstreams bundled — see [`SHOWCASE.md`](SHOWCASE.md) for the full plan. (1) **`rdom-showcase`** — permanent in-tree TUI app that mounts every rdom primitive in one browsable binary; headline feature, dogfooding fixture, CI regression detector. (2) **Event surface bundle** — `dblclick`, `contextmenu`, `keyup`, `mousemove`, `scroll`, `resize`. (3) **`calc()` value system** — length-and-percentage expressions, resolved at cascade/layout time. M1 of the plan also lands substrate completion the showcase depends on: multi-slot stylesheet API, subtree-replacement contract, focus-on-detach spec. |
| **0.3.0** | Client-side routing primitive. |
| **0.4.0** | Async tasks during event handlers. |

Current 0.2.0 progress lives in [`../STATE.md`](../STATE.md).

Open polish items (no fixed milestone): form validation (`:required` / `:invalid` / `pattern`), `:focus-visible`, `::placeholder` / `:placeholder-shown`, multi-text-node `contenteditable`, undo/redo coalescing, blinking caret, line-based selection extension (`Shift+Up` / `Shift+Down`), whitespace normalization in clipboard serialization.

Deferred polish lives in [`TECH_DEBT.md`](TECH_DEBT.md).

## Decision archive

Architectural decisions worth preserving past their original context.

### `rdom-style` exists as a leaf crate

Originally `rdom-tui` owned the CSS data model. When `rdom-css` (the CSS parser) was added, both `rdom-css` and `rdom-tui` needed to depend on the data model — but `rdom-tui` already depended on `rdom-css`, creating a Cargo cycle. Extracting `rdom-style` as a leaf crate (CSS data model + property dispatch + value parsers, no backend deps) resolved the cycle and gave the parser a stable target.

### CSS cascade lives in `rdom-tui`, not `rdom-style`

`rdom-style` owns the property dispatch table and the data model. `rdom-tui` owns the cascade pipeline (specificity, `!important`, custom-property resolution, computed-style propagation, DirtyTracker). The split is: `rdom-style` is what gets applied; `rdom-tui` is how applying happens. Other backends would implement their own cascade against the same `rdom-style` types.

### `border-collapse: collapse` extends to any flex container

Instead of inventing a new property name (`border-join`), reuse the CSS property because the semantic is the same algorithm. The divergence is *extended scope* (applies to any flex container, not just `<table>`), documented in [`DIVERGENCES.md`](DIVERGENCES.md).

### Layout model under `border-collapse`

When collapse is active and an element has a border, `compute_content_area_collapsed` returns the *outer* rect, so children's outer edges coincide with the parent's border-ring cells. Sibling overlap is handled inside the flex resolver only. This concentrates the box-model special case in one function; hit-test, paint, and selection don't need to know about collapse.

### Paint layer invariant: `fill_bg` owns `cell.bg`; glyph painters write `symbol + fg + modifiers` only

Including `bg` in a glyph style during paint causes a second blend pass under opacity, producing double-blended colors. The bug surfaced as visibly brighter text on translucent cards. Two helpers: `style_from_computed` (with bg, for pseudos that paint their own bg) and `glyph_style_from_computed` (without bg, for own-text and IFC fragments whose owner is the IFC block).

### NodeId is arena-scoped and never reused within a `Dom`

Removing a node releases the ID, but the arena never hands the same ID out twice within a single `Dom` instance. This is what makes `NodeId` safe to pass around as an opaque handle without lifetime gymnastics.

### MutationObserver delivery is batched at microtask boundaries

Records reference live nodes only; observers do not deliver records for nodes dropped during the batch. Matches WHATWG DOM.

### Event dispatch is 3-phase (capture → target → bubble) with full `stopPropagation` / `stopImmediatePropagation` / `preventDefault` semantics

Same as UI Events. The `is_synthetic` flag inverts the spec's `isTrusted` for ergonomic reasons (Rust default-`false` matches the common case).

## Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash scripts/spec-lint.sh
```

CI runs the same four gates on `[ubuntu-latest, macos-latest, windows-latest]` for every push and PR. Toolchain pinned via `rust-toolchain.toml`.
