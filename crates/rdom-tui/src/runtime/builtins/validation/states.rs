//! The validity states of one control (HTML §4.10.5.3 / §4.10.7 /
//! §4.10.11 "Constraint validation" paragraphs), from its value and
//! attributes.

use rdom_core::{InputTypeState, NodeId};

use super::ValidityState;
use super::syntax::{is_valid_absolute_url, is_valid_email, parse_float, parse_non_negative};
use crate::TuiDom;
use crate::runtime::builtins::{input, select, toggle};

pub(super) fn compute(dom: &TuiDom, id: NodeId) -> ValidityState {
    let mut s = ValidityState {
        custom_error: dom
            .node(id)
            .ext()
            .is_some_and(|e| !e.custom_validity.is_empty()),
        ..ValidityState::default()
    };
    match dom.node(id).tag_name() {
        Some("input") => input_states(dom, id, &mut s),
        Some("textarea") => {
            let value = dom.node(id).text_content();
            s.value_missing = required(dom, id) && value.is_empty();
            length_states(dom, id, &value, &mut s);
        }
        Some("select") => s.value_missing = required(dom, id) && select_missing(dom, id),
        _ => {}
    }
    s
}

fn required(dom: &TuiDom, id: NodeId) -> bool {
    dom.node(id).has_attribute("required")
}

fn input_states(dom: &TuiDom, id: NodeId, s: &mut ValidityState) {
    use InputTypeState as T;
    let Some(state) = dom.input_type_state(id) else {
        return;
    };
    match state {
        T::Checkbox => {
            s.value_missing = required(dom, id) && !dom.node(id).has_attribute("checked")
        }
        T::Radio => s.value_missing = radio_group_missing(dom, id),
        // `required` does not apply; a range is sanitized into range and
        // onto a step, so it suffers none of the value states.
        T::Hidden | T::Range | T::Color | T::Submit | T::Image | T::Reset | T::Button => {}
        _ => {
            let value = input::value(dom, id);
            s.value_missing = required(dom, id) && value.is_empty();
            if value.is_empty() {
                return;
            }
            let multiple = dom.node(id).has_attribute("multiple");
            match state {
                T::Email => {
                    s.type_mismatch = !email_values(&value, multiple)
                        .into_iter()
                        .all(is_valid_email)
                }
                T::Url => s.type_mismatch = !is_valid_absolute_url(&value),
                T::Number => number_states(dom, id, &value, s),
                _ => {}
            }
            if matches!(
                state,
                T::Text | T::Search | T::Url | T::Tel | T::Email | T::Password
            ) {
                let values: Vec<&str> = if state == T::Email {
                    email_values(&value, multiple)
                } else {
                    vec![value.as_str()]
                };
                s.pattern_mismatch = super::pattern::mismatch(dom, id, &values);
                length_states(dom, id, &value, s);
            }
        }
    }
}

/// An email input's value(s): the whole value, or the comma-separated
/// list of a `multiple` input — each stripped of ASCII whitespace, as
/// HTML's value sanitization leaves them.
fn email_values(value: &str, multiple: bool) -> Vec<&str> {
    if multiple {
        value.split(',').map(str::trim_ascii).collect()
    } else {
        vec![value.trim_ascii()]
    }
}

/// HTML §4.10.5.1.18: any member of the radio group is required and no
/// member is checked. rdom's radio group is the radios sharing a
/// non-empty `name` across the document (`toggle::radio_group`).
fn radio_group_missing(dom: &TuiDom, id: NodeId) -> bool {
    let group = toggle::radio_group(dom, id);
    group.iter().any(|&r| required(dom, r))
        && !group.iter().any(|&r| dom.node(r).has_attribute("checked"))
}

/// HTML §4.10.7: no option is selected, or the only selected one is the
/// placeholder label option.
fn select_missing(dom: &TuiDom, id: NodeId) -> bool {
    let selected = select::selected_options(dom, id);
    let placeholder = placeholder_label_option(dom, id);
    selected.iter().all(|&o| Some(o) == placeholder)
}

/// HTML §4.10.7 "placeholder label option": for a single-select with a
/// display size of 1, its first option when that option's value is
/// `""` and its parent is the select itself (not an `<optgroup>`).
fn placeholder_label_option(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    if select::is_multi(dom, id) || select::display_size(dom, id) != 1 {
        return None;
    }
    let first = *select::options(dom, id).first()?;
    (select::option_value(dom, first).is_empty()
        && dom.node(first).parent_node().map(|p| p.id()) == Some(id))
    .then_some(first)
}

/// `maxlength` / `minlength` (HTML §4.10.5.3.1, §4.10.11): only a value
/// the user last edited (the dirty value flag) is constrained; length
/// is in UTF-16 code units; an empty value is never too short.
fn length_states(dom: &TuiDom, id: NodeId, value: &str, s: &mut ValidityState) {
    if !dom.node(id).ext().is_some_and(|e| e.value_user_edited) {
        return;
    }
    let len = value.encode_utf16().count();
    let limit = |name| {
        dom.node(id)
            .get_attribute(name)
            .and_then(parse_non_negative)
    };
    if let Some(max) = limit("maxlength") {
        s.too_long = len > max;
    }
    if let Some(min) = limit("minlength") {
        s.too_short = len > 0 && len < min;
    }
}

/// `type=number`: bad input, `min` / `max`, `step`.
fn number_states(dom: &TuiDom, id: NodeId, value: &str, s: &mut ValidityState) {
    let Some(v) = parse_float(value) else {
        s.bad_input = true;
        return;
    };
    let bounds = NumberBounds::of(dom, id);
    if let Some(min) = bounds.min {
        s.range_underflow = v < min;
    }
    if let Some(max) = bounds.max {
        s.range_overflow = v > max;
    }
    if let Some(step) = bounds.step {
        s.step_mismatch = !on_step(v, bounds.base, step);
    }
}

/// The numeric constraints of a `type=number` input.
pub(super) struct NumberBounds {
    pub(super) min: Option<f64>,
    pub(super) max: Option<f64>,
    /// `None` for `step="any"`.
    pub(super) step: Option<f64>,
    /// HTML §4.10.5.4 "step base": `min`, else the `value` content
    /// attribute (rdom mirrors edits into it, so the recorded
    /// `defaultValue` stands in), else 0.
    pub(super) base: f64,
}

impl NumberBounds {
    pub(super) fn of(dom: &TuiDom, id: NodeId) -> Self {
        let attr = |name| dom.node(id).get_attribute(name).and_then(parse_float);
        let min = attr("min");
        let step = match dom.node(id).get_attribute("step") {
            Some(s) if s.eq_ignore_ascii_case("any") => None,
            // Invalid, zero or negative → the default step, 1.
            s => Some(s.and_then(parse_float).filter(|n| *n > 0.0).unwrap_or(1.0)),
        };
        let default_value = || {
            use crate::accessors::TuiAccessors;
            dom.node(id)
                .default_value()
                .as_deref()
                .and_then(parse_float)
        };
        Self {
            min,
            max: attr("max"),
            step,
            base: min.or_else(default_value).unwrap_or(0.0),
        }
    }
}

/// Whether `v` is `base` plus an integral number of `step`s, tolerating
/// float noise (`1.3` is on a `0.1` step from `1`).
pub(super) fn on_step(v: f64, base: f64, step: f64) -> bool {
    let q = (v - base) / step;
    (q - q.round()).abs() <= 1e-9 * q.abs().max(1.0)
}
