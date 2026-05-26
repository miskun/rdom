//! §11.3 — Per-property parse. One test per property mapping row
//! in the spec, with a single representative value. The
//! shorthand expansion variants live in `padding_shorthand.rs`
//! (§11.4); the full color value matrix lives in `colors.rs`
//! (§11.5).

use rdom_css::{WarningKind, parse};
use rdom_tui::layout::{
    Border, Direction, Display, Overflow, Padding, PaddingValue, Size, UserSelect, WhiteSpace,
};
use rdom_tui::style::{Content, Value};
use rdom_tui::{Color, TuiColor};

fn first_style(source: &str) -> rdom_tui::TuiStyle {
    let r = parse(source);
    assert!(
        r.warnings.is_empty(),
        "expected no warnings, got: {:?}",
        r.warnings
    );
    assert_eq!(r.stylesheet.rules().len(), 1);
    r.stylesheet.rules()[0].style.clone()
}

// ── Color / modifiers ────────────────────────────────────────────

#[test]
fn color_named_red() {
    let s = first_style("a { color: red; }");
    assert_eq!(
        s.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
}

#[test]
fn background_color_named_blue() {
    let s = first_style("a { background-color: blue; }");
    assert_eq!(
        s.bg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(0, 0, 255))))
    );
}

#[test]
fn border_color_named_green() {
    let s = first_style("a { border-color: green; }");
    assert_eq!(
        s.border_fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(0, 128, 0))))
    );
}

#[test]
fn font_weight_bold() {
    let s = first_style("a { font-weight: bold; }");
    assert_eq!(s.bold, Some(Value::Specified(true)));
}

#[test]
fn font_weight_normal() {
    let s = first_style("a { font-weight: normal; }");
    assert_eq!(s.bold, Some(Value::Specified(false)));
}

#[test]
fn font_style_italic() {
    let s = first_style("a { font-style: italic; }");
    assert_eq!(s.italic, Some(Value::Specified(true)));
}

#[test]
fn text_decoration_underline() {
    let s = first_style("a { text-decoration: underline; }");
    assert_eq!(
        s.text_decoration,
        Some(Value::Specified(
            rdom_style::layout::TextDecoration::Underline
        ))
    );
}

#[test]
fn text_decoration_line_through() {
    let s = first_style("a { text-decoration: line-through; }");
    assert_eq!(
        s.text_decoration,
        Some(Value::Specified(
            rdom_style::layout::TextDecoration::LineThrough
        ))
    );
}

#[test]
fn text_decoration_none() {
    let s = first_style("a { text-decoration: none; }");
    assert_eq!(
        s.text_decoration,
        Some(Value::Specified(rdom_style::layout::TextDecoration::None))
    );
}

// ── Layout ───────────────────────────────────────────────────────

#[test]
fn display_block() {
    let s = first_style("a { display: block; }");
    assert_eq!(s.display, Some(Value::Specified(Display::Block)));
}

#[test]
fn display_inline() {
    let s = first_style("a { display: inline; }");
    assert_eq!(s.display, Some(Value::Specified(Display::Inline)));
}

#[test]
fn display_none() {
    let s = first_style("a { display: none; }");
    assert_eq!(s.display, Some(Value::Specified(Display::None)));
}

#[test]
fn flex_direction_row() {
    let s = first_style("a { flex-direction: row; }");
    assert_eq!(s.direction, Some(Value::Specified(Direction::Row)));
}

#[test]
fn flex_direction_column() {
    let s = first_style("a { flex-direction: column; }");
    assert_eq!(s.direction, Some(Value::Specified(Direction::Column)));
}

#[test]
fn gap_integer() {
    let s = first_style("a { gap: 2; }");
    assert_eq!(s.gap, Some(Value::Specified(2)));
}

#[test]
fn width_fixed_cells() {
    let s = first_style("a { width: 10; }");
    assert_eq!(s.width, Some(Value::Specified(Size::Fixed(10))));
}

