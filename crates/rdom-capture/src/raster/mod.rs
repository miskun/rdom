//! The rasterizer (`specs/CAPTURE.md` §Rasterizer): a grid drawn as pixels,
//! at a profile's cell size and an integer scale — no text layout, no
//! system fonts, integer arithmetic only, so the same capture gives the
//! same image on every OS.
//!
//! Each cell is its background, then its glyph from the bundled font
//! (unscii-16, public domain) in its glyph color, then the attributes the profile draws:
//! bold (a one-pixel smear), italic (the upper half sheared a pixel
//! right, within its cell), underline (single, double, curly, dotted, dashed, in its own
//! color), strikethrough, overline. A wide glyph spans its two cells. A
//! character the font lacks is drawn as a box with its code point and
//! listed ([`Image::missing`]): never blank, never guessed.

pub(crate) mod font;
pub(crate) mod png;

use crate::capture::{Attr, Grid};
use crate::profile::{Profile, Rgb};

/// A rendered image: RGB pixels, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// `width * height` pixels.
    pub pixels: Vec<Rgb>,
    /// The characters the font has no glyph for, each once, in first-seen
    /// order.
    pub missing: Vec<char>,
}

impl Image {
    fn new(width: u32, height: u32, fill: Rgb) -> Self {
        Image {
            width,
            height,
            pixels: vec![fill; (width * height) as usize],
            missing: Vec::new(),
        }
    }

    fn set(&mut self, x: u32, y: u32, c: Rgb) {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = c;
        }
    }

    /// The pixel at `(x, y)`.
    pub fn at(&self, x: u32, y: u32) -> Rgb {
        self.pixels[(y * self.width + x) as usize]
    }

    /// The image as a PNG file.
    pub fn png(&self) -> Vec<u8> {
        png::encode(self)
    }
}

/// How a grid is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Raster<'a> {
    pub profile: &'a Profile,
    /// Pixels per font pixel (1 = the profile's cell size).
    pub scale: u32,
}

impl<'a> Raster<'a> {
    /// `profile` at `scale` (at least 1).
    pub fn new(profile: &'a Profile, scale: u32) -> Self {
        Raster {
            profile,
            scale: scale.max(1),
        }
    }
}

/// `grid` drawn as pixels.
pub fn render(grid: &Grid, how: Raster<'_>) -> Image {
    let scale = how.scale.max(1);
    let (cw, ch) = (
        u32::from(how.profile.cell[0]) * scale,
        u32::from(how.profile.cell[1]) * scale,
    );
    let mut img = Image::new(
        u32::from(grid.width) * cw,
        u32::from(grid.height) * ch,
        how.profile.bg,
    );
    for (y, runs) in grid.rows.iter().enumerate() {
        for run in runs {
            let (fg, bg) = run.shown_colors();
            let bold = run.mods.contains(&Attr::Bold);
            let fg = how.profile.rgb(fg, true, bold);
            let bg = how.profile.rgb(bg, false, false);
            let ul = run
                .ul
                .map(|c| how.profile.rgb(c, true, false))
                .unwrap_or(fg);
            let top = y as u32 * ch;
            // The whole run's background first: a wide glyph covers its
            // trailing cell, which must not paint over it.
            fill(
                &mut img,
                (u32::from(run.x) * cw, top),
                (u32::from(run.w) * cw, ch),
                bg,
            );
            if run.mods.contains(&Attr::Hidden) {
                continue;
            }
            let cells = run.cells();
            for (i, glyph) in cells.iter().enumerate() {
                let at = ((u32::from(run.x) + i as u32) * cw, top);
                if glyph.is_empty() {
                    continue;
                }
                let wide = cells.get(i + 1).is_some_and(|g| g.is_empty());
                draw_glyph(&mut img, at, glyph, fg, &run.mods, how, wide);
                decorate(
                    &mut img,
                    at,
                    (if wide { cw * 2 } else { cw }, ch),
                    &run.mods,
                    ul,
                    fg,
                    how,
                );
            }
        }
    }
    img
}

fn fill(img: &mut Image, (x0, y0): (u32, u32), (w, h): (u32, u32), c: Rgb) {
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            img.set(x, y, c);
        }
    }
}

/// Font pixel `(px, py)` of a cell at `at`, scaled.
fn dot(img: &mut Image, at: (u32, u32), (px, py): (u32, u32), scale: u32, c: Rgb) {
    for dy in 0..scale {
        for dx in 0..scale {
            img.set(at.0 + px * scale + dx, at.1 + py * scale + dy, c);
        }
    }
}

