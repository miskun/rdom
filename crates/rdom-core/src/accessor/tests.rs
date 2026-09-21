use super::*;

fn build(dom: &mut Dom) -> (NodeId, NodeId, NodeId, NodeId) {
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    let c = dom.create_element("c");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    dom.append_child(root, c).unwrap();
    (root, a, b, c)
}

#[test]
fn dom_accessor_returns_owning_dom() {
    let mut dom: Dom = Dom::new();
    let (root, a, _b, _c) = build(&mut dom);
    let node = dom.node(a);
    // dom() returns the same arena `node()` was built from —
    // tree shape observed through it matches.
    let via_dom = node.dom();
    assert_eq!(via_dom.root(), root);
    assert_eq!(via_dom.node(a).parent_node().map(|p| p.id()), Some(root));
}

#[test]
fn navigation_via_noderef() {
    let mut dom: Dom = Dom::new();
    let (root, a, b, c) = build(&mut dom);

    let r = dom.node(root);
    assert_eq!(r.first_child().unwrap().id(), a);
    assert_eq!(r.last_child().unwrap().id(), c);
    assert!(r.has_child_nodes());

    let br = dom.node(b);
    assert_eq!(br.previous_sibling().unwrap().id(), a);
    assert_eq!(br.next_sibling().unwrap().id(), c);
    assert_eq!(br.parent_node().unwrap().id(), root);
}

#[test]
fn child_nodes_iterator_yields_all_in_order() {
    let mut dom: Dom = Dom::new();
    let (root, a, b, c) = build(&mut dom);
    let ids: Vec<NodeId> = dom.node(root).child_nodes().map(|n| n.id()).collect();
    assert_eq!(ids, vec![a, b, c]);
}

#[test]
fn element_child_iter_skips_text_and_comment() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let t = dom.create_text_node("hi");
    let c = dom.create_comment("note");
    let b = dom.create_element("b");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, t).unwrap();
    dom.append_child(root, c).unwrap();
    dom.append_child(root, b).unwrap();

    let ids: Vec<NodeId> = dom.node(root).children().map(|n| n.id()).collect();
    assert_eq!(ids, vec![a, b]);

    assert_eq!(dom.node(root).first_element_child().unwrap().id(), a);
    assert_eq!(dom.node(root).last_element_child().unwrap().id(), b);
    assert_eq!(dom.node(root).child_element_count(), 2);
}

#[test]
fn node_name_matches_spec() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    let t = dom.create_text_node("hi");
    let cm = dom.create_comment("note");
    let frag = dom.create_document_fragment();
    assert_eq!(dom.node(el).node_name(), "div");
    assert_eq!(dom.node(t).node_name(), "#text");
    assert_eq!(dom.node(cm).node_name(), "#comment");
    assert_eq!(dom.node(frag).node_name(), "#document-fragment");
}

#[test]
fn node_value_for_text_and_comment_only() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    let t = dom.create_text_node("hi");
    let cm = dom.create_comment("note");
    assert_eq!(dom.node(t).node_value(), Some("hi"));
    assert_eq!(dom.node(cm).node_value(), Some("note"));
    assert_eq!(dom.node(el).node_value(), None);
}

#[test]
fn set_node_value_updates_text() {
    let mut dom: Dom = Dom::new();
    let t = dom.create_text_node("old");
    dom.node_mut(t).set_node_value("new").unwrap();
    assert_eq!(dom.node(t).node_value(), Some("new"));
}

#[test]
fn set_node_value_errors_on_element() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    assert!(matches!(
        dom.node_mut(el).set_node_value("x").unwrap_err(),
        DomError::WrongNodeType { .. }
    ));
}

#[test]
fn contains_includes_self_and_descendants() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(root, a).unwrap();
    dom.append_child(a, b).unwrap();

    assert!(dom.node(root).contains(a));
    assert!(dom.node(root).contains(b));
    assert!(dom.node(a).contains(b));
    assert!(!dom.node(b).contains(a));
}

