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
    let mut current: Option<(Look<'_>, Run)> = None;
    for x in area.x..area.right() {
        let Some(cell) = buffer.cell(x, y) else {
            continue;
        };
        if cell.is_spacer()
            && let Some((_, run)) = current.as_mut()
        {
            run.w += 1;
            continue;
        }
        let look = Look::of(cell);
        if current.as_ref().is_none_or(|(l, _)| *l != look) {
            if let Some((_, done)) = current.take() {
                runs.push(done);
            }
            let run = look.run(x - area.x, depth);
            current = Some((look, run));
        }
        let (_, run) = current.as_mut().expect("a run is open");
        run.w += 1;
        if !cell.is_spacer() {
            run.text.push_str(cell.symbol());
        }
    }
    if let Some((_, done)) = current {
        runs.push(done);
    }
    runs
}
