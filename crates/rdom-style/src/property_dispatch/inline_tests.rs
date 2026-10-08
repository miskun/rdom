//! Dispatch tests for the CSS Inline Layout 3 properties (Phase 9):
//! `line-height` (§5.1) and `vertical-align` (CSS 2.1 §10.8.1).

use super::*;
use crate::layout::{LineHeight, VerticalAlign};
use crate::{TuiStyle, Value};

/// CSS Inline 3 §5.1: `line-height: normal | <number [0,∞]> |
/// <length-percentage [0,∞]>`, inherited, initial `normal`; a number is
/// kept as a number, a percentage and a context length for the cascade.
#[test]
fn line_height_takes_normal_a_number_or_a_length() {
    use crate::calc::{CalcExpr, CalcUnit};
    for (text, value, out) in [
        ("normal", LineHeight::Normal, "normal"),
        ("NORMAL", LineHeight::Normal, "normal"),
        ("1.5", LineHeight::Number(1.5), "1.5"),
        ("2", LineHeight::Number(2.0), "2"),
        ("0", LineHeight::Number(0.0), "0"),
        ("calc(1 + 1)", LineHeight::Number(2.0), "2"),
        ("3ch", LineHeight::Rows(3.0), "3ch"),
        ("150%", LineHeight::calc(CalcExpr::Percent(150.0)), "150%"),
        (
            "2lh",
            LineHeight::calc(CalcExpr::Dimension {
                value: 2.0,
                unit: CalcUnit::Lh,
            }),
            "2lh",
        ),
    ] {
        let mut style = TuiStyle::new();
        set("line-height", text, &mut style).unwrap_or_else(|e| panic!("{text}: {e:?}"));
        assert_eq!(
            style.text.line_height,
            Some(Value::Specified(value)),
            "{text}"
        );
        assert_eq!(
            serialize("line-height", &style).as_deref(),
            Some(out),
            "{text}"
        );
    }
    for bad in ["-1", "-10%", "auto", "1 2", "none"] {
        assert_eq!(
            set("line-height", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
    assert!(inherits("line-height"));
    assert_eq!(
        crate::layout::TextStyle::default().line_height,
        LineHeight::Normal
    );
}

/// CSS 2.1 §10.8.1: `vertical-align: baseline | sub | super | text-top |
/// text-bottom | middle | top | bottom | <percentage> | <length>`, not
/// inherited, initial `baseline`; a length of either sign.
#[test]
fn vertical_align_takes_its_keywords_and_a_length() {
    use crate::calc::CalcExpr;
    for (text, value, out) in [
        ("baseline", VerticalAlign::Baseline, "baseline"),
        ("SUB", VerticalAlign::Sub, "sub"),
        ("super", VerticalAlign::Super, "super"),
        ("text-top", VerticalAlign::TextTop, "text-top"),
        ("text-bottom", VerticalAlign::TextBottom, "text-bottom"),
        ("middle", VerticalAlign::Middle, "middle"),
        ("top", VerticalAlign::Top, "top"),
        ("bottom", VerticalAlign::Bottom, "bottom"),
        ("2", VerticalAlign::Rows(2.0), "2"),
        ("-1", VerticalAlign::Rows(-1.0), "-1"),
        ("50%", VerticalAlign::calc(CalcExpr::Percent(50.0)), "50%"),
    ] {
        let mut style = TuiStyle::new();
        set("vertical-align", text, &mut style).unwrap_or_else(|e| panic!("{text}: {e:?}"));
        assert_eq!(
            style.vertical_align,
            Some(Value::Specified(value)),
            "{text}"
        );
        assert_eq!(
            serialize("vertical-align", &style).as_deref(),
            Some(out),
            "{text}"
        );
    }
    for bad in ["auto", "center", "top bottom", "none"] {
        assert_eq!(
            set("vertical-align", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
    assert!(!inherits("vertical-align"));
    assert_eq!(
        crate::ComputedStyle::initial().vertical_align,
        VerticalAlign::Baseline
    );
}
