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

/// Shrink `inner` by a 1-cell scrollbar gutter per axis when CSS
/// `scrollbar-gutter` says to reserve it (or when `overflow:
/// scroll` requires a permanent gutter).
///
/// Reservation rules per axis:
/// - `Overflow::Scroll` → always reserve (scrollbar always shown).
/// - `Overflow::Auto` + `scrollbar-gutter: stable` → reserve
///   (matches CSS `scrollbar-gutter: stable` — prevents content
///   reflow when the scrollbar appears mid-frame).
/// - `Overflow::Auto` + `scrollbar-gutter: auto` (the CSS
///   default) → DO NOT reserve. The scrollbar paints over the
///   edge column/row only while it's visible; content gets the
///   cells when scrolling isn't active. Authors who want stable
///   layout opt in with `scrollbar-gutter: stable`.
/// - `Overflow::Hidden` / `Clip` / `Visible` → never reserve.
///
/// The reserved cells live at:
/// - **Vertical scrollbar** (if `overflow_y` reserves): the
///   rightmost column of `inner`, from top to bottom — the leftmost
///   under `direction: rtl` ([`bar_on_left`]).
/// - **Horizontal scrollbar** (if `overflow_x` reserves): the
///   bottom row of `inner`, from left to right.
///
/// When both reserve, the bottom-right corner cell is unclaimed
/// by either strip — paint leaves it blank.
///
/// `force_y` / `force_x` override the cascade decision for `Auto`
/// axes — used by `layout_node`'s two-pass re-layout when overflow
/// was detected in pass 1. `Scroll` always reserves regardless; CSS
/// Overflow 3 §3 "classic" semantic for `Auto` ("consumes space when
/// present") needs the override because at the time of pass 1 the
/// substrate doesn't yet know if overflow will exist. Two-pass:
/// measure → if overflow on an Auto axis, force-reserve in pass 2.
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

/// The column of `computed`'s vertical scrollbar beside its `content`
/// area (the gutter [`reserve_scrollbar_gutter_forced`] reserved): just
/// right of it, or just left of it under [`bar_on_left`].
pub(crate) fn vertical_bar_column(content: LayoutRect, computed: &ComputedStyle) -> i32 {
    if bar_on_left(computed) {
        content.x - 1
    } else {
        content.x + i32::from(content.width)
    }
}

/// Pass-1 gutter reservation — Scroll always, Auto only if
/// `scrollbar-gutter: stable`. `Auto` without `stable` waits for
/// overflow detection then forces the gutter in pass 2 via
/// [`reserve_scrollbar_gutter_forced`].
pub(crate) fn reserve_scrollbar_gutter(inner: LayoutRect, computed: &ComputedStyle) -> LayoutRect {
    reserve_scrollbar_gutter_forced(inner, computed, false, false)
}
