//! Transition engine tests.
use super::*;
use crate::style::Color;
use crate::style::Stylesheet;
use crate::style::transition::TimingFunction;
use crate::style::transition::TransitionProperty;
use crate::{CascadeExt, TuiDom, TuiStyle};
use std::time::Duration;

fn epoch() -> Instant {
    Instant::now()
}

// ── §15.10 — change without transition rule applies instantly

#[test]
fn property_change_without_transition_rule_applies_instantly() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    let s1 = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    dom.cascade(&s1);
    let mut reg = AnimationRegistry::new();
    let now = epoch();
    diff_and_register(&mut dom, &mut reg, now);

    // Switch to blue.
    let s2 = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(0, 0, 255)));
    dom.cascade(&s2);
    diff_and_register(&mut dom, &mut reg, now);

    // No transition rule → no animation registered.
    assert_eq!(reg.len(), 0);
    // Computed value committed immediately.
    assert_eq!(
        dom.node(div).ext().unwrap().computed.as_ref().unwrap().fg,
        Color::Rgb(0, 0, 255)
    );
}

/// `D-M3-3`: a `::before` with its own `transition: color` animates
/// in its own slot — the host's style stays untouched, the event names
/// the pseudo-element, and the record clears at the end.
#[test]
fn pseudo_element_color_transitions_in_its_own_slot() {
    use crate::ext::StyleSlot;
    use crate::style::Content;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = |fg: Color| {
        Stylesheet::bare().rule_unchecked(
            "div::before",
            TuiStyle::new()
                .content(Content::Str("*".into()))
                .fg(fg)
                .transition_property(vec![TransitionProperty::named("color")])
                .transition_duration(vec![100])
                .transition_timing_function(vec![TimingFunction::Linear]),
        )
    };
    dom.cascade(&sheet(Color::Rgb(255, 0, 0)));
    let mut reg = AnimationRegistry::new();
    let start = epoch();
    diff_and_register(&mut dom, &mut reg, start);
    dom.cascade(&sheet(Color::Rgb(0, 0, 255)));
    diff_and_register(&mut dom, &mut reg, start);
    assert_eq!(reg.len(), 1);

    reg.advance(&mut dom, start + Duration::from_millis(50));
    let ext = dom.node(div).ext().unwrap();
    assert!(ext.presentation.is_none(), "host slot untouched");
    let fg = ext.computed_for(StyleSlot::Before).map(|c| c.fg);
    // Red → blue at the midpoint, in gamma-encoded sRGB — the space
    // CSS Color 4 §12.1 requires for legacy sRGB colors (ACID-FIX-14).
    assert_eq!(
        fg,
        Some(Color::Rgb(128, 0, 128)),
        "the ::before's running value"
    );
    assert_eq!(
        ext.base_computed_for(StyleSlot::Before).map(|c| c.fg),
        Some(Color::Rgb(0, 0, 255)),
        "the cascade's style keeps the end value"
    );
    let events = reg.take_pending_events();
    assert!(
        events
            .iter()
            .any(|e| e.slot == StyleSlot::Before && e.kind == TransitionEventKind::Start)
    );

    reg.advance(&mut dom, start + Duration::from_millis(120));
    assert!(reg.is_empty());
    assert!(
        dom.node(div)
            .ext()
            .unwrap()
            .presentation_for(StyleSlot::Before)
            .is_none(),
        "the finished transition releases the override box"
    );
}

// ── §15.11 — transition: color 100ms interpolates ────────────

#[test]
fn transition_color_interpolates_at_midpoint() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    // Round 1: red, with the transition rule attached.
    let s1 = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .fg(Color::Rgb(255, 0, 0))
            .transition_property(vec![TransitionProperty::named("color")])
            .transition_duration(vec![100])
            .transition_timing_function(vec![TimingFunction::Linear])
            .transition_delay(vec![0]),
    );
    dom.cascade(&s1);
    let mut reg = AnimationRegistry::new();
    let start = epoch();
    diff_and_register(&mut dom, &mut reg, start);

    // Round 2: blue (with the same transition rule).
    let s2 = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .fg(Color::Rgb(0, 0, 255))
            .transition_property(vec![TransitionProperty::named("color")])
            .transition_duration(vec![100])
            .transition_timing_function(vec![TimingFunction::Linear])
            .transition_delay(vec![0]),
    );
    dom.cascade(&s2);
    diff_and_register(&mut dom, &mut reg, start);

    // One animation registered for fg.
    assert_eq!(reg.len(), 1);

    // Advance 50ms — the midpoint of red → blue, interpolated in
    // gamma-encoded sRGB (CSS Color 4 §12.1, legacy sRGB colors;
    // ACID-FIX-14): (128, 0, 128).
    let mid = start + Duration::from_millis(50);
    reg.advance(&mut dom, mid);
    // The running value is the computed value.
    let fg = dom.node(div).ext().unwrap().computed.as_ref().unwrap().fg;
    assert_eq!(fg, Color::Rgb(128, 0, 128));

    // Advance to end — animation retires, presentation cleared.
    let end = start + Duration::from_millis(120);
    reg.advance(&mut dom, end);
    assert!(reg.is_empty());
    assert!(dom.node(div).ext().unwrap().presentation.is_none());
}

