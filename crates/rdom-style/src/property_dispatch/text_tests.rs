//! Dispatch tests for the CSS Text properties (Phase 9): `white-space`
//! and its longhands `white-space-collapse` / `text-wrap-mode` (CSS Text
//! 4 §3, §4.1, §6.1); `word-break`, `overflow-wrap` / `word-wrap`,
//! `line-break`, `hyphens` (CSS Text 3 §5.2, §5.5, §5.3, §6.1).

use super::*;
use crate::layout::{TextWrapMode, WhiteSpaceCollapse};
use crate::{TuiStyle, Value};

fn longhands(style: &TuiStyle) -> (Option<WhiteSpaceCollapse>, Option<TextWrapMode>) {
    let c = match &style.text.white_space_collapse {
        Some(Value::Specified(c)) => Some(*c),
        _ => None,
    };
    let m = match &style.text.text_wrap_mode {
        Some(Value::Specified(m)) => Some(*m),
        _ => None,
    };
    (c, m)
}

/// CSS Text 4 §3: `white-space` is a shorthand of `white-space-collapse`
/// and `text-wrap-mode`; its keywords map by the spec's table, the
/// longhand values combine in either order, an omitted one is its initial
/// value; it serializes as the shortest form (the keyword a pair spells,
/// else the non-initial longhands).
#[test]
fn white_space_is_the_shorthand_of_its_two_longhands() {
    use TextWrapMode::{Nowrap, Wrap};
    use WhiteSpaceCollapse as C;
    for (text, pair, out) in [
        ("normal", (C::Collapse, Wrap), "normal"),
        ("pre", (C::Preserve, Nowrap), "pre"),
        ("pre-wrap", (C::Preserve, Wrap), "pre-wrap"),
        ("PRE-LINE", (C::PreserveBreaks, Wrap), "pre-line"),
        ("nowrap", (C::Collapse, Nowrap), "nowrap"),
        ("break-spaces", (C::BreakSpaces, Wrap), "break-spaces"),
        ("preserve wrap", (C::Preserve, Wrap), "pre-wrap"),
        ("nowrap preserve", (C::Preserve, Nowrap), "pre"),
        ("collapse", (C::Collapse, Wrap), "normal"),
        ("wrap", (C::Collapse, Wrap), "normal"),
        (
            "preserve-spaces",
            (C::PreserveSpaces, Wrap),
            "preserve-spaces",
        ),
        (
            "preserve-spaces nowrap",
            (C::PreserveSpaces, Nowrap),
            "preserve-spaces nowrap",
        ),
        (
            "break-spaces nowrap",
            (C::BreakSpaces, Nowrap),
            "break-spaces nowrap",
        ),
        ("preserve-breaks", (C::PreserveBreaks, Wrap), "pre-line"),
    ] {
        let mut style = TuiStyle::new();
        set("white-space", text, &mut style).unwrap_or_else(|e| panic!("{text}: {e:?}"));
        assert_eq!(longhands(&style), (Some(pair.0), Some(pair.1)), "{text}");
        assert_eq!(
            serialize("white-space", &style).as_deref(),
            Some(out),
            "{text}"
        );
    }
    for bad in [
        "pre pre",
        "wrap nowrap",
        "pre wrap",
        "normal nowrap",
        "1",
        "discard",
        "preserve preserve-breaks",
    ] {
        assert_eq!(
            set("white-space", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
}

/// CSS Text 4 §4.1 / §6.1: the longhands take their own keywords and
/// serialize as written; the shorthand serializes from longhands set one
/// by one.
#[test]
fn the_longhands_parse_and_compose_the_shorthand() {
    let mut style = TuiStyle::new();
    set("white-space-collapse", "preserve-breaks", &mut style).unwrap();
    assert_eq!(
        serialize("white-space-collapse", &style).as_deref(),
        Some("preserve-breaks")
    );
    // One longhand alone does not make the shorthand.
    assert_eq!(serialize("white-space", &style), None);
    set("text-wrap-mode", "nowrap", &mut style).unwrap();
    assert_eq!(
        serialize("text-wrap-mode", &style).as_deref(),
        Some("nowrap")
    );
    assert_eq!(
        serialize("white-space", &style).as_deref(),
        Some("preserve-breaks nowrap")
    );
    for (name, bad) in [
        ("white-space-collapse", "pre"),
        ("white-space-collapse", "normal"),
        ("text-wrap-mode", "balance"),
        ("text-wrap-mode", "pre"),
    ] {
        assert_eq!(
            set(name, bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{name}: {bad}"
        );
    }
    // Both inherit (CSS Text 4 §4.1, §6.1); the shorthand does too.
    assert!(inherits("white-space-collapse"));
    assert!(inherits("text-wrap-mode"));
    assert!(inherits("white-space"));
}

/// CSS Text 3 §5.2 / §5.5 / §5.3 / §6.1: the line-breaking keywords,
/// ASCII case-insensitive, serialized as written; `word-wrap` is a legacy
/// name alias of `overflow-wrap` (§5.5: "UAs must treat word-wrap as a
/// legacy name alias"), writing and reading the same field; all inherit.
#[test]
fn the_line_breaking_properties_take_their_keywords() {
    for (name, values) in [
        (
            "word-break",
            &["normal", "break-all", "keep-all", "break-word"][..],
        ),
        ("overflow-wrap", &["normal", "break-word", "anywhere"][..]),
        ("word-wrap", &["normal", "break-word", "anywhere"][..]),
        (
            "line-break",
            &["auto", "loose", "normal", "strict", "anywhere"][..],
        ),
        ("hyphens", &["none", "manual", "auto"][..]),
    ] {
        for v in values {
            let mut style = TuiStyle::new();
            set(name, &v.to_ascii_uppercase(), &mut style)
                .unwrap_or_else(|e| panic!("{name}: {v}: {e:?}"));
            assert_eq!(serialize(name, &style).as_deref(), Some(*v), "{name}: {v}");
        }
        assert_eq!(
            set(name, "wrap", &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{name}"
        );
        assert!(inherits(name), "{name}");
    }
    let mut style = TuiStyle::new();
    set("word-wrap", "break-word", &mut style).unwrap();
    assert_eq!(
        serialize("overflow-wrap", &style).as_deref(),
        Some("break-word")
    );
    assert_eq!(
        style.text.overflow_wrap,
        Some(Value::Specified(crate::layout::OverflowWrap::BreakWord))
    );
}

/// CSS Text 3 §4.2: `tab-size: <number [0,∞]> | <length [0,∞]>` — a
/// number counts spaces, a length is a length (cells, `ch`); negative
/// values and percentages are invalid; inherited, initial 8.
#[test]
fn tab_size_takes_a_number_or_a_length() {
    use crate::layout::TabSize;
    for (text, value, out) in [
        ("4", TabSize::Number(4.0), "4"),
        ("2.5", TabSize::Number(2.5), "2.5"),
        ("0", TabSize::Number(0.0), "0"),
        ("3ch", TabSize::Length(3.0), "3ch"),
    ] {
        let mut style = TuiStyle::new();
        set("tab-size", text, &mut style).unwrap_or_else(|e| panic!("{text}: {e:?}"));
        assert_eq!(style.text.tab_size, Some(Value::Specified(value)), "{text}");
        assert_eq!(
            serialize("tab-size", &style).as_deref(),
            Some(out),
            "{text}"
        );
    }
    for bad in ["-1", "50%", "auto", "4 4"] {
        assert_eq!(
            set("tab-size", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
    assert!(inherits("tab-size"));
    assert_eq!(
        crate::layout::TextStyle::default().tab_size,
        TabSize::Number(8.0)
    );
    assert_eq!(TabSize::Number(2.5).cells(), 2);
    assert_eq!(TabSize::Number(3.5).cells(), 4);
    assert_eq!(TabSize::Length(3.0).cells(), 3);
}
