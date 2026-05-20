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

```rust
use rdom_tui::prelude::*;

let mut dom: TuiDom = TuiDom::new();
// ... build the tree ...
let sheet = Stylesheet::new()
    .rule(".hero", TuiStyle::new().fg(Color::Red))?;

App::new(dom, sheet)?.run() // blocks, returns on Ctrl-C / quit
```

See [`examples/`](examples/) for five working demos.

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
let sheet = Stylesheet::new()
    .rule(".hero", TuiStyle::new()
        .fg(Color::Red)
        .padding(Padding::all(1))
        .border(Border::Single))
    .unwrap();

// Run the cascade.
dom.cascade(&sheet);

// Read the final values.
let c = dom.node(hero).computed().unwrap();
assert_eq!(c.fg, Color::Red);
assert_eq!(c.padding, Padding::all(1));
assert_eq!(c.border, Border::Single);
```

## Stylesheets

Build rules with the fluent API. Selector errors surface at
stylesheet-build time — not at render time.

```rust
let sheet = Stylesheet::new()
    .rule("#nav",             TuiStyle::new().width(Size::Fixed(24)))?
    .rule(".row:hover",       TuiStyle::new().bg(Color::DarkGray))?
    .rule("input:focus",      TuiStyle::new().border_fg(Color::Blue))?
    .rule("tree-item::before", TuiStyle::new().content(Content::Str("▾ ".into())))?
    .rule(":not(.active) a", TuiStyle::new().dim(true))?;
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

`Stylesheet::new()` bakes in UA defaults (`[disabled] { dim: true; }`).
`Stylesheet::bare()` skips them for tests.

## Pseudo-elements and `content`

```rust
let sheet = Stylesheet::new()
    .rule("tree-item::before", TuiStyle::new()
        .content(Content::Str("▾ ".into()))
        .fg(Color::Gray))?;
```

Content can be a literal string, a `var()` reference, a concat of
parts, or explicit suppression:

```rust
Content::Str("▾ ".into())
Content::Var("arrow".into())
Content::Concat(vec![
    Content::Str("▾ ".into()),
    Content::Var("label".into()),
])
Content::None  // content: none; — suppresses the pseudo-element
```

Pseudo-elements inherit from the *host* element's computed style, not
from the host's parent.

Legacy fallback: if no rule supplies `content:`, the cascade falls
back to `TuiExt.before_content` / `after_content` (settable via
`node.set_before_content("→")` on `TuiNodeMutExt`).

## Custom properties and `var()`

```rust
let sheet = Stylesheet::new()
    .define_var("accent", "#3d90ce")
    .define_var("muted",  "gray")
    .rule(".primary",   TuiStyle::new().fg_var("accent"))?
    .rule(".secondary", TuiStyle::new().fg(TuiColor::var_with(
        "unknown", TuiColor::Literal(Color::White))))?;
```

`var()` references are tried in this order:

1. Look up the name in the vars map. If found and parses as a color,
   use it.
2. Otherwise, recurse into the explicit fallback chain
   (`var(--a, var(--b, red))` walks left-to-right).
3. If all fail, fall back to the property's inherit value (typically
   the parent's computed color).

The string color grammar is hex (`#rgb`, `#rrggbb`), ANSI named
(`red`, `blue`, `gray`, `lightcyan`, ...), `reset`, or decimal
`0..=255` for `Color::Indexed`.

## Inline formatting

Block elements with `display: inline` children flow horizontally,
wrap at word boundaries, and style each fragment with its own
cascaded values — `<p>prefix <code>inline</code> suffix</p>` renders
on one line with `inline` yellow while the rest stays default.

