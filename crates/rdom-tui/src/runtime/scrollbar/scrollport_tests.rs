//! C8G-SCROLLPORT — one scrollport and one scrollable overflow area for
//! every reader (TECH_DEBT `SCROLLPORT-1`).
//!
//! CSS Overflow 3 §2.2: the scrollable overflow rectangle is the union of
//! the box's padding box and its content, the content extended by the
//! box's end padding; CSS Overflow 3 §5.2: the scrollbar gutter lies
//! "between the inner border edge and the outer padding edge", so the
//! scrollport is the padding box less the gutters. CSSOM View §4: the
//! scroll range on an axis is the scrolling area's size less the
//! scrollport's. Each test drives a real input (wheel, key, script,
//! `scrollIntoView`, snapping) to the end and checks that the end is on
//! screen.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use rdom_core::NodeId;

use crate::accessors::{TuiAccessors, TuiAccessorsMut};
use crate::render::{Buffer, LayoutExt, PaintExt, Rect};
use crate::runtime::router::Router;
use crate::runtime::scrollbar::handle_scroll_key;
use crate::style::CascadeExt;
use crate::{TuiDom, TuiNodeExt};

const AREA: Rect = Rect::new(0, 0, 14, 8);

/// A `.s` scroller (styled `s`) of `n` one-row `.r` rows (styled `r`),
/// row `i` reading `r<i>`: the dom, the scroller and its rows.
fn scroller(s: &str, r: &str, n: usize) -> (TuiDom, NodeId, Vec<NodeId>) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let sc = dom.create_element("div");
    dom.set_attribute(sc, "class", "s").unwrap();
    dom.append_child(root, sc).unwrap();
    let rows = (0..n)
        .map(|i| {
            let id = dom.create_element("div");
            dom.set_attribute(id, "class", "r").unwrap();
            let text = dom.create_text_node(&format!("r{i}"));
            dom.append_child(id, text).unwrap();
            dom.append_child(sc, id).unwrap();
            id
        })
        .collect();
    let sheet = rdom_css::from_css_strict(&format!(
        ".s {{ box-sizing: border-box; {s} }} .r {{ height: 1; {r} }}"
    ))
    .unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(AREA);
    (dom, sc, rows)
}

fn wheel_many(dom: &mut TuiDom, kind: MouseEventKind, ticks: usize) {
    let mut router = Router::new();
    for _ in 0..ticks {
        router.route(
            dom,
            Event::Mouse(MouseEvent {
                kind,
                column: 3,
                row: 1,
                modifiers: KeyModifiers::empty(),
            }),
        );
        dom.layout_dom(AREA);
    }
}

fn paint(dom: &TuiDom) -> Vec<String> {
    let mut buf = Buffer::empty(AREA);
    dom.paint_dom(&mut buf, AREA);
    (0..AREA.height)
        .map(|y| {
            (0..AREA.width)
                .map(|x| buf.cell(x, y).unwrap().symbol().to_string())
                .collect()
        })
        .collect()
}

fn top(dom: &TuiDom, id: NodeId) -> i32 {
    dom.node(id).scroll_top().unwrap()
}

fn rect(dom: &TuiDom, id: NodeId) -> crate::layout::LayoutRect {
    dom.node(id).layout_rect().unwrap()
}

/// §2.2: block padding is part of the scrollable overflow — `padding: 1
/// 0` around 20 rows makes a 22-row area (`scrollHeight`) in a 5-row
/// scrollport, so the wheel reaches 17 and shows the last row above the
/// end padding.
#[test]
fn the_wheel_reaches_the_end_padding() {
    let (mut dom, s, _) = scroller(
        "width: 10; height: 5; padding: 1 0; overflow-y: auto",
        "",
        20,
    );
    assert_eq!(dom.node(s).scroll_height(), Some(22));
    assert_eq!(dom.node(s).scroll_range().unwrap().y(), 0..=17);
    wheel_many(&mut dom, MouseEventKind::ScrollDown, 30);
    assert_eq!(top(&dom, s), 17);
    let rows = paint(&dom);
    assert!(rows[3].starts_with("r19"), "last row on row 3: {rows:?}");
    assert!(
        rows[4].chars().take(9).all(|c| c == ' '),
        "the end padding below it: {rows:?}"
    );
}

