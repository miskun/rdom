//! `mockup` (`specs/CAPTURE.md` §Variants): the screen as it should be,
//! written as a text view (`views::text`) — glyph lines, then optional
//! `── background ──` / `── foreground ──` key layers with their keys —
//! compared with the frame: the cells whose glyph or color differ, and the
//! rows whose text is there but shifted. A captured text view is a valid
//! mockup of its own frame, so the loop is: capture, edit the text into
//! what it should be, compare.
//!
//! The mockup covers a rectangle at `at`: as many rows as glyph lines and
//! as wide as the longest. A glyph line's cells past its end are expected
//! blank (a text view trims trailing blanks). An `any` character matches
//! every glyph. In a key layer a key's color is expected, `.` the
//! terminal's default (in `foreground`: or no glyph), and a space anything.

use std::collections::HashMap;

use serde::Serialize;

use crate::capture::{ColorValue, Grid};

/// How the frame differs from a mockup.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct MockupReport {
    /// Whether every compared cell matched.
    pub matches: bool,
    /// The cells compared, glyphs and colors.
    pub compared: u32,
    /// The cells that differ in their glyph, or a color.
    pub differ: u32,
    /// Per row, the span of glyphs that differ.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub glyphs: Vec<GlyphDiff>,
    /// Runs of cells whose color differs.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub colors: Vec<ColorDiff>,
    /// Rows whose expected text is in the frame at another column.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shifted: Vec<Shift>,
}

/// A row's glyphs that differ, from the first to the last.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct GlyphDiff {
    pub y: u16,
    pub x: u16,
    pub expected: String,
    pub actual: String,
}

/// A mockup's key layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum KeyLayer {
    Background,
    Foreground,
}

/// Cells of one row whose color differs from the same expected color.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ColorDiff {
    pub layer: KeyLayer,
    pub y: u16,
    pub x: u16,
    pub w: u16,
    pub expected: ColorValue,
    /// The first differing cell's color.
    pub actual: ColorValue,
}

/// A row whose text is in the frame `by` columns off (positive: right).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Shift {
    pub y: u16,
    pub by: i32,
    pub text: String,
}

/// A parsed mockup.
struct Mockup {
    lines: Vec<Vec<String>>,
    /// Per layer title: its rows (`None` for a don't-care cell) of
    /// expected colors, `Some(None)` the default.
    layers: Vec<(KeyLayer, Vec<KeyRow>)>,
}

/// A key layer's row: per cell, `None` for anything, else the expected
/// color (`None`: the default).
type KeyRow = Vec<Option<Option<ColorValue>>>;

/// A key layer being read: its title, rows and keys.
type Section<'a> = (KeyLayer, Vec<&'a str>, HashMap<char, ColorValue>);

/// One frame cell: its glyph and shown colors.
struct Cell<'a> {
    glyph: &'a str,
    fg: ColorValue,
    bg: ColorValue,
}

