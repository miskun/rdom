//! An anchor-positioned box's style resolved against its anchors (CSS
//! Anchor Positioning 1): its `position-area` made a containing block
//! (§3.1.1) with its default alignment (§3.1.3), its `anchor()` and
//! `anchor-size()` made cells (§5.1, §5.2), and `anchor-center` made a
//! containing block centred on the anchor (§3.4) — then placed as any
//! absolutely positioned box is (`place::compute_placed_rect`).

use rdom_core::{Dom, NodeId};

use rdom_style::calc::{AnchorFunction, AnchorSide, AnchorSize, CalcExpr};

use super::lookup::{AnchorIndex, Querying, anchor_box};
use crate::ext::TuiExt;
use crate::layout::{
    Align, Alignment, AreaTracks, LayoutRect, Length, MarginValue, MaxSize, MinSize, PositionArea,
    Size,
};
use crate::style::ComputedStyle;

/// What resolving a box's anchor references needs: the box, its
/// containing block (element and rect), its default anchor's box, and the
/// directions that orient `start` / `end`.
pub(in crate::render::layout_pass) struct Anchoring<'a> {
    pub(in crate::render::layout_pass) dom: &'a Dom<TuiExt>,
    pub(in crate::render::layout_pass) index: &'a AnchorIndex,
    /// Who asks: the element, or a pseudo-element's host.
    pub(in crate::render::layout_pass) querying: Querying,
    /// The containing block's element (`None`: the viewport).
    pub(in crate::render::layout_pass) cb_element: Option<NodeId>,
    pub(in crate::render::layout_pass) cb: LayoutRect,
    /// The default anchor's box (§2.3), when it has one.
    pub(in crate::render::layout_pass) default: Option<LayoutRect>,
    /// The containing block's `direction` is `rtl`.
    pub(in crate::render::layout_pass) cb_rtl: bool,
    /// The box's own is.
    pub(in crate::render::layout_pass) self_rtl: bool,
}

/// Which inset (or axis) a value belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

impl Side {
    fn horizontal(self) -> bool {
        matches!(self, Side::Left | Side::Right)
    }

    /// Whether the inset is measured from the axis's end edge (bottom,
    /// right).
    fn at_end(self) -> bool {
        matches!(self, Side::Right | Side::Bottom)
    }
}

fn bottom(r: LayoutRect) -> i32 {
    r.y + i32::from(r.height)
}

fn right(r: LayoutRect) -> i32 {
    r.x + i32::from(r.width)
}

