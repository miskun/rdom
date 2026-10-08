//! The scrollable overflow area of a scroll container (CSS Overflow 3
//! §2.2), recorded as `scroll_content_{width,height}` and measured from
//! the scrolling area origin (`scrollport` has the model), the clamp of
//! its scroll offsets, and the trailing caret row of an editing host.

use rdom_core::{Dom, NodeId, NodeType};

use super::ClipEdges;
use super::{element_children_of, is_in_flow};
use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::render::inline::InlineLayout;
use crate::style::ComputedStyle;

/// Record `id`'s scrollable overflow area (CSS Overflow 3 §2.2) in its
/// `TuiExt.scroll_content_{width,height}`: the scrollport ∪ the in-flow
/// content's boxes and line boxes extended by the end padding ∪ the
/// absolutely positioned boxes it contains, on each axis from the
/// scrolling area origin's edge of the scrollport (CSSOM View §4) — what
/// lies beyond that edge cannot be scrolled to. `inner` is the content
/// box the children were just laid out in; rects are taken with the
/// box's scroll offset added back, so the area does not depend on it.
pub(crate) fn record_scroll_content_size(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    inner: LayoutRect,
    computed: &ComputedStyle,
) {
    // Only a scroll container has a scrollable extent (CSS Overflow 3
    // §3.1; an `overflow: clip` box is none). Any other box records none,
    // so a box that stopped scrolling keeps no stale extent to scroll by.
    if !computed.is_scroll_container() {
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.scroll_content_width = 0;
            ext.scroll_content_height = 0;
        }
        return;
    }
    let Some(ext) = dom.node(id).ext() else {
        return;
    };
    let (scroll_x, scroll_y) = (ext.scroll_x, ext.scroll_y);
    let port = super::scrollport::scrollport_of(ext, computed);

    // The in-flow content: its own line boxes, the descendants' boxes
    // (the walk stops at a descendant that clips its own content, and
    // skips out-of-flow boxes but floats — `display: none` takes no
    // space, positioned boxes are merged below) and its anonymous boxes.
    let mut content = Extent::default();
    // A pure-text leaf or IFC block packs its lines in `inner` (stored
    // unscrolled): each line box, from its leftmost to its rightmost
    // fragment — left of `inner` for an overflowing `rtl` line. Without
    // these a `<textarea>` with six lines reported zero content and could
    // never scroll (HARDENING-2026-09 R5).
    if let Some(il) = dom.node(id).ext().and_then(|e| e.inline_layout.as_ref()) {
        for r in line_rects(il, inner) {
            content.add(r);
        }
        // An editing host whose text ends in a newline has one more row
        // than the packer emits: the empty line the caret sits on after
        // that newline (a browser `<textarea>` shows it; a `<pre>` does
        // not). The caret code models the same row
        // (`caret::phantom_line_and_column`), so the extent must include
        // it or the caret can never be scrolled into view.
        if trailing_newline_caret_row(dom, id) {
            let rows = i32::from(il.height());
            content.add(LayoutRect::new(inner.x, inner.y + rows, 0, 1));
        }
    }
    let mut extend = |rect: LayoutRect| {
        content.add(LayoutRect {
            x: rect.x + scroll_x,
            y: rect.y + scroll_y,
            ..rect
        })
    };
    for child in element_children_of(dom, id) {
        // A table's captions are outside its table box (CSS 2.1 §17.4),
        // so outside the area it scrolls.
        if crate::render::stacking::outside_content_clip(dom, id, child) {
            continue;
        }
        extend_scrollable_overflow(dom, child, ClipEdges::NONE, &mut extend);
    }
    // Its anonymous block boxes and their line boxes (§2.2), with
    // scrolled rects like element children's.
    if let Some(ext) = dom.node(id).ext() {
        // Its floated pseudo-elements' boxes too, as its floated children's.
        for anon in ext.anonymous_blocks.iter().chain(ext.floated_pseudos()) {
            for r in
                std::iter::once(anon.border_box()).chain(line_rects(&anon.inline_layout, anon.rect))
            {
                extend(r);
            }
        }
    }

    // §2.2's end padding: the content box's distance from the scrollport
    // on the side away from the origin.
    let at_end = origin_at_end(dom, id);
    let (port_x, port_y) = (span(port.x, port.width), span(port.y, port.height));
    let (in_x, in_y) = (span(inner.x, inner.width), span(inner.y, inner.height));
    let mut x = AreaAxis::new(port_x, at_end.0);
    let mut y = AreaAxis::new(port_y, at_end.1);
    if let Some((cx, cy)) = content.spans() {
        x.reach(cx, end_padding(port_x, in_x, at_end.0));
        y.reach(cy, end_padding(port_y, in_y, at_end.1));
    }
    // The absolutely positioned boxes it contains (§2.2), as the last
    // settle of this document found them (`positioned_overflow`, which
    // runs the layout again when they moved), from its border box.
    if let Some(r) = super::positioned_overflow::reach_of(dom, id) {
        let origin = dom.node(id).ext().map_or(inner, TuiExt::border_box);
        x.reach((origin.x + r.left, origin.x + r.right), 0);
        y.reach((origin.y + r.top, origin.y + r.bottom), 0);
    }

    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.scroll_content_width = x.size();
        ext.scroll_content_height = y.size();
    }
}

