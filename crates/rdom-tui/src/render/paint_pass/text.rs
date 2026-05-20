//! Low-level text paint helper + `ComputedStyle` → paint `Style`
//! conversion.
//!
//! All inline / IFC / ::before / ::after paint paths ultimately
//! funnel through `paint_text` so Unicode-width handling, clipping,
//! and wide-glyph placement stay in one place.

use unicode_width::UnicodeWidthStr;

use crate::render::{Buffer, Style};
use crate::style::{Color, ComputedStyle, Modifier};

/// Paint `text` starting at `(x, base_y)`, clipped to
/// `[x..budget_right)` on that row. Returns the x-cursor after the
/// last painted cell so callers can chain multiple paints
/// (`::before` → own text → `::after`).
pub(super) fn paint_text(
    buf: &mut Buffer,
    x: u16,
    base_y: u16,
    budget_right: u16,
    text: &str,
    style: Style,
) -> u16 {
    if x >= budget_right || text.is_empty() {
        return x;
    }
    let max_width = budget_right - x;
    let text_width = UnicodeWidthStr::width(text).min(max_width as usize) as u16;
    let _end = buf.set_stringn(x, base_y, text, max_width, style);
    x + text_width
}

/// Build a paint-layer `Style` from a `ComputedStyle`, including
/// `bg`. Filters `Color::Reset` (means "no color set, use terminal
/// default") and keeps only the modifier bits we actually support.
///
/// **Use this only when the caller is the one painting the bg
/// for the cells it writes** — i.e. the cells don't already have
/// their bg set by an upstream `fill_bg`. Cases:
/// - `::before` / `::after` static pseudo content (the pseudo's
///   bg, if any, paints in the pseudo's cells without a separate
///   `fill_bg`).
/// - `positioned_pseudos` content (the positioning pass does its
///   own bg fill, but this helper is also used for the glyph
///   style — bg in style is redundant at `opacity = 1.0` and
///   currently produces a small double-blend at `opacity < 1.0`,
///   which is documented as deferred polish).
/// - IFC fragments whose owner is an inline-level element with
///   its own `background-color` (the inline child has no
///   `fill_bg` of its own; its bg paints via the fragment glyph
///   style).
///
/// For all other glyph paints (the element's own text, gauge /
/// select chrome, password mask, IFC fragments owned by the IFC
/// block itself) use `glyph_style_from_computed` below — the
/// cell's `bg` is already owned by the upstream `fill_bg`, and
/// including `bg` in the glyph style would cause a second blend
/// pass at `opacity < 1.0`, brightening text cells relative to
/// non-text cells (the "text cells have a different bg from the
/// surrounding bg" symptom).
///
/// This split is the project's paint-layer invariant in practice:
/// `fill_bg` owns `cell.bg`; glyph painters write
/// `symbol + fg + modifiers`.
pub(super) fn style_from_computed(c: &ComputedStyle) -> Style {
    let mut style = Style::new();
    if c.fg != Color::Reset {
        style = style.fg(c.fg);
    }
    if c.bg != Color::Reset {
        style = style.bg(c.bg);
    }
    let mods = c.modifiers
        & (Modifier::BOLD | Modifier::ITALIC | Modifier::UNDERLINED | Modifier::REVERSED);
    if !mods.is_empty() {
        style = style.add_modifier(mods);
    }
    style
}

/// Same as `style_from_computed` but **omits `bg`** — for glyph
/// paints where the cell's `bg` is already owned by an upstream
/// `fill_bg`. See `style_from_computed`'s doc for the call-site
/// split.
pub(super) fn glyph_style_from_computed(c: &ComputedStyle) -> Style {
    let mut style = Style::new();
    if c.fg != Color::Reset {
        style = style.fg(c.fg);
    }
    let mods = c.modifiers
        & (Modifier::BOLD | Modifier::ITALIC | Modifier::UNDERLINED | Modifier::REVERSED);
    if !mods.is_empty() {
        style = style.add_modifier(mods);
    }
    style
}