impl Anchoring<'_> {
    /// The box of the anchor `name` names, or the default anchor's.
    fn anchor(&self, name: Option<&str>) -> Option<LayoutRect> {
        match name {
            None => self.default,
            Some(n) => self
                .index
                .find(self.dom, self.querying, self.cb_element, n)
                .and_then(|a| anchor_box(self.dom, a)),
        }
    }

    /// `f` in the inset `side` (§5.1) or a size / margin on its axis
    /// (§5.2), in cells: the inset's distance from its containing block
    /// edge `cb`, or the anchor's size. `None` without an acceptable
    /// anchor, or for a side that is not on the inset's axis.
    fn resolve(&self, f: &AnchorFunction, side: Side, cb: LayoutRect) -> Option<i32> {
        match f {
            AnchorFunction::Edge { name, side: s, .. } => {
                let a = self.anchor(name.as_deref())?;
                let horizontal = side.horizontal();
                let rtl_axis = horizontal && self.cb_rtl;
                // The fraction of the anchor from its physical start
                // (left / top) edge.
                let t = match *s {
                    AnchorSide::Top | AnchorSide::Bottom if horizontal => return None,
                    AnchorSide::Left | AnchorSide::Right if !horizontal => return None,
                    AnchorSide::Top | AnchorSide::Left => 0.0,
                    AnchorSide::Bottom | AnchorSide::Right => 1.0,
                    AnchorSide::Inside => f64::from(u8::from(side.at_end())),
                    AnchorSide::Outside => f64::from(u8::from(!side.at_end())),
                    AnchorSide::Start => f64::from(u8::from(rtl_axis)),
                    AnchorSide::End => f64::from(u8::from(!rtl_axis)),
                    AnchorSide::SelfStart => f64::from(u8::from(horizontal && self.self_rtl)),
                    AnchorSide::SelfEnd => f64::from(u8::from(!(horizontal && self.self_rtl))),
                    AnchorSide::Center => 0.5,
                    AnchorSide::Percent(p) if rtl_axis => 1.0 - p / 100.0,
                    AnchorSide::Percent(p) => p / 100.0,
                };
                let (start, extent) = if horizontal {
                    (a.x, a.width)
                } else {
                    (a.y, a.height)
                };
                let edge = start + rdom_style::calc::to_cells(t * f64::from(extent));
                Some(match side {
                    Side::Top => edge - cb.y,
                    Side::Left => edge - cb.x,
                    Side::Bottom => bottom(cb) - edge,
                    Side::Right => right(cb) - edge,
                })
            }
            AnchorFunction::Size { name, size, .. } => {
                let a = self.anchor(name.as_deref())?;
                let width = match size {
                    Some(AnchorSize::Width | AnchorSize::Inline | AnchorSize::SelfInline) => true,
                    Some(AnchorSize::Height | AnchorSize::Block | AnchorSize::SelfBlock) => false,
                    None => side.horizontal(),
                };
                Some(i32::from(if width { a.width } else { a.height }))
            }
            // A function a later level adds: its fallback.
            _ => None,
        }
    }

    /// `e` with its anchor functions resolved for `side` against `cb`;
    /// `None` when one has no anchor and no fallback.
    fn substitute(&self, e: &CalcExpr, side: Side, cb: LayoutRect) -> Option<CalcExpr> {
        e.substitute_anchors(&mut |f| self.resolve(f, side, cb))
    }

    /// The containing block `position-area` makes (§3.1.1): the area of
    /// the 3 × 3 grid of the containing block's and the default anchor's
    /// edges it spans. `None` without a default anchor (the property then
    /// has no effect).
    fn area(&self, area: &PositionArea) -> Option<(LayoutRect, AreaTracks)> {
        let a = self.default?;
        let cb = self.cb;
        let t = area.tracks(self.cb_rtl, self.self_rtl);
        let clamp_x = |x: i32| x.clamp(cb.x, right(cb));
        let clamp_y = |y: i32| y.clamp(cb.y, bottom(cb));
        let xs = [
            cb.x,
            clamp_x(a.x),
            clamp_x(right(a)).max(clamp_x(a.x)),
            right(cb),
        ];
        let ys = [
            cb.y,
            clamp_y(a.y),
            clamp_y(bottom(a)).max(clamp_y(a.y)),
            bottom(cb),
        ];
        let (x0, x1) = (
            xs[usize::from(t.columns.0)],
            xs[usize::from(t.columns.1) + 1],
        );
        let (y0, y1) = (ys[usize::from(t.rows.0)], ys[usize::from(t.rows.1) + 1]);
        let rect = LayoutRect::new(
            x0,
            y0,
            (x1 - x0).clamp(0, i32::from(u16::MAX)) as u16,
            (y1 - y0).clamp(0, i32::from(u16::MAX)) as u16,
        );
        Some((rect, t))
    }
}

/// §3.1.3: the `normal` self-alignment in an axis whose tracks the area
/// spans `(first, last)` — towards the anchor, centred on it, or across
/// it — physically (`horizontal`: `left` / `right`; else `start` / `end`,
/// the block axis running top to bottom).
fn area_alignment(tracks: (u8, u8), horizontal: bool) -> Align {
    let (start, end) = if horizontal {
        (Align::Left, Align::Right)
    } else {
        (Align::Start, Align::End)
    };
    match tracks {
        (1, 1) => Align::Center,
        (0, 2) => Align::AnchorCenter,
        (0, _) => end,
        _ => start,
    }
}

/// Whether `computed` asks anything of the anchor layout: an anchor
/// function, `position-area`, `anchor-center` or a fallback.
pub(in crate::render::layout_pass) fn is_anchored(c: &ComputedStyle) -> bool {
    let calc = |e: Option<&CalcExpr>| e.is_some_and(CalcExpr::contains_anchor);
    let length = |l: &Length| {
        calc(match l {
            Length::Calc(e) => Some(e),
            _ => None,
        })
    };
    let size = |s: &Size| {
        calc(match s {
            Size::Calc(e) => Some(e),
            _ => None,
        })
    };
    let min = |s: &MinSize| {
        calc(match s {
            MinSize::Calc(e) => Some(e),
            _ => None,
        })
    };
    let max = |s: &MaxSize| {
        calc(match s {
            MaxSize::Calc(e) => Some(e),
            _ => None,
        })
    };
    let margin = |m: &MarginValue| {
        calc(match m {
            MarginValue::Calc(e) => Some(e),
            _ => None,
        })
    };
    c.anchor.is_anchored()
        || c.justify_self.keyword == Align::AnchorCenter
        || c.align_self.keyword == Align::AnchorCenter
        || [&c.top, &c.right, &c.bottom, &c.left]
            .into_iter()
            .any(length)
        || [&c.width, &c.height].into_iter().any(size)
        || [&c.min_width, &c.min_height].into_iter().any(min)
        || [&c.max_width, &c.max_height].into_iter().any(max)
        || [
            &c.margin.top,
            &c.margin.right,
            &c.margin.bottom,
            &c.margin.left,
        ]
        .into_iter()
        .any(margin)
}

