# rdom-tui recipes

Worked examples of rdom-tui's CSS and runtime, each a complete program that is compiled and run as a doctest (`RecipesDoctests` in `src/lib.rs`), so it keeps working as the API moves. The [README](../README.md) has the overview, the quick start and the "CSS at a glance" table that links here; [`DIVERGENCES.md`](../../../specs/DIVERGENCES.md) lists every deliberate departure from the web platform.

- [Grid layout](#grid-layout)
- [Responsive layout: `@media` and container queries](#responsive-layout-media-and-container-queries)
- [Tables](#tables)
- [Floats and text overflow](#floats-and-text-overflow)
- [Pseudo-elements and `content`](#pseudo-elements-and-content)
- [Lists, counters and generated content](#lists-counters-and-generated-content)
- [Custom highlights: search results](#custom-highlights-search-results)
- [Custom properties and `var()`](#custom-properties-and-var)
- [Inline formatting](#inline-formatting)
- [Interaction state: `:hover`, `:active` and `:focus`](#interaction-state-hover-active-and-focus)
- [Form states](#form-states)
- [Popovers and the top layer](#popovers-and-the-top-layer)
- [Multi-column layout](#multi-column-layout)
- [Transforms, filters, blending and clipping](#transforms-filters-blending-and-clipping)
- [Transitions and animations](#transitions-and-animations)
- [Incremental re-cascade](#incremental-re-cascade)
- [`!important` ladder](#important-ladder)

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

`popover` elements (HTML §6.12) and modal dialogs render in the top layer: above every `z-index`, outside every ancestor's `overflow`, centred in the viewport by the UA sheet, on a `Canvas` background that hides the page under them. A `popovertarget` button toggles its popover; a click outside an auto popover, or Esc, closes it (light dismiss); `runtime::builtins::popover` has `show_popover` / `hide_popover` / `toggle_popover` for script. Tab moves through a popover in tree order, so a popover placed away from its invoker should give the control to start at `autofocus`.

### Anchored popovers, pickers and tooltips

A popover's invoker is its implicit anchor (CSS Anchor Positioning 1 §2.3): `position-area` places it against the button that opened it, in the 3 × 3 grid of the button's and the screen's edges — `bottom span-right` under the button, running right from its left edge. Any absolutely positioned box can name an anchor instead (`anchor-name` on the anchor, `position-anchor` on the box), and `anchor()` / `anchor-size()` in its insets and sizes read the anchor's edges and size.

```rust
use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rdom_tui::prelude::*;
use rdom_tui::runtime::builtins::popover;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict("[popover] { position-area: bottom span-right }")?;
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
    dom.set_attribute(button, "popovertarget", "m")?;
    dom.set_attribute(menu, "id", "m")?;
    dom.set_attribute(menu, "popover", "")?;

    let terminal = Terminal::new(TestBackend::new(24, 8))?;
    let mut app = App::with_backend(dom, sheet, terminal)?;
    app.draw_if_dirty()?;

    // The invoker opens it, under itself.
    app.dom_mut().node_mut(button).click();
    assert!(popover::is_showing(app.dom(), menu));
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

A picker — a custom drop-down list — opens below its button and, where the screen ends, above it: `position-try-fallbacks: flip-block` is tried when the box would overflow below (§4). (rdom's native `<select>` does the same with its own option list.)

```rust
use rdom_tui::prelude::*;
use rdom_tui::runtime::builtins::popover;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        "body { margin: 0 } .spacer { height: 5 }
         [popover] { position-area: bottom span-right; position-try-fallbacks: flip-block }",
    )?;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let body = dom.create_element("body");
    dom.append_child(root, body)?;
    let spacer = dom.create_element("div");
    dom.set_attribute(spacer, "class", "spacer")?;
    dom.append_child(body, spacer)?;
    let button = dom.create_element("button");
    let label = dom.create_text_node("Size: M");
    dom.append_child(button, label)?;
    dom.append_child(body, button)?;
    let list = dom.create_element("div");
    dom.set_attribute(list, "popover", "")?;
    for size in ["S", "M", "L"] {
        let option = dom.create_element("div");
        let text = dom.create_text_node(size);
        dom.append_child(option, text)?;
        dom.append_child(list, option)?;
    }
    dom.append_child(body, list)?;
    popover::show_popover_from(&mut dom, list, Some(button))?;
    dom.set_viewport(Viewport::new(20, 8));
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 20, 8));

    // Three options and a border are five rows: no room below row 5, so
    // the list opens above its button.
    let b = dom.node(button).bounding_rect().unwrap();
    let r = dom.node(list).bounding_rect().unwrap();
    assert_eq!((r.x, r.y + i32::from(r.height)), (b.x, b.y));
    Ok(())
}
```

A tooltip names its anchor and sits above it, centred on it (`position-area: top` spans the columns, so the box is `anchor-center`ed); `position-visibility: anchors-visible` (the initial value) hides it while its anchor is scrolled out of view.

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        "body { margin: 0 } .save { anchor-name: --save; margin: 2 0 0 6 }
         .tip { position: absolute; position-anchor: --save; position-area: top }",
    )?;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let body = dom.create_element("body");
    dom.append_child(root, body)?;
    let save = dom.create_element("button");
    dom.set_attribute(save, "class", "save")?;
    let text = dom.create_text_node("Save");
    dom.append_child(save, text)?;
    dom.append_child(body, save)?;
    let tip = dom.create_element("div");
    dom.set_attribute(tip, "class", "tip")?;
    let hint = dom.create_text_node("Ctrl-S");
    dom.append_child(tip, hint)?;
    dom.append_child(body, tip)?;
    dom.set_viewport(Viewport::new(30, 6));
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 6));

    let b = dom.node(save).bounding_rect().unwrap();
    let t = dom.node(tip).bounding_rect().unwrap();
    // On the row above the button, centred on it.
    assert_eq!(t.y + i32::from(t.height), b.y);
    let centre = |x: i32, w: u16| 2 * x + i32::from(w);
    assert!((centre(t.x, t.width) - centre(b.x, b.width)).abs() <= 1);
    Ok(())
}
```

## Multi-column layout

`columns`, `column-count` and `column-width` flow a box's content through column boxes side by side (CSS Multi-column 1), in whole cells: an `auto` height balances the columns, a fixed one fills them in turn (`column-fill`). `column-rule` draws border glyphs in the gaps and joins the box's own border; a `column-span: all` child spans every column and splits the content above and below it into column sets; `break-before` / `-after` / `-inside`, `orphans` / `widows` and `box-decoration-break` control where and how a box splits (CSS Fragmentation 3). `column-width: 15em` and `columns: 200px` select a count at 8px a column, as breakpoints do.

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        "
        body  { margin: 0 }
        .cols { column-count: 2; column-gap: 3; width: 13; column-rule: solid }
        h2    { margin: 0; column-span: all }
        ",
    )?;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(
        &mut dom,
        r#"<body><div class="cols" id="m">
             <div>a1</div><div>a2</div>
             <h2 id="s">Title</h2>
             <div>b1</div><div>b2</div><div>b3</div>
           </div></body>"#,
        root,
    )?;

    let area = Rect::new(0, 0, 13, 4);
    dom.cascade(&sheet);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);

    let row = |y: u16| -> String { (0..13).map(|x| buf.cell(x, y).unwrap().symbol()).collect() };
    // Two balanced column sets, a rule in each gap, the spanner between.
    assert_eq!(row(0), "a1    │ a2   ");
    assert_eq!(row(1), "Title        ");
    assert_eq!(row(2), "b1    │ b3   ");
    assert_eq!(row(3), "b2    │      ");

    // The spanner is as wide as the container.
    let title = dom.get_element_by_id("s").unwrap();
    let r = dom.node(title).layout_rect().unwrap();
    assert_eq!((r.x, r.y, r.width), (0, 1, 13));
    Ok(())
}
```

A box split across two columns is drawn and hit as its fragments: `client_rects()` lists one rect per column it reaches, and `layout_rect()` is their bounding box. Columns are whole cells wide; `column-count` is capped at one-cell columns.

## Transforms, filters, blending and clipping

The effects properties take effect on the cell grid (CSS Transforms 1 / 2, Filter Effects 1 / 2, Compositing and Blending 1, CSS Masking 1):

- **`translate` and `transform`'s translate functions** move a box and its subtree by whole cells, after layout — paint, hit-testing and scrollable overflow follow, the boxes around it do not. Percentages are of the border box, so the web's centring idiom works. Rotation, scaling and skews parse, cascade and animate, and draw nothing. Any transform makes the box a stacking context and the containing block of its fixed and absolutely positioned descendants.
- **`filter` and `backdrop-filter`** map the colors of every cell the element paints (or of the cells behind it) through the color-matrix functions — `grayscale()`, `sepia()`, `saturate()`, `hue-rotate()`, `invert()`, `brightness()`, `contrast()` — with `opacity()` as group opacity and `drop-shadow()` a whole-cell shade; `blur()` and `url()` are inert.
- **`mix-blend-mode`** blends what the element paints with what lies beneath, within the nearest `isolation: isolate` group or stacking context.
- **`clip-path`** hides the cells outside `inset()`, `circle()`, `ellipse()` or `polygon()` — a cell is in when its centre is — for paint and for the pointer; the legacy `clip: rect()` clips an absolutely positioned box (the visually-hidden pattern).

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        "
        body   { margin: 0 }
        .stage { position: relative; width: 30; height: 10 }
        .card  { position: absolute; top: 50%; left: 50%; width: 10; height: 4;
                 translate: -50% -50%; background-color: rgb(255, 0, 0) }
        .off   { filter: grayscale(1) }
        .half  { clip-path: inset(0 0 50% 0) }
        ",
    )?;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(
        &mut dom,
        r#"<body><div class="stage">
             <div class="card" id="card"></div>
           </div></body>"#,
        root,
    )?;
    let area = Rect::new(0, 0, 30, 10);
    let card = dom.get_element_by_id("card").unwrap();

    // Centred: its top-left corner at the stage's middle, moved back by
    // half its own size.
    dom.cascade(&sheet);
    dom.layout_dom(area);
    let r = dom.node(card).layout_rect().unwrap();
    assert_eq!((r.x, r.y, r.width, r.height), (10, 3, 10, 4));

    // Greyed out: red through `grayscale(1)`'s matrix.
    dom.node_mut(card).add_class("off")?;
    dom.cascade(&sheet);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    assert_eq!(buf.cell(12, 4).unwrap().bg, Color::Rgb(54, 54, 54));

    // Half clipped away: the bottom two rows paint nothing of the card.
    dom.node_mut(card).add_class("half")?;
    dom.cascade(&sheet);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    assert_eq!(buf.cell(12, 4).unwrap().bg, Color::Rgb(54, 54, 54));
    assert_ne!(buf.cell(12, 5).unwrap().bg, Color::Rgb(54, 54, 54));
    Ok(())
}
```

A pasted loading spinner (`@keyframes spin { to { transform: rotate(1turn) } }`) runs and asks for no frames, since rotation draws nothing; a `translate` animation lays out only on the frames its whole-cell offset changes.

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

### A slide-in panel

`translate` and `transform: translate()` move a box by whole cells after layout — its paint, hit-testing and scrollable overflow go with it, the boxes around it stay (CSS Transforms 1 §3) — and they transition like any length. The web's slide-in drawer carries over as written:

```rust
use rdom_tui::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sheet = rdom_css::from_css_strict(
        ".panel { width: 10; height: 3; background-color: rgb(0, 0, 128);
                  translate: -100% 0; transition: translate 100ms linear }
         .panel.open { translate: 0 }",
    )?;
    let mut dom: TuiDom = TuiDom::new();
    let body = dom.create_element("body");
    dom.append_child(dom.root(), body)?;
    let panel = dom.create_element("div");
    dom.set_attribute(panel, "class", "panel")?;
    let after = dom.create_element("p");
    dom.append_child(body, panel)?;
    dom.append_child(body, after)?;
    let terminal = Terminal::new(TestBackend::new(20, 5))?;
    let mut app = App::with_backend(dom, sheet, terminal)?;
    app.advance(0)?;

    let x = |app: &App<TestBackend>| app.dom().node(panel).bounding_rect().unwrap().x;
    assert_eq!(x(&app), -10, "off-screen: -100% of its own width");
    app.dom_mut().set_attribute(panel, "class", "panel open")?;
    app.advance(0)?; // the transition starts
    app.advance(50)?;
    assert_eq!(x(&app), -5);
    app.advance(50)?;
    assert_eq!(x(&app), 0);
    // The paragraph after it lays out where it did: a translation moves
    // no other box.
    assert_eq!(app.dom().node(after).bounding_rect().unwrap().y, 3);
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

**`em` sizes.** The font-relative units (`em`, `rem`, `ex`, `cap`, `ic`) and the absolute ones (`px`, `pt`, …) are dropped with a warning on anything that lays out — a guessed font size would scale browser CSS arbitrarily (DIVERGENCES §1, "Length units"). Write sizes in cells: a cell is about twice as tall as it is wide, so `1em` square is `width: 2; height: 1`, and `padding: 0.5em 1em` is `padding: 0 1`. `ch` (one column) and `lh` (one line) carry over unchanged; border widths, outline widths and shadows still take `px` / `em`, as they only pick a glyph weight, and so does `column-width` (`columns: 15em`), which only picks how many columns fit — at 8px a column, as a `@media` breakpoint.

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
