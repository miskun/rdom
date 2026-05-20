# rdom

A DOM for terminal applications, in Rust.

`rdom` brings the architecture of the browser DOM — arena-backed nodes, CSS-style cascade, flexbox layout, capture/bubble events, mutation observers, selection ranges — to text-mode UIs. It targets terminals (via `crossterm`) but the core tree is renderer-agnostic and can drive headless or alternate backends.

The browser DOM is the reference model: native HTML elements, CSS-faithful cascade, web-platform event semantics. Higher-level component libraries live in downstream projects, not in this repo.

## Quick start

Install:

```toml
[dependencies]
rdom-tui    = "0.1"
rdom-parser = "0.1"   # optional: HTML-ish template strings
rdom-css    = "0.1"   # optional: parse real CSS at runtime
```

`rdom-core` and `rdom-style` are pulled in transitively. For headless DOM work — building and querying a tree without rendering anything — depend on `rdom-core` alone.

Build a tree, attach styles, run it:

```rust
use rdom_parser::parse;
use rdom_tui::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (dom, _ids) = parse::<TuiExt>(r#"
        <div class="hero">
            <h1>Hello, rdom!</h1>
            <button class="primary">Click me</button>
        </div>
    "#)?;

    let sheet = rdom_css::from_css(r#"
        .hero    { padding: 1 2; border: solid; }
        h1       { color: red; font-weight: bold; }
        .primary { background-color: blue; color: white; padding: 0 2; }
        .primary:hover { background-color: lightblue; }
    "#);

    App::new(dom, sheet)?.run()?;
    Ok(())
}
```

See [`crates/rdom-tui/examples/`](crates/rdom-tui/examples/) for seven working demos including buttons with state, scrollable lists, text selection, focusable forms, end-to-end parse+render, and a naked-UA chrome showcase.

## Crates

| Crate | What it is |
|---|---|
| [`rdom-core`](crates/rdom-core) | Pure DOM. Arena, `NodeId`, attributes, classes, tree mutation, CSS selectors, 3-phase event dispatch, `MutationObserver`, `AbortSignal`, `Selection`/`Range`/`Position`. Zero rendering deps. |
| [`rdom-style`](crates/rdom-style) | CSS data model + property dispatch + value parsers. Leaf crate; consumed by `rdom-css` (the parser) and `rdom-tui` (the renderer). |
| [`rdom-css`](crates/rdom-css) | CSS parser. Tokenizer + block parser + `<style>`-tag extraction + inline-style seeding. Produces `Stylesheet` / `TuiStyle` via `rdom-style`'s property dispatch. |
| [`rdom-tui`](crates/rdom-tui) | Terminal backend. CSS cascade, flexbox layout, paint pass, ANSI emission, inline formatting (word wrap, CJK breaks, `<br>`, `white-space`), runtime (event loop, hit test, keyboard/mouse routing, focus, text selection + clipboard), native HTML element behaviors (`<button>`, `<input>` family, `<select>`, `<form>`, `<details>`, `<dialog>`, `<progress>`, `<meter>`, `<table>` family, `<canvas>`). |
| [`rdom-parser`](crates/rdom-parser) | HTML-ish template parser → `Dom<Ext>`. `parseFromString` equivalent. Hand-rolled, no external parser deps. |

## What's in 0.1.0

