//! Scroll offsets and scrollbar gutters: a scrolled parent's offset
//! for its children, and the gutter cells a scroll container reserves
//! out of its content area.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{Direction, LayoutRect, Overflow};
use crate::style::ComputedStyle;

/// `container`'s own scroll offset along `direction`.
pub(crate) fn scroll_offset(dom: &Dom<TuiExt>, container: NodeId, direction: Direction) -> i32 {
    let Some(ext) = dom.node(container).ext() else {
        return 0;
    };
    match direction {
        Direction::Row => ext.scroll_x,
        Direction::Column => ext.scroll_y,
    }
}

/// Shrink the content area `inner` by the scrollbar gutters (CSS
/// Overflow 3 §5.2) [`gutters`] reserves, the `auto` axes forced as
/// `force_y` / `force_x` say — `layout_node`'s second pass forces an
/// `auto` axis whose content overflowed in the first (classic scrollbars
/// take space when present; a terminal cannot overlay one).
///
/// The gutters lie between the padding edge and the border: the vertical
/// bar's column at the padding box's inline-end edge (its left edge under
/// `direction: rtl`, [`bar_on_left`]), the horizontal bar's row at its
/// bottom. The content box keeps its padding on the scrollport's side of
/// them, so it is the content area less a column (row) — the same rect
/// whichever side of the padding the gutter is on; the scrollport
/// (`scrollport`) is the padding box less the gutters. When both bars
/// show, the corner cell where the gutters meet belongs to neither.
pub(crate) fn reserve_scrollbar_gutter_forced(
    inner: LayoutRect,
    computed: &ComputedStyle,
    force_y: bool,
    force_x: bool,
) -> LayoutRect {
    let g = gutters(computed, force_y, force_x);
    LayoutRect::new(
        inner.x + i32::from(g.left),
        inner.y,
        inner.width.saturating_sub(g.columns()),
        inner.height.saturating_sub(g.bottom),
    )
}

/// The gutter cells a box reserves around its content (CSS Overflow 3
/// §3.3): a column on each inline edge that has one, a row at the bottom.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Gutters {
    pub(crate) left: u16,
    pub(crate) right: u16,
    pub(crate) bottom: u16,
}

impl Gutters {
    /// The columns the gutters take from the content's width.
    pub(crate) fn columns(self) -> u16 {
        self.left + self.right
    }
}

/// `computed`'s gutters, the `auto` axes forced as [`gutter_axes`] says:
/// the vertical bar's column on its side ([`bar_on_left`]) and, under
/// `scrollbar-gutter: stable both-edges`, its twin on the opposite
/// inline edge ("if a gutter would be present on one of the inline start
/// edge or the inline end edge of the box, another gutter must be present
/// on the opposite edge as well"); the horizontal bar's row.
pub(crate) fn gutters(computed: &ComputedStyle, force_y: bool, force_x: bool) -> Gutters {
    let (y, x) = gutter_axes(computed, force_y, force_x);
    let both = y && computed.scrollbar_gutter.both_edges();
    let left = bar_on_left(computed);
    Gutters {
        left: u16::from(y && (left || both)),
        right: u16::from(y && (!left || both)),
        bottom: u16::from(x),
    }
}

/// Which axes reserve a scrollbar gutter: `(vertical bar, horizontal
/// bar)`. `Scroll` always; `Auto` when forced (pass 2 saw overflow); and
/// for the vertical bar, under `scrollbar-gutter: stable`, `auto` and
/// `hidden` too (CSS Overflow 3 §3.3: "present for overflow: hidden,
/// scroll, or auto, regardless of whether a scrollbar is actually
/// present"). None under `scrollbar-width: none` (CSS Scrollbars 1 §3: no
/// scrollbar, so no gutter to hold one).
pub(crate) fn gutter_axes(computed: &ComputedStyle, force_y: bool, force_x: bool) -> (bool, bool) {
    if computed.scrollbar_width == crate::layout::ScrollbarWidth::None {
        return (false, false);
    }
    // `scrollbar-gutter` governs the gutters at the inline-start and
    // inline-end edges only (CSS Overflow 3 §3.3) — the vertical bar's in
    // `horizontal-tb`; the horizontal bar's row is reserved only when it
    // shows.
    let stable = computed.scrollbar_gutter.is_stable();
    let reserves = |o: Overflow, force: bool, stable: bool| match o {
        Overflow::Scroll => true,
        Overflow::Auto => force || stable,
        Overflow::Hidden => stable,
        Overflow::Clip | Overflow::Visible => false,
    };
    (
        reserves(computed.overflow_y, force_y, stable),
        reserves(computed.overflow_x, force_x, false),
    )
}

/// Whether `computed`'s vertical scrollbar sits on its left: under
/// `direction: rtl`, on the inline-start side, as Chromium and Gecko
/// place it (CSS Overflow 3 leaves the side to the UA).
pub(crate) fn bar_on_left(computed: &ComputedStyle) -> bool {
    computed.text_direction == crate::layout::TextDirection::Rtl
}

/// Pass-1 gutter reservation ([`gutter_axes`]): `scroll` always; `auto`
/// and `hidden` under `scrollbar-gutter: stable` (CSS Overflow 3 §3.3).
/// `auto` without `stable` waits for overflow detection then forces the
/// gutter in pass 2 via [`reserve_scrollbar_gutter_forced`].
pub(crate) fn reserve_scrollbar_gutter(inner: LayoutRect, computed: &ComputedStyle) -> LayoutRect {
    reserve_scrollbar_gutter_forced(inner, computed, false, false)
}
