//! C9G-LETTER-SPACING — `letter-spacing` and `word-spacing` (CSS Text 3
//! §9.1, §9.2) in whole cells: blank cells after a typographic character
//! unit, a fractional length floored, nothing negative, pixel and
//! font-relative lengths invalid (DESIGN "Pixel lengths select, cells
//! measure": spacing is geometry).

use super::{el, lay_out, paint, paint_text, rows, size, text_block};
use rdom_tui::prelude::*;
use rdom_tui::render::inline::cell_of_position;
use rdom_tui::runtime::selection::clipboard::serialize_selection;

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

/// §9.2: letter spacing is "additional space between typographic
/// character units" — a cell after each grapheme at `1` — and "must not be
/// applied at the beginning or at the end of a line": `abc` is `a b c`,
/// five cells, not six, so a line of it fits five cells.
#[test]
fn letter_spacing_spaces_graphemes_but_not_a_lines_end() {
    assert_eq!(paint_text("letter-spacing: 1", "abc", 7, 1), ["a b c  "]);
    assert_eq!(min_content("letter-spacing: 1", "abc"), 5);
    assert_eq!(paint_text("letter-spacing: 2", "ab", 6, 1), ["a  b  "]);
    // A word that fits only without its last letter's spacing stays on
    // the line; the spacing after the space between words is kept.
    assert_eq!(
        paint_text("width: 3; letter-spacing: 1", "ab cd", 3, 2),
        ["a b", "c d"]
    );
    assert_eq!(
        paint_text("width: 7; letter-spacing: 1", "a bc", 7, 2),
        ["a   b c", "       "]
    ); // `overflow-wrap: anywhere` breaks a word where its graphemes fit
    // without the last one's spacing.
    assert_eq!(
        paint_text(
            "width: 3; overflow-wrap: anywhere; letter-spacing: 1",
            "abcd",
            3,
            2
        ),
        ["a b", "c d"]
    );
}

/// §9.1: word spacing is "additional spacing between words", applied to
/// each word-separator character — a space, collapsed or preserved;
/// letter spacing also follows the space, as it is a character unit.
#[test]
fn word_spacing_widens_word_separators() {
    assert_eq!(paint_text("word-spacing: 2", "a b", 6, 1), ["a   b "]);
    assert_eq!(
        paint_text("word-spacing: 1; white-space: pre", "a  b", 6, 1),
        ["a    b"]
    );
    assert_eq!(
        paint_text("letter-spacing: 1; word-spacing: 1", "ab cd", 11, 1),
        ["a b    c d "]
    );
}

/// Whole cells: a fractional length floors (as `line-height` does,
/// DIVERGENCES §1), a negative one is no spacing (glyphs cannot overlap
/// on a grid), and the `normal` keyword is none; both inherit.
#[test]
fn spacing_is_whole_non_negative_cells() {
    assert_eq!(paint_text("letter-spacing: 1.9", "ab", 4, 1), ["a b "]);
    assert_eq!(paint_text("letter-spacing: 0.5ch", "ab", 4, 1), ["ab  "]);
    assert_eq!(paint_text("letter-spacing: -1", "ab", 4, 1), ["ab  "]);
    assert_eq!(paint_text("letter-spacing: normal", "ab", 4, 1), ["ab  "]);
    let mut dom = TuiDom::new();
    let root = dom.root();
    let outer = el(&mut dom, root, "div", "o");
    let inner = el(&mut dom, outer, "span", "");
    let t = dom.create_text_node("ab");
    dom.append_child(inner, t).unwrap();
    let buf = paint(&mut dom, ".o { letter-spacing: 1 }", 4, 1);
    assert_eq!(rows(&buf, 4, 1), ["a b "]);
}

/// DESIGN "Pixel lengths select, cells measure": spacing is geometry, so a
/// pixel or font-relative length is invalid (dropped with a warning), as
/// `width: 10px` is; a cell, `ch`, viewport or line-height length is
/// taken.
#[test]
fn pixel_and_font_relative_lengths_are_invalid() {
    for value in ["2px", "0.1em", "1rem", "10%"] {
        assert!(
            rdom_css::from_css_strict(&format!(".b {{ letter-spacing: {value} }}")).is_err(),
            "letter-spacing: {value}"
        );
        assert!(
            rdom_css::from_css_strict(&format!(".b {{ word-spacing: {value} }}")).is_err(),
            "word-spacing: {value}"
        );
    }
    assert_eq!(
        paint_text("letter-spacing: 10vw", "ab", 10, 1),
        ["a b       "]
    );
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let mut style = dom.node_mut(b);
    let mut style = style.style_mut().expect("an element");
    style.set_property("letter-spacing", "2ch").unwrap();
    style.set_property("word-spacing", "normal").unwrap();
    let style = dom.node(b).style().expect("an element");
    assert_eq!(style.get_property_value("letter-spacing"), "2ch");
    assert_eq!(style.get_property_value("word-spacing"), "normal");
}

