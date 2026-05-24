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
//! 4. **Recurse** — element children paint at their own `layout`
//!    rects.
//!
//! ## Clipping
//!
//! - Entry takes a `clip: Rect` — the terminal-grid region the
//!   caller wants to paint into. Nothing outside `clip` is ever
//!   written.
//! - Each element's paint is intersected with `clip` via
//!   [`layout_rect_to_grid`].
//! - For `overflow: Hidden | Scroll | Auto`, children are recursed
//!   with a tighter clip = `element.content_layout ∩ clip`.
//! - `overflow: Visible` keeps the incoming clip — children can
//!   draw past the parent.
//!
//! ## Module layout
//!
//! - `mod.rs` — public `PaintExt` trait + `paint_node` dispatch +
//!   `recurse_children` + the shared `layout_rect_to_grid` clip
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

#[cfg(test)]
mod tests;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Border, Display, LayoutRect, Overflow};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::is_ifc_block;
use crate::render::{Buffer, Rect};
use crate::style::{Color, ComputedStyle};

use border::{fill_bg, paint_border};
use inline_paint::{paint_caret_if_editable, paint_ifc, paint_inline_content};

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
        paint_node(self, self.root(), buf, clip);
        // Positioned elements paint after the document walk in
        // z-order (= flat stacking context). `recurse_children`
        // already skipped them.
        paint_z_list(self, buf, clip);
        // Positioned `::before` / `::after` pseudo-elements paint
        // AFTER the z-list. Pseudos have no `NodeId` and can't be
        // widened into `z_list` cleanly; the separate pass also
        // matches the "pseudos don't participate in hit-test" rule
        // and reuses the same `(host.z_index, doc_order,
        // pseudo_order)` sort key.
        positioned_pseudos::paint_positioned_pseudos(self, buf, clip);
        // Overlay backdrop behind modal dialogs. Runs AFTER the main
        // paint pass so the backdrop reliably sits on top of whatever
        // else painted into the viewport — then we re-paint the
        // dialog subtree so it ends up above the backdrop.
        paint_modal_backdrops(self, buf, clip);
        // Border-collapse joiner. Walks the buffer once and rewrites
        // box-drawing glyphs at junctions based on 4-neighbor
        // connectivity. Cheap when no element has `border-collapse:
        // collapse` (short-circuits via the bottom-up tree flag).
        border_join::join_borders(self, buf);
    }
}

/// Walk the entire tree, collect every `position: absolute | fixed`
/// element, sort by `(z_index, document_order)`, and paint each
/// element's subtree in turn. `z-index: auto` resolves to 0 for
/// sorting; document order is the tiebreaker.
///
/// `paint_node` for a positioned element drops back to the regular
/// recursive walk — `recurse_children` skips nested positioned
/// descendants, so they only paint when this z-list itself reaches
/// them. Nested positioned elements collapse into the same flat
/// sort against the root viewport.
fn paint_z_list(dom: &Dom<TuiExt>, buf: &mut Buffer, clip: Rect) {
    let mut list: Vec<(i16, usize, NodeId)> = Vec::new();
    let mut order: usize = 0;
    collect_z_list(dom, dom.root(), &mut list, &mut order);
    list.sort_by_key(|(z, ord, _)| (*z, *ord));
    for (_, _, id) in list {
        paint_node(dom, id, buf, clip);
    }
}

