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
