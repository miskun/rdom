//! One element's paint: its own box — outer shadows, background, inset
//! shadows, border (`paint_box`) — and its content — a canvas callback,
//! an inline formatting context, or `::before` / own text / `::after`,
//! its in-flow children and anonymous blocks, then its scrollbars
//! (`paint_content`). The order between boxes, and which phase paints a
//! box's own box and which its content, is `stacking_walk`'s.

use rdom_core::{Dom, NodeId, NodeType};

use super::background::paint_background;
use super::border::paint_border_sides;
use super::inline_paint::{
    paint_anonymous_blocks, paint_caret_if_editable, paint_ifc, paint_inline_content,
};
use super::layout_rect_to_grid;
use super::scrollbar;
use super::shadow;
use super::stacking_walk::recurse_children;
use crate::ext::TuiExt;
use crate::layout::{Display, LayoutRect};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::is_ifc_block;
use crate::render::stacking::children_clip;
use crate::render::{Buffer, Rect};
use crate::style::{Color, ComputedStyle};

/// What [`box_frame`] hands to [`paint_content`]: the style with the
/// transition presentation overlaid, the border and content rects and the
/// clip the content paints into.
pub(super) struct BoxFrame {
    computed: ComputedStyle,
    outer: LayoutRect,
    inner: LayoutRect,
    pub(super) children_clip: Rect,
    /// The box is drawn (`visibility: visible`, CSS Display 3 §4). A
    /// hidden box paints nothing of its own — no shadow, background,
    /// border, text, canvas or scrollbar — while its descendants paint
    /// by their own `visibility`.
    visible: bool,
}

/// An element's box as paint reads it — its style with an in-flight
/// transition's presentation overlaid, its rects, whether it is drawn and
/// the clip its content paints into (CSS Overflow 3 §3,
/// `stacking::children_clip`). `None` for non-elements and `display:
/// none`, which paint nothing and have no content to paint.
pub(super) fn box_frame(dom: &Dom<TuiExt>, id: NodeId, clip: Rect) -> Option<BoxFrame> {
    if dom.node(id).node_type() != NodeType::Element {
        return None;
    }

    let mut computed = dom
        .node(id)
        .computed()
        .cloned()
        .unwrap_or_else(ComputedStyle::initial);

    // M3: an in-flight transition writes interpolated values
    // into `TuiExt.presentation` each tick. Paint reads them by
    // overlaying onto the local `computed` clone — keeps the
    // existing paint logic unchanged otherwise.
    if let Some(presentation) = dom.node(id).ext().and_then(|e| e.presentation.as_deref()) {
        if let Some(fg) = presentation.fg {
            computed.fg = fg;
        }
        if let Some(bg) = presentation.bg {
            computed.bg = bg;
        }
        if let Some(border_color) = presentation.border_color {
            computed.border_color = border_color;
        }
        if let Some(padding) = &presentation.padding {
            computed.padding = padding.clone();
        }
        if let Some(gap) = presentation.row_gap {
            computed.row_gap = crate::layout::GapValue::Cells(gap);
        }
        if let Some(gap) = presentation.column_gap {
            computed.column_gap = crate::layout::GapValue::Cells(gap);
        }
    }

    // `display: none` — element takes no space and neither it
    // nor its children paint. Matches CSS semantics.
    if computed.display == Display::None {
        return None;
    }

    let outer = dom.node(id).layout_rect().unwrap_or_default();
    let inner = dom.node(id).content_layout_rect().unwrap_or(outer);
    let visible = crate::render::visibility::shows(dom, id, crate::ext::StyleSlot::Host);
    // Inner paint (text + pseudo-elements + children) happens in
    // `content_layout`, clipped by the element's overflow mode at the
    // padding-box edge per CSS Overflow 3 §3 (`stacking::children_clip`).
    let children_clip = children_clip(dom, id, &computed, clip);
    Some(BoxFrame {
        computed,
        outer,
        inner,
        children_clip,
        visible,
    })
}

