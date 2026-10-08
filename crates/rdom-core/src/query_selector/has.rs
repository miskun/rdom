//! The relational pseudo-class `:has()` (Selectors 4 §4.5): an anchor
//! matches when one of its relative selectors matches an element
//! related to it — a descendant, a child, the next sibling or a later
//! sibling, the rest of the selector read from there. Answers are kept
//! in the pass's [`SelectorCaches`](super::caches::SelectorCaches).

use super::matcher::{Cx, Outcome};
use crate::dom::Dom;
use crate::node::NodeData;
use crate::node_id::NodeId;
use crate::selectors::{Combinator, RelativeSelector};

impl<Ext> Dom<Ext> {
    /// Whether the anchor `id` matches `:has(relative)`. Records `id` as
    /// a `:has()` anchor.
    pub(super) fn matches_has(
        &self,
        id: NodeId,
        relative: &[RelativeSelector],
        cx: &mut Cx<'_>,
    ) -> bool {
        cx.caches.note_has_anchor(id);
        relative
            .iter()
            .any(|rel| self.matches_relative(id, rel, cx))
    }

    fn matches_relative(&self, anchor: NodeId, rel: &RelativeSelector, cx: &mut Cx<'_>) -> bool {
        let key = (std::ptr::from_ref(rel) as usize, anchor);
        if let Some(&known) = cx.caches.has.get(&key) {
            return known;
        }
        let found = if rel.combinator == Combinator::Descendant && rel.selector.ancestors.is_empty()
        {
            self.subtree_holds(anchor, rel, cx)
        } else {
            self.search_relative(anchor, rel, cx)
        };
        cx.caches.has.insert(key, found);
        found
    }

    /// `:has(<compound>)`: whether some proper descendant of `anchor`
    /// matches the compound. Computed bottom-up for every element of the
    /// subtree not already known, each recorded, so a nested anchor
    /// reads its answer instead of searching again.
    fn subtree_holds(&self, anchor: NodeId, rel: &RelativeSelector, cx: &mut Cx<'_>) -> bool {
        let at = std::ptr::from_ref(rel) as usize;
        let mut stack = vec![(anchor, false)];
        while let Some((node, expanded)) = stack.pop() {
            if !expanded {
                if cx.caches.has.contains_key(&(at, node)) {
                    continue;
                }
                cx.caches.count_has_node();
                stack.push((node, true));
                let mut child = self.first_element_child_id(node);
                while let Some(c) = child {
                    stack.push((c, false));
                    child = self.next_element_sibling_id(c);
                }
                continue;
            }
            let mut holds = false;
            let mut child = self.first_element_child_id(node);
            while let Some(c) = child {
                holds = holds
                    || cx.caches.has.get(&(at, c)) == Some(&true)
                    || self.matches_compound(c, &rel.selector.subject, cx);
                child = self.next_element_sibling_id(c);
            }
            cx.caches.has.insert((at, node), holds);
        }
        cx.caches.has.get(&(at, anchor)) == Some(&true)
    }

    /// Any other relative selector: try each element it can reach as its
    /// subject — the anchor's descendants (children only, for `>` with no
    /// descendant step after it), or its later siblings (the next one
    /// only, for `+` with no sibling step after it) and, when the
    /// selector steps down from them, their descendants.
    fn search_relative(&self, anchor: NodeId, rel: &RelativeSelector, cx: &mut Cx<'_>) -> bool {
        let steps = || rel.selector.ancestors.iter().map(|(c, _)| *c);
        let steps_down = steps().any(|c| matches!(c, Combinator::Descendant | Combinator::Child));
        let steps_across =
            steps().any(|c| matches!(c, Combinator::AdjacentSibling | Combinator::GeneralSibling));
        let mut roots = Vec::new();
        let mut deep = steps_down;
        match rel.combinator {
            Combinator::Descendant | Combinator::Child => {
                deep |= rel.combinator == Combinator::Descendant;
                let mut child = self.first_element_child_id(anchor);
                while let Some(c) = child {
                    roots.push(c);
                    child = self.next_element_sibling_id(c);
                }
            }
            Combinator::AdjacentSibling | Combinator::GeneralSibling => {
                let mut sib = self.next_element_sibling_id(anchor);
                while let Some(s) = sib {
                    roots.push(s);
                    if rel.combinator == Combinator::AdjacentSibling && !steps_across {
                        break;
                    }
                    sib = self.next_element_sibling_id(s);
                }
            }
        }
        let mut stack: Vec<NodeId> = roots.into_iter().rev().collect();
        while let Some(c) = stack.pop() {
            cx.caches.count_has_node();
            if self.matches_compound(c, &rel.selector.subject, cx)
                && self.match_chain(
                    c,
                    &rel.selector.ancestors,
                    Some((rel.combinator, anchor)),
                    cx,
                ) == Outcome::Matched
            {
                return true;
            }
            if deep {
                let at = stack.len();
                let mut child = self.first_element_child_id(c);
                while let Some(k) = child {
                    stack.push(k);
                    child = self.next_element_sibling_id(k);
                }
                stack[at..].reverse();
            }
        }
        false
    }

    fn first_element_child_id(&self, id: NodeId) -> Option<NodeId> {
        let mut cur = self.get_node(id)?.first_child;
        while let Some(c) = cur {
            let n = self.get_node(c)?;
            if matches!(n.data, NodeData::Element { .. }) {
                return Some(c);
            }
            cur = n.next_sibling;
        }
        None
    }
}
