//! C12-TIMING: negative delays, `transitionrun`, and the before phase
//! (CSS Transitions 1 §2.4, §3, §6; CSS Easing 1 §2.3.1), on the
//! registry's explicit clock.

use std::time::{Duration, Instant};

use super::*;
use crate::layout::Size;
use crate::style::Stylesheet;
use crate::{CascadeExt, TuiDom};

/// `div { height: 0 → 10; transition: height 100ms linear <delay> }`,
/// changed at `start`: the registry and the div.
fn changed(delay: &str, timing: &str) -> (TuiDom, AnimationRegistry, NodeId, Instant) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = |h: u16| {
        rdom_css::parse(&format!(
            "div {{ height: {h}; transition: height 100ms {timing} {delay} }}"
        ))
        .stylesheet
    };
    let s0: Stylesheet = sheet(0);
    dom.cascade(&s0);
    let mut reg = AnimationRegistry::new();
    let start = Instant::now();
    diff_and_register(&mut dom, &mut reg, start);
    dom.cascade(&sheet(10));
    diff_and_register(&mut dom, &mut reg, start);
    (dom, reg, div, start)
}

fn height(dom: &TuiDom, div: NodeId) -> Size {
    dom.node(div)
        .ext()
        .unwrap()
        .computed
        .as_ref()
        .unwrap()
        .height
        .clone()
}

/// CSS Transitions 1 §2.4: a negative delay starts the transition part-way
/// — `-50ms` of `100ms` is half-way at once, and it ends 50ms later; its
/// `transitionrun` and `transitionstart` report the 50ms skipped (§6).
#[test]
fn a_negative_delay_starts_part_way() {
    let (mut dom, mut reg, div, start) = changed("-50ms", "linear");
    assert_eq!(reg.len(), 1);
    reg.advance(&mut dom, start);
    assert_eq!(height(&dom, div), Size::Fixed(5), "half-way at once");
    let events = reg.take_pending_events();
    let kinds: Vec<_> = events.iter().map(|e| (e.kind, e.elapsed_seconds)).collect();
    assert_eq!(
        kinds,
        [
            (TransitionEventKind::Run, 0.05),
            (TransitionEventKind::Start, 0.05)
        ]
    );
    reg.advance(&mut dom, start + Duration::from_millis(50));
    assert!(reg.is_empty(), "ended after the 50ms left");
    assert_eq!(height(&dom, div), Size::Fixed(10));
}

/// §3: a combined duration (`max(duration, 0) + delay`) of zero or less
/// starts no transition — the value changes at once.
#[test]
fn a_delay_past_the_duration_changes_at_once() {
    let (dom, reg, div, _) = changed("-100ms", "linear");
    assert!(reg.is_empty());
    assert_eq!(height(&dom, div), Size::Fixed(10));
}

/// §6: `transitionrun` fires when the transition is created — before its
/// delay — and `transitionstart` when the delay has passed.
#[test]
fn transitionrun_fires_before_the_delay() {
    let (mut dom, mut reg, _, start) = changed("40ms", "linear");
    reg.advance(&mut dom, start + Duration::from_millis(10));
    let first: Vec<_> = reg.take_pending_events().iter().map(|e| e.kind).collect();
    assert_eq!(first, [TransitionEventKind::Run]);
    reg.advance(&mut dom, start + Duration::from_millis(45));
    let second: Vec<_> = reg.take_pending_events().iter().map(|e| e.kind).collect();
    assert_eq!(second, [TransitionEventKind::Start]);
}

/// CSS Easing 1 §2.3.1: during the delay (the before phase) a
/// `jump-start` step has not jumped — the start value shows.
#[test]
fn a_jump_start_step_waits_out_the_delay() {
    let (mut dom, mut reg, div, start) = changed("40ms", "steps(2, jump-start)");
    reg.advance(&mut dom, start + Duration::from_millis(10));
    assert_eq!(
        height(&dom, div),
        Size::Fixed(0),
        "the start value in the delay"
    );
    reg.advance(&mut dom, start + Duration::from_millis(41));
    assert_eq!(
        height(&dom, div),
        Size::Fixed(5),
        "the first jump at the start"
    );
}

/// CSS Transitions 1 §6.1 (C12G-MISC): `transitioncancel`'s `elapsedTime`
/// is the transition's active time when cancelled — the delay not
/// counted, clamped to its duration: 0 inside the delay, 30 ms 30 ms past
/// it; a negative delay's skipped part counts.
#[test]
fn transitioncancel_reports_the_active_time() {
    let cancel_at = |delay: &str, ms: u64| {
        let (mut dom, mut reg, div, start) = changed(delay, "linear");
        let now = start + Duration::from_millis(ms);
        reg.advance(&mut dom, now);
        let _ = reg.take_pending_events();
        reg.cancel_for_node(div, now);
        let events = reg.take_pending_events();
        let cancel = events
            .iter()
            .find(|e| e.kind == TransitionEventKind::Cancel)
            .expect("transitioncancel");
        (cancel.elapsed_seconds * 1000.0).round()
    };
    assert_eq!(cancel_at("100ms", 50), 0.0, "in the delay");
    assert_eq!(cancel_at("100ms", 130), 30.0);
    assert_eq!(cancel_at("-40ms", 10), 50.0);
}

/// CSS Transitions 1 §3, reversing (C12G-MISC): a transition interrupted
/// by a change back to its start value runs back over the share of its
/// duration it had covered — `0 → 10` over 100 ms reversed at 30 ms
/// (eased output 0.3) takes 30 ms back to 0, not 100.
#[test]
fn a_reversed_transition_is_shortened() {
    let (mut dom, mut reg, div, start) = changed("0s", "linear");
    let at = |ms: u64| start + Duration::from_millis(ms);
    reg.advance(&mut dom, at(30));
    assert_eq!(height(&dom, div), Size::Fixed(3));
    let back = rdom_css::parse("div { height: 0; transition: height 100ms linear 0s }").stylesheet;
    dom.cascade(&back);
    diff_and_register(&mut dom, &mut reg, at(30));
    let _ = reg.take_pending_events();
    reg.advance(&mut dom, at(61));
    assert_eq!(height(&dom, div), Size::Fixed(0), "back in 30 ms");
    let ends = reg.take_pending_events();
    assert!(
        ends.iter()
            .any(|e| e.kind == TransitionEventKind::End && (e.elapsed_seconds - 0.03).abs() < 1e-3),
        "{ends:?}"
    );
}
