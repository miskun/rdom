//! Scrollbar paint — track + thumb for each scrollable axis of
//! an element with `overflow: scroll` or `overflow: auto`.
//!
//! The bars sit in the gutters layout reserved between the scrollport
//! and the border (CSS Overflow 3 §5.2; [`tracks`]):
//!
//! - **Vertical scrollbar**: the column right of the scrollport (left of
//!   it under `direction: rtl`), over the scrollport's rows.
//! - **Horizontal scrollbar**: the row below the scrollport, under its
//!   columns.
//! - **Corner** where the two gutters meet: left unpainted.
//!
//! ## Visibility
//!
//! - `Scroll` → always paints a track; thumb fills the track
//!   when content fits, shrinks proportionally when it overflows.
//! - `Auto` → paints nothing when the area fits the scrollport. The gutter was reserved either way so the
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
use crate::layout::Overflow;
use crate::node::TuiNodeExt;
use crate::render::layout_pass::gutter::bar_on_left;
use crate::render::layout_pass::scrollport_of;
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
        if super::fills(p.bg) {
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
        if super::fills(p.bg) {
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
/// overflow properties demand them ([`tracks`]). No-op when both axes are
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
    let (vertical, horizontal) = tracks(dom, id);
    if vertical.is_none() && horizontal.is_none() {
        return;
    }
    let look = Look::of(dom, computed);
    // Both track and thumb glyphs are axis-sensitive (`│` vs `─` for the
    // track; `┃` vs `━` for the thumb).
    if let Some(track) = vertical {
        let (track_glyph, track_style) = look.track(
            ext.computed_scrollbar().map(|s| &**s),
            ScrollbarAxis::Vertical,
        );
        let (thumb_glyph, thumb_style) = look.thumb(
            ext.computed_scrollbar_thumb_vertical().map(|s| &**s),
            ScrollbarAxis::Vertical,
        );
        paint_track(
            buf,
            track,
            true,
            clip,
            (track_glyph, track_style),
            (thumb_glyph, thumb_style),
        );
    }
    if let Some(track) = horizontal {
        let (track_glyph, track_style) = look.track(
            ext.computed_scrollbar().map(|s| &**s),
            ScrollbarAxis::Horizontal,
        );
        let (thumb_glyph, thumb_style) = look.thumb(
            ext.computed_scrollbar_thumb_horizontal().map(|s| &**s),
            ScrollbarAxis::Horizontal,
        );
        paint_track(
            buf,
            track,
            false,
            clip,
            (track_glyph, track_style),
            (thumb_glyph, thumb_style),
        );
    }
}

/// Paint one bar's cells inside `clip`: the thumb's in `thumb`, the rest
/// in `rest`.
fn paint_track(
    buf: &mut Buffer,
    track: Track,
    vertical: bool,
    clip: Rect,
    rest: (&str, Style),
    thumb: (&str, Style),
) {
    let (thumb_size, thumb_off) = track.thumb();
    for i in 0..track.len {
        let along = track.start + i32::from(i);
        let (x, y) = if vertical {
            (track.line, along)
        } else {
            (along, track.line)
        };
        let inside = x >= i32::from(clip.x)
            && x < i32::from(clip.right())
            && y >= i32::from(clip.y)
            && y < i32::from(clip.bottom());
        if !inside {
            continue;
        }
        let (ch, style) = if i >= thumb_off && i < thumb_off + thumb_size {
            thumb
        } else {
            rest
        };
        buf.set_symbol(x as u16, y as u16, ch, style);
    }
}

/// Should a scrollbar paint given the overflow mode + whether
/// content actually exceeds the viewport?
///
/// - `Scroll` → always, even when content fits.
/// - `Auto`   → only when content > viewport.
/// - Anything else → never (caller shouldn't even reach here).
#[deny(clippy::wildcard_enum_match_arm)]
pub(crate) fn should_paint(overflow: Overflow, viewport: usize, content: usize) -> bool {
    match overflow {
        Overflow::Scroll => true,
        Overflow::Auto => content > viewport,
        Overflow::Visible | Overflow::Hidden | Overflow::Clip => false,
    }
}

/// Which of `ext`'s scrollbars show: `(vertical, horizontal)` — on an
/// axis whose gutter layout reserved (`ScrollState::gutters`; none
/// under `scrollbar-width: none`, CSS Scrollbars 1 §3), an `overflow:
/// scroll` bar always and an `auto` one when the area overflows the
/// scrollport. Paint, hit-testing, thumb dragging and the tree guides
/// read this one answer.
pub(crate) fn bars_shown(ext: &crate::ext::TuiExt, computed: &ComputedStyle) -> (bool, bool) {
    let g = crate::runtime::scrollbar::state::gutters(ext);
    let port = scrollport_of(ext, computed);
    (
        g.columns() > 0
            && should_paint(
                computed.overflow_y,
                usize::from(port.height),
                ext.scroll_content_height,
            ),
        g.bottom > 0
            && should_paint(
                computed.overflow_x,
                usize::from(port.width),
                ext.scroll_content_width,
            ),
    )
}

/// One scrollbar's track, in the gutter beside the scrollport (CSS
/// Overflow 3 §5.2) and as long as the scrollport's side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Track {
    /// The bar's column (a vertical bar) or row (a horizontal one).
    pub(crate) line: i32,
    /// The track's first cell along the bar.
    pub(crate) start: i32,
    /// The track's length in cells.
    pub(crate) len: u16,
    /// The scrollport's size on the bar's axis.
    pub(crate) viewport: usize,
    /// The scrollable overflow area's size on the bar's axis.
    pub(crate) content: usize,
    /// The scrollport's distance from the area's start on the axis.
    pub(crate) offset: usize,
}

impl Track {
    /// `(thumb_size, thumb_offset)` in cells ([`thumb_geometry`]).
    pub(crate) fn thumb(&self) -> (u16, u16) {
        thumb_geometry(self.len, self.viewport, self.content, self.offset)
    }
}

/// `id`'s shown scrollbars, `(vertical, horizontal)` ([`bars_shown`]):
/// the vertical bar in the gutter column right of the scrollport (left of
/// it under [`bar_on_left`]), over its rows; the horizontal bar in the
/// gutter row below it, under its columns. The corner where they meet is
/// in neither.
pub(crate) fn tracks(dom: &Dom<TuiExt>, id: NodeId) -> (Option<Track>, Option<Track>) {
    let Some(ext) = dom.node(id).ext() else {
        return (None, None);
    };
    let Some(c) = ext.computed.as_deref() else {
        return (None, None);
    };
    let (y_shown, x_shown) = bars_shown(ext, c);
    let port = scrollport_of(ext, c);
    let (off_x, off_y) = crate::render::layout_pass::offset_from_area_start(dom, id);
    let vertical = y_shown.then(|| Track {
        line: if bar_on_left(c) {
            port.x - 1
        } else {
            port.x + i32::from(port.width)
        },
        start: port.y,
        len: port.height,
        viewport: usize::from(port.height),
        content: ext.scroll_content_height,
        offset: off_y,
    });
    let horizontal = x_shown.then(|| Track {
        line: port.y + i32::from(port.height),
        start: port.x,
        len: port.width,
        viewport: usize::from(port.width),
        content: ext.scroll_content_width,
        offset: off_x,
    });
    (vertical, horizontal)
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
