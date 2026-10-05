//! C8-LINE-CLAMP — `line-clamp` (CSS Overflow 4 §4): a block container
//! with `max-lines: N` and `continue: collapse` ends after its Nth line
//! box — its automatic height cut there, what follows hidden — and that
//! line takes the `block-ellipsis`; the legacy `-webkit-line-clamp` on a
//! vertical `-webkit-box` does the same.

use super::{el, paint, rect, rows};
use rdom_tui::prelude::*;

/// A `.c` (styled `c`, inside a block `.w`) holding `text`, then a `.x`
/// line `X`; painted 8 × 5. Returns the rows and `.c`'s height.
fn clamp(c: &str, text: &str) -> (Vec<String>, u16) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let w = el(&mut dom, root, "div", "w");
    let cbox = el(&mut dom, w, "div", "c");
    let t = dom.create_text_node(text);
    dom.append_child(cbox, t).unwrap();
    let x = el(&mut dom, w, "div", "x");
    let t = dom.create_text_node("X");
    dom.append_child(x, t).unwrap();
    let buf = paint(&mut dom, &format!(".c {{ {c} }}"), 8, 5);
    (rows(&buf, 8, 5), rect(&dom, cbox).height)
}

/// §4.1 / §4.4: `line-clamp: 2` — the box is two lines tall, the third
/// is hidden, the second ends with `…` (`block-ellipsis: auto`), and the
/// next box follows right after.
#[test]
fn the_box_ends_after_its_nth_line_with_an_ellipsis() {
    let (rows, height) = clamp("width: 7; line-clamp: 2", "one two three four");
    assert_eq!(height, 2);
    assert_eq!(
        rows,
        ["one two ", "three…  ", "X       ", "        ", "        "]
    );
}

/// Nothing after the Nth line: no clamp point is reached, no ellipsis.
#[test]
fn a_box_that_fits_is_untouched() {
    let (rows, height) = clamp("width: 7; line-clamp: 2", "one two three");
    assert_eq!(height, 2);
    assert_eq!(&rows[..3], ["one two ", "three   ", "X       "]);
}

/// §4.3: the ellipsis is a string, or nothing; a full last line gives
/// up whole characters to make room for it.
#[test]
fn the_block_ellipsis_takes_its_value_and_its_room() {
    let (rows, _) = clamp("width: 7; line-clamp: 2 '>'", "one two three four");
    assert_eq!(rows[1], "three>  ");
    let (rows, _) = clamp("width: 7; line-clamp: 2 no-ellipsis", "one two three four");
    assert_eq!(rows[1], "three   ");
    let (rows, _) = clamp("width: 5; line-clamp: 2", "abcde fghij klm");
    assert_eq!(&rows[..3], ["abcde   ", "fghi…   ", "X       "]);
}

/// §4.2: the clamp point follows the Nth in-flow line box of the box's
/// content, whichever block descendant holds it; what follows — the
/// rest of that block, the blocks after — is hidden.
#[test]
fn lines_count_across_block_descendants() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let w = el(&mut dom, root, "div", "w");
    let c = el(&mut dom, w, "div", "c");
    for text in ["a\nb", "c\nd", "e"] {
        let p = el(&mut dom, c, "div", "p");
        let t = dom.create_text_node(text);
        dom.append_child(p, t).unwrap();
    }
    let buf = paint(
        &mut dom,
        ".c { line-clamp: 3 } .p { white-space: pre }",
        8,
        5,
    );
    assert_eq!(rect(&dom, c).height, 3);
    assert_eq!(
        rows(&buf, 8, 5),
        ["a       ", "b       ", "c…      ", "        ", "        "]
    );
}

/// The legacy form every browser supports: `display: -webkit-box;
/// -webkit-box-orient: vertical; -webkit-line-clamp: N` clamps (§4.4
/// `-webkit-legacy`); without the vertical orient it does not.
#[test]
fn the_webkit_legacy_form_clamps() {
    let legacy = "width: 7; display: -webkit-box; -webkit-line-clamp: 2; overflow: hidden";
    let (rows, height) = clamp(
        &format!("{legacy}; -webkit-box-orient: vertical"),
        "one two three four",
    );
    assert_eq!(height, 2);
    assert_eq!(&rows[..3], ["one two ", "three…  ", "X       "]);
    let (_, height) = clamp(legacy, "one two three four");
    assert_ne!(height, 2, "a horizontal -webkit-box is not clamped");
}

