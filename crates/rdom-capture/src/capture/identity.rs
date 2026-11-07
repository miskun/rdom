//! Node identity across captures (`specs/CAPTURE.md` §Node identity). A
//! node key (`node_key`) names an element in one capture; across captures
//! — a diff, a film, a fork — elements are matched by their identity
//! instead: a path of steps from the nearest element with a unique `id`,
//! each step the most stable thing that names it among its siblings, in
//! order:
//!
//! 1. `#id` — a unique `id` attribute (a duplicated one names nothing);
//! 2. `[key=k]` — a `data-key` attribute (a list item's model key), with
//!    `:n` when earlier siblings share it;
//! 3. `tag.class.class:n` — the tag and classes (sorted, so their order
//!    does not matter), and its index among the earlier siblings that
//!    share them (`:0` is left off).
//!
//! `#settings/ul.items/[key=r42]/span.label` — and [`By`] says which rule
//! named the element itself, which a diff reports for every pair.

use std::collections::HashMap;

use rdom_tui::TuiDom;
use rdom_tui::core_api::{NodeId, NodeType};
use serde::{Deserialize, Serialize};

/// Which rule named an element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum By {
    /// Its unique `id` attribute.
    Id,
    /// Its `data-key` attribute, under its parent.
    DataKey,
    /// Its tag, classes and place among like siblings, under its parent.
    Fingerprint,
}

/// An element's identity: its path, and which rule named it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Identity {
    pub key: String,
    pub by: By,
}

/// The identity of every element of `dom`.
pub(crate) fn identities(dom: &TuiDom) -> HashMap<NodeId, Identity> {
    let elements: Vec<NodeId> = dom
        .descendants(dom.root())
        .filter(|&id| dom.node(id).node_type() == NodeType::Element)
        .collect();
    let mut ids: HashMap<&str, usize> = HashMap::new();
    for &e in &elements {
        if let Some(i) = dom.node(e).get_attribute("id") {
            *ids.entry(i).or_default() += 1;
        }
    }
    let mut out: HashMap<NodeId, Identity> = HashMap::with_capacity(elements.len());
    // Per parent, how many earlier children took each step.
    let mut seen: HashMap<(Option<NodeId>, String), usize> = HashMap::new();
    // Document order: a parent is named before its children.
    for &e in &elements {
        let node = dom.node(e);
        let parent = node
            .parent_node()
            .map(|p| p.id())
            .filter(|p| dom.node(*p).node_type() == NodeType::Element);
        if let Some(i) = node.get_attribute("id").filter(|i| ids.get(i) == Some(&1)) {
            out.insert(
                e,
                Identity {
                    key: format!("#{i}"),
                    by: By::Id,
                },
            );
            continue;
        }
        let (step, by) = match node.get_attribute("data-key") {
            Some(k) => (format!("[key={k}]"), By::DataKey),
            None => {
                let mut classes: Vec<&str> = dom.class_list(e).collect();
                classes.sort_unstable();
                let mut s = node.tag_name().unwrap_or_default().to_ascii_lowercase();
                for c in classes {
                    s.push('.');
                    s.push_str(c);
                }
                (s, By::Fingerprint)
            }
        };
        let n = seen.entry((parent, step.clone())).or_default();
        let step = if *n == 0 { step } else { format!("{step}:{n}") };
        *n += 1;
        let key = match parent.and_then(|p| out.get(&p)) {
            Some(p) => format!("{}/{step}", p.key),
            None => step,
        };
        out.insert(e, Identity { key, by });
    }
    out
}

#[cfg(test)]
mod tests {
    use rdom_tui::TuiDom;

    use super::{By, identities};

    fn el(
        dom: &mut TuiDom,
        parent: rdom_tui::core_api::NodeId,
        tag: &str,
        attrs: &[(&str, &str)],
    ) -> rdom_tui::core_api::NodeId {
        let e = dom.create_element(tag);
        for (k, v) in attrs {
            dom.set_attribute(e, k, v).unwrap();
        }
        dom.append_child(parent, e).unwrap();
        e
    }

    #[test]
    fn elements_are_named_by_id_then_data_key_then_fingerprint() {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let main = el(&mut dom, root, "main", &[("id", "app")]);
        let list = el(&mut dom, main, "ul", &[("class", "items big")]);
        let a = el(&mut dom, list, "li", &[("data-key", "r1")]);
        let b = el(&mut dom, list, "li", &[("data-key", "r2")]);
        let span = el(&mut dom, b, "span", &[("class", "label")]);
        let p1 = el(&mut dom, main, "p", &[]);
        let p2 = el(&mut dom, main, "p", &[]);
        let ids = identities(&dom);
        let key = |n| ids[&n].key.clone();
        assert_eq!(key(main), "#app");
        assert_eq!(key(list), "#app/ul.big.items", "classes sorted");
        assert_eq!(key(a), "#app/ul.big.items/[key=r1]");
        assert_eq!(key(span), "#app/ul.big.items/[key=r2]/span.label");
        assert_eq!((key(p1), key(p2)), ("#app/p".into(), "#app/p:1".into()));
        assert_eq!(
            (ids[&main].by, ids[&a].by, ids[&p2].by),
            (By::Id, By::DataKey, By::Fingerprint)
        );
    }

    #[test]
    fn a_duplicated_id_names_nothing() {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let x = el(&mut dom, root, "div", &[("id", "dup")]);
        let y = el(&mut dom, root, "div", &[("id", "dup")]);
        let ids = identities(&dom);
        assert_eq!(
            (ids[&x].key.as_str(), ids[&y].key.as_str()),
            ("div", "div:1")
        );
    }

    #[test]
    fn an_unlike_sibling_inserted_before_leaves_a_key_as_it_was() {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let p = el(&mut dom, root, "p", &[("class", "note")]);
        let before = identities(&dom)[&p].key.clone();
        let h = dom.create_element("h1");
        dom.insert_before(root, h, Some(p)).unwrap();
        assert_eq!(identities(&dom)[&p].key, before);
    }
}