#[test]
fn width_auto() {
    let s = first_style("a { width: auto; }");
    assert_eq!(s.width, Some(Value::Specified(Size::Auto)));
}

#[test]
fn width_fr_units() {
    let s = first_style("a { width: 1fr; }");
    assert_eq!(s.width, Some(Value::Specified(Size::Flex(1))));
}

#[test]
fn height_fixed_cells() {
    let s = first_style("a { height: 5; }");
    assert_eq!(s.height, Some(Value::Specified(Size::Fixed(5))));
}

#[test]
fn padding_single_value() {
    let s = first_style("a { padding: 1; }");
    assert_eq!(
        s.padding,
        Some(Value::Specified(Padding {
            top: PaddingValue::Cells(1),
            right: PaddingValue::Cells(1),
            bottom: PaddingValue::Cells(1),
            left: PaddingValue::Cells(1)
        }))
    );
}

#[test]
fn border_keyword_rounded() {
    let s = first_style("a { border: rounded; }");
    assert_eq!(s.border, Some(Value::Specified(Border::rounded())));
}

#[test]
fn overflow_hidden() {
    let s = first_style("a { overflow: hidden; }");
    assert_eq!(s.overflow_x, Some(Value::Specified(Overflow::Hidden)));
    assert_eq!(s.overflow_y, Some(Value::Specified(Overflow::Hidden)));
}

#[test]
fn overflow_x_only() {
    let s = first_style("a { overflow-x: scroll; }");
    assert_eq!(s.overflow_x, Some(Value::Specified(Overflow::Scroll)));
    assert_eq!(s.overflow_y, None);
}

#[test]
fn white_space_pre() {
    let s = first_style("a { white-space: pre; }");
    assert_eq!(s.white_space, Some(Value::Specified(WhiteSpace::Pre)));
}

#[test]
fn user_select_none() {
    let s = first_style("a { user-select: none; }");
    assert_eq!(s.user_select, Some(Value::Specified(UserSelect::None)));
}

// ── Pseudo-element content ────────────────────────────────────────

#[test]
fn content_string() {
    let s = first_style(r#"a { content: "hi"; }"#);
    assert_eq!(
        s.content,
        Some(Value::Specified(Content::Str("hi".to_string())))
    );
}

#[test]
fn content_attr() {
    let s = first_style("a { content: attr(placeholder); }");
    assert_eq!(
        s.content,
        Some(Value::Specified(Content::Attr("placeholder".to_string())))
    );
}

// ── Multiple declarations / robustness ─────────────────────────────

#[test]
fn multiple_declarations_in_one_rule() {
    let s = first_style("a { color: red; gap: 1; display: block; }");
    assert_eq!(
        s.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
    assert_eq!(s.gap, Some(Value::Specified(1)));
    assert_eq!(s.display, Some(Value::Specified(Display::Block)));
}

#[test]
fn whitespace_around_colon_and_semicolon() {
    let s = first_style("a {  color  :  red  ;  gap : 1 ;  }");
    assert_eq!(
        s.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
    assert_eq!(s.gap, Some(Value::Specified(1)));
}

#[test]
fn trailing_semicolon_optional() {
    let s = first_style("a { color: red }");
    assert_eq!(
        s.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
}

#[test]
fn unknown_property_emits_warning_and_skips() {
    let r = parse("a { unknown-prop: 5; color: red; }");
    assert_eq!(r.stylesheet.rules().len(), 1);
    let s = &r.stylesheet.rules()[0].style;
    // The good declaration still landed.
    assert_eq!(
        s.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
    // The unknown one warned.
    assert_eq!(r.warnings.len(), 1);
    match &r.warnings[0].kind {
        WarningKind::UnknownProperty(name) => assert_eq!(name, "unknown-prop"),
        other => panic!("expected UnknownProperty, got {other:?}"),
    }
}
