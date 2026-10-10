# rdom

A DOM for terminal applications, in Rust.

`rdom` brings the architecture of the browser DOM — arena-backed nodes, CSS-style cascade, flexbox and grid layout, capture/bubble events, mutation observers, selection ranges — to text-mode UIs. It targets terminals (via `crossterm`) but the core tree is renderer-agnostic and can drive headless or alternate backends.

The browser DOM is the reference model: native HTML elements, CSS-faithful cascade, web-platform event semantics. Higher-level component libraries live in downstream projects, not in this repo.

## Quick start

Install:

```toml
[dependencies]
rdom-tui    = "0.6"
rdom-parser = "0.6"   # optional: HTML-ish template strings
rdom-css    = "0.6"   # optional: parse real CSS at runtime
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

See [`crates/rdom-tui/examples/`](crates/rdom-tui/examples/) for three self-contained programs (counter button, tab form, parse + render) and [`crates/rdom-showcase/examples/`](crates/rdom-showcase/examples/) for the eleven showcase demos runnable standalone: scrollable lists, text selection, an ARIA tree with lazy children, sticky headers, border collapse, the naked-UA chrome tour, and the acid test's pages.

## Crates

| Crate | What it is |
|---|---|
| [`rdom-core`](crates/rdom-core) | Pure DOM. Arena, `NodeId`, attributes, classes, tree mutation, CSS selectors, 3-phase event dispatch, `MutationObserver`, `AbortSignal`, `Selection`/`Range`/`Position`. Zero rendering deps. |
| [`rdom-style`](crates/rdom-style) | CSS data model + property dispatch + value parsers. Leaf crate; consumed by `rdom-css` (the parser) and `rdom-tui` (the renderer). |
| [`rdom-css`](crates/rdom-css) | CSS parser. Tokenizer + block parser + `<style>`-tag extraction + inline-style seeding. Produces `Stylesheet` / `TuiStyle` via `rdom-style`'s property dispatch. |
| [`rdom-tui`](crates/rdom-tui) | Terminal backend. CSS cascade, flexbox and grid layout, paint pass, ANSI emission, inline formatting (word wrap, CJK breaks, `<br>`, `white-space`), runtime (event loop, hit test, keyboard/mouse routing, focus, text selection + clipboard), native HTML element behaviors (`<button>`, `<input>` family, `<select>`, `<form>`, `<details>`, `<dialog>`, `<progress>`, `<meter>`, `<table>` family, `<canvas>`). |
| [`rdom-parser`](crates/rdom-parser) | HTML-ish template parser → `Dom<Ext>`. `parseFromString` equivalent. Hand-rolled, no external parser deps. |

## Features

Everything below is on `main` and ships in 0.6.0 (not yet published). Upgrading from 0.5: [`UPGRADING-0.6.md`](UPGRADING-0.6.md). Release notes for every version: [`CHANGELOG.md`](CHANGELOG.md). What rdom-tui's CSS covers, area by area with worked examples: the [rdom-tui README](crates/rdom-tui/README.md#css-at-a-glance).

- **DOM substrate.** Arena with generational `NodeId`s, attributes, classes, tree mutation, CSS selectors (Selectors 4), 3-phase event dispatch with `stopPropagation` / `preventDefault` / `AbortSignal`, `MutationObserver`, `Selection` / `Range` / `Position`, the Custom Highlight API registry, serialization (`outer_markup` / `inner_markup`).
- **HTML template parser.** Hand-rolled, no external deps. `parseFromString` equivalent, tokenizing per HTML §13.2. Round-trippable for the supported subset.
- **CSS parser.** Real CSS in, `Stylesheet` out, from standalone sheets, `<style>` blocks (live under an `App`) and inline `style="…"`. Every property in the dispatch table, `!important`, custom properties (`var()` in every property), the CSS-wide keywords and `all`, CSS Nesting, and the at-rules `@layer`, `@scope`, `@import` (through a host loader), `@property`, `@counter-style`, `@keyframes`, `@starting-style`, `@media`, `@supports`, `@container` and `@position-try`; lenient and strict modes with positioned warnings.
- **Cascade and selectors.** UA / author / inline ladder with `!important` inversion, cascade layers and `revert` / `revert-layer`, `@scope` proximity, CSS-faithful specificity. The Selectors 4 structure — `:has()`, `:is()` / `:where()` / `:not()`, `:nth-child(An+B of S)` and its family, `:scope`, `:lang()`, `:dir()`, attribute case flags — and the interaction and form states (`:hover`, `:active`, `:focus-visible`, `:checked`, `:indeterminate`, `:open`, `:valid` / `:invalid`, `:user-valid` / `:user-invalid`, `:modal`, `:popover-open`, …). Pseudo-elements: `::before`, `::after`, `::marker`, `::first-line`, `::first-letter`, `::selection`, `::highlight()`, `::details-content`, `::backdrop`, `::placeholder`, `::scrollbar` / `::scrollbar-thumb`.
- **Values and color.** Lengths in cells, `%`, `fr`, `ch`, `lh` / `rlh`, the viewport and container units; the math functions (`calc()`, `min()`, `max()`, `clamp()`, `round()`, `mod()`, `rem()`, `abs()`, `sign()`, trigonometric and exponential), banker's rounding onto the cell grid. The CSS Color 4 / 5 `<color>` grammar — `hwb()`, `lab()` / `lch()` / `oklab()` / `oklch()`, `color()`, `color-mix()`, relative colors, system colors — with alpha composited over what lies beneath, and `color-scheme` / `light-dark()` following the terminal's light or dark theme (OSC 11 at startup, DEC mode 2031 reports on Unix).
- **Layout.** Block flow with margin collapsing, the root as initial containing block; flexbox and Box Alignment; grid (track sizing, named lines and areas, auto-placement, `subgrid`); a table formatting context (automatic and `fixed` layout, spans, captions, separated and collapsed borders); multi-column layout (balancing, `column-fill`, `column-rule`, `column-span: all`) over a block fragmentation engine (`break-*`, `orphans` / `widows`, `box-decoration-break`); `box-sizing`, the intrinsic sizes, `stretch`, `calc-size()`, `aspect-ratio`, logical properties.
- **Positioning, floats, overflow and scrolling.** `position: relative | absolute | fixed | sticky`, `z-index` and stacking contexts (CSS 2.1 Appendix E), floats and clearance; anchor positioning — `anchor-name`, `position-anchor` (a popover anchored to its invoker), `anchor()` / `anchor-size()`, `position-area`, `anchor-center`, `position-try-fallbacks` with `@position-try` and the flip tactics, `position-visibility`; `overflow` (incl. `clip`), `text-overflow`, `line-clamp`, scrollbars and `scrollbar-gutter`, `overscroll-behavior`, `scroll-padding` / `scroll-margin`, scroll snapping, smooth scrolling.
- **Inline text.** Word wrap at a UAX #14 subset with `word-break` / `line-break` / `overflow-wrap` / soft hyphens, `white-space` and its longhands, `text-align`, `text-indent`, `text-transform`, `text-wrap` (`balance`, `pretty`), tab stops, `letter-spacing` / `word-spacing` in whole cells, `line-height` in whole rows, `vertical-align`, `text-decoration` with underline styles and colors where the terminal draws them, numeric `font-weight`.
- **Lists and generated content.** `::marker` hung in the list's padding, every predefined counter style and `@counter-style`, `counter()` / `counters()`, quotes by language, `content` with alt text.
- **Effects.** `translate` and `transform`'s translate functions in whole cells (rotation and scaling parse, animate and draw nothing); `filter` and `backdrop-filter` with the color-matrix functions, `opacity()` and `drop-shadow()`; `mix-blend-mode` and `isolation`; `clip-path` (`inset()`, `circle()`, `ellipse()`, `polygon()`) and the legacy `clip: rect()`.
- **Conditional rules and containment.** `@media` against the terminal (its size in cells, `prefers-color-scheme` from its background, the preferences an app reports, `matchMedia`), `@supports` tested with rdom's own parser, `@container` size and style queries; `contain`, `content-visibility` and `will-change`.
- **Transitions and animations.** `transition` with the keyword timing functions, `cubic-bezier()`, `steps()` and `linear()` — every animatable longhand interpolates by its spec's animation type, geometry included; `interpolate-size` and `calc-size()` animate `height: auto`; `@starting-style`; `@keyframes` animations with every `animation-*` property and their events; scroll-driven animations; `setTimeout` / `setInterval` and `requestAnimationFrame`.
- **Runtime.** Event loop with the rendering-steps model (drain, tick, rAF, cascade + layout + paint, sleep). Hit testing, mouse routing (`click` synthesized on the nearest common ancestor, as in HTML), keyboard routing, focus navigation (`tabindex`, `Tab` / `Shift-Tab`, autofocus, focus scrolling into view), pointer capture, the terminal pointer's shape from `cursor`, text selection (mouse drag, `Shift+arrow`, `Ctrl-A`, double / triple click, `user-select`) and the system clipboard (`arboard`, OSC 52 fallback), panic safety (terminal state restored on panic).
- **Native HTML built-ins.** `<button>`, `<label>`, `<details>` / `<summary>`, the `<input>` family (text, password, number, checkbox, radio, range, submit, button, reset, hidden, color, search, email, tel, url, image), `<textarea>`, `<select>` / `<option>`, `<form>` with constraint validation, `<fieldset disabled>`, `<dialog>` (modal in the top layer, with an inert page), the `popover` attribute with `popovertarget` invokers and light dismiss, `<progress>`, `<meter>`, the `<table>` family, `<canvas>` + `RenderContext`, `<a href>` with scheme dispatch. Text fields have a field background and no border — a border costs a terminal row above and below — so mark a form state with the field's colours, or add `border` yourself (rdom-tui's [Form states](crates/rdom-tui/docs/RECIPES.md#form-states) recipe). Editable surfaces honor `caret-color` and the rdom-extension `caret-text-color`; `contenteditable` edits across inline boundaries; Blink-model undo.
- **User-agent stylesheet.** HTML's UA rules (about 190) give naked HTML its look: bracketed `[ Label ]` buttons, rounded modal dialogs, `▸` / `▾` disclosure triangles, `•` / `◦` / `▪` list markers and numbered `<ol>`, `│` blockquote rail, `─` `<hr>`, `▾` select chevron, a background-tint `:focus-visible` indicator. Run `cargo run -p rdom-showcase --example ua_chrome` to see it.
- **DOM API completeness.** Per-tag accessors (`input_value`, `select_options`, `details_open`, `form_elements`, …), CSSOM (`style.set_property`, `style_declaration`, camelCase aliases), scroll APIs (`scroll_top` / `scroll_into_view`), `element_from_point`, `bounding_rect` / `client_rects`, programmatic focus / blur / click.
- **Bounded input.** No CSS, markup or DOM can abort the process: values that drive loops are bounded, and every recursion over input depth has a named cap — CSS blocks 32, selector arguments 32, parsed trees 512 (as Blink and WebKit), the box tree 128 — with clean degradation past it ([`specs/DIVERGENCES.md`](specs/DIVERGENCES.md) §2).
- **Terminal niceties.** OSC 52 clipboard fallback, OSC 8 hyperlinks for `<a href>`, truecolor + 256-color fallback, synchronized output (DEC 2026), the kitty keyboard protocol, integer-cell grid, monospaced advance.

## Roadmap

- **0.6.0** — CSS completeness: every CSS feature that means something in a terminal ([`specs/CSS-COMPLETE-2026-10.md`](specs/CSS-COMPLETE-2026-10.md)). What is still missing today is listed in [`specs/DIVERGENCES.md`](specs/DIVERGENCES.md) §3.
- **0.7.0** — Client-side routing primitive.
- **0.8.0** — Async tasks during event handlers.

Open debt is tracked in [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md).

## Out of scope (by design)

- **Subpixel anything.** Terminal cells are integer-aligned, monospaced. No subpixel positioning, no fractional widths, no anti-aliasing.
- **CSS with no meaning on a character grid.** Fonts (`font-family` and `font-size` parse and do nothing; `@font-face` is dropped), images, pixel and font-relative units (`px`, `em`) as geometry (they only select: a border's weight, a breakpoint, a column count), rotation, scaling and 3D transforms (they parse and draw nothing), print and paged media. These are dropped with a warning (or, where noted, kept and inert); the full list is in [`specs/DIVERGENCES.md`](specs/DIVERGENCES.md) §1. (An at-rule rdom does not evaluate is consumed whole with `WarningKind::UnsupportedAtRule`; `@media`, `@supports` and `@container` are evaluated.)
- **Touch, IME / composition, drag-and-drop, long-press gestures.** Web-platform features tied to input devices or interaction models that don't map onto a terminal.
- **Higher-level component libraries.** The substrate ships native HTML elements and zero opinionated components — same shape as the browser. Component libraries that compose those primitives belong in downstream consumer crates, not in this workspace. See [`CLAUDE.md`](CLAUDE.md) §"Substrate First, Backend Second" for the rationale.

## Examples

```bash
# Self-contained programs (each file is the whole example):
cargo run -p rdom-tui --example counter_button          # button + state
cargo run -p rdom-tui --example tab_form                # focus navigation + form controls
cargo run -p rdom-tui --example parse_and_render        # rdom-parser + rdom-css + rdom-tui
# Showcase demos, standalone:
cargo run -p rdom-showcase                              # the whole tour
cargo run -p rdom-showcase --example scrollable_list    # overflow + wheel scrolling
cargo run -p rdom-showcase --example selectable_text    # text selection + clipboard
cargo run -p rdom-showcase --example tree_nav           # ARIA tree: guides, keyboard nav, lazy load
cargo run -p rdom-showcase --example border_collapse_demo  # border-collapse junctions
cargo run -p rdom-showcase --example sticky_demo        # position: sticky in a scroll container
cargo run -p rdom-showcase --example dom_api_demo       # form-edit / tree-walk / cssom (prints, no TUI)
cargo run -p rdom-showcase --example ua_chrome          # naked HTML built-ins with UA defaults
```

## Design docs

- [`specs/DESIGN.md`](specs/DESIGN.md) — architectural overview: crate map, non-negotiable invariants, roadmap.
- [`specs/DIVERGENCES.md`](specs/DIVERGENCES.md) — every deliberate departure from the web platform.
- [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md) — open debt + accepted simplifications.

Detailed behavior lives in the code: each module has a top-level doc comment, and tests document the contracts. The web specs (WHATWG DOM, CSS, UI Events) are the reference; rdom tracks them within the supported subset.

## Testing

```bash
cargo test --workspace                                  # all unit + integration tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

CI runs the same gates on `[ubuntu-latest, macos-latest, windows-latest]` for every push and PR against `main`.

## License

MIT — see [LICENSE](LICENSE).
