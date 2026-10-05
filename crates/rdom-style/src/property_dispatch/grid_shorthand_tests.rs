//! Dispatch tests for the grid shorthands (C7-GRID-AREAS, CSS Grid
//! Layout 2): `grid-template` (§7.4) and `grid` (§7.8).

use super::*;
use crate::layout::{GridAutoFlow, GridTemplate, GridTemplateAreas, TrackSize};
use crate::{ImportantMask, TuiStyle, Value};

fn serialized(name: &str, style: &TuiStyle) -> Option<String> {
    serialize(name, style)
}

/// §7.4: `grid-template: none` sets all three longhands to `none`;
/// `<'grid-template-rows'> / <'grid-template-columns'>` sets those two
/// and `grid-template-areas` to `none`.
#[test]
fn grid_template_sets_rows_and_columns() {
    let mut style = TuiStyle::new();
    set("grid-template-areas", "\"a\"", &mut style).unwrap();
    set("grid-template", "1 2 / [a] repeat(2, 3)", &mut style).unwrap();
    assert_eq!(
        serialized("grid-template-rows", &style).as_deref(),
        Some("1 2")
    );
    assert_eq!(
        serialized("grid-template-columns", &style).as_deref(),
        Some("[a] repeat(2, 3)")
    );
    assert_eq!(
        style.grid_template_areas,
        Some(Value::Specified(GridTemplateAreas::NONE))
    );
    assert_eq!(
        serialized("grid-template", &style).as_deref(),
        Some("1 2 / [a] repeat(2, 3)")
    );
    set("grid-template", "NONE", &mut style).unwrap();
    assert_eq!(
        style.grid_template_rows,
        Some(Value::Specified(GridTemplate::None))
    );
    assert_eq!(serialized("grid-template", &style).as_deref(), Some("none"));
    assert_eq!(
        property_mask("grid-template"),
        Some(
            ImportantMask::GRID_TEMPLATE_ROWS
                | ImportantMask::GRID_TEMPLATE_COLUMNS
                | ImportantMask::GRID_TEMPLATE_AREAS
        )
    );
}

/// §7.4: `[ <line-names>? <string> <track-size>? <line-names>? ]+ [ /
/// <explicit-track-list> ]?` — each string a row of the areas, its size
/// a row track (`auto` when omitted), the names after one row and before
/// the next naming the same line, and the columns `none` when omitted.
/// It serializes back in that form, `auto` sizes left out.
#[test]
fn grid_template_takes_areas_with_row_sizes_and_line_names() {
    let mut style = TuiStyle::new();
    set(
        "grid-template",
        "[top] \"a a\" 1 [mid] [m2] \"b .\" [bot] / 2 1fr",
        &mut style,
    )
    .unwrap();
    assert_eq!(
        style.grid_template_areas,
        Some(Value::Specified(
            GridTemplateAreas::new(["a a", "b ."]).unwrap()
        ))
    );
    assert_eq!(
        serialized("grid-template-rows", &style).as_deref(),
        Some("[top] 1 [mid m2] auto [bot]")
    );
    assert_eq!(
        serialized("grid-template-columns", &style).as_deref(),
        Some("2 1fr")
    );
    assert_eq!(
        serialized("grid-template", &style).as_deref(),
        Some("[top] \"a a\" 1 [mid m2] \"b .\" [bot] / 2 1fr")
    );
    set("grid-template", "\"x\" \"y\"", &mut style).unwrap();
    assert_eq!(
        style.grid_template_columns,
        Some(Value::Specified(GridTemplate::None))
    );
    assert_eq!(
        serialized("grid-template", &style).as_deref(),
        Some("\"x\" \"y\"")
    );
}

