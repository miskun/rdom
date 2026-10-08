//! Dispatch tests for the table properties (C13-TFC): the `display`
//! values of CSS 2.1 §17.2 / CSS Display 3 §2.4 — `table`, `inline-table`
//! and the layout-internal ones — `table-layout` (§17.5.2) and
//! `caption-side` (§17.4.1).

use super::*;
use crate::layout::{CaptionSide, Display, EmptyCells, Flow, TableLayout, TablePart};
use crate::{TuiStyle, Value};

fn spec<T: Clone>(v: &Option<Value<T>>) -> Option<T> {
    match v {
        Some(Value::Specified(x)) => Some(x.clone()),
        _ => None,
    }
}

/// The `(display, flow)` a `display` value declares.
fn declared(css: &str) -> (Display, Flow) {
    let mut style = TuiStyle::new();
    set("display", css, &mut style).unwrap_or_else(|e| panic!("{css}: {e:?}"));
    (spec(&style.display).unwrap(), spec(&style.flow).unwrap())
}

/// CSS Display 3 §2.2: `table` is an inner display type — `table` is
/// `block table`, `inline-table` `inline table` (§2.7) — and §2.4's
/// layout-internal keywords stand alone, each a block container inside
/// where it holds content (a cell or a caption is `flow-root`).
#[test]
fn table_display_values_parse() {
    use TablePart::*;
    for (css, want) in [
        ("table", (Display::Block, Flow::Table)),
        ("block table", (Display::Block, Flow::Table)),
        ("table block", (Display::Block, Flow::Table)),
        ("inline-table", (Display::Inline, Flow::Table)),
        ("inline table", (Display::Inline, Flow::Table)),
        ("TABLE INLINE", (Display::Inline, Flow::Table)),
        (
            "table-row-group",
            (Display::TablePart(RowGroup), Flow::Block),
        ),
        (
            "table-header-group",
            (Display::TablePart(HeaderGroup), Flow::Block),
        ),
        (
            "table-footer-group",
            (Display::TablePart(FooterGroup), Flow::Block),
        ),
        ("table-row", (Display::TablePart(Row), Flow::Block)),
        ("table-cell", (Display::TablePart(Cell), Flow::FlowRoot)),
        (
            "table-column-group",
            (Display::TablePart(ColumnGroup), Flow::Block),
        ),
        ("table-column", (Display::TablePart(Column), Flow::Block)),
        (
            "table-caption",
            (Display::TablePart(Caption), Flow::FlowRoot),
        ),
    ] {
        assert_eq!(declared(css), want, "{css}");
    }
}

/// §2: the internal keywords take no other keyword, and `table` is no
/// list item (`list-item` takes `flow` / `flow-root` only, §2.3).
#[test]
fn invalid_table_display_combinations_are_rejected() {
    for bad in [
        "table list-item",
        "table flex",
        "table table",
        "block table-row",
        "table-cell inline",
        "inline-table block",
    ] {
        let mut style = TuiStyle::new();
        assert_eq!(
            set("display", bad, &mut style),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
}

/// CSSOM §6.7.2: the shortest form — `table`, `inline-table`, and each
/// internal keyword as itself.
#[test]
fn table_display_values_serialize() {
    for (css, out) in [
        ("block table", "table"),
        ("inline table", "inline-table"),
        ("table-row-group", "table-row-group"),
        ("table-header-group", "table-header-group"),
        ("table-footer-group", "table-footer-group"),
        ("table-row", "table-row"),
        ("table-cell", "table-cell"),
        ("table-column-group", "table-column-group"),
        ("table-column", "table-column"),
        ("table-caption", "table-caption"),
    ] {
        let mut style = TuiStyle::new();
        set("display", css, &mut style).unwrap();
        assert_eq!(serialize("display", &style).as_deref(), Some(out), "{css}");
    }
}

/// CSS 2.1 §17.5.2 (and CSS Tables 3's `table-layout`): `auto | fixed`,
/// not inherited, initial `auto`.
#[test]
fn table_layout_parses_serializes_and_does_not_inherit() {
    let mut style = TuiStyle::new();
    set("table-layout", "FIXED", &mut style).unwrap();
    assert_eq!(spec(&style.table.table_layout), Some(TableLayout::Fixed));
    assert_eq!(serialize("table-layout", &style).as_deref(), Some("fixed"));
    set("table-layout", "auto", &mut style).unwrap();
    assert_eq!(serialize("table-layout", &style).as_deref(), Some("auto"));
    assert_eq!(
        set("table-layout", "fixed auto", &mut style),
        Err(DispatchError::InvalidValue)
    );
    assert!(!inherits("table-layout"));
    assert_eq!(
        crate::ComputedStyle::initial().table.table_layout,
        TableLayout::Auto
    );
}

/// CSS 2.1 §17.4.1 and CSS Tables 3's `caption-side: top | bottom`,
/// relative to the table's writing mode — the block-start and block-end
/// sides, so the logical `block-start` / `block-end` are the same two):
/// inherited, initial `top`.
#[test]
fn caption_side_parses_serializes_and_inherits() {
    let mut style = TuiStyle::new();
    for (css, want, out) in [
        ("bottom", CaptionSide::Bottom, "bottom"),
        ("TOP", CaptionSide::Top, "top"),
        ("block-end", CaptionSide::Bottom, "bottom"),
        ("block-start", CaptionSide::Top, "top"),
    ] {
        set("caption-side", css, &mut style).unwrap();
        assert_eq!(spec(&style.table.caption_side), Some(want), "{css}");
        assert_eq!(serialize("caption-side", &style).as_deref(), Some(out));
    }
    for bad in ["left", "inline-start", "top bottom"] {
        assert_eq!(
            set("caption-side", bad, &mut style),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
    assert!(inherits("caption-side"));
    assert_eq!(
        crate::ComputedStyle::initial().table.caption_side,
        CaptionSide::Top
    );
}

/// CSS 2.1 §17.6.1.1: `empty-cells: show | hide`, inherited, initial
/// `show`.
#[test]
fn empty_cells_parses_serializes_and_inherits() {
    let mut style = TuiStyle::new();
    set("empty-cells", "HIDE", &mut style).unwrap();
    assert_eq!(spec(&style.table.empty_cells), Some(EmptyCells::Hide));
    assert_eq!(serialize("empty-cells", &style).as_deref(), Some("hide"));
    set("empty-cells", "show", &mut style).unwrap();
    assert_eq!(serialize("empty-cells", &style).as_deref(), Some("show"));
    assert_eq!(
        set("empty-cells", "none", &mut style),
        Err(DispatchError::InvalidValue)
    );
    assert!(inherits("empty-cells"));
    assert_eq!(
        crate::ComputedStyle::initial().table.empty_cells,
        EmptyCells::Show
    );
}
