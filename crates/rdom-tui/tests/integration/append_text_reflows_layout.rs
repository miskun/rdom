//! Substrate regression: appending a text node to an empty
//! element must mark the parent dirty so the next cascade/layout
//! pass re-flows.
//!
//! Bug discovered while debugging the showcase status bar's
//! mouse-position slot: the slot started empty (intrinsic width
//! = 0 → flex gave it 0 cells), and when the mousemove listener
//! wrote "X: 42 Y: 7" into it, the text never appeared because
//! the layout wasn't re-flowed. The dirty tracker's
//! `ChildListChanged` handler iterates element-only siblings and
//! the added text node has no TuiExt, so nothing got marked
//! dirty.
//!
//! The fix: ChildListChanged additionally marks the parent
//! element dirty.

use rdom_tui::layout::{Direction, Flow, Size};
use rdom_tui::render::Rect;
use rdom_tui::style::DirtyTracker;
use rdom_tui::{CascadeExt, LayoutExt, Stylesheet, TuiDom, TuiNodeExt, TuiStyle};

#[test]
fn text_append_to_empty_flex_item_reflows_distribution() {
    // <parent display:flex>
    //   <left flex:1></left>      ← intrinsic 0, grow 1 → takes all space
    //   <right></right>           ← intrinsic 0 → width 0 initially
    // </parent>
    //
    // Then we append a text node "X: 42 Y: 7" to <right>. The
    // dirty tracker must mark the parent dirty so the next
    // cascade pass re-flows the flex distribution and gives
    // <right> a non-zero width.
    let mut dom: TuiDom = TuiDom::new();
    let _tracker = DirtyTracker::install(&mut dom);
    let root = dom.root();
    let parent = dom.create_element("div");
    dom.set_attribute(parent, "class", "parent").unwrap();
    dom.append_child(root, parent).unwrap();
    let left = dom.create_element("div");
    dom.set_attribute(left, "class", "left").unwrap();
    dom.append_child(parent, left).unwrap();
    let right = dom.create_element("div");
    dom.set_attribute(right, "class", "right").unwrap();
    dom.append_child(parent, right).unwrap();

    let sheet = Stylesheet::new()
        .rule_unchecked(
            ".parent",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .width(Size::Fixed(40))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked(".left", TuiStyle::new().width(Size::Flex(1)));

    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 1));

    let initial = dom.node(right).layout_rect().expect("right laid out");
    eprintln!("DBG initial right rect: {initial:?}");
    assert_eq!(
        initial.width, 0,
        "empty right slot starts at width 0 (flex distributes to left); got {initial:?}"
    );

    // Append text to the empty slot — this is the operation that
    // the mousemove listener does in production.
    let t = dom.create_text_node("X: 42 Y: 7");
    dom.append_child(right, t).unwrap();

    // Re-cascade. With the fix, the dirty tracker should have
    // marked `right`'s parent dirty (the flex container), so
    // `cascade_subtrees` re-cascades + the next layout reflows
    // the flex distribution.
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 1));

    let after = dom.node(right).layout_rect().expect("right laid out");
    eprintln!("DBG right rect after text append: {after:?}");
    assert!(
        after.width > 0,
        "after appending text to the empty slot, the flex container must \
         re-flow and give the slot a non-zero width; got {after:?}. \
         Without this, mutations on empty containers have no visible effect."
    );
}
