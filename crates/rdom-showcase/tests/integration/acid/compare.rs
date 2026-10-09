//! The colour-aware comparator (`specs/ACID.md` ground rules 2–3): paint a
//! page in a 120 × 50 headless `App`, crop each tile, and compare glyph,
//! foreground, background and modifiers cell by cell against the tile's
//! [`Reference`].
//!
//! The page is read back the way a terminal shows it: the `App`'s ANSI
//! output, with every SGR extension enabled (`SgrCapabilities::EXTENDED`),
//! replayed into a `VirtualScreen` — so the underline styles, the
//! underline colour and the overline are compared as emitted.

use rdom_showcase::demos::acid::{self, PAGE_HEIGHT, PAGE_WIDTH, TILES, Tile};
use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::runtime::app::App;
use rdom_tui::{Cell, SgrCapabilities, TuiDom, VirtualScreen};

use super::reference::{Reference, Style, describe, shows_fg, visible_modifiers};

/// The most differing cells a failure lists per tile.
const MAX_LISTED: usize = 40;

/// Paint `page` as the whole document of a 120 × 50 `App`, with the acid
/// sheets in their order: the main sheet, then the late one pushed on top.
pub fn paint_page(page: u8) -> VirtualScreen {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let page_root = acid::build_page(&mut dom, page);
    dom.append_child(root, page_root).unwrap();
    let backend = TestBackend::new(PAGE_WIDTH, PAGE_HEIGHT);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, acid::stylesheet(), terminal)
        .unwrap()
        .with_sgr_capabilities(SgrCapabilities::EXTENDED);
    app.push_stylesheet(acid::late_stylesheet());
    app.advance(0).unwrap();
    let mut screen = VirtualScreen::new(PAGE_WIDTH, PAGE_HEIGHT);
    screen.apply(app.terminal().backend().bytes());
    screen
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
    }
}

/// The style of `cell` as the comparison sees it on a cell showing
/// `glyph`: what a blank cannot show is cleared.
fn visible(glyph: &str, style: Style) -> Style {
    let modifier = visible_modifiers(glyph, style.modifier);
    let underlined = modifier.contains(rdom_tui::Modifier::UNDERLINED);
    Style {
        fg: if shows_fg(glyph, style.modifier) {
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
            let want_style = visible(&want.glyph, want.style);
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

/// Paint the reference's page and compare its tile.
pub fn check(reference: &'static Reference) -> TileReport {
    let screen = paint_page(tile(reference.tile).page);
    check_on(&screen, reference)
}

/// The crop of `t` on `screen` in the reference format, with a legend of
/// its own.
fn painted(screen: &VirtualScreen, t: &Tile) -> String {
    const KEYS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut styles: Vec<Style> = vec![Style::DEFAULT];
    let mut out = String::new();
    for y in t.y..t.y + t.h {
        let (mut glyphs, mut letters) = (String::new(), String::new());
        for x in t.x..t.x + t.w {
            let cell = screen.cell(x, y).expect("tile inside the page");
            if !cell.is_spacer() {
                glyphs.push_str(cell.symbol());
            }
            let glyph = if cell.is_spacer() { "" } else { cell.symbol() };
            let style = visible(glyph, actual_style(cell));
            let i = styles.iter().position(|s| *s == style).unwrap_or_else(|| {
                styles.push(style);
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
