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

/// §6.1: `justify-self: auto | normal | stretch | <baseline-position> |
/// <overflow-position>? [ <self-position> | left | right ]`; §6.2:
/// `justify-items` swaps `auto` for `legacy | legacy && [ left | right |
/// center ]`, its initial value.
#[test]
fn justify_items_and_justify_self_take_their_grammars() {
    for (css, out) in [
        ("normal", "normal"),
        ("stretch", "stretch"),
        ("first baseline", "baseline"),
        ("self-end", "self-end"),
        ("left", "left"),
        ("safe right", "safe right"),
        ("center", "center"),
    ] {
        assert_eq!(round_trip("justify-self", css).as_deref(), Ok(out), "{css}");
        assert_eq!(
            round_trip("justify-items", css).as_deref(),
            Ok(out),
            "{css}"
        );
    }
    assert_eq!(round_trip("justify-self", "auto").as_deref(), Ok("auto"));
    for (css, out) in [
        ("legacy", "legacy"),
        ("legacy left", "legacy left"),
        ("center legacy", "legacy center"),
        ("LEGACY RIGHT", "legacy right"),
    ] {
        assert_eq!(
            round_trip("justify-items", css).as_deref(),
            Ok(out),
            "{css}"
        );
    }
    for bad in [
        "auto",
        "legacy start",
        "legacy legacy",
        "space-between",
        "safe legacy",
    ] {
        assert_eq!(
            set("justify-items", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    for bad in ["legacy", "legacy center", "space-around"] {
        assert_eq!(
            set("justify-self", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert!(!inherits("justify-items") && !inherits("justify-self"));
    let initial = crate::ComputedStyle::initial();
    assert_eq!(
        initial.justify_items,
        Alignment {
            keyword: Align::Normal,
            overflow: OverflowAlign::Default,
            legacy: true,
        }
    );
    assert_eq!(initial.justify_self, Alignment::AUTO);
}

/// §5.5 / §6.4 / §6.5: `place-content: <'align-content'>
/// <'justify-content'>?`, `place-items: <'align-items'>
/// <'justify-items'>?`, `place-self: <'align-self'> <'justify-self'>?` —
/// one value sets both (a `<baseline-position>` sets `justify-content`
/// to `start`); each owns its two longhands and serializes as one value
/// when they agree.
#[test]
fn place_shorthands_set_both_axes() {
    let mut style = TuiStyle::new();
    set("place-content", "center space-between", &mut style).unwrap();
    assert_eq!(
        style.align_content,
        Some(Value::Specified(Alignment::new(Align::Center)))
    );
    assert_eq!(
        style.justify_content,
        Some(Value::Specified(Alignment::new(Align::SpaceBetween)))
    );
    for (name, css, out) in [
        (
            "place-content",
            "center space-between",
            "center space-between",
        ),
        ("place-content", "safe end", "safe end"),
        ("place-content", "end end", "end"),
        ("place-content", "baseline", "baseline"),
        ("place-content", "last baseline left", "last baseline left"),
        ("place-items", "end", "end"),
        ("place-items", "baseline", "baseline"),
        ("place-items", "center legacy left", "center legacy left"),
        ("place-items", "safe end start", "safe end start"),
        ("place-self", "auto", "auto"),
        ("place-self", "center auto", "center auto"),
        ("place-self", "stretch self-end", "stretch self-end"),
    ] {
        assert_eq!(round_trip(name, css).as_deref(), Ok(out), "{name}: {css}");
    }
    let mut style = TuiStyle::new();
    set("place-content", "baseline", &mut style).unwrap();
    assert_eq!(
        style.justify_content,
        Some(Value::Specified(Alignment::new(Align::Start)))
    );
    for (name, bad) in [
        ("place-content", "left"),
        ("place-content", "center center center"),
        ("place-items", "auto"),
        ("place-items", "legacy"),
        ("place-self", "legacy center"),
        ("place-self", ""),
    ] {
        assert_eq!(
            set(name, bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{name}: {bad:?}"
        );
    }
    assert_eq!(
        property_mask("place-content"),
        Some(ImportantMask::ALIGN_CONTENT | ImportantMask::JUSTIFY_CONTENT)
    );
    assert_eq!(
        property_mask("place-items"),
        Some(ImportantMask::ALIGN_ITEMS | ImportantMask::JUSTIFY_ITEMS)
    );
    assert_eq!(
        property_mask("place-self"),
        Some(ImportantMask::ALIGN_SELF | ImportantMask::JUSTIFY_SELF)
    );
    // Only one longhand set: no shorthand.
    let mut style = TuiStyle::new();
    set("align-self", "center", &mut style).unwrap();
    assert_eq!(serialize("place-self", &style), None);
}