#[test]
fn noderef_mutation_via_nodemut() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    dom.node_mut(el).set_attribute("class", "hero").unwrap();
    dom.node_mut(el).add_class("active").unwrap();
    // Per WHATWG DOM, classList changes round-trip through
    // `get_attribute("class")`. Tokens iterate alphabetically
    // (a documented divergence — see
    // `crate::token_list::DomTokenList`), so after
    // `add_class("active")` on top of `set_attribute("class",
    // "hero")` the attribute reads "active hero".
    assert_eq!(dom.node(el).get_attribute("class"), Some("active hero"));
    assert!(dom.node(el).has_class("active"));
    assert!(dom.node(el).has_class("hero"));
}

// ── M4b step 14: NodeRef accessor additions ───────────────────────

#[test]
fn is_connected_true_for_root_and_attached_nodes() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    dom.append_child(root, el).unwrap();
    assert!(dom.node(root).is_connected());
    assert!(dom.node(el).is_connected());
}

#[test]
fn is_connected_false_for_detached_subtree() {
    let mut dom: Dom = Dom::new();
    let detached = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(detached, child).unwrap();
    assert!(!dom.node(detached).is_connected());
    assert!(!dom.node(child).is_connected());
}

#[test]
fn get_root_node_returns_doc_root_for_connected() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    let leaf = dom.create_element("span");
    dom.append_child(el, leaf).unwrap();
    dom.append_child(root, el).unwrap();
    assert_eq!(dom.node(leaf).get_root_node().id(), root);
    assert_eq!(dom.node(root).get_root_node().id(), root);
}

#[test]
fn get_root_node_returns_detached_subtree_root() {
    let mut dom: Dom = Dom::new();
    let outer = dom.create_element("div");
    let inner = dom.create_element("span");
    dom.append_child(outer, inner).unwrap();
    assert_eq!(dom.node(inner).get_root_node().id(), outer);
    assert_eq!(dom.node(outer).get_root_node().id(), outer);
}

#[test]
fn class_name_returns_raw_attribute_or_empty_string() {
    let mut dom: Dom = Dom::new();
    let bare = dom.create_element("div");
    let styled = dom.create_element("div");
    dom.node_mut(styled)
        .set_attribute("class", "hero  active")
        .unwrap();
    assert_eq!(dom.node(bare).class_name(), "");
    assert_eq!(dom.node(styled).class_name(), "hero  active");
}

#[test]
fn class_list_returns_dom_token_list_snapshot() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    dom.node_mut(el).add_class("foo").unwrap();
    dom.node_mut(el).add_class("bar").unwrap();
    let list: DomTokenList = dom.node(el).class_list();
    assert_eq!(list.len(), 2);
    assert!(list.contains("foo"));
    assert!(list.contains("bar"));
    assert!(!list.contains("baz"));
    let collected: Vec<&str> = list.iter().collect();
    assert_eq!(collected, ["bar", "foo"]); // BTreeSet ordering (documented divergence)
}

#[test]
fn matches_simple_selectors() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("button");
    dom.node_mut(el).set_id("go").unwrap();
    dom.node_mut(el).add_class("primary").unwrap();
    assert!(dom.node(el).matches("button"));
    assert!(dom.node(el).matches("#go"));
    assert!(dom.node(el).matches(".primary"));
    assert!(dom.node(el).matches("button.primary#go"));
    assert!(!dom.node(el).matches("div"));
}

#[test]
fn matches_returns_false_on_invalid_selector() {
    // Spec divergence: browser throws SyntaxError; rdom swallows
    // and returns false. Documented on the method.
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    assert!(!dom.node(el).matches("!!!not a selector"));
}

#[test]
fn closest_returns_self_when_self_matches() {
    let mut dom: Dom = Dom::new();
    let outer = dom.create_element("section");
    let inner = dom.create_element("div");
    dom.node_mut(inner).add_class("target").unwrap();
    dom.append_child(outer, inner).unwrap();
    let hit = dom.node(inner).closest(".target").unwrap();
    assert_eq!(hit.id(), inner);
}

