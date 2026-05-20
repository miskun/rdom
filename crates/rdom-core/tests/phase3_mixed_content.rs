//! Phase 3 acceptance — mixed Text + Element + Text siblings round-trip
//! through the tree and every public reader sees them correctly.

use rdom_core::Dom;

/// Build `<div>hello <strong>world</strong>!</div>` and verify:
/// - Three direct children (text, element, text)
/// - text_content concatenates everything
/// - comments are skipped in text_content
/// - inner_markup / outer_markup re-serialize the tree
/// - node_value / data alias work on the Text nodes
#[test]
fn mixed_content_round_trips() {
    let mut dom: Dom = Dom::new();
    let div = dom.create_element("div");
    let t1 = dom.create_text_node("hello ");
    let strong = dom.create_element("strong");
    let tw = dom.create_text_node("world");
    let comment = dom.create_comment(" side note ");
    let t2 = dom.create_text_node("!");

    dom.append_child(strong, tw).unwrap();
    dom.append_child(div, t1).unwrap();
    dom.append_child(div, strong).unwrap();
    dom.append_child(div, comment).unwrap(); // between strong and t2
    dom.append_child(div, t2).unwrap();

    // Four direct children (text, element, comment, text).
    let children: Vec<_> = dom.node(div).child_nodes().map(|n| n.id()).collect();
    assert_eq!(children.len(), 4);

    // text_content skips comments, concatenates text.
    assert_eq!(dom.text_content(div), "hello world!");
    assert_eq!(dom.node(div).text_content(), "hello world!");

    // data() alias works on Text/Comment, returns None on Element.
    assert_eq!(dom.node(t1).data(), Some("hello "));
    assert_eq!(dom.node(comment).data(), Some(" side note "));
    assert_eq!(dom.node(strong).data(), None);

    // inner_markup includes the comment as part of serialization.
    assert_eq!(
        dom.inner_markup(div),
        "hello <strong>world</strong><!-- side note -->!"
    );
    // outer_markup wraps the div.
    assert_eq!(
        dom.outer_markup(div),
        "<div>hello <strong>world</strong><!-- side note -->!</div>"
    );
}

#[test]
fn fragment_insert_unwraps_mixed_content() {
    let mut dom: Dom = Dom::new();
    let div = dom.create_element("div");

    // Fragment holds [text, element, text].
    let frag = dom.create_document_fragment();
    let a = dom.create_text_node("A ");
    let b = dom.create_element("span");
    let bt = dom.create_text_node("B");
    dom.append_child(b, bt).unwrap();
    let c = dom.create_text_node(" C");
    dom.append_child(frag, a).unwrap();
    dom.append_child(frag, b).unwrap();
    dom.append_child(frag, c).unwrap();

    dom.append_child(div, frag).unwrap();

    // Fragment is empty after unwrap.
    assert!(!dom.node(frag).has_child_nodes());
    // Div has three children, text_content sees all.
    assert_eq!(dom.node(div).child_nodes().count(), 3);
    assert_eq!(dom.text_content(div), "A B C");
}

#[test]
fn set_text_content_via_node_mut_replaces_mixed_children() {
    let mut dom: Dom = Dom::new();
    let div = dom.create_element("div");
    let t = dom.create_text_node("before");
    let span = dom.create_element("span");
    dom.append_child(div, t).unwrap();
    dom.append_child(div, span).unwrap();

    dom.node_mut(div).set_text_content("after").unwrap();
    assert_eq!(dom.text_content(div), "after");
    // Original children dropped; div now has exactly one Text child.
    assert_eq!(dom.node(div).child_nodes().count(), 1);
}

#[test]
fn set_data_alias_updates_text() {
    let mut dom: Dom = Dom::new();
    let t = dom.create_text_node("before");
    dom.node_mut(t).set_data("after").unwrap();
    assert_eq!(dom.node(t).data(), Some("after"));
}