/// §7.4: the areas form takes no `repeat()` in its columns and one
/// string per row; a bare track list without `/` is not a value.
#[test]
fn grid_template_rejects_malformed_values() {
    for bad in [
        "",
        "1 2",
        "1 / 2 / 3",
        "\"a\" / repeat(2, 1)",
        "\"a b\" \"c\"",
        "\"a\" 1 2",
        "\"a\" [x] [y] [z] \"b\"",
        "[a] / 1",
        "\"a\" /",
        "none / \"a\"",
    ] {
        assert_eq!(
            set("grid-template", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
}

/// CSSOM §6.7.2: a shorthand serializes to "" when its longhands have no
/// form of it — here areas beside rows that repeat, or that are not one
/// size per area row.
#[test]
fn grid_template_does_not_serialize_unrepresentable_longhands() {
    let mut style = TuiStyle::new();
    set("grid-template-areas", "\"a\" \"b\"", &mut style).unwrap();
    set("grid-template-rows", "repeat(2, 1)", &mut style).unwrap();
    set("grid-template-columns", "none", &mut style).unwrap();
    assert_eq!(serialized("grid-template", &style), None);
    set("grid-template-rows", "1", &mut style).unwrap();
    assert_eq!(serialized("grid-template", &style), None);
    set("grid-template-rows", "1 2", &mut style).unwrap();
    assert_eq!(
        serialized("grid-template", &style).as_deref(),
        Some("\"a\" 1 \"b\" 2")
    );
}

/// §7.8: `grid: <'grid-template'>` sets the template and resets the
/// implicit grid — `grid-auto-rows` / `-columns` to `auto`,
/// `grid-auto-flow` to `row` — and not the gutters.
#[test]
fn grid_resets_the_implicit_grid() {
    let mut style = TuiStyle::new();
    set("grid-auto-flow", "column dense", &mut style).unwrap();
    set("grid-auto-rows", "3", &mut style).unwrap();
    set("gap", "1", &mut style).unwrap();
    set("grid", "\"a\" 2 / 4", &mut style).unwrap();
    assert_eq!(
        style.grid_auto_flow,
        Some(Value::Specified(GridAutoFlow::ROW))
    );
    assert_eq!(
        style.grid_auto_rows,
        Some(Value::Specified(vec![TrackSize::AUTO]))
    );
    assert!(style.row_gap.is_some());
    assert_eq!(serialized("grid", &style).as_deref(), Some("\"a\" 2 / 4"));
    assert_eq!(
        property_mask("grid"),
        Some(
            ImportantMask::GRID_TEMPLATE_ROWS
                | ImportantMask::GRID_TEMPLATE_COLUMNS
                | ImportantMask::GRID_TEMPLATE_AREAS
                | ImportantMask::GRID_AUTO_ROWS
                | ImportantMask::GRID_AUTO_COLUMNS
                | ImportantMask::GRID_AUTO_FLOW
        )
    );
}

/// §7.8: `<'grid-template-rows'> / [ auto-flow && dense? ]
/// <'grid-auto-columns'>?` flows by column, and `[ auto-flow && dense? ]
/// <'grid-auto-rows'>? / <'grid-template-columns'>` by row; the other
/// axis's template, the areas and the other implicit size are reset. Each
/// serializes back in its form; implicit properties the grid-template
/// form cannot hold need one of them.
#[test]
fn grid_takes_the_auto_flow_forms() {
    let mut style = TuiStyle::new();
    set("grid-template-areas", "\"a\"", &mut style).unwrap();
    set("grid", "1 / dense auto-flow 2 3", &mut style).unwrap();
    assert_eq!(
        style.grid_auto_flow,
        Some(Value::Specified(GridAutoFlow::COLUMN.dense()))
    );
    assert_eq!(
        style.grid_template_areas,
        Some(Value::Specified(GridTemplateAreas::NONE))
    );
    assert_eq!(
        serialized("grid-auto-columns", &style).as_deref(),
        Some("2 3")
    );
    assert_eq!(
        serialized("grid", &style).as_deref(),
        Some("1 / auto-flow dense 2 3")
    );
    set("grid", "auto-flow 2 / 1 1fr", &mut style).unwrap();
    assert_eq!(
        serialized("grid-template-rows", &style).as_deref(),
        Some("none")
    );
    assert_eq!(serialized("grid-auto-rows", &style).as_deref(), Some("2"));
    assert_eq!(
        serialized("grid", &style).as_deref(),
        Some("auto-flow 2 / 1 1fr")
    );
    set("grid", "auto-flow dense / 1", &mut style).unwrap();
    assert_eq!(
        serialized("grid", &style).as_deref(),
        Some("auto-flow dense / 1")
    );
    // A row flow with `auto` rows is the grid-template form.
    set("grid", "auto-flow / 1", &mut style).unwrap();
    assert_eq!(serialized("grid", &style).as_deref(), Some("none / 1"));
    // A column flow beside columns has no form of `grid`.
    set("grid-template-columns", "1", &mut style).unwrap();
    set("grid-auto-flow", "column", &mut style).unwrap();
    assert_eq!(serialized("grid", &style), None);
    for bad in [
        "auto-flow / auto-flow",
        "1 / 2 / 3",
        "auto-flow dense dense / 1",
        "dense / 1",
        "auto-flow [a] 1 / 2",
        "1 / auto-flow repeat(2, 1)",
        "\"a\" / auto-flow",
    ] {
        assert_eq!(
            set("grid", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
}