/// `continue: discard` hides what follows the clamp point too (rdom has
/// no fragmentation to discard it into, DIVERGENCES).
#[test]
fn discard_clamps_like_collapse() {
    let (rows, height) = clamp(
        "width: 7; max-lines: 2; continue: discard; block-ellipsis: auto",
        "one two three four",
    );
    assert_eq!(height, 2);
    assert_eq!(rows[1], "three…  ");
}

/// A clamped box sized by its parent's flex layout (here a flex item of
/// the document root) takes the clamped height too.
#[test]
fn a_clamped_flex_item_is_n_lines_tall() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    let t = dom.create_text_node("one two three four");
    dom.append_child(c, t).unwrap();
    super::lay_out(&mut dom, ".c { width: 7; line-clamp: 2 }", 8, 5);
    assert_eq!(rect(&dom, c).height, 2);
}

/// C8G-WEBKIT-CLAMP — the form autoprefixers and Tailwind emit: the
/// legacy declarations, then the standard `line-clamp: 3`, whose
/// `continue: collapse` used to switch the legacy form off and leave a
/// flex row. Every engine clamps it to three lines (§4.4 with the
/// Compat Standard's `-webkit-box`).
#[test]
fn the_prefixed_and_standard_pair_clamps() {
    let (rows, height) = clamp(
        "width: 7; display: -webkit-box; -webkit-box-orient: vertical; \
         -webkit-line-clamp: 2; line-clamp: 2; overflow: hidden",
        "one two three four",
    );
    assert_eq!(height, 2);
    assert_eq!(&rows[..3], ["one two ", "three…  ", "X       "]);
}

/// A vertical `-webkit-box` without a clamp stacks its children, as the
/// legacy flexbox lays its `-webkit-box-orient: vertical` out (Chromium:
/// a column); a horizontal one is a row.
#[test]
fn a_vertical_webkit_box_stacks_its_children() {
    let lay = |orient: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let c = el(&mut dom, root, "div", "c");
        for text in ["ab", "cd"] {
            let s = el(&mut dom, c, "span", "");
            let t = dom.create_text_node(text);
            dom.append_child(s, t).unwrap();
        }
        let buf = paint(
            &mut dom,
            &format!(".c {{ display: -webkit-box; -webkit-box-orient: {orient} }}"),
            8,
            3,
        );
        rows(&buf, 8, 2)
    };
    assert_eq!(lay("vertical"), ["ab      ", "cd      "]);
    assert_eq!(lay("horizontal")[0], "abcd    ");
}

/// Architect N5: a clamped paragraph in a scroller adds its clamped box
/// to the scrollable overflow, not the lines it hides — a 3-row scroller
/// holding a `line-clamp: 2` paragraph of ten lines has nothing to scroll.
#[test]
fn a_clamped_box_adds_no_hidden_lines_to_its_scroller() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = el(&mut dom, root, "div", "s");
    let p = el(&mut dom, s, "div", "p");
    let t = dom.create_text_node("1\n2\n3\n4\n5\n6\n7\n8\n9\n10");
    dom.append_child(p, t).unwrap();
    super::lay_out(
        &mut dom,
        ".s { width: 8; height: 3; overflow: auto } \
         .p { line-clamp: 2; white-space: pre }",
        10,
        5,
    );
    assert_eq!(dom.node(s).scroll_height(), Some(3));
}

/// Architect N5: the clamp point's "content follows" test reads content,
/// not a block's padding — a paragraph's two lines and its bottom padding
/// under `line-clamp: 2` end with no ellipsis.
#[test]
fn a_blocks_padding_after_the_nth_line_is_not_content() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let w = el(&mut dom, root, "div", "w");
    let c = el(&mut dom, w, "div", "c");
    let p = el(&mut dom, c, "p", "p");
    let t = dom.create_text_node("a\nb");
    dom.append_child(p, t).unwrap();
    let buf = paint(
        &mut dom,
        ".c { line-clamp: 2 } .p { padding-bottom: 1; white-space: pre }",
        8,
        4,
    );
    assert_eq!(&rows(&buf, 8, 4)[..2], ["a       ", "b       "]);
}

/// C8G-IDLE-COST: the clamp point layout keeps for paint moves with its
/// box — a relatively positioned clamped box shifted a row down after it
/// was laid out still hides its third line and marks its second.
#[test]
fn a_shifted_clamped_box_keeps_its_clamp_point() {
    let (rows, _) = clamp(
        "width: 7; line-clamp: 2; position: relative; top: 1",
        "one two three four",
    );
    assert_eq!(&rows[1..3], ["one two ", "three…  "]);
    assert_eq!(rows[3], "        ", "the third line hidden");
}
