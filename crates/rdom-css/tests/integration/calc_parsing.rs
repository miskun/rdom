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
fn calc_with_percent_carries_through_as_size_calc() {
    // M6 full: percent-bearing calc parses into Size::Calc and
    // resolves at layout time against the parent's matching-axis
    // dimension. The rule's `style.width` carries the AST.
    let sheet = rdom_css::from_css("div { width: calc(100% - 4); }");
    let rule = sheet
        .rules()
        .iter()
        .find(|r| r.source_text == "div")
        .expect("div rule parses");
    let width = rule.style.width.as_ref().expect("width is set");
    match width {
        rdom_style::Value::Specified(rdom_style::layout::Size::Calc(expr)) => {
            assert!(
                expr.contains_percent(),
                "the AST retains the percent operand"
            );
        }
        other => panic!("expected Size::Calc, got {other:?}"),
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

#[test]
fn calc_with_percent_in_padding_carries_through_as_padding_calc() {
    // CALC-PADMARG-1 closing test. Percent-bearing calc in a padding
    // side must parse into `PaddingValue::Calc` so layout can resolve
    // it against the containing-block width at layout time (CSS 2.1
    // §8.4 — padding percent uses CB width on all four sides).
    use rdom_style::layout::PaddingValue;
    let sheet = rdom_css::from_css("div { padding-top: calc(50% + 1); }");
    let rule = sheet
        .rules()
        .iter()
        .find(|r| r.source_text == "div")
        .expect("div rule parses");
    let padding = rule.style.padding.as_ref().expect("padding is set");
    match padding {
        rdom_style::Value::Specified(p) => match &p.top {
            PaddingValue::Calc(expr) => {
                assert!(expr.contains_percent(), "AST retains the percent operand");
            }
            other => panic!("expected PaddingValue::Calc, got {other:?}"),
        },
        other => panic!("expected Specified, got {other:?}"),
    }
}

#[test]
fn calc_with_percent_in_margin_carries_through_as_margin_calc() {
    // CALC-PADMARG-1 closing test for margin. CSS 2.1 §8.3 — margin
    // percent on ALL four sides resolves against the containing-block
    // width. The AST must reach layout.
    use rdom_style::layout::MarginValue;
    let sheet = rdom_css::from_css("div { margin-left: calc(25% - 2); }");
    let rule = sheet
        .rules()
        .iter()
        .find(|r| r.source_text == "div")
        .expect("div rule parses");
    let margin = rule.style.margin.as_ref().expect("margin is set");
    match margin {
        rdom_style::Value::Specified(m) => match &m.left {
            MarginValue::Calc(expr) => {
                assert!(expr.contains_percent(), "AST retains the percent operand");
            }
            other => panic!("expected MarginValue::Calc, got {other:?}"),
        },
        other => panic!("expected Specified, got {other:?}"),
    }
}

#[test]
fn calc_constant_in_padding_evaluates_to_cells() {
    // Constant calc (no percent) folds at parse time to a plain
    // `PaddingValue::Cells` — same policy as `Size::Fixed` for width.
    use rdom_style::layout::PaddingValue;
    let sheet = rdom_css::from_css("div { padding: calc(2 + 3); }");
    let rule = sheet
        .rules()
        .iter()
        .find(|r| r.source_text == "div")
        .expect("div rule parses");
    let padding = rule.style.padding.as_ref().expect("padding is set");
    match padding {
        rdom_style::Value::Specified(p) => {
            assert_eq!(p.top, PaddingValue::Cells(5));
            assert_eq!(p.left, PaddingValue::Cells(5));
        }
        other => panic!("expected Specified, got {other:?}"),
    }
}