#[test]
fn closest_walks_up_ancestors() {
    let mut dom: Dom = Dom::new();
    let form = dom.create_element("form");
    let label = dom.create_element("label");
    let input = dom.create_element("input");
    dom.append_child(label, input).unwrap();
    dom.append_child(form, label).unwrap();
    let hit = dom.node(input).closest("form").unwrap();
    assert_eq!(hit.id(), form);
}

#[test]
fn closest_returns_none_in_detached_subtree_with_no_match() {
    let mut dom: Dom = Dom::new();
    let detached = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(detached, child).unwrap();
    // No ancestor matches "form" and the subtree is detached.
    assert!(dom.node(child).closest("form").is_none());
}

#[test]
fn closest_returns_none_on_invalid_selector() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    assert!(dom.node(el).closest("!!!").is_none());
}

#[test]
fn query_selector_finds_first_descendant() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let section = dom.create_element("section");
    let a = dom.create_element("p");
    dom.node_mut(a).add_class("hit").unwrap();
    let b = dom.create_element("p");
    dom.node_mut(b).add_class("hit").unwrap();
    dom.append_child(section, a).unwrap();
    dom.append_child(section, b).unwrap();
    dom.append_child(root, section).unwrap();
    let hit = dom.node(section).query_selector(".hit").unwrap();
    assert_eq!(hit.id(), a);
}

#[test]
fn query_selector_returns_none_when_no_descendant_matches() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    assert!(dom.node(el).query_selector(".missing").is_none());
}

#[test]
fn query_selector_excludes_self_per_spec() {
    // DOM spec: querySelector is element-rooted but searches
    // descendants only. The subject element itself is not a
    // candidate.
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    dom.node_mut(el).add_class("foo").unwrap();
    assert!(dom.node(el).query_selector(".foo").is_none());
}

#[test]
fn query_selector_all_returns_node_list_in_document_order() {
    let mut dom: Dom = Dom::new();
    let root = dom.create_element("section");
    let a = dom.create_element("p");
    let b = dom.create_element("p");
    let c = dom.create_element("p");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    dom.append_child(root, c).unwrap();
    let list = dom.node(root).query_selector_all("p");
    assert_eq!(list.len(), 3);
    let ids: Vec<NodeId> = list.iter().map(|n| n.id()).collect();
    assert_eq!(ids, vec![a, b, c]);
}

#[test]
fn query_selector_all_returns_empty_on_invalid_selector() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    let list = dom.node(el).query_selector_all("!!!");
    assert_eq!(list.len(), 0);
}

// ── M4b step 15: NodeMut accessor additions ───────────────────────

#[test]
fn set_class_name_replaces_full_class_attribute() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    dom.node_mut(el).add_class("old").unwrap();
    dom.node_mut(el).set_class_name("hero  active").unwrap();
    assert_eq!(dom.node(el).class_name(), "hero  active");
    // classList is rebuilt: old is gone, both new tokens present.
    let list = dom.node(el).class_list();
    assert!(!list.contains("old"));
    assert!(list.contains("hero"));
    assert!(list.contains("active"));
    assert_eq!(list.len(), 2);
}

#[test]
fn set_class_name_empty_clears_classes() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    dom.node_mut(el).add_class("foo").unwrap();
    dom.node_mut(el).set_class_name("").unwrap();
    assert_eq!(dom.node(el).class_name(), "");
    assert_eq!(dom.node(el).class_list().len(), 0);
}

#[test]
fn class_list_mut_returns_mutating_handle() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    {
        let mut nm = dom.node_mut(el);
        let mut list = nm.class_list_mut();
        list.add("foo").unwrap();
        list.toggle("bar", Some(true)).unwrap();
    }
    assert!(dom.node(el).has_class("foo"));
    assert!(dom.node(el).has_class("bar"));
}

