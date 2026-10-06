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

`display: flex` lays its items out in a row: `flex-direction`'s initial value is `row` (CSS Flexbox §5.1; rdom 0.5 and earlier defaulted to `column`), so a container meant to stack its items needs `flex-direction: column` (`.flex_column()`). `flex-direction` applies to flex containers only: a block container stacks its children on its block axis whatever it says.

Boxes size as `box-sizing: content-box`, the CSS initial value: `width`, `height` and `min-*` / `max-*` measure the content box, padding and border lie outside it (form controls are `border-box` in the UA sheet); start a sheet with `*, ::before, ::after { box-sizing: border-box }` to size every box by its border.

## Grid layout

`display: grid` lays its children out on rows and columns (CSS Grid Layout 2): track lists with cells, `%`, `fr`, `minmax()`, `fit-content()` and `repeat()` (including `auto-fill` / `auto-fit`), named lines and areas, line, span and area placement with `dense` auto-placement, `subgrid`, and Box Alignment in both axes. Lengths are whole cells, so tracks are too. A page laid out with named areas, its `main` a grid of cards that fits as many 4-cell columns as it can:

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        r#"
        .page {
            display: grid;
            height: 5;
            grid-template-columns: 6 1fr;
            grid-template-rows: auto 1fr auto;
            grid-template-areas: "head head" "nav main" "foot foot";
            gap: 0 1;
        }
        .head { grid-area: head }
        .nav  { grid-area: nav }
        .foot { grid-area: foot }
        .main {
            grid-area: main;
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(4, 1fr));
            column-gap: 1;
        }
        .wide { grid-column: 1 / -1 }
        "#,
    )?;

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let mut add = |parent: NodeId, class: &str, text: &str| -> NodeId {
        let id = dom.create_element("div");
        dom.set_attribute(id, "class", class).unwrap();
        if !text.is_empty() {
            let t = dom.create_text_node(text);
            dom.append_child(id, t).unwrap();
        }
        dom.append_child(parent, id).unwrap();
        id
    };
    let page = add(root, "page", "");
    add(page, "head", "Title");
    add(page, "nav", "Menu");
    let main = add(page, "main", "");
    add(page, "foot", "Status");
    add(main, "card", "ab");
    add(main, "card", "cd");
    add(main, "wide", "wide");
    add(main, "card", "ef");

    // Cascade, lay out and paint into a 20 × 5 buffer.
    let area = Rect::new(0, 0, 20, 5);
    dom.cascade(&sheet);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);

    let rows: Vec<String> = (0..5)
        .map(|y| (0..20).map(|x| buf.cell(x, y).unwrap().symbol()).collect())
        .collect();
    // The `main` area is 13 cells wide: two 6-cell columns and a gap.
    // `.wide` spans both (`1 / -1`), so the next card starts a new row.
    assert_eq!(
        rows,
        [
            "Title               ",
            "Menu   ab     cd    ",
            "       wide         ",
            "       ef           ",
            "Status              ",
        ]
    );
    Ok(())
}
```

The builders write the same declarations from Rust — here the page alone, its rules on classes and its items placed by area name:

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), StyleError> {
    let sheet = Stylesheet::new()
        .rule(".page", TuiStyle::new()
            .grid()
            .height(Size::Fixed(5))
            .grid_template_columns(vec![TrackSize::cells(6), TrackSize::fr(1.0)])
            .grid_template_rows(vec![TrackSize::AUTO, TrackSize::fr(1.0), TrackSize::AUTO])
            .grid_template_areas(
                GridTemplateAreas::new(["head head", "nav main", "foot foot"]).unwrap(),
            )
            .column_gap(1u16))?
        .rule(".head", TuiStyle::new().grid_area_named("head"))?
        .rule(".nav", TuiStyle::new().grid_area_named("nav"))?
        .rule(".main", TuiStyle::new().grid_area_named("main"))?
        .rule(".foot", TuiStyle::new().grid_area_named("foot"))?;

    let mut dom: TuiDom = TuiDom::new();
    let page = dom.create_element("div");
    dom.set_attribute(page, "class", "page").unwrap();
    dom.append_child(dom.root(), page).unwrap();
    let mut items = Vec::new();
    for class in ["head", "nav", "main", "foot"] {
        let id = dom.create_element("div");
        dom.set_attribute(id, "class", class).unwrap();
        let text = dom.create_text_node(class);
        dom.append_child(id, text).unwrap();
        dom.append_child(page, id).unwrap();
        items.push(id);
    }

    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 20, 5));
    let rects: Vec<(i32, i32, u16, u16)> = items
        .iter()
        .map(|&id| {
            let r = dom.node(id).layout_rect().unwrap();
            (r.x, r.y, r.width, r.height)
        })
        .collect();
    assert_eq!(rects, [(0, 0, 20, 1), (0, 1, 6, 3), (7, 1, 13, 3), (0, 4, 20, 1)]);
    Ok(())
}
```

