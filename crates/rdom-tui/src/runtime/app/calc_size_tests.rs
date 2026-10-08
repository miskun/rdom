//! C12-ANIMATABLE leftover — `calc-size()` on `min-*`, `max-*` and
//! `flex-basis` (CSS Values 5 §10), and `interpolate-size: allow-keywords`
//! animating them to and from their sizing keywords (§11).

use rdom_core::NodeId;

use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::Stylesheet;

/// `<div id=row><div id=a>xxxxxxxx</div></div>` styled by `css`.
fn sized(css: &str) -> (App<TestBackend>, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let row = dom.create_element("div");
    dom.set_attribute(row, "id", "row").unwrap();
    let a = dom.create_element("div");
    dom.set_attribute(a, "id", "a").unwrap();
    let t = dom.create_text_node("xxxxxxxx");
    dom.append_child(a, t).unwrap();
    dom.append_child(row, a).unwrap();
    dom.append_child(root, row).unwrap();
    let sheet = rdom_css::parse(css);
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(40, 12)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    (app, a)
}

fn width(app: &App<TestBackend>, id: NodeId) -> u16 {
    app.dom().node(id).ext().unwrap().layout.width
}

/// §10: `min-width: calc-size(max-content, size + 2)` floors the box at
/// its max-content width (8) plus 2; `calc-size(auto, …)` on a min size
/// takes the automatic minimum of a box that is no flex or grid item, 0.
#[test]
fn calc_size_floors_a_min_size() {
    let (app, a) = sized("#a { width: 0; min-width: calc-size(max-content, size + 2) }");
    assert_eq!(width(&app, a), 10);
    let (app, a) = sized("#a { width: 0; min-width: calc-size(auto, size + 3) }");
    assert_eq!(width(&app, a), 3);
}

/// §10: `max-width: calc-size(max-content, size * 0.5)` caps the box at
/// half its max-content width.
#[test]
fn calc_size_caps_a_max_size() {
    let (app, a) = sized("#a { max-width: calc-size(max-content, size * 0.5) }");
    assert_eq!(width(&app, a), 4);
}

/// §10: `flex-basis: calc-size(content, size + 3)` is the item's content
/// size plus 3.
#[test]
fn calc_size_sets_a_flex_basis() {
    let (app, a) = sized(
        "#row { display: flex } \
         #a { flex: 0 0 auto; flex-basis: calc-size(content, size + 3) }",
    );
    assert_eq!(width(&app, a), 11);
}

/// §11: under `allow-keywords` a `min-width` of `0` animates to
/// `max-content` (8) through `calc-size()` — 4 half-way.
#[test]
fn allow_keywords_animates_a_min_size_to_a_keyword() {
    let (mut app, a) = sized(
        "#a { interpolate-size: allow-keywords; width: 0; min-width: 0; \
         transition: min-width 100ms linear } #a.on { min-width: max-content }",
    );
    assert_eq!(width(&app, a), 0);
    app.dom_mut().set_attribute(a, "class", "on").unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    assert_eq!(width(&app, a), 4);
    app.advance(60).unwrap();
    assert_eq!(width(&app, a), 8);
}

/// CSS Values 5 §10: a `calc-size()` box's basis is its size with its
/// content laid out as it is — so a `calc-size()`d box inside another
/// (nested `<details>` accordions mid-animation) is resolved first, and
/// the outer box's `auto` counts the inner one at its resolved size, not
/// at its own basis (C12G-CARRYOVER, architect N4). Eight rows inside an
/// inner box at half its height (4) inside an outer box at half its
/// height: 2, not 4.
#[test]
fn a_nested_calc_size_resolves_inside_out() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("div");
    dom.set_attribute(outer, "id", "outer").unwrap();
    let inner = dom.create_element("div");
    dom.set_attribute(inner, "id", "inner").unwrap();
    for _ in 0..8 {
        let p = dom.create_element("div");
        let t = dom.create_text_node("x");
        dom.append_child(p, t).unwrap();
        dom.append_child(inner, p).unwrap();
    }
    dom.append_child(outer, inner).unwrap();
    dom.append_child(root, outer).unwrap();
    let sheet =
        rdom_css::parse("#outer, #inner { height: calc-size(auto, size * 0.5); overflow: hidden }");
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(20, 12)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    let height = |id: NodeId| app.dom().node(id).ext().unwrap().layout.height;
    assert_eq!(height(inner), 4);
    assert_eq!(height(outer), 2);
}

/// C13G-CALC-SIZE-AUTHORED (architect N6) — Chrome's documented accordion,
/// `details[open]::details-content { height: calc-size(auto, size) }`,
/// five `<details>` deep, all open and at rest: each box's sum gives back
/// the size its basis laid it out at, so no level is laid out again — one
/// pass a layout, not `d + 2` (7) — and every box keeps its `auto` height.
#[test]
fn an_authored_identity_calc_size_costs_no_extra_pass() {
    use crate::render::layout_pass::ROUNDS;
    use crate::{LayoutExt, TuiNodeExt};
    let mut dom: TuiDom = TuiDom::new();
    let mut parent = dom.root();
    let mut summaries = Vec::new();
    for level in 0..5 {
        let details = dom.create_element("details");
        dom.set_attribute(details, "open", "").unwrap();
        let summary = dom.create_element("summary");
        let t = dom.create_text_node(&format!("level {level}"));
        dom.append_child(summary, t).unwrap();
        dom.append_child(details, summary).unwrap();
        let p = dom.create_element("p");
        let t = dom.create_text_node("body");
        dom.append_child(p, t).unwrap();
        dom.append_child(details, p).unwrap();
        dom.append_child(parent, details).unwrap();
        summaries.push(summary);
        parent = details;
    }
    let sheet = rdom_css::parse(
        "details[open]::details-content { height: calc-size(auto, size) } \
         details, summary, p { margin: 0 }",
    );
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(30, 20)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    ROUNDS.with(|c| c.set(0));
    app.dom_mut()
        .layout_dom(crate::render::Rect::new(0, 0, 30, 20));
    assert_eq!(ROUNDS.with(std::cell::Cell::get), 1, "layout passes");
    // Every level open at its `auto` height: its summary row, its body
    // row, then the next level.
    let ys: Vec<i32> = summaries
        .iter()
        .map(|&s| app.dom().node(s).layout_rect().unwrap().y)
        .collect();
    assert_eq!(ys, [0, 2, 4, 6, 8]);
}