#[test]
fn toggle_attribute_force_true_is_force_add() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("input");
    // Absent: force-add adds with empty string, returns true.
    assert!(
        dom.node_mut(el)
            .toggle_attribute_force("disabled", Some(true))
            .unwrap()
    );
    assert!(dom.node(el).has_attribute("disabled"));
    // Already present: idempotent, returns true, no overwrite.
    dom.node_mut(el).set_attribute("disabled", "1").unwrap();
    assert!(
        dom.node_mut(el)
            .toggle_attribute_force("disabled", Some(true))
            .unwrap()
    );
    assert_eq!(dom.node(el).get_attribute("disabled"), Some("1"));
}

#[test]
fn toggle_attribute_force_false_is_force_remove() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("input");
    dom.node_mut(el).set_attribute("disabled", "").unwrap();
    // Present: force-remove returns false, attribute gone.
    assert!(
        !dom.node_mut(el)
            .toggle_attribute_force("disabled", Some(false))
            .unwrap()
    );
    assert!(!dom.node(el).has_attribute("disabled"));
    // Already absent: idempotent, returns false.
    assert!(
        !dom.node_mut(el)
            .toggle_attribute_force("disabled", Some(false))
            .unwrap()
    );
}

#[test]
fn toggle_attribute_force_none_flips_state() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("input");
    // Absent → present.
    assert!(
        dom.node_mut(el)
            .toggle_attribute_force("disabled", None)
            .unwrap()
    );
    assert!(dom.node(el).has_attribute("disabled"));
    // Present → absent.
    assert!(
        !dom.node_mut(el)
            .toggle_attribute_force("disabled", None)
            .unwrap()
    );
    assert!(!dom.node(el).has_attribute("disabled"));
}

#[test]
fn toggle_attribute_force_errors_on_non_element() {
    let mut dom: Dom = Dom::new();
    let t = dom.create_text_node("hi");
    assert!(
        dom.node_mut(t)
            .toggle_attribute_force("disabled", Some(true))
            .is_err()
    );
}

// ── M4b step 16: variadic tree helpers ────────────────────────────

use crate::NodeOrString;

fn child_ids<Ext: 'static>(dom: &Dom<Ext>, parent: NodeId) -> Vec<NodeId> {
    dom.node(parent).child_nodes().map(|n| n.id()).collect()
}

#[test]
fn append_variadic_inserts_in_order_with_text_coercion() {
    let mut dom: Dom = Dom::new();
    let parent = dom.create_element("p");
    let strong = dom.create_element("strong");
    let em = dom.create_element("em");
    dom.node_mut(parent)
        .append([
            "hello ".into(),
            strong.into(),
            " mid ".into(),
            em.into(),
            NodeOrString::Text("!".into()),
        ])
        .unwrap();
    let ids = child_ids(&dom, parent);
    assert_eq!(ids.len(), 5);
    // Middle node-typed slots are preserved by id.
    assert_eq!(ids[1], strong);
    assert_eq!(ids[3], em);
    // Text slots produced fresh text nodes.
    assert_eq!(dom.node(ids[0]).node_value(), Some("hello "));
    assert_eq!(dom.node(ids[2]).node_value(), Some(" mid "));
    assert_eq!(dom.node(ids[4]).node_value(), Some("!"));
}

#[test]
fn prepend_variadic_inserts_in_order_before_existing_children() {
    let mut dom: Dom = Dom::new();
    let parent = dom.create_element("ul");
    let existing = dom.create_element("li");
    dom.append_child(parent, existing).unwrap();
    let a = dom.create_element("li");
    let b = dom.create_element("li");
    dom.node_mut(parent)
        .prepend([a.into(), "x".into(), b.into()])
        .unwrap();
    let ids = child_ids(&dom, parent);
    assert_eq!(ids.len(), 4);
    assert_eq!(ids[0], a);
    assert_eq!(dom.node(ids[1]).node_value(), Some("x"));
    assert_eq!(ids[2], b);
    assert_eq!(ids[3], existing);
}

