//! C9-WHITE-SPACE — `white-space` as the shorthand of
//! `white-space-collapse` and `text-wrap-mode` (CSS Text 4 §3), the
//! white space processing rules (CSS Text 3 §4.1) per value, and the
//! trailing-space hanging rules (§4.1.2's Phase II).

use super::{el, lay_out, paint, paint_text, rows, size, text_block};
use rdom_tui::prelude::*;
use rdom_tui::runtime::selection::clipboard::serialize_selection;

/// Text 3 §3 `pre-line`: "collapses consecutive white space characters
/// and allows wrapping, but it preserves segment breaks in the source as
/// forced line breaks"; §4.1.1 step 1 removes the spaces around a
/// segment break.
#[test]
fn pre_line_collapses_spaces_and_keeps_line_feeds() {
    assert_eq!(
        paint_text("white-space: pre-line", "a   b  \n   c  d", 8, 2),
        ["a b     ", "c d     "]
    );
    assert_eq!(
        paint_text("white-space: pre-line; width: 7", "aaa bbb ccc", 8, 2),
        ["aaa bbb ", "ccc     "]
    );
}

/// Text 3 §3 `pre-wrap` and §4.1.2: preserved spaces at the end of a
/// soft-wrapped line hang — "the UA must (unconditionally) hang this
/// sequence" — so they never push the line to wrap, and the next line
/// starts with the next word; a soft wrap opportunity exists only at the
/// end of the sequence (§4.1.1).
#[test]
fn pre_wrap_spaces_hang_at_a_soft_wrap() {
    assert_eq!(
        paint_text("white-space: pre-wrap; width: 3", "ab  cd", 6, 3),
        ["ab    ", "cd    ", "      "]
    );
}

/// Text 3 §3 `break-spaces`: "any sequence of preserved white space ...
/// always takes up space, including at the end of the line", with "a
/// soft wrap opportunity ... after every preserved white space
/// character" — and none before the first of a sequence (§5.2's note on
/// `line-break: anywhere`), so the first space stays on the line it
/// overflows and the second wraps.
#[test]
fn break_spaces_take_up_space_and_wrap() {
    assert_eq!(
        paint_text(
            "white-space: break-spaces; width: 3; overflow: hidden",
            "ab  cd",
            6,
            3
        ),
        ["ab    ", " cd   ", "      "]
    );
}

/// Text 3 §4.1.2: under `pre-wrap` hanging spaces "are not considered
/// when placing the rest of the line during text alignment" — an `rtl`
/// line (inline-start = right, CSS Writing Modes 4 §2.1) is placed by its
/// glyphs, its hanging spaces past the edge.
#[test]
fn hanging_spaces_do_not_count_when_a_line_is_placed() {
    assert_eq!(
        paint_text(
            "white-space: pre-wrap; width: 4; direction: rtl",
            "ab  cd",
            6,
            2
        ),
        ["  ab  ", "  cd  "]
    );
}

/// Text 4 §4.1 `preserve-spaces`: spaces and tabs are preserved and
/// "segment breaks such as line feeds are converted to spaces".
#[test]
fn preserve_spaces_turns_line_feeds_into_spaces() {
    assert_eq!(
        paint_text("white-space-collapse: preserve-spaces", "a  b\nc", 8, 2),
        ["a  b c  ", "        "]
    );
}

/// Text 4 §3: the longhands compose — `white-space-collapse: preserve`
/// with `text-wrap-mode: nowrap` is `pre`; `text-wrap-mode: nowrap` alone
/// keeps collapsing (`nowrap`).
#[test]
fn the_longhands_combine_as_the_shorthand_does() {
    assert_eq!(
        paint_text(
            "white-space-collapse: preserve; text-wrap-mode: nowrap; width: 3",
            "a  b c",
            8,
            2
        ),
        ["a  b c  ", "        "]
    );
    assert_eq!(
        paint_text("text-wrap-mode: nowrap; width: 3", "a   b c", 8, 2),
        ["a b c   ", "        "]
    );
}

/// Text 3 §3: `white-space` "applies to: text" — each inline element's
/// text is processed by its own value, inside a block of another: a
/// `pre` span keeps its spaces, a `nowrap` span does not wrap inside
/// while the block wraps around it (Text 3 §5.1: an opportunity at a
/// space is controlled by the space's own properties).
#[test]
fn white_space_applies_per_inline_element() {
    let lines = |css: &str, w: u16| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let b = el(&mut dom, root, "div", "b");
        let a = dom.create_text_node("aaa ");
        dom.append_child(b, a).unwrap();
        let s = el(&mut dom, b, "span", "s");
        let t = dom.create_text_node("b  b c");
        dom.append_child(s, t).unwrap();
        let buf = paint(&mut dom, css, w, 2);
        rows(&buf, w, 2)
    };
    assert_eq!(
        lines(".s { white-space: pre }", 12),
        ["aaa b  b c  ", "            "]
    );
    assert_eq!(
        lines(".b { width: 6 } .s { white-space: nowrap }", 8),
        ["aaa     ", "b b c   "]
    );
}

/// Text 3 §4.1.3 (the segment break transformation rules, UA-defined): a
/// collapsible segment break between two Chinese characters is removed,
/// so text broken across source lines joins with no space; between Latin
/// letters, or Korean, it becomes a space.
#[test]
fn a_segment_break_between_ideographs_is_removed() {
    assert_eq!(paint_text("", "中文\n字 ab\ncd", 12, 1), ["中文字 ab cd"]);
    assert_eq!(paint_text("", "한\n국", 6, 1), ["한 국 "]);
}

/// Text 3 §4.1.2: spaces that conditionally hang (before a forced break,
/// or at the end of the block) count for max-content — they hang only
/// where they overflow — while the unconditionally hanging ones at a soft
/// wrap count for neither intrinsic size (CSS Sizing 3 §5.1).
#[test]
fn hanging_spaces_and_intrinsic_sizes() {
    let width = |decl: &str, text: &str| {
        let (mut dom, b, _) = text_block(text);
        lay_out(&mut dom, &format!(".b {{ {decl} }}"), 20, 4);
        size(&dom, b).0
    };
    assert_eq!(
        width("white-space: pre-wrap; width: max-content", "ab  "),
        4
    );
    assert_eq!(
        width("white-space: pre-wrap; width: min-content", "ab    cd"),
        2
    );
    assert_eq!(
        width("white-space: break-spaces; width: min-content", "ab  cd"),
        3
    );
}

/// HTML §3.2.7's rendered text collection follows the same rules: a copy
/// of `pre-line` text collapses its spaces and keeps its line feeds.
#[test]
fn copying_pre_line_text_keeps_its_line_breaks() {
    let (mut dom, _, t) = text_block("a   b  \n  c");
    lay_out(&mut dom, ".b { white-space: pre-line }", 10, 2);
    let range = rdom_tui::Range::ordered_unchecked(
        rdom_tui::Position::new(t, 0),
        rdom_tui::Position::new(t, 11),
    );
    assert_eq!(serialize_selection(&dom, &range), "a b\nc");
}
