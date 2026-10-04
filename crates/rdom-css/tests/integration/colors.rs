//! §11.5 — Color values. The full matrix the parser accepts:
//! named, hex (3/4/6/8 digits, alpha on 4 and 8), rgb() / rgba()
//! (legacy and modern syntax, with alpha), var(--name), var(--name,
//! fallback), and nested var() fallback.

use rdom_css::parse;
use rdom_tui::style::Value;
use rdom_tui::{Color, TuiColor};

fn fg_of(source: &str) -> TuiColor {
    let r = parse(source);
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
    let v = r.stylesheet.rules()[0].style.fg.clone().expect("fg set");
    match v {
        Value::Specified(c) => c,
        _ => panic!("expected Specified, got {v:?}"),
    }
}

// ── Hex ─────────────────────────────────────────────────────────

#[test]
fn hex_three_digit() {
    assert_eq!(
        fg_of("a { color: #f00; }"),
        TuiColor::Literal(Color::Rgb(0xff, 0, 0))
    );
}

#[test]
fn hex_six_digit() {
    assert_eq!(
        fg_of("a { color: #ff0000; }"),
        TuiColor::Literal(Color::Rgb(0xff, 0, 0))
    );
}

#[test]
fn hex_four_digit_alpha() {
    // #rgba — short form with alpha (CSS Color 4 §5.2); opaque is Rgb.
    assert_eq!(
        fg_of("a { color: #f00f; }"),
        TuiColor::Literal(Color::Rgb(0xff, 0, 0))
    );
    assert_eq!(
        fg_of("a { color: #f008; }"),
        TuiColor::Literal(Color::Rgba(0xff, 0, 0, 0x88))
    );
}

#[test]
fn hex_eight_digit_alpha() {
    // #rrggbbaa — long form with alpha (CSS Color 4 §5.2).
    assert_eq!(
        fg_of("a { color: #ff000080; }"),
        TuiColor::Literal(Color::Rgba(0xff, 0, 0, 0x80))
    );
}

// ── rgb() / rgba() ──────────────────────────────────────────────

#[test]
fn rgb_function() {
    assert_eq!(
        fg_of("a { color: rgb(0, 128, 255); }"),
        TuiColor::Literal(Color::Rgb(0, 128, 255))
    );
}

#[test]
fn rgb_with_extra_whitespace() {
    assert_eq!(
        fg_of("a { color: rgb( 12 , 34 , 56 ); }"),
        TuiColor::Literal(Color::Rgb(12, 34, 56))
    );
}

#[test]
fn rgba_opaque_alpha_is_rgb() {
    assert_eq!(
        fg_of("a { color: rgba(10, 20, 30, 1); }"),
        TuiColor::Literal(Color::Rgb(10, 20, 30))
    );
}

#[test]
fn rgba_keeps_alpha() {
    // CSS Color 4 §5.1: the alpha is kept, as a byte.
    assert_eq!(
        fg_of("a { color: rgba(10, 20, 30, 0.5); }"),
        TuiColor::Literal(Color::Rgba(10, 20, 30, 128))
    );
}

#[test]
fn rgb_modern_syntax() {
    // CSS Color 4 §5.1: space-separated channels, `/ alpha`,
    // percentages, `none`.
    assert_eq!(
        fg_of("a { color: rgb(100% 0 none / 25%); }"),
        TuiColor::Literal(Color::Rgba(255, 0, 0, 64))
    );
}

// ── var() ───────────────────────────────────────────────────────
//
// CSS Variables 1 §3: a declaration holding `var()` is kept as tokens
// (`TuiStyle::pending`) for the cascade to substitute per element —
// fallbacks included, whatever they hold — and serializes as written.

/// The `var()` text `css`'s first rule keeps for `property`, after
/// checking nothing was parsed into the typed field.
fn var_text(css: &str, property: &str) -> String {
    let r = parse(css);
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
    let style = &r.stylesheet.rules()[0].style;
    assert!(style.fg.is_none() && style.border_color.top.is_none());
    assert_eq!(style.pending.len(), 1);
    rdom_css::property_dispatch::serialize(property, style).expect("serialized")
}

#[test]
fn var_simple() {
    assert_eq!(
        var_text("a { color: var(--accent); }", "color"),
        "var( --accent )"
    );
}

#[test]
fn var_with_fallbacks_is_kept_for_the_cascade() {
    assert_eq!(
        var_text("a { color: var(--accent, red); }", "color"),
        "var( --accent , red )"
    );
    assert_eq!(
        var_text(
            "a { color: var(--accent, var(--secondary, #00f)); }",
            "color"
        ),
        "var( --accent , var( --secondary , #00f ) )"
    );
}

#[test]
fn invalid_var_syntax_is_invalid_at_parse_time() {
    let r = parse("a { color: var(accent); }");
    assert_eq!(r.warnings.len(), 1, "{:?}", r.warnings);
}

// ── Color works on all three target properties ───────────────────

#[test]
fn background_color_hex() {
    let r = parse("a { background-color: #abc; }");
    assert!(r.warnings.is_empty());
    let v = r.stylesheet.rules()[0].style.bg.clone().expect("bg");
    assert_eq!(
        v,
        Value::Specified(TuiColor::Literal(Color::Rgb(0xaa, 0xbb, 0xcc)))
    );
}

#[test]
fn border_color_var() {
    assert_eq!(
        var_text("a { border-color: var(--frame); }", "border-color"),
        "var( --frame )"
    );
}
