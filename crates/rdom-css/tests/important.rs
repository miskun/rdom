//! §11.7 — `!important`. Trailing `!important` on any declaration
//! sets the matching bit on `TuiStyle::important`. The cascade
//! still uses ImportantMask exactly as the fluent builder
//! produces it; this test layer just verifies the parser routes
//! to the correct bit.

use rdom_css::parse;
use rdom_tui::style::{ImportantMask, Value};
use rdom_tui::{Color, TuiColor};

fn first_style(source: &str) -> rdom_tui::TuiStyle {
    let r = parse(source);
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
    r.stylesheet.rules()[0].style.clone()
}

#[test]
fn color_important_sets_fg_bit() {
    let s = first_style("a { color: red !important; }");
    assert_eq!(
        s.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
    assert!(s.important.contains(ImportantMask::FG));
}

#[test]
fn color_without_important_does_not_set_bit() {
    let s = first_style("a { color: red; }");
    assert!(!s.important.contains(ImportantMask::FG));
}

#[test]
fn background_important_sets_bg_bit() {
    let s = first_style("a { background-color: blue !important; }");
    assert!(s.important.contains(ImportantMask::BG));
    assert!(!s.important.contains(ImportantMask::FG));
}

#[test]
fn font_weight_important_sets_bold_bit() {
    let s = first_style("a { font-weight: bold !important; }");
    assert!(s.important.contains(ImportantMask::BOLD));
}

#[test]
fn display_important_sets_display_bit() {
    let s = first_style("a { display: none !important; }");
    assert!(s.important.contains(ImportantMask::DISPLAY));
}

#[test]
fn padding_important_sets_padding_bit() {
    let s = first_style("a { padding: 1 2 3 4 !important; }");
    assert!(s.important.contains(ImportantMask::PADDING));
}

#[test]
fn padding_side_important_sets_padding_bit() {
    let s = first_style("a { padding-top: 5 !important; }");
    assert!(s.important.contains(ImportantMask::PADDING));
}

#[test]
fn overflow_shorthand_important_sets_both_axes() {
    let s = first_style("a { overflow: hidden !important; }");
    assert!(s.important.contains(ImportantMask::OVERFLOW_X));
    assert!(s.important.contains(ImportantMask::OVERFLOW_Y));
}

#[test]
fn extra_whitespace_around_bang_important() {
    let s = first_style("a { color: red    !  important ; }");
    assert!(s.important.contains(ImportantMask::FG));
}

#[test]
fn case_insensitive_important_keyword() {
    let s = first_style("a { color: red !IMPORTANT; }");
    assert!(s.important.contains(ImportantMask::FG));
}

#[test]
fn multiple_declarations_each_track_separately() {
    let s = first_style("a { color: red !important; gap: 2; padding: 1 !important; }");
    assert!(s.important.contains(ImportantMask::FG));
    assert!(!s.important.contains(ImportantMask::GAP));
    assert!(s.important.contains(ImportantMask::PADDING));
}
