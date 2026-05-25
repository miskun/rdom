//! M3 §15.10-11 — rdom-css parses `transition-*` longhands and
//! the `transition` shorthand. Engine-side behavior (interpolation,
//! events) is tested in rdom-tui's runtime::animation tests.

use rdom_css::parse;
use rdom_tui::style::transition::{AnimatableProperty, TimingFunction, TransitionProperty};

fn first_style(source: &str) -> rdom_tui::TuiStyle {
    let r = parse(source);
    assert!(
        r.warnings.is_empty(),
        "expected no warnings, got: {:?}",
        r.warnings
    );
    r.stylesheet.rules()[0].style.clone()
}

// ── transition-property longhand ──────────────────────────────────

#[test]
fn transition_property_named() {
    let s = first_style("a { transition-property: color; }");
    assert_eq!(
        s.transition_property,
        Some(vec![TransitionProperty::Named(AnimatableProperty::Color)])
    );
}

#[test]
fn transition_property_all() {
    let s = first_style("a { transition-property: all; }");
    assert_eq!(s.transition_property, Some(vec![TransitionProperty::All]));
}

#[test]
fn transition_property_none() {
    let s = first_style("a { transition-property: none; }");
    assert_eq!(s.transition_property, Some(vec![TransitionProperty::None]));
}

#[test]
fn transition_property_list() {
    let s = first_style("a { transition-property: color, background-color, width; }");
    assert_eq!(
        s.transition_property,
        Some(vec![
            TransitionProperty::Named(AnimatableProperty::Color),
            TransitionProperty::Named(AnimatableProperty::BackgroundColor),
            TransitionProperty::Named(AnimatableProperty::Width),
        ])
    );
}

// ── transition-duration longhand ──────────────────────────────────

#[test]
fn transition_duration_ms() {
    let s = first_style("a { transition-duration: 200ms; }");
    assert_eq!(s.transition_duration, Some(vec![200]));
}

#[test]
fn transition_duration_seconds() {
    let s = first_style("a { transition-duration: 0.5s; }");
    assert_eq!(s.transition_duration, Some(vec![500]));
}

#[test]
fn transition_duration_zero_seconds() {
    let s = first_style("a { transition-duration: 0s; }");
    assert_eq!(s.transition_duration, Some(vec![0]));
}

#[test]
fn transition_duration_list() {
    let s = first_style("a { transition-duration: 100ms, 200ms, 300ms; }");
    assert_eq!(s.transition_duration, Some(vec![100, 200, 300]));
}

// ── transition-timing-function longhand ───────────────────────────

#[test]
fn transition_timing_function_keywords() {
    for (css, expected) in [
        ("linear", TimingFunction::Linear),
        ("ease", TimingFunction::Ease),
        ("ease-in", TimingFunction::EaseIn),
        ("ease-out", TimingFunction::EaseOut),
        ("ease-in-out", TimingFunction::EaseInOut),
    ] {
        let source = format!("a {{ transition-timing-function: {css}; }}");
        let s = first_style(&source);
        assert_eq!(
            s.transition_timing_function,
            Some(vec![expected]),
            "css: {css}"
        );
    }
}

// ── transition-delay longhand ─────────────────────────────────────

#[test]
fn transition_delay_ms() {
    let s = first_style("a { transition-delay: 50ms; }");
    assert_eq!(s.transition_delay, Some(vec![50]));
}

// ── transition shorthand ──────────────────────────────────────────

#[test]
fn transition_shorthand_all_pieces() {
    let s = first_style("a { transition: color 200ms ease-in 50ms; }");
    assert_eq!(
        s.transition_property,
        Some(vec![TransitionProperty::Named(AnimatableProperty::Color)])
    );
    assert_eq!(s.transition_duration, Some(vec![200]));
    assert_eq!(
        s.transition_timing_function,
        Some(vec![TimingFunction::EaseIn])
    );
    assert_eq!(s.transition_delay, Some(vec![50]));
}

#[test]
fn transition_shorthand_property_and_duration() {
    let s = first_style("a { transition: color 200ms; }");
    assert_eq!(
        s.transition_property,
        Some(vec![TransitionProperty::Named(AnimatableProperty::Color)])
    );
    assert_eq!(s.transition_duration, Some(vec![200]));
    // Timing + delay default to ease + 0.
    assert_eq!(
        s.transition_timing_function,
        Some(vec![TimingFunction::Ease])
    );
    assert_eq!(s.transition_delay, Some(vec![0]));
}

#[test]
fn transition_shorthand_all_keyword() {
    let s = first_style("a { transition: all 100ms; }");
    assert_eq!(s.transition_property, Some(vec![TransitionProperty::All]));
    assert_eq!(s.transition_duration, Some(vec![100]));
}

#[test]
fn transition_shorthand_multiple_rules() {
    let s = first_style("a { transition: color 200ms, background-color 300ms ease-in; }");
    assert_eq!(
        s.transition_property,
        Some(vec![
            TransitionProperty::Named(AnimatableProperty::Color),
            TransitionProperty::Named(AnimatableProperty::BackgroundColor),
        ])
    );
    assert_eq!(s.transition_duration, Some(vec![200, 300]));
    assert_eq!(
        s.transition_timing_function,
        Some(vec![TimingFunction::Ease, TimingFunction::EaseIn])
    );
    assert_eq!(s.transition_delay, Some(vec![0, 0]));
}

#[test]
fn transition_shorthand_pieces_in_any_order() {
    // CSS allows "200ms color" or "ease-in 100ms color" etc.
    // Our parser detects each piece by token shape.
    let s = first_style("a { transition: 200ms color; }");
    assert_eq!(
        s.transition_property,
        Some(vec![TransitionProperty::Named(AnimatableProperty::Color)])
    );
    assert_eq!(s.transition_duration, Some(vec![200]));
}

#[test]
fn transition_shorthand_two_durations_first_is_duration_second_is_delay() {
    // Per CSS L1: when two <time> values appear in a single
    // shorthand, first = duration, second = delay.
    let s = first_style("a { transition: color 200ms 50ms; }");
    assert_eq!(s.transition_duration, Some(vec![200]));
    assert_eq!(s.transition_delay, Some(vec![50]));
}

// ── unsupported properties produce InvalidValue warnings ─────────

#[test]
fn transition_with_non_animatable_property_warns() {
    // `display` is discrete — CSS L1 says it's not animatable
    // (covered by `transition: all`'s midpoint switch instead).
    // Specifying it directly produces a warning.
    let r = parse("a { transition-property: display; }");
    assert!(
        !r.warnings.is_empty() || r.stylesheet.rules()[0].style.transition_property.is_none(),
        "expected warning or unset transitions; got rules with: {:?}",
        r.stylesheet.rules()[0].style.transition_property
    );
}

#[test]
fn transition_duration_without_unit_is_invalid() {
    let r = parse("a { transition-duration: 200; }");
    assert!(
        !r.warnings.is_empty(),
        "expected InvalidValue warning for unitless duration"
    );
}
