//! C9-TEXT-TRANSFORM — `text-transform` (CSS Text 3 §2.1, CSS Text 4
//! §2.1; MathML Core §4.2 for `math-auto`): a rendering transform — the
//! caret, selection and copy work in the source text.

use super::{el, lay_out, paint, paint_text, rows, size, text_block};
use rdom_tui::prelude::*;
use rdom_tui::render::inline::cell_of_position;
use rdom_tui::runtime::selection::clipboard::serialize_selection;

/// §2.1 `uppercase` / `lowercase` "put all letters in" upper / lower case
/// with the full Unicode case mappings: `ß` is `SS` (two cells), and a
/// word-final capital sigma lowers to `ς` (Unicode's Final_Sigma).
#[test]
fn case_mapping_is_full() {
    assert_eq!(
        paint_text("text-transform: uppercase", "Straße", 8, 1),
        ["STRASSE "]
    );
    assert_eq!(
        paint_text("text-transform: lowercase", "ΟΔΟΣ ΣΑ", 8, 1),
        ["οδος σα "]
    );
}

/// §2.1 `capitalize`: "the first typographic letter unit of each word, if
/// lowercase, in titlecase; other characters are unaffected" — words
/// split at spaces and punctuation, an apostrophe inside a word does not
/// start one (one before it does not join the word to it), and
/// titlecase is not uppercase for digraphs (`ǆ` → `ǅ`).
#[test]
fn capitalize_titlecases_each_words_first_letter() {
    assert_eq!(
        paint_text(
            "text-transform: capitalize",
            "hello wORLD don't 3rd well-known ǆungla 'quoted'",
            48,
            1
        ),
        ["Hello WORLD Don't 3rd Well-Known ǅungla 'Quoted'"]
    );
}

/// §2.1 `full-width`: "puts all typographic character units in full-width
/// form" — two cells each, so they break like ideographs (§5); applied
/// after white space collapsing, so only preserved spaces become U+3000
/// (§2.1's ordering note); combined with a case value, the case applies
/// first.
#[test]
fn full_width_uses_the_fullwidth_forms() {
    assert_eq!(
        paint_text("text-transform: full-width", "ab  12", 10, 1),
        ["ａｂ １２ "]
    );
    assert_eq!(
        paint_text("text-transform: full-width; white-space: pre", "a b", 6, 1),
        ["ａ　ｂ"]
    );
    assert_eq!(
        paint_text("text-transform: full-width; width: 4", "abcd", 6, 2),
        ["ａｂ  ", "ｃｄ  "]
    );
    assert_eq!(
        paint_text("text-transform: uppercase full-width", "ab", 4, 1),
        ["ＡＢ"]
    );
}

/// §2.1 `full-size-kana`: "converts all small Kana characters to the
/// equivalent full-size Kana".
#[test]
fn full_size_kana_enlarges_small_kana() {
    assert_eq!(
        paint_text("text-transform: full-size-kana", "ぁっャ", 6, 1),
        ["あつヤ"]
    );
}

/// MathML Core §4.2: "on text nodes containing a single character, if the
/// computed value is math-auto and the character is present in the
/// 'Original' column ... it is converted to the ... 'italic' column"; a
/// longer text node is untouched.
#[test]
fn math_auto_italicizes_a_single_letter() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    for text in ["x", "+", "cos", "h"] {
        let s = el(&mut dom, b, "span", "m");
        let t = dom.create_text_node(text);
        dom.append_child(s, t).unwrap();
    }
    let buf = paint(&mut dom, ".m { text-transform: math-auto }", 8, 1);
    assert_eq!(rows(&buf, 8, 1), ["𝑥+cosℎ  "]);
}

/// §2.1: the transform "has no effect on the underlying content, and must
/// not affect the content of a plain text copy & paste operation" — the
/// caret and hit-testing map the rendered cells back to the source
/// (`ß` is both cells of `SS`), and a copy is the source text.
#[test]
fn the_source_text_is_what_the_caret_and_copy_see() {
    let (mut dom, _, t) = text_block("Straße");
    lay_out(&mut dom, ".b { text-transform: uppercase }", 8, 1);
    assert_eq!(cell_of_position(&dom, Position::new(t, 4)), Some((4, 0)));
    assert_eq!(cell_of_position(&dom, Position::new(t, 6)), Some((6, 0)));
    assert_eq!(dom.position_at(5, 0), Some(Position::new(t, 4)));
    let range = rdom_tui::Range::ordered_unchecked(Position::new(t, 0), Position::new(t, 7));
    assert_eq!(serialize_selection(&dom, &range), "Straße");
}

/// The min-content width of a `.b` block holding `text`, styled `decl`.
fn min_content(decl: &str, text: &str) -> u16 {
    let (mut dom, b, _) = text_block(text);
    lay_out(
        &mut dom,
        &format!(".b {{ width: min-content; {decl} }}"),
        30,
        4,
    );
    size(&dom, b).0
}

/// C9G-TRANSFORM-BREAK. §2.1: the transform applies "before line breaking",
/// and UAX #14 classes the *characters* of the rendered text (LB9: a
/// grapheme takes its first character's class): `ß` uppercased is `SS`, two
/// letters (AL), not one two-cell ideograph (ID) — so `STRASSE` keeps no
/// break opportunity inside, overflows its 5-cell line whole (as browsers
/// do), and its min-content is the whole word.
#[test]
fn a_lengthened_letter_stays_a_letter_for_line_breaking() {
    assert_eq!(
        paint_text("width: 5; text-transform: uppercase", "straße", 8, 2),
        ["STRASSE ", "        "]
    );
    assert_eq!(min_content("text-transform: uppercase", "straße"), 7);
    // `capitalize` titlecases a digraph (`ǆ` → `ǅ`, one cell) and a lowered
    // final sigma is `ς`: neither opens a break inside the word.
    assert_eq!(
        paint_text("width: 3; text-transform: capitalize", "ǆungla", 8, 2),
        ["ǅungla  ", "        "]
    );
    assert_eq!(
        paint_text("width: 4; text-transform: lowercase", "ΟΔΟΣ ΣΑ", 6, 2),
        ["οδος  ", "σα    "]
    );
    assert_eq!(min_content("text-transform: capitalize", "ǆungla"), 6);
}

/// C9G-TRANSFORM-BREAK. §2.1 `full-width` makes letters full-width forms,
/// UAX #14 class ID, so they break on either side — also when one source
/// letter renders as several (`ß` → `ＳＳ` under `uppercase full-width`): the
/// rendered text's own characters decide, not its total width.
#[test]
fn full_width_text_breaks_like_ideographs_whatever_its_length() {
    assert_eq!(
        paint_text("width: 4; text-transform: uppercase full-width", "aß", 6, 2),
        ["Ａ    ", "ＳＳ  "]
    );
    assert_eq!(min_content("text-transform: uppercase full-width", "aß"), 4);
}
