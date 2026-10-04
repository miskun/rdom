//! `Element.scrollIntoView` alignment (`P7-SCROLL-INTO-VIEW-ALIGN-1`):
//! CSSOM View §5.2 "scroll an element into view" over every scroll
//! container on the ancestor chain, with §5.1 "determine the
//! scroll-into-view position" for each `ScrollLogicalPosition` on each
//! axis, through a headless `App` whose layout supplies the boxes.
//! Each pane sits below a spacer, so a scrollport that is not at the
//! origin of the screen is covered.

use rdom_core::NodeId;

use crate::TuiDom;
use crate::accessors::{
    ScrollIntoViewOptions, ScrollLogicalPosition, TuiAccessors, TuiAccessorsMut,
};
use crate::layout::{Direction, Display, Flow, Overflow, Size};
use crate::node::TuiNodeExt;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::{Stylesheet, TuiStyle};

use ScrollLogicalPosition::{Center, End, Nearest, Start};

fn line(dom: &mut TuiDom, parent: NodeId, text: &str) -> NodeId {
    let p = dom.create_element("p");
    let t = dom.create_text_node(text);
    dom.append_child(p, t).unwrap();
    dom.append_child(parent, p).unwrap();
    p
}

fn spacer(dom: &mut TuiDom) {
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "class", "spacer").unwrap();
    dom.append_child(root, s).unwrap();
}

fn base_sheet() -> Stylesheet {
    Stylesheet::bare()
        .rule_unchecked("p", TuiStyle::new().display(Display::Block))
        .rule_unchecked(
            ".spacer",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(2)),
        )
}

fn run(dom: TuiDom, sheet: Stylesheet) -> App<TestBackend> {
    let terminal = Terminal::new(TestBackend::new(40, 30)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    app
}

/// A 5-row vertical scroll pane two rows down: 20 one-row lines
/// (`y` 0..20), then an 8-row `#tall` block (`y` 20..28), then 5 more
/// lines — 33 rows, `scrollTop` max 28. Returns the pane, line 10 and
/// `#tall`.
fn vpane() -> (App<TestBackend>, NodeId, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    spacer(&mut dom);
    let root = dom.root();
    let pane = dom.create_element("div");
    dom.set_attribute(pane, "id", "pane").unwrap();
    dom.append_child(root, pane).unwrap();
    let mut line_10 = pane;
    for i in 0..20 {
        let l = line(&mut dom, pane, &format!("line {i}"));
        if i == 10 {
            line_10 = l;
        }
    }
    let tall = dom.create_element("div");
    dom.set_attribute(tall, "id", "tall").unwrap();
    dom.append_child(pane, tall).unwrap();
    for i in 0..5 {
        line(&mut dom, pane, &format!("after {i}"));
    }
    let sheet = base_sheet()
        .rule_unchecked(
            "#pane",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(20))
                .height(Size::Fixed(5))
                .overflow_y(Overflow::Auto),
        )
        .rule_unchecked(
            "#tall",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(8)),
        );
    (run(dom, sheet), pane, line_10, tall)
}

/// A 10-column horizontal scroll pane two rows down holding one row of
/// ten 4-column cells (`x` 0..40, `scrollLeft` max 30). Returns the
/// pane and cell 5 (`x` 20..24).
fn hpane() -> (App<TestBackend>, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    spacer(&mut dom);
    let root = dom.root();
    let pane = dom.create_element("div");
    dom.set_attribute(pane, "id", "hpane").unwrap();
    dom.append_child(root, pane).unwrap();
    let strip = dom.create_element("div");
    dom.set_attribute(strip, "id", "strip").unwrap();
    dom.append_child(pane, strip).unwrap();
    let mut cell_5 = strip;
    for i in 0..10 {
        let c = dom.create_element("span");
        dom.set_attribute(c, "class", "cell").unwrap();
        dom.append_child(strip, c).unwrap();
        if i == 5 {
            cell_5 = c;
        }
    }
    let sheet = base_sheet()
        .rule_unchecked(
            "#hpane",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10))
                .height(Size::Fixed(3))
                .overflow_x(Overflow::Auto)
                .overflow_y(Overflow::Hidden),
        )
        .rule_unchecked(
            "#strip",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .width(Size::Fixed(40))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked(
            ".cell",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(4))
                .height(Size::Fixed(1))
                .flex_shrink(0.0),
        );
    (run(dom, sheet), pane, cell_5)
}

fn top(app: &App<TestBackend>, id: NodeId) -> i32 {
    app.dom().node(id).scroll_top().unwrap()
}

fn left(app: &App<TestBackend>, id: NodeId) -> i32 {
    app.dom().node(id).scroll_left().unwrap()
}

/// Scroll `pane` to `(x, y)` and lay out, as a user scroll would.
fn scrolled(app: &mut App<TestBackend>, pane: NodeId, x: i32, y: i32) {
    app.dom_mut().node_mut(pane).scroll_to(x, y).unwrap();
    app.advance(0).unwrap();
}