- **DOM substrate.** Arena, attributes, classes, mutation, CSS selectors (Selectors Level 4 subset), 3-phase event dispatch with `stopPropagation` / `preventDefault` / `AbortSignal`, `MutationObserver`, `Selection` / `Range` / `Position`, serialization (`outer_markup` / `inner_markup`).
- **HTML template parser.** Hand-rolled, no external deps. `parseFromString` equivalent. Round-trippable for the supported subset.
- **CSS string parser.** Real CSS in, `Stylesheet` out. Three surfaces unified: standalone stylesheets, `<style>` blocks in templates, inline `style="…"`. Selectors, all properties in the dispatch table (color, sizing, padding, border, positioning, transitions), `!important`, custom properties (`var()` in color positions), comma-separated rules, lenient + strict modes.
- **Cascade.** UA / author / inline ladder with `!important` inversion. CSS-faithful specificity. Interaction pseudo-classes (`:hover`, `:focus`, `:checked`, `:indeterminate`, `:open`, …). Pseudo-elements (`::before`, `::after`, `::selection`, `::backdrop`). `content` property. Custom properties.
- **Layout + paint.** Flexbox for flex containers. `display: inline-block` for content-hugging chrome (buttons, badges, tags). Inline formatting (word wrap at whitespace + CJK + hyphens, `<br>`, `white-space: normal|pre|nowrap`, per-grapheme source tracking). Positioned `::before` / `::after` pseudo-elements (`position: relative | absolute | fixed` honoring `top` / `right` / `bottom` / `left`). Truecolor / 256-color fallback. ANSI emission with synchronized output (DEC 2026).
- **Runtime.** Event loop with rendering-steps model (drain, tick, rAF, cascade + layout + paint, sleep). Hit testing, mouse routing (`mousedown` / `mouseup` / `click` synthesized on nearest common ancestor — matches HTML), keyboard routing, focus navigation (`tabindex`, `Tab` / `Shift-Tab`, autofocus), pointer capture, text selection (mouse drag, `Shift+arrow` including vertical with sticky-x and line-edge via `Shift+Home`/`End`, `Ctrl-A`, double/triple-click, `user-select: none|all|contain`) + system clipboard (`arboard`, OSC 52 fallback), panic safety (terminal state restored on panic).
- **Native HTML built-ins.** `<button>`, `<label>`, `<details>` / `<summary>`, `<input>` family (text, password, number, checkbox, radio, range, submit, button, reset, hidden, color, search, email, tel, url), `<textarea>`, `<select>` / `<option>`, `<form>`, `<dialog>`, `<progress>`, `<meter>`, `<table>` family + column-width sync, `<canvas>` + `RenderContext` escape hatch, `<a href>` with scheme dispatch. Editable surfaces honor `caret-color` (cell bg) and the rdom-extension `caret-text-color` (glyph fg); both default to inverting the cell's cascaded fg/bg. `readonly` fires cancelable `beforeinput` (matches UI Events L2 §5). `contenteditable` supports cross-text-node edits across inline boundaries.
- **User-agent stylesheet.** 128 UA rules ship visual chrome on every native element so naked HTML looks attractive out of the box. Bracketed `[ Label ]` buttons in accent fg. Rounded LightBlue-bordered modal dialogs. `▸`/`▾` disclosure triangles. `•` list bullets. `│` blockquote rail, `─` `<hr>` rule, `▾` `<select>` chevron. Subtle background-tint `:focus` indicator (a single `!important` rule, color-only — no reverse-video, no glyph shift) that authors can override with their own `!important` rule. Run `cargo run -p rdom-tui --example ua_chrome` to see it.
- **DOM API completeness.** Per-tag accessors (`input_value`, `select_options`, `details_open`, `form_elements`, …), CSSOM (`style.set_property`, `style_declaration`, camelCase aliases), scroll APIs (`scroll_top` / `scroll_into_view`), document hit-testing (`element_from_point`), `bounding_rect`, focus/blur/click programmatic dispatch.
- **Positioning.** `position: {static, relative, absolute, fixed}`, `z-index` parsing, `top` / `right` / `bottom` / `left`, `inset` shorthand. Paint order is document order (no stacking contexts in 0.1.0).
- **Timers + transitions.** `setTimeout` / `setInterval`, `requestAnimationFrame` with `DOMHighResTimeStamp`, CSS `transition` with cubic-bezier timing.
- **Terminal niceties.** OSC 52 clipboard fallback, OSC 8 hyperlinks for `<a href>`, truecolor + 256-color fallback, integer-cell grid, monospaced advance.

## Roadmap

- **0.2.0** — `calc()` value system + event surface bundle (`dblclick`, `contextmenu`, `keyup`, `mousemove`, `scroll`, `resize`).
- **0.3.0** — Client-side routing primitive.
- **0.4.0** — Async tasks during event handlers.

Open polish items (no fixed milestone): form validation (`:required` / `:invalid` / `pattern`), `:focus-visible`, `::placeholder` / `:placeholder-shown`, cross-text-node undo (compound edit entries — see `EDIT-1`), undo/redo coalescing, blinking caret, whitespace normalization in clipboard serialization.

## Out of scope (by design)

- **Subpixel anything.** Terminal cells are integer-aligned, monospaced. No subpixel positioning, no fractional widths, no anti-aliasing.
- **`@media` / `@keyframes` / `@font-face` / `@supports`.** Tokens recognized; rule bodies skipped with `WarningKind::UnsupportedAtRule`. CSS animation lands incrementally through named milestones, not via `@keyframes`.
- **Touch, IME / composition, drag-and-drop, long-press gestures.** Web-platform features tied to input devices or interaction models that don't map onto a terminal.
- **Higher-level component libraries.** The substrate ships native HTML elements and zero opinionated components — same shape as the browser. Component libraries that compose those primitives belong in downstream consumer crates, not in this workspace. See [`CLAUDE.md`](CLAUDE.md) §"Substrate First, Backend Second" for the rationale.

## Examples

```bash
cargo run -p rdom-tui --example counter_button     # button + state
cargo run -p rdom-tui --example scrollable_list    # overflow + wheel scrolling
cargo run -p rdom-tui --example selectable_text    # text selection + clipboard
cargo run -p rdom-tui --example tab_form           # focus navigation + form controls
cargo run -p rdom-tui --example parse_and_render   # rdom-parser + rdom-css + rdom-tui
cargo run -p rdom-tui --example dom_api_demo       # form-edit / tree-walk / cssom
cargo run -p rdom-tui --example ua_chrome          # naked HTML built-ins with UA defaults
```

## Design docs

- [`specs/DESIGN.md`](specs/DESIGN.md) — architectural overview: crate map, non-negotiable invariants, roadmap.
- [`specs/DIVERGENCES.md`](specs/DIVERGENCES.md) — every deliberate departure from the web platform that ships in 0.1.0.
- [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md) — open debt + accepted simplifications.

Detailed behavior lives in the code: each module has a top-level doc comment, and tests document the contracts. The web specs (WHATWG DOM, CSS, UI Events) are the reference; rdom tracks them within the supported subset.

## Testing

```bash
cargo test --workspace                                  # all unit + integration tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
bash scripts/spec-lint.sh                               # spec voice-drift lint
```

CI runs the same gates on `[ubuntu-latest, macos-latest, windows-latest]` for every push and PR against `main`.

## License

MIT — see [LICENSE](LICENSE).
