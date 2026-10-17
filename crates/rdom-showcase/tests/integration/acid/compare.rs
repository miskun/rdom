//! The colour-aware comparator (`specs/ACID.md` ground rules 2–3): paint a
//! page in a 120 × 50 headless `App`, crop each tile, and compare glyph,
//! foreground, background, modifiers and hyperlink cell by cell against
//! the tile's [`Reference`].
//!
//! The page is read back the way a terminal shows it: the `App`'s ANSI
//! output, with every SGR extension enabled (`SgrCapabilities::EXTENDED`),
//! replayed into a `VirtualScreen` — so the underline styles, the
//! underline colour, the overline and the OSC 8 links are compared as
//! emitted. The replay is checked against the `App`'s own frame
//! ([`round_trip`]): the two must agree on every cell, and the stream may
//! hold no sequence the screen does not model, so the emulator cannot
//! agree with the backend only by sharing its blind spots.
//!
//! Outside the tiles a page shows only the tiles' labels ([`check_frame`]):
//! a tile whose `overflow: clip` fails, or whose content escapes onto a
//! label row or a gutter, fails the page's frame.
//!
//! Each page is painted once per test run ([`painted_page`]) — every tile
//! test, `report` and `summary` read the same paint — and a page whose
//! paint panics is reported as that, not as a crash of the run.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{LazyLock, OnceLock};

use rdom_showcase::demos::acid::{self, PAGE_HEIGHT, PAGE_WIDTH, TILES, Tile};
use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::runtime::app::App;
use rdom_tui::{Buffer, Cell, Color, SgrCapabilities, TuiDom, VirtualScreen};

use super::coverage::Used;
use super::reference::{Reference, Style, describe, parse_style, shows_fg, visible_modifiers};

/// The most differing cells a failure lists per tile.
const MAX_LISTED: usize = 40;

/// What an acid `App` is configured with before its first frame (a step's
/// caret blink rate, pointer-shape protocol).
pub type Configure = fn(App<TestBackend>) -> App<TestBackend>;

/// `page` as the whole document of a fresh 120 × 50 `App`, with the acid
/// sheets in their order — the main sheet, then the late one pushed on top
/// — after `configure`; its first frame drawn at the clock's 0 and the
/// tiles' load scripts run (`Tile::script`), the frame they change not yet
/// drawn. Stage 1's paint and every stage-2 session open a page this way.
pub fn open_page(page: u8, configure: Option<Configure>) -> App<TestBackend> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let page_root = acid::build_page(&mut dom, page);
    dom.append_child(root, page_root).unwrap();
    let terminal = Terminal::new(TestBackend::new(PAGE_WIDTH, PAGE_HEIGHT)).unwrap();
    let mut app = App::with_backend(dom, acid::stylesheet(), terminal)
        .unwrap()
        .with_sgr_capabilities(SgrCapabilities::EXTENDED)
        .with_import_loader(acid::import_loader());
    if let Some(configure) = configure {
        app = configure(app);
    }
    app.push_stylesheet(acid::late_stylesheet());
    app.advance(0).unwrap();
    acid::run_scripts(app.dom_mut(), page);
    app
}

/// A page as stage 1 paints it.
pub struct PagePaint {
    /// The `App`'s output replayed.
    pub screen: VirtualScreen,
    /// Where the replay and the `App`'s frame disagree ([`round_trip`]).
    pub round_trip: Vec<String>,
    /// What the tiles' rules styled (the coverage test).
    pub used: Used,
}

/// Paint `page` ([`open_page`], then the frame its scripts changed) and
/// replay what the `App` wrote.
pub fn paint_page(page: u8) -> PagePaint {
    let mut app = open_page(page, None);
    app.advance(0).unwrap();
    let mut screen = VirtualScreen::new(PAGE_WIDTH, PAGE_HEIGHT);
    screen.apply(app.terminal().backend().bytes());
    let round_trip = round_trip(app.terminal().buffer(), &screen);
    let mut used = Used::default();
    used.record(&app);
    PagePaint {
        screen,
        round_trip,
        used,
    }
}

