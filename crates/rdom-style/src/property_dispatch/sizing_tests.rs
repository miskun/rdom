//! Dispatch tests for box sizing (CSS-COMPLETE Phase 5): `box-sizing`.

use super::*;
use crate::layout::BoxSizing;
use crate::{ComputedStyle, TuiStyle, Value};

/// CSS UI 3 §3.1 (now CSS Sizing 3, "Box Edges for Sizing"):
/// `box-sizing: content-box | border-box`, ASCII case-insensitive,
/// serialized as the keyword; anything else is invalid.
#[test]
fn box_sizing_takes_its_two_keywords() {
    for (css, kw) in [
        ("content-box", BoxSizing::ContentBox),
        ("BORDER-BOX", BoxSizing::BorderBox),
    ] {
        let mut style = TuiStyle::new();
        set("box-sizing", css, &mut style).unwrap();
        assert_eq!(style.box_sizing, Some(Value::Specified(kw)));
        assert_eq!(
            serialize("box-sizing", &style).as_deref(),
            Some(css.to_ascii_lowercase().as_str())
        );
    }
    for bad in ["padding-box", "auto", "border-box content-box", ""] {
        let mut style = TuiStyle::new();
        assert_eq!(
            set("box-sizing", bad, &mut style),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
}

/// CSS UI 3 §3.1: initial `content-box`, not inherited — so `unset`
/// is `initial`.
#[test]
fn box_sizing_is_content_box_initially_and_not_inherited() {
    assert_eq!(ComputedStyle::initial().box_sizing, BoxSizing::ContentBox);
    assert_eq!(BoxSizing::default(), BoxSizing::ContentBox);
    assert!(!inherits("box-sizing"));
    let mut style = TuiStyle::new();
    set("box-sizing", "unset", &mut style).unwrap();
    assert_eq!(style.box_sizing, Some(Value::Initial));
}

/// The builder setter writes the same field as the declaration, and
/// `!important` routes through the field's own bit.
#[test]
fn box_sizing_builder_matches_the_declaration() {
    let mut declared = TuiStyle::new();
    set("box-sizing", "border-box", &mut declared).unwrap();
    assert_eq!(TuiStyle::new().box_sizing(BoxSizing::BorderBox), declared);
    let important = TuiStyle::new().box_sizing_important(BoxSizing::BorderBox);
    assert!(
        important
            .important
            .contains(crate::ImportantMask::BOX_SIZING)
    );
    assert_eq!(
        property_mask("box-sizing"),
        Some(crate::ImportantMask::BOX_SIZING)
    );
}
