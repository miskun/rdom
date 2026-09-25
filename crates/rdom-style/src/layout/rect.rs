//! `LayoutRect` — the signed-position rectangle layout computes.
//!
//! `LayoutRect` is signed (`i32` x/y) so layout can position children
//! above or left of their parent for scroll clipping; rdom-tui's
//! `render::Rect` is unsigned (`u16`) because it names actual terminal
//! grid cells. Layout computes `LayoutRect`; paint clips and converts
//! to `Rect`.

/// Layout rectangle with signed position (i32) and unsigned dimensions (u16).
///
/// Allows elements to be positioned above/left of the viewport (negative
/// coords) which is needed for scroll clipping — when content has scrolled
/// up, the laid-out rect has a negative `y` and only the visible portion
/// gets painted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LayoutRect {
    pub x: i32,
    pub y: i32,
    pub width: u16,
    pub height: u16,
}

impl LayoutRect {
    pub fn new(x: i32, y: i32, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Right edge (x + width).
    pub fn right(&self) -> i32 {
        self.x + self.width as i32
    }

    /// Bottom edge (y + height).
    pub fn bottom(&self) -> i32 {
        self.y + self.height as i32
    }

    /// Check if this rect intersects another.
    pub fn intersects(&self, other: &LayoutRect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    /// Compute intersection of two rects. Empty if no overlap.
    pub fn intersection(&self, other: &LayoutRect) -> LayoutRect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let r = self.right().min(other.right());
        let b = self.bottom().min(other.bottom());
        if r <= x || b <= y {
            LayoutRect::default()
        } else {
            LayoutRect::new(x, y, (r - x) as u16, (b - y) as u16)
        }
    }

    /// Zero dimensions.
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── LayoutRect ───────────────────────────────────────────────────

    #[test]
    fn right_and_bottom_edges() {
        let r = LayoutRect::new(10, 20, 5, 6);
        assert_eq!(r.right(), 15);
        assert_eq!(r.bottom(), 26);
    }

    #[test]
    fn intersects_overlapping() {
        let a = LayoutRect::new(0, 0, 10, 10);
        let b = LayoutRect::new(5, 5, 10, 10);
        assert!(a.intersects(&b));
    }

    #[test]
    fn intersects_touching_is_false() {
        // Right edge touches left edge — CSS box model: no overlap.
        let a = LayoutRect::new(0, 0, 10, 10);
        let b = LayoutRect::new(10, 0, 10, 10);
        assert!(!a.intersects(&b));
    }

    #[test]
    fn intersection_basic() {
        let a = LayoutRect::new(-5, 0, 10, 10);
        let b = LayoutRect::new(0, 0, 20, 20);
        assert_eq!(a.intersection(&b), LayoutRect::new(0, 0, 5, 10));
    }

    #[test]
    fn intersection_no_overlap_is_empty() {
        let a = LayoutRect::new(-10, 0, 5, 5);
        let b = LayoutRect::new(0, 0, 20, 20);
        assert!(a.intersection(&b).is_empty());
    }

    #[test]
    fn is_empty_zero_dim() {
        assert!(LayoutRect::new(0, 0, 0, 5).is_empty());
        assert!(LayoutRect::new(0, 0, 5, 0).is_empty());
        assert!(!LayoutRect::new(0, 0, 5, 5).is_empty());
    }
}
