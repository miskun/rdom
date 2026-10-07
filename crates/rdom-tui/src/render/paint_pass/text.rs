//! Low-level text paint helper + `ComputedStyle` → paint `Style`
//! conversion.
//!
//! All inline / IFC / ::before / ::after paint paths ultimately
//! funnel through `paint_text` so Unicode-width handling, clipping,
//! and wide-glyph placement stay in one place.

use unicode_segmentation::UnicodeSegmentation;
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
    // z-aware borders: painted content occludes any border the joiner would
    // otherwise re-derive at its cells. Paint runs in stacking order and the
    // joiner runs last, so clearing here means a higher element's content wins
    // over a lower element's border beneath it — matching CSS paint order
    // (content paints above the box's own border). Borders added *after* this
    // paint (higher elements) are untouched, so a higher border still shows
    // over lower content. Mirrors the opaque-`fill_bg` occlusion. Invisible
    // text occludes nothing; translucent text only where its glyph wins.
    let _end = buf.write_text(x, base_y, text, max_width, style, true);
    x + text_width
}

/// [`style_from_computed`] for a pseudo-element, with that slot's
/// in-flight transition overrides applied (`D-M3-3`).
pub(super) fn pseudo_style(c: &ComputedStyle, overrides: &crate::ext::PresentationStyle) -> Style {
    let mut style = style_from_computed(c);
    if let Some(fg) = overrides.fg {
        style = style.fg(fg);
    }
    if let Some(bg) = overrides.bg {
        style = style.bg(bg);
    }
    style
}

/// [`pseudo_style`] without the background: the glyphs of a
/// pseudo-element laid out as a box of its own, whose box painted its
/// background under them once (CSS Backgrounds 3 §3.10).
pub(super) fn pseudo_glyph_style(
    c: &ComputedStyle,
    overrides: &crate::ext::PresentationStyle,
) -> Style {
    let mut style = glyph_style_from_computed(c);
    if let Some(fg) = overrides.fg {
        style = style.fg(fg);
    }
    style
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
/// - IFC fragments whose owner is an inline-level element with
///   its own `background-color` (the inline child has no
///   `fill_bg` of its own; its bg paints via the fragment glyph
///   style).
///
/// For all other glyph paints (the element's own text, gauge /
/// select chrome, password mask, IFC fragments owned by the IFC
/// block itself, a positioned pseudo-element's content — its box is
/// tinted first) use `glyph_style_from_computed` below — the
/// cell's `bg` is already owned by the upstream `fill_bg`, so a
/// glyph paint leaves it alone. (Before group opacity, OPACITY-1,
/// a bg in the glyph style blended a second time and brightened
/// text cells; a translucent background color still would, since a
/// glyph write composites its style's background — the split keeps
/// one owner per cell bg.)
///
/// This split is the project's paint-layer invariant in practice:
/// `fill_bg` owns `cell.bg`; glyph painters write
/// `symbol + fg + modifiers`.
pub(super) fn style_from_computed(c: &ComputedStyle) -> Style {
    let mut style = Style::new();
    if c.fg != Color::Reset {
        style = style.fg(c.fg);
    }
    if super::fills(c.bg) {
        style = style.bg(c.bg);
    }
    text_modifiers(style, c)
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
    text_modifiers(style, c)
}

/// `style` with the CSS-author-visible cell modifiers of text styled
/// `c`, each an SGR code (`sgr::emit_sgr_transition`): the font's bold and
/// italic, and the decorations drawn on the text (CSS Text Decoration 4
/// §2.1, `ComputedStyle::applied_decorations`) — an underline in its
/// style and, where it is not the text's, its color; an overline; a
/// line-through; blink. A terminal draws the overline and line-through in
/// the text's color: their own color has no SGR (DIVERGENCES §2). Other
/// modifier bits (hidden, the caret's and the selection's) are written at
/// their paint sites.
fn text_modifiers(mut style: Style, c: &ComputedStyle) -> Style {
    let d = &c.applied_decorations;
    let mut mods = c.modifiers & (Modifier::BOLD | Modifier::ITALIC);
    // The glyph's color unless the decorating box's differs.
    let mut underline_color = Color::Reset;
    if let Some(underline) = d.underline {
        mods |= Modifier::UNDERLINED | underline_style(underline.style);
        if underline.color != c.fg {
            underline_color = underline.color;
        }
    }
    style = style.underline_color(underline_color);
    if d.overline.is_some() {
        mods |= Modifier::OVERLINED;
    }
    if d.line_through.is_some() {
        mods |= Modifier::CROSSED_OUT;
    }
    if d.blink {
        mods |= Modifier::SLOW_BLINK;
    }
    if !mods.is_empty() {
        style = style.add_modifier(mods);
    }
    style
}

/// The modifier bit of an underline's style (SGR 4:2–4:5); none for
/// `solid`, a plain underline.
fn underline_style(style: crate::layout::TextDecorationStyle) -> Modifier {
    use crate::layout::TextDecorationStyle as S;
    match style {
        S::Solid => Modifier::empty(),
        S::Double => Modifier::UNDERLINE_DOUBLE,
        S::Wavy => Modifier::UNDERLINE_CURLY,
        S::Dotted => Modifier::UNDERLINE_DOTTED,
        S::Dashed => Modifier::UNDERLINE_DASHED,
    }
}

pub(super) fn advance_text_by_cells(text: &str, cells: u16) -> &str {
    let mut consumed: u16 = 0;
    let mut byte_pos: usize = 0;
    for (idx, g) in text.grapheme_indices(true) {
        if consumed >= cells {
            byte_pos = idx;
            return &text[byte_pos..];
        }
        let w = UnicodeWidthStr::width(g) as u16;
        consumed = consumed.saturating_add(w);
        byte_pos = idx + g.len();
    }
    &text[byte_pos..]
}

/// Paint `text` whose logical start is `x` (which may lie left of
/// `clip_left`): the cells that fall before the clip are skipped, not
/// wrapped to the clip edge (`D-M5N-8`), and the rest paints from
/// `max(x, clip_left)` up to `budget_right`. Returns the logical end
/// cursor `x + width(text)` so callers can chain runs regardless of
/// clipping.
pub(super) fn paint_text_from(
    buf: &mut Buffer,
    x: i32,
    y: u16,
    clip_left: u16,
    budget_right: u16,
    text: &str,
    style: Style,
) -> i32 {
    let logical_end = x + UnicodeWidthStr::width(text) as i32;
    if x >= i32::from(clip_left) {
        let _ = paint_text(buf, x as u16, y, budget_right, text, style);
    } else {
        let skip = (i32::from(clip_left) - x).min(i32::from(u16::MAX)) as u16;
        let rest = advance_text_by_cells(text, skip);
        if !rest.is_empty() {
            let _ = paint_text(buf, clip_left, y, budget_right, rest, style);
        }
    }
    logical_end
}
