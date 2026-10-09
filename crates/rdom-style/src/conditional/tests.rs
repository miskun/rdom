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

// ─── C14-SUPPORTS: feature queries (CSS Conditional 3 §6, 4 §6, 5 §5) ─

fn holds(condition: &str) -> bool {
    SupportsCondition::parse(condition)
        .unwrap_or_else(|| panic!("{condition} parses"))
        .matches()
}

/// Conditional 3 §6.1: a declaration is supported when the property is
/// known and its parser takes the value — the real value parser.
#[test]
fn a_declaration_is_tested_with_the_value_parser() {
    assert!(holds("(display: grid)"));
    assert!(holds("(DISPLAY: Grid)"));
    assert!(holds("(width: calc(50% - 2))"));
    assert!(holds("(color: light-dark(red, blue))"));
    assert!(
        holds("(color: var(--x))"),
        "a substitution is kept for the cascade"
    );
    assert!(holds("(color: inherit)"));
    assert!(holds("(--anything: { weird [ } ] )"));
    assert!(
        holds("(color: red !important)"),
        "`!important` is part of a declaration"
    );
    assert!(!holds("(display: frobnicate)"));
    assert!(!holds("(frobnicate: 1)"));
    assert!(!holds("(width: 10px)"), "no pixel geometry");
    assert!(!holds("(color:)"));
}

/// §6: `not` / `and` / `or`, and `<general-enclosed>` unknown — false,
/// and so is its `not`.
#[test]
fn supports_conditions_combine() {
    assert!(holds("not (display: frobnicate)"));
    assert!(holds("(display: grid) and (gap: 1)"));
    assert!(!holds("(display: grid) and (gap: 1px)"));
    assert!(holds("(display: frob) or (display: flex)"));
    assert!(holds("((display: grid) or (x: y)) and (not (z: w))"));
    assert!(!holds("unknown(1)"));
    assert!(!holds("not unknown(1)"));
    assert!(!holds("(display: grid) and unknown(1)"));
    assert!(
        SupportsCondition::parse("display: grid").is_none(),
        "no parentheses"
    );
    assert!(SupportsCondition::parse("(a: b) and (c: d) or (e: f)").is_none());
}

/// Conditional 4 §6.1 `selector()`: one complex selector rdom parses;
/// Conditional 5 §5: rdom draws no fonts.
#[test]
fn selector_and_font_functions() {
    assert!(holds("selector(:has(> img))"));
    assert!(holds("selector(a > b ~ c)"));
    assert!(holds("selector(p::before)"));
    assert!(holds("selector(:is(a, b))"));
    assert!(!holds("selector(a, b)"), "a list is not a complex selector");
    assert!(!holds("selector(:frobnicate)"));
    assert!(!holds("font-tech(color-COLRv1)"));
    assert!(!holds("font-format(woff2)"));
    assert!(holds("not font-format(woff2)"));
}

/// Conditional 3 §7.1 `CSS.supports()`: the two-argument form takes a
/// value (no `!important`); the one-argument form a condition, or a bare
/// declaration.
#[test]
fn css_supports() {
    assert!(supports("display", "grid"));
    assert!(supports("--x", "1 2 3"));
    assert!(!supports("display", "frob"));
    assert!(!supports("color", "red !important"));
    assert!(supports_condition("(display: grid) and selector(a)"));
    assert!(supports_condition("display: grid"));
    assert!(!supports_condition("display: frob"));
    assert!(!supports_condition("nonsense"));
}

// ─── C14-CONTAINER: container queries (CSS Conditional 5 §6.4–§6.5) ───

fn container(text: &str) -> ContainerQuery {
    ContainerQuery::parse(text).unwrap_or_else(|| panic!("{text} parses"))
}

fn eval(text: &str, width: Option<f64>, height: Option<f64>) -> Truth {
    let none = |_: &str| None;
    container(text).conditions()[0].evaluate(&QueryContainer {
        width,
        height,
        custom: &none,
    })
}

/// §6.4: a condition is a name, a query or both; a list of them; the
/// name is not `not` / `and` / `or` / `none`.
#[test]
fn container_preludes_parse() {
    let q = container("card (width > 3), (height < 2), side");
    let names: Vec<Option<&str>> = q.conditions().iter().map(|c| c.name()).collect();
    assert_eq!(names, [Some("card"), None, Some("side")]);
    assert!(q.conditions()[0].needs_size());
    assert!(!q.conditions()[2].needs_size());
    assert!(
        container("not (width > 3)").conditions()[0]
            .name()
            .is_none()
    );
    assert!(container("style(--x: 1)").conditions()[0].name().is_none());
    assert!(!container("style(--x: 1)").conditions()[0].needs_size());
    assert!(container("scroll-state(stuck: top)").conditions()[0].needs_scroll_state());
    assert!(ContainerQuery::parse("").is_none());
    assert!(ContainerQuery::parse("card (width > 3) (height > 1)").is_none());
    assert!(ContainerQuery::parse("a b").is_none());
    assert_eq!(
        container("card (width > 3)").to_string(),
        "card (width > 3)"
    );
}

/// §6.5: the size features read the container's content box; an axis it
/// does not answer on is unknown.
#[test]
fn size_features_read_the_container() {
    assert_eq!(eval("(width > 3)", Some(4.0), None), Truth::True);
    assert_eq!(eval("(inline-size <= 3)", Some(4.0), None), Truth::False);
    assert_eq!(eval("(height > 3)", Some(4.0), None), Truth::Unknown);
    assert_eq!(eval("(block-size: 2)", Some(4.0), Some(2.0)), Truth::True);
    assert_eq!(
        eval("(aspect-ratio > 1)", Some(4.0), Some(2.0)),
        Truth::True
    );
    assert_eq!(
        eval("(orientation: portrait)", Some(4.0), Some(5.0)),
        Truth::True
    );
    assert_eq!(
        eval("(orientation: portrait)", Some(4.0), None),
        Truth::Unknown
    );
    assert_eq!(
        eval("(color)", Some(4.0), Some(5.0)),
        Truth::Unknown,
        "not a size feature"
    );
}

/// §6.4 style queries: custom properties by computed value (whitespace
/// collapsed), or that one has a value; standard properties unknown.
#[test]
fn style_features_read_custom_properties() {
    let vars = |name: &str| match name {
        "--a" => Some("  dark   blue ".to_string()),
        _ => None,
    };
    let eval = |text: &str| {
        container(text).conditions()[0].evaluate(&QueryContainer {
            width: None,
            height: None,
            custom: &vars,
        })
    };
    assert_eq!(eval("style(--a: dark blue)"), Truth::True);
    assert_eq!(eval("style(--a)"), Truth::True);
    assert_eq!(eval("style(--b)"), Truth::False);
    assert_eq!(eval("style(--a: light)"), Truth::False);
    assert_eq!(eval("style((--a: dark blue) and (--b: 1))"), Truth::False);
    assert_eq!(eval("style(color: red)"), Truth::Unknown);
    assert_eq!(eval("scroll-state(stuck: top)"), Truth::Unknown);
}
