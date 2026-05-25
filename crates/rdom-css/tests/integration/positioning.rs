//! M2 §12.1 — rdom-css parses positioning properties.
//!
//! Covers `position` (static/relative/absolute/fixed; sticky
//! warns), `top` / `right` / `bottom` / `left` (integer cells,
//! `auto`, negative values), `z-index` (integer or `auto`), and
//! the `inset` shorthand (1/2/3/4-value forms with the same
//! clockwise expansion as `padding`, but accepting `auto` and
//! negative integers per CSS).

use rdom_css::parse;
use rdom_tui::layout::{Length, Position, ZIndex};
use rdom_tui::style::Value;

fn first_style(source: &str) -> rdom_tui::TuiStyle {
    let r = parse(source);
    assert!(
        r.warnings.is_empty(),
        "expected no warnings, got: {:?}",
        r.warnings
    );
    r.stylesheet.rules()[0].style.clone()
}

// ── position keyword ─────────────────────────────────────────────

#[test]
fn position_static() {
    let s = first_style("a { position: static; }");
    assert_eq!(s.position, Some(Value::Specified(Position::Static)));
}

#[test]
fn position_relative() {
    let s = first_style("a { position: relative; }");
    assert_eq!(s.position, Some(Value::Specified(Position::Relative)));
}

#[test]
fn position_absolute() {
    let s = first_style("a { position: absolute; }");
    assert_eq!(s.position, Some(Value::Specified(Position::Absolute)));
}

#[test]
fn position_fixed() {
    let s = first_style("a { position: fixed; }");
    assert_eq!(s.position, Some(Value::Specified(Position::Fixed)));
}

#[test]
fn position_sticky_parses_as_sticky() {
    // M5.4 lit up sticky positioning. The parser accepts the
    // keyword and writes `Position::Sticky`; the layout pass
    // applies the pin / post-stick rule against the nearest
    // scrollable ancestor.
    let s = first_style("a { position: sticky; }");
    assert_eq!(s.position, Some(Value::Specified(Position::Sticky)));
}

// ── top / right / bottom / left ──────────────────────────────────

#[test]
fn top_positive_integer() {
    let s = first_style("a { top: 5; }");
    assert_eq!(s.top, Some(Value::Specified(Length::Cells(5))));
}

#[test]
fn top_auto() {
    let s = first_style("a { top: auto; }");
    assert_eq!(s.top, Some(Value::Specified(Length::Auto)));
}

#[test]
fn top_negative_integer() {
    // Negative offsets are valid CSS — `top: -2` shifts the
    // element above its containing block's top edge.
    let s = first_style("a { top: -2; }");
    assert_eq!(s.top, Some(Value::Specified(Length::Cells(-2))));
}

#[test]
fn right_positive() {
    let s = first_style("a { right: 3; }");
    assert_eq!(s.right, Some(Value::Specified(Length::Cells(3))));
}

#[test]
fn bottom_auto() {
    let s = first_style("a { bottom: auto; }");
    assert_eq!(s.bottom, Some(Value::Specified(Length::Auto)));
}

#[test]
fn left_negative() {
    let s = first_style("a { left: -1; }");
    assert_eq!(s.left, Some(Value::Specified(Length::Cells(-1))));
}

// ── z-index ─────────────────────────────────────────────────────

#[test]
fn z_index_positive_integer() {
    let s = first_style("a { z-index: 5; }");
    assert_eq!(s.z_index, Some(Value::Specified(ZIndex::Value(5))));
}

#[test]
fn z_index_zero() {
    let s = first_style("a { z-index: 0; }");
    assert_eq!(s.z_index, Some(Value::Specified(ZIndex::Value(0))));
}

#[test]
fn z_index_negative() {
    let s = first_style("a { z-index: -1; }");
    assert_eq!(s.z_index, Some(Value::Specified(ZIndex::Value(-1))));
}

