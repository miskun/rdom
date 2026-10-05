//! `box-shadow` (CSS Backgrounds 3 §6.1): each shadow a shade of whole
//! cells in its color. An outer shadow is the border box offset by the
//! shadow's offsets and grown by its spread, drawn outside the border
//! box only, under the background; an `inset` shadow is the padding box
//! less itself offset and shrunk, drawn above the background. Offsets
//! and spread are whole cells (`PaintLength::offset_cells`, at most
//! `u16::MAX` either way, with saturating geometry here); the blur
//! has no effect (DIVERGENCES §2). The first shadow is on top.
//!
//! Paint order (CSS 2.1 Appendix E, Backgrounds 3 §7.2): a box's
//! shadows paint with its background. For an in-flow box that is step
//! 4 of its paint unit — the in-flow block backgrounds, in tree order,
//! before any inline content — so an opaque shadow there covers what
//! the unit painted beneath (lower layers, the unit root's box) and the
//! earlier siblings' backgrounds and borders, but not their text. rdom
//! paints box by box, so it does that in two strokes ([`Shadows`]):
//! [`paint_backdrop_shadows`] in the unit's background phase, before
//! its content, and again at the box's turn in tree order, under the
//! glyphs already painted. A translucent shadow composites once, at
//! the box's turn: it keeps the glyphs beneath anyway.

use rdom_core::{Dom, NodeType};

use super::background::{fill_bg, tint_bg};
use crate::ext::TuiExt;
use crate::layout::{BoxShadow, LayoutRect, compute_padding_box};
use crate::node::TuiNodeExt;
use crate::render::stacking::ShadowEntry;
use crate::render::{Buffer, Rect};
use crate::style::{Color, ComputedStyle};

/// How [`paint_outer_shadows`] paints a box's outer shadows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Shadows {
    /// A paint unit's root (a stacking context, a `z-index: auto`
    /// positioned box): over everything beneath.
    Whole,
    /// An in-flow box's opaque shadows in its unit's background phase,
    /// before the unit's content: over everything beneath.
    Backdrop,
    /// An in-flow box at its turn in tree order: opaque shadows under
    /// the glyphs already painted (`tint_bg`), translucent ones
    /// composited.
    UnderText,
}

/// Paint `computed`'s outer shadows around the border box `outer`.
pub(super) fn paint_outer_shadows(
    buf: &mut Buffer,
    computed: &ComputedStyle,
    outer: LayoutRect,
    clip: Rect,
    pass: Shadows,
) {
    let outer = Edges::of(outer);
    for s in computed.box_shadow.iter().rev().filter(|s| !s.inset) {
        let shade = outer.offset(s).grow(s.spread.offset_cells());
        for part in shade.minus(outer) {
            fill(buf, part, s.color, clip, pass);
        }
    }
}

/// The background phase of a paint unit: the outer shadows of its
/// in-flow boxes (`stacking::collect_layers`), in tree order.
pub(super) fn paint_backdrop_shadows(dom: &Dom<TuiExt>, boxes: &[ShadowEntry], buf: &mut Buffer) {
    for e in boxes {
        let node = dom.node(e.id);
        if node.node_type() != NodeType::Element {
            continue;
        }
        let Some(computed) = node.computed() else {
            continue;
        };
        let outer = node.layout_rect().unwrap_or_default();
        paint_outer_shadows(buf, computed, outer, e.clip, Shadows::Backdrop);
    }
}

/// Paint `computed`'s inset shadows inside the padding box of the
/// border box `outer`.
pub(super) fn paint_inset_shadows(
    buf: &mut Buffer,
    computed: &ComputedStyle,
    outer: LayoutRect,
    clip: Rect,
) {
    let padding_box = Edges::of(compute_padding_box(outer, computed.border));
    for s in computed.box_shadow.iter().rev().filter(|s| s.inset) {
        let hole = padding_box.offset(s).grow(-s.spread.offset_cells());
        for part in padding_box.minus(hole) {
            fill(buf, part, s.color, clip, Shadows::Whole);
        }
    }
}

