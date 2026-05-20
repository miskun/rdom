//! `Rect` — unsigned grid rectangle, the coordinate space of the
//! terminal screen and the paint `Buffer`.
//!
//! Distinct from `LayoutRect` (in `layout.rs`), which uses signed
//! `i32` position so flex layout can position children above or left
//! of the parent for scroll. `Rect` is what actually gets painted —
//! positive cells only.
//!
//! All arithmetic saturates. A `Rect` with `x = u16::MAX, width = 5`
//! won't overflow; `right()` saturates at `u16::MAX`. This keeps the
//! paint loop trivially safe even with degenerate input.

/// Unsigned grid rectangle in terminal cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    /// Construct a rectangle. Saturates overflows at `u16::MAX`.
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Zero-size rect at origin. Same as `default()`.
    pub const fn zero() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }

    /// `width * height`, saturated to `u32` so 65535 × 65535 works.
    pub fn area(&self) -> u32 {
        self.width as u32 * self.height as u32
    }

    /// Right edge (exclusive): `x + width`, saturated.
    pub fn right(&self) -> u16 {
        self.x.saturating_add(self.width)
    }

    /// Bottom edge (exclusive): `y + height`, saturated.
    pub fn bottom(&self) -> u16 {
        self.y.saturating_add(self.height)
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Does this rect contain the grid cell `(x, y)`?
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// True if this rect and `other` share at least one cell.
    pub fn intersects(&self, other: Rect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    /// Intersection of two rects. Zero-size when they don't overlap.
    pub fn intersection(&self, other: Rect) -> Rect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        if right <= x || bottom <= y {
            Rect::zero()
        } else {
            Rect::new(x, y, right - x, bottom - y)
        }
    }

    /// Smallest rect containing both. Never shrinks.
    pub fn union(&self, other: Rect) -> Rect {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return *self;
        }
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Rect::new(x, y, right - x, bottom - y)
    }

    /// Shrink by `margin` cells on every side. Zero-size if the rect
    /// is too small to absorb the margin.
    pub fn inner(&self, margin: u16) -> Rect {
        let w = self.width.saturating_sub(margin.saturating_mul(2));
        let h = self.height.saturating_sub(margin.saturating_mul(2));
        let x = self.x.saturating_add(margin);
        let y = self.y.saturating_add(margin);
        Rect::new(x, y, w, h)
    }

    /// Shrink the rect by `margin` cells on each of the four sides.
    pub fn inset(&self, top: u16, right: u16, bottom: u16, left: u16) -> Rect {
        let w = self.width.saturating_sub(left.saturating_add(right));
        let h = self.height.saturating_sub(top.saturating_add(bottom));
        let x = self.x.saturating_add(left);
        let y = self.y.saturating_add(top);
        Rect::new(x, y, w, h)
    }

    /// Clamp a point into the rect. Useful for cursor-position logic
    /// at the render boundary.
    pub fn clamp_point(&self, x: u16, y: u16) -> (u16, u16) {
        let cx = x.max(self.x).min(self.right().saturating_sub(1));
        let cy = y.max(self.y).min(self.bottom().saturating_sub(1));
        (cx, cy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_all_fields() {
        let r = Rect::new(3, 4, 10, 20);
        assert_eq!(r.x, 3);
        assert_eq!(r.y, 4);
        assert_eq!(r.width, 10);
        assert_eq!(r.height, 20);
    }

    #[test]
    fn zero_is_zero() {
        let r = Rect::zero();
        assert!(r.is_empty());
        assert_eq!(r.area(), 0);
    }

    #[test]
    fn default_is_zero() {
        assert_eq!(Rect::default(), Rect::zero());
    }

    #[test]
    fn area_does_not_overflow_u16() {
        // 65535 * 65535 = 4_294_836_225 — fits u32.
        let r = Rect::new(0, 0, u16::MAX, u16::MAX);
        assert_eq!(r.area(), u16::MAX as u32 * u16::MAX as u32);
    }

    #[test]
    fn right_and_bottom_saturate() {
        let r = Rect::new(u16::MAX - 3, u16::MAX - 3, 10, 10);
        assert_eq!(r.right(), u16::MAX);
        assert_eq!(r.bottom(), u16::MAX);
    }

    #[test]
    fn contains_inside_outside_edge() {
        let r = Rect::new(5, 10, 4, 3); // x: 5..9, y: 10..13
        assert!(r.contains(5, 10));
        assert!(r.contains(8, 12));
        // Right/bottom edges are exclusive.
        assert!(!r.contains(9, 10));
        assert!(!r.contains(5, 13));
        // Outside.
        assert!(!r.contains(4, 10));
        assert!(!r.contains(5, 9));
    }

    #[test]
    fn intersects_adjacent_is_false() {
        let a = Rect::new(0, 0, 5, 5);
        let b = Rect::new(5, 0, 5, 5);
        assert!(!a.intersects(b));
    }

    #[test]
    fn intersects_overlap() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(5, 5, 10, 10);
        assert!(a.intersects(b));
        assert!(b.intersects(a));
    }

    #[test]
    fn intersection_basic() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(5, 5, 10, 10);
        assert_eq!(a.intersection(b), Rect::new(5, 5, 5, 5));
    }

    #[test]
    fn intersection_nonoverlap_is_zero() {
        let a = Rect::new(0, 0, 5, 5);
        let b = Rect::new(100, 100, 5, 5);
        assert_eq!(a.intersection(b), Rect::zero());
    }

    #[test]
    fn intersection_with_self_is_self() {
        let a = Rect::new(2, 3, 4, 5);
        assert_eq!(a.intersection(a), a);
    }

    #[test]
    fn union_basic() {
        let a = Rect::new(0, 0, 5, 5);
        let b = Rect::new(10, 10, 5, 5);
        assert_eq!(a.union(b), Rect::new(0, 0, 15, 15));
    }

    #[test]
    fn union_with_empty_is_other() {
        let a = Rect::zero();
        let b = Rect::new(5, 5, 10, 10);
        assert_eq!(a.union(b), b);
        assert_eq!(b.union(a), b);
    }

    #[test]
    fn inner_shrinks_by_margin() {
        let r = Rect::new(0, 0, 10, 10).inner(2);
        assert_eq!(r, Rect::new(2, 2, 6, 6));
    }

    #[test]
    fn inner_too_small_is_empty() {
        let r = Rect::new(0, 0, 3, 3).inner(5);
        assert!(r.is_empty());
    }

    #[test]
    fn inset_asymmetric() {
        // top=1 right=2 bottom=3 left=4
        let r = Rect::new(0, 0, 20, 20).inset(1, 2, 3, 4);
        assert_eq!(r, Rect::new(4, 1, 14, 16));
    }

    #[test]
    fn clamp_point_inside_unchanged() {
        let r = Rect::new(2, 3, 10, 10);
        assert_eq!(r.clamp_point(5, 7), (5, 7));
    }

    #[test]
    fn clamp_point_below_origin() {
        let r = Rect::new(2, 3, 10, 10);
        assert_eq!(r.clamp_point(0, 0), (2, 3));
    }

    #[test]
    fn clamp_point_above_edge() {
        let r = Rect::new(2, 3, 10, 10); // x: 2..12, y: 3..13
        assert_eq!(r.clamp_point(100, 100), (11, 12));
    }

    #[test]
    fn copy_eq_hash() {
        let a = Rect::new(1, 2, 3, 4);
        let b = a; // Copy
        assert_eq!(a, b);
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }
}