/// Paint an element's own box — outer shadows, background fill, inset
/// shadows and border (CSS Backgrounds 3 §7.2: the shadows with the
/// background) — and hand back its [`BoxFrame`]. In-flow block-level
/// boxes do this in their paint unit's background phase
/// (`stacking_walk`), before any of its inline content.
pub(super) fn paint_box(
    dom: &Dom<TuiExt>,
    id: NodeId,
    buf: &mut Buffer,
    clip: Rect,
) -> Option<BoxFrame> {
    let frame = box_frame(dom, id, clip)?;
    if !frame.visible {
        return Some(frame);
    }
    let (computed, outer, inner) = (&frame.computed, frame.outer, frame.inner);
    // 0. Outer shadows, under the background (CSS Backgrounds 3 §6.1);
    // they may show while the box itself is outside the clip.
    shadow::paint_outer_shadows(buf, computed, outer, clip);

    // Fast path: element entirely outside the clip.
    let Some(outer_grid) = layout_rect_to_grid(outer, clip) else {
        // Its box is off-screen; its content may still show (a scrolled-
        // off parent's visible children, `overflow: visible`).
        return Some(frame);
    };
    // 1. Background fill over the `background-clip` box: an opaque
    // fill that clears glyphs from earlier paints (full CSS
    // occlusion). `opacity` is applied when the stacking context's
    // layer composites back, not here. See `background.rs`.
    // Tree rows defer their background to the guide pass
    // (`tree_guides`), which fills the FULL row — including the
    // guide gutter to the left of the indented box — so the
    // `aria-selected` / cursor highlight spans edge to edge. A
    // normal box fill here would both stop at the indented box's
    // left edge AND tint the whole open subtree (the box
    // contains the nested group).
    let is_tree_row = dom.node(id).get_attribute("role") == Some("treeitem");
    if !is_tree_row {
        paint_background(buf, computed, outer, inner, clip);
    }
    // Inset shadows, above the background and below the border.
    shadow::paint_inset_shadows(buf, computed, outer, clip);

    // 2. Border. Writes per-cell × per-direction `BorderContribution`s
    // into `buf.border_dirs`; the joiner reads them after the
    // walk and emits the right glyph + color. BORDER-MODEL-1
    // priority encodes "child wins over ancestor" (depth) and
    // "earlier DOM order wins on tie" (`NodeId` proxy for
    // geometric position).
    if !computed.border.is_empty() {
        let priority = compute_border_priority(dom, id);
        paint_border_sides(buf, computed, outer, outer_grid, clip, priority);
    }
    Some(frame)
}

