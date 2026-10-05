//! Box Alignment in block layout (CSS Box Alignment 3 §5.1, §6.1–§6.2):
//! `justify-self` places a block-level box in its containing block's
//! inline axis (as Chromium 130 ships it), and `align-content` shifts a
//! block container's content in its block axis (Chromium 123).
//! `align-self` does not apply to block-level boxes (§6.2).

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{Align, Alignment, LayoutRect, Overflow, OverflowAlign, Size, TextDirection};
use crate::node::TuiNodeExt;
use crate::style::ComputedStyle;

/// A block-level box's effective `justify-self` (§6.1): its own, or for
/// `auto` its parent's `justify-items` — a `legacy` value as its side.
pub(super) fn justify_self_of(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
) -> Alignment {
    if computed.justify_self.keyword != Align::Auto {
        return computed.justify_self;
    }
    let items = crate::render::box_tree::box_parent(dom, id)
        .and_then(|p| dom.node(p).computed().map(|c| c.justify_items))
        .unwrap_or(Alignment::NORMAL);
    Alignment {
        legacy: false,
        ..items
    }
}

/// Whether `value` aligns the box — anything but `normal` / `stretch`,
/// which leave CSS 2.1 §10.3.3's width and margins as they are.
pub(super) fn aligns(value: Alignment) -> bool {
    !matches!(value.keyword, Align::Normal | Align::Stretch | Align::Auto)
}

/// The margin-left offset that places a box `free` cells narrower than
/// its containing block (negative: wider) by `value` (§6.1, §4.2):
/// `start` / `end` by the containing block's direction (`rtl`),
/// `self-start` / `self-end` by the box's own (`self_rtl`), `left` /
/// `right` physically, `center` with the leading space rounded down;
/// `flex-start` / `flex-end` are `start` / `end` outside flex, and the
/// baseline values fall back to `safe start` / `safe end` (§9.3). A
/// `safe` value that would overflow aligns as `start` (§4.4).
pub(super) fn justify_offset(value: Alignment, free: i32, rtl: bool, self_rtl: bool) -> i32 {
    let start = if rtl { free } else { 0 };
    let end = if rtl { 0 } else { free };
    let safe = value.overflow == OverflowAlign::Safe
        || matches!(value.keyword, Align::Baseline | Align::LastBaseline);
    if safe && free < 0 {
        return start;
    }
    match value.keyword {
        Align::Start | Align::FlexStart | Align::Baseline => start,
        Align::End | Align::FlexEnd | Align::LastBaseline => end,
        Align::SelfStart if self_rtl => free,
        Align::SelfStart => 0,
        Align::SelfEnd if self_rtl => 0,
        Align::SelfEnd => free,
        Align::Left => 0,
        Align::Right => free,
        Align::Center => free.div_euclid(2),
        // `normal` / `stretch` / `auto` do not align (`aligns`); the
        // distributions are not in `justify-self`'s grammar.
        Align::Normal
        | Align::Stretch
        | Align::Auto
        | Align::SpaceBetween
        | Align::SpaceAround
        | Align::SpaceEvenly => start,
    }
}

/// Whether `id`'s text direction is `rtl`.
pub(super) fn is_rtl(computed: &ComputedStyle) -> bool {
    computed.text_direction == TextDirection::Rtl
}

/// §5.1: the block-axis offset `align-content` gives a block container's
/// content (`content_height` cells of it) in its content box `inner` —
/// 0 for `normal`, and for a box whose height is its content's (an
/// `auto` height in block flow, resolved from the content after this
/// pass). `start`, `flex-start`, `stretch`, `space-between` and
/// `baseline` keep it at the top; `end`, `flex-end` and `last baseline`
/// put it at the bottom; `center`, `space-around` and `space-evenly`
/// center it (the leading space rounded down). Content taller than the
/// box overflows as asked, unless `safe` or the box scrolls (its
/// overflow is reached from its start).
pub(in crate::render::layout_pass) fn align_content_lead(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    inner: LayoutRect,
    content_height: u16,
) -> i32 {
    let value = computed.align_content;
    if value.keyword == Align::Normal || !computed.flow.is_block_flow() {
        return 0;
    }
    let content_sized = matches!(computed.height, Size::Auto | Size::Intrinsic(_))
        && !matches!(
            computed.position,
            crate::layout::Position::Absolute | crate::layout::Position::Fixed
        )
        && crate::render::box_tree::box_parent(dom, id)
            .and_then(|p| dom.node(p).computed().map(|c| c.flow))
            .is_none_or(|f| f.is_block_flow());
    if content_sized {
        return 0;
    }
    let free = i32::from(inner.height) - i32::from(content_height);
    if free < 0
        && (value.overflow == OverflowAlign::Safe || computed.overflow_y != Overflow::Visible)
    {
        return 0;
    }
    match value.keyword {
        Align::End | Align::FlexEnd | Align::LastBaseline => free,
        Align::Center | Align::SpaceAround | Align::SpaceEvenly => free.div_euclid(2),
        _ => 0,
    }
}
