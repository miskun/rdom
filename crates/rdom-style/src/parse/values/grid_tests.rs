//! C7-GRID-CORE — `grid-template-columns` / `-rows` values (CSS Grid
//! Layout 2 §7.2): the grammar accepted and rejected, and the
//! serialization browsers give a specified value.

use super::*;
use crate::parse::token::tokenize;

fn parse(css: &str) -> Option<GridTemplate> {
    parse_grid_template(&tokenize(css).unwrap())
}

fn round_trip(css: &str) -> String {
    serialize_grid_template(&parse(css).unwrap_or_else(|| panic!("`{css}` parses")))
}

/// §7.2: `none` is no explicit grid; a track list is one track per
/// `<track-size>` — cells, `fr`, `%` and the keywords.
#[test]
fn none_and_plain_track_sizes_parse() {
    assert_eq!(parse("none"), Some(GridTemplate::None));
    assert_eq!(
        parse("10 2fr 25% auto min-content max-content"),
        Some(GridTemplate::from(vec![
            TrackSize::cells(10),
            TrackSize::fr(2.0),
            TrackSize::percent(25.0),
            TrackSize::AUTO,
            TrackSize::Breadth(TrackBreadth::MinContent),
            TrackSize::Breadth(TrackBreadth::MaxContent),
        ]))
    );
}

/// §7.2.2 / §7.2.3.1: `minmax(<inflexible-breadth>, <track-breadth>)`
/// — an `fr` minimum is invalid — and `fit-content(<length-percentage>)`.
#[test]
fn minmax_and_fit_content_follow_their_grammars() {
    assert_eq!(
        parse("minmax(auto, 1fr) fit-content(10)"),
        Some(GridTemplate::from(vec![
            TrackSize::minmax(TrackBreadth::Auto, TrackBreadth::Fr(1.0)),
            TrackSize::fit_content(10),
        ]))
    );
    assert_eq!(parse("minmax(1fr, 2)"), None);
    assert_eq!(parse("minmax(2)"), None);
    assert_eq!(parse("fit-content(auto)"), None);
    assert_eq!(parse("fit-content(1fr)"), None);
    assert_eq!(round_trip("fit-content(50%)"), "fit-content(50%)");
}

/// §7.2: `<line-names>` are bracketed `<custom-ident>`s between the
/// tracks, kept for placement; `span` and `auto` are not line names, and
/// two bracket groups in a row are invalid.
#[test]
fn line_names_sit_between_the_tracks() {
    let GridTemplate::Tracks(list) = parse("[a] 10 [b c] 1fr [d]").unwrap() else {
        panic!("a track list");
    };
    assert_eq!(
        list.line_names,
        [
            vec!["a".to_string()],
            vec!["b".into(), "c".into()],
            vec!["d".into()]
        ]
    );
    assert_eq!(round_trip("[a] 10 [b c] 1fr [d]"), "[a] 10 [b c] 1fr [d]");
    assert_eq!(round_trip("[] 1 []"), "1");
    assert_eq!(parse("[a] [b] 1"), None);
    assert_eq!(parse("[span] 1"), None);
    assert_eq!(parse("[auto] 1"), None);
    assert_eq!(parse("[1] 1"), None);
    assert_eq!(parse("[a"), None);
    assert_eq!(parse("[a]"), None, "a list needs a track");
}

/// §7.2.3: `repeat()` is kept as written — an integer count of at least
/// one, its own line names — and serializes back so.
#[test]
fn repeat_is_kept_as_written() {
    assert_eq!(round_trip("repeat(2, 1fr)"), "repeat(2, 1fr)");
    assert_eq!(
        round_trip("[a] repeat(3, [b] 2 [c] 1fr) 5"),
        "[a] repeat(3, [b] 2 [c] 1fr) 5"
    );
    assert_eq!(parse("repeat(0, 1)"), None);
    assert_eq!(parse("repeat(-1, 1)"), None);
    assert_eq!(parse("repeat(2)"), None);
    assert_eq!(parse("repeat(2, )"), None);
    assert_eq!(parse("repeat(2, repeat(2, 1))"), None, "no nested repeat()");
}

/// §7.2.3.1 `<auto-track-list>`: one `repeat(auto-fill | auto-fit, …)`
/// of fixed sizes, every other size fixed too.
#[test]
fn an_automatic_repetition_takes_fixed_sizes_only() {
    assert_eq!(
        round_trip("repeat(auto-fill, minmax(10, 1fr))"),
        "repeat(auto-fill, minmax(10, 1fr))"
    );
    assert_eq!(
        round_trip("5 repeat(auto-fit, 3 [x]) 20%"),
        "5 repeat(auto-fit, 3 [x]) 20%"
    );
    assert_eq!(parse("repeat(auto-fill, 1fr)"), None);
    assert_eq!(parse("repeat(auto-fill, auto)"), None);
    assert_eq!(parse("repeat(auto-fill, 2) auto"), None);
    assert_eq!(parse("repeat(auto-fill, 2) repeat(auto-fit, 2)"), None);
    assert_eq!(parse("repeat(auto-fill, 2) repeat(2, 1fr)"), None);
    assert!(parse("repeat(auto-fill, 2) repeat(2, 3)").is_some());
}

/// CSS Values 4 §2.1: keywords and function names are ASCII
/// case-insensitive; a length takes rdom's cells, a percentage and a
/// math function; a negative length is invalid.
#[test]
fn keywords_lengths_and_math() {
    assert_eq!(
        round_trip("MINMAX(Auto, 1FR) Repeat(2, Min-Content)"),
        "minmax(auto, 1fr) repeat(2, min-content)"
    );
    assert_eq!(
        round_trip("10 Repeat(AUTO-FILL, 4)"),
        "10 repeat(auto-fill, 4)"
    );
    assert_eq!(round_trip("calc(50% - 2) 0.5fr"), "calc(50% - 2) 0.5fr");
    assert_eq!(parse("-1"), None);
    assert_eq!(parse("-1fr"), None);
    assert_eq!(parse("1fr,"), None);
}

/// CSS Values 4 §6.1.2: a viewport-percentage breadth is absolute at
/// computed-value time — whole cells, or a `calc()` kept for the
/// percentage layout resolves.
#[test]
fn viewport_units_in_a_track_list_compute_to_cells() {
    let mut c = crate::ComputedStyle::initial();
    c.grid_template_columns =
        parse("50vw repeat(2, minmax(10vh, 1fr)) fit-content(calc(50% + 10vw))").expect("parses");
    c.resolve_viewport_units(crate::calc::Viewport::new(80, 20));
    assert_eq!(
        serialize_grid_template(&c.grid_template_columns),
        "40 repeat(2, minmax(2, 1fr)) fit-content(calc(50% + 8))"
    );
}

/// CSS Values 4 §6.1.2 for `grid-auto-*` (CSS Grid 2 §7.6): a declared
/// list's viewport units compute to cells, and the initial `auto` stays
/// the shared, borrowed list (C7G-INITIAL-ALLOC).
#[test]
fn viewport_units_in_an_auto_track_list_compute_to_cells() {
    let mut c = crate::ComputedStyle::initial();
    let sizes = parse_track_sizes(&tokenize("50vw minmax(10vh, 1fr)").unwrap()).expect("parses");
    c.grid_auto_columns = std::borrow::Cow::Owned(sizes);
    c.resolve_viewport_units(crate::calc::Viewport::new(80, 20));
    assert_eq!(
        serialize_track_sizes(&c.grid_auto_columns),
        "40 minmax(2, 1fr)"
    );
    assert!(matches!(c.grid_auto_rows, std::borrow::Cow::Borrowed(_)));
}
