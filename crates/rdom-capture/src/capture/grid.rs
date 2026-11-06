//! The grid layer of a capture: what every cell shows, as run-length rows.
//!
//! A row is a list of [`Run`]s — consecutive cells with the same colors,
//! attributes and link — that tile the row exactly: the first starts at
//! `x = 0`, each starts where the previous one ends, and their widths add
//! up to the grid's width. A wide glyph's trailing cell belongs to the run
//! of its glyph, so `w` counts cells, not characters.
//!
//! Colors are what rdom painted (`fg`, `bg`); where the environment's
//! color depth shows another color, `fg_shown` / `bg_shown` carry the
//! color the terminal is sent.

use rdom_tui::{Buffer, Cell, Color, ColorDepth, Modifier};
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::color::ColorValue;

/// What every cell of the viewport shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Grid {
    pub width: u16,
    pub height: u16,
    /// One list of runs per row, top to bottom.
    pub rows: Vec<Vec<Run>>,
}

/// Consecutive cells of a row with the same colors, attributes and link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Run {
    /// The first cell's column.
    pub x: u16,
    /// How many cells the run covers (a wide glyph covers two).
    pub w: u16,
    /// The glyphs, one per cell or wide glyph; a blank cell is a space.
    pub text: String,
    /// Each cell's glyph (a wide glyph's trailing cell: `""`), present
    /// only where `text` alone does not give the cells back — a cell
    /// holding a lone combining mark or regional indicator, which joins
    /// its neighbour when the text is split into graphemes again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glyphs: Option<Vec<String>>,
    /// The glyph color rdom painted.
    pub fg: ColorValue,
    /// The background rdom painted.
    pub bg: ColorValue,
    /// The glyph color the terminal is sent, where the color depth
    /// changes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fg_shown: Option<ColorValue>,
    /// The background the terminal is sent, where the color depth
    /// changes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bg_shown: Option<ColorValue>,
    /// The text attributes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mods: Vec<Attr>,
    /// The underline's own color, where it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ul: Option<ColorValue>,
    /// The hyperlink target (OSC 8).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

impl Run {
    /// The glyph and background colors the run shows: the colors sent to
    /// the terminal where the color depth changed them, else as painted.
    pub fn shown_colors(&self) -> (ColorValue, ColorValue) {
        (
            self.fg_shown.unwrap_or(self.fg),
            self.bg_shown.unwrap_or(self.bg),
        )
    }

    /// Each cell's glyph, in order: a blank cell is `" "`, a wide glyph's
    /// trailing cell `""`.
    pub fn cells(&self) -> Vec<&str> {
        match &self.glyphs {
            Some(glyphs) => glyphs.iter().map(String::as_str).collect(),
            None => split_cells(&self.text, self.w),
        }
    }
}

/// A text attribute of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Attr {
    Bold,
    Italic,
    Underline,
    UnderlineDouble,
    UnderlineCurly,
    UnderlineDotted,
    UnderlineDashed,
    Overline,
    Strikethrough,
    BlinkSlow,
    BlinkRapid,
    Hidden,
}

impl Attr {
    /// Every attribute with the modifier bit it reads, in output order.
    const ALL: [(Attr, Modifier); 12] = [
        (Attr::Bold, Modifier::BOLD),
        (Attr::Italic, Modifier::ITALIC),
        (Attr::Underline, Modifier::UNDERLINED),
        (Attr::UnderlineDouble, Modifier::UNDERLINE_DOUBLE),
        (Attr::UnderlineCurly, Modifier::UNDERLINE_CURLY),
        (Attr::UnderlineDotted, Modifier::UNDERLINE_DOTTED),
        (Attr::UnderlineDashed, Modifier::UNDERLINE_DASHED),
        (Attr::Overline, Modifier::OVERLINED),
        (Attr::Strikethrough, Modifier::CROSSED_OUT),
        (Attr::BlinkSlow, Modifier::SLOW_BLINK),
        (Attr::BlinkRapid, Modifier::RAPID_BLINK),
        (Attr::Hidden, Modifier::HIDDEN),
    ];

