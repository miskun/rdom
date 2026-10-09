//! The CSS Anchor Positioning 1 properties: `anchor-name`, `anchor-scope`,
//! `position-anchor`, `position-area`, `position-try-fallbacks`,
//! `position-try-order`, the `position-try` shorthand and
//! `position-visibility` — their `set` and `serialize` arms.

use super::value_serializers::specified;
use crate::layout::{AnchorScope, PositionAnchor, PositionArea, PositionVisibility, TryFallback};
use crate::parse::token::Token;
use crate::parse::values::{
    parse_anchor_name, parse_anchor_scope, parse_position_anchor, parse_position_area,
    parse_position_try, parse_position_try_fallbacks, parse_position_try_order,
    parse_position_visibility,
};
use crate::{TuiStyle, Value};

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    let a = &mut style.anchor;
    Some(match name {
        "anchor-name" => {
            parse_anchor_name(value).map(|v| a.anchor_name = Some(Value::Specified(v)))
        }
        "anchor-scope" => {
            parse_anchor_scope(value).map(|v| a.anchor_scope = Some(Value::Specified(v)))
        }
        "position-anchor" => {
            parse_position_anchor(value).map(|v| a.position_anchor = Some(Value::Specified(v)))
        }
        "position-area" => {
            parse_position_area(value).map(|v| a.position_area = Some(Value::Specified(v)))
        }
        "position-try-fallbacks" => parse_position_try_fallbacks(value)
            .map(|v| a.position_try_fallbacks = Some(Value::Specified(v))),
        "position-try-order" => parse_position_try_order(value)
            .map(|v| a.position_try_order = Some(Value::Specified(v))),
        "position-try" => parse_position_try(value).map(|(order, fallbacks)| {
            a.position_try_order = Some(Value::Specified(order));
            a.position_try_fallbacks = Some(Value::Specified(fallbacks));
        }),
        "position-visibility" => parse_position_visibility(value)
            .map(|v| a.position_visibility = Some(Value::Specified(v))),
        _ => return None,
    })
}

fn names_text(names: &[std::sync::Arc<str>]) -> String {
    names.iter().map(|n| &**n).collect::<Vec<_>>().join(", ")
}

/// A `<position-area>` as CSS text.
pub(super) fn area_text(area: &PositionArea) -> String {
    match area.keywords() {
        (a, Some(b)) => format!("{} {}", a.keyword(), b.keyword()),
        (a, None) => a.keyword().to_string(),
    }
}

fn fallbacks_text(list: &[TryFallback]) -> String {
    if list.is_empty() {
        return "none".to_string();
    }
    list.iter()
        .map(|f| match &f.area {
            Some(area) => area_text(area),
            None => f
                .name
                .iter()
                .map(|n| n.to_string())
                .chain(f.tactics.iter().map(|t| t.keyword().to_string()))
                .collect::<Vec<_>>()
                .join(" "),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn visibility_text(v: &PositionVisibility) -> String {
    let parts: Vec<&str> = [
        (v.anchors_valid, "anchors-valid"),
        (v.anchors_visible, "anchors-visible"),
        (v.no_overflow, "no-overflow"),
    ]
    .into_iter()
    .filter_map(|(on, k)| on.then_some(k))
    .collect();
    if parts.is_empty() {
        "always".to_string()
    } else {
        parts.join(" ")
    }
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let a = &style.anchor;
    let order = a.position_try_order.as_ref().and_then(specified);
    let fallbacks = a.position_try_fallbacks.as_ref().and_then(specified);
    Some(match name {
        "anchor-name" => a.anchor_name.as_ref().and_then(specified).map(|n| {
            if n.names().is_empty() {
                "none".to_string()
            } else {
                names_text(n.names())
            }
        }),
        "anchor-scope" => a
            .anchor_scope
            .as_ref()
            .and_then(specified)
            .map(|s| match s {
                AnchorScope::None => "none".to_string(),
                AnchorScope::All => "all".to_string(),
                AnchorScope::Names(names) => names_text(names),
            }),
        "position-anchor" => a
            .position_anchor
            .as_ref()
            .and_then(specified)
            .map(|p| match p {
                PositionAnchor::Auto => "auto".to_string(),
                PositionAnchor::None => "none".to_string(),
                PositionAnchor::Name(n) => n.to_string(),
            }),
        "position-area" => a
            .position_area
            .as_ref()
            .and_then(specified)
            .map(|p| p.as_ref().map_or("none".to_string(), area_text)),
        "position-try-fallbacks" => fallbacks.map(|f| fallbacks_text(f)),
        "position-try-order" => order.map(|o| o.keyword().to_string()),
        // The shortest form: the order when not `normal`, then the
        // fallbacks.
        "position-try" => match (order, fallbacks) {
            (Some(o), Some(f)) => Some(match o {
                crate::layout::PositionTryOrder::Normal => fallbacks_text(f),
                o => format!("{} {}", o.keyword(), fallbacks_text(f)),
            }),
            _ => None,
        },
        "position-visibility" => a
            .position_visibility
            .as_ref()
            .and_then(specified)
            .map(visibility_text),
        _ => return None,
    })
}
