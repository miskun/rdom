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