#[test]
fn before_inserts_siblings_in_order_preceding_self() {
    let mut dom: Dom = Dom::new();
    let parent = dom.create_element("div");
    let target = dom.create_element("span");
    let after = dom.create_element("span");
    dom.append_child(parent, target).unwrap();
    dom.append_child(parent, after).unwrap();
    let a = dom.create_element("p");
    let b = dom.create_element("p");
    dom.node_mut(target)
        .before([a.into(), "hi".into(), b.into()])
        .unwrap();
    let ids = child_ids(&dom, parent);
    assert_eq!(ids.len(), 5);
    assert_eq!(ids[0], a);
    assert_eq!(dom.node(ids[1]).node_value(), Some("hi"));
    assert_eq!(ids[2], b);
    assert_eq!(ids[3], target);
    assert_eq!(ids[4], after);
}

#[test]
fn after_inserts_siblings_in_order_following_self() {
    let mut dom: Dom = Dom::new();
    let parent = dom.create_element("div");
    let before = dom.create_element("span");
    let target = dom.create_element("span");
    let tail = dom.create_element("span");
    dom.append_child(parent, before).unwrap();
    dom.append_child(parent, target).unwrap();
    dom.append_child(parent, tail).unwrap();
    let a = dom.create_element("p");
    let b = dom.create_element("p");
    dom.node_mut(target)
        .after([a.into(), "hi".into(), b.into()])
        .unwrap();
    let ids = child_ids(&dom, parent);
    assert_eq!(ids.len(), 6);
    assert_eq!(ids[0], before);
    assert_eq!(ids[1], target);
    assert_eq!(ids[2], a);
    assert_eq!(dom.node(ids[3]).node_value(), Some("hi"));
    assert_eq!(ids[4], b);
    assert_eq!(ids[5], tail);
}

#[test]
fn before_after_silently_noop_on_parentless_node() {
    let mut dom: Dom = Dom::new();
    let detached = dom.create_element("div");
    let extra = dom.create_element("p");
    // Detached has no parent: before/after silently no-op.
    dom.node_mut(detached).before([extra.into()]).unwrap();
    dom.node_mut(detached).after([extra.into()]).unwrap();
    // Nothing inserted around detached; it's still parentless.
    assert!(dom.node(detached).parent_node().is_none());
}

#[test]
fn replace_children_clears_existing_and_appends_new() {
    let mut dom: Dom = Dom::new();
    let parent = dom.create_element("div");
    let old_a = dom.create_element("a");
    let old_b = dom.create_element("b");
    dom.append_child(parent, old_a).unwrap();
    dom.append_child(parent, old_b).unwrap();
    let new_a = dom.create_element("i");
    let new_b = dom.create_element("u");
    dom.node_mut(parent)
        .replace_children([new_a.into(), "mid".into(), new_b.into()])
        .unwrap();
    let ids = child_ids(&dom, parent);
    assert_eq!(ids.len(), 3);
    assert_eq!(ids[0], new_a);
    assert_eq!(dom.node(ids[1]).node_value(), Some("mid"));
    assert_eq!(ids[2], new_b);
}

#[test]
fn replace_with_inserts_siblings_then_detaches_self() {
    let mut dom: Dom = Dom::new();
    let parent = dom.create_element("div");
    let target = dom.create_element("span");
    let tail = dom.create_element("span");
    dom.append_child(parent, target).unwrap();
    dom.append_child(parent, tail).unwrap();
    let r1 = dom.create_element("p");
    let r2 = dom.create_element("p");
    dom.node_mut(target)
        .replace_with([r1.into(), "mid".into(), r2.into()])
        .unwrap();
    let ids = child_ids(&dom, parent);
    assert_eq!(ids.len(), 4);
    assert_eq!(ids[0], r1);
    assert_eq!(dom.node(ids[1]).node_value(), Some("mid"));
    assert_eq!(ids[2], r2);
    assert_eq!(ids[3], tail);
    // target is still alive in the arena but parentless.
    assert!(dom.node(target).parent_node().is_none());
}

#[test]
fn replace_with_on_parentless_node_is_silent_noop() {
    let mut dom: Dom = Dom::new();
    let detached = dom.create_element("div");
    let extra = dom.create_element("p");
    dom.node_mut(detached).replace_with([extra.into()]).unwrap();
    // No parent, no operation. extra is still detached too.
    assert!(dom.node(detached).parent_node().is_none());
    assert!(dom.node(extra).parent_node().is_none());
}

