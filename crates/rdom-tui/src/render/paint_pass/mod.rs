//! The paint pass.
//!
//! Walks the cascaded + laid-out `Dom<TuiExt>` and writes cells into
//! a `Buffer`. Runs *after* `cascade` + `layout_dom`. The cascade
//! produced `ComputedStyle` per element; layout produced
//! `LayoutRect`. Paint consumes both and emits the final grid of
//! cells.
//!
//! ## Paint order (per element)
//!
//! 1. **Background fill** — `computed.bg` over the element's
//!    **outer** layout rect (CSS way; covers border cells). Skipped
//!    for `Color::Reset`.
//! 2. **Border** — border chars at the outer rect edges using
//!    `computed.border_fg`. Styles (Single, Rounded, Top, Bottom,
//!    Left, Right) pick different character sets.
//! 3. **Inline content** — either the classic `::before` then own
//!    text then `::after` path (non-IFC elements) or the IFC fragment
//!    path (blocks establishing an inline formatting context).
//! 4. **Recurse** — in-flow element children paint at their own
//!    `layout` rects.
//!
//! ## Stacking
//!
//! Positioned children do not paint in the recursion: they belong to
//! the layers of the nearest stacking context (CSS 2.1 Appendix E),
//! which [`paint_stacking_context`] paints around the context root's
//! in-flow content — see [`crate::render::stacking`].
//!
//! ## Clipping
//!
//! - Entry takes a `clip: Rect` — the terminal-grid region the
//!   caller wants to paint into. Nothing outside `clip` is ever
//!   written.
//! - Each element's paint is intersected with `clip` via
//!   [`layout_rect_to_grid`].
//! - For `overflow: Hidden | Scroll | Auto`, children are recursed
//!   with a tighter clip = padding box ∩ clip.
//! - `overflow: Visible` keeps the incoming clip — children can
//!   draw past the parent.
//! - A positioned descendant is clipped by an overflow ancestor only
//!   when its containing block is that ancestor or inside it (CSS 2.1
//!   §11.1.1); `stacking::collect_layers` resolves that clip.
//!
//! ## Module layout
//!
//! - `mod.rs` — public `PaintExt` trait, the stacking-context walk
//!   (`paint_stacking_context` / `paint_box` / `paint_content` /
//!   `recurse_children`) and the shared `layout_rect_to_grid` clip
//!   utility.
//! - [`border`] — background fill + border drawing (box-drawing
//!   chars, edge selection).
//! - [`inline_paint`] — `::before` + own text + `::after` for
//!   non-IFC elements; fragment-driven IFC paint.
//! - [`text`] — `paint_text` low-level helper + `ComputedStyle` →
//!   `Style` conversion.

mod border;
mod border_join;
mod inline_paint;
mod positioned_pseudos;
pub(crate) mod scrollbar;
mod text;
mod tree_guides;

#[cfg(test)]
mod tests;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Display, LayoutRect};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::is_ifc_block;
use crate::render::stacking::{
    LayerEntry, children_clip, collect_layers, creates_stacking_context, is_positioned,
};
use crate::render::{Buffer, Rect};
use crate::style::{Color, ComputedStyle};

use border::{fill_bg, paint_border};
use inline_paint::{
    paint_anonymous_blocks, paint_caret_if_editable, paint_ifc, paint_inline_content,
};

/// Extension trait on `Dom<TuiExt>` adding `paint_dom(buf, clip)`.
pub trait PaintExt {
    /// Paint the entire cascaded + laid-out DOM into `buf`, clipped
    /// to `clip`. Assumes `cascade()` and `layout_dom()` have
    /// already run; paint reads from `ComputedStyle` and
    /// `LayoutRect` only.
    fn paint_dom(&self, buf: &mut Buffer, clip: Rect);
}

