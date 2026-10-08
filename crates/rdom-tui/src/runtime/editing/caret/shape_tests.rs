//! C12-CARET — CSS UI 4 §6.2: `caret-shape` draws the painted caret
//! (rdom hides the hardware cursor and paints its own) as a whole cell
//! (`block`, and `auto`), an underline (`underscore`), or a one-eighth bar
//! (`bar`); `caret-animation: manual` stops the UA's blink.

use rdom_core::{NodeId, Position, Selection};

use crate::TuiDom;
use crate::render::{Buffer, Cell, LayoutExt, PaintExt, Rect};
use crate::style::{CascadeExt, Color, Modifier};

const RED: Color = Color::Rgb(255, 0, 0);

/// `<p contenteditable>ab</p>` focused, the caret at byte `at`, painted
/// under `css`.
fn paint(css: &str, at: usize, blink_off: bool) -> (Buffer, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "contenteditable", "true").unwrap();
    let t = dom.create_text_node("ab");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, at))));
    let sheet = rdom_css::from_css_strict(&format!(
        "p {{ display: block; width: 10; caret-color: red }} {css}"
    ))
    .unwrap();
    dom.cascade(&sheet);
    if blink_off {
        dom.node_mut(p).ext_mut().unwrap().caret_blink_off = true;
    }
    let area = Rect::new(0, 0, 12, 2);
    dom.layout_dom(area);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    (buf, p)
}

fn cell(css: &str, at: usize) -> Cell {
    paint(css, at, false).0.cell(at as u16, 0).unwrap().clone()
}

/// §6.2.2 `block` — and `auto`, which rdom draws as one: the caret
/// cell's background in `caret-color`.
#[test]
fn block_and_auto_fill_the_cell() {
    for css in ["", "p { caret-shape: block }"] {
        let c = cell(css, 0);
        assert_eq!((c.symbol(), c.bg), ("a", RED), "{css:?}");
        assert!(!c.modifier.contains(Modifier::UNDERLINED));
    }
}

/// `underscore`: the glyph and its background stay; the cell is
/// underlined in `caret-color` — on a blank cell (the line's end) too.
#[test]
fn underscore_underlines_the_cell() {
    for at in [0, 2] {
        let c = cell("p { caret-shape: underscore }", at);
        assert_ne!(c.bg, RED, "at {at}: not a block");
        assert!(c.modifier.contains(Modifier::UNDERLINED), "at {at}");
        assert_eq!(c.underline_color, RED, "at {at}");
    }
    assert_eq!(cell("p { caret-shape: underscore }", 0).symbol(), "a");
}

/// `bar`: a one-eighth bar (`▏`) in `caret-color` on a blank cell; over a
/// glyph, which a cell cannot show beside a bar, the underscore's
/// underline.
#[test]
fn bar_is_a_thin_bar_where_the_cell_is_blank() {
    let end = cell("p { caret-shape: bar }", 2);
    assert_eq!((end.symbol(), end.fg), ("▏", RED));
    assert_ne!(end.bg, RED);
    let over = cell("p { caret-shape: bar }", 0);
    assert_eq!(over.symbol(), "a");
    assert!(over.modifier.contains(Modifier::UNDERLINED));
    assert_eq!(over.underline_color, RED);
}

/// §6.2.1 `manual`: the UA does not blink the caret, so a blink phase
/// that is off still paints it; `auto` follows the phase.
#[test]
fn manual_animation_ignores_the_blink_phase() {
    let (buf, _) = paint("p { caret-animation: manual }", 0, true);
    assert_eq!(buf.cell(0, 0).unwrap().bg, RED);
    let (buf, _) = paint("", 0, true);
    assert_ne!(buf.cell(0, 0).unwrap().bg, RED);
}

/// §6.2.3: the `caret` shorthand sets the three longhands.
#[test]
fn the_caret_shorthand_sets_shape_and_animation() {
    let (buf, _) = paint("p { caret: manual underscore blue }", 0, true);
    let c = buf.cell(0, 0).unwrap();
    assert!(c.modifier.contains(Modifier::UNDERLINED));
    assert_eq!(c.underline_color, Color::Rgb(0, 0, 255));
}
