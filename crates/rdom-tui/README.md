# rdom-tui

Terminal rendering + runtime for [`rdom-core`](../rdom-core/). The
"how should this render to a terminal, and how do events reach my
handlers" half of the DOM — flexbox layout, CSS-faithful cascade
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

## Quick start

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

Selector grammar is the same as `rdom_core::selectors`: type, universal
(`*`), `#id`, `.class`, `[attr]`, `[attr=value]`, `[attr~=value]`,
`[attr|=value]`, `[attr^=value]`, `[attr$=value]`, `[attr*=value]`,
compounds, combinators (` `, `>`, `+`, `~`), `:not(...)`, and
pseudo-classes (`:first-child`, `:last-child`, `:only-child`, `:empty`,
`:root`, `:hover`, `:focus`). Pseudo-elements `::before` / `::after`
are stripped from the selector suffix before rdom-core parsing.

Specificity is spec-faithful: `(inline, id, class+attr+pseudo_class,
type+pseudo_element)` compared lexicographically. Same-specificity
ties break on source order. `!important` inverts origin priority, so
a UA `!important` rule beats an author `!important` rule.

`Stylesheet::new()` bakes in UA defaults (`:disabled { color: <muted>; user-select: none }`, …).
`Stylesheet::bare()` skips them for tests.

Boxes size as `box-sizing: content-box`, the CSS initial value: `width`, `height` and `min-*` / `max-*` measure the content box, padding and border lie outside it (form controls are `border-box` in the UA sheet); start a sheet with `*, ::before, ::after { box-sizing: border-box }` to size every box by its border.

## Pseudo-elements and `content`

```rust
use rdom_tui::prelude::*;

let sheet = Stylesheet::new()
    .rule("tree-item::before", TuiStyle::new()
        .content(Content::Str("▾ ".into()))
        .fg(Color::Rgb(128, 128, 128)))
    .unwrap();
```

Content can be a literal string, a `var()` reference, a concat of
parts, or explicit suppression:

```rust
use rdom_tui::prelude::*;

let forms = [
    Content::Str("▾ ".into()),
    Content::Var("arrow".into()),
    Content::Concat(vec![
        Content::Str("▾ ".into()),
        Content::Var("label".into()),
    ]),
    Content::None, // content: none; — suppresses the pseudo-element
];
```

Pseudo-elements inherit from the *host* element's computed style, not
from the host's parent.

Legacy fallback: if no rule supplies `content:`, the cascade falls
back to `TuiExt.before_content` / `after_content` (settable via
`node.set_before_content("→")` on `TuiNodeMutExt`).

## Custom properties and `var()`

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), StyleError> {
    let white = TuiColor::Literal(Color::Rgb(255, 255, 255));
    let sheet = Stylesheet::new()
        .define_var("accent", "#3d90ce")
        .define_var("muted", "gray")
        .rule(".primary", TuiStyle::new().fg_var("accent"))?
        .rule(".secondary", TuiStyle::new().fg(TuiColor::var_with("unknown", white)))?;
    Ok(())
}
```

In CSS text, `var()` works in every property (`padding: var(--gap)`,
`content: var(--label)`, `color: rgb(var(--r), 0, 0)`): the cascade
substitutes each `var()` from the element's custom properties — or its
fallback, any tokens, itself substituted — and parses the result with
the property's grammar; a failure makes the property `unset` (CSS
Variables 1 §3). A custom property's own `var()`s substitute where it
is declared, and a dependency cycle makes its members undefined.

The builder's typed `TuiColor::Var` references are tried in this order:

1. Look up the name in the element's custom-property map (its own
   `--*` declarations layered over the parent's, CSS Variables 1). If
   found and parses as a color, use it.
2. Otherwise, recurse into the explicit fallback chain
   (`var(--a, var(--b, red))` walks left-to-right).
3. If all fail, fall back to the property's inherit value (typically
   the parent's computed color).

A custom property's text is parsed with the full CSS `<color>` grammar
(hex, named colors, `rgb()` / `hsl()` / `oklch()` / `color-mix()` / …,
system colors, `light-dark()`), plus rdom's `reset` and a decimal
`0..=255` for `Color::Indexed`.

## Inline formatting

Block elements with `display: inline` children flow horizontally,
wrap at word boundaries, and style each fragment with its own
cascaded values — `<p>prefix <code>inline</code> suffix</p>` renders
on one line with `inline` yellow while the rest stays default.

```rust
use rdom_tui::prelude::*;