/// Every page's paint, once per test run.
static PAGES: LazyLock<Vec<OnceLock<Result<PagePaint, String>>>> =
    LazyLock::new(|| (0..acid::page_count()).map(|_| OnceLock::new()).collect());

/// `page` painted ([`paint_page`]) the first time it is asked for and kept
/// for the run; `Err` with the panic's message when painting it panicked.
pub fn painted_page(page: u8) -> Result<&'static PagePaint, &'static str> {
    PAGES[usize::from(page) - 1]
        .get_or_init(|| {
            catch_unwind(AssertUnwindSafe(|| paint_page(page)))
                .map_err(|panic| format!("painting page {page} panicked: {}", panic_text(&*panic)))
        })
        .as_ref()
        .map_err(String::as_str)
}

/// A caught panic's message.
pub fn panic_text(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default()
}

/// Where `screen`, a replay of the backend's output, differs from `frame`,
/// the frame the `App` drew — glyph (or wide-glyph spacer), colours,
/// modifiers, underline colour, link — and each sequence the replay did
/// not model.
pub fn round_trip(frame: &Buffer, screen: &VirtualScreen) -> Vec<String> {
    let mut out: Vec<String> = screen
        .unsupported()
        .iter()
        .map(|s| format!("unmodelled sequence {s}"))
        .collect();
    let key = |c: &Cell| {
        (
            c.is_spacer(),
            c.symbol().to_string(),
            c.fg,
            c.bg,
            c.modifier,
            c.underline_color,
            c.link.clone(),
        )
    };
    for y in 0..PAGE_HEIGHT {
        for x in 0..PAGE_WIDTH {
            let (Some(a), Some(b)) = (frame.cell(x, y), screen.cell(x, y)) else {
                out.push(format!("({x}, {y}) outside the frame or the screen"));
                continue;
            };
            if key(a) != key(b) {
                out.push(format!(
                    "({x}, {y}) drawn {:?} | replayed {:?}",
                    key(a),
                    key(b)
                ));
            }
        }
    }
    out
}

/// What a page shows outside its tiles: each tile's label — `"<id>
/// <title>"`, clipped to the tile's width, grey — on the row above it, and
/// blanks everywhere else, or the styles a covering tile gives them there
/// (`refs::FRAMES`). One line per differing cell.
pub fn check_frame(page: u8, screen: &VirtualScreen) -> Vec<String> {
    let tiles: Vec<&Tile> = acid::tiles_on(page).collect();
    let (label_style, blank_style) = match super::refs::FRAMES.iter().find(|f| f.page == page) {
        Some(f) => match (parse_style(f.label), parse_style(f.blank)) {
            (Ok(l), Ok(b)) => (l, b),
            (Err(e), _) | (_, Err(e)) => {
                return vec![format!("page {page}'s frame (tile {}): {e}", f.tile)];
            }
        },
        None => (
            Style {
                fg: Color::Rgb(128, 128, 128),
                ..Style::DEFAULT
            },
            Style::DEFAULT,
        ),
    };
    let mut out = Vec::new();
    for y in 0..PAGE_HEIGHT {
        for x in 0..PAGE_WIDTH {
            let inside = |t: &&Tile| x >= t.x && x < t.x + t.w && y >= t.y && y < t.y + t.h;
            if tiles.iter().any(inside) {
                continue;
            }
            let label = tiles
                .iter()
                .find(|t| y + 1 == t.y && x >= t.x && x < t.x + t.w)
                .map(|t| {
                    let text: Vec<char> = format!("{} {}", t.id, t.title).chars().collect();
                    text.get(usize::from(x - t.x)).copied().unwrap_or(' ')
                });
            let want_glyph = label.unwrap_or(' ').to_string();
            let want = if label.is_some() {
                label_style.clone()
            } else {
                blank_style.clone()
            };
            let cell = screen.cell(x, y).expect("on the page");
            let got_glyph = if cell.is_spacer() { "" } else { cell.symbol() };
            let (w, g) = (
                visible(&want_glyph, want.clone()),
                visible(&want_glyph, actual_style(cell)),
            );
            if got_glyph != want_glyph || w != g {
                let place = if label.is_some() { "label" } else { "gutter" };
                out.push(format!(
                    "({x}, {y}) {place}: expected {want_glyph:?} {} | painted {got_glyph:?} {}",
                    describe(&w),
                    describe(&actual_style(cell)),
                ));
            }
        }
    }
    out
}

