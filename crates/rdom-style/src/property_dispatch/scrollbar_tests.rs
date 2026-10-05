//! Dispatch tests for `scrollbar-gutter`, `scrollbar-width` and
//! `scrollbar-color` (C8-SCROLLBAR; CSS Overflow 3 §3.3, CSS Scrollbars
//! 1 §2–§3).

use super::*;
use crate::TuiStyle;

fn round_trip(name: &str, value: &str) -> Option<String> {
    let mut style = TuiStyle::new();
    set(name, value, &mut style).ok()?;
    serialize(name, &style)
}

/// CSS Overflow 3 §3.3: `scrollbar-gutter: auto | stable && both-edges?`
/// — `both-edges` only beside `stable`, in either order, serialized
/// `stable both-edges`.
#[test]
fn scrollbar_gutter_takes_both_edges_with_stable() {
    assert_eq!(
        round_trip("scrollbar-gutter", "auto").as_deref(),
        Some("auto")
    );
    assert_eq!(
        round_trip("scrollbar-gutter", "stable").as_deref(),
        Some("stable")
    );
    for text in ["stable both-edges", "BOTH-EDGES stable"] {
        assert_eq!(
            round_trip("scrollbar-gutter", text).as_deref(),
            Some("stable both-edges"),
            "{text}"
        );
    }
    for bad in [
        "both-edges",
        "auto both-edges",
        "stable stable",
        "stable both-edges x",
    ] {
        assert_eq!(round_trip("scrollbar-gutter", bad), None, "{bad}");
    }
}

/// CSS Scrollbars 1 §3: `scrollbar-width: auto | thin | none`.
#[test]
fn scrollbar_width_takes_its_keywords() {
    for text in ["auto", "thin", "none"] {
        assert_eq!(round_trip("scrollbar-width", text).as_deref(), Some(text));
    }
    for bad in ["thick", "1", "thin none"] {
        assert_eq!(round_trip("scrollbar-width", bad), None, "{bad}");
    }
}

/// CSS Scrollbars 1 §2: `scrollbar-color: auto | <color>{2}` — the thumb
/// color, then the track color.
#[test]
fn scrollbar_color_takes_auto_or_two_colors() {
    assert_eq!(
        round_trip("scrollbar-color", "auto").as_deref(),
        Some("auto")
    );
    assert_eq!(
        round_trip("scrollbar-color", "#102030 #405060").as_deref(),
        Some("rgb(16, 32, 48) rgb(64, 80, 96)")
    );
    for bad in ["red", "red blue green", "auto red"] {
        assert_eq!(round_trip("scrollbar-color", bad), None, "{bad}");
    }
}

/// CSS Scrollbars 1 §2: `scrollbar-color` is inherited, `scrollbar-width`
/// is not (§3), nor `scrollbar-gutter` (CSS Overflow 3 §3.3).
#[test]
fn only_scrollbar_color_inherits() {
    assert!(inherits("scrollbar-color"));
    assert!(!inherits("scrollbar-width"));
    assert!(!inherits("scrollbar-gutter"));
}
