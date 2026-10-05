//! C8G-DOCS — the Phase 8 API gate's test gaps: `scrollbar-width: none`
//! under hit-testing, the wheel and a drag; Shift+Tab within
//! `scroll-padding`; focus inside nested scrollers; pointer focus not
//! scrolling; `overflow: clip` not containing floats; a stuck sticky box
//! carrying its absolutely positioned child; a `.truncate` flex item.

use super::{el, lay_out, paint, rect, rows};
use crossterm::event::{
    Event as CtEvent, KeyModifiers, MouseButton, MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_tui::prelude::*;

fn mouse(app: &mut App<TestBackend>, kind: MouseEventKind, x: u16, y: u16) {
    app.handle_event(CtEvent::Mouse(CtMouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    }));
}

/// A 10 × 4 `overflow-y: scroll; scrollbar-width: none` `.s` holding six
/// `0123456789` rows, under an `App` 12 × 6: the app, `.s` and its rows.
fn barless() -> (App<TestBackend>, NodeId, Vec<NodeId>) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = el(&mut dom, root, "div", "s");
    let rows: Vec<NodeId> = (0..6)
        .map(|_| {
            let row = el(&mut dom, s, "div", "row");
            let t = dom.create_text_node("0123456789");
            dom.append_child(row, t).unwrap();
            row
        })
        .collect();
    let sheet = rdom_css::from_css_strict(
        ".s { width: 10; height: 4; overflow-y: scroll; scrollbar-width: none } \
         .row { height: 1 }",
    )
    .unwrap();
    let terminal = Terminal::new(TestBackend::new(12, 6)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    (app, s, rows)
}

/// CSS Scrollbars 1 §3 `none`: no scrollbar — the last column is
/// content, painted and hit as such — and the box still scrolls by the
/// wheel; a press-and-drag there drags no thumb.
#[test]
fn a_barless_scroller_is_content_to_its_edge_and_still_scrolls() {
    let (mut app, s, rows) = barless();
    let area = Rect::new(0, 0, 12, 6);
    let mut painted = Buffer::empty(area);
    app.dom().paint_dom(&mut painted, area);
    let column: String = (0..4)
        .map(|y| painted.cell(9, y).unwrap().symbol().to_string())
        .collect();
    assert_eq!(column, "9999", "the last column is content");
    assert_eq!(app.dom().hit_test(9, 1), Some(rows[1]));
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 9, 0);
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), 9, 3);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), 9, 3);
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().node(s).scroll_top(), Some(0), "no thumb to drag");
    mouse(&mut app, MouseEventKind::ScrollDown, 5, 1);
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().node(s).scroll_top(), Some(1), "the wheel scrolls");
}

/// `.port` (10 × 4, `overflow-y: auto`, styled `css`) holding eight
/// one-row buttons on a bare document: the dom, the port and the buttons.
fn port(css: &str) -> (TuiDom, NodeId, Vec<NodeId>) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "port");
    let buttons = (0..8)
        .map(|_| {
            let b = el(&mut dom, p, "button", "b");
            let t = dom.create_text_node("go");
            dom.append_child(b, t).unwrap();
            b
        })
        .collect();
    lay_out(
        &mut dom,
        &format!(
            ".port {{ width: 10; height: 4; overflow-y: auto; {css} }} \
             .b {{ display: block; height: 1 }}"
        ),
        12,
        6,
    );
    (dom, p, buttons)
}

/// Shift+Tab backward past the optimal viewing region's top (the
/// scrollport less `scroll-padding-top: 1`, CSS Scroll Snap 1 §4.1) brings
/// the focused button to that region's top: row 4 under scrollTop 3.
#[test]
fn shift_tab_reveals_within_the_top_padding() {
    let (mut dom, p, b) = port("scroll-padding-top: 1");
    dom.node_mut(p).set_scroll_top(4).unwrap();
    dom.layout_dom(Rect::new(0, 0, 12, 6));
    rdom_tui::runtime::focus::focus_node(&mut dom, Some(b[5]));
    assert_eq!(dom.node(p).scroll_top(), Some(4), "row 5 is in the region");
    rdom_tui::runtime::focus::tabindex::focus_prev(&mut dom);
    assert_eq!(dom.focused(), Some(b[4]));
    assert_eq!(dom.node(p).scroll_top(), Some(3));
}

