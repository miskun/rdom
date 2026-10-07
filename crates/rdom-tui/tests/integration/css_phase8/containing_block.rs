//! C8-CB-COMPLETE — the containing block of a positioned box (CSS 2.1
//! §10.1, CSS Position 3 §2): the nearest ancestor whose `position` is
//! not `static` — `sticky` included — gives an absolutely positioned
//! box, element or `::before` / `::after`, its padding box (less the
//! scrollbar gutter), in its scrolled content space; `fixed` takes the
//! viewport; a grid container gives the grid area (CSS Grid 2 §9.1).

use super::{el, lay_out, rect};
use rdom_tui::prelude::*;
use rdom_tui::render::Rect;

fn xywh(r: rdom_tui::layout::LayoutRect) -> (i32, i32, u16, u16) {
    (r.x, r.y, r.width, r.height)
}

/// The placed `::after` of `host`.
fn after(dom: &TuiDom, host: NodeId) -> (i32, i32, u16, u16) {
    xywh(
        dom.node(host)
            .ext()
            .and_then(|e| e.positioned_pseudos().iter().find_map(|a| a.generated))
            .expect("a positioned ::after is placed")
            .border_box,
    )
}

/// Lay `dom` out again with `id` scrolled to `(x, y)`.
fn scrolled(dom: &mut TuiDom, id: NodeId, x: i32, y: i32, w: u16, h: u16) {
    let mut node = dom.node_mut(id);
    let ext = node.ext_mut().expect("styled");
    ext.scroll_x = x;
    ext.scroll_y = y;
    dom.layout_dom(Rect::new(0, 0, w, h));
}

const STICKY: &str = ".s { position: sticky; margin: 2 0 0 3; padding: 1; height: 4 } ";
const ONE: &str = "position: absolute; top: 0; left: 0; width: 1; height: 1";

/// CSS Position 3 §2.1: an absolutely positioned box's containing block
/// is established "by the nearest ancestor box that establishes an
/// absolute positioning containing block", which a box does when its
/// `position` is anything but `static` (§2) — `sticky` too.
#[test]
fn a_sticky_ancestor_contains_an_absolute_element() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = el(&mut dom, root, "div", "s");
    let abs = el(&mut dom, s, "div", "abs");
    lay_out(&mut dom, &format!("{STICKY} .abs {{ {ONE} }}"), 20, 8);
    assert_eq!(xywh(rect(&dom, abs)), (3, 2, 1, 1));
}

/// §2.1 for a pseudo-element: a `sticky` host, or a `sticky` ancestor of
/// a static host, contains its absolutely positioned `::after`.
#[test]
fn a_sticky_host_or_ancestor_contains_an_absolute_pseudo() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = el(&mut dom, root, "div", "s");
    lay_out(
        &mut dom,
        &format!("{STICKY} .s::after {{ content: \"\"; {ONE} }}"),
        20,
        8,
    );
    assert_eq!(after(&dom, s), (3, 2, 1, 1));

    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = el(&mut dom, root, "div", "s");
    let h = el(&mut dom, s, "div", "h");
    lay_out(
        &mut dom,
        &format!("{STICKY} .h {{ height: 1 }} .h::after {{ content: \"\"; {ONE} }}"),
        20,
        8,
    );
    assert_eq!(after(&dom, h), (3, 2, 1, 1));
}

/// CSS Overflow 3 §3: the scrollbar gutter lies "between the inner
/// border edge and the outer padding edge", so it is no part of the
/// padding box an absolutely positioned descendant is contained by
/// (CSS 2.1 §10.1). The vertical bar's column is on the left under
/// `rtl`; the horizontal bar's row at the bottom.
#[test]
fn the_scrollbar_gutter_is_outside_the_containing_block() {
    let fill = |port: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let p = el(&mut dom, root, "div", "p");
        let abs = el(&mut dom, p, "div", "abs");
        lay_out(
            &mut dom,
            &format!(
                ".p {{ position: relative; width: 10; height: 4; {port} }} \
                 .abs {{ position: absolute; inset: 0 }}"
            ),
            20,
            8,
        );
        xywh(rect(&dom, abs))
    };
    assert_eq!(fill("overflow-y: scroll"), (0, 0, 9, 4));
    assert_eq!(fill("overflow-y: scroll; direction: rtl"), (1, 0, 9, 4));
    assert_eq!(fill("overflow-x: scroll"), (0, 0, 10, 3));
    assert_eq!(
        fill("overflow-y: auto; scrollbar-gutter: stable"),
        (0, 0, 9, 4)
    );
    // No gutter, no change.
    assert_eq!(fill("overflow: hidden"), (0, 0, 10, 4));
}