/// A page's frame: the round trip and what shows outside its tiles.
pub struct FrameReport {
    pub page: u8,
    /// Why the page could not be checked (its paint panicked).
    pub error: Option<String>,
    pub round_trip: Vec<String>,
    pub outside: Vec<String>,
}

impl FrameReport {
    pub fn of(page: u8) -> FrameReport {
        match painted_page(page) {
            Ok(p) => FrameReport {
                page,
                error: None,
                round_trip: p.round_trip.clone(),
                outside: check_frame(page, &p.screen),
            },
            Err(e) => FrameReport {
                page,
                error: Some(e.to_string()),
                round_trip: Vec::new(),
                outside: Vec::new(),
            },
        }
    }

    pub fn passed(&self) -> bool {
        self.error.is_none() && self.round_trip.is_empty() && self.outside.is_empty()
    }

    /// The report printed for a failing frame.
    pub fn failure(&self) -> String {
        let mut out = format!("page {} frame\n", self.page);
        if let Some(e) = &self.error {
            out.push_str(&format!("  {e}\n"));
        }
        for (what, lines) in [
            ("replay differs from the frame drawn", &self.round_trip),
            ("cells outside the tiles differ", &self.outside),
        ] {
            if !lines.is_empty() {
                out.push_str(&format!("  {} {what}:\n", lines.len()));
                for d in lines.iter().take(MAX_LISTED) {
                    out.push_str(&format!("    {d}\n"));
                }
                if lines.len() > MAX_LISTED {
                    out.push_str(&format!("    … {} more\n", lines.len() - MAX_LISTED));
                }
            }
        }
        out
    }
}

/// One tile's result.
pub struct TileReport {
    pub tile: &'static Tile,
    pub spec: &'static [&'static str],
    /// Why the comparison could not run (a malformed reference).
    pub error: Option<String>,
    /// One line per differing cell.
    pub diffs: Vec<String>,
    /// The painted crop in the reference format, for diagnosis — never to
    /// be pasted in as a reference (ground rule 1).
    pub painted: String,
}

impl TileReport {
    pub fn passed(&self) -> bool {
        self.error.is_none() && self.diffs.is_empty()
    }

    /// The report printed for a failing tile.
    pub fn failure(&self) -> String {
        let t = self.tile;
        let mut out = format!(
            "tile {} \"{}\" (page {}, at {},{} size {}×{}) — spec: {}\n",
            t.id,
            t.title,
            t.page,
            t.x,
            t.y,
            t.w,
            t.h,
            self.spec.join("; ")
        );
        if let Some(e) = &self.error {
            out.push_str(&format!("  reference error: {e}\n"));
        }
        if !self.diffs.is_empty() {
            out.push_str(&format!("  {} cells differ:\n", self.diffs.len()));
            for d in self.diffs.iter().take(MAX_LISTED) {
                out.push_str(&format!("    {d}\n"));
            }
            if self.diffs.len() > MAX_LISTED {
                out.push_str(&format!("    … {} more\n", self.diffs.len() - MAX_LISTED));
            }
        }
        out.push_str("  painted (diagnosis only):\n");
        out.push_str(&self.painted);
        out
    }
}

fn tile(id: &str) -> &'static Tile {
    TILES
        .iter()
        .copied()
        .find(|t| t.id == id)
        .unwrap_or_else(|| panic!("no acid tile {id:?}"))
}

fn actual_style(cell: &Cell) -> Style {
    Style {
        fg: cell.fg,
        bg: cell.bg,
        underline_color: cell.underline_color,
        modifier: cell.modifier,
        link: cell.link.as_deref().map(str::to_string),
    }
}

