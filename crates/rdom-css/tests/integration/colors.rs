//! §11.5 — Color values. The full matrix the parser accepts:
//! named, hex (3/4/6/8 digits — alpha dropped on 4 and 8),
//! rgb(), rgba() (alpha dropped), var(--name), var(--name,
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
fn hex_four_digit_alpha_dropped() {
    // #rgba — short form with alpha. Alpha (4th nibble) ignored.
    assert_eq!(
        fg_of("a { color: #f00f; }"),
        TuiColor::Literal(Color::Rgb(0xff, 0, 0))
    );
}

#[test]
fn hex_eight_digit_alpha_dropped() {
    // #rrggbbaa — long form with alpha. Alpha (last two) ignored.
    assert_eq!(
        fg_of("a { color: #ff000080; }"),
        TuiColor::Literal(Color::Rgb(0xff, 0, 0))
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
fn rgba_drops_alpha_integer() {
    // rgba with integer alpha; alpha dropped.
    assert_eq!(
        fg_of("a { color: rgba(10, 20, 30, 1); }"),
        TuiColor::Literal(Color::Rgb(10, 20, 30))
    );
}

#[test]
fn rgba_drops_alpha_decimal() {
    // rgba with float alpha (`0.5`). The float tokenizes as
    // Number(0) Delim('.') Number(5); the parser consumes
    // tokens until RParen so the actual representation doesn't
    // matter for v1.
    assert_eq!(
        fg_of("a { color: rgba(10, 20, 30, 0.5); }"),
        TuiColor::Literal(Color::Rgb(10, 20, 30))
    );
}

// ── var() ───────────────────────────────────────────────────────

#[test]
fn var_simple() {
    let c = fg_of("a { color: var(--accent); }");
    match c {
        TuiColor::Var { name, fallback } => {
            assert_eq!(name, "accent");
            assert!(fallback.is_none());
        }
        other => panic!("expected Var, got {other:?}"),
    }
}

#[test]
fn var_with_named_fallback() {
    let c = fg_of("a { color: var(--accent, red); }");
    match c {
        TuiColor::Var { name, fallback } => {
            assert_eq!(name, "accent");
            assert_eq!(
                fallback.as_deref(),
                Some(&TuiColor::Literal(Color::Rgb(255, 0, 0)))
            );
        }
        other => panic!("expected Var, got {other:?}"),
    }
}

#[test]
fn var_with_hex_fallback() {
    let c = fg_of("a { color: var(--accent, #00ff00); }");
    match c {
        TuiColor::Var { fallback, .. } => {
            assert_eq!(
                fallback.as_deref(),
                Some(&TuiColor::Literal(Color::Rgb(0, 255, 0)))
            );
        }
        other => panic!("expected Var, got {other:?}"),
    }
}

#[test]
fn var_with_nested_var_fallback() {
    let c = fg_of("a { color: var(--accent, var(--secondary, blue)); }");
    match c {
        TuiColor::Var { name, fallback } => {
            assert_eq!(name, "accent");
            let inner = fallback.expect("outer fallback");
            match *inner {
                TuiColor::Var {
                    name: inner_name,
                    fallback: inner_fb,
                } => {
                    assert_eq!(inner_name, "secondary");
                    assert_eq!(
                        inner_fb.as_deref(),
                        Some(&TuiColor::Literal(Color::Rgb(0, 0, 255)))
                    );
                }
                other => panic!("expected nested Var, got {other:?}"),
            }
        }
        other => panic!("expected Var, got {other:?}"),
    }
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
    let r = parse("a { border-color: var(--frame); }");
    assert!(r.warnings.is_empty());
    let v = r.stylesheet.rules()[0]
        .style
        .border_fg
        .clone()
        .expect("border_fg");
    match v {
        Value::Specified(TuiColor::Var { name, .. }) => assert_eq!(name, "frame"),
        other => panic!("expected Var, got {other:?}"),
    }
}