/// A positioned scroll container `.p` (3 rows, 10 rows of content) with
/// an absolutely positioned child `top: 4` and a fixed one `top: 1`.
fn scroller(dom: &mut TuiDom) -> (NodeId, NodeId, NodeId) {
    let root = dom.root();
    let p = el(dom, root, "div", "p");
    el(dom, p, "div", "tall");
    let abs = el(dom, p, "div", "abs");
    let fixed = el(dom, p, "div", "fixed");
    lay_out(
        dom,
        ".p { position: relative; width: 10; height: 3; overflow: hidden } \
         .tall { height: 10 } \
         .abs { position: absolute; top: 4; left: 2; width: 1; height: 1 } \
         .fixed { position: fixed; top: 1; left: 5; width: 1; height: 1 }",
        20,
        8,
    );
    (p, abs, fixed)
}

/// CSS Overflow 3 §2.2 / CSS 2.1 §10.1: a scroll container's padding box
/// is the containing block in its scrolled content, so an absolutely
/// positioned child scrolls with the content, as in every browser; a
/// `fixed` one is contained by the viewport (Position 3 §2.1) and does
/// not move.
#[test]
fn an_absolute_box_scrolls_with_its_containing_scroll_container() {
    let mut dom = TuiDom::new();
    let (p, abs, fixed) = scroller(&mut dom);
    assert_eq!(xywh(rect(&dom, abs)), (2, 4, 1, 1));
    scrolled(&mut dom, p, 0, 2, 20, 8);
    assert_eq!(xywh(rect(&dom, abs)), (2, 2, 1, 1));
    assert_eq!(xywh(rect(&dom, fixed)), (5, 1, 1, 1));
}

/// §10.1: a scroll container that is not the containing block — a
/// positioned box outside it is — does not move the box when it scrolls.
#[test]
fn a_scroll_container_below_the_containing_block_does_not_move_it() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "wrap");
    let p = el(&mut dom, wrap, "div", "p");
    el(&mut dom, p, "div", "tall");
    let abs = el(&mut dom, p, "div", "abs");
    lay_out(
        &mut dom,
        ".wrap { position: relative } .p { height: 3; overflow: hidden } \
         .tall { height: 10 } .abs { position: absolute; top: 4; left: 2; width: 1; height: 1 }",
        20,
        8,
    );
    scrolled(&mut dom, p, 0, 2, 20, 8);
    assert_eq!(xywh(rect(&dom, abs)), (2, 4, 1, 1));
}

/// CSS Position 3 §2.1: a `fixed` box's containing block is the
/// viewport, whatever moves its ancestors — here a `sticky` ancestor
/// stuck to the top of a scrolled container (CSS Position 3 §3.4),
/// whose shift must not carry the fixed box with it.
#[test]
fn a_fixed_box_stays_on_the_viewport_inside_a_stuck_sticky() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let port = el(&mut dom, root, "div", "port");
    let s = el(&mut dom, port, "div", "s");
    let fixed = el(&mut dom, s, "div", "fixed");
    el(&mut dom, port, "div", "tall");
    lay_out(
        &mut dom,
        ".port { height: 4; overflow: hidden } .s { position: sticky; top: 0; height: 1 } \
         .tall { height: 10 } \
         .fixed { position: fixed; top: 6; left: 2; width: 1; height: 1 }",
        20,
        8,
    );
    scrolled(&mut dom, port, 0, 3, 20, 8);
    assert_eq!(rect(&dom, s).y, 0, "the sticky box is stuck");
    assert_eq!(xywh(rect(&dom, fixed)), (2, 6, 1, 1));
}

/// CSS Grid 2 §9.1: a positioned grid container gives an absolutely
/// positioned `::before` / `::after` — its own or a static child's —
/// "the grid area determined by its grid-placement properties", as it
/// gives an element.
#[test]
fn a_grid_gives_an_absolute_pseudo_its_grid_area() {
    const GRID: &str = ".g { display: grid; position: relative; grid-template-columns: 2 3 4; \
                        grid-template-rows: 1 2 } ";
    const AREA: &str = "content: \"\"; position: absolute; inset: 0; grid-column: 2 / 4; \
                        grid-row: 2";
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    lay_out(&mut dom, &format!("{GRID} .g::after {{ {AREA} }}"), 20, 6);
    assert_eq!(after(&dom, g), (2, 1, 7, 2));

    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let item = el(&mut dom, g, "div", "item");
    lay_out(
        &mut dom,
        &format!("{GRID} .item::after {{ {AREA} }}"),
        20,
        6,
    );
    assert_eq!(after(&dom, item), (2, 1, 7, 2));
}
