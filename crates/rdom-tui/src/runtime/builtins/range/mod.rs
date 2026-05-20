//! `<input type="range">` — native HTML slider.
//!
//! ## Markup
//!
//! Plain `<input type="range">` (also an implicit-focusable input
//! from Phase C.1). Attributes:
//!
//! | Attribute | Default | Meaning |
//! |---|---|---|
//! | `min` | `0` | Lower bound |
//! | `max` | `100` | Upper bound |
//! | `value` | midpoint | Current position |
//! | `step` | `1` | Keyboard step increment |
//! | `disabled` | — | Blocks input |
//!
//! ## Paint
//!
//! [`install`] wires a canvas paint callback for every range input
//! in the DOM at construction; [`attach`] / [`attach_all`] cover
//! dynamically-added inputs. The paint draws a horizontal track
//! (`─`) with the thumb (`●`) at `(value - min) / (max - min)`
//! of the available width. Colors come from cascaded fg — UA
//! defaults ship via the `input[type=range]` rule in
//! [`Stylesheet::new()`](rdom_style::Stylesheet::new).
//!
//! ## Keyboard
//!
//! [`install`] also wires a global keydown handler:
//!
//! - `Right` / `Up` — increase by `step`
//! - `Left` / `Down` — decrease by `step`
//! - `Home` — set to `min`
//! - `End` — set to `max`
//! - `PageUp` / `PageDown` — ± `10 × step`
//!
//! Fires `input` and `change` on value change.
//!
//! ## v1 deliberate simplifications
//!
//! - No `<datalist>` tick-mark integration.
//! - No vertical orientation (`writing-mode: vertical-lr`).
//! - No mouse drag — click-to-set + keyboard only. Drag requires
//!   pointer-capture bookkeeping we defer to polish.
//! - `step="any"` treated as step=1 for keyboard navigation;
//!   mouse-click-set lands on the nearest cell boundary.

use rdom_core::{Dom, ListenerOptions, NodeId};

use crate::ext::TuiExt;
use crate::render::Style;
use crate::runtime::builtins::canvas;
use crate::style::Color;
use crate::tui_event::TuiDispatchExt;
use crate::{TuiDom, TuiEvent};

/// Glyph painted along the full width of the range track.
const TRACK_GLYPH: &str = "\u{2500}"; // ─
/// Glyph painted at the thumb position.
const THUMB_GLYPH: &str = "\u{25CF}"; // ●

// ── Install ───────────────────────────────────────────────────────

/// Install the global keyboard handler for range inputs. Called
/// once from `App::build`.
pub fn install(dom: &mut TuiDom) {
    let root = dom.root();
    dom.add_event_listener(root, "keydown", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        let Some(focused) = ctx.dom.focused() else {
            return;
        };
        if !is_range(ctx.dom, focused) {
            return;
        }
        if ctx.dom.node(focused).has_attribute("disabled") {
            return;
        }
        let Some(key) = ctx.event.detail.as_keyboard() else {
            return;
        };
        if key.modifiers.ctrl || key.modifiers.meta || key.modifiers.alt {
            return;
        }
        let step = step_of(ctx.dom, focused);
        let (min, max) = range_of(ctx.dom, focused);
        let current = value_of(ctx.dom, focused);
        let new = match key.key.as_str() {
            "ArrowRight" | "ArrowUp" => current + step,
            "ArrowLeft" | "ArrowDown" => current - step,
            "PageUp" => current + step * 10.0,
            "PageDown" => current - step * 10.0,
            "Home" => min,
            "End" => max,
            _ => return,
        };
        set_value(ctx.dom, focused, new.clamp(min, max));
    })
    .expect("range keydown install");
}

/// Attach a canvas paint callback to every `<input type="range">`
/// currently in the DOM. `App::build` calls this once; apps that
/// add ranges dynamically call [`attach`] for each new instance.
pub fn attach_all(dom: &mut TuiDom) {
    let ranges = collect_ranges(dom);
    for id in ranges {
        attach(dom, id);
    }
}

/// Attach a canvas paint callback to a specific `<input
/// type="range">`. No-op on anything else.
pub fn attach(dom: &mut TuiDom, input: NodeId) {
    if !is_range(dom, input) {
        return;
    }
    canvas::set_paint(dom, input, move |dom, ctx| {
        paint_track(dom, input, ctx);
    });
}

// ── Public read API ───────────────────────────────────────────────

pub fn value_of(dom: &TuiDom, input: NodeId) -> f64 {
    parse_attr::<f64>(dom, input, "value").unwrap_or_else(|| {
        let (min, max) = range_of(dom, input);
        if max < min {
            min
        } else {
            min + (max - min) / 2.0
        }
    })
}

