//! Containing-block resolution.

use super::*;
use crate::layout::{Length, ZIndex};
use crate::render::rect::Rect;
use crate::style::Value;
use crate::style::{ComputedStyle, Stylesheet, TuiStyle};
use crate::{CascadeExt, LayoutExt, TuiDom};

fn build_dom_with_positioned_chain(positions: &[Position]) -> (TuiDom, Vec<NodeId>) {
    // Build a vertical chain root → child[0] → child[1] → ...
    // with the requested computed `position` on each child.
    let mut dom: TuiDom = TuiDom::new();
    let mut ids = Vec::with_capacity(positions.len());
    let root = dom.root();
    let mut parent = root;
    for (i, _p) in positions.iter().enumerate() {
        let id = dom.create_element("div");
        dom.node_mut(id).set_id(&format!("n{i}")).unwrap();
        dom.append_child(parent, id).unwrap();
        ids.push(id);
        parent = id;
    }

    // Author rules: `#nN { position: <p>; }` for each.
    let mut sheet = Stylesheet::bare();
    for (i, p) in positions.iter().enumerate() {
        sheet = sheet.rule_unchecked(&format!("#n{i}"), TuiStyle::new().position(*p));
    }
    dom.cascade(&sheet);
    // Run layout to populate rects.
    let viewport = Rect::new(0, 0, 100, 50);
    dom.layout_dom(viewport);
    (dom, ids)
}

fn viewport() -> LayoutRect {
    LayoutRect::new(0, 0, 100, 50)
}

#[test]
fn absolute_with_no_positioned_ancestor_returns_viewport() {
    let (dom, ids) =
        build_dom_with_positioned_chain(&[Position::Static, Position::Static, Position::Absolute]);
    let cb = containing_block(&dom, ids[2], viewport());
    assert_eq!(cb, viewport());
}

#[test]
fn absolute_inside_relative_uses_relative_parent() {
    let (dom, ids) = build_dom_with_positioned_chain(&[
        Position::Static,
        Position::Relative,
        Position::Absolute,
    ]);
    let parent_rect = dom.node(ids[1]).ext().unwrap().layout;
    let cb = containing_block(&dom, ids[2], viewport());
    assert_eq!(cb, parent_rect);
}

#[test]
fn absolute_skips_static_ancestors_to_find_relative() {
    let (dom, ids) = build_dom_with_positioned_chain(&[
        Position::Relative, // grandparent
        Position::Static,   // parent (skipped)
        Position::Absolute, // self
    ]);
    let grandparent_rect = dom.node(ids[0]).ext().unwrap().layout;
    let cb = containing_block(&dom, ids[2], viewport());
    assert_eq!(cb, grandparent_rect);
}

#[test]
fn absolute_inside_absolute_uses_absolute_parent() {
    let (dom, ids) = build_dom_with_positioned_chain(&[
        Position::Static,
        Position::Absolute,
        Position::Absolute,
    ]);
    let parent_rect = dom.node(ids[1]).ext().unwrap().layout;
    let cb = containing_block(&dom, ids[2], viewport());
    assert_eq!(cb, parent_rect);
}

#[test]
fn fixed_always_uses_viewport_even_with_relative_ancestor() {
    let (dom, ids) =
        build_dom_with_positioned_chain(&[Position::Static, Position::Relative, Position::Fixed]);
    let cb = containing_block(&dom, ids[2], viewport());
    // Fixed ignores ancestors; viewport always wins.
    assert_eq!(cb, viewport());
}

#[test]
fn fixed_uses_viewport_when_no_ancestors_positioned() {
    let (dom, ids) =
        build_dom_with_positioned_chain(&[Position::Static, Position::Static, Position::Fixed]);
    let cb = containing_block(&dom, ids[2], viewport());
    assert_eq!(cb, viewport());
}

#[test]
fn static_returns_parent_layout() {
    let (dom, ids) =
        build_dom_with_positioned_chain(&[Position::Static, Position::Static, Position::Static]);
    let parent_rect = dom.node(ids[1]).ext().unwrap().layout;
    let cb = containing_block(&dom, ids[2], viewport());
    assert_eq!(cb, parent_rect);
}

/// Document existence; not a behavioral assertion. M2 callers
/// invoke `containing_block` only on absolute/fixed; static
/// behavior is documented for completeness.
#[allow(dead_code)]
fn _types_compile() {
    let _: ComputedStyle = ComputedStyle::initial();
    let _: Value<Length> = Value::Specified(Length::Auto);
    let _: ZIndex = ZIndex::Auto;
}
