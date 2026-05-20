//! Color compositing helpers shared by the paint pass and the
//! buffer's cell-write methods.
//!
//! `alpha_blend(src, alpha, dst)` performs straight-alpha
//! composition: `out = α·src + (1-α)·dst`. The function used to
//! live in `paint_pass/mod.rs`, where it pre-blended an element's
//! colors against the resolved `parent_bg` at cascade time (Phase
//! 1 of the opacity rollout). Phase 2 moved the call site to the
//! buffer's cell-write path so the blend resolves against the
//! cell's actual `bg` — correct for z-stacked overlays where the
//! cell underneath has a different bg from the painter's DOM
//! parent. The function itself is unchanged; only its callers moved.

use crate::style::Color;

/// Alpha-blend `src` over `dst` using `alpha` ∈ [0, 1].
/// Straight-alpha compositing: `out = α·src + (1-α)·dst`.
///
/// - `alpha >= 1.0` → returns `src` unchanged.
/// - `src == Color::Reset` → returns `Color::Reset` (no source color
///   to blend; preserves the "transparent" sentinel).
/// - `src` is non-`Rgb` (Indexed, ANSI palette) → returns `src`
///   unchanged. T6 collapsed ANSI to RGB at the cascade level, but
///   the through-path is preserved for defensive parsing.
/// - `dst == Color::Reset` (or other non-`Rgb`) → blends against the
///   `#000000` canvas model. Terminals don't expose their actual
///   default bg, so we pick a deterministic fallback.
pub(crate) fn alpha_blend(src: Color, alpha: f32, dst: Color) -> Color {
    if alpha >= 1.0 {
        return src;
    }
    let alpha = alpha.clamp(0.0, 1.0);
    let (sr, sg, sb) = match src {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Reset => return Color::Reset,
        _ => return src,
    };
    let (dr, dg, db) = match dst {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (0, 0, 0),
    };
    let blend = |s: u8, d: u8| -> u8 {
        let mixed = alpha * s as f32 + (1.0 - alpha) * d as f32;
        mixed.round().clamp(0.0, 255.0) as u8
    };
    Color::Rgb(blend(sr, dr), blend(sg, dg), blend(sb, db))
}
