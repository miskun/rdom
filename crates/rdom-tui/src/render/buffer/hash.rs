//! A buffer's content hash: what a recorded frame is checked by on
//! replay (`runtime::record`).

use super::Buffer;
use crate::Color;

/// FNV-1a, 64-bit.
struct Fnv(u64);

impl Fnv {
    fn bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }

    fn color(&mut self, c: Color) {
        match c {
            Color::Reset => self.bytes(&[0]),
            Color::Indexed(n) => self.bytes(&[1, n]),
            Color::Rgb(r, g, b) => self.bytes(&[2, r, g, b]),
            Color::Rgba(r, g, b, a) => self.bytes(&[3, r, g, b, a]),
        }
    }
}

impl Buffer {
    /// A hash of what every cell shows — its glyph, colors, attributes,
    /// underline color and link — and the buffer's size: two buffers that
    /// draw the same have the same hash. FNV-1a over a fixed encoding,
    /// so it is the same on every platform and in every process.
    pub fn content_hash(&self) -> u64 {
        let mut h = Fnv(0xcbf2_9ce4_8422_2325);
        h.bytes(&self.area.width.to_le_bytes());
        h.bytes(&self.area.height.to_le_bytes());
        for cell in &self.content {
            let symbol = cell.symbol();
            h.bytes(&(symbol.len() as u32).to_le_bytes());
            h.bytes(symbol.as_bytes());
            h.color(cell.fg);
            h.color(cell.bg);
            h.bytes(&cell.modifier.bits().to_le_bytes());
            h.color(cell.underline_color);
            let link = cell.link.as_deref().unwrap_or("");
            h.bytes(&(link.len() as u32).to_le_bytes());
            h.bytes(link.as_bytes());
        }
        h.0
    }
}

#[cfg(test)]
mod tests {
    use crate::render::Buffer;
    use crate::{Color, Rect, Style};

    #[test]
    fn the_hash_follows_what_the_cells_show() {
        let mut a = Buffer::empty(Rect::new(0, 0, 4, 1));
        a.set_string(0, 0, "ab", Style::default());
        let mut b = Buffer::empty(Rect::new(0, 0, 4, 1));
        b.set_string(0, 0, "ab", Style::default());
        assert_eq!(a.content_hash(), b.content_hash());
        b.set_string(0, 0, "ab", Style::default().fg(Color::Rgb(1, 2, 3)));
        assert_ne!(a.content_hash(), b.content_hash());
        // Pinned: the hash is what a recorded log holds.
        assert_eq!(
            a.content_hash(),
            0x8e24_d75b_8694_f9fb,
            "{:x}",
            a.content_hash()
        );
    }
}
