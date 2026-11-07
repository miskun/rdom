//! The rasterizer (`specs/CAPTURE.md` §Rasterizer): a grid drawn as pixels,
//! at a profile's cell size and an integer scale — no text layout, no
//! system fonts, integer arithmetic only, so the same capture gives the
//! same image on every OS.
//!
//! Each cell is [`CELL`] font pixels times the scale: its background, then
//! its glyph from the bundled font
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

/// A cell's size in font pixels: the bundled font's, 8 × 16. The image
/// views' cells are this times the scale; a terminal's real cell shape is
/// the `emulator` view's business, not the rasterizer's.
pub const CELL: [u32; 2] = [8, 16];

/// The largest scale.
pub const MAX_SCALE: u32 = 4;

/// The most pixels an image may have: 64 Mi (a 200 × 60 frame at scale 4
/// is 6400 × 3840, 24.6 Mi).
pub const MAX_PIXELS: u64 = 64 * 1024 * 1024;

/// Why a grid cannot be drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RasterError {
    /// The scale is not 1–[`MAX_SCALE`].
    Scale(u32),
    /// The grid has no cells.
    Empty,
    /// The image would have more than [`MAX_PIXELS`] pixels.
    TooLarge { width: u64, height: u64 },
}

impl std::fmt::Display for RasterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RasterError::Scale(s) => write!(f, "scale {s} is not 1–{MAX_SCALE}"),
            RasterError::Empty => f.write_str("the grid has no cells"),
            RasterError::TooLarge { width, height } => write!(
                f,
                "a {width} × {height} image is past the {MAX_PIXELS}-pixel limit: use a smaller scale or viewport"
            ),
        }
    }
}

impl std::error::Error for RasterError {}

/// A rendered image: RGB pixels, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    width: u32,
    height: u32,
    /// `width * height` pixels.
    pixels: Vec<Rgb>,
    missing: Vec<char>,
}

impl Image {
    /// An image of `width` × `height` pixels, all `fill`; the caller has
    /// checked the size against [`MAX_PIXELS`].
    fn new(width: u32, height: u32, fill: Rgb) -> Self {
        Image {
            width,
            height,
            pixels: vec![fill; width as usize * height as usize],
            missing: Vec::new(),
        }
    }

    fn index(&self, x: u32, y: u32) -> usize {
        y as usize * self.width as usize + x as usize
    }

    fn set(&mut self, x: u32, y: u32, c: Rgb) {
        if x < self.width && y < self.height {
            let i = self.index(x, y);
            self.pixels[i] = c;
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Every pixel, row by row.
    pub fn pixels(&self) -> &[Rgb] {
        &self.pixels
    }

    /// The characters the font has no glyph for — and the marks a
    /// character cluster adds that the rasterizer does not draw (it draws a
    /// cluster's first character) — each once, in first-seen order.
    pub fn missing(&self) -> &[char] {
        &self.missing
    }

    /// The pixel at `(x, y)`; `None` outside the image.
    pub fn at(&self, x: u32, y: u32) -> Option<Rgb> {
        (x < self.width && y < self.height).then(|| self.pixels[self.index(x, y)])
    }

    /// The image as a PNG file.
    pub fn png(&self) -> Vec<u8> {
        png::encode(self)
    }

    fn note_missing(&mut self, c: char) {
        if !self.missing.contains(&c) {
            self.missing.push(c);
        }
    }
}

/// How a grid is drawn: a profile, at a scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Raster<'a> {
    profile: &'a Profile,
    scale: u32,
}

impl<'a> Raster<'a> {
    /// `profile` at `scale`: pixels per font pixel, 1–[`MAX_SCALE`].
    pub fn new(profile: &'a Profile, scale: u32) -> Result<Self, RasterError> {
        if !(1..=MAX_SCALE).contains(&scale) {
            return Err(RasterError::Scale(scale));
        }
        Ok(Raster { profile, scale })
    }

    pub fn profile(&self) -> &'a Profile {
        self.profile
    }

    pub fn scale(&self) -> u32 {
        self.scale
    }

    /// A cell's size in the image, in pixels.
    pub fn cell_px(&self) -> [u32; 2] {
        [CELL[0] * self.scale, CELL[1] * self.scale]
    }
}

/// `grid` drawn as pixels; refused when it has no cells or the image would
/// be past [`MAX_PIXELS`].
pub fn render(grid: &Grid, how: Raster<'_>) -> Result<Image, RasterError> {
    let [cw, ch] = how.cell_px();
    let (width, height) = (
        u64::from(grid.width) * u64::from(cw),
        u64::from(grid.height) * u64::from(ch),
    );
    if width == 0 || height == 0 {
        return Err(RasterError::Empty);
    }
    if width * height > MAX_PIXELS {
        return Err(RasterError::TooLarge { width, height });
    }
    // Both fit in u32: their product is at most MAX_PIXELS.
    let mut img = Image::new(width as u32, height as u32, how.profile.bg);
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
                    if wide { 2 * CELL[0] } else { CELL[0] },
                    &run.mods,
                    ul,
                    fg,
                    how,
                );
            }
        }
    }
    Ok(img)
}

/// The `w` × `h` pixels at `(x0, y0)`, clipped to the image, set to `c`.
fn fill(img: &mut Image, (x0, y0): (u32, u32), (w, h): (u32, u32), c: Rgb) {
    let (x1, y1) = ((x0 + w).min(img.width), (y0 + h).min(img.height));
    if x0 >= x1 {
        return;
    }
    for y in y0..y1 {
        let (from, to) = (img.index(x0, y), img.index(x1, y));
        img.pixels[from..to].fill(c);
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
    let scale = how.scale;
    let mut chars = glyph.chars();
    let Some(c) = chars.next() else {
        return;
    };
    // A cluster's first character is drawn; what the others add (a
    // combining mark, a variation selector, a joined emoji) is not, and is
    // listed, never dropped unsaid.
    for rest in chars {
        img.note_missing(rest);
    }
    if c == ' ' {
        return;
    }
    let Some(g) = font::glyph(c) else {
        img.note_missing(c);
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

/// The lines over a glyph `fw` font pixels wide: underline (in `ul`),
/// strikethrough, overline.
fn decorate(
    img: &mut Image,
    at: (u32, u32),
    fw: u32,
    mods: &[Attr],
    ul: Rgb,
    fg: Rgb,
    how: Raster<'_>,
) {
    let scale = how.scale;
    let p = how.profile;
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
