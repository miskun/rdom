//! C9-BREAKING — line breaking (CSS Text 3 §5): `word-break` (§5.2),
//! `line-break` (§5.3), `overflow-wrap` / `word-wrap` (§5.5), `hyphens`
//! and soft hyphens (§6.1), and the soft wrap opportunities of the UAX #14
//! subset rdom implements (§5.5 "Line Breaking Details").

use super::{el, lay_out, paint, paint_text, rows, size, text_block};
use rdom_tui::prelude::*;
use rdom_tui::render::inline::cell_of_position;

/// The rows of a `.b` block `width: w` holding `text`, `decl` added,
/// painted in a `vw` × `h` viewport.
fn lines(decl: &str, text: &str, w: u16, vw: u16, h: u16) -> Vec<String> {
    paint_text(&format!("width: {w}; {decl}"), text, vw, h)
}

/// The `width` a `.b` block styled `decl` lays out at.
fn width_of(decl: &str, text: &str) -> u16 {
    let (mut dom, b, _) = text_block(text);
    lay_out(&mut dom, &format!(".b {{ {decl} }}"), 30, 6);
    size(&dom, b).0
}

/// §5.2 `break-all`: "any typographic letter units ... are instead
/// treated as ID", so a word breaks between any two letters; `normal`
/// breaks only at the space, the long word overflowing.
#[test]
fn break_all_breaks_between_letters() {
    assert_eq!(
        lines("word-break: break-all", "abcdef ghi", 4, 6, 3),
        ["abcd  ", "ef g  ", "hi    "]
    );
    assert_eq!(
        lines("", "abcdef ghi", 4, 6, 3),
        ["abcdef", "ghi   ", "      "]
    );
}

/// §5.2 `keep-all`: "implicit soft wrap opportunities between typographic
/// letter units (or ... ID) are suppressed" — CJK text breaks only at the
/// space; `normal` breaks between any two ideographs.
#[test]
fn keep_all_keeps_cjk_words_whole() {
    assert_eq!(
        lines("", "日本語の文章", 8, 12, 2),
        ["日本語の    ", "文章        "]
    );
    assert_eq!(
        lines("word-break: keep-all", "日本語の文章", 8, 12, 2),
        ["日本語の文章", "            "]
    );
    assert_eq!(
        lines("word-break: keep-all", "日本語 文章", 8, 12, 2),
        ["日本語      ", "文章        "]
    );
}

/// UAX #14 LB13 (CSS Text 3 §5.5 honours it): no break before closing
/// punctuation — `。` stays with the ideograph before it.
#[test]
fn closing_punctuation_does_not_start_a_line() {
    assert_eq!(lines("", "日本。", 4, 6, 2), ["日    ", "本。  "]);
}

/// §5.5 `overflow-wrap: anywhere` / `break-word` (and `word-wrap`, its
/// legacy name; and §5.2 `word-break: break-word`): "an otherwise
/// unbreakable sequence of characters may be broken at an arbitrary point
/// if there are no otherwise-acceptable break points in the line" — the
/// word that alone overflows its line breaks between graphemes.
#[test]
fn overflow_wrap_breaks_a_word_too_long_for_its_line() {
    assert_eq!(
        lines("", "aa bbbbbbbb", 4, 8, 3),
        ["aa      ", "bbbbbbbb", "        "]
    );
    for decl in [
        "overflow-wrap: anywhere",
        "overflow-wrap: break-word",
        "word-wrap: break-word",
        "word-break: break-word",
    ] {
        assert_eq!(
            lines(decl, "aa bbbbbbbb", 4, 8, 3),
            ["aa      ", "bbbb    ", "bbbb    "],
            "{decl}"
        );
    }
}

/// §5.5: "Soft wrap opportunities introduced by anywhere are considered
/// when calculating min-content intrinsic sizes" — those of `break-word`
/// are not; `break-all`'s are ordinary opportunities; `keep-all` keeps a
/// CJK word whole.
#[test]
fn which_breaks_count_for_min_content() {
    let min = |decl: &str, text: &str| width_of(&format!("width: min-content; {decl}"), text);
    assert_eq!(min("", "abcdef"), 6);
    assert_eq!(min("overflow-wrap: anywhere", "abcdef"), 1);
    assert_eq!(min("overflow-wrap: break-word", "abcdef"), 6);
    assert_eq!(min("word-break: break-word", "abcdef"), 1);
    assert_eq!(min("word-break: break-all", "abcdef"), 1);
    assert_eq!(min("", "日本語"), 2);
    assert_eq!(min("word-break: keep-all", "日本語"), 6);
}

