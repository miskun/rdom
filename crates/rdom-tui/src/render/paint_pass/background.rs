//! Background fill: an element's `background-color`, painted in the
//! box its `background-clip` names (CSS Backgrounds 3 §3.8).
//!
//! The fill blanks every cell it covers (glyph, foreground, modifiers,
//! border contributions) so the element's own border, text and
//! pseudo-element content land on a clean canvas — an opaque box
//! occludes what earlier paints left there.

use super::{fills, layout_rect_to_grid};
use crate::layout::{BorderCollapse, LayoutRect, compute_content_area, compute_padding_box};
use crate::render::{Buffer, Modifier, Rect};
use crate::style::{Color, ComputedStyle};
use rdom_style::layout::{Border, BorderStyle, VisualBox};

/// Paint `computed`'s background color for a box whose border box is
/// `outer` and whose laid-out content box is `inner`, clipped to
/// `clip`. A translucent color composites over what is beneath
/// (C3-ALPHA): the opaque fill is made in a layer.
pub(super) fn paint_background(
    buf: &mut Buffer,
    computed: &ComputedStyle,
    outer: LayoutRect,
    inner: LayoutRect,
    clip: Rect,
) {
    if !fills(computed.bg) {
        return;
    }
    let Some(area) = layout_rect_to_grid(clip_box(computed, outer, inner), clip) else {
        return;
    };
    if computed.bg.is_translucent() {
        let alpha = f32::from(computed.bg.alpha()) / 255.0;
        let bg = computed.bg.opaque();
        buf.paint_translucent(area, alpha, |layer| fill_bg(layer, area, bg));
    } else {
        fill_bg(buf, area, computed.bg);
    }
}

/// The box the background color is painted in: the final layer's
/// `background-clip` (§3.2, §3.8) — the border box (initial), the
/// padding box, or the content box.
///
/// A half-block border never takes the background in its cells: its
/// glyph fills the inward half of the cell and the outward half must
/// show what is beneath, or the pill silhouette is lost. So with any
/// half-block side the box is at most the padding box (DIVERGENCES §2,
/// half-block).
fn clip_box(computed: &ComputedStyle, outer: LayoutRect, inner: LayoutRect) -> LayoutRect {
    let border = computed.border;
    match computed.background_clip {
        VisualBox::BorderBox if !has_half_block(border) => outer,
        VisualBox::BorderBox | VisualBox::PaddingBox => compute_padding_box(outer, border),
        // The laid-out content box, but under `border-collapse:
        // collapse` that box reaches into the shared border ring, so
        // the content box is derived from the padding box instead.
        VisualBox::ContentBox if computed.border_collapse == BorderCollapse::Collapse => {
            compute_content_area(outer, computed.padding.clone(), border)
        }
        VisualBox::ContentBox => inner,
    }
}

/// True iff any border side uses the half-block style.
fn has_half_block(border: Border) -> bool {
    [border.top, border.right, border.bottom, border.left].contains(&BorderStyle::HalfBlock)
}

