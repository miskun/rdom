# Changelog

All notable changes to rdom will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.13] - 2026-06-06

Only `rdom-tui` bumps (0.3.12 → **0.3.13**); the other four crates are unchanged.

### Fixed — drag-autoscroll mid-tick re-render now cascades

The autoscroll tick's mid-tick relayout was `layout_dom` only. When a consumer mutates the DOM inside its `scroll` handler — a virtualized table re-windows its rows and re-runs column-sizing — those mutations need a **cascade** to take effect, but the bare relayout skipped it. The re-materialized nodes laid out unstyled: collapsed column widths (so the synthetic move's coordinate→cell mapping resolved the wrong column → the selection flickered) and wrong spacer heights (so `scroll_content_height` was under-counted and clamped `scroll_top` below the window start → a cropped window). The mid-tick re-render now runs the full frame path (cascade dirty subtrees + advance animations + layout), matching a normal frame, so a consumer's scroll-handler mutations are fully realized before the synthetic move reads layout. Native text selection doesn't mutate in its scroll handler, so its cascade is a no-op (unaffected). Surfaced by `rdom-virtualtable`'s cell-range drag-autoscroll.

## [0.3.12] - 2026-06-05

Only `rdom-tui` bumps (0.3.11 → **0.3.12**); the other four crates are unchanged (`rdom-core` stays 0.3.5, `rdom-style` / `rdom-css` / `rdom-parser` stay 0.3.4).

### Fixed / Changed — drag-autoscroll robustness + text-selection precision

Surfaced by interactive testing of the `selectable_text` showcase demo (the substrate isolation vehicle for `DRAG-AUTOSCROLL`).

- **Sticky drag-scroll container.** A drag now **owns one scroll container for its lifetime** — resolved once the first time the pointer reaches an edge zone, then never re-targeted or disarmed. Fixes autoscroll getting stuck once the captured/anchor block scrolled out of view, and keeps scrolling when the pointer overshoots *past* the container onto a sibling (textarea-forgiving). Edge detection widened from a single row to a **banded zone with a speed ramp** (`AUTOSCROLL_EDGE_ZONE` / `AUTOSCROLL_MAX_STEP`). The autoscroll engine stays selection-agnostic.
- **Empty-space hits snap to the nearest text position.** `position_at` (and `Document.caretPositionFromPoint`) now resolve a point in a gap / above / below all content to the closest text position instead of returning nothing — so drag-select past the bottom edge no longer collapses back to the anchor block.
- **`user-select: none` excluded from a spanning selection's highlight.** A selection that spans *across* a `user-select: none` element (e.g. a chrome bar between two paragraphs) no longer paints the chrome as selected — matching the copy serializer (which already skipped it) and browsers.
- **Drag over `user-select: none` snaps to the nearest selectable position**, not back to the anchor flow — so dragging a selection over a chrome bar extends past it instead of collapsing. A click still cannot *start* a selection on `user-select: none` content.

Deliberate TUI divergences documented in `specs/DIVERGENCES.md`: sticky container (no cursor re-targeting mid-drag), no viewport scroll-chaining, banded cell-grained edge zone.

## [0.3.11] - 2026-06-05

`rdom-core` bumps 0.3.4 → **0.3.5** (additive API) and `rdom-tui` bumps 0.3.10 → **0.3.11**; `rdom-style` / `rdom-css` / `rdom-parser` are unchanged and stay at 0.3.4 (their `rdom-core = "0.3.4"` pins caret-resolve 0.3.5).

### Added — drag autoscroll (`DRAG-AUTOSCROLL-1`)

A drag that reaches a scroll container's edge now **autoscrolls** it so the selection keeps growing past the viewport — like a browser. Built as one substrate primitive used by both text selection and custom drags (see `specs/DRAG-AUTOSCROLL.md`).