/// §6.1 `hyphens: manual` (the initial value): a soft hyphen is a
/// hyphenation opportunity, and "when a line is broken at a hyphenation
/// opportunity, a hyphen is displayed" at the end of the line; unbroken,
/// it shows nothing. `none` takes no break there; `auto` is `manual`
/// (rdom has no hyphenation dictionary).
#[test]
fn soft_hyphens_break_with_a_visible_hyphen() {
    let word = "hy\u{AD}phen";
    assert_eq!(lines("", word, 3, 6, 2), ["hy-   ", "phen  "]);
    assert_eq!(lines("hyphens: auto", word, 3, 6, 2), ["hy-   ", "phen  "]);
    assert_eq!(lines("hyphens: none", word, 3, 6, 2), ["hyphen", "      "]);
    assert_eq!(lines("", word, 10, 10, 1), ["hyphen    "]);
}

/// The hyphen a broken soft hyphen shows is the soft hyphen's rendering:
/// its cell maps back to the soft hyphen in the source, the text after it
/// to its own bytes (CSS Text 3 §6.1; HTML: the caret and selection work
/// in the DOM text).
#[test]
fn the_shown_hyphen_maps_back_to_the_soft_hyphen() {
    let (mut dom, _, t) = text_block("hy\u{AD}phen");
    lay_out(&mut dom, ".b { width: 3 }", 6, 2);
    // `h` `y` at 0 / 1 on row 0, the soft hyphen (2 bytes) at 2, `phen`
    // from byte 4 on row 1 (byte 4 itself, the boundary between the
    // lines, takes the first line's end, as at any break with no space).
    assert_eq!(cell_of_position(&dom, Position::new(t, 2)), Some((2, 0)));
    assert_eq!(cell_of_position(&dom, Position::new(t, 5)), Some((1, 1)));
    assert_eq!(dom.position_at(2, 0), Some(Position::new(t, 2)));
    assert_eq!(dom.position_at(1, 1), Some(Position::new(t, 5)));
}

/// §5.3 `line-break: anywhere`: "a soft wrap opportunity around every
/// typographic character unit ... in the middle of words".
#[test]
fn line_break_anywhere_breaks_inside_words() {
    assert_eq!(
        lines("line-break: anywhere", "abc def", 2, 4, 4),
        ["ab  ", "c   ", "de  ", "f   "]
    );
}

/// §5.3: breaks before the CJK hyphen-like `〜` are "allowed for normal
/// and loose line breaking ... and are otherwise forbidden"; breaks before
/// small kana (class CJ) are "forbidden for normal and strict line
/// breaking and allowed in loose".
#[test]
fn line_break_strictness_governs_cjk_punctuation_and_small_kana() {
    assert_eq!(
        lines("", "日本〜語", 4, 6, 3),
        ["日本  ", "〜語  ", "      "]
    );
    assert_eq!(
        lines("line-break: strict", "日本〜語", 4, 6, 3),
        ["日    ", "本〜  ", "語    "]
    );
    assert_eq!(lines("", "キャット", 2, 6, 2), ["キャッ", "ト    "]);
    assert_eq!(
        lines("line-break: loose", "キャット", 2, 2, 4),
        ["キ", "ャ", "ッ", "ト"]
    );
}

/// UAX #14 LB8 (honoured, §5.5): a zero-width space is a soft wrap
/// opportunity; HTML `<wbr>` "represents a line break opportunity"; a
/// line separator (class BK) is a forced line break "regardless of the
/// white-space value".
#[test]
fn zero_width_space_wbr_and_line_separators() {
    assert_eq!(lines("", "abc\u{200B}def", 3, 6, 2), ["abc   ", "def   "]);
    assert_eq!(
        lines("", "ab\u{2028}cd", 10, 10, 2),
        ["ab        ", "cd        "]
    );
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let t = dom.create_text_node("abc");
    dom.append_child(b, t).unwrap();
    el(&mut dom, b, "wbr", "");
    let t2 = dom.create_text_node("def");
    dom.append_child(b, t2).unwrap();
    let buf = paint(&mut dom, ".b { width: 3 }", 6, 2);
    assert_eq!(rows(&buf, 6, 2), ["abc   ", "def   "]);
}
