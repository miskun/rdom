//! Scrollbar paint — track + thumb for each scrollable axis of
//! an element with `overflow: scroll` or `overflow: auto`.
//!
//! The scrollbar strip sits in the 1-cell gutter that
//! [`reserve_scrollbar_gutter`] carved out during layout:
//!
//! - **Vertical scrollbar**: column at `content_layout.right()`,
//!   rows `content_layout.y` .. `content_layout.bottom()`.
//! - **Horizontal scrollbar**: row at `content_layout.bottom()`,
//!   columns `content_layout.x` .. `content_layout.right()`.
//! - **Corner** at `(right, bottom)`: left unpainted.
//!
//! ## Visibility
//!
//! - `Scroll` → always paints a track; thumb fills the track
//!   when content fits, shrinks proportionally when it overflows.
//! - `Auto` → paints nothing when content fits (`scroll_content_*`
//!   ≤ viewport). The gutter was reserved either way so the
//!   layout doesn't reflow.
//!
//! ## Thumb geometry
//!
//! ```text
//! thumb_size = max(1, viewport * viewport / content)          [cells]
//! thumb_pos  = scroll_offset * (track - thumb_size)
//!             / (content - viewport)                          [cells]
//! ```
//!
//! Clamped to `[0, track - thumb_size]` on both ends.
//!
//! ## Author styling
//!
//! Track and thumb cells are styled via the `::scrollbar` and
//! `::scrollbar-thumb` pseudo-elements (modeled after WebKit's
//! `::-webkit-scrollbar`). The cascade populates
//! `TuiExt::computed_scrollbar` / `computed_scrollbar_thumb_{vertical,horizontal}` for
//! scrollable elements; paint reads them via `track_cell` /
//! `thumb_cell` and falls back to a minimal DarkGray-bg gutter
//! when the cascade output is `None` (i.e. consumer used
//! `Stylesheet::bare()` and didn't supply their own rules).

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{LayoutRect, Overflow};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::gutter::{bar_on_left, vertical_bar_column};
use crate::render::{Buffer, Rect, Style};
use crate::style::{Color, ComputedStyle};

/// Vertical scrollbar track fallback glyph. `│` U+2502 BOX DRAWINGS
/// LIGHT VERTICAL. Used when the cascade output for `::scrollbar`
/// has no `content` property set (UA-default state). The light glyph
/// pairs with the heavy thumb [`FALLBACK_THUMB_V`] — same color,
/// different weight, gives the eye a clear track-vs-thumb signal
/// without needing a bg fill.
const FALLBACK_TRACK_V: &str = "│";

/// Horizontal scrollbar track fallback glyph. `─` U+2500 BOX DRAWINGS
/// LIGHT HORIZONTAL. Mirror of [`FALLBACK_TRACK_V`].
const FALLBACK_TRACK_H: &str = "─";

/// Vertical scrollbar thumb fallback glyph. `┃` U+2503 BOX
/// DRAWINGS HEAVY VERTICAL. Used when the cascade output for
/// `::scrollbar-thumb` has no `content` property set — which is
/// the UA-default state: the UA rule supplies only `fg` so the
/// paint layer can pick the axis-appropriate glyph.
const FALLBACK_THUMB_V: &str = "┃";

/// Horizontal scrollbar thumb fallback glyph. `━` U+2501 BOX
/// DRAWINGS HEAVY HORIZONTAL. Mirror of [`FALLBACK_THUMB_V`].
const FALLBACK_THUMB_H: &str = "━";

/// Which scrollbar axis we're resolving styling for.
/// Used to pick the right paint-level fallback glyph when the
/// cascade's `::scrollbar { content }` / `::scrollbar-thumb
/// { content }` is unset.
#[derive(Debug, Clone, Copy)]
enum ScrollbarAxis {
    Vertical,
    Horizontal,
}

/// Resolve the per-cell `Style` and glyph for a scrollbar track
/// cell. Reads `::scrollbar` if cascade populated one; otherwise
/// falls back to a minimal DarkGray-bg gutter so the scrollbar
/// is visible even against a `Stylesheet::bare()` (no-UA)
/// configuration. The track glyph IS axis-sensitive — the cascade
/// exposes a single `::scrollbar` rule, and paint picks `│` for
/// vertical and `─` for horizontal when `content` is unset (the
/// UA-default state).
fn track_cell<'a>(
    pseudo: Option<&'a crate::style::ComputedStyle>,
    axis: ScrollbarAxis,
) -> (&'a str, Style) {
    let fallback = match axis {
        ScrollbarAxis::Vertical => FALLBACK_TRACK_V,
        ScrollbarAxis::Horizontal => FALLBACK_TRACK_H,
    };
    if let Some(p) = pseudo {
        let glyph: &'a str = p.content.as_deref().unwrap_or(fallback);
        let mut style = Style::new();
        if p.bg != Color::Reset {
            style = style.bg(p.bg);
        }
        if p.fg != Color::Reset {
            style = style.fg(p.fg);
        }
        (glyph, style)
    } else {
        (fallback, Style::new().bg(Color::Rgb(169, 169, 169)))
    }
}

