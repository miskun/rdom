# rdom

A DOM for terminal applications, in Rust.

`rdom` brings the architecture of the browser DOM — arena-backed nodes, CSS-style cascade, flexbox layout, capture/bubble events, mutation observers, selection ranges — to text-mode UIs. It targets terminals (via `crossterm`) but the core tree is renderer-agnostic and can drive headless or alternate backends.

The browser DOM is the reference model: native HTML elements, CSS-faithful cascade, web-platform event semantics. Higher-level component libraries live in downstream projects, not in this repo.

## Quick start

Install:

```toml
[dependencies]
rdom-tui    = "0.5"
rdom-parser = "0.5"   # optional: HTML-ish template strings
rdom-css    = "0.5"   # optional: parse real CSS at runtime
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

See [`crates/rdom-tui/examples/`](crates/rdom-tui/examples/) for three self-contained programs (counter button, tab form, parse + render) and [`crates/rdom-showcase/examples/`](crates/rdom-showcase/examples/) for the ten showcase demos runnable standalone: scrollable lists, text selection, an ARIA tree with lazy children, sticky headers, border collapse, and the naked-UA chrome tour.

## Crates

| Crate | What it is |
|---|---|
| [`rdom-core`](crates/rdom-core) | Pure DOM. Arena, `NodeId`, attributes, classes, tree mutation, CSS selectors, 3-phase event dispatch, `MutationObserver`, `AbortSignal`, `Selection`/`Range`/`Position`. Zero rendering deps. |
| [`rdom-style`](crates/rdom-style) | CSS data model + property dispatch + value parsers. Leaf crate; consumed by `rdom-css` (the parser) and `rdom-tui` (the renderer). |
| [`rdom-css`](crates/rdom-css) | CSS parser. Tokenizer + block parser + `<style>`-tag extraction + inline-style seeding. Produces `Stylesheet` / `TuiStyle` via `rdom-style`'s property dispatch. |
| [`rdom-tui`](crates/rdom-tui) | Terminal backend. CSS cascade, flexbox layout, paint pass, ANSI emission, inline formatting (word wrap, CJK breaks, `<br>`, `white-space`), runtime (event loop, hit test, keyboard/mouse routing, focus, text selection + clipboard), native HTML element behaviors (`<button>`, `<input>` family, `<select>`, `<form>`, `<details>`, `<dialog>`, `<progress>`, `<meter>`, `<table>` family, `<canvas>`). |
| [`rdom-parser`](crates/rdom-parser) | HTML-ish template parser → `Dom<Ext>`. `parseFromString` equivalent. Hand-rolled, no external parser deps. |

## What's in 0.5.0

A stabilization release ([`specs/STABILIZE-2026-09.md`](specs/STABILIZE-2026-09.md)): every open row in the tech-debt ledger was fixed, moved to `DIVERGENCES.md` as a deliberate departure, or documented as an external limitation, so 0.5.0 ships with no open debt. On top of that, these web-platform features:

- **Form validation** (HTML §4.10.20): `required`, `pattern`, length / range / step and type checks, `ValidityState`, `check_validity` / `report_validity` / `set_custom_validity`, the `invalid` event, `:valid` / `:invalid` / `:required` / `:optional`; submission is blocked unless `novalidate` / `formnovalidate`.
- **`fieldset[disabled]`**: disables its controls outside the first `<legend>`; `:disabled` / `:enabled`.
- **Form owner and submitter overrides**: `form="id"`, `formaction` / `formmethod` / `formenctype` / `formtarget` / `formnovalidate`, `<input type=image>`.
- **`:focus-visible`**: the UA focus tint follows the browsers' heuristics (keyboard focus and text fields, not a clicked button).
- **`:active`**, and `:hover` on the ancestors of the hovered element.
- **`::placeholder`** styles a shown placeholder.
- **Smooth scrolling**: `scroll-behavior: smooth` and `ScrollToOptions` / `ScrollIntoViewOptions` with `block` / `inline`.
- **Live `<style>`**: the `App` applies the document's `<style>` elements and re-parses them when they change.
- **Caret blink** at 530 ms under `App::new` (`App::with_caret_blink`).
- **Blink-model undo**: typing, Backspace and Delete runs are one undo step each; `inputType`s per Input Events.

**Breaking** changes in every crate. See [`CHANGELOG.md`](CHANGELOG.md) for the full 0.5.0 notes; each `Breaking` bullet ends with what to change.

## What's in 0.4.0

A hardening release. The whole workspace was reviewed, every documented divergence from the web platform was re-audited (kept, followed, or deleted), and the tech-debt ledger was resolved or re-justified row by row. Highlights, with the web spec each one follows:

- **Generational `NodeId`.** A freed slot's old id no longer resolves to whatever node reuses the slot (DOM object identity). **Breaking:** `NodeId` is no longer a bare index.
- **Event dispatch per DOM §2.9.** Two-pass target invocation, listener-removal-during-dispatch semantics, and a `Dom::set_activation_hook` so checkbox / radio activation behavior runs even when propagation stops.
- **CSS tokenizer per CSS Syntax 3.** Whole-number tokens (`Token::Float`, fractional `Percentage`), string escapes, at-rule consumption, out-of-range integers rejected instead of wrapped, `calc(x / 0)` rejected at parse time. CSS-wide keywords use the same inheritance table as the cascade.
- **HTML tokenizer per HTML §13.2.** `<style>` / `<script>` are raw text, `<textarea>` / `<title>` are RCDATA, ~100 named character references, `a < b` is text, `<!DOCTYPE>` is skipped, `<?…>` / `<!…>` are bogus comments, and `<style>` round-trips through `outer_markup` without corrupting the CSS.
- **Runtime robustness.** One shared scheduler with a re-entrancy guard on every user-code path, flex freeze loops with a fixed pass budget, scrolled hit-testing through a single line↔row mapping, compound undo for cross-node edits, `pointer-events` inherited, `Rc` computed styles.

See [`CHANGELOG.md`](CHANGELOG.md) for the full 0.4.0 notes and migration guidance, and [`specs/HARDENING-2026-09.md`](specs/HARDENING-2026-09.md) for the program log.

## What's in 0.3.0

A substrate-honesty release driven by the first downstream consumer. Highlights:

- **Geometry node setters drive layout.** `set_width` / `set_direction` / `set_gap` / … now write the cascade input, so they actually affect layout (they previously wrote dead fields and silently no-op'd). **Breaking** — see the changelog.
- **`EventCtx::request_redraw()`.** Event listeners can request a repaint when they mutate state the DOM tracker can't see (e.g. a `<canvas>` reading external app state) — unblocks interactive canvas components.
- **`TuiStyle::flex_row()` / `flex_column()` / `flex()` / `inline_flex()`** convenience builders.
- **`remove_child_dropping` / `clear_children_dropping`** — detach **and** free in one call (no arena-slot leak for high-churn UIs).
- **`RenderContext::for_test`** for unit-testing `<canvas>` paint code downstream; the canvas `RenderContext` is now the canonical crate-root export.

See [`CHANGELOG.md`](CHANGELOG.md) for the full 0.3.0 notes (incl. breaking changes) and [`specs/SUBSTRATE-0.3.0.md`](specs/SUBSTRATE-0.3.0.md) for rationale.

## What's in 0.2.0

0.2.0 adds, on top of the 0.1.0 substrate below:

- **Block formatting context.** Semantic HTML stacks per the web platform with no CSS at all — `<div><h1></h1><p></p></div>` is a block-flow column at intrinsic heights. CSS 2.1 normal flow + margin collapse + height resolution + CSS3 `gap` on blocks + atomic `inline-block` in inline formatting contexts, on top of the original flex pass.
- **Native ARIA tree.** `<ul role=tree>` / `role=treeitem` / `role=group` with `│ ├ └` guides + `▼`/`▶` chevrons, keyboard nav (Arrows / Home / End / Enter / Space) via an `aria-activedescendant` cursor, collapse/expand (`aria-expanded`), lazy children (`aria-busy`), and scroll-into-view that follows the cursor.
- **`calc()` value system.** `width` / `height` / `top` / `right` / `bottom` / `left` and the other length axes — CSS precedence, parentheses, nested `calc()`, banker's rounding onto the cell grid.
- **More events.** `keyup` (kitty keyboard protocol), `contextmenu` (right-click + Shift+F10), `dblclick`, `resize`, `scroll`, plus implicit `blur` / `focusout` / `mouseout` / `mouseleave` dispatched before structural detach.
- **Layered border model.** `border-collapse` is non-inheriting and applies to any container's direct children; per-direction conflict resolution (CSS Tables 3 §11.5); full `border-style` keyword set + the rdom-specific `half-block` pill style.
- **Multi-slot stylesheets.** `push_stylesheet` / `remove_stylesheet` + `cascade_all` to stack and swap author sheets over the UA sheet.

See [`CHANGELOG.md`](CHANGELOG.md) for the full 0.2.0 notes, including breaking changes.

### The 0.1.0 substrate

- **DOM substrate.** Arena, attributes, classes, mutation, CSS selectors (Selectors Level 4 subset), 3-phase event dispatch with `stopPropagation` / `preventDefault` / `AbortSignal`, `MutationObserver`, `Selection` / `Range` / `Position`, serialization (`outer_markup` / `inner_markup`).
- **HTML template parser.** Hand-rolled, no external deps. `parseFromString` equivalent. Round-trippable for the supported subset.
- **`pointer-events: none`, modal `<dialog>` focus trap, scrollable `<textarea>`.** Overlays let clicks through; `showModal()` traps Tab and Esc inside the dialog and returns focus on close; text leaves taller than their box scroll with the caret kept in view.

- **CSS string parser.** Real CSS in, `Stylesheet` out. Three surfaces unified: standalone stylesheets, `<style>` blocks in templates, inline `style="…"`. Selectors, all properties in the dispatch table (color, sizing, padding, border, positioning, transitions), `!important`, custom properties (`var()` in every property, substituted per element), the CSS-wide keywords `inherit` / `initial` / `unset` / `revert` / `revert-layer` and the `all` shorthand, `@layer` cascade layers, CSS Nesting (`&`, nested rules, nested `@layer`), `@scope` (roots, limits, `:scope`, scope proximity), `@import` through a host loader, `@property` (typed, animatable custom properties), comma-separated rules, lenient + strict modes with positioned warnings.
- **Cascade.** UA / author / inline ladder with `!important` inversion, cascade layers (`@layer`) and `revert` / `revert-layer` rollback. CSS-faithful specificity. Interaction pseudo-classes (`:hover`, `:active`, `:focus`, `:focus-visible`, `:checked`, `:indeterminate`, `:open`, `:disabled`, `:enabled`, `:valid`, `:invalid`, `:required`, `:optional`, …). Pseudo-elements (`::before`, `::after`, `::selection`, `::backdrop`, `::placeholder`, `::scrollbar`, `::scrollbar-thumb`). `content` property. Custom properties.
- **Layout + paint.** Flexbox for flex containers. `display: inline-block` for content-hugging chrome (buttons, badges, tags). Inline formatting (word wrap at whitespace + CJK + hyphens, `<br>`, `white-space: normal|pre|nowrap`, per-grapheme source tracking). Positioned `::before` / `::after` pseudo-elements (`position: relative | absolute | fixed` honoring `top` / `right` / `bottom` / `left`). Truecolor / 256-color fallback. ANSI emission with synchronized output (DEC 2026).
- **Runtime.** Event loop with rendering-steps model (drain, tick, rAF, cascade + layout + paint, sleep). Hit testing, mouse routing (`mousedown` / `mouseup` / `click` synthesized on nearest common ancestor — matches HTML), keyboard routing, focus navigation (`tabindex`, `Tab` / `Shift-Tab`, autofocus), pointer capture, text selection (mouse drag, `Shift+arrow` including vertical with sticky-x and line-edge via `Shift+Home`/`End`, `Ctrl-A`, double/triple-click, `user-select: none|all|contain`) + system clipboard (`arboard`, OSC 52 fallback), panic safety (terminal state restored on panic).
- **Native HTML built-ins.** `<button>`, `<label>`, `<details>` / `<summary>`, `<input>` family (text, password, number, checkbox, radio, range, submit, button, reset, hidden, color, search, email, tel, url), `<textarea>`, `<select>` / `<option>`, `<form>`, `<dialog>`, `<progress>`, `<meter>`, `<table>` family + column-width sync, `<canvas>` + `RenderContext` escape hatch, `<a href>` with scheme dispatch. Constraint validation: `required`, `pattern`, `minlength` / `maxlength`, `min` / `max` / `step`, email / url type checks, `ValidityState`, `check_validity` / `report_validity` / `set_custom_validity`, the `invalid` event, submission blocked unless `novalidate` / `formnovalidate`, and `:valid` / `:invalid` / `:required` / `:optional`. Editable surfaces honor `caret-color` (cell bg) and the rdom-extension `caret-text-color` (glyph fg); both default to inverting the cell's cascaded fg/bg. `readonly` fires cancelable `beforeinput` (matches UI Events L2 §5). `contenteditable` supports cross-text-node edits across inline boundaries.
- **User-agent stylesheet.** 150 UA rules ship visual chrome on every native element so naked HTML looks attractive out of the box. Bracketed `[ Label ]` buttons in accent fg. Rounded LightBlue-bordered modal dialogs. `▸`/`▾` disclosure triangles. `•` list bullets, numbered `<ol>` via CSS counters. `│` blockquote rail, `─` `<hr>` rule, `▾` `<select>` chevron. Subtle background-tint `:focus-visible` indicator (shown for keyboard focus and text fields, not for a mouse-clicked button) (color-only — no reverse-video, no glyph shift). The text-control tint is `!important`, so it beats the text-field background, and authors override it with their own `!important` rule; the button / summary / link tint is a normal rule. Run `cargo run -p rdom-showcase --example ua_chrome` to see it.
- **DOM API completeness.** Per-tag accessors (`input_value`, `select_options`, `details_open`, `form_elements`, …), CSSOM (`style.set_property`, `style_declaration`, camelCase aliases), scroll APIs (`scroll_top` / `scroll_into_view`), document hit-testing (`element_from_point`), `bounding_rect`, focus/blur/click programmatic dispatch.
- **Positioning.** `position: {static, relative, absolute, fixed, sticky}`, `z-index` with nested stacking contexts (CSS 2.1 Appendix E), static positions, `top` / `right` / `bottom` / `left`, `inset` shorthand.
- **Timers + transitions.** `setTimeout` / `setInterval`, `requestAnimationFrame` with `DOMHighResTimeStamp`, CSS `transition` with the keyword timing functions, `cubic-bezier()` and `steps()`.
- **Terminal niceties.** OSC 52 clipboard fallback, OSC 8 hyperlinks for `<a href>`, truecolor + 256-color fallback, integer-cell grid, monospaced advance.

## Roadmap

- **0.6.0** — CSS completeness: every CSS feature that means something in a terminal ([`specs/CSS-COMPLETE-2026-10.md`](specs/CSS-COMPLETE-2026-10.md)). What is still missing today is listed in [`specs/DIVERGENCES.md`](specs/DIVERGENCES.md) §3.
- **0.7.0** — Client-side routing primitive.
- **0.8.0** — Async tasks during event handlers.

Open debt is tracked in [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md).

## Out of scope (by design)

- **Subpixel anything.** Terminal cells are integer-aligned, monospaced. No subpixel positioning, no fractional widths, no anti-aliasing.
- **CSS with no meaning on a character grid.** Fonts (`font-family`, `font-size`, `@font-face`), images, pixel and font-relative units (`px`, `em`), rotation and 3D transforms, print and paged media. These are dropped with a warning; the full list is in [`specs/DIVERGENCES.md`](specs/DIVERGENCES.md) §1. (Every at-rule is consumed whole with `WarningKind::UnsupportedAtRule` today; `@media`, `@supports`, `@keyframes` and the other applicable ones are scheduled for 0.6.0.)
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
