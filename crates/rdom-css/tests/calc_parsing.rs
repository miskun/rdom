//! M6 — `calc()` end-to-end through `rdom-css::from_css`.
//!
//! Pin that `calc(<expr>)` in CSS source strings parses correctly
//! for the properties M6 supports (constant-eval at parse time).
//! Percent-bearing calc is deferred to a future milestone; tests
//! here pin the parse-time constraint.

use rdom_style::layout::{Length, Size};

#[test]
fn calc_in_width_evaluates_to_constant() {
    let sheet = rdom_css::from_css("div { width: calc(2 + 3); }");
    let rule = sheet
        .rules()
        .iter()
        .find(|r| r.source_text == "div")
        .expect("div rule parses");
    assert_eq!(
        rule.style.width,
        Some(rdom_style::Value::Specified(Size::Fixed(5)))
    );
}

#[test]
fn calc_with_precedence_in_height() {
    let sheet = rdom_css::from_css("div { height: calc(2 + 3 * 4); }");
    let rule = sheet
        .rules()
        .iter()
        .find(|r| r.source_text == "div")
        .unwrap();
    assert_eq!(
        rule.style.height,
        Some(rdom_style::Value::Specified(Size::Fixed(14)))
    );
}

#[test]
fn calc_in_top_yields_signed_cells() {
    let sheet = rdom_css::from_css("div { top: calc(-3 * 2); }");
    let rule = sheet
        .rules()
        .iter()
        .find(|r| r.source_text == "div")
        .unwrap();
    assert_eq!(
        rule.style.top,
        Some(rdom_style::Value::Specified(Length::Cells(-6)))
    );
}

#[test]
fn calc_with_percent_drops_with_warning() {
    // Percent-bearing calc requires layout-time resolution which
    // M6 doesn't ship. The rule should not have width set; the
    // parser emits a warning that gets dropped in lenient mode.
    let sheet = rdom_css::from_css("div { width: calc(100% - 4); }");
    let rule = sheet.rules().iter().find(|r| r.source_text == "div");
    if let Some(rule) = rule {
        // Either no width set or set to None — either way, the
        // value didn't take effect.
        assert!(
            rule.style.width.is_none(),
            "percent-bearing calc must not produce a usable width in 0.2.0 — got {:?}",
            rule.style.width
        );
    }
}

#[test]
fn nested_calc_evaluates() {
    let sheet = rdom_css::from_css("div { width: calc(calc(2 + 3) * 4); }");
    let rule = sheet
        .rules()
        .iter()
        .find(|r| r.source_text == "div")
        .unwrap();
    assert_eq!(
        rule.style.width,
        Some(rdom_style::Value::Specified(Size::Fixed(20)))
    );
}

#[test]
fn calc_with_parens_evaluates() {
    let sheet = rdom_css::from_css("div { width: calc((2 + 3) * 4); }");
    let rule = sheet
        .rules()
        .iter()
        .find(|r| r.source_text == "div")
        .unwrap();
    assert_eq!(
        rule.style.width,
        Some(rdom_style::Value::Specified(Size::Fixed(20)))
    );
}
