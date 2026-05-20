//! `<progress>` + `<meter>` paint helpers.
//!
//! ## Contract (from MDN)
//!
//! - `<progress>` is a determinate or indeterminate task-completion
//!   bar. Determinate: `value` attribute set, bar fills `value/max`
//!   of the available width. Indeterminate: no `value` attribute,
//!   bar shows an "in progress" marker (v1: dim hatching).
//! - `<meter>` is a static measurement gauge over `min..=max`. Bar
//!   fills `(value-min)/(max-min)`. Color reflects the optimum
//!   zone:
//!   - `optimum` is in [low, high] → "optimum" zone is also
//!     [low, high]; outside that range is "suboptimal", and the
//!     opposite extreme from `optimum` is "even-less-good".
//!   - `optimum < low` → optimum zone is [min, low].
//!   - `optimum > high` → optimum zone is [high, max].
//! - Both elements have no events and no editing — pure display.
//!
//! ## v1 deliberate simplifications
//!
//! - Indeterminate progress renders as static hatching (`░`)
//!   rather than animated. Animation would require a tick loop
//!   coupled to the runtime — out of scope for paint-only widgets.
//! - No fractional-cell precision: bar fill rounds to whole cells.
//!   Authors who want sub-cell precision would do it with a richer
//!   custom widget.
//! - Meter colors are hard-coded TUI tokens (LightGreen/Yellow/Red).
//!   Authors override by setting `fg` on the `<meter>` rule via
//!   author CSS — meter paint will respect the cascade fg over
//!   the zone color when explicitly set non-default.

use rdom_core::NodeId;

use crate::TuiDom;
use crate::style::{Color, ComputedStyle};

/// Glyph used to fill the active portion of a gauge.
const FILL: &str = "\u{2588}"; // █ FULL BLOCK
/// Glyph used for the inactive portion (empty track).
const EMPTY: &str = "\u{2591}"; // ░ LIGHT SHADE

/// True when this element is a gauge widget that the paint pass
/// should special-case. Saves the paint pass from doing two
/// separate tag checks.
pub fn is_gauge(dom: &TuiDom, id: NodeId) -> bool {
    matches!(dom.node(id).tag_name(), Some("progress") | Some("meter"))
}

/// Render a gauge bar string of exactly `width` cells. Returns
/// `None` when the element isn't a gauge, in which case the
/// paint pass falls through to its normal text rendering.
///
/// The optional `fg_override` is the foreground color the paint
/// pass should use INSTEAD of the cascaded fg. `<progress>`
/// returns `None` (cascade fg wins). `<meter>` returns the
/// zone-derived color for "suboptimal" / "bad" zones, or `None`
/// for "optimum" (cascade fg wins).
pub fn gauge_text(dom: &TuiDom, id: NodeId, width: u16) -> Option<(String, Option<Color>)> {
    match dom.node(id).tag_name() {
        Some("progress") => Some(progress_text(dom, id, width)),
        Some("meter") => Some(meter_text(dom, id, width)),
        _ => None,
    }
}

/// Render a `<progress>` bar.
fn progress_text(dom: &TuiDom, id: NodeId, width: u16) -> (String, Option<Color>) {
    let value = parse_attr(dom, id, "value");
    let max = parse_attr(dom, id, "max")
        .filter(|v| *v > 0.0)
        .unwrap_or(1.0);
    let bar = match value {
        Some(v) => {
            let ratio = (v / max).clamp(0.0, 1.0);
            fill_bar(width, ratio)
        }
        None => indeterminate_bar(width),
    };
    (bar, None)
}

/// Render a `<meter>` bar with zone color.
fn meter_text(dom: &TuiDom, id: NodeId, width: u16) -> (String, Option<Color>) {
    let min = parse_attr(dom, id, "min").unwrap_or(0.0);
    let max = parse_attr(dom, id, "max").unwrap_or(1.0);
    let value = parse_attr(dom, id, "value").unwrap_or(0.0).clamp(min, max);
    let span = (max - min).max(f64::EPSILON);
    let ratio = ((value - min) / span).clamp(0.0, 1.0);
    let bar = fill_bar(width, ratio);

    let color = meter_zone_color(dom, id, value, min, max);
    (bar, color)
}

/// Compute the color override for a meter based on the optimum
/// zone rules from the HTML spec. Returns `None` for the optimum
/// zone (cascade fg wins) so authors can colorize the optimum
/// state via author CSS without our zone logic stomping on them.
fn meter_zone_color(dom: &TuiDom, id: NodeId, value: f64, min: f64, max: f64) -> Option<Color> {
    let low = parse_attr(dom, id, "low").unwrap_or(min);
    let high = parse_attr(dom, id, "high").unwrap_or(max);
    let optimum = parse_attr(dom, id, "optimum").unwrap_or((min + max) / 2.0);

    // Three named zones (per HTML living standard):
    //   - "optimum"        — same range as `optimum`
    //   - "suboptimal"     — adjacent to optimum
    //   - "even-less-good" — opposite extreme from optimum
    //
    // We collapse to a tri-state color: green / yellow / red.
    let value_zone = zone_of(value, low, high);
    let optimum_zone = zone_of(optimum, low, high);
    if value_zone == optimum_zone {
        None // optimum — let cascade fg show through
    } else if (value_zone as i8 - optimum_zone as i8).abs() == 1 {
        Some(Color::Rgb(255, 255, 0)) // suboptimal
    } else {
        Some(Color::Rgb(255, 0, 0)) // even-less-good
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Zone {
    Low = 0,
    Mid = 1,
    High = 2,
}

fn zone_of(v: f64, low: f64, high: f64) -> Zone {
    if v <= low {
        Zone::Low
    } else if v >= high {
        Zone::High
    } else {
        Zone::Mid
    }
}

/// Build a horizontal bar of `width` cells, filled to `ratio`
/// (0.0..=1.0). Rounds fill to the nearest cell.
fn fill_bar(width: u16, ratio: f64) -> String {
    let total = width as usize;
    let filled = (ratio * total as f64).round() as usize;
    let filled = filled.min(total);
    let empty = total - filled;
    let mut s = String::with_capacity(total * 3);
    for _ in 0..filled {
        s.push_str(FILL);
    }
    for _ in 0..empty {
        s.push_str(EMPTY);
    }
    s
}

/// Indeterminate progress: render the entire track as the empty
/// shade. Browsers animate; v1 stays static — `:indeterminate`
/// pseudo-class + animation is a polish item.
fn indeterminate_bar(width: u16) -> String {
    EMPTY.repeat(width as usize)
}

fn parse_attr(dom: &TuiDom, id: NodeId, name: &str) -> Option<f64> {
    dom.node(id)
        .get_attribute(name)
        .and_then(|s| s.parse().ok())
}

/// Apply the gauge's color override (if any) on top of the
/// cascaded ComputedStyle. Returns the original style untouched
/// when `color` is `None`. Used by paint to thread the meter zone
/// color through `style_from_computed`'s output without rebuilding
/// the cascade.
pub fn override_fg(base: ComputedStyle, color: Option<Color>) -> ComputedStyle {
    let mut out = base;
    if let Some(c) = color {
        out.fg = c;
    }
    out
}

#[cfg(test)]
mod tests;
