//! The grid hash: a stable fingerprint of what a grid shows, the same on
//! every platform and in every process, so two captures can be compared
//! without shipping both grids.
//!
//! FNV-1a (64-bit) over a canonical encoding of the grid's cells — its
//! size, then each run's position, glyphs, colors, attributes, underline
//! color and link, each field tagged and length-prefixed. The encoding is
//! defined here, not by the JSON form, so layers and fields added to a
//! capture later never change the hash of the same picture. It detects
//! change; it is not a security boundary.

use super::{ColorValue, Grid, Run};

/// The grid hash of `grid`: `"fnv1a64:"` and 16 hex digits.
pub fn grid_hash(grid: &Grid) -> String {
    let mut h = Fnv::new();
    h.u16(grid.width);
    h.u16(grid.height);
    for runs in &grid.rows {
        h.u8(b'\n');
        for run in runs {
            run_into(&mut h, run);
        }
    }
    format!("fnv1a64:{:016x}", h.0)
}

fn run_into(h: &mut Fnv, run: &Run) {
    h.u8(b'r');
    h.u16(run.x);
    h.u16(run.w);
    h.str(&run.text);
    match &run.glyphs {
        Some(glyphs) => {
            h.u8(1);
            h.u32(glyphs.len() as u32);
            glyphs.iter().for_each(|g| h.str(g));
        }
        None => h.u8(0),
    }
    color(h, Some(run.fg));
    color(h, Some(run.bg));
    color(h, run.fg_shown);
    color(h, run.bg_shown);
    h.u32(run.mods.len() as u32);
    for m in &run.mods {
        h.u8(*m as u8);
    }
    color(h, run.ul);
    h.str(run.link.as_deref().unwrap_or(""));
}

fn color(h: &mut Fnv, c: Option<ColorValue>) {
    match c {
        None => h.u8(0),
        Some(ColorValue::Default) => h.u8(1),
        Some(ColorValue::Rgb(r, g, b)) => {
            h.u8(2);
            h.bytes(&[r, g, b]);
        }
        Some(ColorValue::Rgba(r, g, b, a)) => {
            h.u8(3);
            h.bytes(&[r, g, b, a]);
        }
        Some(ColorValue::Ansi(n)) => {
            h.u8(4);
            h.u8(n);
        }
    }
}

/// FNV-1a, 64-bit, fed field by field.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
    fn bytes(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    fn u8(&mut self, v: u8) {
        self.bytes(&[v]);
    }
    fn u16(&mut self, v: u16) {
        self.bytes(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }
    fn str(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.bytes(s.as_bytes());
    }
}

/// FNV-1a, 64-bit, of `bytes`.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h = Fnv::new();
    h.bytes(bytes);
    h.0
}