/// `style` resolved against its anchors for placement (module doc): the
/// style to place by and the containing block to place in.
pub(in crate::render::layout_pass) fn resolve_style(
    style: &ComputedStyle,
    an: &Anchoring<'_>,
) -> (ComputedStyle, LayoutRect) {
    let mut s = style.clone();
    let mut cb = an.cb;
    // §3.1: the area is the containing block; an `auto` inset is 0 in it,
    // and `normal` self-alignment hugs the anchor (§3.1.3).
    if let Some(area) = style.anchor.position_area
        && let Some((rect, AreaTracks { columns, rows })) = an.area(&area)
    {
        cb = rect;
        for inset in [&mut s.top, &mut s.right, &mut s.bottom, &mut s.left] {
            if *inset == Length::Auto {
                *inset = Length::Cells(0);
            }
        }
        // An `auto` margin would centre the box in the area (HTML's popover
        // `margin: auto` most of all): 0, so the box hugs its anchor
        // (DIVERGENCES).
        let m = &mut s.margin;
        for margin in [&mut m.top, &mut m.right, &mut m.bottom, &mut m.left] {
            if margin.is_auto() {
                *margin = MarginValue::Cells(0);
            }
        }
        let normal = |a: Alignment| matches!(a.keyword, Align::Normal | Align::Auto);
        if normal(s.justify_self) {
            s.justify_self = Alignment::new(area_alignment(columns, true));
        }
        if normal(s.align_self) {
            s.align_self = Alignment::new(area_alignment(rows, false));
        }
    }
    let sides = [
        (&mut s.top, Side::Top),
        (&mut s.right, Side::Right),
        (&mut s.bottom, Side::Bottom),
        (&mut s.left, Side::Left),
    ];
    for (inset, side) in sides {
        if let Length::Calc(e) = inset
            && e.contains_anchor()
        {
            *inset = an
                .substitute(e, side, cb)
                .map_or(Length::Auto, Length::calc);
        }
    }
    for (size, side) in [(&mut s.width, Side::Left), (&mut s.height, Side::Top)] {
        if let Size::Calc(e) = size
            && e.contains_anchor()
        {
            *size = an.substitute(e, side, cb).map_or(Size::Auto, Size::calc);
        }
    }
    for (min, side) in [
        (&mut s.min_width, Side::Left),
        (&mut s.min_height, Side::Top),
    ] {
        if let MinSize::Calc(e) = min
            && e.contains_anchor()
        {
            *min = an
                .substitute(e, side, cb)
                .map_or(MinSize::Auto, MinSize::calc);
        }
    }
    for (max, side) in [
        (&mut s.max_width, Side::Left),
        (&mut s.max_height, Side::Top),
    ] {
        if let MaxSize::Calc(e) = max
            && e.contains_anchor()
        {
            *max = an
                .substitute(e, side, cb)
                .map_or(MaxSize::None, MaxSize::calc);
        }
    }
    let m = &mut s.margin;
    let margins = [
        (&mut m.top, Side::Top),
        (&mut m.right, Side::Right),
        (&mut m.bottom, Side::Bottom),
        (&mut m.left, Side::Left),
    ];
    for (margin, side) in margins {
        if let MarginValue::Calc(e) = margin
            && e.contains_anchor()
        {
            *margin = an
                .substitute(e, side, cb)
                .map_or(MarginValue::Cells(0), MarginValue::calc);
        }
    }
    // §3.4: `anchor-center` centres the box on its default anchor — in the
    // largest part of its inset-modified containing block centred on the
    // anchor.
    if let Some(a) = an.default {
        if s.justify_self.keyword == Align::AnchorCenter {
            let (left, right) = centred(&s.left, &s.right, cb.x, cb.width, a.x, a.width);
            (s.left, s.right) = (left, right);
        }
        if s.align_self.keyword == Align::AnchorCenter {
            let (top, bottom) = centred(&s.top, &s.bottom, cb.y, cb.height, a.y, a.height);
            (s.top, s.bottom) = (top, bottom);
        }
    }
    (s, cb)
}

/// The insets that make the inset-modified containing block (on an axis
/// from `cb_start`, `cb_extent` long, its insets `start` / `end`, `auto`
/// as 0) as large as it can be while centred on the anchor `[a_start,
/// a_start + a_extent)`.
fn centred(
    start: &Length,
    end: &Length,
    cb_start: i32,
    cb_extent: u16,
    a_start: i32,
    a_extent: u16,
) -> (Length, Length) {
    let basis = i32::from(cb_extent);
    let lo = cb_start + start.cells(basis).unwrap_or(0);
    let hi = cb_start + basis - end.cells(basis).unwrap_or(0);
    // Twice the anchor's centre line, in half cells.
    let centre2 = 2 * a_start + i32::from(a_extent);
    let half2 = (centre2 - 2 * lo).min(2 * hi - centre2).max(0);
    let (from, to) = ((centre2 - half2) / 2, (centre2 + half2).div_euclid(2));
    (
        Length::Cells(from - cb_start),
        Length::Cells(cb_start + basis - to),
    )
}