    /// The attributes `modifier` sets.
    pub fn of(modifier: Modifier) -> Vec<Attr> {
        Attr::ALL
            .iter()
            .filter(|(_, m)| modifier.contains(*m))
            .map(|(a, _)| *a)
            .collect()
    }
}

/// What makes two neighbouring cells one run.
#[derive(PartialEq)]
struct Look<'a> {
    fg: Color,
    bg: Color,
    modifier: Modifier,
    underline: Color,
    link: Option<&'a str>,
}

impl<'a> Look<'a> {
    fn of(cell: &'a Cell) -> Self {
        Look {
            fg: cell.fg,
            bg: cell.bg,
            modifier: cell.modifier,
            underline: cell.underline_color,
            link: cell.link.as_deref(),
        }
    }

    fn run(&self, x: u16, depth: ColorDepth) -> Run {
        let shown = |c: Color| {
            Some(depth.quantize(c))
                .filter(|q| *q != c)
                .map(ColorValue::from)
        };
        Run {
            x,
            w: 0,
            text: String::new(),
            glyphs: None,
            fg: self.fg.into(),
            bg: self.bg.into(),
            fg_shown: shown(self.fg),
            bg_shown: shown(self.bg),
            mods: Attr::of(self.modifier),
            ul: (self.underline != Color::Reset).then(|| self.underline.into()),
            link: self.link.map(str::to_string),
        }
    }
}

impl Grid {
    /// The grid `buffer` shows at `depth`.
    pub fn from_buffer(buffer: &Buffer, depth: ColorDepth) -> Grid {
        let area = buffer.area;
        let rows = (area.y..area.bottom())
            .map(|y| row(buffer, y, depth))
            .collect();
        Grid {
            width: area.width,
            height: area.height,
            rows,
        }
    }

    /// Row `y`'s glyphs, one per cell or wide glyph, a blank cell a space.
    pub fn row_text(&self, y: u16) -> String {
        self.rows
            .get(usize::from(y))
            .map(|runs| runs.iter().map(|r| r.text.as_str()).collect())
            .unwrap_or_default()
    }
}

/// Row `y` of `buffer` as runs.
fn row(buffer: &Buffer, y: u16, depth: ColorDepth) -> Vec<Run> {
    let area = buffer.area;
    let mut runs: Vec<Run> = Vec::new();
    // The open run, how it looks, and its cells' glyphs.
    let mut current: Option<(Look<'_>, Run, Vec<&str>)> = None;
    for x in area.x..area.right() {
        let Some(cell) = buffer.cell(x, y) else {
            continue;
        };
        let glyph = if cell.is_spacer() { "" } else { cell.symbol() };
        if cell.is_spacer()
            && let Some((_, run, cells)) = current.as_mut()
        {
            run.w += 1;
            cells.push(glyph);
            continue;
        }
        let look = Look::of(cell);
        if current.as_ref().is_none_or(|(l, _, _)| *l != look) {
            if let Some(done) = current.take() {
                runs.push(close(done));
            }
            let run = look.run(x - area.x, depth);
            current = Some((look, run, Vec::new()));
        }
        let (_, run, cells) = current.as_mut().expect("a run is open");
        run.w += 1;
        run.text.push_str(glyph);
        cells.push(glyph);
    }
    if let Some(done) = current {
        runs.push(close(done));
    }
    runs
}

/// `run` finished: its `glyphs`, when splitting its text again would not
/// give back `cells`.
fn close((_, mut run, cells): (Look<'_>, Run, Vec<&str>)) -> Run {
    if split_cells(&run.text, run.w) != cells {
        run.glyphs = Some(cells.into_iter().map(str::to_string).collect());
    }
    run
}

/// The cells `text` covers when split into grapheme clusters, each as wide
/// as UAX #11 says (at least one cell), a wide one followed by `""` for its
/// trailing cell; cut or padded with blanks to `w` cells.
pub(crate) fn split_cells(text: &str, w: u16) -> Vec<&str> {
    let mut out = Vec::with_capacity(usize::from(w));
    for g in text.graphemes(true) {
        out.push(g);
        if UnicodeWidthStr::width(g) >= 2 {
            out.push("");
        }
    }
    out.resize(usize::from(w), " ");
    out
}
