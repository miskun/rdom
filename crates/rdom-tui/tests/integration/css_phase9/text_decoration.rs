//! C9-DECORATION — CSS Text Decoration 3 / 4: the `text-decoration`
//! shorthand and its longhands drawn as the terminal's SGR decorations
//! (underline with its style and color, overline, line-through, blink),
//! propagated to descendants' text (§2.1); the placement properties parse
//! and are inert.

use super::{el, paint, rows, text_block};
use rdom_tui::prelude::*;

fn cell(buf: &Buffer, x: u16) -> &Cell {
    buf.cell(x, 0).expect("in the buffer")
}

const RED: Color = Color::Rgb(255, 0, 0);
const BLUE: Color = Color::Rgb(0, 0, 255);

/// §2.6: the shorthand's line, style and color reach the text: an
/// underline in its style and color, an overline, a line-through, blink.
#[test]
fn the_shorthand_draws_each_line_in_its_style_and_color() {
    let (mut dom, _, _) = text_block("ab");
    let css = ".b { color: blue; text-decoration: underline overline wavy red }";
    let buf = paint(&mut dom, css, 2, 1);
    let c = cell(&buf, 0);
    assert!(c.modifier.contains(Modifier::UNDERLINED), "{c:?}");
    assert!(c.modifier.contains(Modifier::UNDERLINE_CURLY), "{c:?}");
    assert!(c.modifier.contains(Modifier::OVERLINED), "{c:?}");
    assert!(!c.modifier.contains(Modifier::CROSSED_OUT), "{c:?}");
    assert_eq!((c.fg, c.underline_color), (BLUE, RED));

    for (style, bit) in [
        ("double", Modifier::UNDERLINE_DOUBLE),
        ("dotted", Modifier::UNDERLINE_DOTTED),
        ("dashed", Modifier::UNDERLINE_DASHED),
    ] {
        let (mut dom, _, _) = text_block("ab");
        let css =
            format!(".b {{ text-decoration-line: underline; text-decoration-style: {style} }}");
        let buf = paint(&mut dom, &css, 2, 1);
        assert!(cell(&buf, 1).modifier.contains(bit), "{style}");
    }
    let (mut dom, _, _) = text_block("ab");
    let buf = paint(&mut dom, ".b { text-decoration: line-through blink }", 2, 1);
    let c = cell(&buf, 0);
    assert!(
        c.modifier
            .contains(Modifier::CROSSED_OUT | Modifier::SLOW_BLINK),
        "{c:?}"
    );
    assert!(!c.modifier.contains(Modifier::UNDERLINED), "{c:?}");
}

/// §2.1: "decorations are propagated to all in-flow children" and "drawn
/// with the decorating box's color" — the paragraph's red underline under
/// its blue span's text, `text-decoration-color`'s initial `currentcolor`
/// the paragraph's color; a
/// child's own decoration adds to it (and `none` removes nothing), but
/// "neither propagation nor text decorations affect ... atomic
/// inline-level descendants such as inline blocks" or out-of-flow boxes.
#[test]
fn decorations_propagate_to_in_flow_descendants_only() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    for (tag, class, text) in [
        ("span", "s", "a"),
        ("span", "t", "b"),
        ("span", "n", "c"),
        ("span", "ib", "d"),
        ("span", "f", "e"),
    ] {
        let id = el(&mut dom, p, tag, class);
        let t = dom.create_text_node(text);
        dom.append_child(id, t).unwrap();
    }
    let css = ".p { width: 8; color: red; text-decoration-line: underline } \
               .s { color: blue } \
               .t { text-decoration: line-through } \
               .n { text-decoration: none } \
               .ib { display: inline-block } \
               .f { float: right }";
    let buf = paint(&mut dom, css, 8, 1);
    assert_eq!(rows(&buf, 8, 1), ["abcd   e"]);
    let a = cell(&buf, 0);
    assert!(a.modifier.contains(Modifier::UNDERLINED));
    assert_eq!((a.fg, a.underline_color), (BLUE, RED));
    let b = cell(&buf, 1);
    assert!(
        b.modifier
            .contains(Modifier::UNDERLINED | Modifier::CROSSED_OUT)
    );
    assert!(cell(&buf, 2).modifier.contains(Modifier::UNDERLINED));
    assert!(!cell(&buf, 3).modifier.contains(Modifier::UNDERLINED));
    assert!(!cell(&buf, 7).modifier.contains(Modifier::UNDERLINED));
}

/// §4.1, §4.2, §2.5, §3.2: the placement properties parse (a strict sheet
/// takes them) and change nothing on a grid.
#[test]
fn the_placement_properties_parse_and_are_inert() {
    let (mut dom, _, _) = text_block("ab");
    let css = ".b { text-decoration: underline 2px; text-decoration-thickness: from-font; \
               text-underline-offset: 0.2em; text-underline-position: under left; \
               text-decoration-skip-ink: none }";
    let buf = paint(&mut dom, css, 2, 1);
    let c = cell(&buf, 0);
    assert!(c.modifier.contains(Modifier::UNDERLINED));
    assert_eq!(
        c.underline_color,
        Color::Reset,
        "currentcolor is the text's"
    );
}
