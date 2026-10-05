//! The scrollable content extent of a box: the union of its children's
//! rects recorded for scrolling (`scroll_content_{width,height}`), the
//! legal scroll offsets and their clamp, and the trailing caret row of
//! an editing host.

use rdom_core::{Dom, NodeId, NodeType};

use super::{element_children_of, is_in_flow};
use crate::ext::TuiExt;
use crate::layout::{LayoutRect, Overflow};
use crate::style::ComputedStyle;

/// Walk `id`'s direct element children (transparently descending
/// through nested Fragments, the same way `element_children_of`
/// does) and write the union of their layout extents back to
/// `id`'s `TuiExt.scroll_content_{width,height}` — with the
/// parent's `scroll_{x,y}` *added back in* so the recorded size
/// is the un-scrolled content extent.
pub(crate) fn record_scroll_content_size(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    inner: LayoutRect,
    computed: &ComputedStyle,
) {
    // Static early-exit: only scrollable containers care.
    let needs = matches!(
        computed.overflow_x,
        Overflow::Scroll | Overflow::Auto | Overflow::Hidden
    ) || matches!(
        computed.overflow_y,
        Overflow::Scroll | Overflow::Auto | Overflow::Hidden
    );
    if !needs {
        return;
    }

    // Parent's own scroll offset — children's layout rects had this
    // subtracted from their main-axis cursor (see flex.rs::
    // layout_flex_children). Add it back to compute the un-scrolled
    // content extent.
    let (scroll_x, scroll_y) = match dom.node(id).ext() {
        Some(ext) => (ext.scroll_x, ext.scroll_y),
        None => return,
    };

    // Content extent = max(child.bottom) - min(child.top) along each
    // axis, with the parent's `scroll_{x,y}` added back so the result
    // is the un-scrolled extent. The min/max framing (rather than
    // anchoring on `inner.{x,y}`) is what makes `collapse_parent_edge_insets`'s
    // top/left layout-time shifts cleanly ignored: those insets push
    // the first child away from `inner` but the children's collective
    // extent is what overflow actually depends on, and that extent
    // is `max - min` regardless of where the first child sits inside
    // the inner rect.
    let mut min_x: Option<i32> = None;
    let mut min_y: Option<i32> = None;
    let mut max_right: i32 = 0;
    let mut max_bottom: i32 = 0;
    let mut any = false;
    let mut extend = |rect: LayoutRect| {
        let top = rect.y + scroll_y;
        let left = rect.x + scroll_x;
        min_x = Some(min_x.map_or(left, |m: i32| m.min(left)));
        min_y = Some(min_y.map_or(top, |m: i32| m.min(top)));
        max_right = max_right.max(left + rect.width as i32);
        max_bottom = max_bottom.max(top + rect.height as i32);
        any = true;
    };
    // CSS Overflow 3 §2.2: the scrollable overflow area covers the
    // in-flow descendants' boxes, not only the children's — a row that
    // stretches to the container still contributes the cells that
    // stick out of it. The walk stops at a descendant that clips its
    // own content (it owns whatever overflows it) and skips out-of-flow
    // boxes: `display:none` takes no space and positioned boxes are
    // placed in phase 2 against their own containing block.
    for child in element_children_of(dom, id) {
        extend_scrollable_overflow(dom, child, &mut extend);
    }

    // Text content: a pure-text leaf or IFC block packs its lines from
    // the top of `inner` (stored unscrolled), so its extent is the line
    // count by the widest line. Anonymous block boxes (mixed content)
    // carry scrolled rects like element children do. Without these a
    // `<textarea>` with six lines reported zero content and could never
    // scroll (HARDENING-2026-09 R5).
    if let Some(ext) = dom.node(id).ext() {
        if let Some(il) = ext.inline_layout.as_ref() {
            let widest = il.lines.iter().map(|l| l.width as i32).max().unwrap_or(0);
            // An editing host whose text ends in a newline has one more
            // row than the packer emits: the empty line the caret sits
            // on after that newline (a browser `<textarea>` shows it; a
            // `<pre>` does not). The caret code models the same row
            // (`caret::phantom_line_and_column`), so the extent must
            // include it or the caret can never be scrolled into view.
            let trailing_caret_line = i32::from(trailing_newline_caret_row(dom, id));
            let top = inner.y;
            let left = inner.x;
            min_x = Some(min_x.map_or(left, |m: i32| m.min(left)));
            min_y = Some(min_y.map_or(top, |m: i32| m.min(top)));
            max_right = max_right.max(left + widest);
            max_bottom = max_bottom.max(top + il.height() as i32 + trailing_caret_line);
            any = true;
        }
        for anon in &ext.anonymous_blocks {
            let top = anon.rect.y + scroll_y;
            let left = anon.rect.x + scroll_x;
            min_x = Some(min_x.map_or(left, |m: i32| m.min(left)));
            min_y = Some(min_y.map_or(top, |m: i32| m.min(top)));
            max_right = max_right.max(left + anon.rect.width as i32);
            max_bottom = max_bottom.max(top + anon.rect.height as i32);
            any = true;
        }
    }

    let (content_w, content_h) = if any {
        (
            (max_right - min_x.unwrap_or(inner.x)).max(0),
            (max_bottom - min_y.unwrap_or(inner.y)).max(0),
        )
    } else {
        (0, 0)
    };

    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.scroll_content_width = content_w as usize;
        ext.scroll_content_height = content_h as usize;
    }
}