- **rdom-core 0.3.5:** `Dom::set_drag_autoscroll(bool)` / `Dom::drag_autoscroll()` — a generic opt-in flag on the existing pointer capture (`set_pointer_capture` etc.), auto-cleared on release. Renderer-agnostic; the backend interprets it.
- **rdom-tui 0.3.11:**
  - **Drag autoscroll.** While a captured drag that opted in dwells at a scroll container's vertical edge, the runtime scrolls the nearest container one step per ~50ms tick and re-dispatches the drag at the held pointer, so the consumer re-evaluates at the revealed content. Vertical only (horizontal pairs with a separate flex cross-axis-scroll gap). Native **text selection** opts in automatically; a custom drag opts in with `dom.set_drag_autoscroll(true)` from a `prevent_default`ed mousedown.
  - **`App::advance(ms)`** — advance the virtual scheduler clock + service due timers/rAF/microtasks headless and deterministically (for testing timer-driven behavior).

## [0.3.10] - 2026-06-05

Only `rdom-tui` bumps (0.3.9 → 0.3.10); the other four crates are unchanged and stay at 0.3.4.

### Fixed

- **`drop_subtree` no longer panics when dropping two siblings in a row** (`CASCADE-FREED-ROOT-1`). Dropping a node fires a `ChildListChanged` mutation, and the incremental cascade's dirty tracker marks every *remaining* sibling dirty (so sibling selectors re-evaluate) — queuing them as cascade roots. If one of those freshly-marked siblings was then itself dropped in the same teardown (e.g. removing a panel and then its tab), it was freed while still queued, and the next redraw dereferenced the reclaimed arena slot and panicked. The incremental cascade now skips any queued root the arena no longer holds. Consumers can drop absolutely-positioned children freely.

## [0.3.9] - 2026-06-05

Only `rdom-tui` bumps (0.3.8 → 0.3.9); the other four crates are unchanged and stay at 0.3.4.

### Added / Fixed

- **Half-block borders now weld across elements** (`HALFBLOCK-JOIN-1`). Previously `border-style: half-block` only rendered the lone-element ring (4 edges + 4 corners); any T-junction, cross, or two half-block borders meeting at a cell fell back to box-drawing glyphs (`┴`/`┤`). Half-block borders now use an **inward-quadrant model**: each cell fills the quadrants pointing toward its element's content (edge → a half, corner → one quadrant), and the joiner **unions** those quadrants across elements, emitting the matching block glyph. All 16 quadrant combinations have a Unicode block element, so half-block welds every junction with no gaps — e.g. a "tab" box whose bottom edge overlaps a panel's top row welds into one tab-panel outline (`▟ █ ▌` at the junction). The lone-element soft-pill ring is unchanged.
- **Painted content now occludes the border beneath it** (`BORDER-Z-OCCLUDE-1`). The border joiner runs as a final pass and re-derived every border cell, so a higher-stacked element's glyph content could be clobbered by a lower element's border — the reverse of CSS stacking. Painted content now clears the border state at the cells it covers, so a higher element's content paints over a lower element's border (while borders contributed later in stacking order still paint over lower content). This already worked for opaque `background-color` fills; it now works for glyph content too.

## [0.3.8] - 2026-06-05

Only `rdom-tui` bumps (0.3.7 → 0.3.8); the other four crates are unchanged and stay at 0.3.4.

### Fixed

- **Half-block borders now render on a 1-row box** (`PAINT-HALFBLOCK-1ROW-1`). A `BorderStyle::HalfBlock` left/right edge on a `height: 1` element emitted no glyph — the half-block joiner only mapped multi-bit direction masks (corners + runs that join with a neighbour) and fell through to nothing for a lone contributor. It now maps single-bit masks too: a one-cell vertical edge paints `▐`/`▌`, a one-cell horizontal edge `▄`/`▀`, independent of box height. (A *full* half-block ring still needs ≥ 2 rows — the border-box wants a top half, content, and a bottom half.)
- **Pseudo / own text no longer double-paints on pseudo- or mixed-content blocks** (`TREE-BFC-PSEUDO-1`, duplicate-text class). A block whose own text lives in an anonymous box (mixed content) was repainted a second time by the inline paint path at a shifted x (`Label` → `Labelel`); and a text-leaf with `::before`/`::after` plus an absolutely-positioned child (e.g. a chip with an absolute dropdown) spuriously created an anonymous block. The inline paint path now bails when anonymous blocks exist, and the pure-text-leaf carve-out ignores out-of-flow element children — so pseudos + own text paint exactly once.
- **Out-of-flow descendants no longer leak into inline content** (companion to the above). The IFC text walk and the intrinsic inline-width walk both recursed into `display:none` / `position: absolute|fixed` descendants, packing their text into an ancestor's inline run and inflating its max-content width. Both now skip out-of-flow descendants per CSS (out-of-flow boxes contribute nothing to the containing block's in-flow inline content). This also fixes a collapsed-tree leak where a `display:none` `[role=group]` leaked its child text into the parent treeitem.

