//! Dispatch tests for `display` (C6-DISPLAY-KEYWORDS, CSS Display 3 §2):
//! the box keywords, the multi-keyword syntax and its mapping onto
//! rdom's outer [`Display`] / inner [`Flow`] pair and the list-item
//! flag, and the shortest serialization.

use super::*;
use crate::layout::{Display, Flow};
use crate::{TuiStyle, Value};

/// The `(display, flow, list_item)` a `display` value declares.
fn declared(css: &str) -> (Display, Flow, bool) {
    let mut style = TuiStyle::new();
    set("display", css, &mut style).unwrap_or_else(|e| panic!("{css}: {e:?}"));
    fn get<T: Copy + std::fmt::Debug>(v: &Option<Value<T>>) -> T {
        match v {
            Some(Value::Specified(x)) => *x,
            other => panic!("{other:?}"),
        }
    }
    (get(&style.display), get(&style.flow), get(&style.list_item))
}

/// CSS Display 3 §2: the outer and inner display types, in either
/// order, an omitted outer type `block` and an omitted inner type
/// `flow` (§2.1 / §2.2); the legacy single keywords are their
/// two-keyword forms (§2.7); `list-item` with an optional outer type
/// and `flow` / `flow-root` (§2.3); `contents` and `none` (§2.5).
#[test]
fn display_keywords_map_onto_outer_and_inner() {
    use Display::{Block, Contents, Inline, InlineBlock, None as NoBox};
    use Flow::{Block as FlowB, Flex, FlowRoot};
    for (css, want) in [
        ("block", (Block, FlowB, false)),
        ("block flow", (Block, FlowB, false)),
        ("flow block", (Block, FlowB, false)),
        ("flow", (Block, FlowB, false)),
        ("flow-root", (Block, FlowRoot, false)),
        ("block flow-root", (Block, FlowRoot, false)),
        ("inline", (Inline, FlowB, false)),
        ("inline flow", (Inline, FlowB, false)),
        ("inline-block", (InlineBlock, FlowB, false)),
        ("inline flow-root", (InlineBlock, FlowB, false)),
        ("flow-root inline", (InlineBlock, FlowB, false)),
        ("flex", (Block, Flex, false)),
        ("block flex", (Block, Flex, false)),
        ("inline-flex", (Inline, Flex, false)),
        ("inline flex", (Inline, Flex, false)),
        ("FLEX INLINE", (Inline, Flex, false)),
        ("list-item", (Block, FlowB, true)),
        ("block list-item", (Block, FlowB, true)),
        ("list-item block flow", (Block, FlowB, true)),
        ("inline list-item", (Inline, FlowB, true)),
        ("list-item flow-root", (Block, FlowRoot, true)),
        ("inline flow-root list-item", (InlineBlock, FlowB, true)),
        ("contents", (Contents, FlowB, false)),
        ("none", (NoBox, FlowB, false)),
    ] {
        assert_eq!(declared(css), want, "{css}");
    }
}

/// CSS Display 3 §2: each component at most once, `list-item` only
/// with `flow` / `flow-root`, the box and legacy keywords alone; rdom
/// has no `run-in` or `ruby` layout, so those are invalid here (the
/// declaration is dropped); `table` is valid since C13-TFC
/// (`table_tests.rs`).
#[test]
fn invalid_display_combinations_are_rejected() {
    for bad in [
        "block inline",
        "flex flow",
        "flex list-item",
        "list-item list-item",
        "contents block",
        "none flow",
        "inline-block flow",
        "inline flex flow",
        "run-in",
        "grid list-item",
        "grid flex",
        "inline-grid flow",
        "grid grid",
        "ruby",
        "",
    ] {
        let mut style = TuiStyle::new();
        assert_eq!(
            set("display", bad, &mut style),
            Err(DispatchError::InvalidValue),
            "{bad:?}"
        );
    }
}

/// CSSOM §6.7.2 with CSS Display 3 §2: the shortest form — the legacy
/// keyword where there is one, the list-item forms as Chromium writes
/// them.
#[test]
fn display_serializes_in_the_shortest_form() {
    for (css, out) in [
        ("block flow", "block"),
        ("flow", "block"),
        ("block flow-root", "flow-root"),
        ("inline flow", "inline"),
        ("inline flow-root", "inline-block"),
        ("block flex", "flex"),
        ("inline flex", "inline-flex"),
        ("block grid", "grid"),
        ("inline grid", "inline-grid"),
        ("block flow list-item", "list-item"),
        ("inline list-item", "inline list-item"),
        ("flow-root list-item", "flow-root list-item"),
        ("inline flow-root list-item", "inline flow-root list-item"),
        ("contents", "contents"),
        ("none", "none"),
    ] {
        let mut style = TuiStyle::new();
        set("display", css, &mut style).unwrap();
        assert_eq!(serialize("display", &style).as_deref(), Some(out), "{css}");
    }
}

/// `display` owns its three fields: `inherit` and removal reach the
/// inner type and the list-item flag too, and a later `display: inline`
/// leaves no flex inner type behind.
#[test]
fn display_owns_inner_type_and_list_item() {
    let mut style = TuiStyle::new();
    set("display", "flex", &mut style).unwrap();
    set("display", "inline", &mut style).unwrap();
    assert_eq!(style.flow, Some(Value::Specified(Flow::Block)));
    set("display", "inherit", &mut style).unwrap();
    assert_eq!(style.list_item, Some(Value::Inherit));
    assert!(remove("display", &mut style));
    assert_eq!(
        (style.display, style.flow, style.list_item),
        (None, None, None)
    );
}

/// CSS Display 3 §2.2 / §2.7, CSS Grid 2 §5.1: `grid` is an inner
/// display type — a block-level grid container alone or with `block`,
/// an inline-level one with `inline` or as the legacy `inline-grid`.
#[test]
fn grid_is_an_inner_display_type() {
    use Display::{Block, Inline};
    for (css, want) in [
        ("grid", (Block, Flow::Grid, false)),
        ("block grid", (Block, Flow::Grid, false)),
        ("grid block", (Block, Flow::Grid, false)),
        ("inline-grid", (Inline, Flow::Grid, false)),
        ("inline grid", (Inline, Flow::Grid, false)),
        ("GRID INLINE", (Inline, Flow::Grid, false)),
    ] {
        assert_eq!(declared(css), want, "{css}");
    }
    assert!(Flow::Grid.is_flex_or_grid() && Flow::Flex.is_flex_or_grid());
    assert!(!Flow::Grid.is_block_flow() && !Flow::FlowRoot.is_flex_or_grid());
}

/// C8G-WEBKIT-CLAMP (API N9): the Compat Standard's legacy keywords read
/// back as written, as browsers serialize them (`getComputedStyle` gives
/// `-webkit-box`), and a later `display` clears the legacy flag.
#[test]
fn the_legacy_webkit_box_keywords_read_back() {
    for kw in ["-webkit-box", "-webkit-inline-box"] {
        let mut style = TuiStyle::new();
        set("display", kw, &mut style).unwrap();
        assert_eq!(serialize("display", &style).as_deref(), Some(kw));
    }
    let mut style = TuiStyle::new();
    set("display", "-webkit-box", &mut style).unwrap();
    set("display", "flex", &mut style).unwrap();
    assert_eq!(serialize("display", &style).as_deref(), Some("flex"));
}