/// `[start, end)` of a run of `len` cells at `start`.
fn span(start: i32, len: u16) -> (i32, i32) {
    (start, start + i32::from(len))
}

/// The padding between the content box `inner` and the scrollport `port`
/// on the side away from the origin: the end side, or the start side
/// where the origin is at the end. Never negative (a collapsed border
/// can put the content box past the padding box).
fn end_padding(port: (i32, i32), inner: (i32, i32), origin_at_end: bool) -> i32 {
    if origin_at_end {
        (inner.0 - port.0).max(0)
    } else {
        (port.1 - inner.1).max(0)
    }
}

/// The bounding box of the rects added, unscrolled.
#[derive(Default)]
struct Extent(Option<(i32, i32, i32, i32)>);

impl Extent {
    fn add(&mut self, r: LayoutRect) {
        let (x0, y0) = (r.x, r.y);
        let (x1, y1) = (r.x + i32::from(r.width), r.y + i32::from(r.height));
        self.0 = Some(match self.0 {
            None => (x0, y0, x1, y1),
            Some((a, b, c, d)) => (a.min(x0), b.min(y0), c.max(x1), d.max(y1)),
        });
    }

    /// `((x0, x1), (y0, y1))`, `None` when nothing was added.
    fn spans(&self) -> Option<((i32, i32), (i32, i32))> {
        self.0.map(|(x0, y0, x1, y1)| ((x0, x1), (y0, y1)))
    }
}

/// One axis of the scrollable overflow area: the scrollport's span,
/// grown by content only away from the origin's edge.
struct AreaAxis {
    start: i32,
    end: i32,
    origin_at_end: bool,
}

impl AreaAxis {
    fn new(port: (i32, i32), origin_at_end: bool) -> Self {
        Self {
            start: port.0,
            end: port.1,
            origin_at_end,
        }
    }

    /// Take in content spanning `[s, e)`, extended by `padding` on the
    /// side away from the origin.
    fn reach(&mut self, (s, e): (i32, i32), padding: i32) {
        if self.origin_at_end {
            self.start = self.start.min(s - padding);
        } else {
            self.end = self.end.max(e + padding);
        }
    }

