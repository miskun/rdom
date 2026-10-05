//! Dispatch tests for the scrolling properties: `overscroll-behavior`
//! (C8-OVERSCROLL; CSS Overscroll Behavior 1 §3).

use super::*;
use crate::TuiStyle;
use crate::layout::OverscrollBehavior;

fn axes(style: &TuiStyle) -> (Option<OverscrollBehavior>, Option<OverscrollBehavior>) {
    let get = |v: &Option<crate::Value<OverscrollBehavior>>| match v {
        Some(crate::Value::Specified(b)) => Some(*b),
        _ => None,
    };
    (
        get(&style.overscroll_behavior_x),
        get(&style.overscroll_behavior_y),
    )
}

/// §3: `overscroll-behavior: [contain | none | auto]{1,2}` — "the first
/// value specifies overscroll-behavior-x and the second
/// overscroll-behavior-y; if only one value is specified, the second
/// value defaults to the same value"; serialized as one value when both
/// match.
#[test]
fn the_shorthand_sets_both_axes() {
    let mut style = TuiStyle::new();
    set("overscroll-behavior", "contain none", &mut style).unwrap();
    assert_eq!(
        axes(&style),
        (
            Some(OverscrollBehavior::Contain),
            Some(OverscrollBehavior::None)
        )
    );
    assert_eq!(
        serialize("overscroll-behavior", &style).as_deref(),
        Some("contain none")
    );
    set("overscroll-behavior", "AUTO", &mut style).unwrap();
    assert_eq!(
        serialize("overscroll-behavior", &style).as_deref(),
        Some("auto")
    );
    for bad in ["contain none auto", "bounce", "1"] {
        assert_eq!(
            set("overscroll-behavior", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
}

/// §3: the physical longhands, and the flow-relative ones — in
/// `horizontal-tb` (rdom's only writing mode) the inline axis is `x`,
/// the block axis `y`, one storage.
#[test]
fn the_longhands_and_the_logical_longhands() {
    let mut style = TuiStyle::new();
    set("overscroll-behavior-x", "none", &mut style).unwrap();
    set("overscroll-behavior-block", "contain", &mut style).unwrap();
    assert_eq!(
        axes(&style),
        (
            Some(OverscrollBehavior::None),
            Some(OverscrollBehavior::Contain)
        )
    );
    set("overscroll-behavior-inline", "auto", &mut style).unwrap();
    assert_eq!(
        serialize("overscroll-behavior-x", &style).as_deref(),
        Some("auto")
    );
    assert_eq!(
        serialize("overscroll-behavior-y", &style).as_deref(),
        Some("contain")
    );
    assert!(!inherits("overscroll-behavior-y"));
}

/// Scroll Snap 1 §4.1: `scroll-padding: [auto | <length-percentage
/// [0,∞]>]{1,4}` — the box-model side order; `auto`, a percentage, no
/// negative value.
#[test]
fn scroll_padding_takes_one_to_four_sides() {
    let mut style = TuiStyle::new();
    set("scroll-padding", "1 auto 10% 2", &mut style).unwrap();
    for (side, text) in [
        ("scroll-padding-top", "1"),
        ("scroll-padding-right", "auto"),
        ("scroll-padding-bottom", "10%"),
        ("scroll-padding-left", "2"),
    ] {
        assert_eq!(serialize(side, &style).as_deref(), Some(text), "{side}");
    }
    assert_eq!(
        serialize("scroll-padding", &style).as_deref(),
        Some("1 auto 10% 2")
    );
    set("scroll-padding", "3", &mut style).unwrap();
    assert_eq!(serialize("scroll-padding", &style).as_deref(), Some("3"));
    for bad in ["-1", "1 2 3 4 5", "none"] {
        assert_eq!(
            set("scroll-padding", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
}

/// Scroll Snap 1 §4.2: `scroll-margin: <length>{1,4}` — either sign, no
/// percentage, no `auto`.
#[test]
fn scroll_margin_takes_one_to_four_lengths() {
    let mut style = TuiStyle::new();
    set("scroll-margin", "1 -2", &mut style).unwrap();
    assert_eq!(
        serialize("scroll-margin-left", &style).as_deref(),
        Some("-2")
    );
    assert_eq!(
        serialize("scroll-margin-bottom", &style).as_deref(),
        Some("1")
    );
    assert_eq!(serialize("scroll-margin", &style).as_deref(), Some("1 -2"));
    for bad in ["auto", "10%", "1 2 3 4 5"] {
        assert_eq!(
            set("scroll-margin", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
}

/// Scroll Snap 1 §4.1–§4.2 with CSS Logical 1: the block-axis
/// longhands and shorthands are the top / bottom ones in
/// `horizontal-tb`.
#[test]
fn the_block_axis_logicals_are_top_and_bottom() {
    let mut style = TuiStyle::new();
    set("scroll-padding-block", "1 2", &mut style).unwrap();
    set("scroll-margin-block-end", "3", &mut style).unwrap();
    assert_eq!(
        serialize("scroll-padding-top", &style).as_deref(),
        Some("1")
    );
    assert_eq!(
        serialize("scroll-padding-bottom", &style).as_deref(),
        Some("2")
    );
    assert_eq!(
        serialize("scroll-margin-bottom", &style).as_deref(),
        Some("3")
    );
    assert!(!inherits("scroll-padding-top") && !inherits("scroll-margin-top"));
}
