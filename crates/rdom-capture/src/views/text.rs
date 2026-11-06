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

use crate::capture::{ColorValue, Grid, NodeInfo, NodeMap, Owners, Run};

/// Which key layers the text view adds below the glyphs. The glyph, bg and
/// fg layers read a [`Grid`] ([`text`]); the owners layer a capture's
/// owners and nodes ([`capture_text`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct TextOptions {
    /// The background of every cell.
    pub bg: bool,
    /// The glyph color of every cell that shows a glyph.
    pub fg: bool,
    /// Which element's paint decided every cell ([`owners_layer`]); read
    /// from a capture's owners layer.
    pub owners: bool,
}

impl Default for TextOptions {
    fn default() -> Self {
        TextOptions {
            bg: true,
            fg: false,
            owners: false,
        }
    }
}

impl TextOptions {
    /// Every key layer.
    pub const ALL: TextOptions = TextOptions {
        bg: true,
        fg: true,
        owners: true,
    };
    /// The glyphs alone.
    pub const GLYPHS: TextOptions = TextOptions {
        bg: false,
        fg: false,
        owners: false,
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

/// The text view of `capture`: [`text`] of its grid, then — when asked
/// and present — its owners layer. `None` without a grid.
pub fn capture_text(capture: &crate::Capture, options: TextOptions) -> Option<String> {
    let mut out = text(capture.grid.as_ref()?, options);
    if options.owners
        && let Some(owners) = &capture.owners
    {
        let empty = NodeMap::default();
        out.push_str(&owners_layer(
            owners,
            capture.nodes.as_ref().unwrap_or(&empty),
        ));
    }
    Some(out)
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

/// The owners layer as a key layer: a key letter per cell for each
/// element-and-part that painted it, `.` for an unpainted cell, and the
/// key below — `a .panel background (#3)`.
pub fn owners_layer(owners: &Owners, nodes: &NodeMap<NodeInfo>) -> String {
    type Key = (Option<String>, String, Vec<String>, Option<String>);
    let mut keys: Vec<Key> = Vec::new();
    let mut rows = String::new();
    for runs in &owners.rows {
        let mut row = String::new();
        for run in runs {
            let key = if run.node.is_none() && run.joined.is_empty() && run.part == "none" {
                '.'
            } else {
                let k = (
                    run.node.clone(),
                    run.part.clone(),
                    run.joined.clone(),
                    run.via.clone(),
                );
                let i = keys.iter().position(|e| *e == k).unwrap_or_else(|| {
                    keys.push(k);
                    keys.len() - 1
                });
                KEYS.get(i).map_or('?', |k| char::from(*k))
            };
            row.extend(std::iter::repeat_n(key, usize::from(run.w)));
        }
        rows.push_str(row.trim_end_matches('.'));
        rows.push('\n');
    }
    if keys.is_empty() {
        return String::new();
    }
    let name = |n: &String| {
        nodes.get(n).map_or_else(
            || format!("#{n}"),
            |info| format!("{} (#{n})", short_path(&info.path)),
        )
    };
    let mut out = format!("── owners ──\n{rows}");
    for (i, (node, part, joined, via)) in keys.iter().enumerate() {
        let key = KEYS.get(i).map_or('?', |k| char::from(*k));
        let who = match (node, joined.is_empty()) {
            (Some(n), _) => name(n),
            (None, false) => joined.iter().map(name).collect::<Vec<_>>().join(" + "),
            (None, true) => String::new(),
        };
        let through = via
            .as_ref()
            .map(|v| format!(" via {}", name(v)))
            .unwrap_or_default();
        out.push_str(&format!("{key} {part} {who}{through}\n").replace("  ", " "));
    }
    if keys.len() > KEYS.len() {
        out.push_str(&format!(
            "(keys ran out: `?` stands for the last {} entries)\n",
            keys.len() - KEYS.len()
        ));
    }
    out
}

/// The last two steps of a node path: enough to recognize an element.
fn short_path(path: &str) -> String {
    let steps: Vec<&str> = path.split(" > ").collect();
    steps[steps.len().saturating_sub(2)..].join(" > ")
}
