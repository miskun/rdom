//! Dispatch tests for `float` and `clear` (C8-FLOAT; CSS 2.1 §9.5.1 /
//! §9.5.2, CSS Logical 1 §2.3: the flow-relative keywords).

use super::*;
use crate::layout::{Clear, Float};
use crate::{TuiStyle, Value};

fn float_of(style: &TuiStyle) -> Option<Float> {
    match &style.float {
        Some(Value::Specified(f)) => Some(*f),
        _ => None,
    }
}

fn clear_of(style: &TuiStyle) -> Option<Clear> {
    match &style.clear {
        Some(Value::Specified(c)) => Some(*c),
        _ => None,
    }
}

/// CSS 2.1 §9.5.1 with CSS Logical 1 §2.3: `float: left | right | none |
/// inline-start | inline-end`, ASCII case-insensitive, serialized as
/// written.
#[test]
fn float_takes_the_physical_and_flow_relative_keywords() {
    for (text, value) in [
        ("left", Float::Left),
        ("RIGHT", Float::Right),
        ("none", Float::None),
        ("inline-start", Float::InlineStart),
        ("inline-end", Float::InlineEnd),
    ] {
        let mut style = TuiStyle::new();
        set("float", text, &mut style).unwrap();
        assert_eq!(float_of(&style), Some(value), "{text}");
        assert_eq!(
            serialize("float", &style).as_deref(),
            Some(text.to_ascii_lowercase().as_str())
        );
    }
    for bad in ["center", "left right", "1", "top"] {
        assert_eq!(
            set("float", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
}

/// CSS 2.1 §9.5.2 with CSS Logical 1 §2.3: `clear: none | left | right |
/// both | inline-start | inline-end`.
#[test]
fn clear_takes_the_physical_and_flow_relative_keywords() {
    for (text, value) in [
        ("none", Clear::None),
        ("left", Clear::Left),
        ("right", Clear::Right),
        ("Both", Clear::Both),
        ("inline-start", Clear::InlineStart),
        ("inline-end", Clear::InlineEnd),
    ] {
        let mut style = TuiStyle::new();
        set("clear", text, &mut style).unwrap();
        assert_eq!(clear_of(&style), Some(value), "{text}");
        assert_eq!(
            serialize("clear", &style).as_deref(),
            Some(text.to_ascii_lowercase().as_str())
        );
    }
    for bad in ["all", "left right", "top"] {
        assert_eq!(
            set("clear", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
}

/// CSS Logical 1 §2.3: `inline-start` / `inline-end` resolve against the
/// containing block's `direction` — the left / right side under `ltr`,
/// the right / left under `rtl`.
#[test]
fn the_flow_relative_keywords_resolve_by_direction() {
    use crate::layout::FloatSide::{Left, Right};
    assert_eq!(Float::InlineStart.side(false), Some(Left));
    assert_eq!(Float::InlineStart.side(true), Some(Right));
    assert_eq!(Float::InlineEnd.side(true), Some(Left));
    assert_eq!(Float::Right.side(true), Some(Right));
    assert_eq!(Float::None.side(false), None);
    assert_eq!(Clear::InlineStart.sides(true), (false, true));
    assert_eq!(Clear::Both.sides(false), (true, true));
    assert_eq!(Clear::InlineEnd.sides(false), (false, true));
    assert_eq!(Clear::None.sides(true), (false, false));
}