A node can carry the same declarations inline: `set_grid()`, `set_grid_template_columns(…)`, `set_grid_area_named("head")`, `set_grid_row(…)` and the other `TuiNodeMutExt` grid setters.

## Floats and text overflow

`float` / `clear` follow CSS 2.1 §9.5: a float leaves the line, lines beside it are shortened, and `clear` moves a box below it; a box that establishes a block formatting context (`overflow: hidden`, `display: flow-root`) contains its floats, and so does the clearfix — an empty block `::after` that clears. `text-overflow` marks a clipped line's cut edge, and `line-clamp` ends a block after its Nth line with an ellipsis. A float floats where its parent lays out in block flow: the document root's children are items of rdom's viewport column, so put the content in a `<body>`, as a browser's is.

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        r#"
        .media { float: left; width: 4; height: 2; margin-right: 1 }
        .card::after { content: ""; display: block; clear: both }
        .truncate { overflow: hidden; white-space: nowrap; text-overflow: ellipsis }
        .clamp { line-clamp: 2 }
        "#,
    )?;

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let body = dom.create_element("body");
    dom.append_child(root, body).unwrap();
    let mut add = |parent: NodeId, tag: &str, class: &str, text: &str| -> NodeId {
        let id = dom.create_element(tag);
        if !class.is_empty() {
            dom.set_attribute(id, "class", class).unwrap();
        }
        if !text.is_empty() {
            let t = dom.create_text_node(text);
            dom.append_child(id, t).unwrap();
        }
        dom.append_child(parent, id).unwrap();
        id
    };
    let card = add(body, "div", "card", "");
    add(card, "div", "media", "IMG");
    add(card, "p", "", "Some text");
    add(body, "p", "truncate", "This line is far too long");
    add(body, "p", "clamp", "one two three four five six seven eight");

    let area = Rect::new(0, 0, 16, 6);
    dom.cascade(&sheet);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);

    let rows: Vec<String> = (0..6)
        .map(|y| (0..16).map(|x| buf.cell(x, y).unwrap().symbol()).collect())
        .collect();
    // The text runs beside the 2-row float (past its margin); the
    // clearfix makes `.card` as tall as the float, so the next paragraph
    // starts below it; the long line ends in `…`; the clamped block keeps
    // two lines, the second marked.
    assert_eq!(
        rows,
        [
            "IMG  Some text  ",
            "                ",
            "This line is fa…",
            "one two three   ",
            "four five six…  ",
            "                ",
        ]
    );
    Ok(())
}
```

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
- **Word wrap** at the soft wrap opportunities of a UAX #14 subset
  (CSS Text 3 §5): whitespace, around CJK ideographs (not before
  closing punctuation or small kana), after hyphens, zero-width spaces,
  `<wbr>`, and soft hyphens, which show `-` where the line breaks
  (`hyphens: manual`). `word-break` (`break-all` / `keep-all`),
  `line-break` (`loose` / `normal` / `strict` / `anywhere`) and
  `overflow-wrap` (`anywhere` / `break-word`, alias `word-wrap`) apply;
  without them a long word overflows its line.
- **Auto-height IFC blocks** grow to fit wrapped content; a fixed
  height lets overflowing lines paint on below the box (CSS `overflow:
  visible`) — `overflow: hidden` or `clip` clips them.
- **`white-space`** (`normal` / `pre` / `pre-wrap` / `pre-line` /
  `nowrap` / `break-spaces`, CSS Text 4 §3) and its longhands
  `white-space-collapse` and `text-wrap-mode` — per element, as they
  apply to text: `normal` collapses whitespace runs and trims line
  edges; `pre` preserves whitespace and treats `\n` as a hard break
  (no soft wrap); `pre-wrap` preserves whitespace, its trailing spaces
  hanging at a soft wrap (HTML `<textarea>` default); `pre-line`
  collapses spaces but keeps line feeds; `break-spaces` keeps spaces
  that take up room and wrap; `nowrap` collapses but never soft-wraps.
  Inherits.
- **`<br>`** — hard break.
- **`line-height`** in whole rows, a fraction floored (`1.5` is one row,
  `2.5` two; CSS Inline 3 §5.1): every inline box and the block's strut
  add their half-leading around the glyph row; `lh` / `rlh` follow it.
- **`letter-spacing` / `word-spacing`** in whole blank cells (CSS Text 3
  §9): after each grapheme / word separator, none at a line's end.
- **`vertical-align`** (CSS 2.1 §10.8.1): `sub` / `super` a row,
  lengths, `middle` / `text-top` / `text-bottom`, and `top` / `bottom`
  aligned subtrees, on spans, generated text and inline blocks.
- **`text-decoration`** (CSS Text Decoration 4): underline (in its
  style and color, SGR `4:x` / `58` where the terminal has them),
  overline, line-through, blink, propagated to the text of in-flow
  descendants.
- **Fonts** — `font-weight` (bold from 600, `bolder` / `lighter`),
  `font-style` (`italic`, `oblique`), the `font` shorthand.
- **Nested inline styles compose** — `<b>bold <i>+italic</i></b>`
  contributes both modifiers to the inner span.

`<sub>` and `<sup>` follow HTML's UA sheet (`vertical-align: sub` /
`super`), and in a terminal that is a whole row: `x<sup>2</sup>` makes a
two-row line where a browser grows it by a third of an em — footnote
markers and `1<sup>st</sup>` in table cells and list rows double them.
To keep such lines one row, opt out:

```rust
use rdom_tui::prelude::*;

fn rows(css: &str) -> Vec<String> {
    let sheet = rdom_css::from_css_strict(css).unwrap();
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.append_child(root, p).unwrap();
    let x = dom.create_text_node("x");
    dom.append_child(p, x).unwrap();
    let sup = dom.create_element("sup");
    let two = dom.create_text_node("2");
    dom.append_child(sup, two).unwrap();
    dom.append_child(p, sup).unwrap();
    let area = Rect::new(0, 0, 3, 2);
    dom.cascade(&sheet);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    (0..2)
        .map(|y| (0..3).map(|x| buf.cell(x, y).unwrap().symbol()).collect())
        .collect()
}

// The UA sheet raises the `2` a row, and the line is two rows tall.
assert_eq!(rows(""), [" 2 ", "x  "]);
// The opt-out keeps it on the baseline: one row.
assert_eq!(rows("sub, sup { vertical-align: baseline }"), ["x2 ", "   "]);
```

What's out of scope:

- Inline borders / margins (`display: inline-block` is supported as an
  atomic inline).
- Full UAX #14 line breaking and dictionary hyphenation (the subset
  rdom implements is listed in DIVERGENCES; `hyphens: auto` is
  `manual`).

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
