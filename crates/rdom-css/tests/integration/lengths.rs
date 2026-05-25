//! §11.8 — Lengths. Cell-based integers and `fr` shares are
//! supported (covered in `properties.rs`); other length units
//! (`px`, `em`, `rem`, `%`) are tokenized but produce
//! `WarningKind::InvalidValue` and the declaration is dropped.

use rdom_css::{WarningKind, parse};
use rdom_tui::layout::Size;
use rdom_tui::style::Value;

#[test]
fn width_with_pixel_unit_warns() {
    let r = parse("a { width: 5px; }");
    assert_eq!(r.warnings.len(), 1);
    match &r.warnings[0].kind {
        WarningKind::InvalidValue { property, value } => {
            assert_eq!(property, "width");
            assert!(value.contains("px"), "value: {value}");
        }
        other => panic!("expected InvalidValue, got {other:?}"),
    }
    // Declaration dropped — width unset.
    assert!(r.stylesheet.rules()[0].style.width.is_none());
}

#[test]
fn width_with_em_unit_warns() {
    let r = parse("a { width: 5em; }");
    assert_eq!(r.warnings.len(), 1);
    assert!(matches!(
        &r.warnings[0].kind,
        WarningKind::InvalidValue { property, .. } if property == "width"
    ));
}

#[test]
fn width_percentage_parses_to_size_percent() {
    // Pre-0.2.0, `width: 50%` was dropped as an unsupported unit
    // (one of the documented divergences). Fixed in 0.2.0 M2:
    // `%` is a *relative* unit (against parent dimensions),
    // semantically distinct from the pixel/font-size units we
    // legitimately can't represent on a cell grid. `Size::Percent`
    // resolves at layout time the same way `Size::Flex` does.
    let r = parse("a { width: 50%; }");
    assert!(
        r.warnings.is_empty(),
        "no warning expected; got {:?}",
        r.warnings
    );
    let rule = &r.stylesheet.rules()[0];
    assert_eq!(
        rule.style.width,
        Some(rdom_style::Value::Specified(
            rdom_style::layout::Size::Percent(50)
        ))
    );
}

#[test]
fn width_negative_warns() {
    // `width: -5` tokenizes as `Delim('-') Number(5)` — not a valid
    // Size value.
    let r = parse("a { width: -5; }");
    assert_eq!(r.warnings.len(), 1);
    assert!(matches!(
        &r.warnings[0].kind,
        WarningKind::InvalidValue { property, .. } if property == "width"
    ));
}

#[test]
fn gap_with_pixel_unit_warns() {
    let r = parse("a { gap: 5px; }");
    assert_eq!(r.warnings.len(), 1);
    assert!(matches!(
        &r.warnings[0].kind,
        WarningKind::InvalidValue { property, .. } if property == "gap"
    ));
}

#[test]
fn padding_with_pixel_unit_warns() {
    let r = parse("a { padding: 5px; }");
    assert_eq!(r.warnings.len(), 1);
    assert!(matches!(
        &r.warnings[0].kind,
        WarningKind::InvalidValue { property, .. } if property == "padding"
    ));
}

#[test]
fn other_declarations_in_same_rule_still_apply() {
    // Lenient: an invalid length doesn't break the rule.
    let r = parse("a { width: 5px; height: 3; }");
    assert_eq!(r.warnings.len(), 1);
    assert_eq!(
        r.stylesheet.rules()[0].style.height,
        Some(Value::Specified(Size::Fixed(3)))
    );
    assert!(r.stylesheet.rules()[0].style.width.is_none());
}
