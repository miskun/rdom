//! C7G-TRACK-VALIDITY — grid values built in Rust (CSS Grid 2 §7.2,
//! §7.6, §8.3) reach layout only in their grammar. A builder whose value
//! has an in-range neighbour clamps to it (DESIGN's clamp-or-panic rule:
//! a span or repetition count of 0 is 1, a negative or NaN `fr` or
//! percentage 0, an empty track list `none`); a value assigned to a
//! `TuiStyle` field without a builder that is outside the grammar is
//! dropped by the cascade, as a CSS parser drops an invalid declaration
//! (CSS Syntax 3 §8.1: the declaration is ignored, so a lower one
//! applies).

use super::{el, lay_out, rect};
use rdom_tui::{
    GridLine, GridTemplate, RepeatCount, TrackBreadth, TrackList, TrackSize, TuiDom, TuiNodeMutExt,
    TuiStyle, Value,
};

/// The `(x, width)` of the two items of a 10-wide grid container `.g`
/// whose inline style is `style`, the sheet `css` beside `.g { display:
/// grid }`; `place` sets the first item's inline style.
fn items(style: TuiStyle, css: &str, place: Option<TuiStyle>) -> [(i32, u16); 2] {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let a = el(&mut dom, g, "div", "");
    let b = el(&mut dom, g, "div", "");
    dom.node_mut(g).set_inline_style(style.grid());
    if let Some(place) = place {
        dom.node_mut(a).set_inline_style(place);
    }
    lay_out(&mut dom, &format!(".g {{ width: 10 }} {css}"), 10, 6);
    [a, b].map(|n| {
        let r = rect(&dom, n);
        (r.x, r.width)
    })
}

/// §7.2.4: `<flex>` is `[0,∞]` — `TrackSize::fr` keeps a negative or NaN
/// factor at 0, as `flex-grow` does (`valid_flex_factor`), so the other
/// track takes the whole free space; §7.2.1 alike for a percentage.
#[test]
fn a_negative_or_nan_fr_or_percent_is_zero() {
    for bad in [
        TrackSize::fr(-1.0),
        TrackSize::fr(f32::NAN),
        TrackSize::percent(-5.0),
        TrackSize::percent(f32::NAN),
    ] {
        let style = TuiStyle::new().grid_template_columns(vec![bad.clone(), TrackSize::fr(1.0)]);
        assert_eq!(items(style, "", None), [(0, 0), (0, 10)], "{bad:?}");
    }
}

/// §7.2.3: `repeat(<integer [1,∞]>, …)` — a count of 0 is 1.
#[test]
fn a_repetition_count_of_zero_is_one() {
    let list = TrackList::default()
        .repeat(RepeatCount::Count(0), [TrackSize::cells(3)])
        .track(TrackSize::cells(4));
    let style = TuiStyle::new().grid_template_columns(list);
    assert_eq!(items(style, "", None), [(0, 3), (3, 4)]);
}

/// §8.3: `span <integer [1,∞]>` — `GridLine::span(0)` is a span of one.
#[test]
fn a_span_of_zero_is_one() {
    let style =
        TuiStyle::new().grid_template_columns(vec![TrackSize::cells(3), TrackSize::cells(4)]);
    let place = TuiStyle::new()
        .grid_column_start(GridLine::line(1))
        .grid_column_end(GridLine::span(0));
    assert_eq!(items(style, "", Some(place)), [(0, 3), (3, 4)]);
}

/// An empty track list is no explicit grid: `grid-template-columns:
/// none`, one implicit `auto` column the container's width.
#[test]
fn an_empty_track_list_is_none() {
    let style = TuiStyle::new().grid_template_columns(Vec::<TrackSize>::new());
    assert_eq!(items(style, "", None), [(0, 10), (0, 10)]);
    let style = TuiStyle::new().grid_template_columns(TrackList::default());
    assert_eq!(items(style, "", None), [(0, 10), (0, 10)]);
}

/// A value outside the grammar written straight into a `TuiStyle` field
/// is ignored by the cascade — the sheet's `4 6` applies — and never
/// reaches layout: an empty `TrackList`, a NaN `fr`, a span of 0, an
/// empty `grid-auto-columns`.
#[test]
fn an_invalid_field_value_is_ignored() {
    let sheet = ".g { grid-template-columns: 4 6 }";
    let mut style = TuiStyle::new();
    style.grid_template_columns =
        Some(Value::Specified(GridTemplate::Tracks(TrackList::default())));
    assert_eq!(items(style, sheet, None), [(0, 4), (4, 6)]);

    let mut style = TuiStyle::new();
    style.grid_template_columns = Some(Value::Specified(GridTemplate::Tracks(TrackList::new([
        TrackSize::Breadth(TrackBreadth::Fr(f32::NAN)),
    ]))));
    assert_eq!(items(style, sheet, None), [(0, 4), (4, 6)]);

    let mut style = TuiStyle::new();
    style.grid_auto_columns = Some(Value::Specified(Vec::new()));
    assert_eq!(
        items(style, ".g { grid-template-columns: 4 }", None),
        [(0, 4), (0, 4)]
    );

    let mut place = TuiStyle::new();
    place.grid_column_end = Some(Value::Specified(GridLine::Span {
        count: 0,
        name: None,
    }));
    assert_eq!(items(TuiStyle::new(), sheet, Some(place)), [(0, 4), (4, 6)]);
}
