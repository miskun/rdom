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

## Responsive layout: `@media` and container queries

`@media` (Media Queries 4 / 5) reads the terminal: `width` / `height` are its columns and rows — a unitless number or `ch` is cells, as in every rdom length (a pixel breakpoint selects at 8px a column and 16px a row, `em` at 16px — `(min-width: 640px)` from 80 columns) — and the preferences an `App` reports (`App::with_media_preferences`: `prefers-reduced-motion`, …), `prefers-color-scheme` from the terminal's background. A resize restyles only when a query flips or a viewport unit is in use; `App::match_media` is `matchMedia()`, its `change` listeners (`add_event_listener("change", …)`, or the legacy `add_listener`) called on each flip — a list with a listener is kept while it has one, so `app.match_media(q).add_listener(f)` works chained. Columns that stack on a narrow terminal:

```rust
use rdom_tui::prelude::*;

/// `(x, y)` of `#b` with the sheet laid out in a `width`-column terminal.
fn second_column(width: u16) -> std::result::Result<(i32, i32), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        ".cols { display: flex }
         .cols > div { flex: 1 }
         @media (width < 60) { .cols { flex-direction: column } }",
    )?;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(
        &mut dom,
        r#"<div class="cols"><div id="a">nav</div><div id="b">main</div></div>"#,
        root,
    )?;
    // The viewport first: the cascade evaluates the queries against it.
    dom.set_viewport(Viewport::new(width, 10));
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, width, 10));
    let b = dom.node(dom.get_element_by_id("b").unwrap()).layout_rect().unwrap();
    Ok((b.x, b.y))
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    assert_eq!(second_column(80)?, (40, 0), "side by side");
    assert_eq!(second_column(40)?, (0, 1), "stacked");
    Ok(())
}
```

A container query asks the size of the box a component sits in rather than the terminal's (CSS Conditional 5 §6): `container-type: inline-size` makes an element a query container — its width no longer depends on its content — and `@container` rules inside it apply while its content box matches. The `cq*` units are percentages of it. The same card, a column in a narrow sidebar and a row in the wide main pane:

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        ".page { display: flex }
         .side { width: 20 }
         .main { flex: 1 }
         .side, .main { container-type: inline-size }
         .card { display: flex; flex-direction: column }
         .card .title { width: 10 }
         @container (width >= 30) { .card { flex-direction: row } }",
    )?;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(
        &mut dom,
        r#"<div class="page">
             <div class="side"><div class="card"><div class="title">A</div><div id="sa">x</div></div></div>
             <div class="main"><div class="card"><div class="title">B</div><div id="ma">y</div></div></div>
           </div>"#,
        root,
    )?;
    dom.set_viewport(Viewport::new(70, 10));
    dom.cascade(&sheet);
    // Layout re-cascades each container's subtree once its width is known.
    dom.layout_dom(Rect::new(0, 0, 70, 10));
    let at = |id: &str| dom.node(dom.get_element_by_id(id).unwrap()).layout_rect().unwrap();
    // In the 20-wide sidebar the card stacks: its body is below the title.
    assert_eq!((at("sa").x, at("sa").y), (0, 1));
    // In the 50-wide main pane it is a row: its body is beside the title.
    assert_eq!((at("ma").x, at("ma").y), (30, 0));
    Ok(())
}
```

## Tables

