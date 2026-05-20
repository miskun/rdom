//! §11.11 — `parse_inline` declaration-list parser.
//!
//! Parses a declaration list with no surrounding `{...}` braces
//! into a `TuiStyle`. Used by the inline `style="…"` attribute
//! cascade rung (see `rdom_tui::cssom::seed_inline_styles` for the
//! tree-walking glue that calls this per element).

use rdom_css::{parse_inline, parse_inline_strict};
use rdom_style::layout::Display;
use rdom_style::{Color, ImportantMask, TuiColor, Value};

#[test]
fn parse_inline_single_declaration() {
    let r = parse_inline("color: red");
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
    assert_eq!(
        r.style.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
}

#[test]
fn parse_inline_multiple_declarations() {
    let r = parse_inline("color: red; display: block; gap: 2");
    assert!(r.warnings.is_empty());
    assert_eq!(
        r.style.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
    assert_eq!(r.style.display, Some(Value::Specified(Display::Block)));
    assert_eq!(r.style.gap, Some(Value::Specified(2)));
}

#[test]
fn parse_inline_trailing_semicolon_optional() {
    let r1 = parse_inline("color: red;");
    let r2 = parse_inline("color: red");
    assert_eq!(r1.style.fg, r2.style.fg);
}

#[test]
fn parse_inline_important_routes_to_mask() {
    let r = parse_inline("color: red !important");
    assert!(r.style.important.contains(ImportantMask::FG));
}

#[test]
fn parse_inline_unknown_property_warns() {
    let r = parse_inline("unknown-prop: 5; color: red");
    assert_eq!(r.warnings.len(), 1);
    assert_eq!(
        r.style.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
}

#[test]
fn parse_inline_strict_clean_input() {
    let s = parse_inline_strict("color: blue").expect("strict parse should succeed");
    assert_eq!(
        s.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(0, 0, 255))))
    );
}

#[test]
fn parse_inline_strict_unknown_errors() {
    let err = parse_inline_strict("nope: 5").expect_err("strict should error");
    let _ = err; // exact kind not asserted; covered in strict.rs
}