#[test]
fn z_index_auto() {
    let s = first_style("a { z-index: auto; }");
    assert_eq!(s.z_index, Some(Value::Specified(ZIndex::Auto)));
}

// ── inset shorthand ─────────────────────────────────────────────

fn inset_of(source: &str) -> (Length, Length, Length, Length) {
    let s = first_style(source);
    let to_l = |v: Option<Value<Length>>| match v {
        Some(Value::Specified(l)) => l,
        _ => panic!("expected Specified Length, got {v:?}"),
    };
    (to_l(s.top), to_l(s.right), to_l(s.bottom), to_l(s.left))
}

#[test]
fn inset_one_value_uniform() {
    let (t, r, b, l) = inset_of("a { inset: 5; }");
    assert_eq!(
        (t, r, b, l),
        (
            Length::Cells(5),
            Length::Cells(5),
            Length::Cells(5),
            Length::Cells(5)
        )
    );
}

#[test]
fn inset_two_values() {
    let (t, r, b, l) = inset_of("a { inset: 1 2; }");
    assert_eq!(
        (t, r, b, l),
        (
            Length::Cells(1),
            Length::Cells(2),
            Length::Cells(1),
            Length::Cells(2)
        )
    );
}

#[test]
fn inset_three_values() {
    let (t, r, b, l) = inset_of("a { inset: 1 2 3; }");
    assert_eq!(
        (t, r, b, l),
        (
            Length::Cells(1),
            Length::Cells(2),
            Length::Cells(3),
            Length::Cells(2)
        )
    );
}

#[test]
fn inset_four_values_clockwise() {
    let (t, r, b, l) = inset_of("a { inset: 1 2 3 4; }");
    assert_eq!(
        (t, r, b, l),
        (
            Length::Cells(1),
            Length::Cells(2),
            Length::Cells(3),
            Length::Cells(4)
        )
    );
}

#[test]
fn inset_with_auto_keyword() {
    let (t, r, b, l) = inset_of("a { inset: auto 5; }");
    assert_eq!(
        (t, r, b, l),
        (
            Length::Auto,
            Length::Cells(5),
            Length::Auto,
            Length::Cells(5)
        )
    );
}

#[test]
fn inset_with_negatives() {
    let (t, r, b, l) = inset_of("a { inset: -1 -2 -3 -4; }");
    assert_eq!(
        (t, r, b, l),
        (
            Length::Cells(-1),
            Length::Cells(-2),
            Length::Cells(-3),
            Length::Cells(-4)
        )
    );
}

#[test]
fn inset_zero_centered_pattern() {
    // Classic modal-centering trick: `position: absolute; inset: 0;
    // margin: auto`. M2 covers the inset half (margin: auto on
    // absolute is M5 follow-up, per the §11.1 Tier A list).
    let (t, r, b, l) = inset_of("a { inset: 0; }");
    assert_eq!(
        (t, r, b, l),
        (
            Length::Cells(0),
            Length::Cells(0),
            Length::Cells(0),
            Length::Cells(0)
        )
    );
}

// ── multi-property + cascade ─────────────────────────────────────

#[test]
fn position_with_all_offsets() {
    let s = first_style("a { position: absolute; top: 1; left: 2; z-index: 9; }");
    assert_eq!(s.position, Some(Value::Specified(Position::Absolute)));
    assert_eq!(s.top, Some(Value::Specified(Length::Cells(1))));
    assert_eq!(s.left, Some(Value::Specified(Length::Cells(2))));
    assert_eq!(s.z_index, Some(Value::Specified(ZIndex::Value(9))));
}

#[test]
fn longhand_after_inset_overrides() {
    // `inset: 1` writes all four sides; `top: 9` then overrides
    // just top — declarations apply in source order.
    let (t, r, b, l) = inset_of("a { inset: 1; top: 9; }");
    assert_eq!(
        (t, r, b, l),
        (
            Length::Cells(9),
            Length::Cells(1),
            Length::Cells(1),
            Length::Cells(1)
        )
    );
}
