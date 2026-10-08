//! The conditional rules' grammar and logic: Media Queries 4 §2–§3 and
//! the three-valued evaluation (§3.1).

use super::*;
use crate::calc::Viewport;
use crate::color::ColorScheme;

fn env(cols: u16, rows: u16) -> MediaEnvironment {
    MediaEnvironment::new(
        Viewport::new(cols, rows),
        ColorScheme::Dark,
        MediaPreferences::default(),
    )
}

fn matches(query: &str, cols: u16, rows: u16) -> bool {
    MediaList::parse(query).matches(&env(cols, rows))
}

/// Media Queries 4 §3.1: Kleene logic — false wins `and`, true wins
/// `or`, `not` keeps unknown.
#[test]
fn truth_is_kleene() {
    use Truth::*;
    assert_eq!(Unknown.not(), Unknown);
    assert_eq!(False.and(Unknown), False);
    assert_eq!(True.and(Unknown), Unknown);
    assert_eq!(True.or(Unknown), True);
    assert_eq!(False.or(Unknown), Unknown);
    assert!(!Unknown.holds());
}

/// §2.1: the empty list matches everything; a list matches when any
/// query does.
#[test]
fn an_empty_list_matches_all() {
    assert!(matches("", 1, 1));
    assert!(matches("print, screen", 1, 1));
    assert!(!matches("print, tv", 1, 1));
}

/// §3: every range spelling; `>=` needs its two characters together.
#[test]
fn range_forms() {
    assert!(matches("(width >= 80)", 80, 1));
    assert!(!matches("(width > 80)", 80, 1));
    assert!(matches("(80 <= width)", 80, 1));
    assert!(matches("(80 = width)", 80, 1));
    assert!(matches("(width: 80)", 80, 1));
    assert!(matches("(10 < width < 81)", 80, 1));
    assert!(matches("(90 > width > 10)", 80, 1));
    assert!(
        !matches("(10 < width > 5)", 80, 1),
        "mixed directions are invalid"
    );
    assert!(!matches("(width > = 80)", 80, 1), "`> =` is not `>=`");
    assert!(matches("(max-width: 80)", 80, 1));
    assert!(!matches("(min-width: 81)", 80, 1));
    assert!(matches("(width >= -5)", 0, 1));
}

/// §3.2: a query that does not parse is `not all`; its neighbours stand.
#[test]
fn an_invalid_query_is_not_all() {
    let list = MediaList::parse("(width >= 1) and, screen");
    assert_eq!(list.queries().len(), 2);
    assert_eq!(list.to_string(), "not all, screen");
    assert!(list.matches(&env(10, 10)));
    assert!(!matches("only (width)", 10, 10));
    assert!(!matches("screen and (width) or (height)", 10, 10));
    assert!(!matches("not", 10, 10));
    assert!(!matches("and", 10, 10));
}

/// §3.1: `<general-enclosed>` and unknown features are unknown at every
/// depth; an unknown result is false even under `not`.
#[test]
fn unknown_is_false_even_negated() {
    assert!(!matches("(fancy)", 10, 10));
    assert!(!matches("not (fancy)", 10, 10));
    assert!(!matches("not screen and (fancy)", 10, 10));
    assert!(!matches("fn(1)", 10, 10));
    assert!(matches("fn(1) or (width)", 10, 10));
    assert!(!matches("(width: 10px)", 10, 10));
    assert!(!matches("(width: red)", 10, 10));
    assert!(
        !matches("(orientation > 2)", 10, 10),
        "a range on a discrete feature"
    );
}

/// §4.3–§4.4: aspect ratio is columns over rows, a `<ratio>` or a number.
#[test]
fn aspect_ratio_and_orientation() {
    assert!(matches("(aspect-ratio: 2/1)", 80, 40));
    assert!(matches("(aspect-ratio: 2)", 80, 40));
    assert!(matches("(min-aspect-ratio: 16 / 9)", 80, 40));
    assert!(
        matches("(orientation: landscape)", 10, 10),
        "square is landscape"
    );
    assert!(matches("(orientation: portrait)", 10, 11));
}

/// Media Queries 5 §12: the preferences come from the environment.
#[test]
fn preferences_come_from_the_environment() {
    let prefs = MediaPreferences::new()
        .with_reduced_motion(true)
        .with_contrast(Contrast::More)
        .with_forced_colors(true)
        .with_pointer(false, PointerAccuracy::None)
        .with_color_bits(0);
    let env = MediaEnvironment::new(Viewport::new(10, 10), ColorScheme::Light, prefs);
    for yes in [
        "(prefers-reduced-motion: reduce)",
        "(prefers-reduced-motion)",
        "(prefers-contrast: more)",
        "(prefers-contrast)",
        "(forced-colors: active)",
        "(hover: none)",
        "not (hover)",
        "(pointer: none)",
        "not (any-pointer)",
        "(monochrome)",
        "not (color)",
        "(prefers-color-scheme: light)",
    ] {
        assert!(MediaList::parse(yes).matches(&env), "{yes}");
    }
}

/// CSSOM §4.2.2: a canonical serialization.
#[test]
fn serialization() {
    let text = |q: &str| MediaList::parse(q).to_string();
    assert_eq!(
        text("SCREEN  AND (MIN-WIDTH:80)"),
        "screen and (width >= 80)"
    );
    assert_eq!(text("(10<width<=20)"), "(10 < width <= 20)");
    assert_eq!(text("not print"), "not print");
    assert_eq!(
        text("(hover) or (pointer: fine)"),
        "(hover) or (pointer: fine)"
    );
    assert_eq!(text("not (width > 5)"), "not (width > 5)");
    assert_eq!(text("(min-aspect-ratio: 16/9)"), "(aspect-ratio >= 16 / 9)");
    assert_eq!(text(""), "");
}

/// Only a list with a viewport feature can change on a resize.
#[test]
fn reads_viewport() {
    assert!(MediaList::parse("screen and (width > 3)").reads_viewport());
    assert!(MediaList::parse("(orientation: portrait)").reads_viewport());
    assert!(!MediaList::parse("(hover)").reads_viewport());
    assert!(!MediaList::parse("print").reads_viewport());
}
