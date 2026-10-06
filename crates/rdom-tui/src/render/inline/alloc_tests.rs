//! C9G-PACKER-ALLOC: what packing default content allocates and counts —
//! nothing per word or per line beyond the fragment text the lines keep
//! (`PACKER-STRING-ALLOC-1`), nothing per query of a fragment's units,
//! nothing per unchanged grapheme under a case transform, and linear work
//! where the merge and `balance` were quadratic.

use super::compute_inline_layout;
use super::source_map::SUMMED;
use crate::render::Rect;
use crate::test_alloc::allocations_in;
use crate::{CascadeExt, LayoutExt, NodeId, TuiDom};

/// A `.b` block of `text` styled `decl`, cascaded and laid out.
fn block(decl: &str, text: &str) -> (TuiDom, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = dom.create_element("div");
    dom.set_attribute(b, "class", "b").unwrap();
    dom.append_child(root, b).unwrap();
    let t = dom.create_text_node(text);
    dom.append_child(b, t).unwrap();
    let sheet = rdom_css::from_css_strict(&format!(".b {{ {decl} }}")).unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 20));
    (dom, b)
}

/// The allocations of packing `text` styled `decl` `width` cells wide.
fn pack_allocations(decl: &str, text: &str, width: u16) -> u64 {
    let (dom, b) = block(decl, text);
    allocations_in(|| {
        compute_inline_layout(&dom, b, width);
    })
}

/// Per word, on one line: the word's text (one string, merged into the
/// line's fragment) and the fragment's amortized growth — no source map
/// units for a word rendered as its source.
#[test]
fn a_word_allocates_its_text_only() {
    let words = |n: usize| pack_allocations("", &"ab ".repeat(n), 1000);
    let per_40 = words(80) - words(40);
    assert!(per_40 <= 40 + 8, "40 words: {per_40} allocations");
}

/// Per line: its fragment list and its word's text — the settled rows
/// reuse the packer's buffers, and a line with only the strut's subtree
/// keeps no baseline list.
#[test]
fn a_line_allocates_its_fragments_and_text_only() {
    let lines = |n: usize| pack_allocations("", &"ab ".repeat(n), 2);
    let per_40 = lines(80) - lines(40);
    assert!(per_40 <= 2 * 40 + 8, "40 lines: {per_40} allocations");
}

/// `uppercase` on text already upper case renders each grapheme as
/// itself: no allocation for it.
#[test]
fn an_unchanged_grapheme_under_a_transform_allocates_nothing() {
    let text = "ABC DEF 123 ".repeat(30);
    let plain = pack_allocations("", &text, 1000);
    let upper = pack_allocations("text-transform: uppercase", &text, 1000);
    assert_eq!(upper, plain);
}

/// The caret, hit-testing and selection walk a fragment's units: no
/// allocation per query, mapped or not.
#[test]
fn walking_a_fragments_units_allocates_nothing() {
    for decl in ["", "text-transform: uppercase"] {
        let (dom, b) = block(decl, "straße straße");
        let il = compute_inline_layout(&dom, b, 40);
        let f = &il.lines[0].fragments[0];
        let n = allocations_in(|| {
            let _ = f.cells_before_source(4);
            let _ = f.source_at_cell(5);
            let _ = f.units().count();
        });
        assert_eq!(n, 0, "{decl:?}");
    }
}

/// Merging a word into its line's fragment reads the fragment's source
/// length; with a source map that was a sum over every unit so far, so a
/// long `pre-wrap` line of tabs was quadratic. Linear now.
#[test]
fn merging_mapped_words_is_linear() {
    let (dom, b) = block("white-space: pre-wrap; tab-size: 4", &"x\t".repeat(300));
    SUMMED.with(|c| c.set(0));
    compute_inline_layout(&dom, b, 4000);
    let summed = SUMMED.with(|c| c.get());
    assert!(summed <= 4 * 600, "{summed} units summed for 600");
}

/// `balance` counts each group's lines once per pack: a 2000-line
/// `pre-line` log under an inherited `balance` was 2000 × 2000.
#[test]
fn balance_counts_lines_linearly() {
    let (dom, b) = block(
        "white-space: pre-line; text-wrap: balance",
        &"aa\n".repeat(2000),
    );
    super::wrap::COUNTED.with(|c| c.set(0));
    compute_inline_layout(&dom, b, 40);
    let counted = super::wrap::COUNTED.with(|c| c.get());
    assert!(counted <= 4 * 2000, "{counted} line visits for 2000 lines");
}