let sheet = Stylesheet::new() // UA defaults include display: inline
    .rule("p", TuiStyle::new().width(Size::Fixed(40)))
    .unwrap();
// Authors usually don't need to set display — UA defaults cover
// b, strong, em, i, u, code, span, a, br as inline; p, h1-3, pre
// as block.
```

What's supported:

- **`display: inline`** — participates in the parent block's inline
  formatting context. UA defaults mark `b`, `strong`, `em`, `i`, `u`,
  `code`, `span`, `a`, `br` as inline.
- **Word wrap** at whitespace, between CJK graphemes, and after
  hyphens. Long words overflow their line (CSS default — no
  char-break).
- **Auto-height IFC blocks** grow to fit wrapped content; **Fixed**
  height clips overflowing lines.
- **`white-space: normal` / `pre` / `pre-wrap` / `nowrap`** —
  `normal` collapses whitespace runs and trims IFC edges; `pre`
  preserves whitespace and treats `\n` as a hard break (no soft
  wrap); `pre-wrap` preserves whitespace, treats `\n` as a hard
  break, AND soft-wraps at spaces (HTML `<textarea>` default);
  `nowrap` collapses but never soft-wraps. Inherits.
- **`<br>`** — hard break.
- **Nested inline styles compose** — `<b>bold <i>+italic</i></b>`
  contributes both modifiers to the inner span.

What's out of scope:

- Inline borders / margins (`display: inline-block` is supported as an
  atomic inline).
- `text-align`, justification, baseline alignment.
- UAX #14 line breaking (we use whitespace + CJK + hyphen).

Mixed block + inline children work as in CSS 2.1 §9.2.1.1: each run
of inline content between block children is wrapped in an anonymous
block box with its own inline formatting context. Static `::before` /
`::after` are laid out as the host's first / last inline content, so
text wraps around them; a `::before` on a host that starts with a
block child gets a line of its own (an `<li>`'s marker rides its
block child's first line).

See the `parse_and_render` example for a working template.

## Interaction state: `:hover`, `:active` and `:focus`

```rust
use rdom_tui::prelude::*;

let mut dom: TuiDom = TuiDom::new();
let button = dom.create_element("button");
let input = dom.create_element("input");
dom.set_hovered(Some(button)); // fires InteractionChanged(Hover)
dom.set_active(Some(button)); // fires InteractionChanged(Active)
dom.set_focused(Some(input)); // fires InteractionChanged(Focus)
```

The setters fire `Mutation::InteractionChanged` records so a
`DirtyTracker` can invalidate the elements whose match flipped.
`:hover`, `:active` and `:focus-within` also match every ancestor of
the element holding the state (Selectors 4 §9.2 / §9.4), so `li:hover`
applies while the pointer is over a `<span>` inside the `<li>`; the
tracker restyles the old and new ancestor chains minus their common
part. The `App` sets `:active` for the duration of a left-button press.

`:focus-visible` matches the focused element while
`dom.focus_visible()` is `true`. The `App` keeps that bit with the
browsers' heuristics — a key press makes focus evident, a mouse click
does so only on a text field or editing host — and the UA focus tint
keys on it; `dom.set_focus_visible(bool)` sets it directly (it fires
`InteractionChanged(FocusVisible)`).

## Incremental re-cascade

Full cascade walks the whole tree. For apps with many elements and
frequent small mutations, install a `DirtyTracker` and cascade only
the affected subtrees:

```rust
use rdom_tui::prelude::*;