fn draw_glyph(
    img: &mut Image,
    at: (u32, u32),
    glyph: &str,
    fg: Rgb,
    mods: &[Attr],
    how: Raster<'_>,
    wide: bool,
) {
    let scale = how.scale.max(1);
    let mut chars = glyph.chars();
    let Some(c) = chars.next() else {
        return;
    };
    if c == ' ' && chars.next().is_none() {
        return;
    }
    let Some(g) = font::glyph(c) else {
        if !img.missing.contains(&c) {
            img.missing.push(c);
        }
        tofu(img, at, c, fg, scale, wide);
        return;
    };
    let width: usize = if g.wide { 16 } else { 8 };
    // What the glyph may cover: its cell, or two for a wide glyph (a wide
    // font glyph in a narrow cell is cut to it).
    let room = if wide { 16 } else { 8 };
    let italic = how.profile.italic && mods.contains(&Attr::Italic);
    let bold = mods.contains(&Attr::Bold);
    for y in 0..16usize {
        // Italic: the upper half a pixel right.
        let shift = u32::from(italic && y < 8);
        for x in 0..width {
            let on = g.at(x, y) || (bold && x > 0 && g.at(x - 1, y));
            if on && x as u32 + shift < room {
                dot(img, at, (x as u32 + shift, y as u32), scale, fg);
            }
        }
    }
}

/// A character the font lacks: a box holding its code point in hex.
fn tofu(img: &mut Image, at: (u32, u32), c: char, fg: Rgb, scale: u32, wide: bool) {
    let w = if wide { 16 } else { 8 };
    for x in 0..w {
        dot(img, at, (x, 0), scale, fg);
        dot(img, at, (x, 15), scale, fg);
    }
    for y in 0..16 {
        dot(img, at, (0, y), scale, fg);
        dot(img, at, (w - 1, y), scale, fg);
    }
    // The low 16 bits as two rows of two 3×5 hex digits.
    let code = u32::from(c);
    let digits = [
        (code >> 12) & 15,
        (code >> 8) & 15,
        (code >> 4) & 15,
        code & 15,
    ];
    for (k, d) in digits.iter().enumerate() {
        let (ox, oy) = (1 + (k as u32 % 2) * 3, 2 + (k as u32 / 2) * 6);
        let bits = HEX_DIGITS[*d as usize];
        for row in 0..5u32 {
            for col in 0..3u32 {
                if bits[row as usize] & (0b100 >> col) != 0 && ox + col < w - 1 {
                    dot(img, at, (ox + col, oy + row), scale, fg);
                }
            }
        }
    }
}

/// 3×5 hex digits, a row per byte, the leftmost pixel in bit 2.
const HEX_DIGITS: [[u8; 5]; 16] = [
    [7, 5, 5, 5, 7],
    [2, 6, 2, 2, 7],
    [7, 1, 7, 4, 7],
    [7, 1, 7, 1, 7],
    [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7],
    [7, 4, 7, 5, 7],
    [7, 1, 1, 1, 1],
    [7, 5, 7, 5, 7],
    [7, 5, 7, 1, 7],
    [7, 5, 7, 5, 5],
    [6, 5, 6, 5, 6],
    [7, 4, 4, 4, 7],
    [6, 5, 5, 5, 6],
    [7, 4, 7, 4, 7],
    [7, 4, 7, 4, 4],
];

/// The lines over a glyph: underline (in `ul`), strikethrough, overline.
fn decorate(
    img: &mut Image,
    at: (u32, u32),
    (w, _h): (u32, u32),
    mods: &[Attr],
    ul: Rgb,
    fg: Rgb,
    how: Raster<'_>,
) {
    let scale = how.scale.max(1);
    let p = how.profile;
    let fw = w / scale;
    let line = |img: &mut Image, y: u32, c: Rgb, keep: &dyn Fn(u32) -> bool| {
        for x in 0..fw {
            if keep(x) {
                dot(img, at, (x, y), scale, c);
            }
        }
    };
    let styled = |a: Attr| p.underline_styles && mods.contains(&a);
    if mods.contains(&Attr::Underline) {
        if styled(Attr::UnderlineDouble) {
            line(img, 13, ul, &|_| true);
            line(img, 15, ul, &|_| true);
        } else if styled(Attr::UnderlineCurly) {
            for x in 0..fw {
                dot(
                    img,
                    at,
                    (x, if (x / 2) % 2 == 0 { 14 } else { 15 }),
                    scale,
                    ul,
                );
            }
        } else if styled(Attr::UnderlineDotted) {
            line(img, 15, ul, &|x| x % 2 == 0);
        } else if styled(Attr::UnderlineDashed) {
            line(img, 15, ul, &|x| x % 4 < 3);
        } else {
            line(img, 15, ul, &|_| true);
        }
    }
    if p.strikethrough && mods.contains(&Attr::Strikethrough) {
        line(img, 8, fg, &|_| true);
    }
    if p.overline && mods.contains(&Attr::Overline) {
        line(img, 0, fg, &|_| true);
    }
}

#[cfg(test)]
mod tests;
