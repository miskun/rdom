//! `validationMessage` text. HTML leaves the wording to the user agent;
//! rdom's messages are fixed English modeled on Chromium's, one per
//! state, checked in the order the table on
//! [`validation_message`](super::validation_message) lists.
//!
//! Character counts are UTF-16 code units, as the constraint is.

use rdom_core::{InputTypeState, NodeId};

use super::ValidityState;
use super::states::NumberBounds;
use crate::TuiDom;
use crate::accessors::TuiAccessors;

pub(super) fn message(dom: &TuiDom, id: NodeId, s: &ValidityState) -> String {
    let node = dom.node(id);
    let attr = |name| node.get_attribute(name).unwrap_or("").to_string();
    let len = || node.value().unwrap_or_default().encode_utf16().count();
    let state = dom.input_type_state(id);
    if s.custom_error {
        return node
            .ext()
            .map(|e| e.custom_validity.clone())
            .unwrap_or_default();
    }
    if s.value_missing {
        match (node.tag_name(), state) {
            (_, Some(InputTypeState::Checkbox)) => "Please check this box if you want to proceed.",
            (_, Some(InputTypeState::Radio)) => "Please select one of these options.",
            (Some("select"), _) => "Please select an item in the list.",
            _ => "Please fill out this field.",
        }
        .to_string()
    } else if s.type_mismatch {
        if state == Some(InputTypeState::Email) {
            "Please enter an email address.".to_string()
        } else {
            "Please enter a URL.".to_string()
        }
    } else if s.bad_input {
        "Please enter a number.".to_string()
    } else if s.pattern_mismatch {
        "Please match the requested format.".to_string()
    } else if s.too_long {
        format!(
            "Please shorten this text to {} characters or less (you are currently using {} characters).",
            attr("maxlength").trim(),
            len()
        )
    } else if s.too_short {
        format!(
            "Please lengthen this text to {} characters or more (you are currently using {} characters).",
            attr("minlength").trim(),
            len()
        )
    } else if s.range_underflow {
        format!("Value must be greater than or equal to {}.", attr("min"))
    } else if s.range_overflow {
        format!("Value must be less than or equal to {}.", attr("max"))
    } else if s.step_mismatch {
        step_message(dom, id)
    } else {
        String::new()
    }
}

/// The step-mismatch message with the two valid values around the
/// current one.
fn step_message(dom: &TuiDom, id: NodeId) -> String {
    let bounds = NumberBounds::of(dom, id);
    let value = dom
        .node(id)
        .value()
        .as_deref()
        .and_then(super::syntax::parse_float);
    match (value, bounds.step) {
        (Some(v), Some(step)) => {
            let low = bounds.base + ((v - bounds.base) / step).floor() * step;
            format!(
                "Please enter a valid value. The two nearest valid values are {} and {}.",
                format_number(low),
                format_number(low + step)
            )
        }
        _ => "Please enter a valid value.".to_string(),
    }
}

/// A step value for display: float noise rounded away, integers without
/// a fraction.
fn format_number(n: f64) -> String {
    let n = (n * 1e10).round() / 1e10;
    if n == n.trunc() && n.abs() < 1e16 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}
