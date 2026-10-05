//! The scrolling properties — `overscroll-behavior` and its longhands
//! (CSS Overscroll Behavior 1 §3), `scroll-padding` and `scroll-margin`
//! with their longhands (CSS Scroll Snap 1 §4): their `set` and
//! `serialize` arms. The flow-relative longhands are `logical`'s.

use super::value_serializers::{serialize_padding_value, specified};
use crate::layout::ScrollPadding;
use crate::layout::{ScrollSnapStop, ScrollSnapStrictness, ScrollSnapType};
use crate::parse::token::Token;
use crate::parse::values::{
    parse_overscroll_behavior, parse_overscroll_behavior_shorthand, parse_scroll_margin,
    parse_scroll_margin_shorthand, parse_scroll_padding, parse_scroll_padding_shorthand,
    parse_scroll_snap_align, parse_scroll_snap_stop, parse_scroll_snap_type,
};
use crate::{TuiStyle, Value};

/// The `scroll-padding` side fields of `style`, top, right, bottom,
/// left.
fn padding_sides(style: &mut TuiStyle) -> [&mut Option<Value<ScrollPadding>>; 4] {
    [
        &mut style.scroll_padding.top,
        &mut style.scroll_padding.right,
        &mut style.scroll_padding.bottom,
        &mut style.scroll_padding.left,
    ]
}

/// The `scroll-margin` side fields of `style`, top, right, bottom, left.
fn margin_sides(style: &mut TuiStyle) -> [&mut Option<Value<i16>>; 4] {
    [
        &mut style.scroll_margin.top,
        &mut style.scroll_margin.right,
        &mut style.scroll_margin.bottom,
        &mut style.scroll_margin.left,
    ]
}

/// The side a `scroll-padding-*` / `scroll-margin-*` longhand names.
fn side_of(name: &str, prefix: &str) -> Option<usize> {
    ["top", "right", "bottom", "left"]
        .iter()
        .position(|s| name.strip_prefix(prefix) == Some(*s))
}

fn padding_text(p: &ScrollPadding) -> String {
    match p {
        ScrollPadding::Auto => "auto".to_string(),
        ScrollPadding::Length(v) => serialize_padding_value(v),
    }
}