impl PaintExt for Dom<TuiExt> {
    fn paint_dom(&self, buf: &mut Buffer, clip: Rect) {
        // The document is the root stacking context (CSS 2.1 Appendix
        // E): positioned descendants paint from its layers, nested
        // contexts recursively.
        paint_stacking_context(self, self.root(), buf, clip, clip);
        // Positioned `::before` / `::after` pseudo-elements paint after
        // every stacking context, in one flat pass ordered by the
        // host's `z-index` (DIVERGENCES): a pseudo has no `NodeId` and
        // no layer slot of its own, and it does not hit-test.
        positioned_pseudos::paint_positioned_pseudos(self, buf, clip);
        // Overlay backdrop behind modal dialogs. Runs AFTER the main
        // paint pass so the backdrop reliably sits on top of whatever
        // else painted into the viewport — then we re-paint the
        // dialog subtree so it ends up above the backdrop.
        paint_modal_backdrops(self, buf, clip);
        // Tree guide lines — emit `│ ├ └` border contributions into
        // the gutter of every `[role=tree]`. Runs BEFORE the joiner
        // so the accumulated direction masks become glyphs.
        tree_guides::paint_tree_guides(self, buf, clip);
        // Border-collapse joiner. Walks the buffer once and rewrites
        // box-drawing glyphs at junctions based on 4-neighbor
        // connectivity. Cheap when no element has `border-collapse:
        // collapse` (short-circuits via the bottom-up tree flag).
        border_join::join_borders(self, buf);
    }
}

/// Paint `root` and everything stacked inside it in CSS 2.1 Appendix E
/// order: the root's own box, child contexts with negative `z-index`,
/// the root's in-flow content, positioned descendants with `z-index:
/// auto | 0` in tree order, child contexts with positive `z-index`.
///
/// `clip` is the region this context paints into; `viewport` the
/// document's clip, which `position: fixed` descendants clip to.
fn paint_stacking_context(
    dom: &Dom<TuiExt>,
    root: NodeId,
    buf: &mut Buffer,
    clip: Rect,
    viewport: Rect,
) {
    if dom.node(root).node_type() == NodeType::Fragment {
        // The document root: no box of its own.
        let layers = collect_layers(dom, root, clip, viewport);
        paint_layers(dom, &layers.negative, buf, viewport);
        recurse_children(dom, root, buf, clip, viewport);
        paint_layers(dom, &layers.zero_auto, buf, viewport);
        paint_layers(dom, &layers.positive, buf, viewport);
        return;
    }
    let Some(frame) = paint_box(dom, root, buf, clip) else {
        return;
    };
    let layers = collect_layers(dom, root, frame.children_clip, viewport);
    paint_layers(dom, &layers.negative, buf, viewport);
    paint_content(dom, root, buf, clip, viewport, &frame);
    paint_layers(dom, &layers.zero_auto, buf, viewport);
    paint_layers(dom, &layers.positive, buf, viewport);
    buf.exit_compose_ctx(frame.saved_ctx);
}

fn paint_layers(dom: &Dom<TuiExt>, entries: &[LayerEntry], buf: &mut Buffer, viewport: Rect) {
    for e in entries {
        if e.context {
            paint_stacking_context(dom, e.id, buf, e.clip, viewport);
        } else {
            paint_plain(dom, e.id, buf, e.clip, viewport);
        }
    }
}

/// Paint an element as a plain box: its own box, then its in-flow
/// content. Used for in-flow elements and for `z-index: auto`
/// positioned boxes, whose positioned descendants belong to the
/// enclosing context's layers.
fn paint_plain(dom: &Dom<TuiExt>, id: NodeId, buf: &mut Buffer, clip: Rect, viewport: Rect) {
    let Some(frame) = paint_box(dom, id, buf, clip) else {
        return;
    };
    paint_content(dom, id, buf, clip, viewport, &frame);
    buf.exit_compose_ctx(frame.saved_ctx);
}

/// Find every open modal `<dialog>` (any element with both `open`
/// and `data-rdom-modal` attributes), overlay its `::backdrop`
/// style across the viewport, and re-paint the dialog subtree on
/// top. Works without z-index support by running as a post-pass.
fn paint_modal_backdrops(dom: &Dom<TuiExt>, buf: &mut Buffer, clip: Rect) {
    let mut modals: Vec<NodeId> = Vec::new();
    collect_modal_dialogs(dom, dom.root(), &mut modals);
    for dialog_id in modals {
        let Some(backdrop_style) = dom
            .node(dialog_id)
            .ext()
            .and_then(|e| e.computed_backdrop.clone())
        else {
            continue;
        };
        fill_backdrop(buf, clip, &backdrop_style);
        paint_stacking_context(dom, dialog_id, buf, clip, clip);
    }
}

