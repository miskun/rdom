//! C8-SCROLLBAR — `scrollbar-gutter: stable both-edges` (CSS Overflow 3
//! §3.3), `scrollbar-width` (CSS Scrollbars 1 §3) and `scrollbar-color`
//! (§2), with their precedence over rdom's `::scrollbar` /
//! `::scrollbar-thumb` pseudo-elements (Chromium's: a standard property
//! that is not `auto` turns the pseudo-element styling off).

use super::{el, lay_out, paint, rows};
use rdom_tui::prelude::*;
use rdom_tui::render::Color;

/// A 10 × 4 `.s` holding six one-row children, styled `css`: the dom,
/// `.s` and its content box `(x, width)`.
fn scroller(css: &str) -> (TuiDom, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = el(&mut dom, root, "div", "s");
    for _ in 0..6 {
        let row = el(&mut dom, s, "div", "row");
        let t = dom.create_text_node("x");
        dom.append_child(row, t).unwrap();
    }
    lay_out(
        &mut dom,
        &format!(".s {{ width: 10; height: 4 }} .row {{ height: 1 }} {css}"),
        12,
        6,
    );
    (dom, s)
}

fn content_x_width(dom: &TuiDom, id: NodeId) -> (i32, u16) {
    let c = dom.node(id).ext().unwrap().content_layout;
    (c.x, c.width)
}

/// §3.3: `both-edges` — "if a gutter would be present on one of the
/// inline start edge or the inline end edge of the box, another gutter
/// must be present on the opposite edge as well": the content box loses
/// a column on each side.
#[test]
fn both_edges_reserves_a_gutter_on_each_inline_edge() {
    let (dom, s) = scroller(".s { overflow-y: auto; scrollbar-gutter: stable both-edges }");
    assert_eq!(content_x_width(&dom, s), (1, 8));
    let (dom, s) = scroller(".s { overflow-y: scroll; scrollbar-gutter: stable both-edges }");
    assert_eq!(content_x_width(&dom, s), (1, 8));
}

/// §3.3: `stable` — "the scrollbar gutter is present for `overflow:
/// hidden`, `scroll`, or `auto`, regardless of whether a scrollbar is
/// actually present"; a `visible` / `clip` box has none.
#[test]
fn a_stable_gutter_is_present_on_a_hidden_box() {
    let (dom, s) = scroller(".s { overflow-y: hidden; scrollbar-gutter: stable }");
    assert_eq!(content_x_width(&dom, s), (0, 9));
    let (dom, s) = scroller(".s { overflow: clip; scrollbar-gutter: stable }");
    assert_eq!(content_x_width(&dom, s), (0, 10));
}

/// CSS Scrollbars 1 §3: `none` — "no scrollbar should be displayed but
/// the element must remain scrollable": no gutter, no bar painted, the
/// offset still settable.
#[test]
fn scrollbar_width_none_hides_the_bar_but_keeps_scrolling() {
    let (mut dom, s) = scroller(".s { overflow-y: scroll; scrollbar-width: none }");
    assert_eq!(content_x_width(&dom, s), (0, 10));
    dom.node_mut(s).set_scroll_top(2).unwrap();
    assert_eq!(dom.node(s).scroll_top(), Some(2));
    let css = ".s { width: 10; height: 4; overflow-y: scroll; scrollbar-width: none } \
               .row { height: 1 }";
    let buf = paint(&mut dom, css, 12, 6);
    let column: String = (0..4)
        .map(|y| buf.cell(9, y).unwrap().symbol().to_string())
        .collect();
    assert_eq!(column, "    ");
}

/// The bar of a scroller styled `css` (an `overflow-y: scroll` 10 × 4
/// one holding six rows, at its top): its column's four cells.
fn bar(css: &str) -> Vec<(String, Color, Color)> {
    let (mut dom, _) = scroller("");
    let sheet =
        format!(".s {{ width: 10; height: 4; overflow-y: scroll }} .row {{ height: 1 }} {css}");
    let buf = paint(&mut dom, &sheet, 12, 6);
    (0..4)
        .map(|y| {
            let c = buf.cell(9, y).unwrap();
            (c.symbol().to_string(), c.fg, c.bg)
        })
        .collect()
}

/// rdom's terminal `thin` (decided, DIVERGENCES §1): the bar keeps its
/// one cell — no cell is narrower — but drops the track glyph and draws
/// the thumb with the light line `│` instead of the heavy `┃`.
#[test]
fn a_thin_bar_draws_a_light_thumb_and_no_track() {
    let cells = bar(".s { scrollbar-width: thin }");
    let glyphs: Vec<&str> = cells.iter().map(|c| c.0.as_str()).collect();
    assert_eq!(glyphs, ["│", "│", " ", " "], "a 2-cell thumb over 4");
}

/// §2: `scrollbar-color: <thumb> <track>` — the thumb in the first
/// color, the track in the second (its cells filled with it).
#[test]
fn scrollbar_color_paints_the_thumb_and_the_track() {
    let cells = bar(".s { scrollbar-color: #ff0000 #0000ff }");
    let red = Color::Rgb(255, 0, 0);
    let blue = Color::Rgb(0, 0, 255);
    assert_eq!(cells[0], ("┃".to_string(), red, blue));
    assert_eq!(cells[3], ("│".to_string(), blue, blue));
}

/// Precedence, Chromium's: `::scrollbar-thumb` styles the bar while the
/// standard properties are `auto`, and is ignored once one is not.
#[test]
fn a_standard_property_turns_the_scrollbar_pseudos_off() {
    let pseudo = ".s::scrollbar-thumb { content: \"#\"; color: #00ff00 }";
    assert_eq!(bar(pseudo)[0].0, "#");
    let both = format!("{pseudo} .s {{ scrollbar-color: #ff0000 #0000ff }}");
    assert_eq!(bar(&both)[0].0, "┃");
    let thin = format!("{pseudo} .s {{ scrollbar-width: thin }}");
    assert_eq!(bar(&thin)[0].0, "│");
}

/// The gutters count in a box's intrinsic width (CSS Overflow 3 §3.3:
/// the gutter is part of the box): an inline block `abc` wide with a
/// stable gutter on both edges is 5 cells, with `scrollbar-width: none`
/// 3.
#[test]
fn the_gutters_count_in_the_intrinsic_width() {
    let width = |css: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let c = el(&mut dom, root, "div", "c");
        let i = el(&mut dom, c, "span", "i");
        let t = dom.create_text_node("abc");
        dom.append_child(i, t).unwrap();
        lay_out(
            &mut dom,
            &format!(".i {{ display: inline-block; overflow-y: scroll; {css} }}"),
            20,
            2,
        );
        super::rect(&dom, i).width
    };
    assert_eq!(width("scrollbar-gutter: stable both-edges"), 5);
    assert_eq!(width("scrollbar-width: none"), 3);
    assert_eq!(width(""), 4);
}

/// `both-edges` paints the bar on its own side only: the opposite gutter
/// is blank.
#[test]
fn the_opposite_gutter_stays_blank() {
    let (mut dom, _) = scroller("");
    let css = ".s { width: 10; height: 4; overflow-y: scroll; \
               scrollbar-gutter: stable both-edges } .row { height: 1 }";
    let buf = paint(&mut dom, css, 12, 6);
    let row: String = rows(&buf, 12, 1)[0].chars().take(10).collect();
    assert_eq!(row, " x       ┃");
}
