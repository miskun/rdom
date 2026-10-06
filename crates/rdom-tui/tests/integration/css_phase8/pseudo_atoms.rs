//! C8G-PSEUDO-ATOMS — `::before` / `::after` take the box their
//! `display` and `float` make (CSS Pseudo 4 §2: they "are rendered as
//! boxes … as if they were real elements"; CSS 2.1 §12.1): an
//! `inline-block` / `inline-flex` / `inline-grid` / `inline flow-root` one
//! is an atomic inline of its host's line (CSS Display 3 §2.4, CSS 2.1
//! §10.8), a floated one floats (§9.5), and a `flex` / `grid` one lays its
//! content — one anonymous item (CSS Flexbox §4, CSS Grid 2 §6.1) — out as
//! a flex or grid container.

use super::{el, lay_out, paint, rect};
use rdom_tui::prelude::*;
use rdom_tui::style::Color;

const RED: Color = Color::Rgb(200, 0, 0);

/// `<body><div class=h>{text}</div></body>` painted `w` × `h` under
/// `css`: the rows and the buffer.
fn host(css: &str, text: &str, w: u16, h: u16) -> (Vec<String>, Buffer) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let host = el(&mut dom, body, "div", "h");
    let t = dom.create_text_node(text);
    dom.append_child(host, t).unwrap();
    let buf = paint(&mut dom, css, w, h);
    (super::rows(&buf, w, h), buf)
}

/// CSS 2.1 §10.8 / Display 3 §2.4: an `inline-block` pseudo-element is
/// one box in its line — its padding and background drawn there, the
/// host's text after it.
#[test]
fn an_inline_block_pseudo_is_an_atom_with_its_box() {
    let (rows, buf) = host(
        ".h::before { content: \"X\"; display: inline-block; padding: 0 1; \
         background-color: rgb(200, 0, 0) }",
        "body",
        10,
        2,
    );
    assert_eq!(rows[0], " X body   ");
    for x in 0..3 {
        assert_eq!(buf.cell(x, 0).unwrap().bg, RED, "column {x}");
    }
    assert_ne!(buf.cell(3, 0).unwrap().bg, RED);
}

/// Its content wraps inside it, and its baseline — its last line box
/// (§10.8.1) — sits on the host's text row: a two-row atom puts "body"
/// on the second row.
#[test]
fn a_multi_row_pseudo_atom_aligns_its_last_line_with_the_text() {
    let (rows, _) = host(
        ".h::before { content: \"aa bb\"; display: inline-block; width: 2 }",
        "body",
        10,
        3,
    );
    assert_eq!(rows[..2], ["aa        ", "bbbody    "]);
}

/// `inline flow-root` is `inline-block`'s two-value spelling.
#[test]
fn an_inline_flow_root_pseudo_is_an_atom() {
    let (rows, _) = host(
        ".h::before { content: \"ab\"; display: inline flow-root; width: 4 }",
        "body",
        10,
        1,
    );
    assert_eq!(rows[0], "ab  body  ");
}

/// An `inline-flex` pseudo-element is an atom whose text is a flex item:
/// `justify-content: flex-end` packs it at the end of the box's 6 cells.
#[test]
fn an_inline_flex_pseudo_lays_its_content_out_as_flex() {
    let (rows, _) = host(
        ".h::before { content: \"ab\"; display: inline-flex; width: 6; \
         justify-content: flex-end }",
        "body",
        10,
        1,
    );
    assert_eq!(rows[0], "    abbody");
}

/// An `inline-grid` pseudo-element is an atom whose text is a grid item:
/// `justify-items: center` centres it in its 6-cell area.
#[test]
fn an_inline_grid_pseudo_lays_its_content_out_as_grid() {
    let (rows, _) = host(
        ".h::before { content: \"ab\"; display: inline-grid; width: 6; \
         justify-items: center }",
        "body",
        10,
        1,
    );
    assert_eq!(rows[0], "  ab  body");
}

/// A block-level `flex` pseudo-element is a flex container: its one item
/// centred by `justify-content` (free space 9, the leading half rounded
/// down, DIVERGENCES §1); `flex-direction: column` with `align-items:
/// center` centres it on the cross axis instead.
#[test]
fn a_flex_pseudo_is_a_flex_container() {
    let (rows, _) = host(
        ".h::before { content: \"T\"; display: flex; justify-content: center }",
        "body",
        10,
        2,
    );
    assert_eq!(rows, ["    T     ", "body      "]);
    let (rows, _) = host(
        ".h::before { content: \"T\"; display: flex; flex-direction: column; \
         align-items: center }",
        "body",
        10,
        2,
    );
    assert_eq!(rows, ["    T     ", "body      "]);
}

