//! The validity states of one control (HTML §4.10.5.3 / §4.10.7 /
//! §4.10.11 "Constraint validation" paragraphs), from its value and
//! attributes.

use rdom_core::{InputTypeState, NodeId};

use super::ValidityState;
use super::dates;
use super::syntax::{is_valid_absolute_url, is_valid_email, parse_float, parse_non_negative};
use crate::TuiDom;
use crate::runtime::builtins::{input, select};

/// Radio-group verdicts memoized over one batch of [`compute_in`]
/// calls: the first radio of a group computes whether the group is
/// missing a value (one walk of its tree, `Dom::radio_group`) and every
/// member reuses it, so a batch walks once per group, not per radio.
#[derive(Debug, Default)]
pub(super) struct RadioGroups {
    missing: std::collections::HashMap<NodeId, bool>,
    /// Per radio: its group has no checked member (`:indeterminate`).
    unchecked: std::collections::HashMap<NodeId, bool>,
}

impl RadioGroups {
    fn missing(&mut self, dom: &TuiDom, id: NodeId) -> bool {
        if let Some(&m) = self.missing.get(&id) {
            return m;
        }
        let group = dom.radio_group(id);
        let m = group_missing(dom, &group);
        for r in group {
            self.missing.insert(r, m);
        }
        m
    }

    /// Whether `id`'s radio group has no checked member — what
    /// `:indeterminate` matches on a radio (HTML §4.16.3).
    pub(super) fn unchecked(&mut self, dom: &TuiDom, id: NodeId) -> bool {
        if let Some(&u) = self.unchecked.get(&id) {
            return u;
        }
        let group = dom.radio_group(id);
        let u = !group.iter().any(|&r| dom.node(r).has_attribute("checked"));
        for r in group {
            self.unchecked.insert(r, u);
        }
        u
    }

    /// Forget the verdicts (the tree may have changed).
    pub(super) fn clear(&mut self) {
        self.missing.clear();
        self.unchecked.clear();
    }
}

pub(super) fn compute(dom: &TuiDom, id: NodeId) -> ValidityState {
    compute_with(dom, id, None)
}

/// [`compute`], with radio-group verdicts from (and into) `groups`.
pub(super) fn compute_in(dom: &TuiDom, id: NodeId, groups: &mut RadioGroups) -> ValidityState {
    compute_with(dom, id, Some(groups))
}

fn compute_with(dom: &TuiDom, id: NodeId, groups: Option<&mut RadioGroups>) -> ValidityState {
    let mut s = ValidityState {
        custom_error: dom.node(id).ext().is_some_and(|e| {
            e.form_state
                .get()
                .is_some_and(|f| !f.custom_validity.is_empty())
        }),
        ..ValidityState::default()
    };
    match dom.node(id).tag_name() {
        Some("input") => input_states(dom, id, &mut s, groups),
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

fn input_states(dom: &TuiDom, id: NodeId, s: &mut ValidityState, groups: Option<&mut RadioGroups>) {
    use InputTypeState as T;
    let Some(state) = dom.input_type_state(id) else {
        return;
    };
    match state {
        T::Checkbox => {
            s.value_missing = required(dom, id) && !dom.node(id).has_attribute("checked")
        }
        T::Radio => {
            s.value_missing = match groups {
                Some(groups) => groups.missing(dom, id),
                None => group_missing(dom, &dom.radio_group(id)),
            }
        }
        // `required` does not apply (`Dom::required_applies`: hidden,
        // range, color, the buttons), nor does any other value state — a
        // range is sanitized into range and onto a step.
        _ if !dom.required_applies(id) => {}
        // HTML §4.10.5.1.7–§4.10.5.1.11: the value is the `value`
        // attribute rdom keeps live (these states are not edited as
        // text), sanitized to `""` unless it parses.
        _ if dates::is_date_like(state) => {
            let value = dom.node(id).get_attribute("value").unwrap_or("");
            let parsed = dates::parse(state, value);
            s.value_missing = required(dom, id) && parsed.is_none();
            if let Some(v) = parsed {
                date_states(dom, id, state, v, s);
            }
        }
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
/// member is checked. The group is HTML's (same tree, form owner and
/// `name`; `Dom::radio_group`).
fn group_missing(dom: &TuiDom, group: &[NodeId]) -> bool {
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
    if !dom
        .node(id)
        .ext()
        .is_some_and(|e| e.form_state.get().is_some_and(|f| f.value_user_edited))
    {
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

/// HTML §4.10.5.3.7: a date-like value before `min` suffers from an
/// underflow, after `max` from an overflow — unless `min` / `max` form a
/// *reversed range* (a time whose `max` is before its `min`, a period
/// across midnight), where a value after `max` and before `min` suffers
/// from both. An unparsable `min` / `max` is none.
fn date_states(dom: &TuiDom, id: NodeId, state: InputTypeState, v: f64, s: &mut ValidityState) {
    let limit = |name| {
        dom.node(id)
            .get_attribute(name)
            .and_then(|a| dates::parse(state, a))
    };
    let (min, max) = (limit("min"), limit("max"));
    match (min, max) {
        (Some(lo), Some(hi)) if state == InputTypeState::Time && hi < lo => {
            let outside = v > hi && v < lo;
            s.range_underflow = outside;
            s.range_overflow = outside;
        }
        _ => {
            s.range_underflow = min.is_some_and(|lo| v < lo);
            s.range_overflow = max.is_some_and(|hi| v > hi);
        }
    }
}

/// HTML §4.16.3 "has range limitations": a number or date-like input
/// with a `min` or `max` its state parses, and every range input (it has
/// a default minimum and maximum).
pub(super) fn range_limited(dom: &TuiDom, id: NodeId) -> bool {
    let Some(state) = dom.input_type_state(id) else {
        return false;
    };
    let parses = |name| {
        dom.node(id)
            .get_attribute(name)
            .is_some_and(|a| match state {
                InputTypeState::Number => parse_float(a).is_some(),
                _ => dates::parse(state, a).is_some(),
            })
    };
    match state {
        InputTypeState::Range => true,
        InputTypeState::Number => parses("min") || parses("max"),
        _ if dates::is_date_like(state) => parses("min") || parses("max"),
        _ => false,
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
