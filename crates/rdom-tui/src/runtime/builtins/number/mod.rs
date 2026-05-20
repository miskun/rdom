//! `<input type="number">` — numeric input filter + Up/Down
//! stepping.
//!
//! ## Contract (from MDN)
//!
//! - Accepts a valid floating-point number: digits, optional
//!   leading sign (`-`/`+`), optional one `.` for decimals.
//! - Rejects letters and other non-numeric characters at the
//!   character-insertion level. v1 filters per-keystroke via
//!   `beforeinput` — the inserted text must be entirely digits,
//!   sign, or a decimal point. Whether the resulting string is a
//!   well-formed number (e.g. only one decimal point, sign at the
//!   start) is a validation concern apps handle.
//! - Up / Down arrow on the focused input steps the value by `step`
//!   (default `1`), clamped to `min`/`max`. Both `input` and
//!   `change` events fire after stepping (non-cancelable).
//!
//! ## v1 deliberate simplifications
//!
//! - No scientific notation (`e2`, `1.5E10`) — MDN-faithful.
//! - `step="any"` is treated as `1` for stepping purposes (the
//!   spec lets `any` skip step validation entirely; v1 just uses
//!   the default step). Beforeinput filter still allows decimals.
//! - `min` / `max` clamping happens only during stepping, not
//!   during character entry — typing past the bound is allowed
//!   (matches browser behavior).

use rdom_core::{ListenerOptions, NodeId};

use crate::tui_event::TuiDispatchExt;
use crate::{TuiDom, TuiEvent};

/// Install the number-input default actions. Two root-level
/// listeners: `beforeinput` (filter non-numeric chars), keydown
/// (Up/Down stepping).
pub fn install(dom: &mut TuiDom) {
    let root = dom.root();

    // `beforeinput` filter — cancel inserts that contain non-
    // numeric characters when the editable target is a number
    // input. Pure deletes (empty `detail`) always pass.
    dom.add_event_listener(
        root,
        "beforeinput",
        ListenerOptions::default(),
        move |ctx| {
            if ctx.event.default_prevented() {
                return;
            }
            let Some(target) = ctx.event.target else {
                return;
            };
            if !is_number_input(ctx.dom, target) {
                return;
            }
            // beforeinput now carries typed Input detail; the
            // proposed text is in `data` (Some for inserts, None
            // for deletes / history). Deletions are always allowed.
            let Some(input) = ctx.event.detail.as_input() else {
                return;
            };
            let Some(data) = input.data.as_deref() else {
                return; // deletion or history op — allow
            };
            if data.is_empty() {
                return;
            }
            if !data.chars().all(is_numeric_char) {
                ctx.event.prevent_default();
            }
        },
    )
    .expect("number beforeinput filter install");

    // Up / Down arrow → step by `step` attribute. No modifiers.
    dom.add_event_listener(root, "keydown", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        let Some(focused) = ctx.dom.focused() else {
            return;
        };
        if !is_number_input(ctx.dom, focused) {
            return;
        }
        if ctx.dom.node(focused).has_attribute("disabled")
            || ctx.dom.node(focused).has_attribute("readonly")
        {
            return;
        }
        let Some(key) = ctx.event.detail.as_keyboard() else {
            return;
        };
        if key.modifiers.ctrl || key.modifiers.shift || key.modifiers.alt || key.modifiers.meta {
            return;
        }
        let direction = match key.key.as_str() {
            "ArrowUp" => 1.0,
            "ArrowDown" => -1.0,
            _ => return,
        };
        step(ctx.dom, focused, direction);
    })
    .expect("number arrow stepper install");
}

/// Apply one step in the given `direction` (`+1.0` for Up, `-1.0`
/// for Down). Reads `step`, `min`, `max` attributes; defaults to
/// step=1, no clamp. Writes the new value via `input::set_value`
/// (keeps the `value` attribute and text content in sync) and
/// fires `input` + `change`.
fn step(dom: &mut TuiDom, input: NodeId, direction: f64) {
    let current: f64 = crate::runtime::builtins::input::value(dom, input)
        .parse()
        .unwrap_or(0.0);
    let step_size = parse_step(dom.node(input).get_attribute("step"));
    let mut next = current + direction * step_size;
    if let Some(min) = parse_bound(dom.node(input).get_attribute("min")) {
        next = next.max(min);
    }
    if let Some(max) = parse_bound(dom.node(input).get_attribute("max")) {
        next = next.min(max);
    }
    let formatted = format_number(next);
    crate::runtime::builtins::input::set_value(dom, input, &formatted);

    // Stepper changes the value through a UI affordance, not text
    // entry; DOM convention is InsertReplacementText + data: null
    // for these synthetic value-update inputs (listeners read the
    // new value off the input's `value` attribute).
    let mut input_ev = TuiEvent::input(rdom_core::InputType::InsertReplacementText, None);
    let _ = dom.dispatch_tui_event(input, &mut input_ev);
    let mut change_ev = TuiEvent::new("change");
    let _ = dom.dispatch_tui_event(input, &mut change_ev);
}

// ── Helpers ────────────────────────────────────────────────────────

/// Numeric character whitelist: digits, sign, decimal point. The
/// validity of a particular position (e.g. sign mid-string) is
/// the app's concern — v1 errs on the side of permissive entry.
fn is_numeric_char(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '-' | '+' | '.')
}

fn is_number_input(dom: &TuiDom, id: NodeId) -> bool {
    dom.node(id).tag_name() == Some("input") && dom.node(id).get_attribute("type") == Some("number")
}

/// Parse `step` attribute into a positive f64. Defaults to `1.0`
/// when absent, malformed, or `"any"` (the spec's "no step
/// constraint" sentinel — v1 still steps by 1).
fn parse_step(s: Option<&str>) -> f64 {
    match s {
        None | Some("any") => 1.0,
        Some(v) => v.parse::<f64>().ok().filter(|n| *n > 0.0).unwrap_or(1.0),
    }
}

fn parse_bound(s: Option<&str>) -> Option<f64> {
    s.and_then(|v| v.parse::<f64>().ok())
}

/// Format an f64 the way HTML number inputs do: integer-looking
/// values display without a trailing `.0`, fractional values
/// preserve enough precision for the step. v1 uses Rust's default
/// f64 Display, which trims trailing zeros after the decimal.
fn format_number(n: f64) -> String {
    if n == n.trunc() && n.is_finite() && n.abs() < 1e16 {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

#[cfg(test)]
mod tests;