let mut dom: TuiDom = TuiDom::new();
let tracker = DirtyTracker::install(&mut dom);
let sheet = Stylesheet::new()
    .rule("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
    .unwrap();

// The size `vw` / `vh` resolve against, for every cascade below
// (the `App` sets its terminal's; `layout_dom(area)` records its area).
dom.set_viewport(Viewport::new(80, 24));

// Initial paint — cascade everything once.
dom.cascade(&sheet);

// Later, mutations fire observers; the tracker collects dirty roots.
let div = dom.create_element("div");
dom.append_child(dom.root(), div).unwrap();
dom.set_attribute(div, "class", "hero").unwrap();

// Re-cascade just the changed subtrees.
let roots = tracker.take_roots();
dom.cascade_subtrees(&sheet, &roots);
```

`DirtyTracker` handles: attribute + class changes (node+subtree),
tree insertions/removals (with sibling-dependent re-matching for
`:first-child`, `+`, `~`), hover / active / focus changes (the
previous and next targets' ancestor chains, minus their common part),
and stylesheet swap. Text-content changes dirty only the elements
whose `:empty` / `:placeholder-shown` they can flip; they (and
selection moves) set flags an `App` turns into a layout / repaint.

The `TuiNodeMutExt` style setters (`set_width`, `set_padding`, …,
`set_inline_style`) reflect into the `style` attribute like a CSSOM
write, so the tracker sees them. Bypass the observer (e.g. writing a
`TuiExt` field directly, such as `TuiExt::set_inline_style`)? Call
`tracker.mark_dirty(&mut dom, id)` manually.

## `!important` ladder

The cascade applies declarations in six ordered passes:

1. UA normal  →  2. Author normal  →  3. Inline normal  →
4. Author important  →  5. Inline important  →  6. UA important.

Within each pass, candidates are sorted by `(specificity, source_idx)`
ascending; later wins. `!important` inverts origin priority, but the
`style` attribute beats the author's rules at both importances (CSS
Cascade 4 §6.1 "element-attached styles"), so an inline `!important`
beats an author `!important` — as in browsers. Author passes run once
per cascade layer (`@layer`); the inline passes sort above every layer
(Cascade 5 §6.1).

`revert` (CSS Cascade 4 §7.3) rolls a property back through this
ladder: in an author or inline declaration to the value after pass 1
(the UA origin), in a UA declaration to the value before any pass
(`unset`).

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
        .tick_rate(Duration::from_millis(50))
        .on_tick(|_ctx| {
            // drain background channels, mutate DOM, request redraw
            ControlFlow::Continue
        })
        .run()
}
```

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

## Features

- `test-util` — exposes `VirtualScreen` (`rdom_tui::VirtualScreen`, also in the prelude), a headless VT emulator that replays the ANSI bytes a backend emits into an inspectable cell grid, for asserting what a real terminal would show after a sequence of frames, resizes and clears. It is for test suites: enable it on the dev-dependency (`rdom-tui = { version = "…", features = ["test-util"] }`), not on the normal one. It is not part of the default API, and docs.rs builds with it so the type is documented.
- `no-synchronized-output` — skips the BSU / ESU synchronized-output sequences (DEC private mode 2026) for terminals that mishandle them. Off by default; terminals without support ignore the sequences.

## Terminal notes

- **Startup color-scheme query.** Unless the app sets a scheme (`App::with_color_scheme`), `App::run` asks the terminal for its background with OSC 11 before the first frame (Unix, when stdout is a terminal; replies are read from stdin, or `/dev/tty` when stdin is redirected), and prefers the light or dark scheme it calls for — what `light-dark()` and `color-scheme: normal` follow. A terminal that answers ends the wait at once; one that answers nothing costs 200 ms; a reply that has begun is waited for up to 800 ms more. Keys typed during the wait are kept and handled after it, and a reply that arrives later is still taken as the answer. `App::detected_background()` is the reported color, `None` when there was no answer.
- **Theme changes.** `App::run` enables DEC mode 2031 on Unix: a terminal that supports it (Contour, Ghostty, kitty, and others) reports a switch between its light and dark themes, and the preferred scheme follows, restyling the tree — unless the app set the scheme. On Windows neither the startup query nor theme reports are read (crossterm's console reader delivers no terminal replies), so the scheme is dark unless the app sets it.
- **Input.** On Unix rdom reads the terminal itself (its own reader, inside `App::run`), parsing keys (including the kitty keyboard protocol), SGR mouse, paste, focus and resize into crossterm's `Event` types; an unknown escape sequence is consumed instead of stalling the input after it, and a lone `ESC` is the Esc key after 25 ms without a following byte.
- **iTerm2 and hover.** iTerm2 may ignore the any-motion mouse mode until it sees a real click, at launch and after every refocus, so `:hover` styles start following the pointer only after one click. Other terminals (Alacritty, Kitty, Ghostty, WezTerm) honor it immediately. The cause and the failed re-arm attempts are recorded in `specs/DIVERGENCES.md` §4.

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

## Further reading

- [`DESIGN.md`](../../specs/DESIGN.md) — architectural overview: crate map, non-negotiable invariants, roadmap.
- [`DIVERGENCES.md`](../../specs/DIVERGENCES.md) — every deliberate departure from the web platform.
- Module docs under `src/style/`, `src/render/`, and `src/runtime/` — each submodule has a top-level doc comment with the specific role.