/// Fill `area` cells with `bg` as an opaque CSS box: writes
/// `cell.bg = bg` AND **clears `cell.symbol` to SPACE**, clears
/// `cell.fg` to `Color::Reset`, clears `cell.modifier`, and clears
/// the cell's border contributions. Any glyph an earlier paint
/// deposited in this cell is replaced by a blank canvas, ready for
/// this element's own border / text / pseudo content to paint over
/// it. Without this clear, glyphs from lower z-layers (or earlier
/// tree-order paints) leak through an opaque overlay's bg — the
/// visible bug in `positioning_demo` before 2026-05-18.
///
/// There is one regime: `opacity` is not a per-fill concern. An
/// `opacity < 1` subtree paints into its own layer at full opacity
/// and `Buffer::composite_group` folds the layer back, which is
/// where a translucent box's glyphs beneath show through (OPACITY-1).
///
/// `Color::Reset` for `bg` is honored at the call site (caller
/// gates `if computed.bg != Color::Reset`); this function assumes
/// the caller has decided to paint.
///
/// Wide-glyph handling: when the opaque clear writes a SPACE over
/// a wide-glyph primary cell, the partner spacer at `x+1` is also
/// cleared (and vice versa for spacer cells). Without this pairing,
/// the fill would leave half-cell residue at area boundaries that
/// straddle a wide glyph.
pub(super) fn fill_bg(buf: &mut Buffer, area: Rect, bg: Color) {
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            // Clear symbol/fg/modifier so the element's subsequent
            // border/text/pseudo paints land on a blank canvas, then
            // write the raw bg. Translucency is not a per-fill concern
            // any more: an `opacity < 1` subtree paints into its own
            // layer at full opacity and `Buffer::composite_group` blends
            // the layer (OPACITY-1).
            clear_cell_for_opaque_fill(buf, x, y);
            // BORDER-MODEL-1: an opaque fill completely occludes
            // anything painted at this cell earlier in the walk,
            // including border contributions from underlying
            // elements. Clear the per-direction state so the
            // joiner doesn't re-emit a border glyph here.
            clear_border_dirs(buf, x, y);
            if let Some(cell) = buf.cell_mut(x, y) {
                cell.bg = bg;
            }
        }
    }
}

/// Set `area`'s background to `bg` under what is painted there: the
/// glyphs, their colors and modifiers stay, border contributions are
/// cleared (a background covers the borders painted before it). An
/// in-flow box's opaque outer shadow at its turn in tree order — under
/// the text of the boxes before it (`shadow`).
pub(super) fn tint_bg(buf: &mut Buffer, area: Rect, bg: Color) {
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            clear_border_dirs(buf, x, y);
            if let Some(cell) = buf.cell_mut(x, y) {
                cell.bg = bg;
            }
        }
    }
}

/// Clear the per-direction border state at `(x, y)`. Called by
/// the opaque-`fill_bg` fast path so subsequent joiner runs don't
/// resurrect border glyphs that the opaque fill should occlude.
fn clear_border_dirs(buf: &mut Buffer, x: u16, y: u16) {
    use crate::render::buffer::BorderDirState;
    for dir in 0..4 {
        buf.set_border_dir(x, y, dir, BorderDirState::default());
    }
    // Half-block borders accumulate inward quadrants separately; an opaque
    // fill occludes those too, so the joiner doesn't re-emit a half-block
    // glyph under the fill.
    buf.clear_half_block_quads(x, y);
}

/// Clear `(x, y)` to a blank cell suitable for opaque-bg overpainting:
/// SPACE symbol, `fg = Reset`, no modifier. If the cell is the primary
/// of a wide glyph, also clear the trailing spacer at `x+1`; if it's a
/// spacer, also clear the primary at `x-1`. Without this pairing, an
/// area whose edge cuts a wide glyph would leave a half-cell residue.
fn clear_cell_for_opaque_fill(buf: &mut Buffer, x: u16, y: u16) {
    let (also_clear_x, primary_side): (Option<u16>, _) = match buf.cell(x, y) {
        Some(c) if c.is_spacer() => (x.checked_sub(1), "spacer's primary"),
        Some(c) if c.cell_width() == 2 => (Some(x.saturating_add(1)), "wide-glyph spacer"),
        _ => (None, ""),
    };
    let _ = primary_side; // documentation hint only
    if let Some(partner_x) = also_clear_x
        && let Some(partner) = buf.cell_mut(partner_x, y)
    {
        partner.set_symbol(" ");
        partner.fg = Color::Reset;
        partner.modifier = Modifier::empty();
    }
    if let Some(cell) = buf.cell_mut(x, y) {
        cell.set_symbol(" ");
        cell.fg = Color::Reset;
        cell.modifier = Modifier::empty();
    }
}