/// HTML's focusing steps scroll the element into view in every scroll
/// container around it: the inner port to the button, the outer port to
/// the inner one.
#[test]
fn focus_scrolls_every_scroller_around_the_element() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let outer = el(&mut dom, root, "div", "outer");
    let spacer = el(&mut dom, outer, "div", "spacer");
    let _ = spacer;
    let inner = el(&mut dom, outer, "div", "inner");
    let buttons: Vec<NodeId> = (0..6)
        .map(|_| {
            let b = el(&mut dom, inner, "button", "b");
            let t = dom.create_text_node("go");
            dom.append_child(b, t).unwrap();
            b
        })
        .collect();
    lay_out(
        &mut dom,
        ".outer { width: 10; height: 4; overflow-y: auto } .spacer { height: 6 } \
         .inner { height: 3; overflow-y: auto } .b { display: block; height: 1 }",
        12,
        6,
    );
    rdom_tui::runtime::focus::focus_node(&mut dom, Some(buttons[5]));
    // The button's row 5 in the 3-row inner port: scrollTop 3; the inner
    // port's rows 6–8 in the 4-row outer one: scrollTop 5.
    assert_eq!(dom.node(inner).scroll_top(), Some(3));
    assert_eq!(dom.node(outer).scroll_top(), Some(5));
}

/// Focus a pointer moved does not scroll: a press on the visible top row
/// of a 5-row button in a 3-row scroller focuses it where it is.
#[test]
fn pointer_focus_does_not_scroll() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "port");
    let top = el(&mut dom, p, "div", "top");
    let t = dom.create_text_node("x");
    dom.append_child(top, t).unwrap();
    let b = el(&mut dom, p, "button", "tall");
    let t = dom.create_text_node("go");
    dom.append_child(b, t).unwrap();
    let sheet = rdom_css::from_css_strict(
        ".port { width: 10; height: 3; overflow-y: auto } .tall { display: block; height: 5 }",
    )
    .unwrap();
    let terminal = Terminal::new(TestBackend::new(12, 6)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 2, 1);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), 2, 1);
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().focused(), Some(b));
    assert_eq!(app.dom().node(p).scroll_top(), Some(0));
}

/// CSS Overflow 3 §3.1: `overflow: clip` makes no formatting context, so
/// it does not contain its float: `.c` is 0 rows and the next paragraph
/// wraps beside the float — which `.c` clips away at its 0-row clip edge,
/// still excluding (`overflow: hidden` contains it).
#[test]
fn overflow_clip_does_not_contain_floats() {
    for (overflow, height, next) in [("clip", 0, "  next    "), ("hidden", 3, "next      ")] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let body = el(&mut dom, root, "body", "");
        let c = el(&mut dom, body, "div", "c");
        let f = el(&mut dom, c, "div", "f");
        let t = dom.create_text_node("A");
        dom.append_child(f, t).unwrap();
        let p = el(&mut dom, body, "p", "");
        let t = dom.create_text_node("next");
        dom.append_child(p, t).unwrap();
        let buf = paint(
            &mut dom,
            &format!(".c {{ overflow: {overflow} }} .f {{ float: left; width: 2; height: 3 }}"),
            10,
            4,
        );
        assert_eq!(rect(&dom, c).height, height, "{overflow}");
        let row = if height == 0 { 0 } else { 3 };
        assert_eq!(rows(&buf, 10, 4)[row], next, "{overflow}");
    }
}

/// A stuck `position: sticky` box (CSS Position 3 §3) moves with its
/// absolutely positioned child, whose containing block it is (§2.1): the
/// child stays on the stuck box's row.
#[test]
fn a_stuck_sticky_box_carries_its_absolute_child() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let port = el(&mut dom, root, "div", "port");
    let head = el(&mut dom, port, "div", "head");
    let t = dom.create_text_node("H");
    dom.append_child(head, t).unwrap();
    let badge = el(&mut dom, head, "span", "badge");
    let t = dom.create_text_node("B");
    dom.append_child(badge, t).unwrap();
    for _ in 0..8 {
        let r = el(&mut dom, port, "div", "row");
        let t = dom.create_text_node("x");
        dom.append_child(r, t).unwrap();
    }
    let css = ".port { width: 10; height: 4; overflow-y: auto } \
               .head { position: sticky; top: 0; height: 1 } \
               .badge { position: absolute; top: 0; left: 5 } .row { height: 1 }";
    lay_out(&mut dom, css, 12, 6);
    dom.node_mut(port).set_scroll_top(3).unwrap();
    dom.layout_dom(Rect::new(0, 0, 12, 6));
    let (h, b) = (rect(&dom, head), rect(&dom, badge));
    assert_eq!(h.y, 0, "stuck at the scrollport's top");
    assert_eq!((b.x, b.y), (5, h.y));
}

/// A flex item that truncates: `text-overflow` applies to the item, a
/// block container with `overflow: hidden` (CSS Overflow 4 §3) — the
/// flex container's own marking does not, the item's does.
#[test]
fn a_truncating_flex_item_draws_its_ellipsis() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let row = el(&mut dom, root, "div", "row");
    let t = el(&mut dom, row, "div", "truncate");
    let text = dom.create_text_node("abcdefghij");
    dom.append_child(t, text).unwrap();
    let buf = paint(
        &mut dom,
        ".row { display: flex } \
         .truncate { width: 6; overflow: hidden; white-space: nowrap; text-overflow: ellipsis }",
        10,
        1,
    );
    assert_eq!(rows(&buf, 10, 1)[0], "abcde…    ");
}
