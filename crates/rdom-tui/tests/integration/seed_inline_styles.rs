//! `cssom::seed_inline_styles` — the `style="…"` attribute walker
//! that writes parsed `TuiStyle` into each element's
//! `TuiExt::inline_style`. Pure-parser declaration tests live in
//! `rdom-css/tests/inline_style.rs`; this file covers the tree-
//! walking glue that bridges the parser and the cascade.

use rdom_style::{Color, ImportantMask, TuiColor, Value};
use rdom_tui::{TuiDom, TuiNodeExt, seed_inline_styles};

#[test]
fn seed_writes_inline_style_for_attribute() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.set_attribute(div, "style", "color: red; gap: 1")
        .unwrap();
    dom.append_child(root, div).unwrap();

    let warnings = seed_inline_styles(&mut dom);
    assert!(warnings.is_empty());

    let inline = dom.node(div).inline_style().expect("inline_style present");
    assert_eq!(
        inline.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
    assert_eq!(inline.gap, Some(Value::Specified(1)));
}

#[test]
fn seed_parses_min_max_width_height_from_css() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.set_attribute(
        div,
        "style",
        "min-width: 10; max-width: 100; min-height: 5; max-height: 50",
    )
    .unwrap();
    dom.append_child(root, div).unwrap();

    let warnings = seed_inline_styles(&mut dom);
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");

    let inline = dom.node(div).inline_style().expect("inline_style present");
    use rdom_style::layout::MinSize;
    assert_eq!(inline.min_width, Some(Value::Specified(MinSize::Cells(10))));
    assert_eq!(inline.max_width, Some(Value::Specified(100)));
    assert_eq!(inline.min_height, Some(Value::Specified(MinSize::Cells(5))));
    assert_eq!(inline.max_height, Some(Value::Specified(50)));
}

#[test]
fn seed_skips_elements_without_style_attribute() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    let warnings = seed_inline_styles(&mut dom);
    assert!(warnings.is_empty());

    // No style attribute → inline_style stays empty (default).
    let inline = dom.node(div).inline_style().expect("ext present");
    assert!(inline.fg.is_none());
}

#[test]
fn seed_walks_nested_elements() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("section");
    dom.set_attribute(outer, "style", "color: red").unwrap();
    let inner = dom.create_element("p");
    dom.set_attribute(inner, "style", "color: blue").unwrap();
    dom.append_child(outer, inner).unwrap();
    dom.append_child(root, outer).unwrap();

    seed_inline_styles(&mut dom);

    let outer_fg = dom.node(outer).inline_style().unwrap().fg.clone();
    let inner_fg = dom.node(inner).inline_style().unwrap().fg.clone();
    assert_eq!(
        outer_fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
    assert_eq!(
        inner_fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(0, 0, 255))))
    );
}

#[test]
fn seed_propagates_warnings_from_inner_parse() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.set_attribute(div, "style", "unknown-prop: 5").unwrap();
    dom.append_child(root, div).unwrap();

    let warnings = seed_inline_styles(&mut dom);
    assert_eq!(warnings.len(), 1);
}

#[test]
fn seed_inline_style_important_bit_propagates() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.set_attribute(div, "style", "color: red !important")
        .unwrap();
    dom.append_child(root, div).unwrap();

    seed_inline_styles(&mut dom);
    let inline = dom.node(div).inline_style().unwrap();
    assert!(inline.important.contains(ImportantMask::FG));
}
