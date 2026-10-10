# rdom-tui

Terminal rendering + runtime for [`rdom-core`](../rdom-core/). The
"how should this render to a terminal, and how do events reach my
handlers" half of the DOM — flexbox and grid layout, CSS-faithful cascade
(specificity, `!important`, `:hover` / `:focus`, `::before` /
`::after`, `var(--…)`, `::selection`), an event loop, hit testing,
mouse + keyboard routing, focus navigation, pointer capture, text
selection, clipboard.

The pure DOM tree lives in `rdom-core`. This crate parameterises
`Dom<Ext>` with `TuiExt` and layers on everything presentational
and interactive.

```rust,no_run
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dom: TuiDom = TuiDom::new();
    // ... build the tree ...
    let sheet = Stylesheet::new().rule(".hero", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))?;

    App::new(dom, sheet)?.run()?; // blocks, returns on Ctrl-C / quit
    Ok(())
}
```

See [`examples/`](examples/) for three self-contained programs; the in-tree `rdom-showcase` crate tours every primitive.

## Contents

- [Quick start](#quick-start)
- [Stylesheets](#stylesheets)
- [The document root and a full-screen app](#the-document-root-and-a-full-screen-app)
- [CSS at a glance](#css-at-a-glance) — every feature area, linked to its recipe in [`docs/RECIPES.md`](docs/RECIPES.md)
- [Runtime](#runtime--app-event-loop-hit-test-routing) and its [configuration](#configuration)
- [Terminal notes](#terminal-notes), [features](#features), [examples](#examples)
- [Upgrading from 0.5](#upgrading-from-05)

## Quick start

```toml
[dependencies]
rdom-tui    = "0.6"
rdom-css    = "0.6"   # optional: parse CSS text
rdom-parser = "0.6"   # optional: HTML-ish template strings
```

`rdom-core` and `rdom-style` come in through `rdom-tui`, which re-exports what an app names; `use rdom_tui::prelude::*;` brings in the common set. Build a tree, cascade a sheet, read the result:

```rust
use rdom_tui::prelude::*;

let mut dom: TuiDom = TuiDom::new();
let root = dom.root();

// Build the tree.
let hero = dom.create_element("div");
dom.node_mut(hero).add_class("hero").unwrap();
dom.append_child(root, hero).unwrap();

// Author some rules.
let red = Color::Rgb(255, 0, 0);
let sheet = Stylesheet::new()
    .rule(".hero", TuiStyle::new()
        .fg(red)
        .padding(Padding::all(1))
        .border(Border::single()))
    .unwrap();

// Run the cascade.
dom.cascade(&sheet);

// Read the final values.
let c = dom.node(hero).computed().unwrap();
assert_eq!(c.fg, red);
assert_eq!(c.padding, Padding::all(1));
assert_eq!(c.border, Border::single());
```

## Stylesheets

Build rules with the fluent API. Selector errors surface at
stylesheet-build time — not at render time.

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), StyleError> {
    let sheet = Stylesheet::new()
        .rule("#nav", TuiStyle::new().width(Size::Fixed(24)))?
        .rule(".row:hover", TuiStyle::new().bg(Color::Rgb(64, 64, 64)))?
        .rule("input:focus", TuiStyle::new().border_fg(Color::Rgb(0, 0, 255)))?
        .rule("tree-item::before", TuiStyle::new().content(Content::Str("▾ ".into())))?
        .rule(":not(.active) a", TuiStyle::new().italic(true))?;
    Ok(())
}
```

Selector grammar is `rdom_core::selectors`' (Selectors 4): type,
universal (`*`), `#id`, `.class`, the attribute selectors with their case
flags, compounds, the four combinators and the column combinator (`||`),
`:is()` / `:where()` / `:not()` / `:has()`, the structural
pseudo-classes (`:nth-child(An+B of S)` and its family, `:empty`,
`:root`, …) and the interaction and form states (`:hover`,
`:focus-visible`, `:checked`, `:user-invalid`, …). Pseudo-elements
(`::before`, `::after`, `::marker`, `::selection`, …) are split from the
selector suffix before rdom-core parsing. The full list is in the
[`rdom-css` README](../rdom-css/README.md#supported-grammar).

Specificity is spec-faithful: `(inline, id, class+attr+pseudo_class,
type+pseudo_element)` compared lexicographically. Same-specificity
ties break on source order. `!important` inverts origin priority, so
a UA `!important` rule beats an author `!important` rule.

`Stylesheet::new()` bakes in UA defaults (`:disabled { color: <muted>; user-select: none }`, …).
`Stylesheet::bare()` skips them for tests.

`display: flex` lays its items out in a row: `flex-direction`'s initial value is `row` (CSS Flexbox §5.1; rdom 0.5 and earlier defaulted to `column`), so a container meant to stack its items needs `flex-direction: column` (`.flex_column()`). `flex-direction` applies to flex containers only: a block container stacks its children on its block axis whatever it says.

Boxes size as `box-sizing: content-box`, the CSS initial value: `width`, `height` and `min-*` / `max-*` measure the content box, padding and border lie outside it (form controls are `border-box` in the UA sheet); start a sheet with `*, ::before, ::after { box-sizing: border-box }` to size every box by its border.

## The document root and a full-screen app

The document root (`dom.root()`, a fragment) is the initial containing block, the viewport's size (CSS 2.1 §10.1), and lays its children out as a browser lays out `<body>`'s: block boxes as tall as their content, their margins collapsing, floats floating, inline elements and text in lines. So a top-level element fills the screen as it would on a web page — `height: 100%`, a percentage of the viewport — and an app shell is a flex column inside it, its panes `flex: 1`. (rdom 0.5 laid the root's children out in a viewport flex column, where `flex: 1` alone filled it.)

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        ".app  { height: 100%; display: flex; flex-direction: column }
         .main { flex: 1 }",
    )?;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(
        &mut dom,
        r#"<div class="app"><header>title</header><main class="main">body</main><footer id="f">status</footer></div>"#,
        root,
    )?;
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 12));
    // The footer sits on the last row: the shell fills the viewport and
    // the main pane takes what the header and footer leave.
    let footer = dom.get_element_by_id("f").unwrap();
    assert_eq!(dom.node(footer).layout_rect().unwrap().y, 11);
    Ok(())
}
```

Give a padded or bordered shell `box-sizing: border-box`, so its `100%` includes them: block flow does not shrink a box to fit the viewport.

The root is also the root element: `:root { color: …; background-color: … }` styles it, every top-level element inherits from it, and its background fills the whole screen (CSS Backgrounds 3 §2.11.2). Under a transparent root, a parsed `<html>` / `<body>` tree's background propagates as in a browser; any other top-level element paints only its own box. `:root` custom properties are ordinary declarations of the root, so one inside `@media (prefers-color-scheme: dark)` follows the query.

## CSS at a glance

rdom-tui implements the CSS that means something on a character grid, following each specification; what it cannot express is approximated in whole cells or kept inert, and every such departure is listed in [`DIVERGENCES.md`](../../specs/DIVERGENCES.md). Property by property, [`CSS-COVERAGE.md`](../../specs/CSS-COVERAGE.md) records the status. The recipes are in [`docs/RECIPES.md`](docs/RECIPES.md), each a doctested program.

| Area | What works | Recipe | Terminal notes |
|---|---|---|---|
| Cascade and selectors | Origins, `!important`, `@layer`, `@scope`, nesting, `revert` / `revert-layer`; Selectors 4 with `:has()`, `:is()` / `:where()` / `:not()`, `:nth-child(… of S)`, the interaction and form states | [Stylesheets](#stylesheets), [the `!important` ladder](docs/RECIPES.md#important-ladder), [incremental re-cascade](docs/RECIPES.md#incremental-re-cascade) | `:visited` never matches |
| Custom properties | `var()` in every property, `@property`, `attr()` with a type | [Custom properties](docs/RECIPES.md#custom-properties-and-var) | — |
| Values and units | Cells, `%`, `fr`, `ch`, `lh` / `rlh`, viewport and container units, `calc()` and the math functions | — | `px` / `em` never become geometry; they select border weights and breakpoints (8px a column, 16px a row) |
| Colour | CSS Color 4 / 5, alpha composited over what lies beneath, `color-mix()`, relative colours, system colours, `light-dark()` | — | 24-bit colour, emitted as the nearest of the 256- or 16-colour palette (or none, `NO_COLOR`) at the depth detected from `COLORTERM` / `TERM` (`App::with_color_depth` overrides); the light or dark scheme follows the terminal's background |
| Box model and sizing | `content-box` / `border-box`, `min-*` / `max-*`, the intrinsic keywords, `stretch`, `calc-size()`, `aspect-ratio`, logical properties | [A full-screen app](#the-document-root-and-a-full-screen-app) | Whole cells; a border is a row or column of glyphs |
| Flexbox and alignment | Flex containers, Box Alignment, `gap`, `order` | [A full-screen app](#the-document-root-and-a-full-screen-app) | — |
| Grid | Track sizing, named lines and areas, auto-placement, `subgrid` | [Grid layout](docs/RECIPES.md#grid-layout) | — |
| Tables | The table formatting context, automatic and `fixed` layout, spans, captions, separated and collapsed borders | [Tables](docs/RECIPES.md#tables) | Collapsed borders join into box-drawing junctions |
| Positioning, floats, overflow, scrolling | `relative` / `absolute` / `fixed` / `sticky`, `z-index`, floats and clearance, `overflow`, `text-overflow`, `line-clamp`, scrollbars, scroll snapping | [Floats and text overflow](docs/RECIPES.md#floats-and-text-overflow) | — |
| Inline text and decoration | Line breaking, `white-space`, `text-align`, `line-height` in rows, `vertical-align`, decorations | [Inline formatting](docs/RECIPES.md#inline-formatting) | Underline styles and colours where the terminal draws them (`SgrCapabilities`); fonts and sizes are inert |
| Lists, counters, generated content | `::marker`, counter styles, `@counter-style`, `content`, quotes | [Pseudo-elements](docs/RECIPES.md#pseudo-elements-and-content), [Lists and counters](docs/RECIPES.md#lists-counters-and-generated-content) | — |
| Conditional rules and containment | `@media` against the terminal, `@supports`, `@container` size and style queries, `contain`, `content-visibility` | [Responsive layout](docs/RECIPES.md#responsive-layout-media-and-container-queries) | Sizes in cells; `prefers-color-scheme` from the terminal |
| Transitions and animations | Transitions, `@keyframes`, `@starting-style`, scroll-driven animations | [Transitions and animations](docs/RECIPES.md#transitions-and-animations) | Frames at up to 60 fps, on the `App`'s clock |
| Forms and interaction | `:hover` / `:active` / `:focus-visible`, validity states, `appearance`, `outline`, `cursor`, `caret-*`, `::placeholder`, custom highlights | [Interaction state](docs/RECIPES.md#interaction-state-hover-active-and-focus), [Form states](docs/RECIPES.md#form-states), [Custom highlights](docs/RECIPES.md#custom-highlights-search-results) | A text field has no UA border; the pointer shape needs OSC 22 |
| Popovers, the top layer, anchor positioning | `popover`, modal `<dialog>`, `::backdrop`, `anchor()`, `position-area`, `@position-try` fallbacks | [Popovers and the top layer](docs/RECIPES.md#popovers-and-the-top-layer) | — |
| Multi-column layout | `columns`, rules, spanners, balancing, fragmentation | [Multi-column layout](docs/RECIPES.md#multi-column-layout) | Whole-cell columns |
| Transforms, filters, blending, clipping | `translate` / `transform`, `filter`, `backdrop-filter`, `mix-blend-mode`, `isolation`, `clip-path`, `clip` | [Effects](docs/RECIPES.md#transforms-filters-blending-and-clipping) | Translation in whole cells; rotation, scaling and `blur()` are inert |

## Runtime — `App`, event loop, hit test, routing

`App::run` is the app-facing entry point. It wraps the DOM in an
event loop (rdom's own terminal input reader on Unix, crossterm's
elsewhere) that runs the "rendering steps" model
borrowed from the HTML spec: drain events, tick, run
`requestAnimationFrame` callbacks, cascade + layout + paint when
dirty, then sleep on the next event. One paint per task-end, no
matter how many mutations happen inside a single handler.

```rust,no_run
use std::time::Duration;

use rdom_tui::prelude::*;

fn main() -> std::io::Result<()> {
    let (dom, sheet) = (TuiDom::new(), Stylesheet::new());
    App::new(dom, sheet)?
        .with_tick_rate(Duration::from_millis(50))
        .with_tick_handler(|_ctx| {
            // drain background channels, mutate DOM, request redraw
            ControlFlow::Continue
        })
        .run()
}
```

### Configuration

Options set once, when the `App` is built, are consuming `with_*`
builders chained after either constructor; the `&mut self` `set_*`
methods are for what an app changes while it runs. `App::new` and
`App::with_backend` differ only where a test must not depend on the
environment or the clock:

| Option | Builder | `App::new` | `App::with_backend` |
|---|---|---|---|
| Event-poll timeout / tick cadence | `with_tick_rate` | 50 ms | 50 ms |
| Tick handler | `with_tick_handler` | none | none |
| Frame rate while anything animates | `with_animation_frame_rate` | 60 fps | 60 fps |
| Caret blink half-period | `with_caret_blink` | 530 ms | steady (`None`) |
| SGR extensions emitted | `with_sgr_capabilities` | from the environment (`SgrCapabilities::from_env`) | the backend's (a `TestBackend`'s `BASIC`) |
| Pointer-shape protocol | `with_pointer_shapes` | from the environment (`PointerShapes::from_env`) | `None` |
| Preferred color scheme | `with_color_scheme`; at run time `set_color_scheme` | asked of the terminal when `run` starts (OSC 11), dark without an answer | dark |
| Media preferences (`prefers-reduced-motion`, `prefers-contrast`, the pointer, …) | `with_media_preferences`; at run time `set_media_preferences` | no preference, a mouse, 24-bit color | the same |
| Clipboard | `with_clipboard` | the system clipboard | the system clipboard |
| `<a href>` URL opener | `with_url_opener` | the system opener | the system opener |
| `@import` loader for `<style>` sheets | `with_import_loader` | none (imports unresolved) | none |

At run time: `set_color_scheme`, `set_media_preferences`, `match_media` (a live `MediaQueryList` with change listeners), the stylesheet stack
(`push_stylesheet`, `set_stylesheet`, `remove_stylesheet`) and
`register_property`.

What the runtime gives you, roughly in order of the `RDOM_RUNTIME`
phases:

- **Hit testing** — `HitTestExt::hit_test(x, y) → Option<NodeId>`
  and `hit_test_path(x, y) → Vec<NodeId>`. Handles overflow
  clipping + IFC fragment lookup + paint-order stacking.
- **Mouse routing** — `mousedown`, `mouseup`, `mousemove`, `click`
  synthesized on the nearest common ancestor of down + up targets
  (matches HTML), auto-`mouseover` / `mouseout` on transitions,
  `:hover` cascade re-runs via `InteractionChanged` mutations.
- **Wheel scrolling** — walks up from the hit target for the nearest
  `overflow: Scroll | Auto` ancestor that can still move in that
  direction (one at its rail end chains to the next). Cancelable via
  `prevent_default` on `wheel`.
- **Focus navigation** — `tabindex` attribute parsing, `Tab` /
  `Shift-Tab` cycles focusable elements (positive indices first,
  then DOM order), focus-on-click walks up to nearest focusable.
  `focus` / `blur` (non-bubbling) + `focusin` / `focusout`
  (bubbling). `:focus` cascade responds.
- **Pointer capture** — `dom.set_pointer_capture(id)` /
  `release_pointer_capture()`. While captured, `mousemove` /
  `mouseup` route to the captured element regardless of hit;
  auto-released on `mouseup`.
- **Text selection** — `Dom::selection()` / `set_selection()` with
  a browser-faithful `Selection { anchor, focus }` model. Mouse
  drag, `Shift+arrow` (grapheme), `Shift+Ctrl+arrow` (word),
  `Ctrl-A` (select-all within focused element), double-click
  (word), triple-click (line). `user-select: { Auto, Text, None,
  All, Contain }` gates selectability; it is not inherited (CSS UI 4
  §6.1) — the runtime resolves `auto`'s used value from the parent.
  The selection drag takes no pointer capture: `mousemove` /
  `mouseup` keep targeting the element under the pointer and `click`
  goes to the common ancestor, as in browsers. `::selection`
  pseudo-element paints selected cells with a reversed-fg/bg overlay.
- **Clipboard** — Ctrl-C / Ctrl-X / Ctrl-V dispatch `copy` /
  `cut` / `paste` events (cancelable). Default action writes the
  serialized selection to the system clipboard via `arboard`;
  tests inject `MemoryClipboard` via `App::with_clipboard`.
- **Panic safety** — a process-wide panic hook runs
  `leave_tui_mode` before the default hook prints, so the panic
  message lands on the main screen instead of the cleared
  alt-screen. `App::run` also wraps the loop in `catch_unwind`.

Listeners that want the key payload or mouse payload for the
currently-dispatching event read it as typed detail off the core
`Event`: `ctx.event.detail.as_keyboard()` returns
`Option<&KeyboardDetail>` (DOM-faithful `key: String`, four-bool
modifiers, `repeat`); `ctx.event.detail.as_mouse()` returns
`Option<&MouseDetail>` (button + buttons bitmask + client_x/y +
wheel deltas + modifiers). Translation from crossterm's `KeyEvent`
/ `MouseEvent` lives in `tui_event::key_translate` and runs inside
the `TuiEvent::keydown` / `keyup` / `keypress` / `click` / mouse /
`wheel` builders.

## Terminal notes

- **Startup color-scheme query.** Unless the app sets a scheme (`App::with_color_scheme`), `App::run` asks the terminal for its background with OSC 11 before the first frame (Unix, when stdout is a terminal; replies are read from stdin, or `/dev/tty` when stdin is redirected), and prefers the light or dark scheme it calls for — what `light-dark()` and `color-scheme: normal` follow. A terminal that answers ends the wait at once; one that answers nothing costs 200 ms; a reply that has begun is waited for up to 800 ms more. Keys typed during the wait are kept and handled after it, and a reply that arrives later is still taken as the answer. `App::detected_background()` is the reported color, `None` when there was no answer.
- **Theme changes.** `App::run` enables DEC mode 2031 on Unix: a terminal that supports it (Contour, Ghostty, kitty, and others) reports a switch between its light and dark themes, and the preferred scheme follows, restyling the tree — unless the app set the scheme. On Windows neither the startup query nor theme reports are read (crossterm's console reader delivers no terminal replies), so the scheme is dark unless the app sets it.
- **Input.** On Unix rdom reads the terminal itself (its own reader, inside `App::run`), parsing keys (including the kitty keyboard protocol), SGR mouse, paste, focus and resize into crossterm's `Event` types; an unknown escape sequence is consumed instead of stalling the input after it, a lone `ESC` is the Esc key after 25 ms without a following byte, and a bracketed paste is capped at 1 MiB, Ctrl+C ending one that never ends.
- **Text decorations and `SgrCapabilities`.** An underline's style (`4:2`–`4:5`) and color (`58:2::r:g:b`) and the overline (`53`) are SGR extensions a terminal that does not know them may misread (`4:3` as underline plus italic), so a backend emits only those its `SgrCapabilities` list; elsewhere an underline is a plain `4`. `App::new` guesses them from the environment (`SgrCapabilities::from_env`): all three for kitty, WezTerm, foot, Ghostty, mintty, VTE 0.60+ and tmux 3.2+ (which passes each on where its outer terminal's terminfo has it: `Smulx`, `Setulc`, `Smol`); the underline styles and color for Alacritty and VS Code; the styles alone for iTerm2 3.4+; none under GNU screen or an older tmux — a multiplexer wins over the outer terminal's inherited variables — nor for anything else, Windows Terminal included (its `WT_SESSION` carries no version). Override the guess on the `App`:

  ```rust
  use rdom_tui::prelude::*;
  use rdom_tui::SgrCapabilities;

  let terminal = Terminal::new(TestBackend::new(20, 2)).unwrap();
  let app = App::with_backend(TuiDom::new(), Stylesheet::new(), terminal)
      .unwrap()
      // Windows Terminal 1.20+: every extension.
      .with_sgr_capabilities(SgrCapabilities::EXTENDED);
  assert_eq!(app.sgr_capabilities(), SgrCapabilities::EXTENDED);
  // Curly underlines and nothing else; or `BASIC` for a log or recording.
  let _custom = SgrCapabilities::BASIC.with_styled_underline(true);
  ```

- **The pointer's shape and `PointerShapes`.** `cursor` (CSS UI 4 §4.1) sets the terminal pointer over the element under it — the text pointer over selectable text, `pointer` over a link — sent as OSC 22 to the terminals known to take CSS pointer names: kitty, foot, WezTerm and Ghostty (`PointerShapes::from_env`; nothing under a multiplexer or elsewhere). Leaving TUI mode, the panic path included, puts the default pointer back. Override the guess on the `App`:

  ```rust
  use rdom_tui::prelude::*;
  use rdom_tui::PointerShapes;

  let terminal = Terminal::new(TestBackend::new(20, 2)).unwrap();
  let app = App::with_backend(TuiDom::new(), Stylesheet::new(), terminal)
      .unwrap()
      .with_pointer_shapes(PointerShapes::Osc22);
  assert_eq!(app.pointer_shapes(), PointerShapes::Osc22);
  ```

- **iTerm2 and hover.** iTerm2 may ignore the any-motion mouse mode until it sees a real click, at launch and after every refocus, so `:hover` styles start following the pointer only after one click. Other terminals (Alacritty, Kitty, Ghostty, WezTerm) honor it immediately. The cause and the failed re-arm attempts are recorded in `specs/DIVERGENCES.md` §4.

## Features

- `test-util` — exposes `VirtualScreen` (`rdom_tui::VirtualScreen`, also in the prelude), a headless VT emulator that replays the ANSI bytes a backend emits into an inspectable cell grid, for asserting what a real terminal would show after a sequence of frames, resizes and clears. It is for test suites: enable it on the dev-dependency (`rdom-tui = { version = "…", features = ["test-util"] }`), not on the normal one. It is not part of the default API, and docs.rs builds with it so the type is documented.
- `no-synchronized-output` — skips the BSU / ESU synchronized-output sequences (DEC private mode 2026) for terminals that mishandle them. Off by default; terminals without support ignore the sequences.

## Architecture

- `rdom-core` stays style-agnostic. No `Color`, no `Stylesheet`, no
  `TuiStyle` in the core crate.
- `ComputedStyle` on `TuiExt` is the *only* post-cascade source of
  truth. Layout and paint should read `node.computed()`, never
  `inline_style` directly.
- Which properties inherit is declared once, in
  `rdom_style::property_dispatch::inherits`; the cascade's
  `inherit_inheritable_from` is pinned to it by a test that probes
  every property.
- Selector matching goes through `rdom_core::Dom::matches_list` for each
  candidate rule from the sheet's rightmost-selector index, once
  per rule per element. There is one matching engine.
- `MutationObserver` is the invalidation mechanism. `DirtyTracker` is
  *one* observer; future devtools / a11y mirrors / reactive bindings
  can register their own without touching the cascade code.
- Bounded against hostile values and hostile depth: no CSS, markup or
  DOM can overflow the stack. The box tree stops at
  `rdom_tui::MAX_LAYOUT_DEPTH` (128) — the element that deep skips its
  contents, which are not styled, laid out, painted or hit — so every
  pass that recurses over it is bounded, and every walk over the whole
  DOM is iterative. The parsers cap their own nesting (rdom-css's
  `MAX_BLOCK_DEPTH`, rdom-core's `MAX_SELECTOR_NESTING`, rdom-parser's
  `MAX_TREE_DEPTH`); `specs/DIVERGENCES.md` §2 lists every cap.

## Examples

```text
cargo run -p rdom-tui --example counter_button    # click listener + text-node mutation
cargo run -p rdom-tui --example tab_form          # native form controls, focus navigation, submit
cargo run -p rdom-tui --example parse_and_render  # rdom-parser + rdom-css + rdom-tui composing
```

Each file is the whole program and uses `App::run`. No manual event
loops, no direct `enter_tui_mode` in user code. Ten more demos
(scrollable list, text selection, ARIA tree, sticky, border collapse,
UA chrome, …) live in the workspace's `rdom-showcase` crate:
`cargo run -p rdom-showcase` for the tour, or
`cargo run -p rdom-showcase --example <name>` for one.

## Benchmarks

```text
cargo bench -p rdom-tui --bench runtime
```

Measures: hit-test on a 10k-node tree, dispatch depth-50 (full
capture + bubble), full-frame cascade + layout + paint at 80×24
and 200×60, range serialization on a 10k-cell selection,
scroll-list steady-state mutation (1k / 10k rows), Unicode
paragraph wrap (10k graphemes, CJK + emoji). Results comparable
to ratatui's criterion widget benches.

## Upgrading from 0.5

0.6 makes the CSS behave as the web's: `display: flex` is a row, boxes are `content-box`, the document root lays out in block flow, HTML tables are CSS tables, and the properties 0.5 dropped as unknown take effect. [`UPGRADING-0.6.md`](../../UPGRADING-0.6.md) has the fifteen changes most apps hit with their fixes, an index by symptom, every behaviour change by area, and the compile breaks.

## Further reading

- [`DESIGN.md`](../../specs/DESIGN.md) — architectural overview: crate map, non-negotiable invariants, roadmap.
- [`docs/RECIPES.md`](docs/RECIPES.md) — the worked examples, doctested.
- [`DIVERGENCES.md`](../../specs/DIVERGENCES.md) — every deliberate departure from the web platform.
- [`CSS-COVERAGE.md`](../../specs/CSS-COVERAGE.md) — every CSS property and its status.
- Module docs under `src/style/`, `src/render/`, and `src/runtime/` — each submodule has a top-level doc comment with the specific role.
