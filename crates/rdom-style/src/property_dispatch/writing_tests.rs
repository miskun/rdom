//! Dispatch tests for writing modes (CSS-COMPLETE Phase 5): `direction`
//! and `writing-mode`.

use super::*;
use crate::layout::{TextDirection, WritingMode};
use crate::{ComputedStyle, TuiStyle, Value};

/// CSS Writing Modes 4 §2.1: `direction: ltr | rtl`, inherited, initial
/// `ltr`; ASCII case-insensitive. CSS Cascade 4 §3.2: `all` leaves it
/// alone.
#[test]
fn direction_takes_ltr_and_rtl() {
    for (css, want) in [("ltr", TextDirection::Ltr), ("RTL", TextDirection::Rtl)] {
        let mut style = TuiStyle::new();
        set("direction", css, &mut style).unwrap();
        assert_eq!(style.text_direction, Some(Value::Specified(want)));
        assert_eq!(
            serialize("direction", &style).as_deref(),
            Some(css.to_ascii_lowercase().as_str())
        );
    }
    for bad in ["auto", "left", "ltr rtl", ""] {
        let mut style = TuiStyle::new();
        assert_eq!(
            set("direction", bad, &mut style),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert!(inherits("direction"));
    assert_eq!(ComputedStyle::initial().text_direction, TextDirection::Ltr);
    let mut style = TuiStyle::new();
    set("all", "initial", &mut style).unwrap();
    assert_eq!(style.text_direction, None, "`all` excludes `direction`");
    assert_eq!(TuiStyle::new().text_direction(TextDirection::Rtl), {
        let mut s = TuiStyle::new();
        set("direction", "rtl", &mut s).unwrap();
        s
    });
}

/// CSS Writing Modes 4 §3.1: `writing-mode: horizontal-tb | vertical-rl |
/// vertical-lr | sideways-rl | sideways-lr`, inherited, initial
/// `horizontal-tb`.
#[test]
fn writing_mode_takes_its_five_keywords() {
    for (css, want) in [
        ("horizontal-tb", WritingMode::HorizontalTb),
        ("vertical-rl", WritingMode::VerticalRl),
        ("Vertical-LR", WritingMode::VerticalLr),
        ("sideways-rl", WritingMode::SidewaysRl),
        ("sideways-lr", WritingMode::SidewaysLr),
    ] {
        let mut style = TuiStyle::new();
        set("writing-mode", css, &mut style).unwrap();
        assert_eq!(style.writing_mode, Some(Value::Specified(want)));
        assert_eq!(
            serialize("writing-mode", &style).as_deref(),
            Some(css.to_ascii_lowercase().as_str())
        );
    }
    let mut style = TuiStyle::new();
    assert_eq!(
        set("writing-mode", "vertical", &mut style),
        Err(DispatchError::InvalidValue)
    );
    assert!(inherits("writing-mode"));
    assert_eq!(
        ComputedStyle::initial().writing_mode,
        WritingMode::HorizontalTb
    );
}
