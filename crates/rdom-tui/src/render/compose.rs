//! Color compositing helpers shared by the paint pass and the
//! buffer's cell-write methods.
//!
//! `alpha_blend(src, alpha, dst)` performs straight-alpha
//! composition: `out = α·src + (1-α)·dst`. It is called by
//! `Buffer::composite_group`, which blends a layer back onto the
//! backdrop — a subtree's at the element's `opacity` (CSS group
//! opacity, OPACITY-1), or one translucent paint's at its color's
//! alpha (C3-ALPHA) — so every paint inside blends exactly once.
use crate::style::Color;
use rdom_style::color::ColorScheme;

/// Resolve a background colour through the canvas model: `Color::Reset`
/// (the terminal's default background) is the canvas background of
/// `scheme` ([`ColorScheme::canvas`]) — terminals report their colours
/// only on request, so compositing takes the scheme's.
pub(crate) fn canvas_bg(c: Color, scheme: ColorScheme) -> Color {
    if c == Color::Reset {
        scheme.canvas().0
    } else {
        c
    }
}

/// Resolve a foreground colour through the canvas model: `Color::Reset`
/// (the terminal's default foreground) is the canvas text of `scheme`.
pub(crate) fn canvas_fg(c: Color, scheme: ColorScheme) -> Color {
    if c == Color::Reset {
        scheme.canvas().1
    } else {
        c
    }
}

/// Alpha-blend `src` over `dst` using `alpha` ∈ [0, 1].
/// Straight-alpha compositing: `out = α·src + (1-α)·dst`.
///
/// - `alpha >= 1.0` → returns `src` unchanged.
/// - `src == Color::Reset` → returns `Color::Reset` (no source color
///   to blend; preserves the "transparent" sentinel). Callers that
///   mean a default colour resolve it first with [`canvas_fg`] /
///   [`canvas_bg`].
/// - `src` is a palette index → returns `src` unchanged.
/// - `dst == Color::Reset` (or a palette index) → blends against black.
///   Callers resolve the default first ([`canvas_bg`]).
///
/// Both colours' own alpha is ignored: the callers pass opaque ones.
pub(crate) fn alpha_blend(src: Color, alpha: f32, dst: Color) -> Color {
    if alpha >= 1.0 {
        return src;
    }
    let alpha = alpha.clamp(0.0, 1.0);
    let (sr, sg, sb) = match src {
        Color::Rgb(r, g, b) | Color::Rgba(r, g, b, _) => (r, g, b),
        Color::Reset => return Color::Reset,
        _ => return src,
    };
    let (dr, dg, db) = match dst {
        Color::Rgb(r, g, b) | Color::Rgba(r, g, b, _) => (r, g, b),
        _ => (0, 0, 0),
    };
    let blend = |s: u8, d: u8| -> u8 {
        let mixed = alpha * s as f32 + (1.0 - alpha) * d as f32;
        mixed.round().clamp(0.0, 255.0) as u8
    };
    Color::Rgb(blend(sr, dr), blend(sg, dg), blend(sb, db))
}
