//! C12-TIMING: the easing functions of CSS Easing 1 / 2 and the
//! transition value lists of CSS Transitions 1 §2.

use super::{LinearStop, StepPosition, TimingFunction};
use crate::parse::token::tokenize;
use crate::parse::values::{
    parse_time_list, parse_timing_function_list, parse_transition_property_list,
    parse_transition_shorthand,
};

fn easing(src: &str) -> Option<TimingFunction> {
    parse_timing_function_list(&tokenize(src).unwrap()).map(|mut v| v.remove(0))
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

/// CSS Easing 2 §2.1: `linear()` stops — a missing first / last input is
/// 0% / 100%, missing inputs between known ones spread evenly, an input
/// below an earlier one is raised to it, and a stop with two percentages
/// is two stops (a hold).
#[test]
fn linear_stops_canonicalize() {
    let stops = |src: &str| match easing(src) {
        Some(TimingFunction::LinearStops(s)) => {
            s.iter().map(|s| (s.input, s.output)).collect::<Vec<_>>()
        }
        other => panic!("{src}: {other:?}"),
    };
    assert_eq!(
        stops("linear(0, 0.25 75%, 1)"),
        [(0.0, 0.0), (0.75, 0.25), (1.0, 1.0)]
    );
    assert_eq!(
        stops("linear(0, 0.5, 1)"),
        [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)]
    );
    assert_eq!(
        stops("linear(0, 0.5 25% 75%, 1)"),
        [(0.0, 0.0), (0.25, 0.5), (0.75, 0.5), (1.0, 1.0)]
    );
    assert_eq!(
        stops("linear(0 50%, 1 20%)"),
        [(0.5, 0.0), (0.5, 1.0)],
        "an input below the largest before it is raised to it"
    );
    assert_eq!(easing("linear(1)"), None, "fewer than two stops");
    assert_eq!(easing("linear()"), None);
    assert_eq!(easing("linear(0, 1 2 3)"), None);
    assert!(matches!(easing("linear"), Some(TimingFunction::Linear)));
}

/// CSS Easing 2 §2.1.2: the output is the piecewise-linear function of
/// the points; an input matching several points takes the last one's.
#[test]
fn linear_easing_interpolates_between_stops() {
    let f = easing("linear(0, 0.25 75%, 1)").unwrap();
    assert!(close(f.ease(0.375), 0.125));
    assert!(close(f.ease(0.875), 0.625));
    assert!(close(f.ease(1.0), 1.0));
    let hold = easing("linear(0, 0.5 25% 75%, 1)").unwrap();
    assert!(close(hold.ease(0.5), 0.5), "the hold");
    let jump = easing("linear(0, 0 50%, 1 50%, 1)").unwrap();
    assert!(close(jump.ease(0.5), 1.0), "the last point at an input");
    assert!(close(jump.ease(0.25), 0.0));
    assert!(close(jump.ease(0.75), 1.0));
}

/// CSS Easing 1 §2.3.1: in the before phase (a transition's delay) the
/// "before flag" takes a `jump-start` step back, so the start value shows.
#[test]
fn steps_take_the_before_flag() {
    let s = TimingFunction::Steps {
        count: 2,
        position: StepPosition::Start,
    };
    assert_eq!(s.ease(0.0), 0.5, "active phase at 0: the first jump");
    assert_eq!(s.ease_before(0.0), 0.0, "before phase at 0: none yet");
    assert_eq!(TimingFunction::Linear.ease_before(0.0), 0.0);
}

/// CSS Transitions 1 §2.4: a negative `transition-delay` is valid (the
/// transition starts part-way); a negative duration is not.
#[test]
fn delays_may_be_negative_durations_not() {
    let t = |s: &str| tokenize(s).unwrap();
    assert_eq!(parse_time_list(&t("-500ms, 1s")), Some(vec![-500, 1000]));
    let rules = parse_transition_shorthand(&t("width 1s ease -0.5s")).unwrap();
    assert_eq!(rules[0].delay(), -500);
    assert_eq!(rules[0].duration(), 1000);
    assert_eq!(
        parse_transition_shorthand(&t("width -1s")),
        None,
        "negative duration"
    );
}

/// CSS Transitions 1 §2.1, §2.5: `none` is valid only alone, in the
/// longhand and the shorthand.
#[test]
fn none_is_only_valid_alone() {
    let t = |s: &str| tokenize(s).unwrap();
    assert!(parse_transition_property_list(&t("none")).is_some());
    assert_eq!(parse_transition_property_list(&t("none, color")), None);
    assert!(parse_transition_shorthand(&t("none 1s")).is_some());
    assert_eq!(parse_transition_shorthand(&t("color 1s, none 2s")), None);
}

/// `LinearStop` is a plain record.
#[test]
fn linear_stop_builds() {
    let s = LinearStop::new(0.5, 0.25);
    assert_eq!((s.input, s.output), (0.5, 0.25));
}