### Remaining (tracked)

- The `::before` / `::after` **prefix** on a *true* mixed-content block (a text run plus an in-flow block child) is still dropped — folding it into the first/last anonymous block as a reserved fragment is a layout change, tracked in `specs/TECH_DEBT.md`. Does not affect text-leaf elements (chips), which paint pseudos correctly.

## [0.3.7] - 2026-06-04

Only `rdom-tui` bumps (0.3.6 → 0.3.7); the other four crates are unchanged and stay at 0.3.4.

### Fixed

- **`display:none` elements no longer keep a stale layout rect** (`LAYOUT-DISPLAY-NONE-STALE-RECT`). An element laid out while visible kept its rect after going `display:none` — the in-flow layout filter dropped it without zeroing `ext.layout`, so the stale rect still drove paint and hit-test. `layout_node` now collapses every `display:none` child subtree's geometry after laying out children (early-returning on already-zero subtrees, so steady-state hidden subtrees stay O(1)). Masked before by nodes that are rebuilt every frame (e.g. a virtualized `<tbody>`'s cells); exposed by persistent nodes like table headers.
- **Stale anonymous-block boxes no longer double-paint** (`PAINT-RELATIVE-ABSPOS-DOUBLE`). A block element with an element child wraps its own text in an anonymous box (`TuiExt.anonymous_blocks`); when that child was removed the element became a pure-text leaf via a `layout_children` carve-out that set `inline_layout` but never cleared `anonymous_blocks`, so the paint pass kept drawing the old inline run at its previous position (a glyph echoing at a stale slot). `anonymous_blocks` is now cleared once at the top of `layout_children` — only the block-flow arm repopulates it — covering every dispatch path (IFC, pure-text-leaf, flex, block). Both bugs surfaced building `rdom-virtualtable`'s column show/hide dropdown and only appeared under the runtime's *incremental* cascade.

### Internal

- `is_in_flow` promoted from `block.rs` to the layout-pass module root as the single source for the "skip out-of-flow children" filter, shared by flex / fragment layout and the scroll-content walk (`DRY-1`, layout sites).

## [0.3.6] - 2026-06-04

Only `rdom-tui` bumps (0.3.5 → 0.3.6); the other four crates are unchanged and stay at 0.3.4.

### Fixed

