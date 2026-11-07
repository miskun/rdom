//! A contact sheet: several grids laid out as one, each under a label
//! line, for `matrix` — composed in cells and drawn by the same
//! rasterizer, so it has its limits and its determinism.

use crate::capture::{Attr, ColorValue, Grid, Run};

/// `tiles` (each a label and a grid) laid out `columns` to a row, each in
/// a slot as large as the largest tile plus its label line, one blank
/// column and row between slots.
pub fn sheet(tiles: &[(String, &Grid)], columns: usize) -> Grid {
    let columns = columns.clamp(1, tiles.len().max(1));
    let slot_w = tiles.iter().map(|(_, g)| g.width).max().unwrap_or(0);
    let slot_h = tiles.iter().map(|(_, g)| g.height).max().unwrap_or(0) + 1;
    let rows_of_tiles = tiles.len().div_ceil(columns);
    let width = (slot_w as usize * columns + columns.saturating_sub(1)).min(usize::from(u16::MAX));
    let height = (usize::from(slot_h) * rows_of_tiles + rows_of_tiles.saturating_sub(1))
        .min(usize::from(u16::MAX));
    let mut rows: Vec<Vec<Run>> = vec![Vec::new(); height];
    for (k, (label, grid)) in tiles.iter().enumerate() {
        let x0 = (k % columns) * (usize::from(slot_w) + 1);
        let y0 = (k / columns) * (usize::from(slot_h) + 1);
        let fits = |x: usize, w: usize| x + w <= width;
        // The label: bold, cut to the slot.
        let text: String = label.chars().take(usize::from(slot_w)).collect();
        let w = unicode_width::UnicodeWidthStr::width(text.as_str());
        if w > 0 && y0 < height && fits(x0, w) {
            rows[y0].push(run(
                x0,
                w,
                text,
                ColorValue::Default,
                ColorValue::Default,
                vec![Attr::Bold],
                None,
            ));
        }
        for (dy, src) in grid.rows.iter().enumerate() {
            let y = y0 + 1 + dy;
            if y >= height {
                break;
            }
            for r in src {
                let cells = r.cells();
                for (i, glyph) in cells.iter().enumerate() {
                    if glyph.is_empty() {
                        continue;
                    }
                    let w = if cells.get(i + 1).is_some_and(|g| g.is_empty()) {
                        2
                    } else {
                        1
                    };
                    let x = x0 + usize::from(r.x) + i;
                    if !fits(x, w) {
                        continue;
                    }
                    let (fg, bg) = r.shown_colors();
                    rows[y].push(run(
                        x,
                        w,
                        (*glyph).to_string(),
                        fg,
                        bg,
                        r.mods.clone(),
                        r.ul,
                    ));
                }
            }
        }
    }
    // A grid's runs tile each row exactly: the gaps are blank runs.
    let rows = rows
        .into_iter()
        .map(|mut row| {
            row.sort_by_key(|r| r.x);
            let mut tiled = Vec::with_capacity(row.len() * 2 + 1);
            let mut x = 0;
            for r in row {
                if usize::from(r.x) > x {
                    tiled.push(blank(x, usize::from(r.x) - x));
                }
                x = usize::from(r.x + r.w);
                tiled.push(r);
            }
            if x < width {
                tiled.push(blank(x, width - x));
            }
            tiled
        })
        .collect();
    Grid {
        width: width as u16,
        height: height as u16,
        rows,
    }
}

/// `w` blank cells at `x`.
fn blank(x: usize, w: usize) -> Run {
    run(
        x,
        w,
        " ".repeat(w),
        ColorValue::Default,
        ColorValue::Default,
        Vec::new(),
        None,
    )
}

fn run(
    x: usize,
    w: usize,
    text: String,
    fg: ColorValue,
    bg: ColorValue,
    mods: Vec<Attr>,
    ul: Option<ColorValue>,
) -> Run {
    Run {
        x: x as u16,
        w: w as u16,
        text,
        glyphs: None,
        fg,
        bg,
        fg_shown: None,
        bg_shown: None,
        mods,
        ul,
        link: None,
    }
}

#[cfg(test)]
mod tests {
    use rdom_tui::{Buffer, ColorDepth, Rect, Style};

    use super::sheet;
    use crate::capture::Grid;

    fn grid(w: u16, h: u16, text: &str) -> Grid {
        let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
        buf.set_string(0, 0, text, Style::default());
        Grid::from_buffer(&buf, ColorDepth::TrueColor)
    }

    #[test]
    fn tiles_are_laid_out_under_their_labels() {
        let (a, b, c) = (grid(4, 2, "aa"), grid(6, 3, "bbb"), grid(4, 2, "c"));
        let s = sheet(&[("A".into(), &a), ("B".into(), &b), ("C".into(), &c)], 2);
        // Slots 6 × (3 + 1), a gutter between: 2 × 6 + 1 wide, 2 × 4 + 1 high.
        assert_eq!((s.width, s.height), (13, 9));
        assert_eq!(
            s.row_text(0).trim_end(),
            "A      B",
            "the labels of the first row of tiles"
        );
        assert_eq!(&s.row_text(1)[..2], "aa");
        assert_eq!(&s.row_text(1)[7..10], "bbb");
        assert_eq!(&s.row_text(5)[..1], "C");
        assert_eq!(&s.row_text(6)[..1], "c");
        // The runs tile every row.
        for row in &s.rows {
            assert_eq!(row.iter().map(|r| r.w).sum::<u16>(), s.width);
        }
    }
}
