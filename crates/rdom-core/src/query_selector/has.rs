//! The relational pseudo-class `:has()` (Selectors 4 §4.5): an anchor
//! matches when one of its relative selectors matches an element
//! related to it — a descendant, a child, the next sibling or a later
//! sibling, the rest of the selector read from there. Answers are kept
//! in the pass's [`SelectorCaches`](super::caches::SelectorCaches), per
//! scoping root.
//!
//! Two strategies, by the relative selector's combinators:
//!
//! - **Downward** (descendant and child combinators only: `:has(.x)`,
//!   `:has(> li)`, `:has(p div)`): answered bottom-up per (element,
//!   compound) — "does some element below this one, related as the
//!   combinator says, match this compound and the rest to its right?" —
//!   each computed once per pass and shared by every anchor, so nested
//!   anchors cost O(N · compounds), not a search each.
//! - **With a sibling combinator** (`:has(+ h2)`, `:has(~ p a)`,
//!   `:has(> a + b)`): each element the relative selector can reach as
//!   its subject is tried, the chain read back towards the anchor — its
//!   climbs bounded by the anchor (`Dom::match_chain`).

use super::caches::HasKey;
use super::matcher::{Cx, Outcome};
use crate::dom::Dom;
use crate::node::NodeData;
use crate::node_id::NodeId;
use crate::selectors::{Combinator, CompoundSelector, RelativeSelector};

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
        if is_downward(rel) {
            return self.holds_below(anchor, rel, 0, cx);
        }
        let key = HasKey::new(rel, u32::MAX, cx.scope, anchor);
        if let Some(&known) = cx.caches.has.get(&key) {
            return known;
        }
        let found = self.search_relative(anchor, rel, cx);
        cx.caches.has.insert(key, found);
        found
    }

    /// For a downward relative selector with compounds `c0 … cn` left to
    /// right (`c0` related to the anchor by the leading combinator):
    /// whether some element below `node`, related to it by the
    /// combinator left of `c_k`, matches `c_k` and — unless it is the
    /// subject — holds `c_k+1 …` below it in turn. Every element of
    /// `node`'s subtree the answer reads is computed once per pass
    /// (bottom-up, iteratively) and recorded.
    fn holds_below(&self, node: NodeId, rel: &RelativeSelector, k: usize, cx: &mut Cx<'_>) -> bool {
        let scope = cx.scope;
        let key = |n| HasKey::new(rel, k as u32, scope, n);
        if let Some(&known) = cx.caches.has.get(&key(node)) {
            return known;
        }
        let deep = combinator_left_of(rel, k) == Combinator::Descendant;
        let mut stack = vec![(node, false)];
        while let Some((n, expanded)) = stack.pop() {
            let n_key = key(n);
            if !expanded {
                if cx.caches.has.contains_key(&n_key) {
                    continue;
                }
                cx.caches.count_has_node();
                stack.push((n, true));
                if deep {
                    let mut child = self.first_element_child_id(n);
                    while let Some(c) = child {
                        stack.push((c, false));
                        child = self.next_element_sibling_id(c);
                    }
                }
                continue;
            }
            let mut holds = false;
            let mut child = self.first_element_child_id(n);
            while let Some(c) = child
                && !holds
            {
                holds = (deep && cx.caches.has.get(&key(c)) == Some(&true))
                    || (self.matches_compound(c, compound_at(rel, k), cx)
                        && (k == last_index(rel) || self.holds_below(c, rel, k + 1, cx)));
                child = self.next_element_sibling_id(c);
            }
            cx.caches.has.insert(n_key, holds);
        }
        cx.caches.has.get(&key(node)) == Some(&true)
    }

    /// A relative selector with a sibling combinator: try each element it
    /// can reach as its subject — the anchor's descendants (children
    /// only, for `>` with no descendant step after it), or its later
    /// siblings (the next one only, for `+` with no sibling step after
    /// it) and, when the selector steps down from them, their
    /// descendants.
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
            // A relative selector leads with `>`, `+`, `~` or nothing
            // (Selectors 4 §3.4): the parser makes no `||` lead.
            Combinator::Column => {}
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

    pub(crate) fn first_element_child_id(&self, id: NodeId) -> Option<NodeId> {
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

/// Whether every combinator of `rel` — the leading one and those between
/// its compounds — is a descendant or child combinator.
fn is_downward(rel: &RelativeSelector) -> bool {
    std::iter::once(rel.combinator)
        .chain(rel.selector.ancestors.iter().map(|(c, _)| *c))
        .all(|c| matches!(c, Combinator::Descendant | Combinator::Child))
}

/// The index of the subject in `rel`'s compounds, left to right.
fn last_index(rel: &RelativeSelector) -> usize {
    rel.selector.ancestors.len()
}

/// `rel`'s `k`-th compound, left to right (the subject last). `ancestors`
/// is right to left.
fn compound_at(rel: &RelativeSelector, k: usize) -> &CompoundSelector {
    let n = rel.selector.ancestors.len();
    if k == n {
        &rel.selector.subject
    } else {
        &rel.selector.ancestors[n - 1 - k].1
    }
}

/// The combinator left of `rel`'s `k`-th compound: the leading one for
/// the first; otherwise the one linking the compound before it (stored
/// with that compound, right to left).
fn combinator_left_of(rel: &RelativeSelector, k: usize) -> Combinator {
    if k == 0 {
        rel.combinator
    } else {
        let n = rel.selector.ancestors.len();
        rel.selector.ancestors[n - k].0
    }
}