- **`<table>` column sizing now respects explicit widths** (`TABLE-COLSYNC-1`). `size_columns` used to write its measured column widths back onto every cell's `inline_style.width` (conflating author intent with the computed result) and stamp `data-rdom-colsync` to force a re-cascade — so an author's explicit width couldn't survive a re-size, `Column`-style fixed widths were silently overwritten, and a sort glyph in an `::after` got clipped. It now resolves each column's *used* width from author input (a cell's `inline_style.width: Fixed`) where specified, content width otherwise, and records it on a **layout-side** field (`TuiExt::table_used_width`, read by flex + intrinsic sizing) — **never** on `inline_style`. The `data-rdom-colsync` hack is gone (the value is read by full layout each frame, not the incremental cascade), obsoleting the `TABLE-COLSYNC-DIRTY-1` workaround. **Divergence:** explicit column widths are honored via inline `style="width"` / `set_width`, not via a CSS rule (`td { width }`) — full CSS table layout is the `TABLE-TFC-1` roadmap item.

## [0.3.5] - 2026-06-03

Only `rdom-tui` bumps (0.3.4 → 0.3.5); the other four crates are unchanged and stay at 0.3.4.

### Fixed

- **`<table>` columns re-cascade after a `size_columns` re-sync** (`TABLE-COLSYNC-DIRTY-1`). `size_columns` wrote each cell's width straight to inline style, which fires no mutation — so the runtime's *incremental* cascade left re-used cells with a stale computed width while full layout read it. It bit `<thead>` headers when a consumer rebuilt only the `<tbody>` (a virtualized table swapping its row window): the column visibly shifted until an unrelated later mutation re-dirtied the header. `size_columns` now stamps a column-width signature (`data-rdom-colsync`) on the `<table>` **only when the widths change**, dirtying the table so the whole subtree (headers included) re-cascades. Surfaced by `rdom-virtualtable`; consumers can drop their own dirty workaround.

### Notes

- **Scroll API for virtualized consumers is already complete** — `Element.scroll_top` / `scroll_left` / `scroll_width` / `scroll_height` (read) and `set_scroll_top` / `set_scroll_left` (write, clamped to the padding-box scrollport, firing a bubbling `scroll` event) are exposed via `TuiAccessors` / `TuiAccessorsMut`. A component can listen to `scroll`, read `scroll_top()`, and re-window.
- **Horizontal scroll for a wide `<table>`** is done the web way: wrap it in a `Row`-flex `overflow-x` container (the `<div style="overflow-x:auto">` analogue); header and body scroll together. A `<table>` is a column-flex container, so it can't be its own cross-axis scroll container — see `SCROLL-CROSS-AXIS-1` in `TECH_DEBT.md`. `colspan`/`rowspan` remain unimplemented (`TABLE-COLSPAN-1`); not needed for virtualization.

## [0.3.4] - 2026-06-03

### Changed

- **Focus is now a typed affordance vocabulary, not a universal background tint** (`FOCUS-VOCAB-1`). The web shows focus with an outline on every focusable element; a TUI can't draw a no-reflow ring, so rdom matches the cue to the element kind instead of flooding every focused element's background. **Atomic controls** (`button`/`input`/`textarea`/`select`/`summary`/`a`/`area`) keep the `:focus` background tint. **Scroll containers** get an accent scrollbar thumb (see below). **Grid/tree/listbox** use their internal cursor. **Everything else** (a bare focusable `<div>`/`<table>` with no scrollbar) gets **no default fill** — a full-area tint on a large container is destructive and unlike the web's outline; the consumer expresses focus in CSS. This replaced the old generic `:focus` tint and its per-element opt-out hacks (`canvas:focus`, `[role=tree]:focus`), which are now deleted. Migration: an app that relied on a focused container being tinted adds its own `:focus { … }` rule.

### Added

- **Scrollable overflow containers are implicitly keyboard-focusable** (`FOCUS-VOCAB-1`), matching modern browsers' keyboard-focusable scrollers. A container qualifies only when it actually shows a scrollbar (clips on an axis *and* content overflows) **and** has no focus stop of its own, so it never adds a redundant tab stop. A non-scrolling `<div>`/`<table>` remains non-focusable, exactly as on the web.
- **Keyboard scrolling acts on the scroll region the focus is in** — the keymap (`ArrowUp`/`Down` (+ `Left`/`Right` for horizontal) by a line, `PageUp`/`PageDown`/`Space`/`Shift+Space` by a page, `Home`/`End` to the ends) scrolls the **nearest scrollable ancestor** of the focused element (the focused element itself when it's a focusable scroll container). So `PageDown` while focused on an `<input>` pages the form's scroll pane. Runs after the editable-key default (a focused field still moves its caret) and after widget activation (`<button>`/tree claim their keys), and `preventDefault` suppresses it.
- **`:focus-within::scrollbar-thumb` UA rule** — the scroll region **containing the focus** (the focused scroll container itself, or the pane around a focused control) shows an accent (DodgerBlue) thumb **glyph** — a thin colored handle, not a filled block; other thumbs stay gray. It marks the region the keyboard scroll keys act on. The container analog of the web's focus outline, at zero extra area (it reuses scrollbar chrome the element already owns).

### Fixed

- **A tree's full-width row highlight no longer bleeds under its vertical scrollbar.** The `[role=treeitem]` cursor/selected-row background is painted edge-to-edge by the guide pass; once a tree can own its scroll, that fill ran under the scrollbar thumb. It now reserves the scrollbar gutter (the rightmost padding-box column) when the tree shows a vertical scrollbar, matching how normal content reserves it.
- **A focused `[role=tree]` no longer double-scrolls on arrow keys.** Its keydown handler moved the cursor (and `scroll_into_view` followed it) but never `preventDefault`'d, so the new focused-scroll-container keymap *also* line-scrolled the same arrow. The tree now claims the keys it navigates, leaving Tab / unhandled keys to fall through.
- **A dynamically-added `<input>` / `<textarea>` is now typeable.** `input::seed_all` only ran once at `App::build`, so an editable mounted later (a switched-in view, a runtime-built form) had no text-node child — focusing it seeded no caret and keystrokes were silently dropped. The focus path now seeds the editable lazily (`input::ensure_seeded`), so any focused editable accepts text whenever it was created.

## [0.3.3] - 2026-06-02

### Added

- **`:where()` selector support** (Selectors Level 4). Matches like `:is()` — any complex selector in its forgiving list matches the element — but contributes **zero specificity**. This is the web-faithful mechanism a component library uses to ship default styles that any author rule overrides without a specificity fight (the role browsers give the UA origin, but achieved inside the author origin — where a downstream library actually lives). Combinators are allowed inside the argument (`:where(table:focus td)`), and since the cascade orders author rules by specificity, a plain `td { … }` beats a `:where(…)`-wrapped default regardless of source order. `:is()` (specificity = most-specific argument) remains unimplemented. Driven by the first component-library consumer (`rdom-virtualtable`), whose highlight defaults now ship in `:where()` so consumers recolor them with ordinary CSS.

## [0.3.2] - 2026-06-02

### Fixed

- **Removing the focused (or otherwise observed) node from inside an event handler no longer panics** (`DROP-SUBTREE-FREE-ORDER-1`). `drop_subtree` freed the subtree's arena slots *before* firing its `ChildListChanged` mutation, so any observer that inspected the removed node in its callback — the dirty tracker, the implicit blur/focusout-on-detach dispatch — dereferenced a reclaimed slot and panicked (`unwrap` on `None`). It now fires the mutation while the subtree is still alive, then frees — matching `remove_child` and the MutationObserver contract that `removedNodes` are readable in the callback. Removing the focused element in a click/keydown handler (modal dismiss, list-item delete, view switch) is now safe, as it is on the web.

## [0.3.1] - 2026-06-02

### Fixed

- **A focused `<canvas>` is no longer painted over** (`UA-FOCUS-OVERRIDABLE-1`). rdom's focus indicator is a background tint (a no-reflow TUI substitute for the web's focus outline). For form controls that's the right affordance, but `<canvas>` is a replaced/content element the app paints — the web focuses a canvas with a non-destructive outline and never touches its pixels. rdom now matches that: a focused canvas keeps its background by default (a UA `canvas:focus` opt-out), so app-drawn canvases (charts, etc.) aren't grayed out on focus. Apps that *want* a focused-canvas background can still set one.
- **The focus tint is now overridable.** It was `!important` (and UA-`!important` is the strongest cascade origin), so it couldn't be overridden by any author or inline rule. The generic `:focus` tint is now non-important; `!important` is retained only on `input/textarea/select:focus`, where it must beat those controls' own high-specificity field background. No visual change to existing UIs — only canvas (now clean) and overridability.

## [0.3.0] - 2026-06-02

Substrate-honesty release, driven by the first downstream consumer (`rdom-extensions`, a data-visualization component crate). All five published crates bump together to `0.3.0`. Pre-1.0, so this minor carries breaking changes alongside additive ones.

### Added

- **`EventCtx::request_redraw()`** — a listener can ask the runtime to repaint after it mutated state the DOM mutation tracker can't see (e.g. a `<canvas>` whose paint reads external app state). Harvested on both the keyboard and mouse paths. Unblocks interactive canvas components.
- **`TuiStyle::flex()` / `flex_row()` / `flex_column()` / `inline_flex()`** — convenience builders for `display: flex` (`Display::Block` + `Flow::Flex`), avoiding the `.display()`-resets-`.flow()` ordering trap.
- **`Dom::remove_child_dropping` / `clear_children_dropping`** — detach **and** free a subtree in one call, for high-churn UIs that would otherwise leak arena slots (detach alone never frees; only `drop_subtree` does).
- **`RenderContext::for_test(buffer, area)`** — public constructor so downstream crates can unit-test `<canvas>` paint code over a scratch `Buffer`.

### Changed / Fixed

- **Geometry node setters now drive layout.** `set_width` / `set_height` / `set_min_*` / `set_max_*` / `set_direction` / `set_padding` / `set_border` / `set_gap` / `set_overflow` wrote raw `ext` fields the cascade and layout never read — a silent no-op for layout while the accessors echoed the set values. They now write `inline_style` (the cascade input), so they actually affect layout. **Breaking:** the accessors return `None` when a property is unset (was `Some(default)`); `set_inline_style` replaces the whole inline style, including geometry the setters wrote (seed the inline style first). Also fixed: `scroll_into_view`'s scrollable-ancestor check read a never-populated raw `overflow` field; it now honors CSS `overflow`.

### Removed

- **The dead `render::RenderContext`** (used only by its own tests) and its crate-root re-export. The canvas `RenderContext` (the `<canvas>` `set_paint` callback type) is now re-exported at the crate root as the canonical `RenderContext`, so `use rdom_tui::*` resolves to the one a paint callback actually receives. **Breaking** only for code that imported the unused type.

See [`specs/SUBSTRATE-0.3.0.md`](specs/SUBSTRATE-0.3.0.md) for the full rationale and the `TECH_DEBT` IDs (`EXT-LAYOUT-SETTERS-1`, `EVENT-REDRAW-1`, `TUISTYLE-FLEX-BUILDER-1`, `RENDERCTX-DEDUP-1`, `CANVAS-TEST-CTOR-1`, `ARENA-RECLAIM-1`).

## [0.2.0] - 2026-06-02

Second release. All five published crates bump together to `0.2.0` (shared workspace version); `rdom-core` changed, so every consumer re-pins and re-publishes. Pre-1.0, so this minor carries both additive features and breaking changes.

### Added

- **Block formatting context.** Semantic HTML now stacks per the web platform with no CSS at all — `<div><h1></h1><p></p></div>` is a block-flow column at intrinsic heights. CSS 2.1 §10 normal flow + §8.3.1 margin collapse + §10.6.3 height resolution + CSS3 `gap` on block containers + atomic `inline-block` participating in inline formatting contexts, layered on top of the original flex pass.
- **Native ARIA tree** — `<ul role=tree>` / `role=treeitem` / `role=group`. Guide lines (`│ ├ └`) + `▾`/`▸` chevrons, keyboard navigation (Arrows / Home / End / Enter / Space) with an `aria-activedescendant` cursor, pointer + keyboard activation through one bubbling `click`, collapse/expand via `aria-expanded` (presence = branch, so lazy/unloaded branches still render), lazy children via `aria-busy`, and scroll-into-view that follows the cursor.
- **`calc()` value system** for `width` / `height` / inset / length axes — recursive-descent parser with CSS precedence, parentheses, unary minus, nested `calc()`, banker's rounding onto the cell grid. (`padding` / `margin` / `gap` accept constant-only `calc()`.)
- **Event surface bundle** — `keyup` (terminals with the kitty keyboard protocol), `contextmenu` (right-click + Shift+F10), `dblclick`, `resize` (document root), `scroll` (on offset change). Plus the implicit-detach ceremony: `blur` / `focusout` on focus loss and `mouseout` / `mouseleave` on hover loss fire *before* structural unlink, bubbling through the still-live ancestor chain.
- **Multi-slot stylesheet API** — `App::push_stylesheet` / `remove_stylesheet` + `cascade_all`, so a consumer can stack (and swap) author sheets on top of the UA sheet.
- **Full `border-style` keyword set** with terminal-faithful degradation (`solid` / `double` get distinct glyphs; `dashed` / `dotted` / `ridge` / `outset` / `groove` / `inset` rank in conflict resolution and render as `solid`), the `hidden` kill-switch, and the rdom-specific `half-block` pill style.

### Changed

- **Layered border model (breaking).** `border-collapse` is now layout-only and **non-inheriting**, and applies to any container's direct children (not only `<table>`). Visual merging is per-direction (style / color / priority) with CSS Tables 3 §11.5 conflict resolution. Declare `border-collapse` on each container whose direct children should share borders.
- **`calc()` support made `Size` / `Length` non-`Copy` (breaking)** — `.clone()` at move boundaries; `ComputedStyle` / `AnimatedValue` are no longer `Copy`.
- **Percentage height resolves against flex-sized ancestors** (CSS Flexbox §9.8) — `height: 100%` inside a `flex: 1` pane resolves instead of falling back to content height.
- **Scroll offset clamps to content on layout**, not only on input — replacing a scroll container's content with shorter content snaps a stale `scrollTop` / `scrollLeft` back into range, matching the browser.

### Fixed

- Tree guides clip to scroll-container ancestors and span wrapped labels; the active-row highlight fills the tree's padding box.
- Mixed-content blocks (leading text + a block child) lay out and paint correctly under scroll — no leading-text bleed onto a scrolled-in sibling.
- A run of layout / paint fixes surfaced by dogfooding: `border-collapse` junctions, flex-shrink content squish, `position: sticky` under `overflow: auto`, and overflow-clip bleed between an app-shell pane and its sibling.

The in-tree `rdom-showcase` dogfooding app was built out across this release but is `publish = false` — it is not part of the crates.io publish set.

## [0.1.0] - 2026-05-17

Initial public release. The five workspace crates ship together on crates.io:

- `rdom-core` — pure DOM substrate (arena, selectors, events, mutation observers, selection)
- `rdom-style` — CSS data model + property dispatch (leaf crate)
- `rdom-css` — CSS string parser
- `rdom-parser` — HTML-ish template parser (`parseFromString` equivalent)
- `rdom-tui` — terminal cascade + layout + paint + runtime + native HTML built-ins

See [README.md](README.md) §"What's in 0.1.0" for the detailed scope. Summary:

### Added

- **DOM substrate** — arena-backed nodes, CSS selectors (Selectors Level 4 subset), 3-phase event dispatch with `stopPropagation` / `preventDefault` / `AbortSignal`, `MutationObserver`, `Selection` / `Range` / `Position`, HTML serialization (`outer_markup` / `inner_markup`).
- **HTML template parser** — hand-rolled, no external dependencies, round-trippable for the supported subset.
- **CSS string parser** — standalone stylesheets, `<style>` blocks in templates, and inline `style="…"` attributes through one parser. Full property-dispatch-table coverage (color, sizing, padding, border, positioning, transitions). `!important`, custom properties (`var()` in color positions), comma-separated rules, lenient + strict modes.
- **Cascade** — UA / author / inline ladder with `!important` inversion. CSS-faithful specificity. Interaction pseudo-classes (`:hover`, `:focus`, `:checked`, `:indeterminate`, `:open`). Pseudo-elements (`::before`, `::after`, `::selection`, `::backdrop`). `content` property. Custom properties.
- **Layout + paint** — flexbox for flex containers, `display: inline-block` for content-hugging chrome (buttons, badges), inline formatting (word wrap at whitespace + CJK + hyphens, `<br>`, `white-space: normal|pre|nowrap`), positioned `::before` / `::after` pseudo-elements (`position: relative | absolute | fixed` honoring `top` / `right` / `bottom` / `left`). Truecolor / 256-color fallback. ANSI emission with synchronized output (DEC 2026).
- **Runtime** — event loop with HTML-spec rendering-steps model, hit testing, mouse routing (`mousedown` / `mouseup` / `click` synthesized on nearest common ancestor), keyboard routing, focus navigation (`tabindex`, `Tab` / `Shift-Tab`, `autofocus`), pointer capture, text selection (mouse drag, `Shift+arrow`, `Ctrl-A`, double/triple-click) + system clipboard (`arboard`, OSC 52 fallback), panic safety (terminal state restored on panic).
- **Native HTML built-ins** — `<button>`, `<label>`, `<details>` / `<summary>`, `<input>` family (text, password, number, checkbox, radio, range, submit, button, reset, hidden, color, search, email, tel, url), `<textarea>`, `<select>` / `<option>`, `<form>`, `<dialog>`, `<progress>`, `<meter>`, `<table>` family with column-width sync, `<canvas>` + `RenderContext` escape hatch, `<a href>` with scheme dispatch.
- **User-agent stylesheet (137 rules)** — out-of-the-box visual chrome on every native HTML element. Bracketed `[ Label ]` buttons in accent (LightBlue) fg. Rounded LightBlue-bordered modal dialogs with `::backdrop` overlay. `▸`/`▾` disclosure triangles on `<details>` / `<summary>` with closed-body suppression. Bullet markers on `<ul>` and `<ol>`. `│` blockquote rail, `─` `<hr>` rule, `▾` `<select>` chevron pinned via positioned `::after`. Unified two-signal `:focus` indicator (SGR-7 inverse + leading `▸ ` glyph) with per-state composition (`▸[ Save ]`, `▸[x] `, `▸(•) `) so the focus signal survives color-vision deficiency and reverse-suppressed themes.
- **DOM API completeness** — per-tag accessors (`input_value`, `select_options`, `details_open`, `form_elements`, …), CSSOM (`style.set_property`, `style_declaration` with camelCase aliases), scroll APIs (`scroll_top` / `scroll_into_view`), document hit-testing (`element_from_point`), `bounding_rect`, programmatic focus/blur/click dispatch.
- **Positioning** — `position: {static, relative, absolute, fixed}`, `z-index` parsing, `top` / `right` / `bottom` / `left`, `inset` shorthand. Paint order is document order (no stacking contexts in 0.1.0).
- **Timers + transitions** — `setTimeout`, `setInterval`, `requestAnimationFrame` with `DOMHighResTimeStamp`, CSS `transition` with cubic-bezier timing.
- **Terminal niceties** — OSC 52 clipboard fallback, OSC 8 hyperlinks for `<a href>`, truecolor + 256-color fallback, integer-cell grid.
- **Seven examples** — `counter_button`, `scrollable_list`, `selectable_text`, `tab_form`, `parse_and_render`, `dom_api_demo`, plus `ua_chrome` (a naked-UA showcase that demos every UA-chrome class — buttons, lists, disclosure, dialog — with no author CSS except a structural shell).
- **Snapshot test harness** — `crates/rdom-tui/tests/common/mod.rs` renders an example's DOM through the cascade → layout → paint pipeline and golden-compares the painted buffer (`UPDATE_SNAPSHOTS=1` regen path, unified-diff on mismatch). First consumer: the `ua_chrome` example plus a focused-button companion that witnesses the two-signal focus indicator. Catches visible regressions without needing a TTY.

### Out of scope for 0.1.0

See [README.md](README.md) §"Roadmap" and §"Out of scope (by design)" for the full breakdown, and [`specs/DIVERGENCES.md`](specs/DIVERGENCES.md) for every deliberate departure from the web platform.

[0.1.0]: https://github.com/miskun/rdom/releases/tag/v0.1.0
