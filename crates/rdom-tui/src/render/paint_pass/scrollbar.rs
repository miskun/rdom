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
//! `TuiExt::computed_scrollbar` / `computed_scrollbar_thumb` for
//! scrollable elements; paint reads them via `track_cell` /
//! `thumb_cell` and falls back to a minimal DarkGray-bg gutter
//! when the cascade output is `None` (i.e. consumer used
//! `Stylesheet::bare()` and didn't supply their own rules).

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{LayoutRect, Overflow};
use crate::node::TuiNodeExt;
use crate::render::{Buffer, Rect, Style};
use crate::style::{Color, ComputedStyle};

/// Fallback track glyph when no `::scrollbar { content }` rule
/// supplies one. Empty space — the track reads as a colored
/// gutter via `bg`, no foreground rail glyph. The UA stylesheet
/// installs the canonical default; this fallback only fires for
/// `Stylesheet::bare()` (UA-free) consumers.
const FALLBACK_TRACK_CHAR: &str = " ";

/// Vertical scrollbar thumb fallback glyph. `┃` U+2503 BOX
/// DRAWINGS HEAVY VERTICAL. Used when the cascade output for
/// `::scrollbar-thumb` has no `content` property set — which is
/// the UA-default state: the UA rule supplies only `bg` + `fg`
/// so the paint layer can pick the axis-appropriate glyph.
const FALLBACK_THUMB_V: &str = "┃";

/// Horizontal scrollbar thumb fallback glyph. `━` U+2501 BOX
/// DRAWINGS HEAVY HORIZONTAL. Mirror of [`FALLBACK_THUMB_V`].
const FALLBACK_THUMB_H: &str = "━";

/// Which scrollbar axis we're resolving styling for.
/// Used to pick the right paint-level fallback thumb glyph when
/// the cascade's `::scrollbar-thumb { content }` is unset.
#[derive(Debug, Clone, Copy)]
enum ScrollbarAxis {
    Vertical,
    Horizontal,
}

/// Resolve the per-cell `Style` and glyph for a scrollbar track
/// cell. Reads `::scrollbar` if cascade populated one; otherwise
/// falls back to a minimal DarkGray-bg gutter so the scrollbar
/// is visible even against a `Stylesheet::bare()` (no-UA)
/// configuration. Track is axis-agnostic — bg color works the
/// same vertically and horizontally.
fn track_cell<'a>(pseudo: Option<&'a crate::style::ComputedStyle>) -> (&'a str, Style) {
    if let Some(p) = pseudo {
        let glyph: &'a str = p.content.as_deref().unwrap_or(FALLBACK_TRACK_CHAR);
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
            FALLBACK_TRACK_CHAR,
            Style::new().bg(Color::Rgb(169, 169, 169)),
        )
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

    let (scroll_x, scroll_y) = (ext.scroll_x, ext.scroll_y);
    let (content_w, content_h) = (ext.scroll_content_width, ext.scroll_content_height);

    // Track is axis-agnostic — resolve once.
    let (track_glyph, track_style) = track_cell(ext.computed_scrollbar.as_ref());
    // Thumb glyph IS axis-sensitive (`┃` vs `━` default).
    // Resolve per-axis at the call site below.

    // CSS `scrollbar-gutter`: `Scroll` axes always reserve a
    // gutter (and paint the track inside it). `Auto` axes only
    // reserve when `scrollbar-gutter: stable` is set; otherwise
    // the track overlays the rightmost content column / bottom
    // content row when a scrollbar appears. This must agree with
    // `layout_pass::reserve_scrollbar_gutter`'s decision.
    use crate::layout::ScrollbarGutter;
    let reserves = |o: Overflow| match o {
        Overflow::Scroll => true,
        Overflow::Auto => matches!(computed.scrollbar_gutter, ScrollbarGutter::Stable),
        Overflow::Hidden | Overflow::Visible => false,
    };
    let y_gutter = reserves(computed.overflow_y);
    let x_gutter = reserves(computed.overflow_x);

    let y_paints = matches!(computed.overflow_y, Overflow::Scroll | Overflow::Auto);
    let x_paints = matches!(computed.overflow_x, Overflow::Scroll | Overflow::Auto);

    if y_paints {
        let (thumb_glyph, thumb_style) = thumb_cell(
            ext.computed_scrollbar_thumb.as_ref(),
            ScrollbarAxis::Vertical,
        );
        paint_vertical_scrollbar(
            buf,
            content_layout,
            x_gutter,
            y_gutter,
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
        let (thumb_glyph, thumb_style) = thumb_cell(
            ext.computed_scrollbar_thumb.as_ref(),
            ScrollbarAxis::Horizontal,
        );
        paint_horizontal_scrollbar(
            buf,
            content_layout,
            y_gutter,
            x_gutter,
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
    content: LayoutRect,
    has_h_scrollbar: bool,
    self_reserves_gutter: bool,
    overflow: Overflow,
    scroll_offset: usize,
    content_size: usize,
    clip: Rect,
    track_glyph: &str,
    track_style: Style,
    thumb_glyph: &str,
    thumb_style: Style,
) {
    // Track column placement depends on `scrollbar-gutter`:
    // - Reserved (`stable` or `Scroll` overflow): track lives at
    //   `content.right()` — the dedicated gutter column.
    // - Not reserved (`auto` + `Auto` overflow): track overlays
    //   the rightmost content column at `content.right() - 1`.
    //   Content cells at that column get overwritten when the
    //   scrollbar shows. Matches CSS `scrollbar-gutter: auto`.
    let track_x_signed = if self_reserves_gutter {
        content.x + content.width as i32
    } else {
        content.x + content.width as i32 - 1
    };
    let track_x = track_x_signed as i64;
    if track_x < clip.x as i64 || track_x >= clip.right() as i64 {
        return;
    }
    let track_x = track_x as u16;

    let track_top = content.y.max(clip.y as i32);
    let mut track_bottom = (content.y + content.height as i32).min(clip.bottom() as i32);
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
    content: LayoutRect,
    has_v_scrollbar: bool,
    self_reserves_gutter: bool,
    overflow: Overflow,
    scroll_offset: usize,
    content_size: usize,
    clip: Rect,
    track_glyph: &str,
    track_style: Style,
    thumb_glyph: &str,
    thumb_style: Style,
) {
    // Track row placement — symmetric to vertical case. See
    // `paint_vertical_scrollbar` for the rationale.
    let track_y_signed = if self_reserves_gutter {
        content.y + content.height as i32
    } else {
        content.y + content.height as i32 - 1
    };
    let track_y = track_y_signed as i64;
    if track_y < clip.y as i64 || track_y >= clip.bottom() as i64 {
        return;
    }
    let track_y = track_y as u16;

    let track_left = content.x.max(clip.x as i32);
    let mut track_right = (content.x + content.width as i32).min(clip.right() as i32);
    if has_v_scrollbar {
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
