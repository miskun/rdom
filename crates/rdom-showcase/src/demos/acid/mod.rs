//! Acid — rdom's acid test page (`specs/ACID.md`).
//!
//! Many features rendered together, one *interaction* per tile, on
//! fixed 120 × 50 pages: the reference each tile is compared against is
//! derived by hand from the spec (`crates/rdom-showcase/tests/integration/
//! acid/`), so a tile that looks wrong is a bug in rdom, not in the page.
//!
//! ## Pages
//!
//! ACID.md's 26 tiles do not fit one 120 × 50 screen, so the acid test is
//! a sequence of **pages**, each exactly 120 × 50 — the viewport the
//! references are derived for. Every [`Tile`] names its page and its
//! rectangle there. The tests lay out and paint one page at a time, as the
//! whole document of a 120 × 50 headless `App`; the showcase entry
//! (`Built-ins → Acid`) stacks every page in its view pane, where the
//! pane, not the terminal, is the viewport — a tile that reads the
//! viewport (`position: fixed`, the viewport units, `@media`) is right only
//! on its own page at 120 × 50: `cargo run -p rdom-showcase --example acid
//! -- <page>` runs one page full screen. The showcase entry runs no
//! [`Tile::script`] (a page's load handler, which the tests and the example
//! run after the first frame), so a scripted tile shows its initial state
//! there.
//!
//! ## Sheets
//!
//! Every tile's rules go into one author sheet ([`css`]), each scoped by
//! its tile's class; [`late_css`] is a second sheet the tests push after
//! it, for the cascade-order contests. The showcase entry has one sheet
//! slot, so there the late rules follow the main ones in a single sheet
//! (the same cascade order: a later sheet and later rules in one sheet
//! win the same contests).

use std::io;
use std::sync::LazyLock;

use rdom_parser::parse_into;
use rdom_tui::{App, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

mod tile;
pub mod tiles;

pub use tile::Tile;
pub use tiles::TILES;

/// The pages' size: the viewport every reference is derived for.
pub const PAGE_WIDTH: u16 = 120;
/// See [`PAGE_WIDTH`].
pub const PAGE_HEIGHT: u16 = 50;

/// The page frame: the page box and every tile's label and box.
const FRAME_CSS: &str = r#"
.acid-page { position: relative; width: 120; height: 50; }
.acid-label {
  position: absolute;
  height: 1;
  overflow: clip;
  white-space: nowrap;
  color: rgb(128, 128, 128);
}
.acid-tile { position: absolute; overflow: clip; z-index: 0; }
"#;

static CSS: LazyLock<String> = LazyLock::new(|| {
    let mut css = String::from(FRAME_CSS);
    for tile in TILES {
        css.push_str(tile.css);
    }
    css
});

static LATE_CSS: LazyLock<String> = LazyLock::new(|| TILES.iter().map(|t| t.late_css).collect());

static MARKUP: LazyLock<String> = LazyLock::new(|| {
    (1..=page_count())
        .map(page_markup)
        .collect::<Vec<_>>()
        .join("\n")
});

/// The main author sheet's text: the frame and every tile's rules.
pub fn css() -> &'static str {
    &CSS
}

/// The later author sheet's text ([`Tile::late_css`] of every tile).
pub fn late_css() -> &'static str {
    &LATE_CSS
}

/// How many pages the tiles fill.
pub fn page_count() -> u8 {
    TILES.iter().map(|t| t.page).max().unwrap_or(1)
}

/// The tiles on `page`, in [`TILES`] order.
pub fn tiles_on(page: u8) -> impl Iterator<Item = &'static Tile> {
    TILES.iter().copied().filter(move |t| t.page == page)
}