    fn size(&self) -> usize {
        usize::try_from(self.end - self.start).unwrap_or(0)
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
pub(crate) fn origin_at_end(dom: &Dom<TuiExt>, id: NodeId) -> (bool, bool) {
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

/// Clamp `id`'s scroll offset to its legal range on each axis
/// ([`super::scrollport::scroll_bounds`]; CSS keeps `scrollTop` /
/// `scrollLeft` in range as content changes) — the same range the
/// runtime's wheel, keys, scrollbar and scroll API clamp to. Only scroll
/// containers can hold a non-zero offset, so a box that is not one has
/// its offset dropped. Returns whether an offset changed (the caller then
/// lays the children out again at the corrected position).
pub(crate) fn clamp_scroll_offset(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
) -> bool {
    let Some(ext) = dom.node(id).ext() else {
        return false;
    };
    let (x, y) = (ext.scroll_x, ext.scroll_y);
    // A box that is not a scroll container has no scroll offset (CSS
    // Overflow 3 §3.1: `clip` "forbids all scrolling"): one left from
    // when it was, or written by hand, is dropped.
    let (new_x, new_y) = if computed.is_scroll_container() {
        match super::scrollport::scroll_bounds(dom, id) {
            Some(b) => b.clamp(x, y),
            None => return false,
        }
    } else {
        (0, 0)
    };
    if (new_x, new_y) == (x, y) {
        return false;
    }
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.scroll_x = new_x;
        ext.scroll_y = new_y;
    }
    true
}

/// Feed `extend` the boxes `id`'s subtree contributes to an ancestor's
/// scrollable overflow, each cut to `clip` — the overflow clip edges of
/// the `overflow: clip` boxes between: its own layout rect and — unless
/// it is a scroll container — its anonymous boxes and in-flow
/// descendants' boxes. An intermediate scroll container contributes its
/// border box alone: everything inside it is its own scrollable overflow
/// (CSS Overflow 3 §2.2). A `clip` box's content counts up to its
/// overflow clip edge on its `clip` axes, wholly on a `visible` one.
fn extend_scrollable_overflow(
    dom: &Dom<TuiExt>,
    id: NodeId,
    clip: ClipEdges,
    extend: &mut impl FnMut(LayoutRect),
) {
    // A float is out of flow but in the flow's overflow: its border box
    // counts like any descendant's (§2.2).
    if !is_in_flow(dom, id) && super::float::float_side(dom, id).is_none() {
        return;
    }
    extend_box_overflow(dom, id, clip, extend);
}

/// [`extend_scrollable_overflow`] for `id` whether or not it is in flow:
/// an absolutely positioned box counts in the scroll container it is
/// contained in (`positioned_overflow`), with its own scrollable
/// overflow.
pub(super) fn extend_box_overflow(
    dom: &Dom<TuiExt>,
    id: NodeId,
    clip: ClipEdges,
    extend: &mut impl FnMut(LayoutRect),
) {
    let Some(ext) = dom.node(id).ext() else {
        return;
    };
    // A box-less element (CSS Display 3 §2.5) has no box to count and
    // none to clip with (`overflow` applies to containers, CSS Overflow
    // 3 §3.1): its children count as its parent's.
    let box_less = crate::render::box_tree::is_contents(dom, id);
    let mut inner = clip;
    if !box_less {
        if let Some(r) = clip.cut(ext.layout) {
            extend(r);
        }
        // A `<select>` picker's option list is in the top layer, part of no
        // scroller's overflow (CSS Position 4); the select's own row is.
        if dom.top_layer_kind(id) == Some(rdom_core::TopLayerKind::Picker) {
            return;
        }
        if let Some(c) = ext.computed.as_ref() {
            if c.is_scroll_container() {
                return;
            }
            inner = clip.narrow(ClipEdges::of_element(dom, id, ext, c));
        }
    }
    // Its line boxes, which may reach past its box (a `nowrap` line, lines
    // below a fixed height): the scrollable overflow covers them (§2.2).
    // A box-less element lays out no lines of its own.
    if !box_less && let Some(il) = ext.inline_layout.as_ref() {
        for r in line_rects(il, ext.content_layout) {
            if let Some(r) = inner.cut(r) {
                extend(r);
            }
        }
    }
    for anon in ext.anonymous_blocks.iter().chain(ext.floated_pseudos()) {
        for r in
            std::iter::once(anon.border_box()).chain(line_rects(&anon.inline_layout, anon.rect))
        {
            if let Some(r) = inner.cut(r) {
                extend(r);
            }
        }
    }
    for child in crate::render::box_tree::children(dom, id) {
        let child = dom.node(child);
        match child.node_type() {
            NodeType::Element => extend_scrollable_overflow(dom, child.id(), inner, extend),
            // A fragment has no box; its element children count as ours.
            NodeType::Fragment => {
                for grand in child.child_nodes() {
                    if grand.node_type() == NodeType::Element {
                        extend_scrollable_overflow(dom, grand.id(), inner, extend);
                    }
                }
            }
            _ => {}
        }
    }
}

/// The rects of `il`'s line boxes laid out at `origin` (their content
/// box): each line's rows, from its leftmost to its rightmost fragment or
/// generated run — its packed width when it holds neither.
fn line_rects(il: &InlineLayout, origin: LayoutRect) -> impl Iterator<Item = LayoutRect> + '_ {
    il.lines.iter().map(move |line| {
        let spans = line
            .fragments
            .iter()
            .map(|f| (f.x, f.x + i32::from(f.width)))
            .chain(
                line.generated
                    .iter()
                    .map(|g| (g.x, g.x + i32::from(g.width))),
            );
        let (start, end) = spans
            .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)))
            .unwrap_or((0, i32::from(line.width)));
        LayoutRect::new(
            origin.x + start,
            origin.y + i32::from(line.top),
            u16::try_from(end - start).unwrap_or(0),
            line.height,
        )
    })
}
