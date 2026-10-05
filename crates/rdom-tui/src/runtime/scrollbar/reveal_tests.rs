//! C8G-CARET-RTL (architect N8): the caret reveal reads the caret's real
//! cell, not one clamped onto the screen — a caret above the screen's top
//! row is scrolled to by its whole distance.

use rdom_core::{Position, Selection};

use crate::TuiDom;
use crate::accessors::{TuiAccessors, TuiAccessorsMut};
use crate::render::{LayoutExt, Rect};
use crate::style::CascadeExt;

/// A 2-row spacer, then a 3-row `<textarea>` of ten lines scrolled to
/// 5: its first line is at row 2 − 5 = −3, above the screen.
#[test]
fn a_caret_above_the_screen_is_revealed_by_its_whole_distance() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let spacer = dom.create_element("div");
    dom.set_attribute(spacer, "class", "sp").unwrap();
    dom.append_child(root, spacer).unwrap();
    let ta = dom.create_element("textarea");
    let t = dom.create_text_node("0\n1\n2\n3\n4\n5\n6\n7\n8\n9");
    dom.append_child(ta, t).unwrap();
    dom.append_child(root, ta).unwrap();
    let sheet = rdom_css::from_css_strict(
        ".sp { height: 2 } textarea { width: 6; height: 3; overflow-y: auto }",
    )
    .unwrap();
    dom.cascade(&sheet);
    let area = Rect::new(0, 0, 10, 8);
    dom.layout_dom(area);
    dom.node_mut(ta).set_scroll_top(5).unwrap();
    dom.layout_dom(area);
    dom.set_focused(Some(ta));
    dom.set_selection(Some(Selection::caret(Position::new(t, 0))));
    super::reveal_caret(&mut dom);
    assert_eq!(dom.node(ta).scroll_top(), Some(0));
}
