//! Counter style tests (CSS Counter Styles 3): the predefined styles of
//! §6, the generation algorithm of §3.1 / §1.1 (ranges, fallback,
//! negative, pad) and its bounds.

use super::{CounterStyle, MAX_REPRESENTATION_CHARS};

fn f(style: &str, n: i32) -> String {
    CounterStyle::named(style).format(n)
}

/// §6.1 numeric styles: decimal, the native-digit styles (each script's
/// own ten digits), `cjk-decimal`; `decimal-leading-zero` pads to two
/// digits, the negative sign counting toward the pad (§3.1.6).
#[test]
fn numeric_styles_write_positional_digits() {
    assert_eq!(f("decimal", 12), "12");
    assert_eq!(f("decimal", -5), "-5");
    assert_eq!(f("decimal", i32::MIN), "-2147483648");
    assert_eq!(f("decimal-leading-zero", 7), "07");
    assert_eq!(f("decimal-leading-zero", -7), "-7");
    assert_eq!(f("decimal-leading-zero", 123), "123");
    assert_eq!(f("arabic-indic", 120), "\u{661}\u{662}\u{660}");
    assert_eq!(f("persian", 5), "\u{6f5}");
    assert_eq!(f("devanagari", 9), "\u{96f}");
    assert_eq!(f("bengali", 10), "\u{9e7}\u{9e6}");
    assert_eq!(f("thai", 10), "\u{e51}\u{e50}");
    assert_eq!(f("tibetan", 3), "\u{f23}");
    assert_eq!(f("khmer", 2), "\u{17e2}");
    assert_eq!(f("cambodian", 2), "\u{17e2}");
    assert_eq!(f("cjk-decimal", 2024), "二〇二四");
    for style in [
        "gujarati",
        "gurmukhi",
        "kannada",
        "lao",
        "malayalam",
        "mongolian",
        "myanmar",
        "oriya",
        "tamil",
        "telugu",
    ] {
        assert_eq!(f(style, 10).chars().count(), 2, "{style}");
        assert_ne!(f(style, 1), "1", "{style} has its own digits");
    }
}

/// §6.2 alphabetic styles: bijective base-N, range 1 to infinity; 0 and
/// negatives fall back to `decimal`.
#[test]
fn alphabetic_styles_count_bijectively() {
    assert_eq!(f("lower-alpha", 1), "a");
    assert_eq!(f("lower-latin", 27), "aa");
    assert_eq!(f("upper-alpha", 28), "AB");
    assert_eq!(f("upper-latin", 26), "Z");
    assert_eq!(f("lower-alpha", 0), "0");
    assert_eq!(f("lower-alpha", -1), "-1");
    assert_eq!(f("lower-greek", 1), "α");
    assert_eq!(f("lower-greek", 17), "ρ");
    assert_eq!(f("lower-greek", 18), "σ", "no final sigma");
    assert_eq!(f("lower-greek", 24), "ω");
    assert_eq!(f("lower-greek", 25), "αα");
    assert_eq!(f("hiragana", 1), "あ");
    assert_eq!(f("hiragana", 48), "ん");
    assert_eq!(f("hiragana", 49), "ああ");
    assert_eq!(f("hiragana-iroha", 3), "は");
    assert_eq!(f("katakana", 3), "ウ");
    assert_eq!(f("katakana-iroha", 1), "イ");
    assert_eq!(f("lower-alpha", i32::MAX), "fxshrxw");
}