fn collect_z_list(
    dom: &Dom<TuiExt>,
    id: NodeId,
    out: &mut Vec<(i16, usize, NodeId)>,
    order: &mut usize,
) {
    if dom.node(id).node_type() == NodeType::Element {
        let computed = dom.node(id).ext().and_then(|e| e.computed.as_ref());
        let positioned = computed
            .map(|c| {
                matches!(
                    c.position,
                    crate::layout::Position::Absolute | crate::layout::Position::Fixed
                )
            })
            .unwrap_or(false);
        if positioned {
            let z = computed
                .map(|c| match c.z_index {
                    crate::layout::ZIndex::Auto => 0,
                    crate::layout::ZIndex::Value(n) => n,
                })
                .unwrap_or(0);
            out.push((z, *order, id));
            *order += 1;
        }
    }
    for child in dom.node(id).child_nodes() {
        match child.node_type() {
            NodeType::Element | NodeType::Fragment => {
                collect_z_list(dom, child.id(), out, order);
            }
            _ => {}
        }
    }
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
        paint_node(dom, dialog_id, buf, clip);
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

fn paint_node(dom: &Dom<TuiExt>, id: NodeId, buf: &mut Buffer, clip: Rect) {
    let ty = dom.node(id).node_type();

    // Fragment: no own box, just recurse into element children with
    // the same clip. (Layout pass puts fragments through
    // transparently too.)
    if ty == NodeType::Fragment {
        recurse_children(dom, id, buf, clip);
        return;
    }

    // Text and Comment don't paint on their own — they're consumed
    // via their parent element's "own text" pass.
    if ty != NodeType::Element {
        return;
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
        if let Some(padding) = ext.presentation.padding {
            computed.padding = padding;
        }
        if let Some(gap) = ext.presentation.gap {
            computed.gap = gap;
        }
    }

    // `display: none` — element takes no space and neither it
    // nor its children paint. Matches CSS semantics. Replaces
    // the pre-Display::None hack of `width:0; height:0;
    // overflow:hidden` + the per-case `is_option_in_closed_dropdown`
    // paint skip.
    if computed.display == Display::None {
        return;
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
    // This replaces the pre-Phase-2 cascade-time pre-bake of
    // `computed.fg/bg/border_fg = alpha_blend(.., parent_bg)`,
    // which only blended against the painter's DOM parent — wrong
    // when a z-stacked element below the painter has its own bg.
    // The per-cell blend resolves against the actual cell, which
    // captures whatever paint deposited there earlier.
    let parent_bg = resolve_parent_bg(dom, id);
    let saved_ctx = buf.enter_compose_ctx(computed.opacity, parent_bg);

    let outer = dom.node(id).layout_rect().unwrap_or_default();
    let inner = dom.node(id).content_layout_rect().unwrap_or(outer);

    // Fast path: element entirely outside the clip.
    if let Some(outer_grid) = layout_rect_to_grid(outer, clip) {
        // 1. Background fill over outer rect. Pass `computed.opacity`
        // so the fill chooses the right compositing regime: opaque
        // fills clear glyphs from earlier paints (full CSS occlusion);
        // translucent fills set `cell.bg` only and let underlying
        // glyphs bleed through. See `border.rs::fill_bg` for the rule.
        if computed.bg != Color::Reset {
            fill_bg(buf, outer_grid, computed.bg, computed.opacity);
        }

        // 2. Border. Glyph paint only — `paint_border` does not write
        // `cell.bg`; the bg invariant is owned by `fill_bg` above.
        if !matches!(computed.border, Border::None) {
            paint_border(buf, outer, computed.border, computed.border_fg, clip);
        }
    } else {
        // Element off-screen: skip paint but still recurse — a
        // scrolled-off parent may have visible children when
        // overflow: Visible.
    }

    // Inner paint (text + pseudo-elements + children) happens in
    // `content_layout`, clipped by the element's overflow mode.
    // Either axis being non-Visible clips (matches browser
    // behavior — you can't have a half-clipped element).
    let clips = !matches!(computed.overflow_x, Overflow::Visible)
        || !matches!(computed.overflow_y, Overflow::Visible);
    let children_clip = if clips {
        layout_rect_to_grid(inner, clip).unwrap_or_else(|| Rect::new(clip.x, clip.y, 0, 0))
    } else {
        clip
    };

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
        scrollbar::paint_scrollbars(dom, id, &computed, buf, clip);
        buf.exit_compose_ctx(saved_ctx);
        return;
    }

    // IFC block: paint the inline formatting context (interleaved
    // text + inline element children, each with its own cascaded
    // style) and skip both the default own-text paint and child
    // recursion.
    if is_ifc_block(dom, id) {
        paint_ifc(dom, id, &computed, inner, buf, children_clip);
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
        scrollbar::paint_scrollbars(dom, id, &computed, buf, clip);
        buf.exit_compose_ctx(saved_ctx);
        return;
    }

    // Compute ::before / own text / ::after paint positions.
    paint_inline_content(dom, id, &computed, inner, buf, children_clip);

    // Caret overlay for pure-text leaf blocks (e.g. <input>,
    // <textarea>) — they go through `paint_inline_content` rather
    // than `paint_ifc`, so the caret has to be painted at this call
    // site too. `paint_caret_if_editable` is a no-op for elements
    // that aren't the inline-flow container of the focused caret.
    if crate::render::inline::has_inline_layout(dom, id) {
        paint_caret_if_editable(dom, buf, id, children_clip);
    }

    // Recurse into element children (they paint at their own layouts).
    recurse_children(dom, id, buf, children_clip);

    // Scrollbar overlay (after children so it sits on top if
    // anything encroached).
    scrollbar::paint_scrollbars(dom, id, &computed, buf, clip);

    buf.exit_compose_ctx(saved_ctx);
}

fn recurse_children(dom: &Dom<TuiExt>, id: NodeId, buf: &mut Buffer, clip: Rect) {
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
                // M2: position: absolute / fixed children are
                // skipped here — they paint via the z-list post-
                // pass after the document walk completes (see
                // `paint_z_list`). Lifted into the global stacking
                // context.
                let is_positioned = computed_ref
                    .map(|c| {
                        matches!(
                            c.position,
                            crate::layout::Position::Absolute | crate::layout::Position::Fixed
                        )
                    })
                    .unwrap_or(false);
                if is_positioned {
                    continue;
                }
                paint_node(dom, cid, buf, clip);
            }
            NodeType::Fragment => paint_node(dom, cid, buf, clip),
            NodeType::Text | NodeType::Comment => {} // consumed by parent's inline pass
        }
    }
}

// ─── LayoutRect → Rect clipping (shared utility) ────────────────────

/// Convert a `LayoutRect` (signed) to an unsigned grid `Rect`
/// clipped to `clip`. Returns `None` when the layout rect has no
/// visible area within the clip.
///
/// Used by [`paint_node`], [`inline_paint::paint_ifc`], and
/// [`inline_paint::paint_inline_content`].
pub(super) fn layout_rect_to_grid(layout: LayoutRect, clip: Rect) -> Option<Rect> {
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
