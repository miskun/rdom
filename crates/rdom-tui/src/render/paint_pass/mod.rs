//! The paint pass.
//!
//! Walks the cascaded + laid-out `Dom<TuiExt>` and writes cells into
//! a `Buffer`. Runs *after* `cascade` + `layout_dom`. The cascade
//! produced `ComputedStyle` per element; layout produced
//! `LayoutRect`. Paint consumes both and emits the final grid of
//! cells.
//!
//! ## Paint order
//!
//! An element's own box (`box_paint::paint_box`): its outer shadows
//! (`box-shadow` shades outside the border box), its background fill
//! (`computed.bg` over the box its `background-clip` names; skipped for
//! `Color::Reset`), its inset shadows, its border (per-direction
//! contributions the joiner turns into glyphs). Its content
//! (`box_paint::paint_content`): its inline content — the classic
//! `::before` / own text / `::after` path, or the IFC fragment path —
//! its in-flow children and its scrollbars.
//!
//! Between boxes the order is CSS 2.1 Appendix E's (`stacking_walk`):
//! within each paint unit — a stacking context, a `z-index: auto`
//! positioned box, a float, an atomic box — the in-flow block-level
//! boxes' own boxes in tree order (step 4), the floats (step 5), then the
//! inline content in tree order (step 7), an atomic box whole at its turn.
//!
//! ## Stacking
//!
//! Positioned children do not paint in the recursion: they belong to
//! the layers of the nearest stacking context (CSS 2.1 Appendix E),
//! which `paint_stacking_context` paints around the context root's
//! in-flow content — see `crate::render::stacking`.
//!
//! ## Clipping
//!
//! - Entry takes a `clip: Rect` — the terminal-grid region the
//!   caller wants to paint into. Nothing outside `clip` is ever
//!   written.
//! - Each element's paint is intersected with `clip` via
//!   `layout_rect_to_grid`.
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
//! - `mod.rs` — public `PaintExt` trait and the shared
//!   `layout_rect_to_grid` clip utility.
//! - `stacking_walk` — the stacking-context walk
//!   (`paint_stacking_context`, `paint_layers`, `recurse_children`) and
//!   each paint unit's phases (`paint_unit`, `paint_atomic`).
//! - `box_paint` — one element's box (`box_frame`, `paint_box`) and
//!   content (`paint_content`), the border priority, and `fills`.
//! - `background` — the `background-color` fill, clipped by
//!   `background-clip`.
//! - `shadow` — `box-shadow`: outer shades under the background, inset
//!   ones above it.
//! - `border` — border drawing: per-direction contributions for the
//!   joiner (`border_join`), and half-block quadrants (`half_block`).
//! - `group` — `opacity` group rendering through a bounded layer.
//! - `top_layer` — the top layer (modal dialogs, popovers) and each
//!   element's `::backdrop`, painted after the document.
//! - `inline_paint` — `::before` + own text + `::after` for
//!   non-IFC elements; fragment-driven IFC paint. Split into the
//!   fragment painter (`mod.rs`), the chrome-substitution seam
//!   (`chrome`), the single-row painter (`single_row`), the caret
//!   (`caret`) and the highlight overlays (`highlight_overlay`:
//!   `::highlight()` and `::selection`).
//! - `text` — `paint_text` low-level helper + `ComputedStyle` →
//!   `Style` conversion.

mod background;
mod border;
mod border_join;
mod box_paint;
mod generated_box;
mod group;
mod inline_paint;
pub(crate) mod outline;
pub(crate) mod scrollbar;
mod shadow;
mod stacking_walk;
mod text;
pub(crate) mod top_layer;
mod tree_guides;

#[cfg(test)]
mod color_tests;
#[cfg(test)]
mod phase_cost_tests;
#[cfg(test)]
mod tests;

use rdom_core::Dom;

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::render::{Buffer, Rect};

pub(crate) use box_paint::fills;
// The chrome-substitution contract the built-ins implement
// (`runtime::builtins::inline_chrome`); see `inline_paint::chrome`.
pub(crate) use inline_paint::{ChromeText, InlineChromeFn};
pub(crate) use stacking_walk::paints_child_box;
use stacking_walk::{paint_line_atom, paint_stacking_context};

/// Extension trait on `Dom<TuiExt>` adding `paint_dom(buf, clip)`.
pub trait PaintExt: crate::sealed::Sealed {
    /// Paint the entire cascaded + laid-out DOM into `buf`, clipped
    /// to `clip`. Assumes `cascade()` and `layout_dom()` have
    /// already run; paint reads from `ComputedStyle` and
    /// `LayoutRect` only.
    fn paint_dom(&self, buf: &mut Buffer, clip: Rect);
}

impl PaintExt for Dom<TuiExt> {
    fn paint_dom(&self, buf: &mut Buffer, clip: Rect) {
        // Translucent paints blend the terminal's default colors as the
        // canvas of the document's color scheme.
        buf.set_color_scheme(crate::style::CascadeExt::color_scheme(self));
        // The document is the root stacking context (CSS 2.1 Appendix
        // E): positioned descendants — positioned `::before` / `::after`
        // included — paint from its layers, nested contexts recursively.
        paint_stacking_context(self, self.root(), buf, clip, clip);
        // The top layer (CSS Position 4): modal dialogs and popovers,
        // each over its `::backdrop`, above the whole document.
        top_layer::paint_top_layer(self, buf, clip);
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

// ─── LayoutRect → Rect clipping (shared utility) ────────────────────

/// Convert a `LayoutRect` (signed) to an unsigned grid `Rect`
/// clipped to `clip`. Returns `None` when the layout rect has no
/// visible area within the clip.
///
/// Used by `box_paint::paint_box`, `inline_paint::paint_ifc`,
/// `inline_paint::paint_inline_content` and `stacking::children_clip`.
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

// `alpha_blend` lives in `render::compose`; opacity is applied when a
// subtree's layer composites back (`Buffer::composite_group`).
