//! Transition engine tests.
use super::*;
use crate::style::Stylesheet;
use crate::style::transition::AnimatableProperty;
use crate::style::transition::TransitionProperty;
use crate::{CascadeExt, TuiDom, TuiStyle};

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
/// in its own slot — the host's presentation stays untouched, the
/// event names the pseudo-element, and the override clears at the end.
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
                .transition_property(vec![TransitionProperty::Named(AnimatableProperty::Color)])
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
    let Some(Color::Rgb(r, _, b)) = ext.presentation_before.as_ref().and_then(|p| p.fg) else {
        panic!("no ::before override");
    };
    assert!((r as i16 - 128).abs() <= 2 && (b as i16 - 128).abs() <= 2);
    let events = reg.take_pending_events();
    assert!(
        events
            .iter()
            .any(|e| e.slot == StyleSlot::Before && e.kind == TransitionEventKind::Start)
    );

    reg.advance(&mut dom, start + Duration::from_millis(120));
    assert!(reg.is_empty());
    assert!(
        dom.node(div).ext().unwrap().presentation_before.is_none(),
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
            .transition_property(vec![TransitionProperty::Named(AnimatableProperty::Color)])
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
            .transition_property(vec![TransitionProperty::Named(AnimatableProperty::Color)])
            .transition_duration(vec![100])
            .transition_timing_function(vec![TimingFunction::Linear])
            .transition_delay(vec![0]),
    );
    dom.cascade(&s2);
    diff_and_register(&mut dom, &mut reg, start);

    // One animation registered for fg.
    assert_eq!(reg.len(), 1);

    // Advance 50ms — linear midpoint of red (255,0,0) →
    // blue (0,0,255) = (128, 0, 128) (within ±2 due to rounding).
    let mid = start + Duration::from_millis(50);
    reg.advance(&mut dom, mid);
    let pres_fg = dom
        .node(div)
        .ext()
        .unwrap()
        .presentation
        .as_ref()
        .unwrap()
        .fg
        .unwrap();
    match pres_fg {
        Color::Rgb(r, g, b) => {
            assert!((r as i16 - 128).abs() <= 2, "r = {r}");
            assert_eq!(g, 0);
            assert!((b as i16 - 128).abs() <= 2, "b = {b}");
        }
        other => panic!("expected Rgb, got {other:?}"),
    }

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
                .transition_property(vec![TransitionProperty::Named(AnimatableProperty::Color)])
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
        ending
            .iter()
            .any(|e| e.kind == TransitionEventKind::End && e.property == AnimatedProp::Fg),
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
                .transition_property(vec![TransitionProperty::Named(AnimatableProperty::Color)])
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
            .any(|e| e.kind == TransitionEventKind::Cancel && e.property == AnimatedProp::Fg)
    );
    // And there's exactly one animation now (the new red-ish→green).
    assert_eq!(reg.len(), 1);
}
