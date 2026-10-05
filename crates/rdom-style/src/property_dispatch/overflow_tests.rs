//! Dispatch tests for `overflow` and its longhands (C8-OVERFLOW-CLIP,
//! CSS Overflow 3 §3).

use super::*;
use crate::layout::{Overflow, OverflowClipMargin, VisualBox};
use crate::{ImportantMask, TuiStyle, Value};

fn axes(style: &TuiStyle) -> (Option<Overflow>, Option<Overflow>) {
    let get = |v: &Option<Value<Overflow>>| match v {
        Some(Value::Specified(o)) => Some(*o),
        _ => None,
    };
    (get(&style.overflow_x), get(&style.overflow_y))
}

/// §3.1: `visible | hidden | clip | scroll | auto` on each longhand.
#[test]
fn every_longhand_takes_clip() {
    for name in ["overflow-x", "overflow-y"] {
        let mut style = TuiStyle::new();
        set(name, "CLIP", &mut style).unwrap();
        assert_eq!(serialize(name, &style).as_deref(), Some("clip"));
    }
    let mut style = TuiStyle::new();
    set("overflow", "clip", &mut style).unwrap();
    assert_eq!(axes(&style), (Some(Overflow::Clip), Some(Overflow::Clip)));
}

/// §3.1: `overflow: [visible | hidden | clip | scroll | auto]{1,2}` —
/// "the first value is assigned to overflow-x and the second to
/// overflow-y"; one value sets both. It serializes to one value when
/// both are the same.
#[test]
fn the_shorthand_takes_two_values() {
    let mut style = TuiStyle::new();
    set("overflow", "hidden scroll", &mut style).unwrap();
    assert_eq!(
        axes(&style),
        (Some(Overflow::Hidden), Some(Overflow::Scroll))
    );
    assert_eq!(
        serialize("overflow", &style).as_deref(),
        Some("hidden scroll")
    );
    set("overflow", "auto", &mut style).unwrap();
    assert_eq!(serialize("overflow", &style).as_deref(), Some("auto"));
    for bad in ["hidden scroll auto", "hidden,scroll", "1", "overlay"] {
        assert_eq!(
            set("overflow", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
}

/// CSS Logical 1 / Overflow 3 §3.1: `overflow-block` / `overflow-inline`
/// are the block- and inline-axis longhands — in `horizontal-tb`
/// (rdom's only writing mode) `overflow-y` and `overflow-x`, one storage.
#[test]
fn the_logical_longhands_are_the_axes() {
    let mut style = TuiStyle::new();
    set("overflow-block", "clip", &mut style).unwrap();
    set("overflow-inline", "auto", &mut style).unwrap();
    assert_eq!(axes(&style), (Some(Overflow::Auto), Some(Overflow::Clip)));
    assert_eq!(serialize("overflow-block", &style).as_deref(), Some("clip"));
    assert!(property_names().contains(&"overflow-inline"));
}

/// §3.2: `overflow-clip-margin: <visual-box> || <length [0,∞]>`, the
/// box `padding-box` and the length 0 when omitted.
#[test]
fn overflow_clip_margin_takes_a_box_and_a_length() {
    let margin = |css: &str| {
        let mut style = TuiStyle::new();
        set("overflow-clip-margin", css, &mut style).ok()?;
        match style.overflow_clip_margin {
            Some(Value::Specified(m)) => Some((m, serialize("overflow-clip-margin", &style)?)),
            _ => None,
        }
    };
    let m = |visual_box, cells| OverflowClipMargin::new(visual_box, cells);
    assert_eq!(margin("2"), Some((m(VisualBox::PaddingBox, 2), "2".into())));
    assert_eq!(
        margin("content-box"),
        Some((m(VisualBox::ContentBox, 0), "content-box".into()))
    );
    assert_eq!(
        margin("1 border-box"),
        Some((m(VisualBox::BorderBox, 1), "border-box 1".into()))
    );
    assert_eq!(
        margin("padding-box 3"),
        Some((m(VisualBox::PaddingBox, 3), "3".into()))
    );
    for bad in ["-1", "10%", "auto", "1 2", "content-box border-box"] {
        assert_eq!(margin(bad), None, "{bad:?}");
    }
    assert!(!inherits("overflow-clip-margin"));
    assert_eq!(
        property_mask("overflow-clip-margin"),
        Some(ImportantMask::OVERFLOW_CLIP_MARGIN)
    );
}