```rust
let sheet = Stylesheet::new()  // UA defaults include display: inline
    .rule("p",    TuiStyle::new().width(Size::Fixed(40)))?;
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

What's out of scope (for v1):

- `display: inline-block`.
- Inline borders / margins.
- `text-align`, justification, baseline alignment.
- UAX #14 line breaking (we use whitespace + CJK + hyphen).

Mixed block+inline children are a **cascade error** (§4 of
`RDOM_INLINE.md`) — the block degrades to non-IFC and the inline
kids are skipped at paint. Keep your children all-inline or
all-block per container.

See `RDOM_INLINE.md` for the full algorithm and the
`parse_and_render` example for a working template.

## Interaction state: `:hover` and `:focus`

```rust
dom.set_hovered(Some(button));   // fires InteractionChanged(Hover)
dom.set_focused(Some(input));    // fires InteractionChanged(Focus)
```

Both setters fire `Mutation::InteractionChanged` records so a
`DirtyTracker` can invalidate both the previously-hovered node and
the newly-hovered node, causing the next cascade to re-evaluate
`:hover` / `:focus` matches on both sides.

## Incremental re-cascade

Full cascade walks the whole tree. For apps with many elements and
frequent small mutations, install a `DirtyTracker` and cascade only
the affected subtrees:

```rust
let mut dom: TuiDom = TuiDom::new();
let tracker = DirtyTracker::install(&mut dom);
let sheet = Stylesheet::new()
    .rule("div", TuiStyle::new().fg(Color::Red))
    .unwrap();

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
`:first-child`, `+`, `~`), hover/focus changes (both previous and
next targets), and stylesheet swap. Text-content changes do NOT
dirty — text doesn't affect selector matching.

Bypass the observer (e.g. writing `TuiExt.inline_style` directly via
`set_inline_style`)? Call `tracker.mark_dirty(&mut dom, id)` manually.

## `!important` ladder

The cascade applies declarations in six ordered passes:

1. UA normal  →  2. Author normal  →  3. Inline normal  →
4. Inline important  →  5. Author important  →  6. UA important.

Within each pass, candidates are sorted by `(specificity, source_idx)`
ascending; later wins. This means an `!important` declaration in an
author stylesheet beats `!important` on an inline style (matches
browser behavior).

## Architecture

- `rdom-core` stays style-agnostic. No `Color`, no `Stylesheet`, no
  `TuiStyle` in the core crate.
- `ComputedStyle` on `TuiExt` is the *only* post-cascade source of
  truth. Layout and paint should read `node.computed()`, never
  `inline_style` directly.
- Inheritance is data, not code. Edit `INHERITS_MASK` in `cascade.rs`
  to change which properties inherit — the cascade walk stays the
  same.
- Selector matching goes through `rdom_core::Dom::matches_list` once
  per rule per element. There is one matching engine.
- `MutationObserver` is the invalidation mechanism. `DirtyTracker` is
  *one* observer; future devtools / a11y mirrors / reactive bindings
  can register their own without touching the cascade code.

## Runtime — `App`, event loop, hit test, routing

`App::run` is the app-facing entry point. It wraps the DOM in a
crossterm-driven loop that runs the "rendering steps" model
borrowed from the HTML spec: drain events, tick, run
`requestAnimationFrame` callbacks, cascade + layout + paint when
dirty, then sleep on the next event. One paint per task-end, no
matter how many mutations happen inside a single handler.

```rust
App::new(dom, sheet)?
    .tick_rate(Duration::from_millis(50))
    .on_tick(|ctx| {
        // drain background channels, mutate DOM, request redraw
        ControlFlow::Continue
    })
    .run()
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
- **Wheel auto-scroll** — walks up from the hit target for the
  nearest `overflow: Scroll | Auto` ancestor and adjusts its
  scroll offset. Cancelable via `prevent_default` on `wheel`.
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
  All, Contain }` gates selectability. `::selection` pseudo-element
  paints selected cells with a reversed-fg/bg overlay.
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

## Examples

```text
cargo run -p rdom-tui --example counter_button
cargo run -p rdom-tui --example scrollable_list
cargo run -p rdom-tui --example tab_form
cargo run -p rdom-tui --example selectable_text
cargo run -p rdom-tui --example parse_and_render
```

All five examples use `App::run`. No manual event loops, no direct
`enter_tui_mode` in user code.

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