fn collect_modal_dialogs(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    let node = dom.node(id);
    if node.tag_name() == Some("dialog")
        && node.has_attribute("open")
        && node.has_attribute("data-rdom-modal")
    {
        out.push(id);
    }
    for child in node.child_nodes() {
        collect_modal_dialogs(dom, child.id(), out);
    }
}

/// Fill every cell of `clip` with the backdrop's bg (and optional
/// fg). Uses `Buffer::cell_mut` so the pre-existing symbols are
/// preserved underneath — apps that want a solid wipe set an
/// explicit `content: " "` override on `dialog::backdrop`.
fn fill_backdrop(buf: &mut Buffer, clip: Rect, style: &ComputedStyle) {
    let bg = style.bg;
    let fg = style.fg;
    for y in clip.y..clip.bottom() {
        for x in clip.x..clip.right() {
            let Some(cell) = buf.cell_mut(x, y) else {
                continue;
            };
            if bg != Color::Reset {
                cell.set_bg(bg);
            }
            if fg != Color::Reset {
                cell.set_fg(fg);
            }
        }
    }
}

// ─── Per-node paint ─────────────────────────────────────────────────

/// What [`paint_box`] hands to [`paint_content`]: the style with the
/// transition presentation overlaid, the content rect, the clip the
/// content paints into, and the compose context the caller exits once
/// the element's content — and, for a stacking context, its layers —
/// have painted.
struct BoxFrame {
    computed: ComputedStyle,
    inner: LayoutRect,
    children_clip: Rect,
    saved_ctx: (f32, Color),
}

