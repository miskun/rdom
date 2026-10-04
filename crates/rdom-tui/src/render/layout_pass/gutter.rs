//! Scroll offsets and scrollbar gutters: a scrolled parent's offset
//! for its children, and the gutter cells a scroll container reserves
//! out of its content area.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{Direction, LayoutRect, Overflow};
use crate::style::ComputedStyle;

/// Scroll offset for the parent container along `direction`. Reads
/// the *first* child's ext-parent to find the scroll config — since
/// all children share the same parent.
pub(crate) fn parent_scroll(dom: &Dom<TuiExt>, children: &[NodeId], direction: Direction) -> i32 {
    let Some(&first) = children.first() else {
        return 0;
    };
    let Some(parent) = dom.node(first).parent_node() else {
        return 0;
    };
    let Some(ext) = parent.ext() else { return 0 };
    match direction {
        Direction::Row => ext.scroll_x as i32,
        Direction::Column => ext.scroll_y as i32,
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
/// - `Overflow::Hidden` / `Visible` → never reserve.
///
/// The reserved cells live at:
/// - **Vertical scrollbar** (if `overflow_y` reserves): the
///   rightmost column of `inner`, from top to bottom.
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
    let (reserve_y, reserve_x) = gutter_axes(computed, force_y, force_x);
    LayoutRect::new(
        inner.x,
        inner.y,
        if reserve_y {
            inner.width.saturating_sub(1)
        } else {
            inner.width
        },
        if reserve_x {
            inner.height.saturating_sub(1)
        } else {
            inner.height
        },
    )
}

/// Which axes reserve a scrollbar gutter: `(vertical bar, horizontal
/// bar)`. `Scroll` always; `Auto` when forced (pass 2 saw overflow) or
/// under `scrollbar-gutter: stable`; never otherwise.
pub(crate) fn gutter_axes(computed: &ComputedStyle, force_y: bool, force_x: bool) -> (bool, bool) {
    use crate::layout::ScrollbarGutter;
    let reserves = |o: Overflow, force: bool| match o {
        Overflow::Scroll => true,
        Overflow::Auto => force || matches!(computed.scrollbar_gutter, ScrollbarGutter::Stable),
        Overflow::Hidden | Overflow::Visible => false,
    };
    (
        reserves(computed.overflow_y, force_y),
        reserves(computed.overflow_x, force_x),
    )
}

/// Pass-1 gutter reservation — Scroll always, Auto only if
/// `scrollbar-gutter: stable`. `Auto` without `stable` waits for
/// overflow detection then forces the gutter in pass 2 via
/// [`reserve_scrollbar_gutter_forced`].
pub(crate) fn reserve_scrollbar_gutter(inner: LayoutRect, computed: &ComputedStyle) -> LayoutRect {
    reserve_scrollbar_gutter_forced(inner, computed, false, false)
}