fn parse(text: &str) -> Result<Mockup, String> {
    let mut lines = Vec::new();
    let mut layers = Vec::new();
    let mut section: Option<Section<'_>> = None;
    let close = |s: Option<Section<'_>>, layers: &mut Vec<_>| -> Result<(), String> {
        let Some((title, rows, keys)) = s else {
            return Ok(());
        };
        let rows = rows
            .iter()
            .map(|r| {
                r.chars()
                    .map(|k| match k {
                        // `?` is every color past a text view's 62nd.
                        ' ' | '?' => Ok(None),
                        '.' => Ok(Some(None)),
                        k => keys
                            .get(&k)
                            .map(|c| Some(Some(*c)))
                            .ok_or_else(|| format!("{title:?}: key {k:?} has no color below")),
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        layers.push((title, rows));
        Ok(())
    };
    // Past an `owners` layer (which a mockup does not compare).
    let mut skipping = false;
    for line in text.lines() {
        let header = line.strip_prefix("── ").and_then(|l| l.strip_suffix(" ──"));
        match header {
            Some(t @ ("background" | "foreground")) => {
                close(section.take(), &mut layers)?;
                let title = if t == "background" {
                    KeyLayer::Background
                } else {
                    KeyLayer::Foreground
                };
                section = Some((title, Vec::new(), HashMap::new()));
                skipping = false;
                continue;
            }
            Some("owners") => {
                close(section.take(), &mut layers)?;
                skipping = true;
                continue;
            }
            // A row of the screen that looks like a header (`── Settings ──`)
            // is glyphs, before the key layers.
            Some(other) if section.is_some() || skipping => {
                return Err(format!(
                    "no key layer {other:?}: background, foreground (or owners, skipped)"
                ));
            }
            _ => {}
        }
        if skipping {
            continue;
        }
        match section.as_mut() {
            None => lines.push(cells_of(line)),
            Some((_, rows, keys)) => {
                // `k #rrggbb` lines are the keys; the others are rows.
                let mut it = line.splitn(2, ' ');
                match (it.next(), it.next()) {
                    (Some(k), Some(c))
                        if k.chars().count() == 1 && c.parse::<ColorValue>().is_ok() =>
                    {
                        keys.insert(k.chars().next().expect("one"), c.parse().expect("checked"));
                    }
                    _ => rows.push(line),
                }
            }
        }
    }
    close(section, &mut layers)?;
    Ok(Mockup { lines, layers })
}

/// The cells of `line` from its first to its last non-blank one.
fn content(line: &[String]) -> Option<(usize, usize)> {
    let blank = |c: &String| c == " " || c.is_empty();
    let from = line.iter().position(|c| !blank(c))?;
    let to = line.iter().rposition(|c| !blank(c))? + 1;
    Some((from, to))
}

/// A glyph line as cells, split as the grid splits its text: grapheme
/// clusters, a wide one followed by `""`.
fn cells_of(line: &str) -> Vec<String> {
    let w = unicode_width::UnicodeWidthStr::width(line);
    crate::capture::grid_cells(line, w.min(usize::from(u16::MAX)) as u16)
        .into_iter()
        .map(str::to_string)
        .collect()
}

/// How `grid` differs from the mockup `text` placed at `at`; `any` matches
/// every glyph.
pub fn compare(
    grid: &Grid,
    text: &str,
    at: [u16; 2],
    any: Option<char>,
) -> Result<MockupReport, String> {
    let m = parse(text)?;
    let frame: Vec<Vec<Cell<'_>>> = grid
        .rows
        .iter()
        .map(|runs| {
            runs.iter()
                .flat_map(|r| {
                    let (fg, bg) = r.shown_colors();
                    r.cells()
                        .into_iter()
                        .map(move |glyph| Cell { glyph, fg, bg })
                })
                .collect()
        })
        .collect();
    let width = m.lines.iter().map(Vec::len).max().unwrap_or(0);
    let cell = |x: usize, y: usize| -> Option<&Cell<'_>> {
        frame
            .get(y + usize::from(at[1]))?
            .get(x + usize::from(at[0]))
    };
    let mut r = MockupReport::default();
    let any = any.map(|c| c.to_string());
    for (y, line) in m.lines.iter().enumerate() {
        let mut bad: Vec<usize> = Vec::new();
        for x in 0..width {
            let want = line.get(x).map_or(" ", String::as_str);
            if any.as_deref() == Some(want)
                || (want.is_empty() && x > 0 && line.get(x - 1).is_some())
            {
                continue;
            }
            r.compared += 1;
            let got = cell(x, y).map_or("", |c| c.glyph);
            if got != want {
                bad.push(x);
            }
        }
        if let (Some(&first), Some(&last)) = (bad.first(), bad.last()) {
            r.differ += bad.len() as u32;
            let span = |f: &dyn Fn(usize) -> String| (first..=last).map(f).collect::<String>();
            let expected = span(&|x| line.get(x).cloned().unwrap_or_else(|| " ".into()));
            let actual = span(&|x| cell(x, y).map_or(" ".into(), |c| c.glyph.to_string()));
            let yy = (y + usize::from(at[1])) as u16;
            r.glyphs.push(GlyphDiff {
                y: yy,
                x: (first + usize::from(at[0])) as u16,
                expected,
                actual,
            });
            // The row's text elsewhere in the frame's row: shifted.
            if let Some(row) = frame.get(y + usize::from(at[1]))
                && let Some((from, to)) = content(line)
            {
                let want = &line[from..to];
                let expected_x = usize::from(at[0]) + from;
                let found = (0..row.len().saturating_sub(want.len() - 1))
                    .find(|&x| want.iter().zip(&row[x..]).all(|(w, c)| w == c.glyph));
                if let Some(found) = found
                    && found != expected_x
                {
                    r.shifted.push(Shift {
                        y: yy,
                        by: found as i32 - expected_x as i32,
                        text: want.concat(),
                    });
                }
            }
        }
    }
    for (title, rows) in &m.layers {
        for (y, row) in rows.iter().enumerate() {
            // Past a key row's end: the default, as a text view trims it.
            let mut run: Option<ColorDiff> = None;
            for x in 0..width.max(row.len()) {
                let want = row.get(x).copied().unwrap_or(Some(None));
                let Some(want) = want else {
                    continue;
                };
                let Some(c) = cell(x, y) else {
                    continue;
                };
                r.compared += 1;
                let got = if *title == KeyLayer::Background {
                    c.bg
                } else {
                    c.fg
                };
                let blank = c.glyph == " " || c.glyph.is_empty();
                let ok = match want {
                    None => got == ColorValue::Default || (*title == KeyLayer::Foreground && blank),
                    Some(w) => got == w && !(*title == KeyLayer::Foreground && blank),
                };
                let expected = want.unwrap_or(ColorValue::Default);
                if ok {
                    if let Some(d) = run.take() {
                        r.colors.push(d);
                    }
                    continue;
                }
                r.differ += 1;
                let (yy, xx) = (
                    (y + usize::from(at[1])) as u16,
                    (x + usize::from(at[0])) as u16,
                );
                match run.as_mut() {
                    Some(d) if d.expected == expected && d.x + d.w == xx => d.w += 1,
                    _ => {
                        if let Some(d) = run.take() {
                            r.colors.push(d);
                        }
                        run = Some(ColorDiff {
                            layer: *title,
                            y: yy,
                            x: xx,
                            w: 1,
                            expected,
                            actual: got,
                        });
                    }
                }
            }
            if let Some(d) = run {
                r.colors.push(d);
            }
        }
    }
    r.matches = r.differ == 0;
    Ok(r)
}

impl<A: crate::app::CaptureApp> crate::session::Session<A> {
    /// How the current frame differs from `mockup` placed at `at`
    /// ([`compare`]).
    pub fn mockup(
        &self,
        mockup: &str,
        at: [u16; 2],
        any: Option<char>,
    ) -> Result<MockupReport, crate::session::SessionError> {
        let c = self.capture(&crate::session::CaptureRequest::with_layers([
            crate::capture::Layer::Grid,
        ]))?;
        compare(c.grid.as_ref().expect("asked for"), mockup, at, any)
            .map_err(crate::session::SessionError::InvalidInput)
    }
}

#[cfg(test)]
mod tests {
    use rdom_tui::{Buffer, Color, ColorDepth, Rect, Style};

    use super::compare;
    use crate::capture::{ColorValue, Grid};
    use crate::views::text::{TextOptions, text};

    fn frame() -> Grid {
        let mut buf = Buffer::empty(Rect::new(0, 0, 12, 2));
        buf.set_string(
            0,
            0,
            "Name: ada",
            Style::default().bg(Color::Rgb(30, 30, 30)),
        );
        buf.set_string(2, 1, "[ Save ]", Style::default().fg(Color::Rgb(0, 200, 0)));
        Grid::from_buffer(&buf, ColorDepth::TrueColor)
    }

    #[test]
    fn a_frames_own_text_view_is_a_matching_mockup() {
        let g = frame();
        let view = text(&g, TextOptions::ALL);
        let r = compare(&g, &view, [0, 0], None).unwrap();
        assert!(r.matches, "{r:#?}\n{view}");
        assert!(r.compared > 24);
    }

    #[test]
    fn a_header_like_row_and_a_captures_owners_layer_round_trip() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 14, 1));
        buf.set_string(0, 0, "── Settings ──", Style::default());
        let g = Grid::from_buffer(&buf, ColorDepth::TrueColor);
        let view = text(&g, TextOptions::ALL);
        let with_owners = format!("{view}── owners ──\naaaa\na 12 p\n");
        let r = compare(&g, &with_owners, [0, 0], None).unwrap();
        assert!(r.matches, "{r:#?}");
    }

    #[test]
    fn glyphs_colors_and_shifts_are_reported() {
        let g = frame();
        let mockup = "Name: bob\n[ Save ]\n── foreground ──\n\n..aaaaaaaa\na #00c800\n";
        let r = compare(&g, mockup, [0, 0], None).unwrap();
        assert!(!r.matches);
        assert_eq!((r.glyphs[0].y, r.glyphs[0].x), (0, 6));
        assert_eq!(
            (r.glyphs[0].expected.as_str(), r.glyphs[0].actual.as_str()),
            ("bob", "ada")
        );
        // "[ Save ]" is in the frame two columns right of where the mockup
        // has it.
        assert_eq!(r.shifted.len(), 1, "{r:#?}");
        assert_eq!(
            r.shifted[0].by, 2,
            "the frame's text is 2 columns right: {r:#?}"
        );
        // The foreground layer is compared where the mockup puts it: the
        // frame's glyphs are 2 columns right, so the expected color is
        // missing from the first two cells.
        assert!(
            r.colors
                .iter()
                .all(|c| c.layer == super::KeyLayer::Foreground)
        );
        assert!(
            r.colors
                .iter()
                .any(|c| c.expected == ColorValue::Rgb(0, 0xc8, 0))
        );
        // A wildcard ignores glyphs.
        let loose = compare(&g, "Name: ???", [0, 0], Some('?')).unwrap();
        assert!(loose.matches, "{loose:#?}");
        assert!(compare(&g, "x\n── background ──\n── middle ──\n", [0, 0], None).is_err());
    }
}