/// The style of `cell` as the comparison sees it on a cell showing
/// `glyph`: what a blank cannot show is cleared.
fn visible(glyph: &str, style: Style) -> Style {
    let modifier = visible_modifiers(glyph, style.modifier);
    let underlined = modifier.contains(rdom_tui::Modifier::UNDERLINED);
    Style {
        fg: if shows_fg(glyph, style.modifier, style.underline_color) {
            style.fg
        } else {
            rdom_tui::Color::Reset
        },
        bg: style.bg,
        underline_color: if underlined {
            style.underline_color
        } else {
            rdom_tui::Color::Reset
        },
        modifier,
        link: style.link,
    }
}

/// Compare `reference` against its tile on `screen`.
pub fn check_on(screen: &VirtualScreen, reference: &'static Reference) -> TileReport {
    let t = tile(reference.tile);
    let mut report = TileReport {
        tile: t,
        spec: reference.spec,
        error: None,
        diffs: Vec::new(),
        painted: painted(screen, t),
    };
    let parsed = match reference.parse(t.w, t.h) {
        Ok(p) => p,
        Err(e) => {
            report.error = Some(e);
            return report;
        }
    };
    for (dy, row) in parsed.rows.iter().enumerate() {
        for (dx, want) in row.iter().enumerate() {
            let (x, y) = (t.x + dx as u16, t.y + dy as u16);
            let cell = screen.cell(x, y).expect("tile inside the page");
            let got_glyph = if cell.is_spacer() { "" } else { cell.symbol() };
            let want_style = visible(&want.glyph, want.style.clone());
            let got_style = visible(&want.glyph, actual_style(cell));
            if got_glyph != want.glyph || got_style != want_style {
                report.diffs.push(format!(
                    "({dx}, {dy}) expected {:?} {} | painted {:?} {}",
                    want.glyph,
                    describe(&want_style),
                    got_glyph,
                    describe(&actual_style(cell)),
                ));
            }
        }
    }
    report
}

/// Compare the reference's tile on its page, painted once per run
/// ([`painted_page`]).
pub fn check(reference: &'static Reference) -> TileReport {
    let t = tile(reference.tile);
    match painted_page(t.page) {
        Ok(p) => check_on(&p.screen, reference),
        Err(e) => TileReport {
            tile: t,
            spec: reference.spec,
            error: Some(e.to_string()),
            diffs: Vec::new(),
            painted: String::new(),
        },
    }
}

/// The crop of `t` on `screen` in the reference format, with a legend of
/// its own — also what a stage-2 step compares to see whether a tile it
/// does not name changed.
pub fn painted(screen: &VirtualScreen, t: &Tile) -> String {
    const KEYS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut styles: Vec<Style> = vec![Style::DEFAULT];
    let mut out = String::new();
    for y in t.y..t.y + t.h {
        let (mut glyphs, mut letters) = (String::new(), String::new());
        for x in t.x..t.x + t.w {
            // A screen resized smaller than the page shows none of it.
            let Some(cell) = screen.cell(x, y) else {
                glyphs.push('×');
                letters.push('.');
                continue;
            };
            if !cell.is_spacer() {
                glyphs.push_str(cell.symbol());
            }
            let glyph = if cell.is_spacer() { "" } else { cell.symbol() };
            let style = visible(glyph, actual_style(cell));
            let i = styles.iter().position(|s| *s == style).unwrap_or_else(|| {
                styles.push(style.clone());
                styles.len() - 1
            });
            letters.push(if i == 0 {
                '.'
            } else {
                char::from(KEYS[(i - 1).min(KEYS.len() - 1)])
            });
        }
        out.push_str(&format!("    |{glyphs}|\n    |{letters}|\n"));
    }
    for (i, s) in styles.iter().enumerate().skip(1) {
        let key = char::from(KEYS[(i - 1).min(KEYS.len() - 1)]);
        out.push_str(&format!("    {key} = {}\n", describe(s)));
    }
    out
}