/// A block-level `grid` pseudo-element is a grid container: its item in
/// the first of two 3-cell columns wraps there — two rows, where a block
/// box would have held "ab cd" on one.
#[test]
fn a_grid_pseudo_is_a_grid_container() {
    let (rows, _) = host(
        ".h::before { content: \"ab cd\"; display: grid; grid-template-columns: 3 3 }",
        "body",
        10,
        3,
    );
    assert_eq!(rows, ["ab        ", "cd        ", "body      "]);
}

/// CSS 2.1 §9.5: a floated `::before` is a float at the start of its
/// host's content — the lines beside it shortened — with its own box.
#[test]
fn a_floated_before_floats() {
    let (rows, buf) = host(
        ".h::before { content: \"F\"; float: left; width: 2; height: 2; \
         background-color: rgb(200, 0, 0) }",
        "body text",
        10,
        3,
    );
    assert_eq!(rows[..2], ["F body    ", "  text    "]);
    for (x, y) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        assert_eq!(buf.cell(x, y).unwrap().bg, RED, "({x}, {y})");
    }
    assert_ne!(buf.cell(2, 0).unwrap().bg, RED);
}

/// A floated `::after` is met at the end of the content: §9.5.1 rule 6
/// places it on the current line when it fits there.
#[test]
fn a_floated_after_floats_at_the_end_of_the_content() {
    let (rows, _) = host(".h::after { content: \"R\"; float: right }", "body", 10, 1);
    assert_eq!(rows[0], "body     R");
}

/// A floated `::before` of a block container holding blocks floats
/// beside them: the first block's line starts past it.
#[test]
fn a_floated_pseudo_floats_beside_block_children() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let h = el(&mut dom, body, "div", "h");
    let p = el(&mut dom, h, "p", "");
    let t = dom.create_text_node("para");
    dom.append_child(p, t).unwrap();
    let buf = paint(
        &mut dom,
        ".h::before { content: \"F\"; float: left; width: 2 }",
        10,
        2,
    );
    assert_eq!(super::rows(&buf, 10, 2)[0], "F para    ");
}

/// `clear` past a floated pseudo-element: the cleared block goes below
/// it (§9.5.2).
#[test]
fn a_block_clears_a_floated_pseudo() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let h = el(&mut dom, body, "div", "h");
    let p = el(&mut dom, h, "p", "p");
    let t = dom.create_text_node("para");
    dom.append_child(p, t).unwrap();
    lay_out(
        &mut dom,
        ".h::before { content: \"F\"; float: left; width: 2; height: 3 } \
         .p { clear: left }",
        10,
        4,
    );
    assert_eq!(rect(&dom, p).y, 3);
}

/// Intrinsic sizes count the boxes: a flex item hosting an inline-block
/// `::before` (3 cells with its padding) and the text "x" is 4 wide; one
/// hosting a 2-wide float beside "x", 3.
#[test]
fn pseudo_atoms_and_floats_count_in_the_hosts_intrinsic_width() {
    for (css, width) in [
        (
            ".h::before { content: \"X\"; display: inline-block; padding: 0 1 }",
            4,
        ),
        (".h::before { content: \"F\"; float: left; width: 2 }", 3),
    ] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let row = el(&mut dom, root, "div", "row");
        let h = el(&mut dom, row, "div", "h");
        let t = dom.create_text_node("x");
        dom.append_child(h, t).unwrap();
        lay_out(
            &mut dom,
            &format!(".row {{ display: flex; align-items: flex-start }} {css}"),
            20,
            4,
        );
        assert_eq!(rect(&dom, h).width, width, "{css}");
    }
}

/// A pseudo-element is part of its host's box: a click on a floated
/// `::before` hanging below its one-row host targets the host; beside the
/// float, below the host, the click reaches `<body>`.
#[test]
fn a_click_on_a_floated_pseudo_targets_its_host() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let h = el(&mut dom, body, "div", "h");
    let t = dom.create_text_node("b");
    dom.append_child(h, t).unwrap();
    lay_out(
        &mut dom,
        ".h::before { content: \"F\"; float: left; width: 2; height: 3 }",
        10,
        4,
    );
    assert_eq!(rect(&dom, h).height, 1);
    assert_eq!(dom.hit_test(0, 2), Some(h));
    assert_eq!(dom.hit_test(5, 2), Some(body));
}

/// C9G-CLEARANCE-COST — a floated `::before`'s exclusion is the box it is
/// laid out in (CSS 2.1 §9.5): its height is measured and laid out by the
/// same packing of its content (a line height of 2 included), so the
/// host's text runs beside it for exactly its rows and below it after.
#[test]
fn a_floated_pseudos_exclusion_is_its_laid_out_box() {
    let (rows, _) = host(
        ".h { width: 6 } .h::before { content: 'a b'; float: left; width: 1; line-height: 2 }",
        "zz zz zz zz zz zz zz zz zz zz",
        6,
        6,
    );
    assert_eq!(
        rows,
        ["azz zz", " zz zz", "bzz zz", " zz zz", "zz zz ", "      "]
    );
}