HTML tables, and `display: table` on any element, lay out in a table formatting context (CSS 2.1 §17): columns sized by the automatic or `fixed` algorithm, `colspan` / `rowspan`, captions, separated borders with `border-spacing` or collapsed ones joined into junctions — all in whole cells. The UA sheet is HTML's: `th` bold and centred, `caption` centred, rows centring their cells, no borders. A data table — collapsed borders, zebra rows, a right-aligned numeric column picked by the column combinator, one-line rows at `width: max-content` in a wrapper that scrolls — and its used column ranges read back with `table_tracks()`, for a header or a resize handle drawn outside it:

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        "
        .scroll { width: 20; overflow: auto }
        table   { width: max-content; border-collapse: collapse }
        th, td  { border: solid }
        tbody tr:nth-child(even) { background-color: rgb(0, 0, 80) }
        col.num || td { text-align: right }
        ",
    )?;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(
        &mut dom,
        r#"<div><div class="scroll"><table id="files">
             <colgroup><col><col class="num"></colgroup>
             <thead><tr><th>File</th><th>Size</th></tr></thead>
             <tbody>
               <tr><td>notes.txt</td><td>12</td></tr>
               <tr><td>build-output.log</td><td>4096</td></tr>
               <tr><td>b</td><td>7</td></tr>
             </tbody>
           </table></div></div>"#,
        root,
    )?;

    let area = Rect::new(0, 0, 30, 11);
    dom.cascade(&sheet);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);

    let row = |y: u16| -> String { (0..20).map(|x| buf.cell(x, y).unwrap().symbol()).collect() };
    // The table is 27 cells wide; the wrapper shows 20 and scrolls.
    assert_eq!(row(1), "│       File       │");
    assert_eq!(row(3), "│ notes.txt        │");
    assert_eq!(row(5), "│ build-output.log │");
    // The even body row is striped across its cells.
    assert_eq!(buf.cell(3, 5).unwrap().bg, Color::Rgb(0, 0, 80));
    assert_ne!(buf.cell(3, 3).unwrap().bg, Color::Rgb(0, 0, 80));

    // Each column's cells from the table's content edge, the one-cell
    // collapsed lines between them: a header drawn elsewhere lines up.
    let table = dom.get_element_by_id("files").unwrap();
    let tracks = dom.node(table).table_tracks().unwrap();
    assert_eq!(tracks.columns(), [1..19, 20..26]);
    Ok(())
}
```

Scroll the wrapper (`scroll_to`) to bring the `Size` column, right-aligned by `col.num || td`, into view. Without `width: max-content` the table shrinks to its wrapper and wraps its cells, as a browser's does; `td { white-space: nowrap }` keeps rows one line either way. For exact column widths use `table-layout: fixed` with a table `width`: the first row's cells and the `<col>`s set the columns, and rows added later cannot move them.

## Floats and text overflow

`float` / `clear` follow CSS 2.1 §9.5: a float leaves the line, lines beside it are shortened, and `clear` moves a box below it; a box that establishes a block formatting context (`overflow: hidden`, `display: flow-root`) contains its floats, and so does the clearfix — an empty block `::after` that clears. `text-overflow` marks a clipped line's cut edge, and `line-clamp` ends a block after its Nth line with an ellipsis. A float floats where its parent lays out in block flow — the document root's children included, in the initial containing block.

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

## Lists, counters and generated content

List items are `display: list-item`: each carries a `::marker` whose text `list-style-type` makes from the `list-item` counter (CSS Lists 3). The marker hangs outside the item, in the list's four cells of padding, unless `list-style-position: inside` puts it in the line. Every predefined counter style is supported, and `@counter-style` defines more. `counter()` and `counters()` read a counter in `content`, `::marker` takes `color` and `content`, and `ol[reversed]`, `<li value>` and `<ol start>` number as HTML does. `content` also takes quotes (`open-quote`, `quotes`, `<q>`). `::first-line` and `::first-letter` style a block's first line and letter.

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        r#"
        @counter-style check { system: cyclic; symbols: "✓"; suffix: " " }
        .roman { list-style-type: upper-roman; margin-left: 2 }
        .roman li::marker { color: red }
        .outline { list-style-position: inside; padding-left: 0 }
        .outline .outline { padding-left: 2 }
        .outline li::marker { content: counters(list-item, ".") ". " }
        .done { list-style-type: check }
        "#,
    )?;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let body = dom.create_element("body");
    dom.append_child(root, body)?;
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
    let roman = add(body, "ol", "roman", "");
    for item in ["one", "two", "three"] {
        add(roman, "li", "", item);
    }
    let outline = add(body, "ol", "outline", "");
    add(outline, "li", "", "Intro");
    let part = add(outline, "li", "", "Body");
    let inner = add(part, "ol", "outline", "");
    add(inner, "li", "", "Part");
    let done = add(body, "ul", "done", "");
    add(done, "li", "", "milk");

    let area = Rect::new(0, 0, 14, 7);
    dom.cascade(&sheet);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);

    let rows: Vec<String> = (0..7)
        .map(|y| (0..14).map(|x| buf.cell(x, y).unwrap().symbol()).collect())
        .collect();
    // The markers hang in the lists' four cells of padding, ending where
    // the items start — a wider one past it, so the roman list keeps a
    // margin (a list at column 0 would cut "III. " to "II. "); the outline
    // numbers nest with `counters()`; the custom style's `✓` hangs too.
    assert_eq!(
        rows,
        [
            "   I. one     ",
            "  II. two     ",
            " III. three   ",
            "1. Intro      ",
            "2. Body       ",
            "  2.1. Part   ",
            "  ✓ milk      ",
        ]
    );
    // `::marker { color: red }` colours the marker, not the item's text.
    let red = Color::Rgb(255, 0, 0);
    assert_eq!(buf.cell(3, 0).unwrap().fg, red);
    assert_ne!(buf.cell(6, 0).unwrap().fg, red);
    Ok(())
}
```

