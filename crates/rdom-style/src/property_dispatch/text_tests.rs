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

/// CSS Text 4 §2.1: `text-transform: none | [capitalize | uppercase |
/// lowercase] || full-width || full-size-kana | math-auto` — the case
/// keyword once, the others in any order, serialized in the grammar's
/// order; inherited.
#[test]
fn text_transform_takes_its_combinations() {
    use crate::layout::{TextCase, TextTransform};
    for (text, value, out) in [
        ("none", TextTransform::NONE, "none"),
        (
            "uppercase",
            TextTransform {
                case: TextCase::Uppercase,
                ..TextTransform::NONE
            },
            "uppercase",
        ),
        (
            "full-width capitalize",
            TextTransform {
                case: TextCase::Capitalize,
                full_width: true,
                ..TextTransform::NONE
            },
            "capitalize full-width",
        ),
        (
            "full-size-kana full-width lowercase",
            TextTransform {
                case: TextCase::Lowercase,
                full_width: true,
                full_size_kana: true,
                math_auto: false,
            },
            "lowercase full-width full-size-kana",
        ),
        (
            "math-auto",
            TextTransform {
                math_auto: true,
                ..TextTransform::NONE
            },
            "math-auto",
        ),
    ] {
        let mut style = TuiStyle::new();
        set("text-transform", text, &mut style).unwrap_or_else(|e| panic!("{text}: {e:?}"));
        assert_eq!(
            style.text.text_transform,
            Some(Value::Specified(value)),
            "{text}"
        );
        assert_eq!(
            serialize("text-transform", &style).as_deref(),
            Some(out),
            "{text}"
        );
    }
    for bad in [
        "uppercase lowercase",
        "full-width full-width",
        "none uppercase",
        "math-auto uppercase",
        "capitalise",
    ] {
        assert_eq!(
            set("text-transform", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
    assert!(inherits("text-transform"));
}

/// CSS Text 3 §8.1: `text-indent: <length-percentage> && hanging? &&
/// each-line?` — the keywords in any order, a length of either sign, a
/// percentage, `calc()`; serialized length first; inherited.
#[test]
fn text_indent_takes_a_length_and_its_keywords() {
    use crate::layout::{Length, TextIndent};
    for (text, value, out) in [
        ("2", TextIndent::cells(2), "2"),
        ("-3", TextIndent::cells(-3), "-3"),
        (
            "hanging 1 each-line",
            TextIndent {
                hanging: true,
                each_line: true,
                ..TextIndent::cells(1)
            },
            "1 hanging each-line",
        ),
        (
            "each-line 4",
            TextIndent {
                each_line: true,
                ..TextIndent::cells(4)
            },
            "4 each-line",
        ),
    ] {
        let mut style = TuiStyle::new();
        set("text-indent", text, &mut style).unwrap_or_else(|e| panic!("{text}: {e:?}"));
        assert_eq!(
            style.text.text_indent,
            Some(Value::Specified(value)),
            "{text}"
        );
        assert_eq!(
            serialize("text-indent", &style).as_deref(),
            Some(out),
            "{text}"
        );
    }
    let mut style = TuiStyle::new();
    set("text-indent", "10%", &mut style).unwrap();
    let Some(Value::Specified(TextIndent {
        length: Length::Calc(_),
        ..
    })) = &style.text.text_indent
    else {
        panic!("a percentage resolves at layout")
    };
    for bad in ["hanging", "1 2", "1 hanging hanging", "auto", "1 first"] {
        assert_eq!(
            set("text-indent", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
    assert!(inherits("text-indent"));
}

/// CSS Text 3 §6.1–§6.4: `text-align` is the shorthand of
/// `text-align-all` and `text-align-last` — a value other than
/// `justify-all` / `match-parent` sets `text-align-all` and resets
/// `text-align-last` to `auto`; `text-justify` takes `distribute` as a
/// legacy alias of `inter-character`; all inherit.
#[test]
fn text_align_is_the_shorthand_of_all_and_last() {
    use crate::layout::{TextAlign, TextAlignLast, TextJustify};
    let longhands = |style: &TuiStyle| {
        let all = match &style.text.text_align_all {
            Some(Value::Specified(a)) => Some(*a),
            _ => None,
        };
        let last = match &style.text.text_align_last {
            Some(Value::Specified(l)) => Some(*l),
            _ => None,
        };
        (all, last)
    };
    for (text, all, last, out) in [
        ("start", TextAlign::Start, TextAlignLast::Auto, "start"),
        ("CENTER", TextAlign::Center, TextAlignLast::Auto, "center"),
        (
            "justify",
            TextAlign::Justify,
            TextAlignLast::Auto,
            "justify",
        ),
        (
            "justify-all",
            TextAlign::Justify,
            TextAlignLast::Justify,
            "justify-all",
        ),
        (
            "match-parent",
            TextAlign::MatchParent,
            TextAlignLast::MatchParent,
            "match-parent",
        ),
    ] {
        let mut style = TuiStyle::new();
        set("text-align", text, &mut style).unwrap_or_else(|e| panic!("{text}: {e:?}"));
        assert_eq!(longhands(&style), (Some(all), Some(last)), "{text}");
        assert_eq!(
            serialize("text-align", &style).as_deref(),
            Some(out),
            "{text}"
        );
    }
    let mut style = TuiStyle::new();
    set("text-align", "left", &mut style).unwrap();
    set("text-align-last", "right", &mut style).unwrap();
    assert_eq!(serialize("text-align", &style), None);
    assert_eq!(serialize("text-align-all", &style).as_deref(), Some("left"));
    assert_eq!(
        serialize("text-align-last", &style).as_deref(),
        Some("right")
    );
    for (text, value) in [
        ("auto", TextJustify::Auto),
        ("none", TextJustify::None),
        ("inter-word", TextJustify::InterWord),
        ("inter-character", TextJustify::InterCharacter),
        ("distribute", TextJustify::InterCharacter),
    ] {
        let mut style = TuiStyle::new();
        set("text-justify", text, &mut style).unwrap();
        assert_eq!(
            style.text.text_justify,
            Some(Value::Specified(value)),
            "{text}"
        );
    }
    for (name, bad) in [
        ("text-align", "auto"),
        ("text-align", "left right"),
        ("text-align-all", "justify-all"),
        ("text-align-last", "justify-all"),
        ("text-justify", "inter-ideograph"),
    ] {
        assert_eq!(
            set(name, bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{name}: {bad}"
        );
    }
    for name in [
        "text-align",
        "text-align-all",
        "text-align-last",
        "text-justify",
    ] {
        assert!(inherits(name), "{name}");
    }
}

/// CSS Text 4: `text-wrap: <'text-wrap-mode'> || <'text-wrap-style'>`,
/// an omitted longhand its initial value, serialized shortest;
/// `text-wrap-style: auto | balance | stable | pretty |
/// avoid-short-last-line`, inherited.
#[test]
fn text_wrap_is_the_shorthand_of_mode_and_style() {
    use crate::layout::{TextWrapMode, TextWrapStyle};
    for (text, mode, style_v, out) in [
        ("wrap", TextWrapMode::Wrap, TextWrapStyle::Auto, "wrap"),
        (
            "nowrap",
            TextWrapMode::Nowrap,
            TextWrapStyle::Auto,
            "nowrap",
        ),
        (
            "balance",
            TextWrapMode::Wrap,
            TextWrapStyle::Balance,
            "balance",
        ),
        (
            "pretty nowrap",
            TextWrapMode::Nowrap,
            TextWrapStyle::Pretty,
            "nowrap pretty",
        ),
        (
            "stable",
            TextWrapMode::Wrap,
            TextWrapStyle::Stable,
            "stable",
        ),
        (
            "avoid-short-last-line",
            TextWrapMode::Wrap,
            TextWrapStyle::AvoidShortLastLine,
            "avoid-short-last-line",
        ),
    ] {
        let mut style = TuiStyle::new();
        set("text-wrap", text, &mut style).unwrap_or_else(|e| panic!("{text}: {e:?}"));
        assert_eq!(
            style.text.text_wrap_mode,
            Some(Value::Specified(mode)),
            "{text}"
        );
        assert_eq!(
            style.text.text_wrap_style,
            Some(Value::Specified(style_v)),
            "{text}"
        );
        assert_eq!(
            serialize("text-wrap", &style).as_deref(),
            Some(out),
            "{text}"
        );
    }
    for bad in ["wrap nowrap", "balance pretty", "auto auto", "normal"] {
        assert_eq!(
            set("text-wrap", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
    let mut style = TuiStyle::new();
    set("text-wrap-style", "pretty", &mut style).unwrap();
    assert_eq!(
        serialize("text-wrap-style", &style).as_deref(),
        Some("pretty")
    );
    assert_eq!(serialize("text-wrap", &style), None);
    assert!(inherits("text-wrap-style"));
    assert!(inherits("text-wrap"));
}
