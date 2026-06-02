# Changelog

All notable changes to rdom will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Focus is now a typed affordance vocabulary, not a universal background tint** (`FOCUS-VOCAB-1`). The web shows focus with an outline on every focusable element; a TUI can't draw a no-reflow ring, so rdom matches the cue to the element kind instead of flooding every focused element's background. **Atomic controls** (`button`/`input`/`textarea`/`select`/`summary`/`a`/`area`) keep the `:focus` background tint. **Scroll containers** get an accent scrollbar thumb (see below). **Grid/tree/listbox** use their internal cursor. **Everything else** (a bare focusable `<div>`/`<table>` with no scrollbar) gets **no default fill** — a full-area tint on a large container is destructive and unlike the web's outline; the consumer expresses focus in CSS. This replaced the old generic `:focus` tint and its per-element opt-out hacks (`canvas:focus`, `[role=tree]:focus`), which are now deleted. Migration: an app that relied on a focused container being tinted adds its own `:focus { … }` rule.

### Added

- **Scrollable overflow containers are implicitly keyboard-focusable** (`FOCUS-VOCAB-1`), matching modern browsers' keyboard-focusable scrollers. A container qualifies only when it actually shows a scrollbar (clips on an axis *and* content overflows) **and** has no focus stop of its own, so it never adds a redundant tab stop. A non-scrolling `<div>`/`<table>` remains non-focusable, exactly as on the web.
- **A focused scroll container scrolls with the keyboard** — `ArrowUp`/`ArrowDown` (and `ArrowLeft`/`ArrowRight` when horizontally scrollable) by a line, `PageUp`/`PageDown`/`Space`/`Shift+Space` by a page, `Home`/`End` to the ends. Runs after the editable-key default, so a focused `<input>`/`<textarea>` still moves its caret. `preventDefault` on the `keydown` suppresses it.
- **`:focus::scrollbar-thumb` UA rule** — a focused scroll container's scrollbar thumb **glyph** turns accent (DodgerBlue, foreground — a colored handle, not a filled block); unfocused thumbs stay gray. The container analog of the web's focus outline, at zero extra area (it reuses scrollbar chrome the element already owns).

### Fixed

- **A tree's full-width row highlight no longer bleeds under its vertical scrollbar.** The `[role=treeitem]` cursor/selected-row background is painted edge-to-edge by the guide pass; once a tree can own its scroll, that fill ran under the scrollbar thumb. It now reserves the scrollbar gutter (the rightmost padding-box column) when the tree shows a vertical scrollbar, matching how normal content reserves it.

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