## Custom highlights: search results

The CSS Custom Highlight API styles ranges of text without touching the tree. Register a `Highlight` of `Range`s under a name in `dom.highlights_mut()` and style it with `::highlight(name)`. The ranges are live, so they move with text edits, insertions and removals. `Dom::descendants` walks the text nodes, and `Dom::range_between` checks each pair of byte offsets. The registry reports one `Mutation::HighlightsChanged` after each change, which an `App` repaints on.

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        "::highlight(search) { background-color: yellow; color: black }",
    )?;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let body = dom.create_element("body");
    dom.append_child(root, body)?;
    for line in ["error: disk full", "ok", "an error again"] {
        let p = dom.create_element("p");
        let t = dom.create_text_node(line);
        dom.append_child(p, t)?;
        dom.append_child(body, p)?;
    }

    // One range per match, in every text node: `match_indices` gives byte
    // offsets, which is what a `Position` holds.
    let query = "error";
    let mut ranges = Vec::new();
    for id in dom.descendants(root) {
        let node = dom.node(id);
        if node.node_type() != NodeType::Text {
            continue;
        }
        for (at, hit) in node.node_value().unwrap_or_default().match_indices(query) {
            let (start, end) = (Position::new(id, at), Position::new(id, at + hit.len()));
            ranges.push(dom.range_between(start, end)?);
        }
    }
    dom.highlights_mut().set("search", Highlight::new(ranges));

    let area = Rect::new(0, 0, 16, 3);
    dom.cascade(&sheet);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);

    // Each row's text, and which cells the highlight painted yellow.
    let yellow = Color::Rgb(255, 255, 0);
    let row = |y: u16| -> (String, String) {
        (0..16)
            .map(|x| {
                let cell = buf.cell(x, y).unwrap();
                (cell.symbol().to_string(), if cell.bg == yellow { '^' } else { ' ' })
            })
            .unzip()
    };
    assert_eq!(row(0), ("error: disk full".into(), "^^^^^           ".into()));
    assert_eq!(row(1), ("ok              ".into(), "                ".into()));
    assert_eq!(row(2), ("an error again  ".into(), "   ^^^^^        ".into()));
    assert_eq!(buf.cell(0, 0).unwrap().fg, Color::Rgb(0, 0, 0));
    Ok(())
}
```

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
- **`text-align`** (CSS Text 3 §6; with `text-align-all` / `-last` and
  `text-justify`): `start` / `end` / `left` / `right` / `center`,
  `justify` (spaces widened in whole cells), `justify-all`,
  `match-parent`.
- **`text-indent`** (CSS Text 3 §8.1): the first line (`each-line`: also
  each after a forced break) starts that many cells in, `hanging`
  inverts it; `2ch` / `2` work, `em` does not.
- **`text-transform`** (CSS Text 3 §2.1): `uppercase` / `lowercase` /
  `capitalize`, `full-width`, `full-size-kana`; the caret, selection and
  copy work in the DOM's text.
- **`tab-size`** (CSS Text 3 §4.2): a preserved tab advances to the next
  stop (initial 8 cells).
- **`text-wrap`** (CSS Text 4 §6.1; with `text-wrap-mode` /
  `text-wrap-style`): `wrap` / `nowrap`, `balance` (lines of even
  length), `pretty`, `stable`.
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
block child gets a line of its own. A list item's `::marker` rides the
item's first line box — its own, or its block child's — hung outside the
item in the list's padding (`list-style-position: inside` puts it in the
line); a `display: list-item` `::before` / `::after` has a marker of its
own (`::before::marker`) on its own first line. A `<details>` element's
content is laid out in its `::details-content` box, which takes a
background, border, padding, a size and `overflow`.

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

## Form states

The input pseudo-classes follow HTML: `:valid` / `:invalid`, `:required` / `:optional`, `:read-only` / `:read-write`, `:in-range` / `:out-of-range`, `:default`, `:indeterminate` (a checkbox's flag, a radio group with nothing checked), and `:user-valid` / `:user-invalid`. The last two wait for the user: a field is judged once the user commits a change to it — leaving an edited text field, Enter in it, a toggle or a pick — or once its form's submission is attempted, and a reset forgets it. So an empty required field is `:invalid` from the start, but is shown as wrong only when the user has had a go at it:

```rust
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict("input:user-invalid { background-color: red }")?;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    dom.append_child(root, form)?;
    let email = dom.create_element("input");
    dom.set_attribute(email, "type", "email")?;
    dom.append_child(form, email)?;
    let name = dom.create_element("input");
    dom.set_attribute(name, "required", "")?;
    dom.append_child(form, name)?;
    let send = dom.create_element("button");
    let label = dom.create_text_node("Send");
    dom.append_child(send, label)?;
    dom.append_child(form, send)?;

    let terminal = Terminal::new(TestBackend::new(30, 6))?;
    let mut app = App::with_backend(dom, sheet, terminal)?;
    let key = |code| Event::Key(KeyEvent::new(code, KeyModifiers::NONE));
    let user_invalid =
        |app: &App<TestBackend>, id: NodeId| app.dom().matches(id, ":user-invalid").unwrap();

    // Untouched, the required field is `:invalid` but not `:user-invalid`.
    assert!(app.dom().matches(name, ":invalid")?);
    assert!(!user_invalid(&app, name));

    // Typing is not a commit: the edited field is not judged yet…
    app.dom_mut().node_mut(email).focus();
    for c in "nope".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }
    assert!(!user_invalid(&app, email));
    // …until it loses focus: `change` fires and its user validity is set.
    app.handle_event(key(KeyCode::Tab));
    assert!(user_invalid(&app, email));
    assert!(!user_invalid(&app, name), "focused, but never edited");

    // A submission attempt judges every field the form owns; the form
    // being invalid, it is blocked and the first invalid field focused.
    app.dom_mut().node_mut(send).click();
    assert!(user_invalid(&app, name));
    assert_eq!(app.dom().focused(), Some(email));

    // The next frame styles them (the focused field shows the UA's
    // `:focus-visible` tint, which is `!important`, over it).
    app.draw_if_dirty()?;
    let red = Color::Rgb(255, 0, 0);
    assert_eq!(app.dom().node(name).computed().unwrap().bg, red);
    Ok(())
}
```

Without an `App`, a bare `TuiDom` calls `runtime::builtins::validation::install` once before cascading, so the form states match.

### A text field has no border

A text field has no border in rdom: a browser draws a 2px inset one, but in a terminal a border is a whole row above and below and a column each side, so a one-row `<input>` would be three rows tall. The UA field is `padding: 0 1` on a `Field` background instead. So `input:user-invalid { border-color: red }`, the first rule a web developer writes, paints nothing — there is no border to colour. Mark the state with the field's own colours, or give the field a border yourself and pay its rows:

```rust
use rdom_tui::prelude::*;
use rdom_tui::runtime::builtins::validation;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        r#"
        .bare:invalid { border-color: red }
        .tint:invalid { background-color: red; color: white }
        .boxed { border: solid }
        .boxed:invalid { border-color: red }
        "#,
    )?;
    let mut dom: TuiDom = TuiDom::new();
    validation::install(&mut dom); // an `App` does this itself
    let root = dom.root();
    let body = dom.create_element("body");
    dom.append_child(root, body)?;
    let mut field = |class: &str| -> NodeId {
        let input = dom.create_element("input");
        dom.set_attribute(input, "class", class).unwrap();
        dom.set_attribute(input, "required", "").unwrap();
        dom.append_child(body, input).unwrap();
        input
    };
    field("bare");
    field("tint");
    field("boxed");

    let area = Rect::new(0, 0, 24, 5);
    dom.cascade(&sheet);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);

    let rows: Vec<String> = (0..5)
        .map(|y| (0..24).map(|x| buf.cell(x, y).unwrap().symbol()).collect())
        .collect();
    // `border-color` alone changes nothing: no border. The tinted field
    // keeps its one row (20 cells and its padding); the boxed one is
    // three, its border around the same box.
    assert_eq!(
        rows,
        [
            "                        ",
            "                        ",
            "┌──────────────────────┐",
            "│                      │",
            "└──────────────────────┘",
        ]
    );
    let red = Color::Rgb(255, 0, 0);
    let reddened = |y: u16| (0..24).any(|x| {
        let cell = buf.cell(x, y).unwrap();
        cell.fg == red || cell.bg == red
    });
    assert!(!reddened(0));
    assert!((0..22).all(|x| buf.cell(x, 1).unwrap().bg == red));
    assert_ne!(buf.cell(22, 1).unwrap().bg, red);
    assert!((2..5).all(|y| buf.cell(0, y).unwrap().fg == red));
    Ok(())
}
```

## Popovers and the top layer

`popover` elements (HTML §6.12) and modal dialogs render in the top layer: above every `z-index`, outside every ancestor's `overflow`, centred in the viewport by the UA sheet, on a `Canvas` background that hides the page under them. A `popovertarget` button toggles its popover; a click outside an auto popover, or Esc, closes it (light dismiss); `runtime::builtins::popover` has `show_popover` / `hide_popover` / `toggle_popover` for script. Until anchor positioning lands, place a popover yourself: in a `beforetoggle` listener the event's `source` is the invoker (`popover::invoker_of` answers once it shows), and its `bounding_rect` gives the cells to put the popover under, as `top` / `left` after `inset: auto`. Tab moves through a popover in tree order, so a popover placed away from its invoker should give the control to start at `autofocus`.

```rust
use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rdom_tui::prelude::*;
use rdom_tui::runtime::builtins::popover;
use rdom_tui::ToggleState;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(".menu { inset: auto }")?;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let body = dom.create_element("body");
    dom.append_child(root, body)?;
    let mut add = |parent: NodeId, tag: &str, text: &str| -> NodeId {
        let id = dom.create_element(tag);
        if !text.is_empty() {
            let t = dom.create_text_node(text);
            dom.append_child(id, t).unwrap();
        }
        dom.append_child(parent, id).unwrap();
        id
    };
    let button = add(body, "button", "Menu");
    for _ in 0..6 {
        add(body, "p", "xxxxxxxxxxxxxxxxxxxxxxxx");
    }
    let menu = add(body, "div", "");
    add(menu, "div", "Copy");
    add(menu, "div", "Paste");
    dom.set_attribute(button, "popovertarget", "menu")?;
    dom.set_attribute(menu, "id", "menu")?;
    dom.set_attribute(menu, "popover", "")?;
    dom.set_attribute(menu, "class", "menu")?;

    // Place the menu under its invoker as it opens.
    dom.add_event_listener(menu, "beforetoggle", ListenerOptions::default(), move |ctx| {
        let Some(toggle) = ctx.event.detail.as_toggle() else {
            return;
        };
        if toggle.new_state != ToggleState::Open {
            return;
        }
        let Some(at) = toggle.source.and_then(|b| ctx.dom.node(b).bounding_rect()) else {
            return;
        };
        let mut node = ctx.dom.node_mut(menu);
        let mut style = node.style_mut().unwrap();
        style.set_property("top", &(at.y + i32::from(at.height)).to_string()).unwrap();
        style.set_property("left", &at.x.to_string()).unwrap();
    })?;

    let terminal = Terminal::new(TestBackend::new(24, 8))?;
    let mut app = App::with_backend(dom, sheet, terminal)?;
    app.draw_if_dirty()?;

    // The invoker opens it, under itself.
    app.dom_mut().node_mut(button).click();
    assert!(popover::is_showing(app.dom(), menu));
    assert_eq!(popover::invoker_of(app.dom(), menu), Some(button));
    app.draw_if_dirty()?;
    let r = app.dom().node(menu).bounding_rect().unwrap();
    let b = app.dom().node(button).bounding_rect().unwrap();
    assert_eq!((r.x, r.y), (b.x, b.y + i32::from(b.height)));

    // Its `Canvas` fill hides the page: no `x` shows through its box.
    let area = Rect::new(0, 0, 24, 8);
    let mut buf = Buffer::empty(area);
    app.dom_mut().paint_dom(&mut buf, area);
    for y in r.y..r.y + i32::from(r.height) {
        for x in r.x..r.x + i32::from(r.width) {
            assert_ne!(buf.cell(x as u16, y as u16).unwrap().symbol(), "x");
        }
    }

    // Light dismiss: a press and release outside close it.
    let mouse = |kind| {
        Event::Mouse(MouseEvent {
            kind,
            column: 23,
            row: 7,
            modifiers: KeyModifiers::NONE,
        })
    };
    app.handle_event(mouse(MouseEventKind::Down(MouseButton::Left)));
    app.handle_event(mouse(MouseEventKind::Up(MouseButton::Left)));
    assert!(!popover::is_showing(app.dom(), menu));
    Ok(())
}
```

## Transitions and animations

`transition-*`, `@keyframes` / `animation-*` and `@starting-style` run on the `App`'s clock (CSS Transitions 1 / 2, CSS Animations 1 / 2). A running value *is* the computed value (Web Animations 1 §5.4.5): layout, paint and inheritance read it frame by frame, and so does `node.computed()` — mid-flight a `width: 2 → 10` transition reads `6` there; `node.base_computed()` is the cascade's style without the running values. Geometry moves in whole cells. `App::run` drives the clock from wall time; a test or a headless driver steps it with `App::advance(ms)`, so paint can be checked at fixed times:

```rust
use std::cell::RefCell;
use std::rc::Rc;