#[test]
fn remove_self_detaches_from_parent() {
    let mut dom: Dom = Dom::new();
    let parent = dom.create_element("div");
    let target = dom.create_element("span");
    let sibling = dom.create_element("span");
    dom.append_child(parent, target).unwrap();
    dom.append_child(parent, sibling).unwrap();
    dom.node_mut(target).remove_self().unwrap();
    let ids = child_ids(&dom, parent);
    assert_eq!(ids, vec![sibling]);
    assert!(dom.node(target).parent_node().is_none());
}

#[test]
fn remove_self_on_parentless_node_is_silent_noop() {
    let mut dom: Dom = Dom::new();
    let detached = dom.create_element("div");
    // Already parentless: silent no-op.
    dom.node_mut(detached).remove_self().unwrap();
    assert!(dom.contains(detached));
}

// ── M4b step 19: HTMLElement IDL accessors ────────────────────────

#[test]
fn dataset_round_trips_camelcase_to_kebab() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    {
        let mut nm = dom.node_mut(el);
        let mut ds = nm.dataset_mut();
        ds.set("fooBar", "yes").unwrap();
        ds.set("x", "1").unwrap();
    }
    // Raw attribute names use kebab-case.
    assert_eq!(dom.node(el).get_attribute("data-foo-bar"), Some("yes"));
    assert_eq!(dom.node(el).get_attribute("data-x"), Some("1"));
    // Dataset getter reads back via camelCase.
    let ds = dom.node(el).dataset();
    assert_eq!(ds.get("fooBar"), Some("yes"));
    assert_eq!(ds.get("x"), Some("1"));
    assert_eq!(ds.len(), 2);
}

#[test]
fn tab_index_returns_parsed_attribute_or_none() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("input");
    assert_eq!(dom.node(el).tab_index(), None);
    dom.node_mut(el).set_tab_index(0).unwrap();
    assert_eq!(dom.node(el).tab_index(), Some(0));
    dom.node_mut(el).set_tab_index(-1).unwrap();
    assert_eq!(dom.node(el).tab_index(), Some(-1));
    // Manually written garbage attribute returns None.
    dom.node_mut(el)
        .set_attribute("tabindex", "not a number")
        .unwrap();
    assert_eq!(dom.node(el).tab_index(), None);
}

#[test]
fn hidden_reads_attribute_presence() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    assert!(!dom.node(el).hidden());
    dom.node_mut(el).set_hidden(true).unwrap();
    assert!(dom.node(el).hidden());
    assert!(dom.node(el).has_attribute("hidden"));
    dom.node_mut(el).set_hidden(false).unwrap();
    assert!(!dom.node(el).hidden());
    assert!(!dom.node(el).has_attribute("hidden"));
}

#[test]
fn content_editable_defaults_to_inherit() {
    let mut dom: Dom = Dom::new();
    let el = dom.create_element("div");
    // No attribute: IDL returns "inherit".
    assert_eq!(dom.node(el).content_editable(), "inherit");
    dom.node_mut(el).set_content_editable("true").unwrap();
    assert_eq!(dom.node(el).content_editable(), "true");
    dom.node_mut(el)
        .set_content_editable("plaintext-only")
        .unwrap();
    assert_eq!(dom.node(el).content_editable(), "plaintext-only");
}

#[test]
fn inner_html_and_outer_html_getters_delegate_to_markup() {
    let mut dom: Dom = Dom::new();
    let parent = dom.create_element("div");
    dom.node_mut(parent).set_id("hero").unwrap();
    let child = dom.create_element("span");
    let text = dom.create_text_node("hi");
    dom.append_child(child, text).unwrap();
    dom.append_child(parent, child).unwrap();
    // inner_html: just the children's markup.
    assert_eq!(dom.node(parent).inner_html(), "<span>hi</span>");
    // outer_html: includes self.
    assert_eq!(
        dom.node(parent).outer_html(),
        r#"<div id="hero"><span>hi</span></div>"#
    );
}
