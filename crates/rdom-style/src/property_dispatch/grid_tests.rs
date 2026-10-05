//! Dispatch tests for the grid properties (C7, CSS Grid Layout 2).

use super::*;
use crate::layout::{GridTemplate, RepeatCount, TrackList, TrackSize};
use crate::{ImportantMask, TuiStyle, Value};

/// CSS Grid 2 §7.2: `grid-template-columns` / `grid-template-rows` take
/// `none | <track-list> | <auto-track-list>`, initial `none`, not
/// inherited; each owns its own field and `!important` bit, and
/// serializes the specified value as written.
#[test]
fn grid_template_properties_take_track_lists() {
    let mut style = TuiStyle::new();
    set("grid-template-columns", "[a] 1fr repeat(2, 3)", &mut style).unwrap();
    set("grid-template-rows", "NONE", &mut style).unwrap();
    assert_eq!(
        style.grid_template_columns,
        Some(Value::Specified(GridTemplate::from(
            TrackList::new([])
                .line("a")
                .track(TrackSize::fr(1.0))
                .repeat(RepeatCount::Count(2), [TrackSize::cells(3)])
        )))
    );
    assert_eq!(
        serialize("grid-template-columns", &style).as_deref(),
        Some("[a] 1fr repeat(2, 3)")
    );
    assert_eq!(
        serialize("grid-template-rows", &style).as_deref(),
        Some("none")
    );
    for bad in [
        "",
        "1fr,",
        "repeat(auto-fill, 1fr)",
        "minmax(1fr, 1)",
        "[a]",
    ] {
        assert_eq!(
            set("grid-template-columns", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert!(!inherits("grid-template-columns"));
    assert_eq!(
        property_mask("grid-template-columns"),
        Some(ImportantMask::GRID_TEMPLATE_COLUMNS)
    );
    assert_eq!(
        property_mask("grid-template-rows"),
        Some(ImportantMask::GRID_TEMPLATE_ROWS)
    );
    assert!(remove("grid-template-columns", &mut style));
    assert_eq!(style.grid_template_columns, None);
    assert_eq!(
        crate::ComputedStyle::initial().grid_template_columns,
        GridTemplate::None
    );
}

/// The builder writes what the declaration would (`TuiStyle` setters
/// follow the property names), and refuses a value outside the grammar
/// in a release build as a parser drops it.
#[test]
fn grid_template_builders_write_the_declarations() {
    let style = TuiStyle::new()
        .grid_template_columns(vec![TrackSize::fr(1.0), TrackSize::cells(3)])
        .grid_template_rows_important(GridTemplate::None);
    assert_eq!(
        serialize("grid-template-columns", &style).as_deref(),
        Some("1fr 3")
    );
    assert_eq!(
        serialize("grid-template-rows", &style).as_deref(),
        Some("none")
    );
    assert!(style.important.contains(ImportantMask::GRID_TEMPLATE_ROWS));
    assert!(
        !style
            .important
            .contains(ImportantMask::GRID_TEMPLATE_COLUMNS)
    );
}

/// DESIGN (C6G-ALIGN-API's rule): a typed value outside the grammar has
/// no in-range neighbour, so the builder panics in a debug build.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "is not a value of `grid-template-columns`")]
fn an_invalid_track_list_panics_in_a_debug_build() {
    let _ = TuiStyle::new().grid_template_columns(vec![TrackSize::minmax(
        crate::layout::TrackBreadth::Fr(1.0),
        crate::layout::TrackBreadth::Auto,
    )]);
}

/// `.grid()` / `.inline_grid()` write `display: grid` / `inline-grid`
/// (CSS Display 3 §2.7: the block-level and inline-level grid container).
#[test]
fn grid_display_builders() {
    let style = TuiStyle::new().grid();
    assert_eq!(serialize("display", &style).as_deref(), Some("grid"));
    let style = TuiStyle::new().inline_grid();
    assert_eq!(serialize("display", &style).as_deref(), Some("inline-grid"));
}

/// CSS Grid 2 §7.6: `grid-auto-columns` / `grid-auto-rows` take
/// `<track-size>+` — no line names, no `repeat()` — initial `auto`, not
/// inherited, each with its own field and bit, serialized as written.
#[test]
fn grid_auto_properties_take_track_sizes() {
    let mut style = TuiStyle::new();
    set(
        "grid-auto-rows",
        "1 MINMAX(2, 1fr) fit-content(3)",
        &mut style,
    )
    .unwrap();
    set("grid-auto-columns", "auto", &mut style).unwrap();
    assert_eq!(
        style.grid_auto_rows,
        Some(Value::Specified(vec![
            TrackSize::cells(1),
            TrackSize::minmax(2, crate::layout::TrackBreadth::Fr(1.0)),
            TrackSize::fit_content(3),
        ]))
    );
    assert_eq!(
        serialize("grid-auto-rows", &style).as_deref(),
        Some("1 minmax(2, 1fr) fit-content(3)")
    );
    assert_eq!(
        serialize("grid-auto-columns", &style).as_deref(),
        Some("auto")
    );
    for bad in ["", "none", "repeat(2, 1)", "[a] 1", "minmax(1fr, 1)", "1,"] {
        assert_eq!(
            set("grid-auto-columns", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert!(!inherits("grid-auto-rows"));
    assert_eq!(
        property_mask("grid-auto-columns"),
        Some(ImportantMask::GRID_AUTO_COLUMNS)
    );
    assert_eq!(
        property_mask("grid-auto-rows"),
        Some(ImportantMask::GRID_AUTO_ROWS)
    );
    assert_eq!(
        crate::ComputedStyle::initial().grid_auto_rows,
        vec![TrackSize::AUTO]
    );
    let built = TuiStyle::new()
        .grid_auto_columns([TrackSize::fr(1.0), TrackSize::cells(2)])
        .grid_auto_rows_important([TrackSize::AUTO]);
    assert_eq!(
        serialize("grid-auto-columns", &built).as_deref(),
        Some("1fr 2")
    );
    assert!(built.important.contains(ImportantMask::GRID_AUTO_ROWS));
}

/// CSS Grid 2 §8.3: `<grid-line> = auto | <custom-ident> | [ <integer>
/// && <custom-ident>? ] | [ span && [ <integer [1,∞]> || <custom-ident>
/// ] ]` — `0` and a negative span are invalid, `span` and `auto` are no
/// line names; each longhand serializes the shortest form.
#[test]
fn grid_placement_longhands_take_grid_lines() {
    use crate::layout::GridLine;
    for (css, line, out) in [
        ("auto", GridLine::Auto, "auto"),
        ("3", GridLine::line(3), "3"),
        ("-1", GridLine::line(-1), "-1"),
        ("a", GridLine::named("a"), "a"),
        ("2 a", GridLine::nth_named(2, "a"), "2 a"),
        ("a -2", GridLine::nth_named(-2, "a"), "-2 a"),
        ("span 2", GridLine::span(2), "span 2"),
        ("SPAN a", GridLine::span_named(1, "a"), "span a"),
        ("a span 3", GridLine::span_named(3, "a"), "span 3 a"),
    ] {
        let mut style = TuiStyle::new();
        set("grid-row-start", css, &mut style).unwrap_or_else(|e| panic!("{css}: {e:?}"));
        assert_eq!(style.grid_row_start, Some(Value::Specified(line)), "{css}");
        assert_eq!(
            serialize("grid-row-start", &style).as_deref(),
            Some(out),
            "{css}"
        );
    }
    for bad in [
        "0",
        "span 0",
        "span -1",
        "span",
        "span auto",
        "1 2",
        "a b",
        "auto 1",
        "1.5",
        "",
    ] {
        assert_eq!(
            set("grid-column-end", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    for name in [
        "grid-row-start",
        "grid-row-end",
        "grid-column-start",
        "grid-column-end",
    ] {
        assert!(!inherits(name));
        assert_eq!(
            crate::ComputedStyle::initial().grid_row_start,
            GridLine::Auto,
            "{name}"
        );
    }
}

/// CSS Grid 2 §8.4: `grid-row` / `grid-column` set start and end — an
/// omitted end is the start's `<custom-ident>`, else `auto` — and
/// `grid-area` all four (row-start / column-start / row-end /
/// column-end), each omitted one copying the custom ident it pairs with;
/// each serializes the shortest form that gives the longhands back.
#[test]
fn grid_placement_shorthands_expand_and_serialize() {
    use crate::layout::GridLine;
    let mut style = TuiStyle::new();
    set("grid-row", "2 / span 3", &mut style).unwrap();
    assert_eq!(
        (style.grid_row_start.clone(), style.grid_row_end.clone()),
        (
            Some(Value::Specified(GridLine::line(2))),
            Some(Value::Specified(GridLine::span(3)))
        )
    );
    assert_eq!(serialize("grid-row", &style).as_deref(), Some("2 / span 3"));
    set("grid-column", "a", &mut style).unwrap();
    assert_eq!(
        style.grid_column_end,
        Some(Value::Specified(GridLine::named("a")))
    );
    assert_eq!(serialize("grid-column", &style).as_deref(), Some("a"));
    set("grid-column", "3", &mut style).unwrap();
    assert_eq!(
        style.grid_column_end,
        Some(Value::Specified(GridLine::Auto))
    );
    assert_eq!(serialize("grid-column", &style).as_deref(), Some("3"));

    set("grid-area", "x", &mut style).unwrap();
    for f in [
        &style.grid_row_start,
        &style.grid_column_start,
        &style.grid_row_end,
        &style.grid_column_end,
    ] {
        assert_eq!(f, &Some(Value::Specified(GridLine::named("x"))));
    }
    assert_eq!(serialize("grid-area", &style).as_deref(), Some("x"));
    set("grid-area", "1 / 2 / 3", &mut style).unwrap();
    assert_eq!(
        style.grid_column_end,
        Some(Value::Specified(GridLine::Auto))
    );
    assert_eq!(serialize("grid-area", &style).as_deref(), Some("1 / 2 / 3"));
    set("grid-area", "a / b / 3 / 4", &mut style).unwrap();
    assert_eq!(
        serialize("grid-area", &style).as_deref(),
        Some("a / b / 3 / 4")
    );
    assert_eq!(serialize("grid-row", &style).as_deref(), Some("a / 3"));
    for bad in ["1 / 2 / 3 / 4 / 5", "/ 1", "1 /", "0 / 1"] {
        assert_eq!(
            set("grid-area", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert_eq!(
        property_mask("grid-area"),
        Some(
            ImportantMask::GRID_ROW_START
                | ImportantMask::GRID_COLUMN_START
                | ImportantMask::GRID_ROW_END
                | ImportantMask::GRID_COLUMN_END
        )
    );
}

/// CSS Grid 2 §7.7: `grid-auto-flow: [ row | column ] || dense`, initial
/// `row`; `row dense` serializes as `dense`.
#[test]
fn grid_auto_flow_takes_its_keywords() {
    use crate::layout::{Direction, GridAutoFlow};
    for (css, flow, out) in [
        ("row", GridAutoFlow::ROW, "row"),
        ("column", GridAutoFlow::COLUMN, "column"),
        ("dense", GridAutoFlow::ROW.dense(), "dense"),
        ("ROW DENSE", GridAutoFlow::ROW.dense(), "dense"),
        ("dense column", GridAutoFlow::COLUMN.dense(), "column dense"),
    ] {
        let mut style = TuiStyle::new();
        set("grid-auto-flow", css, &mut style).unwrap();
        assert_eq!(style.grid_auto_flow, Some(Value::Specified(flow)), "{css}");
        assert_eq!(serialize("grid-auto-flow", &style).as_deref(), Some(out));
    }
    for bad in ["row column", "dense dense", "auto", ""] {
        assert_eq!(
            set("grid-auto-flow", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    assert_eq!(
        crate::ComputedStyle::initial().grid_auto_flow,
        GridAutoFlow::ROW
    );
    assert_eq!(GridAutoFlow::COLUMN.direction, Direction::Column);
}

/// The placement builders write what the declarations would, and refuse
/// a line outside the grammar as the other grid builders do.
#[test]
fn grid_placement_builders_write_the_declarations() {
    use crate::layout::{GridAutoFlow, GridLine};
    let style = TuiStyle::new()
        .grid_row(GridLine::line(1), GridLine::span(2))
        .grid_column_start_important(GridLine::named("a"))
        .grid_auto_flow(GridAutoFlow::COLUMN.dense());
    assert_eq!(serialize("grid-row", &style).as_deref(), Some("1 / span 2"));
    assert_eq!(serialize("grid-column-start", &style).as_deref(), Some("a"));
    assert!(style.important.contains(ImportantMask::GRID_COLUMN_START));
    assert_eq!(
        serialize("grid-auto-flow", &style).as_deref(),
        Some("column dense")
    );
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "is not a value of `grid-row-start`")]
fn a_zero_grid_line_panics_in_a_debug_build() {
    let _ = TuiStyle::new().grid_row_start(crate::layout::GridLine::line(0));
}