/// True when `id` is an editing host (`<textarea>`, text `<input>`,
/// `contenteditable`) whose last text child ends with `\n`: the caret
/// can then stand on an empty row after that newline, which the line
/// packer does not emit as a line box.
fn trailing_newline_caret_row(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    use crate::node::TuiNodeExt;
    if !dom.node(id).is_editable() {
        return false;
    }
    let last_text = dom
        .node(id)
        .child_nodes()
        .filter(|c| c.node_type() == rdom_core::NodeType::Text)
        .last();
    last_text.is_some_and(|t| t.node_value().is_some_and(|v| v.ends_with('\n')))
}

/// Whether `id`'s scrolling area origin is at its right / bottom edge
/// (CSSOM View §4): the edge content starts from on each axis. A
/// horizontal box starts at its inline-start edge — the right one under
/// `direction: rtl` (CSS Writing Modes 4 §2.1); a flex container at the
/// main-start edge of its main axis (CSS Flexbox §5.1: the right one of
/// an `ltr` `row-reverse`, the bottom one of a `column-reverse`) and the
/// cross-start edge of its cross axis.
fn origin_at_end(dom: &Dom<TuiExt>, id: NodeId) -> (bool, bool) {
    let Some(c) = dom.node(id).ext().and_then(|e| e.computed.as_ref()) else {
        return (false, false);
    };
    if c.flow == crate::layout::Flow::Flex {
        let flip = super::flex::AxisFlip::of(c, c.direction);
        return match c.direction {
            crate::layout::Direction::Row => (flip.main, flip.cross),
            crate::layout::Direction::Column => (flip.cross, flip.main),
        };
    }
    (super::margin_trim::inline_reversed(c), false)
}

/// The legal offsets on one axis: `0 ..= overflow` from an origin at the
/// left / top edge, `-overflow ..= 0` from one at the right / bottom.
fn bounds(overflow: usize, origin_at_end: bool) -> (i32, i32) {
    let overflow = i32::try_from(overflow).unwrap_or(i32::MAX);
    if origin_at_end {
        (-overflow, 0)
    } else {
        (0, overflow)
    }
}

/// The legal `scrollLeft` values of `id` for a scrollport `viewport`
/// cells wide (CSSOM View §4): `0 ..= overflow`, or `-overflow ..= 0`
/// when the scrolling area origin is the right edge (an `rtl` box, a
/// `row-reverse` flex container under `ltr`; [`origin_at_end`]),
/// `overflow` being the scroll width past the scrollport.
pub(crate) fn scroll_x_bounds(dom: &Dom<TuiExt>, id: NodeId, viewport: usize) -> (i32, i32) {
    let Some(ext) = dom.node(id).ext() else {
        return (0, 0);
    };
    let overflow = ext.scroll_content_width.saturating_sub(viewport);
    bounds(overflow, origin_at_end(dom, id).0)
}

/// The legal `scrollTop` values of `id` for a scrollport `viewport`
/// rows tall, as [`scroll_x_bounds`]: `-overflow ..= 0` for a
/// `column-reverse` flex container, whose origin is its bottom edge.
pub(crate) fn scroll_y_bounds(dom: &Dom<TuiExt>, id: NodeId, viewport: usize) -> (i32, i32) {
    let Some(ext) = dom.node(id).ext() else {
        return (0, 0);
    };
    let overflow = ext.scroll_content_height.saturating_sub(viewport);
    bounds(overflow, origin_at_end(dom, id).1)
}

