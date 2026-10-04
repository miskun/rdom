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
    assert_eq!(
        inline.gap,
        Some(Value::Specified(rdom_style::layout::GapValue::Cells(1)))
    );
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
    assert_eq!(
        inline.max_width,
        Some(Value::Specified(Some(rdom_tui::layout::MaxSize::Cells(
            100
        ))))
    );
    assert_eq!(inline.min_height, Some(Value::Specified(MinSize::Cells(5))));
    assert_eq!(
        inline.max_height,
        Some(Value::Specified(Some(rdom_tui::layout::MaxSize::Cells(50))))
    );
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

// ── P7G-INLINE-STYLE-SEED-1: the App seeds mount-time inline styles ──

/// Parse `markup` into a fresh `TuiDom` (optionally seeding its inline
/// styles first), mount it in an `App` and draw one frame; returns the
/// App and the markup's `<p>`.
fn mounted(
    markup: &str,
    seed_first: bool,
) -> (
    rdom_tui::runtime::app::App<rdom_tui::render::TestBackend>,
    rdom_tui::NodeId,
) {
    use rdom_tui::render::{Terminal, TestBackend};
    use rdom_tui::runtime::app::App;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, markup, root).expect("markup parses");
    if seed_first {
        assert!(seed_inline_styles(&mut dom).is_empty());
    }
    let p = dom
        .query_selector_in(root, "p")
        .unwrap()
        .expect("markup has a <p>");
    let terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();
    let mut app = App::with_backend(dom, rdom_tui::Stylesheet::new(), terminal).unwrap();
    app.draw_if_dirty().unwrap();
    (app, p)
}

fn computed_fg(
    app: &rdom_tui::runtime::app::App<rdom_tui::render::TestBackend>,
    id: rdom_tui::NodeId,
) -> Color {
    app.dom().node(id).computed().expect("cascaded").fg
}

#[test]
fn an_inline_style_in_parsed_markup_applies_under_the_app_without_seeding() {
    let (app, p) = mounted(r#"<p style="color: red">hi</p>"#, false);
    assert_eq!(computed_fg(&app, p), Color::Rgb(255, 0, 0));
}

#[test]
fn seeding_before_the_app_does_not_change_the_result() {
    let markup = r#"<p style="color: red; --x: 1">hi</p>"#;
    let (unseeded, p) = mounted(markup, false);
    let (seeded, q) = mounted(markup, true);
    assert_eq!(computed_fg(&seeded, q), Color::Rgb(255, 0, 0));
    assert_eq!(
        seeded.dom().node(q).inline_style(),
        unseeded.dom().node(p).inline_style(),
        "seeding twice applies the declarations once"
    );
}