/// The shortest one-to-four-value form of four sides (CSS Box 3's
/// order).
fn shortest(sides: [String; 4]) -> String {
    let [t, r, b, l] = sides;
    if r == l {
        if t == b {
            if t == r { t } else { format!("{t} {r}") }
        } else {
            format!("{t} {r} {b}")
        }
    } else {
        format!("{t} {r} {b} {l}")
    }
}

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    Some(match name {
        "overscroll-behavior" => parse_overscroll_behavior_shorthand(value).map(|(x, y)| {
            style.overscroll_behavior_x = Some(Value::Specified(x));
            style.overscroll_behavior_y = Some(Value::Specified(y));
        }),
        "overscroll-behavior-x" => parse_overscroll_behavior(value).map(|b| {
            style.overscroll_behavior_x = Some(Value::Specified(b));
        }),
        "overscroll-behavior-y" => parse_overscroll_behavior(value).map(|b| {
            style.overscroll_behavior_y = Some(Value::Specified(b));
        }),
        "scroll-snap-type" => parse_scroll_snap_type(value).map(|t| {
            style.scroll_snap_type = Some(Value::Specified(t));
        }),
        "scroll-snap-align" => parse_scroll_snap_align(value).map(|a| {
            style.scroll_snap_align = Some(Value::Specified(a));
        }),
        "scroll-snap-stop" => parse_scroll_snap_stop(value).map(|t| {
            style.scroll_snap_stop = Some(Value::Specified(t));
        }),
        "scroll-padding" => parse_scroll_padding_shorthand(value).map(|sides| {
            for (field, v) in padding_sides(style).into_iter().zip(sides) {
                *field = Some(Value::Specified(v));
            }
        }),
        "scroll-margin" => parse_scroll_margin_shorthand(value).map(|sides| {
            for (field, v) in margin_sides(style).into_iter().zip(sides) {
                *field = Some(Value::Specified(v));
            }
        }),
        _ => {
            if let Some(i) = side_of(name, "scroll-padding-") {
                parse_scroll_padding(value).map(|v| {
                    let [a, b, c, d] = padding_sides(style);
                    *[a, b, c, d].into_iter().nth(i).expect("four sides") =
                        Some(Value::Specified(v));
                })
            } else if let Some(i) = side_of(name, "scroll-margin-") {
                parse_scroll_margin(value).map(|v| {
                    let [a, b, c, d] = margin_sides(style);
                    *[a, b, c, d].into_iter().nth(i).expect("four sides") =
                        Some(Value::Specified(v));
                })
            } else {
                return None;
            }
        }
    })
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let x = style.overscroll_behavior_x.as_ref().and_then(specified);
    let y = style.overscroll_behavior_y.as_ref().and_then(specified);
    Some(match name {
        "overscroll-behavior-x" => x.map(|b| b.keyword().to_string()),
        "overscroll-behavior-y" => y.map(|b| b.keyword().to_string()),
        // One value when the axes match, else `x y`.
        "overscroll-behavior" => match (x, y) {
            (Some(x), Some(y)) if x == y => Some(x.keyword().to_string()),
            (Some(x), Some(y)) => Some(format!("{} {}", x.keyword(), y.keyword())),
            _ => None,
        },
        "scroll-snap-type" => {
            style
                .scroll_snap_type
                .as_ref()
                .and_then(specified)
                .map(|t| match t {
                    ScrollSnapType::None => "none".to_string(),
                    ScrollSnapType::Snap(axis, ScrollSnapStrictness::Proximity) => {
                        axis.keyword().to_string()
                    }
                    ScrollSnapType::Snap(axis, ScrollSnapStrictness::Mandatory) => {
                        format!("{} mandatory", axis.keyword())
                    }
                })
        }
        "scroll-snap-align" => style
            .scroll_snap_align
            .as_ref()
            .and_then(specified)
            .map(|a| {
                if a.block == a.inline {
                    a.block.keyword().to_string()
                } else {
                    format!("{} {}", a.block.keyword(), a.inline.keyword())
                }
            }),
        "scroll-snap-stop" => {
            style
                .scroll_snap_stop
                .as_ref()
                .and_then(specified)
                .map(|t| match t {
                    ScrollSnapStop::Normal => "normal".to_string(),
                    ScrollSnapStop::Always => "always".to_string(),
                })
        }
        "scroll-padding" => {
            let sides = [
                &style.scroll_padding.top,
                &style.scroll_padding.right,
                &style.scroll_padding.bottom,
                &style.scroll_padding.left,
            ]
            .map(|f| f.as_ref().and_then(specified).map(padding_text));
            match sides {
                [Some(t), Some(r), Some(b), Some(l)] => Some(shortest([t, r, b, l])),
                _ => None,
            }
        }
        "scroll-margin" => {
            let sides = [
                &style.scroll_margin.top,
                &style.scroll_margin.right,
                &style.scroll_margin.bottom,
                &style.scroll_margin.left,
            ]
            .map(|f| f.as_ref().and_then(specified).map(i16::to_string));
            match sides {
                [Some(t), Some(r), Some(b), Some(l)] => Some(shortest([t, r, b, l])),
                _ => None,
            }
        }
        _ => {
            let padding = [
                &style.scroll_padding.top,
                &style.scroll_padding.right,
                &style.scroll_padding.bottom,
                &style.scroll_padding.left,
            ];
            let margin = [
                &style.scroll_margin.top,
                &style.scroll_margin.right,
                &style.scroll_margin.bottom,
                &style.scroll_margin.left,
            ];
            if let Some(i) = side_of(name, "scroll-padding-") {
                padding[i].as_ref().and_then(specified).map(padding_text)
            } else if let Some(i) = side_of(name, "scroll-margin-") {
                margin[i].as_ref().and_then(specified).map(i16::to_string)
            } else {
                return None;
            }
        }
    })
}