/// How far `id`'s scrollport sits from the left edge of its scrollable
/// overflow area: `scrollLeft` less its minimum ([`scroll_x_bounds`]
/// for a `viewport`-cell scrollport). Physical and never negative — the
/// offset a horizontal scrollbar thumb is drawn at.
pub(crate) fn scroll_x_from_area_start(dom: &Dom<TuiExt>, id: NodeId, viewport: usize) -> usize {
    let (min_x, _) = scroll_x_bounds(dom, id, viewport);
    let scroll_x = dom.node(id).ext().map_or(0, |e| e.scroll_x);
    usize::try_from(scroll_x.saturating_sub(min_x)).unwrap_or(0)
}

/// [`scroll_x_from_area_start`] for the vertical axis: how far the
/// scrollport sits below the top edge of the scrollable overflow area.
pub(crate) fn scroll_y_from_area_start(dom: &Dom<TuiExt>, id: NodeId, viewport: usize) -> usize {
    let (min_y, _) = scroll_y_bounds(dom, id, viewport);
    let scroll_y = dom.node(id).ext().map_or(0, |e| e.scroll_y);
    usize::try_from(scroll_y.saturating_sub(min_y)).unwrap_or(0)
}

/// Clamp `id`'s scroll offset to its legal range on each axis
/// (`[0, scroll size − viewport size]`, or the mirrored range of an
/// `rtl` box's `scrollLeft`, [`scroll_x_bounds`]; CSS keeps
/// `scrollTop`/`scrollLeft` in range as content changes). Only scroll containers can hold a non-zero
/// offset, so non-scrollable elements are a no-op. The viewport is
/// the element's final `content_layout` — the region children are
/// laid out and clipped into (after the two-pass scrollbar gutter
/// reflow), so this max matches what the runtime's wheel / scrollbar
/// / scroll-into-view path can actually reach. Returns whether an
/// offset changed (the caller then re-lays-out the children at the
/// corrected position).
pub(crate) fn clamp_scroll_offset(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
) -> bool {
    let scrolls = !matches!(computed.overflow_x, Overflow::Visible)
        || !matches!(computed.overflow_y, Overflow::Visible);
    if !scrolls {
        return false;
    }
    let Some(ext) = dom.node(id).ext() else {
        return false;
    };
    let vp = ext.content_layout;
    let (min_x, max_x) = scroll_x_bounds(dom, id, vp.width as usize);
    let (min_y, max_y) = scroll_y_bounds(dom, id, vp.height as usize);
    let new_x = ext.scroll_x.clamp(min_x, max_x);
    let new_y = ext.scroll_y.clamp(min_y, max_y);
    if new_x == ext.scroll_x && new_y == ext.scroll_y {
        return false;
    }
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.scroll_x = new_x;
        ext.scroll_y = new_y;
    }
    true
}

/// Feed `extend` the boxes `id`'s subtree contributes to an ancestor's
/// scrollable overflow: its own layout rect and — unless it clips — its
/// anonymous boxes and in-flow descendants' boxes. A clipping box (an
/// intermediate scroll container) contributes its border box alone:
/// everything inside it, anonymous boxes included, is its own
/// scrollable overflow (CSS Overflow 3 §2.2).
fn extend_scrollable_overflow(dom: &Dom<TuiExt>, id: NodeId, extend: &mut impl FnMut(LayoutRect)) {
    if !is_in_flow(dom, id) {
        return;
    }
    let Some(ext) = dom.node(id).ext() else {
        return;
    };
    extend(ext.layout);
    let clips = ext.computed.as_ref().is_some_and(|c| {
        !matches!(c.overflow_x, Overflow::Visible) || !matches!(c.overflow_y, Overflow::Visible)
    });
    if clips {
        return;
    }
    for anon in &ext.anonymous_blocks {
        extend(anon.rect);
    }
    for child in dom.node(id).child_nodes() {
        match child.node_type() {
            NodeType::Element => extend_scrollable_overflow(dom, child.id(), extend),
            // A fragment has no box; its element children count as ours.
            NodeType::Fragment => {
                for grand in child.child_nodes() {
                    if grand.node_type() == NodeType::Element {
                        extend_scrollable_overflow(dom, grand.id(), extend);
                    }
                }
            }
            _ => {}
        }
    }
}
