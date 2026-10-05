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

/// The four margins in cells, an undeclared side 0.
fn margin(s: &TuiStyle) -> [i16; 4] {
    s.margin.each().map(|v| match v {
        None => 0,
        Some(Value::Specified(MarginValue::Cells(n))) => *n,
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
    assert_eq!(
        s.padding.top,
        Some(Value::Specified(PaddingValue::Cells(3)))
    );
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
        let p = resolved(&s, d).padding;
        (p.left, p.right)
    };
    let cells = |n| Some(Value::Specified(PaddingValue::Cells(n)));
    assert_eq!(p(TextDirection::Ltr), (cells(1), cells(2)));
    assert_eq!(p(TextDirection::Rtl), (cells(2), cells(1)));

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

// ── CSSOM reads of the inline axis (C5G-CSSOM-LOGICAL) ─────────────

/// CSSOM §6.6 `getPropertyValue`: a longhand reads the value of the last
/// declaration that sets it — its own, or a shorthand's component
/// (`margin-inline: 1 2` sets `margin-inline-start` to `1`, CSS Logical 1
/// §4); a shorthand serializes from its longhands, and only when every
/// one of them is set.
#[test]
fn inline_axis_reads_expand_shorthands() {
    let s = style(&[("margin-inline", "1 2")]);
    assert_eq!(serialize("margin-inline-start", &s).as_deref(), Some("1"));
    assert_eq!(serialize("margin-inline-end", &s).as_deref(), Some("2"));
    assert_eq!(serialize("margin-inline", &s).as_deref(), Some("1 2"));

    let s = style(&[("margin-inline-start", "1"), ("margin-inline", "3")]);
    assert_eq!(serialize("margin-inline-start", &s).as_deref(), Some("3"));
    assert_eq!(serialize("margin-inline", &s).as_deref(), Some("3"));

    let s = style(&[("margin-inline", "3"), ("margin-inline-start", "1")]);
    assert_eq!(serialize("margin-inline-start", &s).as_deref(), Some("1"));
    assert_eq!(serialize("margin-inline", &s).as_deref(), Some("1 3"));

    let s = style(&[("padding-inline-end", "2")]);
    assert_eq!(serialize("padding-inline", &s), None);
    assert_eq!(serialize("padding-inline-start", &s), None);

    let s = style(&[("border-inline-color", "red blue")]);
    assert_eq!(
        serialize("border-inline-start-color", &s).as_deref(),
        Some("red")
    );
    assert_eq!(
        serialize("border-inline-end-color", &s).as_deref(),
        Some("blue")
    );
}

/// CSSOM §6.6 `getPropertyPriority`: a longhand's priority is its last
/// declaration's — a logical one's own `!important`, not the bits of
/// the physical sides it may land on; a shorthand is important when
/// every longhand is.
#[test]
fn inline_axis_priority_is_the_declarations_own() {
    let mut s = style(&[("margin-inline-start", "1")]);
    set_important("margin-inline-start", true, &mut s);
    set("margin-left", "2", &mut s).unwrap();
    assert!(is_important("margin-inline-start", &s));
    assert!(!is_important("margin-inline-end", &s));
    assert!(!is_important("margin-inline", &s));
    // C5G-LOGICAL-IMPORTANT: the physical declaration beside it keeps
    // its own (normal) priority.
    assert!(!is_important("margin-left", &s));

    let mut s = style(&[("margin-inline-start", "1"), ("margin-left", "2")]);
    set_important("margin-left", true, &mut s);
    assert!(!is_important("margin-inline-start", &s));
    assert!(is_important("margin-left", &s));

    let mut s = style(&[("margin-inline", "1 2")]);
    set_important("margin-inline", true, &mut s);
    assert!(is_important("margin-inline", &s));
    assert!(is_important("margin-inline-end", &s));
    set("margin-inline-end", "4", &mut s).unwrap();
    assert!(!is_important("margin-inline", &s));
    assert!(is_important("margin-inline-start", &s));
}

/// C5G-LOGICAL-IMPORTANT — CSS Cascade 4 §6.4: an important inline-axis
/// declaration marks only the side it maps to for the direction, and a
/// block's normal and important kept declarations replay apart, so an
/// important logical side beats a later normal physical one of its own
/// block.
#[test]
fn replayed_declarations_mark_their_own_side() {
    let mut s = style(&[("border-inline-start-color", "red")]);
    set_important("border-inline-start-color", true, &mut s);
    set("border-right-color", "blue", &mut s).unwrap();
    assert!(s.important.is_empty(), "no physical bit in the block");
    let split = |d| {
        let cx = SubstitutionContext::new().with_direction(d);
        s.substituted_pending_split(&HashMap::new(), &cx)
    };
    let [normal, important] = split(TextDirection::Ltr);
    assert!(
        important
            .important
            .contains(crate::ImportantMask::BORDER_LEFT_COLOR)
    );
    assert!(
        !important
            .important
            .contains(crate::ImportantMask::BORDER_RIGHT_COLOR)
    );
    assert!(normal.border_color.right.is_some() && important.border_color.right.is_none());
    let [_, important] = split(TextDirection::Rtl);
    assert!(
        important
            .important
            .contains(crate::ImportantMask::BORDER_RIGHT_COLOR)
    );
    assert!(
        !important
            .important
            .contains(crate::ImportantMask::BORDER_LEFT_COLOR)
    );
}