#[test]
fn transitionend_event_queued_at_completion() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    let make_sheet = |c: Color| {
        Stylesheet::bare().rule_unchecked(
            "div",
            TuiStyle::new()
                .fg(c)
                .transition_property(vec![TransitionProperty::named("color")])
                .transition_duration(vec![100])
                .transition_timing_function(vec![TimingFunction::Linear])
                .transition_delay(vec![0]),
        )
    };
    dom.cascade(&make_sheet(Color::Rgb(255, 0, 0)));
    let mut reg = AnimationRegistry::new();
    let start = epoch();
    diff_and_register(&mut dom, &mut reg, start);
    dom.cascade(&make_sheet(Color::Rgb(0, 0, 255)));
    diff_and_register(&mut dom, &mut reg, start);

    // Drain initial events (transitionstart fires on first
    // tick where now ≥ started_at + delay).
    reg.advance(&mut dom, start + Duration::from_millis(0));
    let initial = reg.take_pending_events();
    assert!(initial.iter().any(|e| e.kind == TransitionEventKind::Start));

    // Advance to completion.
    reg.advance(&mut dom, start + Duration::from_millis(150));
    let ending = reg.take_pending_events();
    assert!(
        ending.iter().any(|e| e.kind == TransitionEventKind::End
            && e.property == Longhand::from_name("color").unwrap()),
        "expected transitionend; got {:?}",
        ending
    );
}

#[test]
fn re_setting_property_mid_flight_fires_cancel_and_restarts_from_current() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    let make_sheet = |c: Color| {
        Stylesheet::bare().rule_unchecked(
            "div",
            TuiStyle::new()
                .fg(c)
                .transition_property(vec![TransitionProperty::named("color")])
                .transition_duration(vec![100])
                .transition_timing_function(vec![TimingFunction::Linear])
                .transition_delay(vec![0]),
        )
    };
    dom.cascade(&make_sheet(Color::Rgb(255, 0, 0)));
    let mut reg = AnimationRegistry::new();
    let t0 = epoch();
    diff_and_register(&mut dom, &mut reg, t0);
    dom.cascade(&make_sheet(Color::Rgb(0, 0, 255))); // → blue
    diff_and_register(&mut dom, &mut reg, t0);

    // Halfway through the red→blue transit, retarget to green.
    let mid = t0 + Duration::from_millis(50);
    reg.advance(&mut dom, mid);
    let _ = reg.take_pending_events();
    dom.cascade(&make_sheet(Color::Rgb(0, 255, 0)));
    diff_and_register(&mut dom, &mut reg, mid);

    let after_retarget = reg.take_pending_events();
    // The retarget should have fired transitioncancel for fg.
    assert!(
        after_retarget
            .iter()
            .any(|e| e.kind == TransitionEventKind::Cancel
                && e.property == Longhand::from_name("color").unwrap())
    );
    // And there's exactly one animation now (the new red-ish→green).
    assert_eq!(reg.len(), 1);
}

/// CSS Color 4 §12.3: colors interpolate with premultiplied alpha, so
/// fading in from transparent black keeps the target's hue instead of
/// passing through gray.
#[test]
fn color_interpolation_premultiplies_alpha() {
    use rdom_style::animation::lerp_color;
    let mid = lerp_color(
        Color::Rgba(0, 0, 0, 0),
        Color::Rgb(255, 0, 0),
        0.5,
        Color::Reset,
    );
    assert_eq!(mid, Color::Rgba(255, 0, 0, 128));
    let half = lerp_color(
        Color::Rgba(0, 0, 255, 128),
        Color::Rgba(0, 0, 255, 128),
        0.3,
        Color::Reset,
    );
    assert_eq!(half, Color::Rgba(0, 0, 255, 128));
}

// ── A `reset` endpoint (C3G-SCHEME-CONSISTENCY) ────────────────────