/// §5.2: a horizontal bar's row is outside the scrollport — with both
/// bars the scrollport is 9 × 4, so `End` puts the last row on row 3,
/// above the bar, and the wheel reaches the last column.
#[test]
fn the_last_row_is_above_the_horizontal_bar() {
    let (mut dom, s, rows) = scroller("width: 10; height: 5; overflow: auto", "width: 30", 20);
    assert_eq!(dom.node(s).scroll_range().unwrap().y(), 0..=16);
    assert_eq!(dom.node(s).scroll_range().unwrap().x(), 0..=21);
    dom.set_attribute(s, "tabindex", "0").unwrap();
    dom.set_focused(Some(s));
    handle_scroll_key(&mut dom, KeyEvent::new(KeyCode::End, KeyModifiers::empty()));
    dom.layout_dom(AREA);
    assert_eq!(top(&dom, s), 16);
    assert_eq!(rect(&dom, rows[19]).y, 3);
    let painted = paint(&dom);
    assert!(painted[3].starts_with("r19"), "{painted:?}");
    wheel_many(&mut dom, MouseEventKind::ScrollRight, 30);
    assert_eq!(dom.node(s).scroll_left(), Some(21));
    assert_eq!(rect(&dom, rows[19]).x + 30, 9, "the last column at the bar");
}

/// The same with the padding on both axes and script scrolling:
/// `scrollTop = MAX` and `scrollLeft = MAX` land on the ends of a
/// 1-padded box with both bars.
#[test]
fn script_reaches_both_ends_with_padding_and_both_bars() {
    let (mut dom, s, rows) = scroller(
        "width: 12; height: 7; padding: 1; overflow: scroll",
        "width: 30",
        20,
    );
    // Scrollport 11 × 6 (the gutters at the padding edge); area 1 + 30 +
    // 1 wide, 1 + 20 + 1 tall.
    assert_eq!(dom.node(s).scroll_width(), Some(32));
    assert_eq!(dom.node(s).scroll_height(), Some(22));
    dom.node_mut(s).set_scroll_top(i32::MAX).unwrap();
    dom.node_mut(s).set_scroll_left(i32::MAX).unwrap();
    assert_eq!((dom.node(s).scroll_left(), top(&dom, s)), (Some(21), 16));
    dom.layout_dom(AREA);
    let last = rect(&dom, rows[19]);
    assert_eq!(
        (last.y, last.x + 30),
        (4, 10),
        "last row and column above / left of the end padding"
    );
}

/// §5.2 with CSSOM View §4: an `rtl` box's horizontal origin is its
/// right edge and its vertical bar on its left — the range runs to
/// `-(area − scrollport)`, and at its minimum the content's left edge is
/// at the scrollport's left edge (column 1, right of the bar), less the
/// box's left padding.
#[test]
fn rtl_reaches_the_left_end_beside_its_bar() {
    let (mut dom, s, rows) = scroller(
        "direction: rtl; width: 10; height: 5; padding: 0 1; overflow: auto",
        "width: 30",
        20,
    );
    // Scrollport 9 × 4 (the bar's column on the left, the horizontal
    // bar's row); area 1 + 30 + 1 wide.
    assert_eq!(dom.node(s).scroll_range().unwrap().x(), -23..=0);
    wheel_many(&mut dom, MouseEventKind::ScrollLeft, 40);
    assert_eq!(dom.node(s).scroll_left(), Some(-23));
    assert_eq!(rect(&dom, rows[0]).x, 2, "bar column 0, padding column 1");
}

