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
use rdom_tui::layout::Padding;
use rdom_tui::style::Value;

fn padding_of(source: &str) -> Padding {
    let r = parse(source);
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
    let v = r.stylesheet.rules()[0]
        .style
        .padding
        .expect("padding declared");
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
            top: 5,
            right: 5,
            bottom: 5,
            left: 5
        }
    );
}

#[test]
fn padding_two_values_vertical_horizontal() {
    let p = padding_of("a { padding: 1 2; }");
    assert_eq!(
        p,
        Padding {
            top: 1,
            right: 2,
            bottom: 1,
            left: 2
        }
    );
}

#[test]
fn padding_three_values_top_horizontal_bottom() {
    let p = padding_of("a { padding: 1 2 3; }");
    assert_eq!(
        p,
        Padding {
            top: 1,
            right: 2,
            bottom: 3,
            left: 2
        }
    );
}

#[test]
fn padding_four_values_clockwise() {
    let p = padding_of("a { padding: 1 2 3 4; }");
    assert_eq!(
        p,
        Padding {
            top: 1,
            right: 2,
            bottom: 3,
            left: 4
        }
    );
}

#[test]
fn padding_top_longhand() {
    let p = padding_of("a { padding-top: 7; }");
    assert_eq!(
        p,
        Padding {
            top: 7,
            right: 0,
            bottom: 0,
            left: 0
        }
    );
}

#[test]
fn padding_right_longhand() {
    let p = padding_of("a { padding-right: 7; }");
    assert_eq!(
        p,
        Padding {
            top: 0,
            right: 7,
            bottom: 0,
            left: 0
        }
    );
}

#[test]
fn padding_bottom_longhand() {
    let p = padding_of("a { padding-bottom: 7; }");
    assert_eq!(
        p,
        Padding {
            top: 0,
            right: 0,
            bottom: 7,
            left: 0
        }
    );
}

#[test]
fn padding_left_longhand() {
    let p = padding_of("a { padding-left: 7; }");
    assert_eq!(
        p,
        Padding {
            top: 0,
            right: 0,
            bottom: 0,
            left: 7
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
            top: 9,
            right: 1,
            bottom: 1,
            left: 1
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
            top: 2,
            right: 0,
            bottom: 0,
            left: 4
        }
    );
}
