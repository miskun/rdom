//! The text view: the grid's glyphs, one line per row, then optional key
//! layers — the background and the glyph color of every cell as one
//! letter each, with the key below. Exact and cheap to read, for agents
//! and for snapshot tests.
//!
//! ```text
//! ┌ Settings ─┐
//! │ Name: ada │
//! ── background ──
//! aaaaaaaaaaaa
//! abbbbbbbbbba
//! a #1e1e1e
//! b #2c2a3a
//! ```
//!
//! The keys are the colors the terminal is shown — after the color depth,
//! where it changes them. Trailing blanks of a glyph row, and trailing `.`
//! (the terminal's default) of a key row, are trimmed. A layer whose every cell is the
//! default is left out. Keys run `a`–`z`, `A`–`Z`, `0`–`9`; a 63rd color
//! and every one after it share `?`, listed as such.

use serde::{Deserialize, Serialize};

use crate::capture::{ColorValue, Grid, Run};

/// Which key layers the text view adds below the glyphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct TextOptions {
    /// The background of every cell.
    pub bg: bool,
    /// The glyph color of every cell that shows a glyph.
    pub fg: bool,
}

impl Default for TextOptions {
    fn default() -> Self {
        TextOptions {
            bg: true,
            fg: false,
        }
    }
}

impl TextOptions {
    /// Both key layers.
    pub const ALL: TextOptions = TextOptions { bg: true, fg: true };
    /// The glyphs alone.
    pub const GLYPHS: TextOptions = TextOptions {
        bg: false,
        fg: false,
    };
}

const KEYS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// The text view of `grid`.
pub fn text(grid: &Grid, options: TextOptions) -> String {
    let mut out = String::new();
    for y in 0..grid.height {
        out.push_str(grid.row_text(y).trim_end());
        out.push('\n');
    }
    if options.bg {
        out.push_str(&layer(grid, "background", |run, _| {
            Some(run.shown_colors().1)
        }));
    }
    if options.fg {
        out.push_str(&layer(grid, "foreground", |run, glyph| {
            (glyph != " ").then_some(run.shown_colors().0)
        }));
    }
    out
}

/// One key layer: `color` of each cell (`None`: shown as `.`), given its
/// run and its glyph.
fn layer(grid: &Grid, title: &str, color: impl Fn(&Run, &str) -> Option<ColorValue>) -> String {
    let mut colors: Vec<ColorValue> = Vec::new();
    let mut rows = String::new();
    for runs in &grid.rows {
        let mut row = String::new();
        for run in runs {
            for glyph in run.cells() {
                let key = match color(run, glyph).filter(|c| *c != ColorValue::Default) {
                    None => '.',
                    Some(c) => {
                        let i = colors.iter().position(|k| *k == c).unwrap_or_else(|| {
                            colors.push(c);
                            colors.len() - 1
                        });
                        KEYS.get(i).map_or('?', |k| char::from(*k))
                    }
                };
                row.push(key);
            }
        }
        rows.push_str(row.trim_end_matches('.'));
        rows.push('\n');
    }
    if colors.is_empty() {
        return String::new();
    }
    let mut out = format!("── {title} ──\n{rows}");
    for (i, c) in colors.iter().enumerate() {
        let key = KEYS.get(i).map_or('?', |k| char::from(*k));
        out.push_str(&format!("{key} {c}\n"));
    }
    out
}
