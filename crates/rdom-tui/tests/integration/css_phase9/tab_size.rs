//! C9-TAB-SIZE — `tab-size` and tab stops (CSS Text 3 §4.2): a preserved
//! tab advances to the next multiple of the tab size from the block's
//! starting content edge.

use super::{el, lay_out, paint, paint_text, rows, text_block};
use rdom_tui::prelude::*;
use rdom_tui::render::inline::cell_of_position;

/// §4.2: "each preserved tab is rendered as a horizontal shift that lines
/// up the start edge of the next glyph with the next tab stop ... Tab
/// stops occur at points that are multiples of the tab size from the
/// starting content edge"; the initial `tab-size` is 8.
#[test]
fn a_tab_advances_to_the_next_tab_stop() {
    assert_eq!(
        paint_text("white-space: pre", "a\tb\nabc\td", 12, 2),
        ["a       b   ", "abc     d   "]
    );
    assert_eq!(
        paint_text("white-space: pre; tab-size: 4", "a\tb\tc\n\t\td", 12, 2),
        ["a   b   c   ", "        d   "]
    );
    assert_eq!(
        paint_text("white-space: pre; tab-size: 2ch", "ab\tc", 6, 1),
        ["ab  c "]
    );
}

/// §4.2: "If the tab size is zero, preserved tabs are not rendered"; a
/// collapsible tab (`normal`) is a space (§4.1.1).
#[test]
fn a_zero_tab_size_hides_tabs_and_collapsed_tabs_are_spaces() {
    assert_eq!(
        paint_text("white-space: pre; tab-size: 0", "a\tb", 4, 1),
        ["ab  "]
    );
    assert_eq!(paint_text("tab-size: 4", "a\tb", 4, 1), ["a b "]);
}

/// §4.2: stops are measured from the block's content edge — a tab after
/// a soft wrap starts from the new line's start; and under `pre-wrap` a
/// tab is preserved white space, a soft wrap opportunity after it
/// (§4.1.1).
#[test]
fn tab_stops_restart_on_each_line() {
    assert_eq!(
        paint_text(
            "white-space: pre-wrap; tab-size: 4; width: 6",
            "ab\tcd\tef",
            8,
            2
        ),
        ["ab  cd  ", "ef      "]
    );
}

/// The caret steps over a tab as one character of the source, drawn at
/// the cell after the tab's stop (HTML editing works in the DOM text).
#[test]
fn the_caret_moves_over_a_tab_as_one_character() {
    let (mut dom, _, t) = text_block("a\tb");
    lay_out(&mut dom, ".b { white-space: pre; tab-size: 4 }", 8, 1);
    assert_eq!(cell_of_position(&dom, Position::new(t, 1)), Some((1, 0)));
    assert_eq!(cell_of_position(&dom, Position::new(t, 2)), Some((4, 0)));
    assert_eq!(dom.position_at(2, 0), Some(Position::new(t, 1)));
    assert_eq!(dom.position_at(4, 0), Some(Position::new(t, 2)));
}

/// §4.2: stops are measured "from the starting content edge" of the block
/// — not from the line's start beside a float — and a line moved below a
/// float (CSS 2.1 §9.5) places its tabs from where it lands; a tab that
/// wraps to a new line is placed again there.
#[test]
fn tab_stops_count_from_the_content_edge() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    let f = el(&mut dom, c, "div", "f");
    let ft = dom.create_text_node("F");
    dom.append_child(f, ft).unwrap();
    let t = dom.create_text_node("\tab\n\tabcdefghij");
    dom.append_child(c, t).unwrap();
    let css = ".c { width: 14; white-space: pre; tab-size: 4 } \
               .f { float: left; width: 3; height: 1 }";
    let buf = paint(&mut dom, css, 14, 2);
    assert_eq!(rows(&buf, 14, 2), ["F   ab        ", "    abcdefghij"]);
    assert_eq!(
        paint_text(
            "white-space: pre-wrap; tab-size: 4; width: 6",
            "aaaaa b\tc",
            8,
            2
        ),
        ["aaaaa   ", "b   c   "]
    );
    // A line too wide for the band beside the float moves below it and
    // places its tab from the content edge there.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    let f = el(&mut dom, c, "div", "f");
    let ft = dom.create_text_node("F");
    dom.append_child(f, ft).unwrap();
    let t = dom.create_text_node("\tabcdefg");
    dom.append_child(c, t).unwrap();
    let css = ".c { width: 10; white-space: pre; tab-size: 4 } \
               .f { float: left; width: 3; height: 1 }";
    let buf = paint(&mut dom, css, 12, 2);
    assert_eq!(rows(&buf, 12, 2), ["F           ", "    abcdefg "]);
}
