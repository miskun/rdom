//! C9-TEXT-ALIGN — `text-align` (the shorthand of `text-align-all` and
//! `text-align-last`), `text-justify` (CSS Text 3 §6.1–§6.4, §7): line
//! content aligned within its line box — the box beside floats, past the
//! indent — whole cells, justification deterministic.

use super::{el, paint, paint_text, rows};
use rdom_tui::prelude::*;

fn lines(decl: &str, text: &str, w: u16, h: u16) -> Vec<String> {
    paint_text(&format!("width: {w}; {decl}"), text, w, h)
}

/// §6.1 `left` / `right` / `center`: content at the line-left or
/// line-right edge, or centered (the leading free space rounded down —
/// the whole-cell rule of DIVERGENCES §1).
#[test]
fn left_right_and_center() {
    assert_eq!(lines("text-align: right", "ab cd", 9, 1), ["    ab cd"]);
    assert_eq!(lines("text-align: center", "ab cd", 9, 1), ["  ab cd  "]);
    assert_eq!(lines("text-align: center", "abc", 8, 1), ["  abc   "]);
    assert_eq!(
        lines("text-align: left; direction: rtl", "ab", 6, 1),
        ["ab    "]
    );
}

/// §6.1 `start` / `end`: the start and end edges of the line box, which
/// `direction` decides (CSS Writing Modes 4 §2.1) — closing C5-WRITING's
/// note that `text-align: start / end` waited for this.
#[test]
fn start_and_end_follow_direction() {
    assert_eq!(lines("text-align: end", "ab", 6, 1), ["    ab"]);
    assert_eq!(
        lines("text-align: end; direction: rtl", "ab", 6, 1),
        ["ab    "]
    );
    assert_eq!(
        lines("text-align: start; direction: rtl", "ab", 6, 1),
        ["    ab"]
    );
}

/// §6.1 `justify` with `text-justify: auto` (inter-word for Latin text,
/// §6.4): word separators take the free space in whole cells, the
/// remainder to the first gaps; the last line, and a line before a
/// forced break, are start-aligned (`text-align-last: auto`, §6.3).
#[test]
fn justify_fills_the_line_but_not_the_last() {
    assert_eq!(
        lines("text-align: justify", "aa bb cc dd", 10, 2),
        ["aa  bb  cc", "dd        "]
    );
    assert_eq!(
        lines("text-align: justify", "a b c d eeeeee", 9, 2),
        ["a  b  c d", "eeeeee   "]
    );
    assert_eq!(
        lines(
            "text-align: justify; white-space: pre-line",
            "aa bb\ncc",
            7,
            2
        ),
        ["aa bb  ", "cc     "]
    );
}

/// §6.1 `justify-all` justifies the last line too; §6.3 `text-align-last`
/// aligns the last line its own way.
#[test]
fn the_last_line_follows_text_align_last() {
    assert_eq!(lines("text-align: justify-all", "aa bb", 7, 1), ["aa   bb"]);
    assert_eq!(
        lines(
            "text-align: justify; text-align-last: center",
            "aa bb cc dd",
            10,
            2
        ),
        ["aa  bb  cc", "    dd    "]
    );
    assert_eq!(
        lines("text-align: center; text-align-last: right", "aa bb", 7, 1),
        ["  aa bb"]
    );
}

/// §6.4 `text-justify`: `inter-character` spreads the space between every
/// pair of adjacent typographic character units; `none` disables
/// justification; `auto` also expands between CJK characters.
#[test]
fn text_justify_picks_the_opportunities() {
    assert_eq!(
        lines(
            "text-align: justify; text-justify: inter-character",
            "abc de fghijk",
            10,
            2
        ),
        ["a b c   de", "fghijk    "]
    );
    assert_eq!(
        lines(
            "text-align: justify; text-justify: none",
            "abc de fghijk",
            10,
            2
        ),
        ["abc de    ", "fghijk    "]
    );
    assert_eq!(
        lines("text-align: justify", "日本語 abcdefghij", 10, 2),
        ["日  本  語", "abcdefghij"]
    );
    assert_eq!(
        lines(
            "text-align: justify; text-justify: distribute",
            "abc de fghijk",
            10,
            2
        ),
        ["a b c   de", "fghijk    "]
    );
}

/// §6.1: "If (after justification, if any) the inline contents of a line
/// box are too long to fit within it, then the contents are start-aligned".
#[test]
fn an_overflowing_line_is_start_aligned() {
    assert_eq!(
        paint_text(
            "width: 4; text-align: right; white-space: nowrap",
            "abcdef",
            8,
            1
        ),
        ["abcdef  "]
    );
}

/// §6.1: alignment is within the line box — beside a float, the band the
/// float leaves (CSS 2.1 §9.5); past the indent (§8.1); an atomic inline
/// moves with the line.
#[test]
fn alignment_is_within_the_line_box() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    let f = el(&mut dom, c, "div", "f");
    let ft = dom.create_text_node("FFF");
    dom.append_child(f, ft).unwrap();
    let t = dom.create_text_node("ab");
    dom.append_child(c, t).unwrap();
    let buf = paint(
        &mut dom,
        ".c { width: 10; text-align: center } .f { float: left; width: 3; height: 1 }",
        10,
        1,
    );
    assert_eq!(rows(&buf, 10, 1), ["FFF  ab   "]);
    assert_eq!(
        lines("text-align: right; text-indent: 2", "ab", 6, 1),
        ["    ab"]
    );
    assert_eq!(
        lines("text-align: center; text-indent: 2", "ab", 8, 1),
        ["    ab  "]
    );
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let a = el(&mut dom, b, "span", "a");
    let at = dom.create_text_node("XY");
    dom.append_child(a, at).unwrap();
    let buf = paint(
        &mut dom,
        ".b { width: 6; text-align: center } .a { display: inline-block }",
        6,
        1,
    );
    assert_eq!(rows(&buf, 6, 1), ["  XY  "]);
}

/// §6.1 `match-parent`: "an inherited value of start or end is
/// interpreted against the parent's direction value and results in a
/// computed value of either left or right".
#[test]
fn match_parent_resolves_against_the_parent_direction() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    let c = el(&mut dom, p, "div", "c");
    let t = dom.create_text_node("ab");
    dom.append_child(c, t).unwrap();
    let buf = paint(
        &mut dom,
        ".p { direction: rtl; text-align: start; width: 6 } \
         .c { direction: ltr; text-align: match-parent }",
        6,
        1,
    );
    assert_eq!(rows(&buf, 6, 1), ["    ab"]);
    let computed = dom.node(c).computed().unwrap();
    assert_eq!(computed.text.text_align_all, rdom_tui::TextAlign::Right);
}

/// Justification space belongs to the word separator it widens: the caret
/// and hit-testing map the cells back to the source (the separator's byte),
/// and the text after it to its own bytes.
#[test]
fn justified_cells_map_back_to_the_source() {
    use rdom_tui::render::inline::cell_of_position;
    let (mut dom, _, t) = super::text_block("aa bb cc dd");
    super::lay_out(&mut dom, ".b { width: 10; text-align: justify }", 10, 2);
    assert_eq!(cell_of_position(&dom, Position::new(t, 3)), Some((4, 0)));
    assert_eq!(cell_of_position(&dom, Position::new(t, 6)), Some((8, 0)));
    assert_eq!(dom.position_at(3, 0), Some(Position::new(t, 2)));
    assert_eq!(dom.position_at(4, 0), Some(Position::new(t, 3)));
}