/// Resolve the per-cell `Style` and glyph for a scrollbar thumb
/// cell. Unlike the track, the thumb glyph IS axis-sensitive:
/// the cascade exposes a single `::scrollbar-thumb` rule; if
/// `content` is unset (UA default) paint picks `┃` for vertical
/// and `━` for horizontal. If the author specified `content`,
/// the literal glyph applies to both axes — picking a glyph
/// that reads both ways (block characters) is the documented
/// path. Per-axis pseudo-class targeting (`:vertical` /
/// `:horizontal`) is tracked as `UA-SB-1` in TECH_DEBT.
fn thumb_cell<'a>(
    pseudo: Option<&'a crate::style::ComputedStyle>,
    axis: ScrollbarAxis,
) -> (&'a str, Style) {
    let fallback = match axis {
        ScrollbarAxis::Vertical => FALLBACK_THUMB_V,
        ScrollbarAxis::Horizontal => FALLBACK_THUMB_H,
    };
    if let Some(p) = pseudo {
        let glyph: &'a str = p.content.as_deref().unwrap_or(fallback);
        let mut style = Style::new();
        if p.bg != Color::Reset {
            style = style.bg(p.bg);
        }
        if p.fg != Color::Reset {
            style = style.fg(p.fg);
        }
        (glyph, style)
    } else {
        (
            fallback,
            Style::new()
                .fg(Color::Rgb(128, 128, 128))
                .bg(Color::Rgb(169, 169, 169)),
        )
    }
}

/// Light thumb glyphs of a `scrollbar-width: thin` bar: the track's line
/// weight, where the default thumb is heavy.
const THIN_THUMB_V: &str = "│";
const THIN_THUMB_H: &str = "─";

/// What styles `computed`'s bars (CSS Scrollbars 1 §2–§3): rdom's
/// `::scrollbar` / `::scrollbar-thumb` pseudo-elements while
/// `scrollbar-width` and `scrollbar-color` are both `auto`; once either is
/// not, the standard properties alone, the pseudo-elements ignored —
/// Chromium's precedence over its `::-webkit-scrollbar` pseudo-elements.
#[derive(Clone, Copy)]
enum Look {
    Pseudos,
    /// `thin`, and the thumb and track colors (`None`: the platform's).
    Standard {
        thin: bool,
        colors: Option<(Color, Color)>,
    },
}

impl Look {
    fn of(dom: &Dom<TuiExt>, computed: &ComputedStyle) -> Self {
        use crate::layout::{ScrollbarColor, ScrollbarWidth};
        if computed.scrollbar_width == ScrollbarWidth::Auto
            && computed.scrollbar_color == ScrollbarColor::Auto
        {
            return Self::Pseudos;
        }
        let colors = match &computed.scrollbar_color {
            ScrollbarColor::Auto => None,
            // Specified colors, resolved against the element's own color
            // and used color scheme (`currentcolor`, `var()`,
            // `light-dark()`), as `caret-color` is.
            ScrollbarColor::Colors { thumb, track } => {
                let scheme = computed
                    .color_scheme
                    .used(crate::style::CascadeExt::color_scheme(dom));
                let cx = crate::ColorContext::new(computed.fg).with_scheme(scheme);
                let resolve = |c: &crate::TuiColor| c.resolve(&computed.vars, &cx);
                resolve(thumb).zip(resolve(track))
            }
        };
        Self::Standard {
            thin: computed.scrollbar_width == ScrollbarWidth::Thin,
            colors,
        }
    }

