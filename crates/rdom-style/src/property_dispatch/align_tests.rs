//! Dispatch tests for the Box Alignment properties (CSS Box Alignment 3
//! §5–§6; C6-JUSTIFY, C6-ALIGN, C6-ALIGN-CONTENT, C6-PLACE).

use super::*;
use crate::layout::{Align, Alignment, OverflowAlign};
use crate::{ImportantMask, TuiStyle, Value};

/// Set `name: css` on a fresh style; its serialization back.
fn round_trip(name: &str, css: &str) -> Result<String, DispatchError> {
    let mut style = TuiStyle::new();
    set(name, css, &mut style)?;
    Ok(serialize(name, &style).expect("serializes"))
}

/// §5.2: `justify-content: normal | <content-distribution> |
/// <overflow-position>? [ <content-position> | left | right ]`, initial
/// `normal`, not inherited.
#[test]
fn justify_content_takes_its_grammar() {
    for (css, out) in [
        ("normal", "normal"),
        ("flex-start", "flex-start"),
        ("FLEX-END", "flex-end"),
        ("start", "start"),
        ("end", "end"),
        ("left", "left"),
        ("right", "right"),
        ("center", "center"),
        ("space-between", "space-between"),
        ("space-around", "space-around"),
        ("space-evenly", "space-evenly"),
        ("stretch", "stretch"),
        ("safe center", "safe center"),
        ("unsafe end", "unsafe end"),
        ("safe left", "safe left"),
    ] {
        assert_eq!(
            round_trip("justify-content", css).as_deref(),
            Ok(out),
            "{css}"
        );
    }
    let mut style = TuiStyle::new();
    set("justify-content", "safe flex-end", &mut style).unwrap();
    assert_eq!(
        style.justify_content,
        Some(Value::Specified(Alignment {
            keyword: Align::FlexEnd,
            overflow: OverflowAlign::Safe,
            legacy: false,
        }))
    );
    for bad in [
        "",
        "auto",
        "baseline",
        "first baseline",
        "self-start",
        "safe space-between",
        "safe",
        "center safe",
        "left right",
        "legacy",
    ] {
        assert_eq!(
            set("justify-content", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert!(!inherits("justify-content"));
    assert_eq!(
        property_mask("justify-content"),
        Some(ImportantMask::JUSTIFY_CONTENT)
    );
    assert_eq!(
        crate::ComputedStyle::initial().justify_content,
        Alignment::NORMAL
    );
}

/// §6.3 / §6.1: `align-items: normal | stretch | <baseline-position> |
/// <overflow-position>? <self-position>` (no `left` / `right`, no
/// distribution); `align-self` adds `auto`, its initial value.
#[test]
fn align_items_and_align_self_take_their_grammars() {
    for (css, out) in [
        ("normal", "normal"),
        ("stretch", "stretch"),
        ("baseline", "baseline"),
        ("first baseline", "baseline"),
        ("LAST BASELINE", "last baseline"),
        ("center", "center"),
        ("self-start", "self-start"),
        ("self-end", "self-end"),
        ("flex-end", "flex-end"),
        ("safe end", "safe end"),
        ("unsafe self-start", "unsafe self-start"),
    ] {
        assert_eq!(round_trip("align-items", css).as_deref(), Ok(out), "{css}");
        assert_eq!(round_trip("align-self", css).as_deref(), Ok(out), "{css}");
    }
    assert_eq!(round_trip("align-self", "auto").as_deref(), Ok("auto"));
    for bad in [
        "auto",
        "left",
        "space-between",
        "safe baseline",
        "first",
        "last",
    ] {
        assert_eq!(
            set("align-items", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert_eq!(
        set("align-self", "right", &mut TuiStyle::new()),
        Err(DispatchError::InvalidValue)
    );
    assert!(!inherits("align-items") && !inherits("align-self"));
    assert_eq!(
        property_mask("align-items"),
        Some(ImportantMask::ALIGN_ITEMS)
    );
    assert_eq!(property_mask("align-self"), Some(ImportantMask::ALIGN_SELF));
    let initial = crate::ComputedStyle::initial();
    assert_eq!(initial.align_items, Alignment::NORMAL);
    assert_eq!(initial.align_self, Alignment::AUTO);
}

/// §5.1: `align-content: normal | <baseline-position> |
/// <content-distribution> | <overflow-position>? <content-position>` —
/// no `left` / `right`, no self positions.
#[test]
fn align_content_takes_its_grammar() {
    for (css, out) in [
        ("normal", "normal"),
        ("baseline", "baseline"),
        ("last baseline", "last baseline"),
        ("space-between", "space-between"),
        ("space-around", "space-around"),
        ("space-evenly", "space-evenly"),
        ("stretch", "stretch"),
        ("center", "center"),
        ("safe flex-end", "safe flex-end"),
        ("unsafe start", "unsafe start"),
    ] {
        assert_eq!(
            round_trip("align-content", css).as_deref(),
            Ok(out),
            "{css}"
        );
    }
    for bad in ["auto", "left", "self-start", "safe stretch", ""] {
        assert_eq!(
            set("align-content", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert!(!inherits("align-content"));
    assert_eq!(
        property_mask("align-content"),
        Some(ImportantMask::ALIGN_CONTENT)
    );
    assert_eq!(
        crate::ComputedStyle::initial().align_content,
        Alignment::NORMAL
    );
}
