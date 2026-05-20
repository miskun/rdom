# Changelog

All notable changes to rdom will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
