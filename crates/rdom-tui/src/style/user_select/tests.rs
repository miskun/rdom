//! Used-value resolution of `user-select` (CSS UI 4 §6.1): the
//! property is not inherited; `auto` resolves against the parent's
//! used value, and `contain` never propagates.

use rdom_core::NodeId;

use super::{all_host, contain_host, is_unselectable, used_value};
use crate::TuiDom;
use crate::layout::UserSelect;
use crate::style::{CascadeExt, Stylesheet, TuiStyle};

/// `root > outer > inner > text`, each element tagged by name.
fn chain(outer: &str, inner: &str) -> (TuiDom, [NodeId; 3]) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let o = dom.create_element(outer);
    let i = dom.create_element(inner);
    let t = dom.create_text_node("text");
    dom.append_child(i, t).unwrap();
    dom.append_child(o, i).unwrap();
    dom.append_child(root, o).unwrap();
    (dom, [o, i, t])
}

fn sheet(rules: &[(&str, UserSelect)]) -> Stylesheet {
    rules.iter().fold(Stylesheet::bare(), |s, (sel, v)| {
        s.rule_unchecked(sel, TuiStyle::new().user_select(*v))
    })
}

#[test]
fn auto_everywhere_resolves_text() {
    let (mut dom, [o, i, t]) = chain("div", "p");
    dom.cascade(&Stylesheet::bare());
    assert_eq!(used_value(&dom, o), UserSelect::Text);
    assert_eq!(used_value(&dom, i), UserSelect::Text);
    assert_eq!(used_value(&dom, t), UserSelect::Text);
    assert!(!is_unselectable(&dom, t));
}

#[test]
fn none_propagates_to_auto_descendants_through_the_used_value() {
    let (mut dom, [o, i, t]) = chain("div", "p");
    dom.cascade(&sheet(&[("div", UserSelect::None)]));
    assert_eq!(used_value(&dom, o), UserSelect::None);
    assert_eq!(used_value(&dom, i), UserSelect::None);
    assert!(is_unselectable(&dom, t));
}

#[test]
fn explicit_text_child_of_a_none_parent_is_selectable() {
    let (mut dom, [o, i, t]) = chain("div", "span");
    dom.cascade(&sheet(&[
        ("div", UserSelect::None),
        ("span", UserSelect::Text),
    ]));
    assert!(is_unselectable(&dom, o));
    assert_eq!(used_value(&dom, i), UserSelect::Text);
    assert!(
        !is_unselectable(&dom, t),
        "`text` stops the propagation of `none`"
    );
}

#[test]
fn auto_child_of_a_contain_host_resolves_text() {
    let (mut dom, [o, i, _]) = chain("div", "p");
    dom.cascade(&sheet(&[("div", UserSelect::Contain)]));
    assert_eq!(used_value(&dom, o), UserSelect::Contain);
    assert_eq!(
        used_value(&dom, i),
        UserSelect::Text,
        "`contain` never propagates"
    );
}

#[test]
fn nested_contain_host_is_the_inner_declaring_element() {
    // `.panel{contain} > .card{contain} > text`: the card is its own host.
    let (mut dom, [_panel, card, t]) = chain("panel", "card");
    dom.cascade(&sheet(&[
        ("panel", UserSelect::Contain),
        ("card", UserSelect::Contain),
    ]));
    assert_eq!(contain_host(&dom, t), Some(card));
}

#[test]
fn contain_host_is_the_declaring_ancestor_not_an_auto_descendant() {
    let (mut dom, [panel, _p, t]) = chain("panel", "p");
    dom.cascade(&sheet(&[("panel", UserSelect::Contain)]));
    assert_eq!(contain_host(&dom, t), Some(panel));
}

#[test]
fn an_auto_editing_host_is_a_contain_host() {
    let (mut dom, [ed, _p, t]) = chain("div", "p");
    dom.set_attribute(ed, "contenteditable", "true").unwrap();
    dom.cascade(&Stylesheet::bare());
    assert_eq!(used_value(&dom, ed), UserSelect::Contain);
    assert_eq!(contain_host(&dom, t), Some(ed));
}

#[test]
fn all_host_is_the_outermost_all_element_of_the_chain() {
    let (mut dom, [o, _i, t]) = chain("div", "p");
    dom.cascade(&sheet(&[("div", UserSelect::All)]));
    assert_eq!(all_host(&dom, t), Some(o));

    let (mut dom, [o, _i, t]) = chain("div", "p");
    dom.cascade(&sheet(&[("div", UserSelect::All), ("p", UserSelect::All)]));
    assert_eq!(
        all_host(&dom, t),
        Some(o),
        "nested `all` joins the outer host"
    );
}

/// CSS UI 4 §6.1 `all`: "if a selection would contain part of the
/// element, then the selection must contain the entire element" — an
/// explicit `text` descendant is still part of its `all` ancestor.
#[test]
fn explicit_text_inside_an_all_host_stays_inside_the_host() {
    let (mut dom, [o, i, t]) = chain("div", "span");
    dom.cascade(&sheet(&[
        ("div", UserSelect::All),
        ("span", UserSelect::Text),
    ]));
    assert_eq!(used_value(&dom, i), UserSelect::Text);
    assert_eq!(all_host(&dom, t), Some(o));
}