/// A transition from the terminal's default color (`reset`) has no sRGB
/// endpoint; it starts from the canvas model of the element's used color
/// scheme for the property's role — the canvas background for
/// `background-color`, the canvas text for `color` — and interpolates in
/// sRGB like any other pair (CSS Color 4 §12.1; ACID-FIX-14). It used to
/// start from a fixed light gray in every scheme.
#[test]
fn reset_endpoint_interpolates_from_the_scheme_canvas_for_its_role() {
    use rdom_style::color::{ColorScheme, interpolate_srgb};
    let blue = Color::Rgb(0, 0, 255);
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let mut dom: TuiDom = TuiDom::new();
        dom.set_color_scheme(scheme);
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();
        let sheet = |c: Color| {
            Stylesheet::bare().rule_unchecked(
                "div",
                TuiStyle::new()
                    .fg(c)
                    .bg(c)
                    .transition_property(vec![
                        TransitionProperty::named("color"),
                        TransitionProperty::named("background-color"),
                    ])
                    .transition_duration(vec![100])
                    .transition_timing_function(vec![TimingFunction::Linear]),
            )
        };
        dom.cascade(&sheet(Color::Reset));
        let mut reg = AnimationRegistry::new();
        let start = epoch();
        diff_and_register(&mut dom, &mut reg, start);
        dom.cascade(&sheet(blue));
        diff_and_register(&mut dom, &mut reg, start);
        reg.advance(&mut dom, start + Duration::from_millis(50));
        let p = dom.node(div).ext().unwrap().computed.clone().unwrap();
        let (canvas_bg, canvas_fg) = scheme.canvas();
        assert_eq!(
            Some(p.bg),
            interpolate_srgb(canvas_bg, blue, 0.5),
            "{scheme:?} bg"
        );
        assert_eq!(
            Some(p.fg),
            interpolate_srgb(canvas_fg, blue, 0.5),
            "{scheme:?} fg"
        );
    }
}

/// C8-Z-INDEX — CSS Transitions 1 / CSS Values 4 §3.2: an `<integer>`
/// interpolates as a real number rounded to the nearest integer, over
/// the whole range — exactly, where an `f32` would lose the units.
#[test]
fn z_index_interpolates_over_the_full_integer_range() {
    use crate::layout::ZIndex;
    let z = |a: i32, b: i32, t: f64| {
        let (mut from, mut to) = (ComputedStyle::initial(), ComputedStyle::initial());
        from.z_index = ZIndex::Value(a);
        to.z_index = ZIndex::Value(b);
        let mut out = to.clone();
        Longhand::from_name("z-index").unwrap().interpolate(
            &from,
            &to,
            t,
            rdom_style::color::ColorScheme::Dark,
            &mut out,
        );
        match out.z_index {
            ZIndex::Value(n) => n,
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(z(0, 100_000, 0.5), 50_000);
    assert_eq!(z(2_000_000_001, 2_000_000_003, 0.5), 2_000_000_002);
    assert_eq!(z(i32::MIN, i32::MAX, 1.0), i32::MAX);
}

/// C12G-DETACHED: the registry alone (no `App`, no removal record) still
/// cancels the transition of an element that left the document — CSS
/// Transitions 1 §3, a transition of an element no longer rendered is
/// cancelled: `transitioncancel`, and nothing left to step.
#[test]
fn advancing_cancels_the_transition_of_a_detached_element() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = |c: Color| {
        Stylesheet::bare().rule_unchecked(
            "div",
            TuiStyle::new()
                .fg(c)
                .transition_property(vec![TransitionProperty::named("color")])
                .transition_duration(vec![100])
                .transition_timing_function(vec![TimingFunction::Linear]),
        )
    };
    dom.cascade(&sheet(Color::Rgb(255, 0, 0)));
    let mut reg = AnimationRegistry::new();
    let t0 = epoch();
    diff_and_register(&mut dom, &mut reg, t0);
    dom.cascade(&sheet(Color::Rgb(0, 0, 255)));
    diff_and_register(&mut dom, &mut reg, t0);
    reg.advance(&mut dom, t0 + Duration::from_millis(20));
    let _ = reg.take_pending_events();
    dom.remove_child(root, div).unwrap();
    let later = t0 + Duration::from_millis(40);
    reg.advance(&mut dom, later);
    let events = reg.take_pending_events();
    assert!(
        events.iter().any(|e| e.kind == TransitionEventKind::Cancel),
        "{events:?}"
    );
    assert!(reg.is_empty());
    assert!(!reg.needs_frames(later));
}