/// The markup of `page`: the page box holding its tiles' labels and boxes.
pub fn page_markup(page: u8) -> String {
    let mut out = format!(r#"<div class="acid-page acid-page-{page}">"#);
    for t in tiles_on(page) {
        out.push_str(&format!(
            r#"<div class="acid-label" style="left: {x}; top: {ly}; width: {w}">{id} {title}</div><div class="acid-tile {class}" style="left: {x}; top: {y}; width: {w}; height: {h}">{markup}</div>"#,
            x = t.x,
            ly = t.y - 1,
            y = t.y,
            w = t.w,
            h = t.h,
            id = t.id,
            title = t.title,
            class = t.class,
            markup = t.markup.trim(),
        ));
    }
    out.push_str("</div>");
    out
}

/// Build `page` alone, unattached: the tree the acid tests paint.
pub fn build_page(dom: &mut TuiDom, page: u8) -> NodeId {
    let host = dom.create_element("div");
    dom.set_attribute(host, "class", "acid").unwrap();
    parse_into(dom, &page_markup(page), host).expect("acid page parses");
    run_setups(dom, host, tiles_on(page));
    host
}

/// Build every page, stacked, unattached: the showcase entry.
pub fn build(dom: &mut TuiDom) -> NodeId {
    let host = dom.create_element("div");
    dom.set_attribute(host, "class", "acid").unwrap();
    parse_into(dom, &MARKUP, host).expect("acid pages parse");
    run_setups(dom, host, TILES.iter().copied());
    host
}

/// Run each tile's [`Tile::setup`] on its box under `host`.
fn run_setups<'a>(dom: &mut TuiDom, host: NodeId, tiles: impl Iterator<Item = &'a Tile>) {
    for t in tiles {
        if let Some(setup) = t.setup {
            setup(dom, tile_box(dom, host, t));
        }
    }
}

/// Run the [`Tile::script`] of every tile on `page`, built under the
/// document's root — after the page's first frame.
pub fn run_scripts(dom: &mut TuiDom, page: u8) {
    let root = dom.root();
    for t in tiles_on(page) {
        if let Some(script) = t.script {
            script(dom, tile_box(dom, root, t));
        }
    }
}

/// The box of tile `t` under `host`.
fn tile_box(dom: &TuiDom, host: NodeId, t: &Tile) -> NodeId {
    dom.query_selector_in(host, &format!(".{}", t.class))
        .expect("a valid class selector")
        .expect("the tile's box is on its page")
}

/// The main sheet ([`css`]), with the UA defaults.
pub fn stylesheet() -> Stylesheet {
    rdom_css::from_css(css())
}

/// The later sheet ([`late_css`]), without the UA defaults: it is pushed
/// on top of [`stylesheet`].
pub fn late_stylesheet() -> Stylesheet {
    rdom_css::parse(late_css()).stylesheet
}

/// Run `page` full screen (`cargo run -p rdom-showcase --example acid --
/// <page>`), with both sheets as the tests push them.
pub fn run_standalone(page: u8) -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let page_root = build_page(&mut dom, page);
    dom.append_child(root, page_root).unwrap();
    let mut app = App::new(dom, stylesheet())?;
    app.push_stylesheet(late_stylesheet());
    // The scripts need the first frame's layout: queue them for the
    // loop's second iteration, after it has drawn the page once.
    let handle = app.handle();
    let next = handle.clone();
    handle.inject(move |_| {
        next.inject(move |ctx| run_scripts(ctx.dom, page));
        next.request_redraw();
    });
    app.run()
}

static SOURCE_CSS: LazyLock<String> = LazyLock::new(|| format!("{}{}", css(), late_css()));

/// The showcase entry (`Built-ins → Acid`).
pub struct Acid;

impl Demo for Acid {
    fn slug(&self) -> &'static str {
        "builtins/acid"
    }

    fn title(&self) -> &'static str {
        "Acid"
    }

    fn category(&self) -> Category {
        Category::BuiltIns
    }

    fn build(&self, dom: &mut TuiDom) -> NodeId {
        build(dom)
    }

    fn stylesheet(&self) -> Stylesheet {
        rdom_css::from_css(&SOURCE_CSS)
    }

    fn source(&self) -> Source {
        Source {
            markup: &MARKUP,
            css: &SOURCE_CSS,
        }
    }
}
