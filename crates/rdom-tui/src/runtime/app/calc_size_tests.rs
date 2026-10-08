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
