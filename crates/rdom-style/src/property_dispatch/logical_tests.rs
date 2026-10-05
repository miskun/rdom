//! Dispatch tests for the flow-relative properties (CSS Logical 1),
//! C5-LOGICAL: each maps onto its physical property in rdom's
//! `horizontal-tb` writing mode, the inline-axis ones by `direction`.

use std::collections::HashMap;

use super::*;
use crate::layout::{BorderStyle, MarginValue, PaddingValue, Size, TextDirection};
use crate::var::SubstitutionContext;
use crate::{TuiStyle, Value};

fn style(decls: &[(&str, &str)]) -> TuiStyle {
    let mut s = TuiStyle::new();
    for (name, value) in decls {
        set(name, value, &mut s).unwrap_or_else(|e| panic!("{name}: {value}: {e:?}"));
    }
    s
}

/// The block's declarations as the cascade applies them for an element
/// of `direction`: the block, then its replayed pending declarations.
fn resolved(s: &TuiStyle, direction: TextDirection) -> TuiStyle {
    let cx = SubstitutionContext::new().with_direction(direction);
    s.substituted(&HashMap::new(), &cx)
}

fn margin(s: &TuiStyle) -> [i16; 4] {
    let Some(Value::Specified(m)) = &s.margin else {
        panic!("no margin")
    };
    [&m.top, &m.right, &m.bottom, &m.left].map(|v| match v {
        MarginValue::Cells(n) => *n,
        other => panic!("{other:?}"),
    })
}

// ── block axis and sizes: one storage with the physical ones ───────

/// CSS Logical 1 §2–§4: in `horizontal-tb` the block axis is vertical
/// and the inline axis horizontal, so `inline-size` is `width`,
/// `margin-block-start` is `margin-top`, `border-block` both
/// `border-top` and `border-bottom` — written to the same storage, and
/// read back through either name.
#[test]
fn block_axis_and_size_properties_are_their_physical_twins() {
    let s = style(&[
        ("inline-size", "6"),
        ("max-block-size", "4"),
        ("margin-block", "1 2"),
        ("padding-block-start", "3"),
        ("inset-block-end", "5"),
        ("border-block", "solid"),
        ("border-block-end-style", "double"),
    ]);
    assert_eq!(s.width, Some(Value::Specified(Size::Fixed(6))));
    assert_eq!(serialize("max-height", &s).as_deref(), Some("4"));
    assert_eq!(margin(&s), [1, 0, 2, 0]);
    let Some(Value::Specified(p)) = &s.padding else {
        panic!()
    };
    assert_eq!(p.top, PaddingValue::Cells(3));
    assert_eq!(serialize("bottom", &s).as_deref(), Some("5"));
    assert_eq!(
        s.border_style.top,
        Some(Value::Specified(BorderStyle::Solid))
    );
    assert_eq!(
        s.border_style.bottom,
        Some(Value::Specified(BorderStyle::Double))
    );
    for (name, text) in [
        ("inline-size", "6"),
        ("block-size", "auto"),
        ("margin-block-start", "1"),
        ("margin-block", "1 2"),
        ("inset-block", "auto 5"),
        ("border-block-start-style", "solid"),
    ] {
        let s2 = style(&[(name, text)]);
        assert_eq!(serialize(name, &s2).as_deref(), Some(text), "{name}");
    }
}

/// CSS Logical 1 §4: a logical property and its physical twin are one
/// "logical property group" member — the later declaration wins.
#[test]
fn block_axis_and_physical_declarations_cascade_in_order() {
    assert_eq!(
        margin(&style(&[("margin-top", "1"), ("margin-block-start", "3")])),
        [3, 0, 0, 0]
    );
    assert_eq!(
        margin(&style(&[("margin-block-start", "3"), ("margin-top", "1")])),
        [1, 0, 0, 0]
    );
    let s = style(&[("height", "2"), ("block-size", "7")]);
    assert_eq!(s.height, Some(Value::Specified(Size::Fixed(7))));
}

// ── inline axis: mapped per element by `direction` ─────────────────

