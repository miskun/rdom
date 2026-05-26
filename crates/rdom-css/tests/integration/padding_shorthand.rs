//! §11.4 — Padding shorthand. CSS padding accepts 1, 2, 3, or 4
//! values, expanding per the standard rule:
//!
//! - `padding: a`            → top=right=bottom=left=a
//! - `padding: a b`          → top=bottom=a, right=left=b
//! - `padding: a b c`        → top=a, right=left=b, bottom=c
//! - `padding: a b c d`      → top=a, right=b, bottom=c, left=d
//!
//! Plus the per-side longhands `padding-top` / `padding-right` /
//! `padding-bottom` / `padding-left`.

use rdom_css::parse;
use rdom_tui::layout::{Padding, PaddingValue};
use rdom_tui::style::Value;

fn padding_of(source: &str) -> Padding {
    let r = parse(source);
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
    let v = r.stylesheet.rules()[0]
        .style
        .padding
        .as_ref()
        .expect("padding declared")
        .clone();
    match v {
        Value::Specified(p) => p,
        _ => panic!("expected Specified, got {v:?}"),
    }
}

#[test]
fn padding_one_value_uniform() {
    let p = padding_of("a { padding: 5; }");
    assert_eq!(
        p,
        Padding {
            top: PaddingValue::Cells(5),
            right: PaddingValue::Cells(5),
            bottom: PaddingValue::Cells(5),
            left: PaddingValue::Cells(5)
        }
    );
}

#[test]
fn padding_two_values_vertical_horizontal() {
    let p = padding_of("a { padding: 1 2; }");
    assert_eq!(
        p,
        Padding {
            top: PaddingValue::Cells(1),
            right: PaddingValue::Cells(2),
            bottom: PaddingValue::Cells(1),
            left: PaddingValue::Cells(2)
        }
    );
}

#[test]
fn padding_three_values_top_horizontal_bottom() {
    let p = padding_of("a { padding: 1 2 3; }");
    assert_eq!(
        p,
        Padding {
            top: PaddingValue::Cells(1),
            right: PaddingValue::Cells(2),
            bottom: PaddingValue::Cells(3),
            left: PaddingValue::Cells(2)
        }
    );
}

#[test]
fn padding_four_values_clockwise() {
    let p = padding_of("a { padding: 1 2 3 4; }");
    assert_eq!(
        p,
        Padding {
            top: PaddingValue::Cells(1),
            right: PaddingValue::Cells(2),
            bottom: PaddingValue::Cells(3),
            left: PaddingValue::Cells(4)
        }
    );
}

#[test]
fn padding_top_longhand() {
    let p = padding_of("a { padding-top: 7; }");
    assert_eq!(
        p,
        Padding {
            top: PaddingValue::Cells(7),
            right: PaddingValue::Cells(0),
            bottom: PaddingValue::Cells(0),
            left: PaddingValue::Cells(0)
        }
    );
}

#[test]
fn padding_right_longhand() {
    let p = padding_of("a { padding-right: 7; }");
    assert_eq!(
        p,
        Padding {
            top: PaddingValue::Cells(0),
            right: PaddingValue::Cells(7),
            bottom: PaddingValue::Cells(0),
            left: PaddingValue::Cells(0)
        }
    );
}

#[test]
fn padding_bottom_longhand() {
    let p = padding_of("a { padding-bottom: 7; }");
    assert_eq!(
        p,
        Padding {
            top: PaddingValue::Cells(0),
            right: PaddingValue::Cells(0),
            bottom: PaddingValue::Cells(7),
            left: PaddingValue::Cells(0)
        }
    );
}

#[test]
fn padding_left_longhand() {
    let p = padding_of("a { padding-left: 7; }");
    assert_eq!(
        p,
        Padding {
            top: PaddingValue::Cells(0),
            right: PaddingValue::Cells(0),
            bottom: PaddingValue::Cells(0),
            left: PaddingValue::Cells(7)
        }
    );
}

#[test]
fn padding_shorthand_then_longhand_overrides() {
    // Cascade-within-block: declarations are applied in source order.
    // `padding: 1` sets all sides to 1; `padding-top: 9` then
    // overrides just the top.
    let p = padding_of("a { padding: 1; padding-top: 9; }");
    assert_eq!(
        p,
        Padding {
            top: PaddingValue::Cells(9),
            right: PaddingValue::Cells(1),
            bottom: PaddingValue::Cells(1),
            left: PaddingValue::Cells(1)
        }
    );
}

#[test]
fn padding_longhand_then_longhand_combines() {
    // Two different longhands set independent sides.
    let p = padding_of("a { padding-top: 2; padding-left: 4; }");
    assert_eq!(
        p,
        Padding {
            top: PaddingValue::Cells(2),
            right: PaddingValue::Cells(0),
            bottom: PaddingValue::Cells(0),
            left: PaddingValue::Cells(4)
        }
    );
}
