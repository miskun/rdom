//! Borders across paint layers: a later stacking context covers the
//! borders beneath it (CSS 2.1 Appendix E — stacking contexts paint
//! atomically, in order), and inside one stacking context borders of
//! unrelated boxes take the paint order, while a collapse group keeps
//! CSS Tables 3 §11.5 conflict resolution. See DIVERGENCES
//! "A stacking context covers the borders beneath it".

use crate::prelude::*;

// ── Helpers ──────────────────────────────────────────────────────

const RED: Color = Color::Rgb(255, 0, 0);
const BLUE: Color = Color::Rgb(0, 0, 255);
const WHITE: Color = Color::Rgb(255, 255, 255);

fn render(html: &str, css: &str, w: u16, h: u16) -> Buffer {
    let mut dom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, html, root).expect("markup parses");
    let sheet = rdom_css::from_css_strict(css).expect("css parses");
    let viewport = Rect::new(0, 0, w, h);
    dom.cascade(&sheet);
    dom.layout_dom(viewport);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);
    buf
}

fn row(buf: &Buffer, y: u16) -> String {
    (buf.area.x..buf.area.right())
        .filter_map(|x| buf.cell(x, y))
        .filter(|c| !c.is_spacer())
        .map(|c| c.symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// One letter per cell of row `y`: `R` / `B` / `W` for a glyph in red,
/// blue or white, `.` for a blank, `?` for any other color.
fn colors(buf: &Buffer, y: u16) -> String {
    (buf.area.x..buf.area.right())
        .filter_map(|x| buf.cell(x, y))
        .filter(|c| !c.is_spacer())
        .map(|c| match c.fg {
            _ if c.symbol() == " " => '.',
            f if f == RED => 'R',
            f if f == BLUE => 'B',
            f if f == WHITE => 'W',
            _ => '?',
        })
        .collect::<String>()
        .trim_end_matches('.')
        .to_string()
}

fn screen(buf: &Buffer) -> String {
    (buf.area.y..buf.area.bottom())
        .map(|y| format!("{:<24}{}", row(buf, y), colors(buf, y)))
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Tree guides paint in the tree's own layer ────────────────────

#[test]
fn a_higher_layer_covers_tree_guides() {
    // The guides are part of the tree's paint, so a fixed overlay above
    // the tree covers them like any other content of the tree.
    let buf = render(
        r#"<ul role="tree"><li role="treeitem" aria-expanded="true">root<ul role="group"><li role="treeitem">a</li><li role="treeitem">b</li><li role="treeitem">c</li></ul></li></ul><div class="ov">xxxxxxxx</div>"#,
        ".ov { position: fixed; left: 0; top: 1; width: 10; height: 2; background-color: blue; z-index: 100 }",
        14,
        5,
    );
    assert_eq!(row(&buf, 1), "xxxxxxxx", "{}", screen(&buf));
    assert_eq!(row(&buf, 2), "", "{}", screen(&buf));
    assert_eq!(row(&buf, 3), "└   c", "{}", screen(&buf));
}
