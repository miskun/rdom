//! C5G-ATOM-BOX — an inline block in a line is one atomic box. CSS 2.1
//! §10.8 / §10.8.1: the line box is tall enough for the atom's margin
//! box, the atom's baseline (its last line box) sits on the line's
//! baseline; CSS 2.1 Appendix E, 7.2.1.4.1.1 (step 5 of the inline
//! content): the atom paints atomically at its turn in the line —
//! outer shadow, background, border, padding, then its content —
//! exactly once, whether the line is an inline formatting context of
//! its own or an anonymous block box beside bare text.

use super::{el, paint, rect, rows};
use rdom_tui::{Color, NodeId, TuiDom};

const RED: Color = Color::Rgb(255, 0, 0);

fn text(dom: &mut TuiDom, parent: NodeId, s: &str) {
    let t = dom.create_text_node(s);
    dom.append_child(parent, t).unwrap();
}

/// `<p>aa<span class=ib>b</span>cc</p>`, the text bare or inside `<i>`
/// (which makes `<p>` an inline formatting context; bare text goes
/// through an anonymous block box). Returns the dom, the atom and `p`.
fn line_with_atom(bare: bool) -> (TuiDom, NodeId, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    if bare {
        text(&mut dom, p, "aa");
    } else {
        let i = el(&mut dom, p, "i", "");
        text(&mut dom, i, "aa");
    }
    let ib = el(&mut dom, p, "span", "ib");
    text(&mut dom, ib, "b");
    if bare {
        text(&mut dom, p, "cc");
    } else {
        let i = el(&mut dom, p, "i", "");
        text(&mut dom, i, "cc");
    }
    (dom, ib, p)
}

/// The atom's border and background paint, the line box grows to the
/// atom's three rows and the text sits on the atom's baseline (its
/// content row) — inside text and beside bare text alike.
#[test]
fn an_inline_block_paints_its_border_and_background_in_the_line() {
    for bare in [false, true] {
        let (mut dom, ib, _) = line_with_atom(bare);
        let buf = paint(
            &mut dom,
            ".ib { display: inline-block; border: solid; background-color: red }",
            7,
            4,
        );
        assert_eq!(
            rows(&buf, 7, 4),
            vec!["  ┌─┐  ", "aa│b│cc", "  └─┘  ", "       "],
            "bare text: {bare}"
        );
        assert_eq!(
            rect(&dom, ib),
            rdom_tui::LayoutRect::new(2, 0, 3, 3),
            "bare: {bare}"
        );
        for (x, y) in [(2, 0), (3, 1), (4, 2)] {
            assert_eq!(buf.cell(x, y).unwrap().bg, RED, "({x}, {y}), bare: {bare}");
        }
        assert_ne!(buf.cell(1, 1).unwrap().bg, RED, "bare: {bare}");
    }
}

/// Padding is part of the atom's box: its background fills it, and the
/// content sits inside it.
#[test]
fn an_inline_blocks_padding_is_painted_with_its_background() {
    for bare in [false, true] {
        let (mut dom, _, _) = line_with_atom(bare);
        let buf = paint(
            &mut dom,
            ".ib { display: inline-block; padding: 0 1; background-color: red }",
            7,
            1,
        );
        assert_eq!(rows(&buf, 7, 1), vec!["aa b cc"], "bare: {bare}");
        for x in 2..5 {
            assert_eq!(buf.cell(x, 0).unwrap().bg, RED, "({x}, 0), bare: {bare}");
        }
    }
}

/// The atom paints exactly once: a translucent background composites
/// one time over the paragraph's, the same beside bare text as inside
/// an inline formatting context (CSS Color 4 §4.2: one layer, one blend).
#[test]
fn an_inline_block_paints_once() {
    for bare in [false, true] {
        let (mut dom, _, _) = line_with_atom(bare);
        let buf = paint(
            &mut dom,
            "p { background-color: rgb(0 0 255) } \
             .ib { display: inline-block; background-color: rgb(255 0 0 / 50%) }",
            7,
            1,
        );
        assert_eq!(rows(&buf, 7, 1), vec!["aabcc  "], "bare: {bare}");
        assert_eq!(
            buf.cell(2, 0).unwrap().bg,
            Color::Rgb(128, 0, 127),
            "bare: {bare}"
        );
    }
}

/// A line after a tall atom starts below the atom's line box, so the
/// atom's bottom border does not lie over the next line's text.
#[test]
fn the_next_line_starts_below_a_tall_atom() {
    for bare in [false, true] {
        let (mut dom, _, _) = line_with_atom(bare);
        let buf = paint(
            &mut dom,
            "p { width: 5 } .ib { display: inline-block; border: solid }",
            7,
            4,
        );
        // `aa` + the 3-cell atom fill the 5-cell line; `cc` wraps.
        assert_eq!(
            rows(&buf, 7, 4),
            vec!["  ┌─┐  ", "aa│b│  ", "  └─┘  ", "cc     "],
            "bare: {bare}"
        );
    }
}