/// Paint an element's own box — background fill and border — and enter
/// its compose context. `None` for non-elements and `display: none`,
/// which paint nothing and have no content to paint.
fn paint_box(dom: &Dom<TuiExt>, id: NodeId, buf: &mut Buffer, clip: Rect) -> Option<BoxFrame> {
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
    if let Some(ext) = dom.node(id).ext() {
        if let Some(fg) = ext.presentation.fg {
            computed.fg = fg;
        }
        if let Some(bg) = ext.presentation.bg {
            computed.bg = bg;
        }
        if let Some(border_fg) = ext.presentation.border_fg {
            computed.border_fg = border_fg;
        }
        if let Some(padding) = &ext.presentation.padding {
            computed.padding = padding.clone();
        }
        if let Some(gap) = ext.presentation.gap {
            computed.gap = crate::layout::GapValue::Cells(gap);
        }
    }

    // `display: none` — element takes no space and neither it
    // nor its children paint. Matches CSS semantics.
    if computed.display == Display::None {
        return None;
    }

    // CSS `opacity` — cell-level compositing. We enter a compose
    // context on the buffer that turns every subsequent cell write
    // (fill_bg, paint_border, text/inline/pseudo, builtins) into an
    // alpha-blend against the cell's actual existing bg. The
    // resolved `parent_bg` is the fallback when a cell's bg is
    // `Color::Reset` (its own fallback is `#000000`, the
    // canvas-model default — terminals don't expose their real
    // default bg). For `opacity = 1.0` the context is a no-op
    // fast path; the colors flow through unchanged. For
    // `opacity = 0` the context blends every write fully toward
    // the destination — the element is visually invisible without
    // erasing the cells it overlays.
    //
    // The per-cell blend resolves against the actual cell, which
    // captures whatever paint deposited there earlier — including a
    // z-stacked element below the painter with its own bg.
    let parent_bg = resolve_parent_bg(dom, id);
    let saved_ctx = buf.enter_compose_ctx(computed.opacity, parent_bg);

    let outer = dom.node(id).layout_rect().unwrap_or_default();
    let inner = dom.node(id).content_layout_rect().unwrap_or(outer);
    // CSS Overflow 3 §3: the scrollport is the padding-box. Overflow
    // clipping, scrollbar paint, and sticky pinning all use this rect,
    // never the layout-side `content_layout` (which under M5.5b border-
    // collapse can widen into the border ring for child positioning —
    // a layout concern, not a paint-clipping one).
    let padding_box = rdom_style::layout::compute_padding_box(outer, computed.border);

    // Fast path: element entirely outside the clip.
    if let Some(outer_grid) = layout_rect_to_grid(outer, clip) {
        // 1. Background fill over outer rect. Pass `computed.opacity`
        // so the fill chooses the right compositing regime: opaque
        // fills clear glyphs from earlier paints (full CSS occlusion);
        // translucent fills set `cell.bg` only and let underlying
        // glyphs bleed through. See `border.rs::fill_bg` for the rule.
        // Tree rows defer their background to the guide pass
        // (`tree_guides`), which fills the FULL row — including the
        // guide gutter to the left of the indented box — so the
        // `aria-selected` / cursor highlight spans edge to edge. A
        // normal box fill here would both stop at the indented box's
        // left edge AND tint the whole open subtree (the box
        // contains the nested group).
        let is_tree_row = dom.node(id).get_attribute("role") == Some("treeitem");
        if computed.bg != Color::Reset && !is_tree_row {
            // For `border: half-block`, skip painting bg under the
            // border cells — the half-block paint relies on the
            // surrounding (parent) bg showing through the "empty"
            // half of each glyph to produce the pill silhouette.
            // This is the rdom analog of CSS `background-clip:
            // padding-box`, hard-coded for the half-block style.
            // See DIVERGENCES.md for the wider story.
            let fill_area = if border_has_half_block(computed.border)
                && let Some(pb_grid) = layout_rect_to_grid(padding_box, clip)
            {
                pb_grid
            } else {
                outer_grid
            };
            fill_bg(buf, fill_area, computed.bg, computed.opacity);
        }

        // 2. Border. Writes per-cell × per-direction `BorderContribution`s
        // into `buf.border_dirs`; the joiner reads them after the
        // walk and emits the right glyph + color. BORDER-MODEL-1
        // priority encodes "child wins over ancestor" (depth) and
        // "earlier DOM order wins on tie" (`NodeId` proxy for
        // geometric position).
        if !computed.border.is_empty() {
            let priority = compute_border_priority(dom, id);
            paint_border(
                buf,
                outer,
                computed.border,
                computed.border_fg,
                clip,
                priority,
            );
        }
    }
    // Else: element off-screen — skip its box but still paint its
    // content; a scrolled-off parent may have visible children when
    // overflow is `Visible`.

    // Inner paint (text + pseudo-elements + children) happens in
    // `content_layout`, clipped by the element's overflow mode at the
    // padding-box edge per CSS Overflow 3 §3 (`stacking::children_clip`).
    let children_clip = children_clip(dom, id, &computed, clip);

    Some(BoxFrame {
        computed,
        inner,
        children_clip,
        saved_ctx,
    })
}

/// Paint an element's content: its canvas callback, or its inline
/// formatting context, or `::before` / own text / `::after` plus its
/// in-flow children and anonymous blocks; then its scrollbars. The
/// caller exits `frame.saved_ctx` afterwards.
fn paint_content(
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
        paint_ifc(dom, id, computed, text_inner, buf, children_clip);
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
        scrollbar::paint_scrollbars(dom, id, computed, buf, clip);
        return;
    }

    // Compute ::before / own text / ::after paint positions.
    let text_inner = crate::render::inline::scrolled_content_rect(dom, id).unwrap_or(inner);
    paint_inline_content(dom, id, computed, text_inner, buf, children_clip);

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
    paint_anonymous_blocks(dom, id, buf, children_clip);

    // Scrollbar overlay (after children so it sits on top if
    // anything encroached).
    scrollbar::paint_scrollbars(dom, id, computed, buf, clip);
}

