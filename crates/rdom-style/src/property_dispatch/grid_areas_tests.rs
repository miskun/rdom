//! Dispatch tests for named grid areas (C7-GRID-AREAS, CSS Grid Layout
//! 2 §7.3): `grid-template-areas`.

use super::*;
use crate::layout::{GridTemplateAreas, NamedArea};
use crate::{ImportantMask, TuiStyle, Value};

/// CSS Grid 2 §7.3: `grid-template-areas: none | <string>+`, initial
/// `none`, not inherited. Each string is a row; a run of name code
/// points is a named cell token, a run of `.` a null cell token, and
/// whitespace separates them. The value serializes with each string's
/// tokens one space apart and every null cell token a single `.`, as
/// browsers serialize it.
#[test]
fn grid_template_areas_take_strings_of_cell_tokens() {
    let mut style = TuiStyle::new();
    set(
        "grid-template-areas",
        "\"head head\" 'nav  main' \"... foot\"",
        &mut style,
    )
    .unwrap();
    let areas = GridTemplateAreas::new(["head head", "nav main", ". foot"]).unwrap();
    assert_eq!(
        style.grid.grid_template_areas,
        Some(Value::Specified(areas.clone()))
    );
    assert_eq!(
        serialize("grid-template-areas", &style).as_deref(),
        Some("\"head head\" \"nav main\" \". foot\"")
    );
    assert_eq!((areas.row_count(), areas.column_count()), (3, 2));
    assert_eq!(
        areas.areas(),
        [
            NamedArea {
                name: "head".into(),
                rows: 0..1,
                columns: 0..2
            },
            NamedArea {
                name: "nav".into(),
                rows: 1..2,
                columns: 0..1
            },
            NamedArea {
                name: "main".into(),
                rows: 1..2,
                columns: 1..2
            },
            NamedArea {
                name: "foot".into(),
                rows: 2..3,
                columns: 1..2
            },
        ]
    );
    set("grid-template-areas", "None", &mut style).unwrap();
    assert_eq!(
        serialize("grid-template-areas", &style).as_deref(),
        Some("none")
    );
    assert!(!inherits("grid-template-areas"));
    assert_eq!(
        property_mask("grid-template-areas"),
        Some(ImportantMask::GRID_TEMPLATE_AREAS)
    );
    assert_eq!(
        crate::ComputedStyle::initial().grid.grid_template_areas,
        GridTemplateAreas::NONE
    );
}

/// §7.3: the declaration is invalid when the strings do not all hold the
/// same number of cell tokens, when a string holds a trash token, or when
/// a name's cells do not form a single filled rectangle.
#[test]
fn grid_template_areas_reject_non_rectangles_and_trash() {
    for bad in [
        "",
        "\"a b\" \"c\"",
        "\"a b a\"",
        "\"a a\" \"a b\"",
        "\"a b\" \"b a\"",
        "\"a !\"",
        "\"\"",
        "a",
        "\"a\" none",
    ] {
        assert_eq!(
            set("grid-template-areas", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
    // An L-shape is not a rectangle; a 2×2 block is.
    assert!(GridTemplateAreas::new(["a a", "a ."]).is_none());
    assert!(GridTemplateAreas::new(["a a", "a a"]).is_some());
    // Name code points include digits, `-`, `_` and non-ASCII.
    assert!(GridTemplateAreas::new(["1 x-y _z é"]).is_some());
}

/// The builder writes what the declaration would.
#[test]
fn grid_template_areas_builder_writes_the_declaration() {
    let style =
        TuiStyle::new().grid_template_areas_important(GridTemplateAreas::new(["a b"]).unwrap());
    assert_eq!(
        serialize("grid-template-areas", &style).as_deref(),
        Some("\"a b\"")
    );
    assert!(style.important.contains(ImportantMask::GRID_TEMPLATE_AREAS));
}