fn block(pos: ScrollLogicalPosition) -> ScrollIntoViewOptions {
    ScrollIntoViewOptions::new().block(pos)
}

fn inline(pos: ScrollLogicalPosition) -> ScrollIntoViewOptions {
    ScrollIntoViewOptions::new().inline(pos)
}

fn into_view(app: &mut App<TestBackend>, id: NodeId, options: ScrollIntoViewOptions) {
    app.dom_mut()
        .node_mut(id)
        .scroll_into_view_with(options)
        .unwrap();
}

#[test]
fn the_options_default_to_block_start_inline_nearest() {
    let o = ScrollIntoViewOptions::new();
    assert_eq!((o.block, o.inline), (Start, Nearest));
}

#[test]
fn the_legacy_boolean_maps_to_start_or_end_with_inline_nearest() {
    let t = ScrollIntoViewOptions::from(true);
    assert_eq!((t.block, t.inline), (Start, Nearest));
    let f = ScrollIntoViewOptions::from(false);
    assert_eq!((f.block, f.inline), (End, Nearest));
}

// ── Block axis ──────────────────────────────────────────────────────

#[test]
fn block_start_aligns_the_top_edges() {
    let (mut app, pane, line_10, _) = vpane();
    into_view(&mut app, line_10, block(Start));
    assert_eq!(top(&app, pane), 10);
}

#[test]
fn block_end_aligns_the_bottom_edges() {
    let (mut app, pane, line_10, _) = vpane();
    into_view(&mut app, line_10, block(End));
    assert_eq!(top(&app, pane), 6);
}

#[test]
fn block_center_centers_the_element_in_the_scrollport() {
    let (mut app, pane, line_10, _) = vpane();
    into_view(&mut app, line_10, block(Center));
    assert_eq!(top(&app, pane), 8);
}

#[test]
fn block_nearest_below_the_scrollport_aligns_the_bottom_edges() {
    let (mut app, pane, line_10, _) = vpane();
    into_view(&mut app, line_10, block(Nearest));
    assert_eq!(top(&app, pane), 6);
}

#[test]
fn block_nearest_above_the_scrollport_aligns_the_top_edges() {
    let (mut app, pane, line_10, _) = vpane();
    scrolled(&mut app, pane, 0, 15);
    into_view(&mut app, line_10, block(Nearest));
    assert_eq!(top(&app, pane), 10);
}

#[test]
fn block_nearest_leaves_a_visible_element_alone() {
    let (mut app, pane, line_10, _) = vpane();
    scrolled(&mut app, pane, 0, 8);
    into_view(&mut app, line_10, block(Nearest));
    assert_eq!(top(&app, pane), 8);
}

#[test]
fn block_nearest_leaves_an_element_covering_the_scrollport_alone() {
    let (mut app, pane, _, tall) = vpane();
    scrolled(&mut app, pane, 0, 22);
    into_view(&mut app, tall, block(Nearest));
    assert_eq!(top(&app, pane), 22);
}

#[test]
fn block_nearest_aligns_the_top_of_an_element_taller_than_the_scrollport() {
    let (mut app, pane, _, tall) = vpane();
    into_view(&mut app, tall, block(Nearest));
    assert_eq!(top(&app, pane), 20, "never scroll past its top edge");
}

#[test]
fn block_end_is_clamped_to_the_scroll_range() {
    let (mut app, pane, _, tall) = vpane();
    into_view(&mut app, tall, block(Start));
    assert_eq!(top(&app, pane), 20);
    let last = app.dom().node(pane).last_element_child().unwrap().id();
    into_view(&mut app, last, block(Start));
    assert_eq!(top(&app, pane), 28, "max scrollTop");
}

#[test]
fn offsets_written_since_the_last_layout_are_accounted_for() {
    let (mut app, pane, line_10, _) = vpane();
    // No frame between the two scrolls: the boxes are from the layout
    // at scrollTop 0.
    app.dom_mut().node_mut(pane).set_scroll_top(15).unwrap();
    into_view(&mut app, line_10, block(Nearest));
    assert_eq!(top(&app, pane), 10);
}

// ── Inline axis ─────────────────────────────────────────────────────

#[test]
fn inline_start_aligns_the_left_edges() {
    let (mut app, pane, cell_5) = hpane();
    into_view(&mut app, cell_5, inline(Start));
    assert_eq!(left(&app, pane), 20);
}

#[test]
fn inline_end_aligns_the_right_edges() {
    let (mut app, pane, cell_5) = hpane();
    into_view(&mut app, cell_5, inline(End));
    assert_eq!(left(&app, pane), 14);
}

#[test]
fn inline_center_centers_the_element_in_the_scrollport() {
    let (mut app, pane, cell_5) = hpane();
    into_view(&mut app, cell_5, inline(Center));
    assert_eq!(left(&app, pane), 17);
}

#[test]
fn inline_nearest_right_of_the_scrollport_aligns_the_right_edges() {
    let (mut app, pane, cell_5) = hpane();
    into_view(&mut app, cell_5, inline(Nearest));
    assert_eq!(left(&app, pane), 14);
}

