//! `Longhand::add` — the addition of computed values (Web Animations 1
//! §5.4.4, CSS Values 4 §3 "Combining Values"), which an animation's
//! `add` / `accumulate` composite operation applies to the underlying
//! value (CSS Animations 2 §3.2).

use super::*;
use crate::color::ColorScheme;
use crate::layout::{Length, Size};

fn longhand(name: &str) -> Longhand {
    Longhand::from_name(name).unwrap()
}

/// CSS Values 4 §3.1: lengths add — whole cells stay cells, a
/// percentage keeps its own term.
#[test]
fn lengths_add() {
    let mut a = ComputedStyle::initial();
    let mut b = ComputedStyle::initial();
    a.width = Size::Fixed(2);
    b.width = Size::Fixed(3);
    let mut out = ComputedStyle::initial();
    assert!(longhand("width").add(&a, &b, ColorScheme::Dark, &mut out));
    assert_eq!(out.width, Size::Fixed(5));
    a.left = Length::Cells(-1);
    b.left = Length::Cells(4);
    assert!(longhand("left").add(&a, &b, ColorScheme::Dark, &mut out));
    assert_eq!(out.left, Length::Cells(3));
}

/// Numbers add (`opacity` clamped to its range afterwards, CSS Color 4
/// §13); an integer as an integer.
#[test]
fn numbers_add_within_their_range() {
    let mut a = ComputedStyle::initial();
    let mut b = ComputedStyle::initial();
    a.opacity = 0.75;
    b.opacity = 0.5;
    let mut out = ComputedStyle::initial();
    assert!(longhand("opacity").add(&a, &b, ColorScheme::Dark, &mut out));
    assert_eq!(out.opacity, 1.0);
    a.z_index = crate::layout::ZIndex::Value(2);
    b.z_index = crate::layout::ZIndex::Value(3);
    assert!(longhand("z-index").add(&a, &b, ColorScheme::Dark, &mut out));
    assert_eq!(out.z_index, crate::layout::ZIndex::Value(5));
}

/// CSS Color 4 (Web Animations 1 §5.4.4's color addition): the channels
/// add, clamped.
#[test]
fn colors_add_by_channel() {
    let mut a = ComputedStyle::initial();
    let mut b = ComputedStyle::initial();
    a.bg = crate::Color::Rgb(100, 20, 200);
    b.bg = crate::Color::Rgb(100, 30, 100);
    let mut out = ComputedStyle::initial();
    assert!(longhand("background-color").add(&a, &b, ColorScheme::Dark, &mut out));
    assert_eq!(out.bg, crate::Color::Rgb(200, 50, 255));
}

/// Web Animations 1 §5.4.4: a discrete value has no addition — the
/// composite replaces it.
#[test]
fn a_discrete_value_does_not_add() {
    let a = ComputedStyle::initial();
    let b = ComputedStyle::initial();
    let mut out = ComputedStyle::initial();
    assert!(!longhand("display").add(&a, &b, ColorScheme::Dark, &mut out));
    assert!(!longhand("position").add(&a, &b, ColorScheme::Dark, &mut out));
}