    /// A track cell: the pseudo-element's, or the standard bar's — none
    /// drawn when thin, else the light line; the track color filling the
    /// cell, or the platform's track color for the glyph.
    fn track(self, pseudo: Option<&ComputedStyle>, axis: ScrollbarAxis) -> (&str, Style) {
        let Self::Standard { thin, colors } = self else {
            return track_cell(pseudo, axis);
        };
        let glyph = match (thin, axis) {
            (true, _) => " ",
            (false, ScrollbarAxis::Vertical) => FALLBACK_TRACK_V,
            (false, ScrollbarAxis::Horizontal) => FALLBACK_TRACK_H,
        };
        let style = match colors {
            Some((_, track)) => Style::new().fg(track).bg(track),
            None if thin => Style::new(),
            None => Style::new().fg(crate::layout::NATIVE_SCROLLBAR_TRACK),
        };
        (glyph, style)
    }

    /// A thumb cell: the pseudo-element's, or the standard bar's — the
    /// light line when thin, else the heavy one — in the thumb color on
    /// the track color, or the platform's thumb color.
    fn thumb(self, pseudo: Option<&ComputedStyle>, axis: ScrollbarAxis) -> (&str, Style) {
        let Self::Standard { thin, colors } = self else {
            return thumb_cell(pseudo, axis);
        };
        let glyph = match (thin, axis) {
            (true, ScrollbarAxis::Vertical) => THIN_THUMB_V,
            (true, ScrollbarAxis::Horizontal) => THIN_THUMB_H,
            (false, ScrollbarAxis::Vertical) => FALLBACK_THUMB_V,
            (false, ScrollbarAxis::Horizontal) => FALLBACK_THUMB_H,
        };
        let style = match colors {
            Some((thumb, track)) => Style::new().fg(thumb).bg(track),
            None => Style::new().fg(crate::layout::NATIVE_SCROLLBAR_THUMB),
        };
        (glyph, style)
    }
}