/// A rectangle by its edges, in signed cells: a grown shade may be
/// wider than a `LayoutRect`'s `u16` extent. Every operation saturates;
/// offsets and spread are at most `u16::MAX` either way
/// (`PaintLength::offset_cells`), so nothing comes near the limits.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Edges {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Edges {
    fn of(r: LayoutRect) -> Edges {
        Edges {
            left: r.x,
            top: r.y,
            right: r.x.saturating_add(i32::from(r.width)),
            bottom: r.y.saturating_add(i32::from(r.height)),
        }
    }

    fn is_empty(self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    /// Moved by the shadow's offsets.
    fn offset(self, s: &BoxShadow<Color>) -> Edges {
        let (dx, dy) = (s.offset_x.offset_cells(), s.offset_y.offset_cells());
        Edges {
            left: self.left.saturating_add(dx),
            top: self.top.saturating_add(dy),
            right: self.right.saturating_add(dx),
            bottom: self.bottom.saturating_add(dy),
        }
    }

    /// Grown by `by` cells on every side (shrunk when negative; a
    /// shrunk-away rectangle is empty).
    fn grow(self, by: i32) -> Edges {
        Edges {
            left: self.left.saturating_sub(by),
            top: self.top.saturating_sub(by),
            right: self.right.saturating_add(by),
            bottom: self.bottom.saturating_add(by),
        }
    }

    /// The parts of `self` outside `other`: up to four bands.
    fn minus(self, other: Edges) -> impl Iterator<Item = Edges> {
        let i = Edges {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        };
        let bands = if i.is_empty() || other.is_empty() {
            [Some(self), None, None, None]
        } else {
            let band = |left, top, right, bottom| {
                let e = Edges {
                    left,
                    top,
                    right,
                    bottom,
                };
                (!e.is_empty()).then_some(e)
            };
            [
                band(self.left, self.top, self.right, i.top),
                band(self.left, i.bottom, self.right, self.bottom),
                band(self.left, i.top, i.left, i.bottom),
                band(i.right, i.top, self.right, i.bottom),
            ]
        };
        bands.into_iter().flatten().filter(|e| !e.is_empty())
    }

    /// The cells inside `clip`, if any.
    fn clipped(self, clip: Rect) -> Option<Rect> {
        let left = self.left.max(i32::from(clip.x));
        let top = self.top.max(i32::from(clip.y));
        let right = self.right.min(i32::from(clip.right()));
        let bottom = self.bottom.min(i32::from(clip.bottom()));
        // Inside `clip`, so every bound fits a `u16`.
        (left < right && top < bottom).then(|| {
            Rect::new(
                left as u16,
                top as u16,
                (right - left) as u16,
                (bottom - top) as u16,
            )
        })
    }
}

/// Fill `part` with `color` as an opaque shade over what is beneath (or
/// under its glyphs, `UnderText`), or a translucent one composited over
/// it (C3-ALPHA) — once, not in the `Backdrop` pass.
fn fill(buf: &mut Buffer, part: Edges, color: Color, clip: Rect, pass: Shadows) {
    if !super::fills(color) {
        return;
    }
    let Some(area) = part.clipped(clip) else {
        return;
    };
    match (color.is_translucent(), pass) {
        (true, Shadows::Backdrop) => {}
        (true, _) => {
            let alpha = f32::from(color.alpha()) / 255.0;
            buf.paint_translucent(area, alpha, |layer| fill_bg(layer, area, color.opaque()));
        }
        (false, Shadows::UnderText) => tint_bg(buf, area, color),
        (false, _) => fill_bg(buf, area, color),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edges(x: i32, y: i32, w: u16, h: u16) -> Edges {
        Edges::of(LayoutRect::new(x, y, w, h))
    }

    /// The parts of a rect outside another cover exactly the cells of
    /// the first not in the second.
    #[test]
    fn minus_leaves_the_cells_outside() {
        let a = edges(0, 0, 4, 3);
        let b = edges(1, 1, 2, 5);
        let mut cells: Vec<(i32, i32)> = a
            .minus(b)
            .flat_map(|r| {
                (r.top..r.bottom).flat_map(move |y| (r.left..r.right).map(move |x| (x, y)))
            })
            .collect();
        cells.sort();
        let mut expected: Vec<(i32, i32)> = (0..3)
            .flat_map(|y| (0..4).map(move |x| (x, y)))
            .filter(|&(x, y)| !(1..3).contains(&x) || y < 1)
            .collect();
        expected.sort();
        assert_eq!(cells, expected);
        assert_eq!(a.minus(edges(9, 9, 1, 1)).collect::<Vec<_>>(), vec![a]);
        // Less an empty rectangle: all of it.
        assert_eq!(a.minus(a.grow(-5)).collect::<Vec<_>>(), vec![a]);
    }

    /// `C4G-SHADOW-CLAMP`: growing past a `u16` extent keeps every edge
    /// (no clamp of the far edge), and the extremes saturate.
    #[test]
    fn grow_keeps_edges_past_a_u16_extent() {
        let max = i32::from(u16::MAX);
        let g = edges(1, 1, 2, 1).grow(max);
        assert_eq!((g.left, g.right), (1 - max, 3 + max));
        let far = Edges {
            left: i32::MIN,
            top: 0,
            right: i32::MAX,
            bottom: 1,
        };
        assert_eq!(far.grow(max).left, i32::MIN);
        assert_eq!(far.grow(max).right, i32::MAX);
        assert_eq!(
            g.clipped(Rect::new(0, 0, 6, 5)),
            Some(Rect::new(0, 0, 6, 5))
        );
    }
}
