//! Dispatch tests for the `animation-*` longhands and the `animation`
//! shorthand (C12-KEYFRAMES; CSS Animations 1 §4, CSS Animations 2 §3,
//! Scroll-driven Animations 1 §4.1 for `animation-timeline`'s keywords).

use super::*;
use crate::TuiStyle;

/// Set `name: value` and read it back.
fn round_trip(name: &str, value: &str) -> Option<String> {
    let mut style = TuiStyle::new();
    set(name, value, &mut style).ok()?;
    serialize(name, &style)
}

fn rejects(name: &str, value: &str) -> bool {
    set(name, value, &mut TuiStyle::new()).is_err()
}

/// CSS Animations 1 §4.1–§4.8: each longhand is a comma-separated list
/// of its single value; keywords are ASCII case-insensitive, a
/// `<keyframes-name>` keeps its case and may be a string.
#[test]
fn every_longhand_takes_a_list() {
    for (name, value, want) in [
        (
            "animation-name",
            "slide, none, \"a b\"",
            "slide, none, \"a b\"",
        ),
        ("animation-name", "Fade", "Fade"),
        (
            "animation-duration",
            "1s, 250ms, auto",
            "1000ms, 250ms, auto",
        ),
        (
            "animation-timing-function",
            "ease-in, steps(2)",
            "ease-in, steps(2, jump-end)",
        ),
        ("animation-delay", "-1s, 200ms", "-1000ms, 200ms"),
        (
            "animation-iteration-count",
            "infinite, 2.5, 0",
            "infinite, 2.5, 0",
        ),
        (
            "animation-direction",
            "normal, reverse, alternate, ALTERNATE-REVERSE",
            "normal, reverse, alternate, alternate-reverse",
        ),
        (
            "animation-fill-mode",
            "none, forwards, backwards, both",
            "none, forwards, backwards, both",
        ),
        ("animation-play-state", "running, paused", "running, paused"),
        (
            "animation-composition",
            "replace, add, accumulate",
            "replace, add, accumulate",
        ),
        ("animation-timeline", "auto, none", "auto, none"),
    ] {
        assert_eq!(
            round_trip(name, value).as_deref(),
            Some(want),
            "{name}: {value}"
        );
    }
}

/// The value ranges: durations and iteration counts are non-negative;
/// a `<custom-ident>` name excludes the CSS-wide keywords and `default`
/// (CSS Values 4 §4.2) — `none` is its own keyword.
#[test]
fn out_of_range_values_are_invalid() {
    for (name, value) in [
        ("animation-duration", "-1s"),
        ("animation-duration", "1"),
        ("animation-iteration-count", "-1"),
        ("animation-iteration-count", "1s"),
        ("animation-name", "default"),
        ("animation-name", "a b"),
        ("animation-name", "slide,"),
        ("animation-direction", "sideways"),
        ("animation-play-state", "running paused"),
    ] {
        assert!(rejects(name, value), "{name}: {value}");
    }
}

/// CSS Animations 1 §4.9: `animation` sets each piece's longhands in any
/// order — the first `<time>` is the duration, the second the delay — and
/// a keyword that could be another component's is that component's
/// unless it is already set; the reset-only longhands
/// (`animation-composition`, `animation-timeline`) go back to their
/// initial values.
#[test]
fn the_shorthand_sets_every_longhand() {
    let mut style = TuiStyle::new();
    set("animation-composition", "add", &mut style).unwrap();
    set(
        "animation",
        "slide 1s ease-in 200ms infinite alternate both paused, 2s fade",
        &mut style,
    )
    .unwrap();
    let get = |n: &str| serialize(n, &style);
    assert_eq!(get("animation-name").as_deref(), Some("slide, fade"));
    assert_eq!(get("animation-duration").as_deref(), Some("1000ms, 2000ms"));
    assert_eq!(
        get("animation-timing-function").as_deref(),
        Some("ease-in, ease")
    );
    assert_eq!(get("animation-delay").as_deref(), Some("200ms, 0ms"));
    assert_eq!(
        get("animation-iteration-count").as_deref(),
        Some("infinite, 1")
    );
    assert_eq!(
        get("animation-direction").as_deref(),
        Some("alternate, normal")
    );
    assert_eq!(get("animation-fill-mode").as_deref(), Some("both, none"));
    assert_eq!(
        get("animation-play-state").as_deref(),
        Some("paused, running")
    );
    assert_eq!(get("animation-composition").as_deref(), Some("replace"));
    assert_eq!(get("animation-timeline").as_deref(), Some("auto"));
    assert_eq!(
        get("animation").as_deref(),
        Some(
            "1000ms ease-in 200ms infinite alternate both paused slide, 2000ms ease 0ms 1 normal none running fade"
        )
    );
}

/// §4.9: "a keyword that is valid for more than one component is
/// assigned to the first unset one"; the name comes last — `ease ease`
/// is an easing and a name; `none` alone is the name.
#[test]
fn the_shorthand_resolves_keyword_ambiguity() {
    let mut style = TuiStyle::new();
    set("animation", "ease ease", &mut style).unwrap();
    assert_eq!(serialize("animation-name", &style).as_deref(), Some("ease"));
    assert_eq!(
        serialize("animation-timing-function", &style).as_deref(),
        Some("ease")
    );
    set("animation", "none", &mut style).unwrap();
    assert_eq!(serialize("animation-name", &style).as_deref(), Some("none"));
    assert!(rejects("animation", "slide fade"), "two names");
    assert!(rejects("animation", "1s 2s 3s"), "three times");
}

/// None of them inherits (CSS Animations 1 §4: "Inherited: no").
#[test]
fn no_animation_longhand_inherits() {
    for name in [
        "animation-name",
        "animation-duration",
        "animation-timing-function",
        "animation-delay",
        "animation-iteration-count",
        "animation-direction",
        "animation-fill-mode",
        "animation-play-state",
        "animation-composition",
        "animation-timeline",
    ] {
        assert!(!inherits(name), "{name}");
        assert!(property_names().contains(&name), "{name}");
    }
    assert!(property_names().contains(&"animation"));
}