/// Intrinsic sizes count the spacing as layout places it: min-content is
/// the widest word without its trailing spacing, max-content the line.
#[test]
fn intrinsic_sizes_count_the_spacing() {
    assert_eq!(min_content("letter-spacing: 1", "abc de"), 5);
    let (mut dom, b, _) = text_block("ab cd");
    lay_out(
        &mut dom,
        ".b { width: max-content; letter-spacing: 1; word-spacing: 1 }",
        30,
        4,
    );
    assert_eq!(size(&dom, b).0, 10);
}

/// §9.2 / §7.3: justification adds its space after the word separator,
/// which letter spacing has widened — still an opportunity.
#[test]
fn justification_still_finds_the_widened_separators() {
    assert_eq!(
        paint_text(
            "width: 10; text-align: justify; letter-spacing: 1",
            "ab cd ef",
            10,
            2
        ),
        ["a b    c d", "e f       "]
    );
}

/// The spacing is not source text: the caret after `a` sits past its
/// spacing, a click on the spacing is on `a`, and a copy is the DOM text.
#[test]
fn the_caret_and_copy_see_the_source() {
    let (mut dom, _, t) = text_block("ab cd");
    lay_out(&mut dom, ".b { letter-spacing: 1 }", 12, 1);
    assert_eq!(cell_of_position(&dom, Position::new(t, 1)), Some((2, 0)));
    assert_eq!(cell_of_position(&dom, Position::new(t, 3)), Some((6, 0)));
    assert_eq!(dom.position_at(1, 0), Some(Position::new(t, 0)));
    assert_eq!(dom.position_at(6, 0), Some(Position::new(t, 3)));
    let range = rdom_tui::Range::ordered_unchecked(Position::new(t, 0), Position::new(t, 5));
    assert_eq!(serialize_selection(&dom, &range), "ab cd");
}

/// §9.2: "When the effective spacing between two characters is not zero
/// … UAs should not apply letter-spacing to cursive scripts": Arabic
/// letters join, so they take none.
#[test]
fn cursive_scripts_take_no_letter_spacing() {
    assert_eq!(min_content("letter-spacing: 1", "سلام"), 4);
    assert_eq!(min_content("letter-spacing: 1", "ab"), 3);
}

/// Generated text is spaced as text is.
#[test]
fn generated_text_is_spaced() {
    assert_eq!(
        paint_text("letter-spacing: 1 } .b::before { content: 'xy'", "z", 6, 1),
        ["x y z "]
    );
}

/// CSS Overflow 4 §3: an ellipsis replaces whole units at the end of the
/// clipped line, the spacing with its letter.
#[test]
fn text_overflow_cuts_spaced_text_by_units() {
    assert_eq!(
        paint_text(
            "width: 4; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; \
             letter-spacing: 1",
            "abcdef",
            6,
            1
        ),
        ["a b…  "]
    );
}

/// C9G-MISC-CORRECTNESS — CSS Sizing 3 §5.1: an inline block holding only
/// generated text is as wide as that text laid out — measured by the
/// packer, as it is painted: transformed (`ß` → `SS`), collapsed, at its tab
/// stops, letter-spaced.
#[test]
fn generated_text_is_measured_as_it_is_laid_out() {
    for (decl, width) in [
        ("content: 'straße'; text-transform: uppercase", 7),
        ("content: 'ab'; letter-spacing: 1", 3),
        ("content: 'a    b'", 3),
        ("content: 'a\\9 b'; white-space: pre; tab-size: 4", 5),
    ] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let b = el(&mut dom, root, "div", "b");
        let ib = el(&mut dom, b, "span", "ib");
        lay_out(
            &mut dom,
            &format!(".ib {{ display: inline-block }} .ib::before {{ {decl} }}"),
            20,
            2,
        );
        assert_eq!(size(&dom, ib).0, width, "{decl}");
    }
}
