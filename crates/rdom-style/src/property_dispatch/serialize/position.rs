//! `serialize` for positioning (`position`, the insets, `z-index`), the
//! transitions and the counters.

use super::super::value_serializers::{
    join_csv, serialize_counter_ops, serialize_length, serialize_timing_function,
    serialize_transition_behavior, serialize_transition_property, serialize_transition_shorthand,
    specified,
};
use crate::TuiStyle;
use crate::layout::{Position, ZIndex};

/// `name`'s serialization when it is one of this family's properties —
/// `Some(None)` when it is not set — else `None`.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let out = match name {
        "position" => style.position.as_ref().and_then(specified).map(|p| {
            match p {
                Position::Static => "static",
                Position::Relative => "relative",
                Position::Absolute => "absolute",
                Position::Fixed => "fixed",
                Position::Sticky => "sticky",
            }
            .to_string()
        }),
        "top" => style.top.as_ref().and_then(specified).map(serialize_length),
        "right" => style
            .right
            .as_ref()
            .and_then(specified)
            .map(serialize_length),
        "bottom" => style
            .bottom
            .as_ref()
            .and_then(specified)
            .map(serialize_length),
        "left" => style
            .left
            .as_ref()
            .and_then(specified)
            .map(serialize_length),
        "z-index" => style.z_index.as_ref().and_then(specified).map(|z| match z {
            ZIndex::Auto => "auto".to_string(),
            ZIndex::Value(n) => n.to_string(),
        }),
        "overlay" => style.overlay.as_ref().and_then(specified).map(|o| {
            match o {
                crate::layout::Overlay::None => "none",
                crate::layout::Overlay::Auto => "auto",
            }
            .to_string()
        }),
        // `inset` shorthand emits whenever all four sides agree on
        // some Specified Length. (CSS L1 only allows agreement;
        // mismatched values need the longhands.)
        "inset" => match (
            style.top.as_ref().and_then(specified),
            style.right.as_ref().and_then(specified),
            style.bottom.as_ref().and_then(specified),
            style.left.as_ref().and_then(specified),
        ) {
            (Some(t), Some(r), Some(b), Some(l)) => Some(format!(
                "{} {} {} {}",
                serialize_length(t),
                serialize_length(r),
                serialize_length(b),
                serialize_length(l),
            )),
            _ => None,
        },

        // Transitions (M3)
        "transition-property" => style
            .transition_property
            .as_ref()
            .and_then(specified)
            .map(|list| join_csv(list.iter(), serialize_transition_property)),
        "transition-duration" => style
            .transition_duration
            .as_ref()
            .and_then(specified)
            .map(|list| join_csv(list.iter(), |ms| format!("{ms}ms"))),
        "transition-timing-function" => style
            .transition_timing_function
            .as_ref()
            .and_then(specified)
            .map(|list| join_csv(list.iter(), serialize_timing_function)),
        "transition-delay" => style
            .transition_delay
            .as_ref()
            .and_then(specified)
            .map(|list| join_csv(list.iter(), |ms| format!("{ms}ms"))),
        "transition-behavior" => {
            style
                .transition_behavior
                .as_ref()
                .and_then(specified)
                .map(|list| {
                    join_csv(list.iter(), |b| {
                        serialize_transition_behavior(*b).to_string()
                    })
                })
        }
        "transition" => serialize_transition_shorthand(style),
        "counter-reset" => style
            .counter_reset
            .as_ref()
            .and_then(specified)
            .map(|ops| serialize_counter_ops(ops)),
        "counter-increment" => style
            .counter_increment
            .as_ref()
            .and_then(specified)
            .map(|ops| serialize_counter_ops(ops)),
        "counter-set" => style
            .counter_set
            .as_ref()
            .and_then(specified)
            .map(|ops| serialize_counter_ops(ops)),

        _ => return None,
    };
    Some(out)
}
