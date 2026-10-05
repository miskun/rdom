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