use rdom_tui::prelude::*;
use rdom_tui::Size;

/// The cells of row `y` painted blue.
fn blue(app: &mut App<TestBackend>, y: u16) -> usize {
    let area = Rect::new(0, 0, 20, 2);
    let mut buf = Buffer::empty(area);
    app.dom_mut().paint_dom(&mut buf, area);
    (0..20)
        .filter(|&x| buf.cell(x, y).unwrap().bg == Color::Rgb(0, 0, 255))
        .count()
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        ".bar { width: 2; height: 1; background-color: rgb(0, 0, 255);
                transition: width 100ms linear }
         .bar.wide { width: 10 }
         @keyframes pulse { from { width: 2 } to { width: 6 } }
         .pulse { width: 2; height: 1; background-color: rgb(0, 0, 255);
                  animation: pulse 100ms linear 2 alternate }",
    )?;
    let mut dom: TuiDom = TuiDom::new();
    // The bars in a `<body>`: under a transparent root, a `<body>`'s
    // background is the canvas's, painted over the whole screen.
    let body = dom.create_element("body");
    dom.append_child(dom.root(), body)?;
    let (bar, pulse) = (dom.create_element("div"), dom.create_element("div"));
    dom.set_attribute(bar, "class", "bar")?;
    dom.set_attribute(pulse, "class", "pulse")?;
    dom.append_child(body, bar)?;
    dom.append_child(body, pulse)?;
    let events = Rc::new(RefCell::new(Vec::new()));
    for kind in ["animationiteration", "animationend"] {
        let events = events.clone();
        dom.add_event_listener(pulse, kind, ListenerOptions::default(), move |ctx| {
            events.borrow_mut().push(ctx.event.event_type.clone());
        })?;
    }
    let terminal = Terminal::new(TestBackend::new(20, 2))?;
    let mut app = App::with_backend(dom, sheet, terminal)?;
    app.advance(0)?; // t = 0: the pulse starts

    app.dom_mut().set_attribute(bar, "class", "bar wide")?;
    app.advance(0)?; // the width transition starts
    app.advance(50)?; // t = 50 ms
    assert_eq!((blue(&mut app, 0), blue(&mut app, 1)), (6, 4));
    let node = app.dom().node(bar);
    assert_eq!(node.computed().unwrap().width, Size::Fixed(6));
    assert_eq!(node.base_computed().unwrap().width, Size::Fixed(10));

    app.advance(75)?; // t = 125 ms: the transition is over, the pulse runs back
    assert_eq!((blue(&mut app, 0), blue(&mut app, 1)), (10, 5));
    assert_eq!(*events.borrow(), ["animationiteration"]);

    app.advance(100)?; // t = 225 ms: two iterations done, no fill
    assert_eq!(blue(&mut app, 1), 2);
    assert_eq!(*events.borrow(), ["animationiteration", "animationend"]);
    Ok(())
}
```

### Porting web patterns that do not carry over

A terminal has no fonts and no pixels, and a `transform` only moves a box by whole cells (it cannot scale or rotate one), so three common browser patterns need a terminal form.

**A scroll-progress bar.** The web's `.progress { position: fixed; animation: grow linear; animation-timeline: scroll(root) }` scales the bar with `transform: scaleX()` and follows the viewport's scroller; rdom has no viewport scrolling (`scroll(root)` follows the root element, an element root only when it is a scroll container, and the root fragment never scrolls) and `scaleX()` draws nothing. Name the scroller's timeline, hoist it with `timeline-scope` to an ancestor the bar shares, and animate `width`:

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        "body { timeline-scope: --page }
         .progress { height: 1; background-color: rgb(0, 0, 255);
                     animation: grow linear both; animation-timeline: --page }
         @keyframes grow { from { width: 0% } to { width: 100% } }
         .page { height: 4; overflow-y: auto; scroll-timeline: --page }
         .page p { margin: 0 }",
    )?;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let body = dom.create_element("body");
    let (bar, page) = (dom.create_element("div"), dom.create_element("div"));
    dom.set_attribute(bar, "class", "progress")?;
    dom.set_attribute(page, "class", "page")?;
    for i in 0..12 {
        let p = dom.create_element("p");
        let t = dom.create_text_node(&format!("line {i}"));
        dom.append_child(p, t)?;
        dom.append_child(page, p)?;
    }
    dom.append_child(body, bar)?;
    dom.append_child(body, page)?;
    dom.append_child(root, body)?;
    let terminal = Terminal::new(TestBackend::new(20, 5))?;
    let mut app = App::with_backend(dom, sheet, terminal)?;
    app.advance(0)?;
    app.advance(0)?; // the first layout gives the scroll range the timeline reads

    let width = |app: &App<TestBackend>| app.dom().node(bar).bounding_rect().unwrap().width;
    assert_eq!(width(&app), 0);
    // 12 lines in 4 rows: a range of 8, half of it scrolled.
    app.dom_mut().node_mut(page).set_scroll_top(4)?;
    app.advance(0)?;
    assert_eq!(width(&app), 10);
    Ok(())
}
```

