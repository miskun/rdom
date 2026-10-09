//! `ComputedStyle` tests.

use super::*;
use crate::Content;
use std::collections::HashMap;

#[test]
fn initial_is_safe_defaults() {
    let s = ComputedStyle::initial();
    assert_eq!(s.fg, Color::Reset);
    // CSS Backgrounds 3 §3.2: `background-color` starts `transparent`.
    assert_eq!(s.bg, Color::TRANSPARENT);
    assert_eq!(s.modifiers, Modifier::empty());
    assert_eq!(s.width, Size::Auto);
    assert_eq!(s.height, Size::Auto);
    // CSS Flexbox §5.1: `flex-direction`'s initial value is `row`.
    assert_eq!(s.direction, Direction::Row);
    assert_eq!(s.overflow_x, Overflow::Visible);
    assert_eq!(s.overflow_y, Overflow::Visible);
    assert_eq!(s.border, Border::none());
    assert_eq!(s.row_gap, crate::layout::GapValue::Normal);
    assert_eq!(s.column_gap, crate::layout::GapValue::Normal);
    assert_eq!(s.padding, Padding::default());
    assert_eq!(s.display, Display::Block);
    assert_eq!(s.flow, crate::layout::Flow::Block);
    assert!(!s.establishes_new_bfc);
    assert_eq!(s.text, crate::layout::TextStyle::default());
    assert_eq!(
        s.text.white_space(),
        Some(crate::layout::WhiteSpace::Normal)
    );
    assert_eq!(s.user_select, UserSelect::Auto);
    assert!(s.content.is_none());
    assert!(s.vars.is_empty());
}

#[test]
fn default_matches_initial() {
    assert_eq!(ComputedStyle::default(), ComputedStyle::initial());
}

#[test]
fn content_str_resolves_to_itself() {
    let vars = HashMap::new();
    assert_eq!(Content::Str("→".into()).resolve(&vars), Some("→".into()));
}

#[test]
fn content_var_looks_up() {
    let mut vars = HashMap::new();
    vars.insert("arrow".into(), "▾".into());
    assert_eq!(
        Content::Var("arrow".into()).resolve(&vars),
        Some("▾".into())
    );
}

#[test]
fn content_var_unresolved_empty_string() {
    let vars = HashMap::new();
    assert_eq!(
        Content::Var("nope".into()).resolve(&vars),
        Some(String::new())
    );
}

#[test]
fn content_concat_joins() {
    let mut vars = HashMap::new();
    vars.insert("x".into(), "BAR".into());
    let c = Content::Concat(vec![
        Content::Str("FOO ".into()),
        Content::Var("x".into()),
        Content::Str(" BAZ".into()),
    ]);
    assert_eq!(c.resolve(&vars), Some("FOO BAR BAZ".into()));
}

#[test]
fn content_none_returns_none() {
    let vars = HashMap::new();
    assert_eq!(Content::None.resolve(&vars), None);
}

#[test]
fn content_none_inside_concat_contributes_nothing() {
    let vars = HashMap::new();
    let c = Content::Concat(vec![
        Content::Str("A".into()),
        Content::None,
        Content::Str("B".into()),
    ]);
    assert_eq!(c.resolve(&vars), Some("AB".into()));
}

/// CSS Flexbox §5.1: `flex-direction` read back as one value.
#[test]
fn flex_direction_reads_axis_and_reverse_as_one() {
    use crate::layout::FlexDirection;
    let mut s = ComputedStyle::initial();
    assert_eq!(s.flex_direction(), FlexDirection::Row);
    s.direction = Direction::Column;
    s.flex_reverse = true;
    assert_eq!(s.flex_direction(), FlexDirection::ColumnReverse);
}

// ── C15G-STYLE-SIZE ─────────────────────────────────────────────────

/// Every element carries a `ComputedStyle` (and its `::before` / `::after`
/// one); the rarely set groups — effects, multi-column, anchor positioning,
/// the UI properties — are `Shared`, one pointer each, so their size is
/// paid once by the elements that set one. Tripwire: a new inline field
/// lands in a group, or raises this bound in review.
#[test]
fn computed_style_size_tripwire() {
    const MAX: usize = 2560;
    let size = std::mem::size_of::<ComputedStyle>();
    assert!(
        size <= MAX,
        "size_of::<ComputedStyle>() = {size}, bound {MAX}"
    );
}

/// A declaration block (`TuiStyle`) is held per rule and per inline style:
/// its rare groups are `Shared` as the computed ones are.
#[test]
fn tui_style_size_tripwire() {
    const MAX: usize = 3000;
    let size = std::mem::size_of::<crate::TuiStyle>();
    assert!(size <= MAX, "size_of::<TuiStyle>() = {size}, bound {MAX}");
}

/// An element that sets none of a rare group shares one default of it:
/// the initial styles hold the same group, and a write copies it first.
#[test]
fn the_initial_styles_share_their_rare_groups_until_written() {
    let a = ComputedStyle::initial();
    let mut b = ComputedStyle::initial();
    assert!(crate::Shared::ptr_eq(&a.effects, &b.effects));
    assert!(crate::Shared::ptr_eq(&a.multicol, &b.multicol));
    assert!(crate::Shared::ptr_eq(&a.anchor, &b.anchor));
    assert!(crate::Shared::ptr_eq(&a.ui, &b.ui));
    b.effects.isolation = crate::layout::Isolation::Isolate;
    assert!(!crate::Shared::ptr_eq(&a.effects, &b.effects));
    assert_eq!(a.effects.isolation, crate::layout::Isolation::Auto);
    assert_ne!(a, b);
    let d = crate::TuiStyle::new();
    let e = crate::TuiStyle::new();
    assert!(crate::Shared::ptr_eq(&d.effects, &e.effects));
    assert!(crate::Shared::ptr_eq(&d.masks, &e.masks));
}