/// §6.1 additive styles: roman (1–3999), Hebrew (1–10999), Armenian
/// (1–9999), Georgian (1–19999); outside its range a style falls back
/// to `decimal`.
#[test]
fn additive_styles_sum_their_weights() {
    assert_eq!(f("lower-roman", 1994), "mcmxciv");
    assert_eq!(f("upper-roman", 3999), "MMMCMXCIX");
    assert_eq!(f("upper-roman", 4), "IV");
    assert_eq!(f("upper-roman", 4000), "4000");
    assert_eq!(f("lower-roman", 0), "0");
    assert_eq!(f("upper-roman", i32::MAX), "2147483647");
    assert_eq!(f("hebrew", 1), "\u{5d0}");
    assert_eq!(f("hebrew", 15), "\u{5d8}\u{5d5}");
    assert_eq!(f("hebrew", 16), "\u{5d8}\u{5d6}");
    assert_eq!(f("hebrew", 300), "\u{5e9}");
    assert_eq!(f("hebrew", 1000), "\u{5d0}\u{5f3}");
    assert_eq!(f("hebrew", 11000), "11000");
    assert_eq!(f("armenian", 1), "Ա");
    assert_eq!(f("upper-armenian", 2024), "ՍԻԴ");
    assert_eq!(f("lower-armenian", 1), "ա");
    assert_eq!(f("armenian", 10000), "10000");
    assert_eq!(f("georgian", 1), "ა");
    assert_eq!(f("georgian", 10000), "ჵ");
    assert_eq!(f("georgian", 20000), "20000");
}

/// §6.3 symbolic (cyclic) styles and §6.4's fixed CJK styles, which
/// fall back to `cjk-decimal` past their twelve / ten symbols.
#[test]
fn symbol_and_fixed_styles() {
    assert_eq!(f("disc", 1), "•");
    assert_eq!(f("disc", 7), "•");
    assert_eq!(f("circle", 2), "◦");
    assert_eq!(f("square", 3), "▪");
    assert_eq!(f("disclosure-open", 1), "▾");
    assert_eq!(f("disclosure-closed", 1), "▸");
    assert_eq!(
        CounterStyle::named("disclosure-closed").format_in(1, true),
        "◂",
        "right-to-left points the other way"
    );
    assert_eq!(f("cjk-earthly-branch", 1), "子");
    assert_eq!(f("cjk-earthly-branch", 12), "亥");
    assert_eq!(f("cjk-earthly-branch", 13), "一三");
    assert_eq!(f("cjk-heavenly-stem", 10), "癸");
    assert_eq!(f("cjk-heavenly-stem", 0), "〇");
}

/// §3 (`<counter-style-name>`): an unknown name is `decimal`; `none`
/// generates nothing. Predefined names match ASCII case-insensitively,
/// others keep their case; CSS-wide keywords and `default` are no name.
#[test]
fn names_resolve_and_parse() {
    assert_eq!(f("no-such-style", 4), "4");
    assert_eq!(f("none", 4), "");
    assert_eq!(
        CounterStyle::parse("DECIMAL"),
        Some(CounterStyle::decimal())
    );
    assert_eq!(
        CounterStyle::parse("Upper-Roman").unwrap().name(),
        Some("upper-roman")
    );
    assert_eq!(
        CounterStyle::parse("MyStyle").unwrap().name(),
        Some("MyStyle")
    );
    for bad in [
        "inherit",
        "initial",
        "unset",
        "revert",
        "revert-layer",
        "default",
    ] {
        assert_eq!(CounterStyle::parse(bad), None, "{bad}");
    }
}

/// §3.1.3–§3.1.4: a marker is the representation between the style's
/// `prefix` and `suffix` — `". "` by default, `" "` for the symbolic
/// styles, `"、"` for the CJK ones; `counter()` is the bare
/// representation.
#[test]
fn markers_add_prefix_and_suffix() {
    let m = |s: &str, n| CounterStyle::named(s).marker_text(n, false);
    assert_eq!(m("decimal", 3), "3. ");
    assert_eq!(m("lower-roman", 4), "iv. ");
    assert_eq!(m("disc", 1), "• ");
    assert_eq!(m("cjk-decimal", 3), "三、");
    assert_eq!(m("hiragana", 1), "あ、");
    assert_eq!(m("none", 1), "");
}

/// §3.1 note: a representation may be capped — UAs "must support
/// representations at least 60 Unicode codepoints long" and may use the
/// fallback past that. Every predefined style stays bounded over the
/// whole `i32` range.
#[test]
fn every_representation_is_bounded() {
    for style in super::predefined_names() {
        for n in [i32::MIN, -1_000_000, -1, 0, 1, 99_999, i32::MAX] {
            let s = f(style, n);
            assert!(
                s.chars().count() <= MAX_REPRESENTATION_CHARS,
                "{style} {n}: {} chars",
                s.chars().count()
            );
        }
    }
}