pub fn range_of(dom: &TuiDom, input: NodeId) -> (f64, f64) {
    let min = parse_attr::<f64>(dom, input, "min").unwrap_or(0.0);
    let max = parse_attr::<f64>(dom, input, "max").unwrap_or(100.0);
    (min, max)
}

pub fn step_of(dom: &TuiDom, input: NodeId) -> f64 {
    match dom.node(input).get_attribute("step") {
        Some("any") | None => 1.0,
        Some(v) => v.parse::<f64>().ok().filter(|n| *n > 0.0).unwrap_or(1.0),
    }
}

/// Programmatic value setter — clamps to `[min, max]`, updates
/// the `value` attribute, fires `input` + `change`. No-op when
/// the value didn't actually change.
pub fn set_value(dom: &mut TuiDom, input: NodeId, v: f64) {
    let (min, max) = range_of(dom, input);
    let clamped = v.clamp(min, max);
    let current = value_of(dom, input);
    if (clamped - current).abs() < f64::EPSILON {
        return;
    }
    let _ = dom.set_attribute(input, "value", &format_number(clamped));
    // Range slider value updates are UI affordances, not text
    // entry. Use InsertReplacementText + data: null per the DOM
    // convention for synthetic value-update events.
    let mut input_ev = TuiEvent::input(rdom_core::InputType::InsertReplacementText, None);
    let _ = dom.dispatch_tui_event(input, &mut input_ev);
    let mut change_ev = TuiEvent::new("change");
    let _ = dom.dispatch_tui_event(input, &mut change_ev);
}

// ── Paint ─────────────────────────────────────────────────────────

fn paint_track(dom: &Dom<TuiExt>, input: NodeId, ctx: &mut canvas::RenderContext<'_>) {
    let style = style_from_dom(dom, input);
    let w = ctx.width();
    if w == 0 || ctx.height() == 0 {
        return;
    }
    for x in 0..w {
        ctx.set(x, 0, TRACK_GLYPH.chars().next().unwrap(), style);
    }
    let (min, max) = range_of_generic(dom, input);
    let value = value_of_generic(dom, input);
    let ratio = if max <= min {
        0.0
    } else {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    };
    let thumb_x = (ratio * (w as f64 - 1.0)).round() as u16;
    ctx.set(
        thumb_x.min(w - 1),
        0,
        THUMB_GLYPH.chars().next().unwrap(),
        style,
    );
}

fn style_from_dom(dom: &Dom<TuiExt>, id: NodeId) -> Style {
    let fg = dom
        .node(id)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .map(|c| c.fg)
        .unwrap_or(Color::Reset);
    if fg == Color::Reset {
        Style::new()
    } else {
        Style::new().fg(fg)
    }
}

// ── Helpers ───────────────────────────────────────────────────────

fn is_range(dom: &TuiDom, id: NodeId) -> bool {
    dom.node(id).tag_name() == Some("input") && dom.node(id).get_attribute("type") == Some("range")
}

fn collect_ranges(dom: &TuiDom) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk(dom, dom.root(), &mut out);
    out
}

fn walk(dom: &TuiDom, id: NodeId, out: &mut Vec<NodeId>) {
    if is_range(dom, id) {
        out.push(id);
    }
    for child in dom.node(id).child_nodes() {
        walk(dom, child.id(), out);
    }
}

fn parse_attr<T: std::str::FromStr>(dom: &TuiDom, id: NodeId, name: &str) -> Option<T> {
    dom.node(id)
        .get_attribute(name)
        .and_then(|s| s.parse().ok())
}

// Paint-time variants operate on the generic `Dom<TuiExt>` that
// the canvas callback receives, not just `TuiDom` — these re-
// implement the attribute parsing against the lower-level API.
fn value_of_generic(dom: &Dom<TuiExt>, id: NodeId) -> f64 {
    dom.node(id)
        .get_attribute("value")
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            let (min, max) = range_of_generic(dom, id);
            if max < min {
                min
            } else {
                min + (max - min) / 2.0
            }
        })
}

fn range_of_generic(dom: &Dom<TuiExt>, id: NodeId) -> (f64, f64) {
    let min = dom
        .node(id)
        .get_attribute("min")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    let max = dom
        .node(id)
        .get_attribute("max")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100.0);
    (min, max)
}

fn format_number(n: f64) -> String {
    if n == n.trunc() && n.is_finite() && n.abs() < 1e16 {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

#[cfg(test)]
mod tests;