/// Paint the in-flow element children of `id` in tree order. Positioned
/// children are skipped — they paint from the enclosing stacking
/// context's layers — and a child that establishes a stacking context
/// without being positioned (`opacity < 1`) paints atomically in place.
fn recurse_children(dom: &Dom<TuiExt>, id: NodeId, buf: &mut Buffer, clip: Rect, viewport: Rect) {
    for child in dom.node(id).child_nodes() {
        let cid = child.id();
        match child.node_type() {
            NodeType::Element => {
                // Display:inline children outside an IFC context are
                // a cascade error in CSS — but in rdom, a flex
                // container with at least one `display: inline-block`
                // child (e.g. `<button>`) escapes IFC and treats all
                // its children as flex items (see
                // `layout_pass/ifc.rs`'s `is_ifc_block`). In that
                // case an inline-display child gets a real layout
                // rect plus its own `inline_layout` from the pure-
                // text-leaf branch in flex layout — and must paint
                // through the normal element path.
                //
                // The original suppression still applies to "truly
                // orphaned" inline elements (no layout rect, no
                // inline_layout) — keep skipping those so they don't
                // paint as zero-sized blocks at (0,0).
                let computed_ref = child.ext().and_then(|e| e.computed.as_ref());
                let is_inline = computed_ref
                    .map(|c| c.display == Display::Inline)
                    .unwrap_or(false);
                let has_inline_layout =
                    child.ext().and_then(|e| e.inline_layout.as_ref()).is_some();
                if is_inline && !has_inline_layout {
                    continue;
                }
                match computed_ref {
                    Some(c) if is_positioned(c) => continue,
                    Some(c) if creates_stacking_context(c) => {
                        paint_stacking_context(dom, cid, buf, clip, viewport);
                    }
                    _ => paint_plain(dom, cid, buf, clip, viewport),
                }
            }
            NodeType::Fragment => recurse_children(dom, cid, buf, clip, viewport),
            NodeType::Text | NodeType::Comment => {} // consumed by parent's inline pass
        }
    }
}

// ─── LayoutRect → Rect clipping (shared utility) ────────────────────

/// Convert a `LayoutRect` (signed) to an unsigned grid `Rect`
/// clipped to `clip`. Returns `None` when the layout rect has no
/// visible area within the clip.
///
/// Used by [`paint_box`], [`inline_paint::paint_ifc`],
/// [`inline_paint::paint_inline_content`] and `stacking::children_clip`.
pub(crate) fn layout_rect_to_grid(layout: LayoutRect, clip: Rect) -> Option<Rect> {
    // Convert layout to inclusive-exclusive signed bounds.
    let left = layout.x;
    let top = layout.y;
    let right = layout.x + layout.width as i32;
    let bottom = layout.y + layout.height as i32;

    // Intersect with clip (unsigned → signed).
    let cl = clip.x as i32;
    let ct = clip.y as i32;
    let cr = clip.right() as i32;
    let cb = clip.bottom() as i32;

    let x = left.max(cl);
    let y = top.max(ct);
    let r = right.min(cr);
    let b = bottom.min(cb);
    if r <= x || b <= y {
        return None;
    }
    Some(Rect::new(
        x as u16,
        y as u16,
        (r - x) as u16,
        (b - y) as u16,
    ))
}

// ─── CSS `opacity` alpha-blend (T4) ─────────────────────────────────

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

/// True iff any of the element's four border sides uses the
/// half-block style. Drives the padding-box `fill_bg` clip in the
/// paint flow above — bg under half-block border cells must NOT
/// be painted, so the parent's bg shows through the empty half of
/// each half-block glyph (producing the pill silhouette).
fn border_has_half_block(border: rdom_style::layout::Border) -> bool {
    use rdom_style::layout::BorderStyle;
    matches!(border.top, BorderStyle::HalfBlock)
        || matches!(border.right, BorderStyle::HalfBlock)
        || matches!(border.bottom, BorderStyle::HalfBlock)
        || matches!(border.left, BorderStyle::HalfBlock)
}

/// Resolve the parent background color by walking up the DOM tree.
/// Returns the first ancestor's non-`Reset` `computed.bg`, or
/// `Color::Reset` if no ancestor has set one. Caller decides what
/// `Reset` means for blending — the canvas model is `#000000`.
fn resolve_parent_bg(dom: &Dom<TuiExt>, id: NodeId) -> Color {
    let mut cur = dom.node(id).parent_node();
    while let Some(node) = cur {
        if let Some(c) = node.ext().and_then(|e| e.computed.as_ref())
            && c.bg != Color::Reset
        {
            return c.bg;
        }
        cur = node.parent_node();
    }
    Color::Reset
}

// `alpha_blend` lives in `render::compose` for shared use by
// `Buffer`'s cell-write path. `paint_pass` doesn't call it directly
// — opacity is applied at write time via the buffer's compose
// context (see `Buffer::enter_compose_ctx`).