**A custom checkbox.** The web draws one with `input[type=checkbox] { appearance: none; width: 1em; height: 1em; border: 1px solid }` and fills it on `:checked`. Under `appearance: none` rdom drops the UA's `[x] ` mark too, and `em` is dropped (below), so that rule leaves an empty box. Keep `appearance: none` and draw the mark as text with `::before`:

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        "input[type=checkbox] { appearance: none }
         input[type=checkbox]::before { content: '( ) ' }
         input[type=checkbox]:checked::before { content: '(•) ' }",
    )?;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let check = dom.create_element("input");
    dom.set_attribute(check, "type", "checkbox")?;
    dom.append_child(root, check)?;
    let terminal = Terminal::new(TestBackend::new(10, 1))?;
    let mut app = App::with_backend(dom, sheet, terminal)?;
    app.draw_if_dirty()?;

    let row = |app: &mut App<TestBackend>| {
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);
        app.dom_mut().paint_dom(&mut buf, area);
        (0..10).map(|x| buf.cell(x, 0).unwrap().symbol().to_string()).collect::<String>()
    };
    assert!(row(&mut app).starts_with("( ) "));
    app.dom_mut().node_mut(check).click();
    app.draw_if_dirty()?;
    assert!(row(&mut app).starts_with("(•) "));
    Ok(())
}
```

**`em` sizes.** The font-relative units (`em`, `rem`, `ex`, `cap`, `ic`) and the absolute ones (`px`, `pt`, …) are dropped with a warning on anything that lays out — a guessed font size would scale browser CSS arbitrarily (DIVERGENCES §1, "Length units"). Write sizes in cells: a cell is about twice as tall as it is wide, so `1em` square is `width: 2; height: 1`, and `padding: 0.5em 1em` is `padding: 0 1`. `ch` (one column) and `lh` (one line) carry over unchanged; border widths, outline widths and shadows still take `px` / `em`, as they only pick a glyph weight.

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
