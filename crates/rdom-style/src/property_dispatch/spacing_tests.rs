//! Dispatch tests for the box spacing longhands (C6-MARGIN-SIDES):
//! `margin-*` and `padding-*` are one field and one `!important` bit
//! per side (CSS Box 3 §3.2 / §4.2), set by their shorthands.

use super::*;
use crate::layout::{MarginValue, PaddingValue, Sides};
use crate::{ImportantMask, TuiStyle, Value};

fn spec<T>(v: T) -> Option<Value<T>> {
    Some(Value::Specified(v))
}

/// CSS Box 3 §3.2: a side longhand sets its side only; the others stay
/// undeclared.
#[test]
fn a_side_longhand_declares_its_side_only() {
    let mut style = TuiStyle::new();
    set("margin-left", "2", &mut style).unwrap();
    set("padding-top", "3", &mut style).unwrap();
    assert_eq!(
        style.margin,
        Sides::new(None, None, None, spec(MarginValue::Cells(2)))
    );
    assert_eq!(
        style.padding,
        Sides::new(spec(PaddingValue::Cells(3)), None, None, None)
    );
    assert_eq!(serialize("margin", &style), None);
    assert_eq!(serialize("margin-left", &style).as_deref(), Some("2"));
    assert_eq!(serialize("margin-top", &style), None);
}

/// CSS Cascade 4 §6.4: each side its own bit; the shorthand owns the
/// four.
#[test]
fn each_side_has_its_own_important_bit() {
    assert_eq!(
        property_mask("margin-left"),
        Some(ImportantMask::MARGIN_LEFT)
    );
    assert_eq!(
        property_mask("padding-top"),
        Some(ImportantMask::PADDING_TOP)
    );
    assert_eq!(
        property_mask("margin"),
        Some(
            ImportantMask::MARGIN_TOP
                | ImportantMask::MARGIN_RIGHT
                | ImportantMask::MARGIN_BOTTOM
                | ImportantMask::MARGIN_LEFT
        )
    );
    let mut style = TuiStyle::new();
    set("margin", "1", &mut style).unwrap();
    set_important("margin-top", true, &mut style);
    assert!(is_important("margin-top", &style));
    assert!(!is_important("margin-left", &style));
    assert!(!is_important("margin", &style));
}

/// CSSOM §6.7.2: the shorthand serializes when every side is set, in
/// its shortest form; a CSS-wide keyword on one side is that side's.
#[test]
fn shorthands_serialize_shortest_and_keywords_stay_per_side() {
    for (css, out) in [
        ("1", "1"),
        ("1 2", "1 2"),
        ("1 2 3", "1 2 3"),
        ("1 2 3 4", "1 2 3 4"),
        ("1 auto 1 auto", "1 auto"),
    ] {
        let mut style = TuiStyle::new();
        set("margin", css, &mut style).unwrap();
        assert_eq!(serialize("margin", &style).as_deref(), Some(out), "{css}");
    }
    let mut style = TuiStyle::new();
    set("padding", "1 2", &mut style).unwrap();
    set("padding-left", "inherit", &mut style).unwrap();
    assert_eq!(style.padding.left, Some(Value::Inherit));
    assert_eq!(style.padding.top, spec(PaddingValue::Cells(1)));
    assert_eq!(
        serialize("padding-left", &style).as_deref(),
        Some("inherit")
    );
    assert_eq!(serialize("padding", &style), None);
    assert!(remove("padding-left", &mut style));
    assert_eq!(style.padding.left, None);
    assert_eq!(style.padding.right, spec(PaddingValue::Cells(2)));
}