#[test]
fn inline_nearest_left_of_the_scrollport_aligns_the_left_edges() {
    let (mut app, pane, cell_5) = hpane();
    scrolled(&mut app, pane, 30, 0);
    into_view(&mut app, cell_5, inline(Nearest));
    assert_eq!(left(&app, pane), 20);
}

#[test]
fn inline_nearest_leaves_a_visible_element_alone() {
    let (mut app, pane, cell_5) = hpane();
    scrolled(&mut app, pane, 16, 0);
    into_view(&mut app, cell_5, inline(Nearest));
    assert_eq!(left(&app, pane), 16);
}

// ── Legacy forms ────────────────────────────────────────────────────

#[test]
fn scroll_into_view_without_arguments_is_block_start_inline_nearest() {
    let (mut app, pane, line_10, _) = vpane();
    app.dom_mut().node_mut(line_10).scroll_into_view().unwrap();
    assert_eq!(top(&app, pane), 10);

    let (mut app, pane, cell_5) = hpane();
    app.dom_mut().node_mut(cell_5).scroll_into_view().unwrap();
    assert_eq!(left(&app, pane), 14, "inline nearest, not start");
}

#[test]
fn scroll_into_view_false_is_block_end() {
    let (mut app, pane, line_10, _) = vpane();
    into_view(&mut app, line_10, false.into());
    assert_eq!(top(&app, pane), 6);
}

// ── Nested scroll containers ────────────────────────────────────────

/// A 6-row `#outer` pane two rows down: 10 lines, a 4-row `#inner`
/// pane of 12 lines (`scrollTop` max 8), 10 more lines (24 rows,
/// `scrollTop` max 18). Returns outer, inner and inner line 8.
fn nested() -> (App<TestBackend>, NodeId, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    spacer(&mut dom);
    let root = dom.root();
    let outer = dom.create_element("div");
    dom.set_attribute(outer, "id", "outer").unwrap();
    dom.append_child(root, outer).unwrap();
    for i in 0..10 {
        line(&mut dom, outer, &format!("before {i}"));
    }
    let inner = dom.create_element("div");
    dom.set_attribute(inner, "id", "inner").unwrap();
    dom.append_child(outer, inner).unwrap();
    let mut inner_8 = inner;
    for i in 0..12 {
        let l = line(&mut dom, inner, &format!("inner {i}"));
        if i == 8 {
            inner_8 = l;
        }
    }
    for i in 0..10 {
        line(&mut dom, outer, &format!("after {i}"));
    }
    let pane = |h: u16| {
        TuiStyle::new()
            .display(Display::Block)
            .width(Size::Fixed(20))
            .height(Size::Fixed(h))
            .overflow_y(Overflow::Auto)
    };
    let sheet = base_sheet()
        .rule_unchecked("#outer", pane(6))
        .rule_unchecked("#inner", pane(4));
    (run(dom, sheet), outer, inner, inner_8)
}

#[test]
fn every_scroll_container_on_the_ancestor_chain_scrolls() {
    let (mut app, outer, inner, inner_8) = nested();
    into_view(&mut app, inner_8, block(Start));
    assert_eq!(top(&app, inner), 8, "inner: line 8 at its top");
    assert_eq!(top(&app, outer), 10, "outer: inner's scrollport at its top");
}

#[test]
fn nested_block_end_aligns_through_both_containers() {
    let (mut app, outer, inner, inner_8) = nested();
    into_view(&mut app, inner_8, block(End));
    assert_eq!(top(&app, inner), 5, "inner: line 8 on its last row");
    // Line 8 is on inner's last row, outer row 13 of its content.
    assert_eq!(top(&app, outer), 8, "outer: row 13 on its last row");
}

#[test]
fn nested_containers_paint_the_element_at_the_top_of_the_outer_pane() {
    let (mut app, outer, _, inner_8) = nested();
    into_view(&mut app, inner_8, block(Start));
    app.advance(0).unwrap();
    let port_top = app.dom().node(outer).tui_ext().unwrap().layout.y;
    let line_top = app.dom().node(inner_8).tui_ext().unwrap().layout.y;
    assert_eq!(line_top, port_top);
}

/// `P7G-INTO-VIEW-NONE-1`, CSSOM View §5.2 step 1: a descendant of a
/// `display: none` element has no box, so `scrollIntoView` returns —
/// the rect layout leaves on it (zeroed) is no position to scroll to.
#[test]
fn a_child_of_a_display_none_parent_does_not_scroll() {
    let (mut app, pane, _, tall) = vpane();
    let child = line(app.dom_mut(), tall, "inside tall");
    app.advance(0).unwrap();
    assert!(
        app.dom().node(child).tui_ext().unwrap().layout.height > 0,
        "laid out while shown"
    );
    app.dom_mut()
        .set_attribute(tall, "style", "display: none")
        .unwrap();
    scrolled(&mut app, pane, 0, 10);
    into_view(&mut app, child, block(Start));
    app.advance(0).unwrap();
    assert_eq!(top(&app, pane), 10, "no box, no scroll");
}
