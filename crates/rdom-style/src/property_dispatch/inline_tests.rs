//! Dispatch tests for the CSS Inline Layout 3 properties (Phase 9):
//! `line-height` (§5.1).

use super::*;
use crate::layout::LineHeight;
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
        (
            "150%",
            LineHeight::Calc(Box::new(CalcExpr::Percent(150.0))),
            "150%",
        ),
        (
            "2lh",
            LineHeight::Calc(Box::new(CalcExpr::Dimension {
                value: 2.0,
                unit: CalcUnit::Lh,
            })),
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
