//! Dispatch tests for the scroll-driven animation properties
//! (C12-SCROLL-DRIVEN; Scroll-driven Animations 1 §2–§4, CSS Animations 2
//! §3.7 for `animation-timeline`).

use super::*;
use crate::TuiStyle;

fn round_trip(name: &str, value: &str) -> Option<String> {
    let mut style = TuiStyle::new();
    set(name, value, &mut style).ok()?;
    serialize(name, &style)
}

fn rejects(name: &str, value: &str) -> bool {
    set(name, value, &mut TuiStyle::new()).is_err()
}

/// §2.2 / §3.2: the timeline names are `none` or `<dashed-ident>`s, the
/// axes `block | inline | x | y`, the insets `[auto | <length-percentage>]{1,2}`
/// per timeline; §4.2 `timeline-scope: none | all | <dashed-ident>#`.
#[test]
fn the_timeline_longhands_take_their_lists() {
    for (name, value, want) in [
        ("scroll-timeline-name", "--a, none, --B", "--a, none, --B"),
        (
            "scroll-timeline-axis",
            "block, INLINE, x, y",
            "block, inline, x, y",
        ),
        ("view-timeline-name", "--v", "--v"),
        ("view-timeline-axis", "y", "y"),
        ("view-timeline-inset", "auto, 1 2, 10%", "auto, 1 2, 10%"),
        ("timeline-scope", "--a, --b", "--a, --b"),
        ("timeline-scope", "all", "all"),
        ("timeline-scope", "none", "none"),
    ] {
        assert_eq!(
            round_trip(name, value).as_deref(),
            Some(want),
            "{name}: {value}"
        );
    }
    for (name, value) in [
        ("scroll-timeline-name", "a"),
        ("scroll-timeline-axis", "z"),
        ("view-timeline-inset", "1 2 3"),
        ("timeline-scope", "none, --a"),
        ("timeline-scope", "auto"),
    ] {
        assert!(rejects(name, value), "{name}: {value}");
    }
}

/// §2.3 / §3.3: `scroll-timeline` and `view-timeline` take a name, then
/// optionally an axis (and, for a view timeline, insets), per timeline.
#[test]
fn the_timeline_shorthands_set_their_longhands() {
    let mut style = TuiStyle::new();
    set("scroll-timeline", "--a x, --b", &mut style).unwrap();
    assert_eq!(
        serialize("scroll-timeline-name", &style).as_deref(),
        Some("--a, --b")
    );
    assert_eq!(
        serialize("scroll-timeline-axis", &style).as_deref(),
        Some("x, block")
    );
    assert_eq!(
        serialize("scroll-timeline", &style).as_deref(),
        Some("--a x, --b")
    );
    set("view-timeline", "--v inline 1 auto", &mut style).unwrap();
    assert_eq!(
        serialize("view-timeline-axis", &style).as_deref(),
        Some("inline")
    );
    assert_eq!(
        serialize("view-timeline-inset", &style).as_deref(),
        Some("1 auto")
    );
    assert!(rejects("scroll-timeline", "x --a"), "the name comes first");
}

/// CSS Animations 2 §3.7, Scroll-driven Animations 1 §2.1 / §3.1:
/// `animation-timeline` takes a timeline name, `scroll()` with a scroller
/// and an axis in either order, and `view()` with an axis and insets.
#[test]
fn animation_timeline_takes_scroll_view_and_names() {
    for (value, want) in [
        ("--a", "--a"),
        ("scroll()", "scroll()"),
        ("scroll(root)", "scroll(root)"),
        ("scroll(x self)", "scroll(self x)"),
        ("scroll(nearest block)", "scroll()"),
        ("view()", "view()"),
        ("view(inline 2 1)", "view(inline 2 1)"),
        ("view(auto)", "view()"),
        ("auto, none, --a", "auto, none, --a"),
    ] {
        assert_eq!(
            round_trip("animation-timeline", value).as_deref(),
            Some(want),
            "{value}"
        );
    }
    for value in ["scroll(root root)", "view(1 2 3)", "scroll(", "a"] {
        assert!(rejects("animation-timeline", value), "{value}");
    }
}

/// Scroll-driven Animations 1 §4.3: a range boundary is `normal`, a
/// `<length-percentage>`, or a `<timeline-range-name>` with an optional
/// one — the name alone is 0% at the start, 100% at the end.
#[test]
fn animation_range_takes_names_and_offsets() {
    for (name, value, want) in [
        ("animation-range-start", "normal", "normal"),
        ("animation-range-start", "entry", "entry 0%"),
        ("animation-range-start", "contain 20%", "contain 20%"),
        ("animation-range-start", "10%, 3", "10%, 3"),
        ("animation-range-end", "exit", "exit 100%"),
        (
            "animation-range-end",
            "exit-crossing 50%",
            "exit-crossing 50%",
        ),
        ("animation-range-end", "cover", "cover 100%"),
    ] {
        assert_eq!(
            round_trip(name, value).as_deref(),
            Some(want),
            "{name}: {value}"
        );
    }
    for (name, value) in [
        ("animation-range-start", "auto"),
        ("animation-range-start", "sideways 10%"),
        ("animation-range-end", "entry 10% 20%"),
    ] {
        assert!(rejects(name, value), "{name}: {value}");
    }
}

/// §4.3: `animation-range` sets both boundaries; a start with a range
/// name and no end ends at that range's 100%; otherwise the omitted end
/// is `normal`.
#[test]
fn the_range_shorthand_fills_the_end_from_the_start() {
    let mut style = TuiStyle::new();
    set(
        "animation-range",
        "entry 10% exit 90%, contain, 20%",
        &mut style,
    )
    .unwrap();
    assert_eq!(
        serialize("animation-range-start", &style).as_deref(),
        Some("entry 10%, contain 0%, 20%")
    );
    assert_eq!(
        serialize("animation-range-end", &style).as_deref(),
        Some("exit 90%, contain 100%, normal")
    );
}

/// CSS Animations 2 §3.9: `animation` resets the range longhands too.
#[test]
fn the_animation_shorthand_resets_the_range() {
    let mut style = TuiStyle::new();
    set("animation-range", "entry", &mut style).unwrap();
    set("animation", "a 1s", &mut style).unwrap();
    assert_eq!(
        serialize("animation-range-start", &style).as_deref(),
        Some("normal")
    );
    assert_eq!(
        serialize("animation-range-end", &style).as_deref(),
        Some("normal")
    );
}

/// None of them inherits.
#[test]
fn no_timeline_property_inherits() {
    for name in [
        "scroll-timeline-name",
        "scroll-timeline-axis",
        "view-timeline-name",
        "view-timeline-axis",
        "view-timeline-inset",
        "timeline-scope",
        "animation-range-start",
        "animation-range-end",
    ] {
        assert!(!inherits(name), "{name}");
        assert!(property_names().contains(&name), "{name}");
    }
}
