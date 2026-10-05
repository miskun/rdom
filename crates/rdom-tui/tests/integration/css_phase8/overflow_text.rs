//! C8-OVERFLOW-TEXT — a descendant's line boxes in its scroll container's
//! scrollable overflow (CSS Overflow 3 §2.2): "the scrollable overflow
//! area is the union of ... the border boxes of all boxes for which it
//! is the containing block ... [and] the line boxes" of its content, so
//! text that overflows a non-clipping box can be scrolled to.

use super::el;
use super::lay_out;
use rdom_tui::prelude::*;

/// `.port` (6 × 2, `overflow: hidden`) around a `.p` styled `p`, holding
/// `text`: the port's `(scrollWidth, scrollHeight)` — never less than its
/// 6 × 2 scrollport, which the area always covers (§2.2).
fn extent(p: &str, text: &str) -> (Option<i32>, Option<i32>) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let port = el(&mut dom, root, "div", "port");
    let para = el(&mut dom, port, "div", "p");
    let t = dom.create_text_node(text);
    dom.append_child(para, t).unwrap();
    lay_out(
        &mut dom,
        &format!(".port {{ width: 6; height: 2; overflow: hidden }} .p {{ {p} }}"),
        10,
        4,
    );
    let node = dom.node(port);
    (node.scroll_width(), node.scroll_height())
}

/// §2.2: a line wider than its 4-cell box — no soft wrap opportunity
/// under `nowrap` — reaches 10 cells into the scroll container.
#[test]
fn an_overflowing_line_widens_the_scrollable_overflow() {
    assert_eq!(
        extent("width: 4; white-space: nowrap", "abcdefghij"),
        (Some(10), Some(2))
    );
}

/// §2.2: lines below a 1-row box (`white-space: pre`, three lines)
/// reach 3 rows down.
#[test]
fn overflowing_lines_deepen_the_scrollable_overflow() {
    assert_eq!(
        extent("width: 4; height: 1; white-space: pre", "ab\ncd\nef"),
        (Some(6), Some(3))
    );
}

/// The overflow is scrollable: `scrollLeft` reaches its last cells.
#[test]
fn the_overflowing_line_can_be_scrolled_to() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let port = el(&mut dom, root, "div", "port");
    let para = el(&mut dom, port, "div", "p");
    let t = dom.create_text_node("abcdefghij");
    dom.append_child(para, t).unwrap();
    lay_out(
        &mut dom,
        ".port { width: 6; height: 2; overflow: hidden } .p { width: 4; white-space: nowrap }",
        10,
        4,
    );
    dom.node_mut(port).set_scroll_left(4).unwrap();
    assert_eq!(dom.node(port).scroll_left(), Some(4));
}

/// A clipping descendant owns its overflow: a scroll container's lines
/// are its own scrollable overflow, not its ancestor's (§2.2).
#[test]
fn a_scroll_containers_lines_stay_its_own() {
    assert_eq!(
        extent(
            "width: 4; white-space: nowrap; overflow: hidden",
            "abcdefghij"
        ),
        (Some(6), Some(2))
    );
}

/// §2.2 for mixed content: the text beside a block child is laid out in
/// an anonymous block box (CSS 2.1 §9.2.1.1), whose overflowing line
/// counts too.
#[test]
fn an_anonymous_block_boxs_line_counts() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let port = el(&mut dom, root, "div", "port");
    let para = el(&mut dom, port, "div", "p");
    let t = dom.create_text_node("abcdefghij");
    dom.append_child(para, t).unwrap();
    el(&mut dom, para, "div", "b");
    lay_out(
        &mut dom,
        ".port { width: 6; height: 2; overflow: hidden } \
         .p { width: 4; white-space: nowrap } .b { height: 1 }",
        10,
        4,
    );
    assert_eq!(dom.node(port).scroll_width(), Some(10));
}

/// The scroll container's own anonymous block box: its overflowing line
/// is the container's scrollable overflow too.
#[test]
fn the_scroll_containers_own_anonymous_line_counts() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let port = el(&mut dom, root, "div", "port");
    let t = dom.create_text_node("abcdefghij");
    dom.append_child(port, t).unwrap();
    el(&mut dom, port, "div", "b");
    lay_out(
        &mut dom,
        ".port { width: 6; height: 2; overflow: hidden; white-space: nowrap } .b { height: 1 }",
        10,
        4,
    );
    assert_eq!(dom.node(port).scroll_width(), Some(10));
}

/// The `.port` of [`extent`] with `.p` styled `p` holding `text`, painted
/// (8 × 3) after scrolling the port to `scroll_left`.
fn painted(p: &str, text: &str, scroll_left: i32) -> Vec<String> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let port = el(&mut dom, root, "div", "port");
    let para = el(&mut dom, port, "div", "p");
    let t = dom.create_text_node(text);
    dom.append_child(para, t).unwrap();
    let css = format!(".port {{ width: 6; height: 2; overflow: hidden }} .p {{ {p} }}");
    super::lay_out(&mut dom, &css, 8, 3);
    dom.node_mut(port).set_scroll_left(scroll_left).unwrap();
    let buf = super::paint(&mut dom, &css, 8, 3);
    super::rows(&buf, 8, 3)
}

/// CSS Overflow 3 §3.1: `visible` content "is not clipped" — a line past
/// its box paints on, up to the scroll container's clip, and scrolling
/// the container brings the rest of it into view.
#[test]
fn an_overflowing_line_paints_past_its_box_and_scrolls_into_view() {
    let p = "width: 4; white-space: nowrap";
    assert_eq!(
        painted(p, "abcdefghij", 0),
        ["abcdef  ", "        ", "        "]
    );
    assert_eq!(
        painted(p, "abcdefghij", 4),
        ["efghij  ", "        ", "        "]
    );
}

/// The same for the lines below a box's fixed height.
#[test]
fn lines_below_a_fixed_height_paint() {
    assert_eq!(
        painted("width: 4; height: 1; white-space: pre", "ab\ncd\nef", 0),
        ["ab      ", "cd      ", "        "]
    );
}