/// A `column-reverse` box's vertical origin is its bottom edge: the
/// range is `-(area − scrollport)..=0`, with the top padding at the far
/// end.
#[test]
fn column_reverse_reaches_the_top_end() {
    let (mut dom, s, rows) = scroller(
        "display: flex; flex-direction: column-reverse; width: 10; height: 5; \
         padding: 1 0; overflow-y: auto",
        "flex-shrink: 0",
        20,
    );
    assert_eq!(dom.node(s).scroll_range().unwrap().y(), -17..=0);
    wheel_many(&mut dom, MouseEventKind::ScrollUp, 30);
    assert_eq!(top(&dom, s), -17);
    // `column-reverse` stacks row 0 at the bottom: row 19 is on top,
    // one row of padding above it.
    assert_eq!(rect(&dom, rows[19]).y, 1);
}

/// Scroll Snap 1 §4.1: the snapport is the scrollport — an `end`-aligned
/// last item rests above the horizontal bar, not under it.
#[test]
fn an_end_snap_rests_above_the_horizontal_bar() {
    let (mut dom, s, rows) = scroller(
        "width: 10; height: 5; overflow: auto; scroll-snap-type: y mandatory",
        "width: 30; height: 3; scroll-snap-align: end",
        5,
    );
    dom.set_attribute(s, "tabindex", "0").unwrap();
    dom.set_focused(Some(s));
    handle_scroll_key(&mut dom, KeyEvent::new(KeyCode::End, KeyModifiers::empty()));
    dom.layout_dom(AREA);
    assert_eq!(top(&dom, s), 11);
    let last = rect(&dom, rows[4]);
    assert_eq!(
        last.y + i32::from(last.height),
        4,
        "bottom at the scrollport's"
    );
}

/// CSSOM View §5.1 `scrollIntoView({block: "end"})` aligns with the
/// scrollport's end edge, above a horizontal bar.
#[test]
fn scroll_into_view_end_stays_above_the_horizontal_bar() {
    let (mut dom, s, rows) = scroller("width: 10; height: 5; overflow: auto", "width: 30", 20);
    dom.node_mut(rows[10])
        .scroll_into_view_with(
            crate::accessors::ScrollIntoViewOptions::new()
                .block(crate::accessors::ScrollLogicalPosition::End),
        )
        .unwrap();
    dom.layout_dom(AREA);
    assert_eq!(top(&dom, s), 7);
    assert_eq!(rect(&dom, rows[10]).y, 3);
}

/// §5.2: the vertical bar sits in the gutter at the padding edge — the
/// box's last column inside the border, right of the right padding; its
/// track spans the scrollport's rows, and at the end of the range the
/// thumb's last cell is the track's last.
#[test]
fn the_bar_is_at_the_padding_edge_and_its_thumb_reaches_the_end() {
    let (mut dom, s, _) = scroller(
        "width: 10; height: 5; padding: 1; overflow-y: scroll",
        "",
        20,
    );
    let painted = paint(&dom);
    let col = |rows: &[String], x: usize| -> String {
        rows.iter()
            .take(5)
            .map(|r| r.chars().nth(x).unwrap())
            .collect()
    };
    let bar = col(&painted, 9);
    assert!(
        bar.chars().all(|c| c == '│' || c == '┃'),
        "track over all 5 rows at column 9: {bar:?}"
    );
    dom.node_mut(s).set_scroll_top(i32::MAX).unwrap();
    dom.layout_dom(AREA);
    let bar = col(&paint(&dom), 9);
    assert_eq!(bar.chars().last(), Some('┃'), "thumb at the end: {bar:?}");
}

/// §5.2: content does not paint into a gutter — a `stable` gutter with
/// nothing to scroll keeps its column blank, though a `nowrap` line
/// overflows into it under `overflow-x: hidden`.
#[test]
fn content_is_clipped_at_the_gutter() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "class", "s").unwrap();
    dom.append_child(root, s).unwrap();
    let text = dom.create_text_node("abcdefghi");
    dom.append_child(s, text).unwrap();
    let sheet = rdom_css::from_css_strict(
        ".s { width: 6; height: 2; overflow-y: auto; overflow-x: hidden; \
              scrollbar-gutter: stable; white-space: nowrap }",
    )
    .unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(AREA);
    let rows = paint(&dom);
    assert!(rows[0].starts_with("abcde "), "{rows:?}");
}