/// Paint an element's content: its canvas callback, or its inline
/// formatting context, or `::before` / own text / `::after` plus its
/// in-flow children and anonymous blocks; then its scrollbars. The
/// caller exits `frame.saved_ctx` afterwards.
pub(super) fn paint_content(
    dom: &Dom<TuiExt>,
    id: NodeId,
    buf: &mut Buffer,
    clip: Rect,
    viewport: Rect,
    frame: &BoxFrame,
) {
    let computed = &frame.computed;
    let inner = frame.inner;
    let children_clip = frame.children_clip;

    // C.9 `<canvas>` escape hatch: when an element has a
    // registered canvas paint callback, invoke it with a bounded
    // `RenderContext` instead of running the normal inline paint
    // path + child recursion. No callback → fall through to the
    // normal paint (HTML fallback-content behavior).
    if let Some(paint) = dom.node(id).ext().and_then(|e| e.canvas_paint.clone()) {
        if !frame.visible {
            return;
        }
        let mut ctx = crate::runtime::builtins::canvas::RenderContext::new(
            buf,
            inner.x,
            inner.y,
            inner.width,
            inner.height,
            children_clip,
        );
        paint.call(dom, &mut ctx);
        scrollbar::paint_scrollbars(dom, id, computed, buf, clip);
        return;
    }

    // IFC block: paint the inline formatting context (interleaved
    // text + inline element children, each with its own cascaded
    // style) and skip both the default own-text paint and child
    // recursion.
    if is_ifc_block(dom, id) {
        // Text rows are addressed through the *scrolled* content rect so
        // a scroll container's first `scroll_y` lines sit above the port.
        let text_inner = crate::render::inline::scrolled_content_rect(dom, id).unwrap_or(inner);
        paint_ifc(dom, id, text_inner, buf, children_clip, viewport);
        // Caret overlay — paint at the end so it sits on top of
        // every fragment in the inline flow. IFC blocks always have
        // an `inline_layout`, so this fires unconditionally.
        paint_caret_if_editable(dom, buf, id, children_clip);
        // Scrollbar overlay — on top of any content that might
        // have leaked into the gutter (it can't, but paint order
        // still puts scrollbars last so any future strategy
        // change stays correct). `clip` (the incoming clip, NOT
        // children_clip) so the scrollbar can sit in the gutter
        // which is outside children_clip when overflow clips.
        if frame.visible {
            scrollbar::paint_scrollbars(dom, id, computed, buf, clip);
        }
        return;
    }

    // Compute ::before / own text / ::after paint positions.
    let text_inner = crate::render::inline::scrolled_content_rect(dom, id).unwrap_or(inner);
    paint_inline_content(
        dom,
        id,
        computed,
        text_inner,
        buf,
        (children_clip, viewport),
    );

    // Caret overlay for pure-text leaf blocks (e.g. <input>,
    // <textarea>) — they go through `paint_inline_content` rather
    // than `paint_ifc`, so the caret has to be painted at this call
    // site too. `paint_caret_if_editable` is a no-op for elements
    // that aren't the inline-flow container of the focused caret.
    if crate::render::inline::has_inline_layout(dom, id) {
        paint_caret_if_editable(dom, buf, id, children_clip);
    }

    // Recurse into in-flow element children (they paint at their own
    // layouts).
    recurse_children(dom, id, buf, children_clip, viewport);

    // Paint each anonymous block box synthesized by the block
    // layout pass (BFC-1 phase 3). Anonymous boxes wrap runs of
    // inline-level children inside a block container that also
    // has block children; each carries its own `InlineLayout` +
    // rect on `TuiExt.anonymous_blocks`. No-op when the Vec is
    // empty (pure-flex, pure-IFC, or pure-block containers).
    // A generated flex item's own box, under its lines.
    super::generated_box::paint_generated_boxes(dom, id, buf, children_clip);
    paint_anonymous_blocks(dom, id, buf, children_clip, viewport);

    // Scrollbar overlay (after children so it sits on top if
    // anything encroached).
    if frame.visible {
        scrollbar::paint_scrollbars(dom, id, computed, buf, clip);
    }
}

/// Compute the structural priority for an element's border
/// contributions. Encodes CSS Tables 3 §11.5 rules 5 + 6:
///
/// - **Rule 5 (closer-to-cell wins).** Deeper elements outrank
///   their ancestors. We walk the parent chain to count depth.
/// - **Rule 6 (geometric position).** Within the same depth,
///   earlier-in-DOM wins. We use `NodeId.as_u32()` as a stable
///   proxy: nodes are created in monotonic order, and for the
///   common case of "build the tree top-down, append children in
///   order" that aligns with leftmost / topmost in geometric
///   layout. Edge cases (re-parented elements, mixed creation
///   order) are accepted simplifications — documented in
///   `DIVERGENCES.md` under "Layout."
fn compute_border_priority(dom: &Dom<TuiExt>, id: NodeId) -> u64 {
    use crate::render::buffer::BorderContribution;
    let mut depth: u16 = 0;
    let mut cur = dom.node(id).parent_node();
    while let Some(node) = cur {
        depth = depth.saturating_add(1);
        cur = node.parent_node();
    }
    BorderContribution::pack_priority(depth, id.as_u32())
}

/// True when `bg` fills a box: not the terminal default (`Reset`,
/// which leaves the cells to what is beneath) and not fully
/// transparent (CSS Color 4 §6.3).
pub(crate) fn fills(bg: Color) -> bool {
    bg != Color::Reset && bg.alpha() > 0
}