/// CSS Logical 1 §4 with CSS Writing Modes 4 §2.1: `margin-inline-start`
/// is `margin-left` under `ltr` and `margin-right` under `rtl`, so it is
/// kept as written and mapped when the element's direction is known;
/// CSSOM reads it back as written.
#[test]
fn inline_axis_properties_map_by_direction() {
    let s = style(&[("margin-inline-start", "4"), ("padding-inline", "1 2")]);
    assert_eq!(serialize("margin-inline-start", &s).as_deref(), Some("4"));
    assert_eq!(serialize("padding-inline", &s).as_deref(), Some("1 2"));
    assert_eq!(margin(&resolved(&s, TextDirection::Ltr)), [0, 0, 0, 4]);
    assert_eq!(margin(&resolved(&s, TextDirection::Rtl)), [0, 4, 0, 0]);
    let p = |d| {
        let Some(Value::Specified(p)) = resolved(&s, d).padding else {
            panic!()
        };
        (p.left, p.right)
    };
    assert_eq!(
        p(TextDirection::Ltr),
        (PaddingValue::Cells(1), PaddingValue::Cells(2))
    );
    assert_eq!(
        p(TextDirection::Rtl),
        (PaddingValue::Cells(2), PaddingValue::Cells(1))
    );

    let r = style(&[("border-start-end-radius", "1"), ("inset-inline-end", "3")]);
    let ltr = resolved(&r, TextDirection::Ltr);
    let rtl = resolved(&r, TextDirection::Rtl);
    assert!(ltr.border_radius.top_right.is_some() && ltr.border_radius.top_left.is_none());
    assert!(rtl.border_radius.top_left.is_some() && rtl.border_radius.top_right.is_none());
    assert_eq!(serialize("right", &ltr).as_deref(), Some("3"));
    assert_eq!(serialize("left", &rtl).as_deref(), Some("3"));
}

/// CSS Logical 1 §4: declaration order between an inline-axis property
/// and a physical one holds whichever side the logical one lands on —
/// and a longhand after its shorthand keeps the shorthand's other sides.
#[test]
fn inline_axis_and_physical_declarations_cascade_in_order() {
    let s = style(&[("margin-inline-start", "4"), ("margin-left", "1")]);
    assert_eq!(margin(&resolved(&s, TextDirection::Ltr)), [0, 0, 0, 1]);
    assert_eq!(margin(&resolved(&s, TextDirection::Rtl)), [0, 4, 0, 1]);
    let s = style(&[("margin-left", "1"), ("margin-inline-start", "4")]);
    assert_eq!(margin(&resolved(&s, TextDirection::Ltr)), [0, 0, 0, 4]);
    let s = style(&[("margin", "1"), ("margin-inline-end", "3")]);
    assert_eq!(margin(&resolved(&s, TextDirection::Ltr)), [1, 3, 1, 1]);
    assert_eq!(margin(&resolved(&s, TextDirection::Rtl)), [1, 1, 1, 3]);
}

/// The same keeps a `var()` longhand after its shorthand in one block
/// (CSS Variables 1 §3): `margin: 1; margin-left: var(--x)` is 1 on the
/// other sides (the replay used to start from an empty margin).
#[test]
fn a_substituted_longhand_keeps_its_shorthands_other_sides() {
    let s = style(&[("margin", "1"), ("margin-left", "var(--x)")]);
    let vars = HashMap::from([("x".to_string(), crate::CustomValue::new("5"))]);
    let out = s.substituted(&vars, &SubstitutionContext::new());
    assert_eq!(margin(&out), [1, 1, 1, 5]);
}

/// An inline-axis property's value is checked when declared: an invalid
/// one is dropped like any other (CSS Syntax 3 §5.4.4).
#[test]
fn inline_axis_values_are_validated_when_declared() {
    for (name, bad) in [
        ("margin-inline-start", "red"),
        ("padding-inline", "1 2 3"),
        ("border-inline-start-width", "fat"),
        ("inset-inline", "x"),
        ("border-end-end-radius", "-1"),
    ] {
        let mut s = TuiStyle::new();
        assert_eq!(
            set(name, bad, &mut s),
            Err(DispatchError::InvalidValue),
            "{name}: {bad}"
        );
        assert!(!s.has_pending(), "{name}");
    }
}