/// Paint vertical and/or horizontal scrollbars for `id` if its
/// overflow properties demand them. No-op when both axes are
/// `Visible` / `Hidden`.
pub(super) fn paint_scrollbars(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    buf: &mut Buffer,
    clip: Rect,
) {
    let Some(ext) = dom.node(id).tui_ext() else {
        return;
    };
    let content_layout = ext.content_layout;
    // CSS Overflow 3 §3: the scrollbar gutter lives inside the padding-
    // box. Under M5.5b border-collapse `content_layout` can extend into
    // the border ring (a child-positioning concern); the scrollbar
    // track must NOT paint there. `padding_box` defines the spec-correct
    // outer bound for the track extent on both axes.
    let padding_box = crate::layout::compute_padding_box(ext.layout, computed.border);

    // The horizontal thumb is drawn at the scrollport's distance from
    // the left of the scrollable area — at the right at rest for an
    // `rtl` box, whose scroll origin is its right edge (CSSOM View §4).
    let scroll_x = crate::render::layout_pass::scroll_x_from_area_start(
        dom,
        id,
        content_layout.width as usize,
    );
    // The vertical thumb likewise: at the bottom at rest for a
    // `column-reverse` box (its scroll origin is the bottom edge).
    let scroll_y = crate::render::layout_pass::scroll_y_from_area_start(
        dom,
        id,
        content_layout.height as usize,
    );
    let (content_w, content_h) = (ext.scroll_content_width, ext.scroll_content_height);

    // Both track and thumb glyphs are axis-sensitive (`│` vs `─`
    // for the track; `┃` vs `━` for the thumb). Resolve per-axis at
    // the call sites below.

    // Both axes always reserve their gutter when the scrollbar
    // actually paints — see `layout_pass::reserve_scrollbar_gutter`
    // and its two-pass companion for `Auto`. We only need to know
    // whether the OTHER axis also paints so the bottom-right corner
    // stays unclaimed.
    let (y_paints, x_paints) = bars_shown(ext, computed);
    let look = Look::of(dom, computed);

    if y_paints {
        let (track_glyph, track_style) =
            look.track(ext.computed_scrollbar.as_deref(), ScrollbarAxis::Vertical);
        let (thumb_glyph, thumb_style) = look.thumb(
            ext.computed_scrollbar_thumb_vertical.as_deref(),
            ScrollbarAxis::Vertical,
        );
        paint_vertical_scrollbar(
            buf,
            vertical_bar_column(content_layout, computed),
            content_layout,
            padding_box,
            x_paints,
            computed.overflow_y,
            scroll_y,
            content_h,
            clip,
            track_glyph,
            track_style,
            thumb_glyph,
            thumb_style,
        );
    }
    if x_paints {
        let (track_glyph, track_style) =
            look.track(ext.computed_scrollbar.as_deref(), ScrollbarAxis::Horizontal);
        let (thumb_glyph, thumb_style) = look.thumb(
            ext.computed_scrollbar_thumb_horizontal.as_deref(),
            ScrollbarAxis::Horizontal,
        );
        paint_horizontal_scrollbar(
            buf,
            bar_on_left(computed),
            content_layout,
            padding_box,
            y_paints,
            computed.overflow_x,
            scroll_x,
            content_w,
            clip,
            track_glyph,
            track_style,
            thumb_glyph,
            thumb_style,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_vertical_scrollbar(
    buf: &mut Buffer,
    track_x: i32,
    content: LayoutRect,
    padding_box: LayoutRect,
    has_h_scrollbar: bool,
    overflow: Overflow,
    scroll_offset: usize,
    content_size: usize,
    clip: Rect,
    track_glyph: &str,
    track_style: Style,
    thumb_glyph: &str,
    thumb_style: Style,
) {
    // Track column = the dedicated gutter cell `track_x`, at
    // `content.right()` (or left of `content` under `rtl`).
    // The layout pass guarantees the gutter is reserved (via either
    // `Scroll` always-reserves, `Auto + scrollbar-gutter: stable`, or
    // `Auto`'s two-pass force-reserve when overflow is detected).
    // CSS Overflow 3 §3 + the TUI medium constraint (no cell overlay)
    // mean overlay positioning is unreachable for paint — by the
    // time the scrollbar actually paints, its column belongs to it.
    let track_x = i64::from(track_x);
    if track_x < clip.x as i64 || track_x >= clip.right() as i64 {
        return;
    }
    let track_x = track_x as u16;

    // Track vertical extent is bounded by the **padding-box**, not
    // `content_layout`. Under M5.5b border-collapse, `content_layout`
    // can widen vertically into the border ring (a child-positioning
    // concern). CSS Overflow 3 §3 places the scrollport at the
    // padding-box; the track lives inside that scrollport and must
    // not paint into the border row.
    let track_top = content.y.max(padding_box.y).max(clip.y as i32);
    let mut track_bottom = (content.y + content.height as i32)
        .min(padding_box.y + padding_box.height as i32)
        .min(clip.bottom() as i32);
    if has_h_scrollbar {
        track_bottom -= 1;
    }
    if track_bottom <= track_top {
        return;
    }
    let track_len = (track_bottom - track_top) as u16;
    let viewport = content.height;

    if !should_paint(overflow, viewport as usize, content_size) {
        return;
    }

    let (thumb_size, thumb_off) =
        thumb_geometry(track_len, viewport as usize, content_size, scroll_offset);

    for i in 0..track_len {
        let y = track_top as u16 + i;
        let in_thumb = i >= thumb_off && i < thumb_off + thumb_size;
        let (ch, style) = if in_thumb {
            (thumb_glyph, thumb_style)
        } else {
            (track_glyph, track_style)
        };
        buf.set_symbol(track_x, y, ch, style);
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_horizontal_scrollbar(
    buf: &mut Buffer,
    corner_left: bool,
    content: LayoutRect,
    padding_box: LayoutRect,
    has_v_scrollbar: bool,
    overflow: Overflow,
    scroll_offset: usize,
    content_size: usize,
    clip: Rect,
    track_glyph: &str,
    track_style: Style,
    thumb_glyph: &str,
    thumb_style: Style,
) {
    // Track row = the dedicated gutter row at `content.bottom()`.
    // Mirror of the vertical case — layout guarantees reservation by
    // the time paint runs.
    let track_y_signed = content.y + content.height as i32;
    let track_y = track_y_signed as i64;
    if track_y < clip.y as i64 || track_y >= clip.bottom() as i64 {
        return;
    }
    let track_y = track_y as u16;

    // Track horizontal extent bounded by the padding-box (mirror of
    // the vertical case): under M5.5b `content_layout` can widen into
    // the border ring on the left/right; the track must not paint
    // there per CSS Overflow 3 §3.
    let mut track_left = content.x.max(padding_box.x).max(clip.x as i32);
    let mut track_right = (content.x + content.width as i32)
        .min(padding_box.x + padding_box.width as i32)
        .min(clip.right() as i32);
    // The corner beside the vertical bar stays unclaimed: bottom-right,
    // or bottom-left under `rtl`.
    if has_v_scrollbar && corner_left {
        track_left += 1;
    } else if has_v_scrollbar {
        track_right -= 1;
    }
    if track_right <= track_left {
        return;
    }
    let track_len = (track_right - track_left) as u16;
    let viewport = content.width;

    if !should_paint(overflow, viewport as usize, content_size) {
        return;
    }

    let (thumb_size, thumb_off) =
        thumb_geometry(track_len, viewport as usize, content_size, scroll_offset);

    for i in 0..track_len {
        let x = track_left as u16 + i;
        let in_thumb = i >= thumb_off && i < thumb_off + thumb_size;
        let (ch, style) = if in_thumb {
            (thumb_glyph, thumb_style)
        } else {
            (track_glyph, track_style)
        };
        buf.set_symbol(x, track_y, ch, style);
    }
}

/// Should a scrollbar paint given the overflow mode + whether
/// content actually exceeds the viewport?
///
/// - `Scroll` → always, even when content fits.
/// - `Auto`   → only when content > viewport.
/// - Anything else → never (caller shouldn't even reach here).
pub(crate) fn should_paint(overflow: Overflow, viewport: usize, content: usize) -> bool {
    match overflow {
        Overflow::Scroll => true,
        Overflow::Auto => content > viewport,
        _ => false,
    }
}

/// Which of `ext`'s scrollbars show: `(vertical, horizontal)` — an
/// `overflow: scroll` axis always, an `auto` one when its content
/// overflows the scrollport; none under `scrollbar-width: none` (CSS
/// Scrollbars 1 §3: the box scrolls with no bar). Paint, hit-testing and
/// thumb dragging read this one answer, so the corner cell a horizontal
/// bar takes from the vertical track (and back) is the same in all three.
pub(crate) fn bars_shown(ext: &crate::ext::TuiExt, computed: &ComputedStyle) -> (bool, bool) {
    if computed.scrollbar_width == crate::layout::ScrollbarWidth::None {
        return (false, false);
    }
    let content = ext.content_layout;
    (
        should_paint(
            computed.overflow_y,
            usize::from(content.height),
            ext.scroll_content_height,
        ),
        should_paint(
            computed.overflow_x,
            usize::from(content.width),
            ext.scroll_content_width,
        ),
    )
}

/// Compute `(thumb_size, thumb_offset)` in cells for a track of
/// length `track` rendering a viewport of `viewport` inside a
/// content of length `content`, with `scroll_offset` cells
/// already scrolled. All values in cells.
pub(crate) fn thumb_geometry(
    track: u16,
    viewport: usize,
    content: usize,
    scroll_offset: usize,
) -> (u16, u16) {
    if content == 0 || content <= viewport {
        return (track, 0);
    }
    let content = content.max(1);
    // thumb_size = max(1, track * viewport / content)
    let thumb_size = (track as usize * viewport / content).max(1) as u16;
    let thumb_size = thumb_size.min(track);
    let travel = content.saturating_sub(viewport);
    let track_travel = track.saturating_sub(thumb_size) as usize;
    let thumb_off = (scroll_offset * track_travel)
        .checked_div(travel)
        .unwrap_or(0)
        .min(track_travel) as u16;
    (thumb_size, thumb_off)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_fills_track_when_content_fits() {
        let (size, off) = thumb_geometry(10, 20, 15, 0);
        assert_eq!(size, 10);
        assert_eq!(off, 0);
    }

    #[test]
    fn thumb_half_size_when_content_twice_viewport() {
        let (size, off) = thumb_geometry(10, 10, 20, 0);
        assert_eq!(size, 5);
        assert_eq!(off, 0);
    }

    #[test]
    fn thumb_at_bottom_when_scrolled_to_end() {
        let (size, off) = thumb_geometry(10, 10, 20, 10);
        assert_eq!(size, 5);
        // Track travel = 10 - 5 = 5. At end, thumb at offset 5.
        assert_eq!(off, 5);
    }

    #[test]
    fn thumb_min_size_1() {
        // Tall content vs. short track should still show a thumb.
        let (size, _) = thumb_geometry(5, 5, 10_000, 0);
        assert_eq!(size, 1);
    }

    #[test]
    fn should_paint_auto_hides_when_content_fits() {
        assert!(!should_paint(Overflow::Auto, 10, 10));
        assert!(!should_paint(Overflow::Auto, 10, 5));
        assert!(should_paint(Overflow::Auto, 10, 11));
    }

    #[test]
    fn should_paint_scroll_always_shows() {
        assert!(should_paint(Overflow::Scroll, 10, 5));
        assert!(should_paint(Overflow::Scroll, 10, 10));
        assert!(should_paint(Overflow::Scroll, 10, 100));
    }
}
