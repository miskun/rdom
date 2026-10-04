# DESIGN — rdom architectural overview

rdom is a DOM for terminal applications, in Rust. It brings the architecture of the browser DOM — arena-backed nodes, CSS-style cascade, flexbox layout, capture/bubble events, mutation observers, selection ranges — to text-mode UIs.

This document is the durable architectural reference. For where rdom departs from the web platform, see [`DIVERGENCES.md`](DIVERGENCES.md). For the operational guide humans and AI agents follow when working on the code, see [`../CLAUDE.md`](../CLAUDE.md) (a.k.a. `AGENTS.md`).

## Crate map

Five published crates plus the in-tree `rdom-showcase` demo (`publish = false`). Crates publish independently and bump independently: only a crate whose source changed bumps, together with every crate that pins it (a `rdom-core` change therefore bumps all five). The root `Cargo.toml` workspace version is the default for crates that have not diverged, not a shared release number; `rdom-core` and `rdom-tui` have carried their own versions since 0.3.x.

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

**Auto height is finalized after the children are placed.** `layout_node` writes the element's rects, lays out the children inside the pre-measured content area, then resolves `height: auto` from the measured extent (`resolve_auto_height`). A descendant therefore must not read its parent's `content_layout` as a basis during its own layout: a percentage `top` / `bottom` on a relatively positioned box is `auto` unless the parent's height is definite (CSS 2.1 §9.3.2), percentage heights resolve only against definite ancestors (§10.5), sticky pinning and scroll extents read the rect after the pass, and the classic-scrollbar trigger compares against the final height. A measure-then-place pass would remove the ordering but is not needed by anything today.

**Classic scrollbars take two passes.** CSS Overflow 3 §3 lets `scrollbar-gutter: auto` follow the platform's scrollbar kind; terminal cells cannot be overlay-composited, so rdom takes the classic path: a scrollbar consumes a row or column. `overflow: scroll` and `scrollbar-gutter: stable` reserve the gutter up front; `overflow: auto` lays out once without it, and when the content overflows an axis, `layout_node` reserves the gutter, lays the children out again in the smaller area and re-resolves the element's `auto` height (the gutter row is part of the box). A smaller area can only increase overflow, so the second pass converges.