/// Hit-testing follows the taller line box: the text beside the atom is
/// on the line's baseline row, and every row of the atom's border box
/// hits the atom (CSS 2.1 §10.8; CSSOM View `elementFromPoint`).
#[test]
fn hit_testing_reads_the_taller_line_box() {
    use rdom_tui::HitTestExt;
    for bare in [false, true] {
        let (mut dom, ib, p) = line_with_atom(bare);
        paint(
            &mut dom,
            ".ib { display: inline-block; border: solid }",
            7,
            4,
        );
        for y in 0..3 {
            assert_eq!(dom.hit_test(3, y), Some(ib), "(3, {y}), bare: {bare}");
        }
        let cc = dom.position_at(6, 1).expect("the text beside the atom");
        assert_eq!(dom.node(cc.node).node_value(), Some("cc"), "bare: {bare}");
        assert_eq!(cc.offset, 1, "bare: {bare}");
        // The rows above / below the text, beside the atom, are the
        // line box's: they hit the paragraph, no text.
        assert_ne!(dom.hit_test(6, 0), Some(ib), "bare: {bare}");
        assert_ne!(dom.hit_test(6, 0), None, "bare: {bare}");
        let _ = p;
    }
}

/// CSS 2.1 §10.8.1: an inline block's baseline is its last line box —
/// with a `height` taller than its content the text beside it lines up
/// with the content's line, and the box hangs below — or, with no line
/// box in it, its bottom margin edge: an empty box sits on the text row
/// and rises above it.
#[test]
fn an_atoms_baseline_is_its_last_line_or_its_bottom_edge() {
    for bare in [false, true] {
        let (mut dom, ib, _) = line_with_atom(bare);
        paint(
            &mut dom,
            ".ib { display: inline-block; height: 3; background-color: red }",
            7,
            4,
        );
        assert_eq!(
            rect(&dom, ib),
            rdom_tui::LayoutRect::new(2, 0, 1, 3),
            "bare: {bare}"
        );

        let (mut dom, ib, _) = line_with_atom(bare);
        dom.set_text_content(ib, "").unwrap();
        let buf = paint(
            &mut dom,
            ".ib { display: inline-block; width: 2; height: 2; background-color: red }",
            7,
            3,
        );
        assert_eq!(
            rect(&dom, ib),
            rdom_tui::LayoutRect::new(2, 0, 2, 2),
            "bare: {bare}"
        );
        assert_eq!(
            rows(&buf, 7, 3),
            vec!["       ", "aa  cc ", "       "],
            "bare: {bare}"
        );
    }
}

/// C6G-ATOM-HIT — an atom is a box for hit-testing too (CSSOM View
/// `elementFromPoint`, CSS 2.1 Appendix E): a point on its content hits
/// that content, inside an inline formatting context as beside bare
/// text; and a `visibility: hidden` atom (CSS Display 3 §4) is no
/// target, while its `visible` child is — with the atom on its path.
#[test]
fn hit_testing_descends_into_an_atom() {
    use rdom_tui::HitTestExt;
    for bare in [false, true] {
        let (mut dom, ib, p) = line_with_atom(bare);
        let k = el(&mut dom, ib, "b", "k");
        text(&mut dom, k, "k");
        paint(
            &mut dom,
            ".ib { display: inline-block; border: solid }",
            8,
            4,
        );
        // `│bk│` at x 2..6 on the content row: `b` at 3 is the atom's
        // own text, `k` at 4 its child.
        assert_eq!(dom.hit_test(4, 1), Some(k), "bare: {bare}");
        assert!(dom.hit_test_path(4, 1).contains(&ib), "bare: {bare}");
        assert_eq!(dom.hit_test(3, 1), Some(ib), "bare: {bare}");

        paint(
            &mut dom,
            ".ib { display: inline-block; border: solid; visibility: hidden } \
             .k { visibility: visible }",
            8,
            4,
        );
        assert_eq!(dom.hit_test(4, 1), Some(k), "bare: {bare}");
        assert!(dom.hit_test_path(4, 1).contains(&ib), "bare: {bare}");
        // The hidden atom's own cells are what is beneath: the paragraph.
        assert_eq!(dom.hit_test(2, 1), Some(p), "bare: {bare}");
    }
}

/// ACID-FIX-10 (found by acid tile 25). CSS 2.1 §10.3.9 / §9.4.2: an
/// inline block's margins apply, and the box it places in its line is its
/// margin box (§10.8.1) — `margin-left: 1` and `margin-right: 2` put a
/// blank cell before the atom and two after it, in a line and in a
/// shrink-to-fit width alike. rdom placed the border box alone, the
/// horizontal margins dropped. A negative horizontal margin counts as
/// zero, as a negative vertical one does (DIVERGENCES §2).
#[test]
fn an_inline_blocks_horizontal_margins_take_their_cells() {
    for bare in [false, true] {
        let (mut dom, ib, _) = line_with_atom(bare);
        let buf = paint(
            &mut dom,
            ".ib { display: inline-block; margin-left: 1; margin-right: 2 }",
            8,
            2,
        );
        assert_eq!(rows(&buf, 8, 1), vec!["aa b  cc"], "bare text: {bare}");
        assert_eq!(rect(&dom, ib).x, 3, "bare text: {bare}");
    }
    // The shrink-to-fit width of a float holding the line counts them.
    let (mut dom, _, p) = line_with_atom(true);
    let buf = paint(
        &mut dom,
        "p { float: left } .ib { display: inline-block; margin: 0 2 0 1 }",
        10,
        2,
    );
    assert_eq!(rows(&buf, 10, 1), vec!["aa b  cc  "]);
    assert_eq!(rect(&dom, p).width, 8);
    // An inline-block `::before` likewise (CSS Pseudo-Elements 4 §2).
    let (mut dom, _, _) = line_with_atom(true);
    let buf = paint(
        &mut dom,
        ".ib::before { content: 'x'; display: inline-block; margin: 0 2 0 1 }",
        10,
        2,
    );
    assert_eq!(rows(&buf, 10, 1), vec!["aa x  bcc "]);
}
