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