**`establishes_new_bfc`** (Phase 1 cascade field): true for `display: flex`, `display: inline-block`, `overflow != visible`, `position: absolute|fixed`. Used by margin-collapse to gate parent-child collapse + by parent-bottom/last-child trapping (a BFC traps its children's margins inside its content height instead of letting them escape upward).

## Roadmap

0.1.0 ships the DOM substrate, the cascade, flexbox layout, the runtime, native HTML built-ins, the UA stylesheet, the CSS string parser, and the HTML template parser. See the root [`README.md`](../README.md#whats-in-010) for the shipped feature list.

The work that fed into 0.1.0 was organized in five internal milestones (M1 CSS parser, M2 positioning, M3 timers + transitions, M4 DOM API completeness, M5 layout primitives bundle). Going forward, releases are numbered by semver only.

| Version | Scope | Status |
|---|---|---|
| **0.2.0** | Three workstreams bundled (see [`SHOWCASE.md`](SHOWCASE.md)): **`rdom-showcase`** (in-tree TUI app touring every primitive), the **event surface bundle** (`dblclick`, `contextmenu`, `keyup`, `mousemove`, `scroll`, `resize`), and the **`calc()` value system**. Plus BFC, native ARIA tree, multi-slot stylesheets, layered border model. | ✅ Released 2026-06-02 |
| **0.3.0** | **Substrate honesty** — the seven friction points the first downstream consumer hit (geometry setters drive layout, repaint-from-listeners, arena ergonomics). Routing slid out to 0.4.0. | ✅ Released 2026-06-02 |
| **0.3.1 – 0.3.14** | Divergent `rdom-tui`-only patch releases, mostly driven by the `rdom-virtualtable` consumer: focus/`:where()`/`drop_subtree` fixes, table column-sizing, layout stale-state, half-block borders, **drag-autoscroll** + its robustness/selection-precision follow-ups. See [`../CHANGELOG.md`](../CHANGELOG.md) + [`../STATE.md`](../STATE.md). | ✅ Released (latest 0.3.14, 2026-06-06) |
| **0.4.0** | **Hardening** ([`HARDENING-2026-09.md`](HARDENING-2026-09.md)): full workspace review, DIVERGENCES / TECH_DEBT re-audit, four gated fix batches (core, style + css, tui, parser). All five crates ship together. | ✅ Released 2026-09-24 |
| **0.5.0** | **Stabilize** ([`STABILIZE-2026-09.md`](STABILIZE-2026-09.md)): every open `TECH_DEBT.md` row fixed, reclassified as a documented divergence, or documented as external; plus form validation, `:focus-visible`, live `<style>` and the rest of Phase 7. All five crates ship together. | Shipped 2026-09-29 |
| **0.6.0** | **CSS completeness** ([`CSS-COMPLETE-2026-10.md`](CSS-COMPLETE-2026-10.md)): every CSS feature that means something in a terminal, from the [`CSS-COVERAGE.md`](CSS-COVERAGE.md) audit; plus the [`ACID.md`](ACID.md) acid test. | In progress |
| **0.7.0** | Client-side routing primitive. | Planned |
| **0.8.0** | Async tasks during event handlers. | Planned |

Current progress + the full release ledger live in [`../STATE.md`](../STATE.md).

Web-platform surface not yet shipped is listed in [`DIVERGENCES.md`](DIVERGENCES.md) §3; open debt, if any, in [`TECH_DEBT.md`](TECH_DEBT.md).

## Decision archive

Architectural decisions worth preserving past their original context.

### `rdom-style` exists as a leaf crate

Originally `rdom-tui` owned the CSS data model. When `rdom-css` (the CSS parser) was added, both `rdom-css` and `rdom-tui` needed to depend on the data model — but `rdom-tui` already depended on `rdom-css`, creating a Cargo cycle. Extracting `rdom-style` as a leaf crate (CSS data model + property dispatch + value parsers, no backend deps) resolved the cycle and gave the parser a stable target.

### `from_css` is a free function in `rdom-css`, not `impl Stylesheet`

(Formerly TECH_DEBT `D-M1-1`, accepted permanently; moved here in 0.5.0.) The first draft made CSS parsing inherent methods on `Stylesheet`. `Stylesheet` and `TuiStyle` live in `rdom-style`, and `rdom-css` depends on `rdom-style`, so an inherent method would need `rdom-style` to depend on its own parser — a Cargo cycle — or the parser folded into the data-model crate, which the parser/data-model split exists to prevent. The parser therefore exposes free functions (`rdom_css::from_css`, `parse`, `parse_inline`, and their `_strict` forms).

### CSS cascade lives in `rdom-tui`, not `rdom-style`

`rdom-style` owns the property dispatch table and the data model. `rdom-tui` owns the cascade pipeline (specificity, `!important`, custom-property resolution, computed-style propagation, DirtyTracker). The split is: `rdom-style` is what gets applied; `rdom-tui` is how applying happens. Other backends would implement their own cascade against the same `rdom-style` types.

### `border-collapse: collapse` extends to any flex container

Instead of inventing a new property name (`border-join`), reuse the CSS property because the semantic is the same algorithm. The divergence is *extended scope* (applies to any flex container, not just `<table>`), documented in [`DIVERGENCES.md`](DIVERGENCES.md).

### Layout model under `border-collapse`

When collapse is active and an element has a border, `compute_content_area_collapsed` returns the *outer* rect, so children's outer edges coincide with the parent's border-ring cells. Sibling overlap is handled inside the flex resolver only. This concentrates the box-model special case in one function; hit-test, paint, and selection don't need to know about collapse.

### `opacity` is group opacity (and color alpha shares it)

An element with `opacity < 1` paints its whole stacking context into a layer at full opacity — a copy of the frame buffer's row band that the subtree can paint (`render/paint_pass/group.rs`, bounded below); `Buffer::composite_group` then folds that layer back onto the backdrop at the element's alpha, so nested opacities multiply and every paint inside the group blends exactly once, with no per-write compose state on the buffer. A cell holds one glyph, one foreground and one background, so the fold cannot mix two glyphs the way a pixel renderer mixes coverage. The rules, per cell:

- **Background.** A background the layer changed blends over the backdrop's: `α·layer + (1-α)·backdrop`.
- **Glyph contest.** A glyph the layer painted — text, or a border it contributed — takes the cell when the backdrop cell shows no glyph, or when `α ≥ 0.5` (the layer covers at least half of what shows). Its colour blends against the backdrop background, and it carries the layer's modifiers and link state (including no link). Otherwise the backdrop glyph stays. Over an empty backdrop a glyph therefore fades in smoothly from `α = 0`; between two visible glyphs the one with more coverage wins.
- **Same glyph.** When the layer repaints the backdrop's own symbol with other state, there is no contest: the layer's modifiers and link state win at any `α`, and only the foreground blends, layer against backdrop colour (`render/buffer/composite.rs`, the `LayerGlyph::Same` arm). This exists for selection and highlight overlays, which restyle a cell without changing its symbol — under a translucent ancestor the highlight must still show.
- **Tint.** A backdrop glyph that stays under a background the layer changed is tinted toward it: `α·layer_bg + (1-α)·fg` — a 0.9-opacity white card leaves the text beneath at 10% contrast, not full contrast.
- **Borders.** Border glyphs are materialised after all painting by the joiner, from per-direction contributions that carry their colour. Compositing therefore blends the contributions, not glyphs: those the layer added follow the glyph contest and blend like a glyph; backdrop contributions under a changed background tint like a backdrop glyph. The joiner stays the single source of border glyphs and colours, and a translucent box's border joins opaque neighbours' borders from the same tables. (Joining the layer before compositing would have materialised the translucent border twice and lost those junctions.)
- **Wide glyphs** composite as a unit: a layer's wide glyph takes its spacer cell with it, and a primary that loses its spacer, or a spacer that loses its primary, becomes a space.
- **`opacity: 0`** composites nothing.

`Color::Reset` has to become a colour before it can blend. The canvas model takes the document's color scheme (`ColorScheme::canvas`, set on the frame buffer by the paint pass): under dark — the default, and what a terminal that does not answer the startup OSC 11 query gets — `Reset` background is `#000000` and foreground `#FFFFFF`; under light, the reverse. A translucent element's default-coloured text is thus written as an explicit grey.

**Color alpha shares these rules.** A paint in a translucent color (`rgb(255 0 0 / 50%)`, CSS Color 4 §4.2) is made at full opacity into a layer covering just the cells it paints and composited back at the color's alpha (`render/buffer/translucent.rs`): a background fill blends with the background beneath and tints the glyphs and borders it leaves; a glyph contests the backdrop's and blends with the background; a border's contributions blend like a glyph. A write whose background and foreground are both translucent composites twice, background first. The paint pass routes every author color through this — box fills, borders, text, `::backdrop`, row highlights, `::selection` and caret styles through `Buffer::set_style` — so no cell ever holds an alpha; `Cell::set_fg` / `set_bg` called directly blend straight-alpha against the cell instead.

The layer is bounded: it copies and composites only the rows the element's subtree can paint (every box, anonymous box, pseudo box and inline line in it, plus one row of margin), across the full frame width, so a translucent element costs O(W · its height) per frame rather than O(W · H). Full width and the margin keep the paint pass's buffer-edge rules (the border off-buffer filter, the wide-glyph right-edge ellipsis) exactly as they are on the frame. In the crate's own tests every group is also painted through a full-frame layer and the two results are asserted equal, so the bound is checked by every paint test that uses `opacity`.

### Paint layer invariant: `fill_bg` owns `cell.bg`; glyph painters write `symbol + fg + modifiers` only

Each cell's background has one owner. Before group opacity, a `bg` in a glyph style blended a second time under `opacity` and surfaced as visibly brighter text on translucent cards; group compositing removed that failure mode, and the split keeps the ownership explicit. Two helpers: `style_from_computed` (with bg, for pseudos that paint their own bg) and `glyph_style_from_computed` (without bg, for own-text and IFC fragments whose owner is the IFC block).

### NodeId is arena-scoped and never reused within a `Dom`

Removing a node releases its slot for reuse, but the `NodeId` carries a per-slot generation that changes on every recycle, so a handle to a dropped node never resolves to the slot's next occupant. This is what makes `NodeId` safe to pass around as an opaque handle without lifetime gymnastics.

### `unset` is resolved when a declaration is parsed

CSS Cascade 4 defines `unset` as `inherit` for inherited properties and `initial` otherwise. Which properties inherit is a fact about the property, not the tree, so `rdom-style`'s dispatch table (`property_dispatch::inherits`) resolves the keyword into `Value::Inherit` / `Value::Initial` at parse time and the cascade never sees `unset`. `revert` (Cascade 4 §7.3) is the opposite case: its value depends on the declaration's origin, so it is stored as `Value::Revert` and the cascade resolves it from the ladder's rollback state (`rdom-tui` `cascade/ladder.rs`: the rolled-back states are replayed on demand, memoized per step, so a cascade without `revert` pays nothing). The cascade copies that same set in `inherit_inheritable_from`; a cascade test probes every property against the table, so there is one declaration and one proof, no second list.

### Cascade layers: declared per sheet, ordered per cascade

`@layer` (CSS Cascade 5 §6.4) is parsed by `rdom-css` into the sheet it appears in: the sheet records the layers it declares in order of first declaration (`Stylesheet::layers`, one name segment under a parent each, anonymous ones unnamed) and each rule its `LayerId`. The order that decides the cascade is not a property of one sheet: CSSOM orders all of a document's sheets together, so `rdom_style::LayerOrder::new(&sheets)` merges the layers of every sheet of a cascade run by name, in run order (an `App`: `<style>` sheets in tree order, then its own), and ranks them — siblings by first declaration, sublayers below their parent's own rules, unlayered rules last. The cascade computes it once per run; the ladder (`rdom-tui` `cascade/ladder.rs`) gives each layer present among an element's matched author rules its own step, ascending for normal declarations and descending for `!important`, so `revert-layer` rolls back to the step's own start exactly like `revert` rolls back to the UA step. `Stylesheet::append` merges one sheet into another with the same name rules, which is how `from_css` and `extend_from_style_tags` keep layers.

### `:root` custom properties are published twice on purpose

A `:root { --x: v }` rule is an ordinary rule whose custom properties the cascade scopes per element like any other (CSS Variables 1). The parser *also* mirrors those values into `Stylesheet::vars()`, and the cascade seeds the document root's map from every sheet's `vars()`. The sheet-level map is the programmatic API (`define_var`, consumers that read variables without a tree); the mirror keeps it truthful for parsed sheets. `:root` matches the tree's root node, which carries no computed style of its own, so the seed is how its custom properties reach the elements — and the mirror must therefore be the cascade's answer for that node: `rdom-css` computes it after the parse over the sheet's unscoped rules whose selector is exactly `:root` (a selector-list item included), important before normal, then layer order (unlayered above every layer for normal declarations, reversed for `!important`), then order of appearance with `@import`ed rules at the import's position (`rdom-css/src/root_vars.rs`, C1G-ROOT-SEED). Across sheets the later sheet's map wins per name, as their unlayered normal rules would; a layered or important `:root` declaration in an earlier sheet does not outrank a later sheet's (accepted: the cross-sheet layer order is a cascade-run property the per-sheet map cannot see).

### Which public types are `#[non_exhaustive]`

One rule, applied to every public enum and public-field struct in the five published crates: **a type is `#[non_exhaustive]` when a consumer that meets a variant or field it does not know has a correct thing to do — ignore it, or fall back generically — so adding one must not be semver-major.** That covers three kinds:

- **Open web vocabularies** that rdom implements in part or that the platform keeps extending, where unknown members can be ignored: selector components (`Combinator`, `SimpleSelector`, `PseudoClass`), `PseudoElementTarget`, `RuleOrigin`, node kinds (`NodeType`, `NodeData`), attribute states (`InputTypeState`, `ContentEditableState`, `FormMethod`, `FormEnctype`), `EventDetail`, mutation / interaction kinds (`Mutation`, `InteractionKind`), `AnimatableProperty` (an unknown one changes discretely, CSS Transitions 1 §2), the math-expression vocabularies `MathFunction`, `CalcUnit` and `CalcKind` (each names itself — `name()`, `css_name()` — and an expression resolves without a consumer matching on them), `CounterStyle` (an unknown style falls back to `decimal`), `ScrollBehavior` (CSSOM View §12.1: a UA may ignore it), `Token`, the CSSOM View option enums, rdom-tui's slot / animation / scrollbar-part mirrors of these, and the event payload dictionaries (`Event`, `TransitionDetail`, `InputDetail`, `ToggleDetail`, `MouseDetail`, `KeyboardDetail`, `SubmitDetail`).
- **Error, warning and outcome sets**, enums and records alike: `DomError`, `InvariantViolation`, `ParseError`s, `TokenizerError(Kind)`, `DispatchError`, `StyleError`, `ParseErrorKind`, `WarningKind`, `Warning`, `ParseResult`, `SetPropertyError`, `SubmitOutcome`, `EditOutcome`, `UndoOutcome`, `RouteOutcome`, `CompletedFrame`, `ScrollbarHit`, `ValidityState`.
- **Options bags**: `ListenerOptions`, `ScrollToOptions`, `ScrollIntoViewOptions`, `ResolveCtx`, and rdom-style's `Rule` record.

Not `#[non_exhaustive]`: closed data, where every consumer must handle every member for correctness and a new one is deliberately breaking —

- **CSS value and style-record types** the cascade, layout and paint resolve: `Value<T>` (the CSS-wide keywords), the keyword enums (`Display`, `Position`, `Direction`, `Overflow`, `WhiteSpace`, …), `Size` / `MinSize` / `MaxSize` / `FlexBasis` / `FlexShorthand` / `Length` / `GapValue` / `MarginValue` / `PaddingValue`, `CalcExpr`, `RoundingStrategy`, `ViewportUnit` / `ViewportSize` / `ViewportAxis`, `Content`, `TimingFunction`, `TransitionRule`, `TuiStyle`, `ComputedStyle`. A wildcard arm in the renderer would be a silent fallback (a new `display` value laid out as something else); the compiler naming every match is the point.
- **Sets fixed by their definition**: `EventPhase`, `AdjacentPosition`, `AttrOp`, `ToggleState`, `ActivationPhase`, `NodeOrString`, `StepPosition`, `BorderCollapse`, `ZIndex`, `CalcOp`, axes and sides (`ScrollAxis`, `BorderSide`), `ControlFlow`, geometry (`Rect`, `LayoutRect`, `Viewport`, `Position` / `Range` / `Selection`, `Border` / `Padding` / `Margin`, `Specificity`), and enums whose catch-all variant already covers the whole grammar (`MouseButton::Other`, `InputType::Other`, `TransitionProperty::Discrete`), so a new named variant would be a behavior change anyway.
- **Sealed by private fields** (outside the rule, which covers public-field types): `TuiExt`, and `AspectRatio`, whose terms only `AspectRatio::new` sets, so a non-finite or negative one never reaches layout.
- **Handles and runtime internals exposed for inspection**: `ListenerId`, `EventCtx`, `AppContext`, `TimerCtx`, `TuiEvent`, `KeyboardModifiers`, render buffers / cells / line boxes / fragments, `EditEntry` / `Edit` (records a consumer builds), and `TuiExt` (already sealed by private fields).

A `#[non_exhaustive]` struct a consumer must build gets a constructor or builder (`MouseDetail::new(..).with_delta(..)`, `ListenerOptions::capture().with_once(true)`, `ScrollToOptions::new().top(..)`). Inside the workspace, an exhaustive match over a vocabulary lives in the crate that defines it where it can (Selectors 4 specificity is `ComplexSelector::specificity` in rdom-core, not a match in rdom-style); a sibling crate that must map every member (rdom-css turning `DispatchError` into warnings, the DirtyTracker reading `Mutation`) keeps a wildcard arm that `debug_assert!`s, so the workspace's tests catch an unmapped addition.

### MutationObserver delivery is synchronous, one record per mutation

Each mutation notifies every registered observer before the mutating call returns (no microtask batching — the runtime has no task queue to batch against). Records reference live nodes at delivery time; the `ChildListChanged` for a drop fires before the slot is freed. Observers must not mutate the tree during a callback (panics), but may add or remove observers.

### Event dispatch is 3-phase (capture → target → bubble) with full `stopPropagation` / `stopImmediatePropagation` / `preventDefault` semantics

Same as UI Events. The `is_synthetic` flag inverts the spec's `isTrusted` for ergonomic reasons (Rust default-`false` matches the common case).

## Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same three gates (fmt, clippy, test) on `[ubuntu-latest, macos-latest, windows-latest]` for every push and PR. Toolchain pinned via `rust-toolchain.toml`.
