//! `<progress>` + `<meter>` gauge tests — pure helpers.
//!
//! End-to-end paint tests that verify the gauge integration with
//! the cascade + paint pass live in
//! `render/paint_pass/tests.rs` (`gauge_*` group).

use crate::TuiDom;
use crate::runtime::builtins::gauge;
use crate::style::Color;

// ── progress ──────────────────────────────────────────────────────

#[test]
fn progress_with_value_fills_proportionally() {
    let mut dom: TuiDom = TuiDom::new();
    let p = dom.create_element("progress");
    dom.set_attribute(p, "value", "0.7").unwrap();
    dom.set_attribute(p, "max", "1").unwrap();
    let (bar, color) = gauge::gauge_text(&dom, p, 10).unwrap();
    assert_eq!(
        bar,
        "\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2591}\u{2591}\u{2591}"
    );
    assert!(color.is_none(), "progress uses cascade fg");
}

#[test]
fn progress_value_clamps_above_max() {
    let mut dom: TuiDom = TuiDom::new();
    let p = dom.create_element("progress");
    dom.set_attribute(p, "value", "5").unwrap();
    dom.set_attribute(p, "max", "1").unwrap();
    let (bar, _) = gauge::gauge_text(&dom, p, 4).unwrap();
    assert_eq!(bar, "\u{2588}\u{2588}\u{2588}\u{2588}");
}

#[test]
fn progress_value_clamps_below_zero() {
    let mut dom: TuiDom = TuiDom::new();
    let p = dom.create_element("progress");
    dom.set_attribute(p, "value", "-1").unwrap();
    dom.set_attribute(p, "max", "10").unwrap();
    let (bar, _) = gauge::gauge_text(&dom, p, 4).unwrap();
    assert_eq!(bar, "\u{2591}\u{2591}\u{2591}\u{2591}");
}

#[test]
fn progress_default_max_is_one() {
    // value=0.5, max omitted → defaults to 1.0, so half-filled.
    let mut dom: TuiDom = TuiDom::new();
    let p = dom.create_element("progress");
    dom.set_attribute(p, "value", "0.5").unwrap();
    let (bar, _) = gauge::gauge_text(&dom, p, 4).unwrap();
    assert_eq!(bar, "\u{2588}\u{2588}\u{2591}\u{2591}");
}

#[test]
fn progress_without_value_is_indeterminate_empty_track() {
    let mut dom: TuiDom = TuiDom::new();
    let p = dom.create_element("progress");
    dom.set_attribute(p, "max", "100").unwrap();
    let (bar, _) = gauge::gauge_text(&dom, p, 5).unwrap();
    assert_eq!(bar, "\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}");
}

#[test]
fn progress_zero_max_falls_back_to_default() {
    // max="0" is invalid (must be > 0) — fall back to default 1.0
    // so we don't divide by zero.
    let mut dom: TuiDom = TuiDom::new();
    let p = dom.create_element("progress");
    dom.set_attribute(p, "value", "0.5").unwrap();
    dom.set_attribute(p, "max", "0").unwrap();
    let (bar, _) = gauge::gauge_text(&dom, p, 4).unwrap();
    assert_eq!(bar, "\u{2588}\u{2588}\u{2591}\u{2591}");
}

// ── meter ─────────────────────────────────────────────────────────

#[test]
fn meter_uses_min_max_for_fill_ratio() {
    // value=50 in [0, 100] → half-filled.
    let mut dom: TuiDom = TuiDom::new();
    let m = dom.create_element("meter");
    dom.set_attribute(m, "min", "0").unwrap();
    dom.set_attribute(m, "max", "100").unwrap();
    dom.set_attribute(m, "value", "50").unwrap();
    let (bar, _) = gauge::gauge_text(&dom, m, 4).unwrap();
    assert_eq!(bar, "\u{2588}\u{2588}\u{2591}\u{2591}");
}

#[test]
fn meter_optimum_zone_uses_no_color_override() {
    // optimum=80 (high zone), value=85 (high zone) → optimum.
    let mut dom: TuiDom = TuiDom::new();
    let m = dom.create_element("meter");
    dom.set_attribute(m, "min", "0").unwrap();
    dom.set_attribute(m, "max", "100").unwrap();
    dom.set_attribute(m, "low", "33").unwrap();
    dom.set_attribute(m, "high", "66").unwrap();
    dom.set_attribute(m, "optimum", "80").unwrap();
    dom.set_attribute(m, "value", "85").unwrap();
    let (_, color) = gauge::gauge_text(&dom, m, 4).unwrap();
    assert_eq!(color, None);
}

#[test]
fn meter_suboptimal_zone_is_yellow() {
    // optimum=80 (high zone), value=50 (mid zone) → suboptimal.
    let mut dom: TuiDom = TuiDom::new();
    let m = dom.create_element("meter");
    dom.set_attribute(m, "min", "0").unwrap();
    dom.set_attribute(m, "max", "100").unwrap();
    dom.set_attribute(m, "low", "33").unwrap();
    dom.set_attribute(m, "high", "66").unwrap();
    dom.set_attribute(m, "optimum", "80").unwrap();
    dom.set_attribute(m, "value", "50").unwrap();
    let (_, color) = gauge::gauge_text(&dom, m, 4).unwrap();
    assert_eq!(color, Some(Color::Rgb(255, 255, 0)));
}

#[test]
fn meter_even_less_good_zone_is_red() {
    // optimum=80 (high zone), value=10 (low zone) → opposite extreme.
    let mut dom: TuiDom = TuiDom::new();
    let m = dom.create_element("meter");
    dom.set_attribute(m, "min", "0").unwrap();
    dom.set_attribute(m, "max", "100").unwrap();
    dom.set_attribute(m, "low", "33").unwrap();
    dom.set_attribute(m, "high", "66").unwrap();
    dom.set_attribute(m, "optimum", "80").unwrap();
    dom.set_attribute(m, "value", "10").unwrap();
    let (_, color) = gauge::gauge_text(&dom, m, 4).unwrap();
    assert_eq!(color, Some(Color::Rgb(255, 0, 0)));
}

#[test]
fn meter_value_clamps_to_min_max() {
    let mut dom: TuiDom = TuiDom::new();
    let m = dom.create_element("meter");
    dom.set_attribute(m, "min", "0").unwrap();
    dom.set_attribute(m, "max", "10").unwrap();
    dom.set_attribute(m, "value", "999").unwrap();
    let (bar, _) = gauge::gauge_text(&dom, m, 4).unwrap();
    assert_eq!(bar, "\u{2588}\u{2588}\u{2588}\u{2588}");
}

#[test]
fn meter_default_value_is_zero() {
    let mut dom: TuiDom = TuiDom::new();
    let m = dom.create_element("meter");
    let (bar, _) = gauge::gauge_text(&dom, m, 3).unwrap();
    assert_eq!(bar, "\u{2591}\u{2591}\u{2591}");
}

// ── is_gauge ──────────────────────────────────────────────────────

#[test]
fn is_gauge_recognizes_progress_and_meter_only() {
    let mut dom: TuiDom = TuiDom::new();
    let p = dom.create_element("progress");
    let m = dom.create_element("meter");
    let div = dom.create_element("div");
    assert!(gauge::is_gauge(&dom, p));
    assert!(gauge::is_gauge(&dom, m));
    assert!(!gauge::is_gauge(&dom, div));
}

// ── gauge_text returns None for non-gauges ────────────────────────

#[test]
fn gauge_text_returns_none_for_non_gauge_element() {
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    assert!(gauge::gauge_text(&dom, div, 10).is_none());
}
