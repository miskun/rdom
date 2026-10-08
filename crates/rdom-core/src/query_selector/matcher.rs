//! The matcher: a complex selector right to left from its subject,
//! each compound's simple selectors in turn.

use super::attribute::match_attribute;
use super::caches::SelectorCaches;
use crate::dom::Dom;
use crate::language::lang_range_matches;
use crate::node::NodeData;
use crate::node_id::NodeId;
use crate::selectors::{
    self, Combinator, CompoundSelector, PseudoClass, SelectorList, SimpleSelector,
};

/// How a [`Dom::match_chain`] attempt ended — Servo's matching
/// results, which say how far back a failure sends the search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Outcome {
    Matched,
    /// No candidate anywhere can match: stop.
    NotMatchedGlobally,
    /// Retry from the nearest descendant combinator to the right.
    RestartFromClosestDescendant,
    /// Retry from the nearest subsequent-sibling combinator (or
    /// descendant one) to the right.
    RestartFromClosestLaterSibling,
}

/// What a chain step must find: an element matching a compound, or
/// the `:has()` anchor itself.
#[derive(Clone, Copy)]
enum Target<'s> {
    Compound(&'s CompoundSelector),
    Node(NodeId),
}

/// What one match reads beyond the tree: the scoping root `:scope`
/// matches (Selectors 4 §14.3) and the pass's caches.
pub(super) struct Cx<'c> {
    pub scope: Option<NodeId>,
    pub caches: &'c mut SelectorCaches,
}

impl<Ext> Dom<Ext> {
    /// Whether `id` matches any complex selector of `list`.
    pub(super) fn matches_list_cx(&self, id: NodeId, list: &SelectorList, cx: &mut Cx<'_>) -> bool {
        list.0
            .iter()
            .any(|complex| self.matches_complex(id, complex, cx))
    }

    pub(super) fn matches_complex(
        &self,
        id: NodeId,
        complex: &selectors::ComplexSelector,
        cx: &mut Cx<'_>,
    ) -> bool {
        self.matches_compound(id, &complex.subject, cx)
            && self.match_chain(id, &complex.ancestors, None, cx) == Outcome::Matched
    }

    /// Match `chain` (compounds right to left, each with the combinator
    /// linking it to the one on its right) outward from `el`, which
    /// matched the compound right of `chain[0]`; then, with `anchor`,
    /// the anchor element `:has()` relates the leftmost compound to
    /// (Selectors 4 §4.5: the relative selector's leading combinator).
    ///
    /// Backtracking (Selectors 4 §3.1: *some* assignment of elements
    /// must satisfy every combinator): a candidate whose rest fails is
    /// followed by the next candidate where trying one can still help.
    /// The outcomes bound that search as Servo's matcher does — a
    /// failure that no farther candidate of this combinator can fix
    /// returns at once — so it stays linear in the depth per descendant
    /// combinator, not exponential.
    pub(super) fn match_chain(
        &self,
        el: NodeId,
        chain: &[(Combinator, CompoundSelector)],
        anchor: Option<(Combinator, NodeId)>,
        cx: &mut Cx<'_>,
    ) -> Outcome {
        let (comb, target) = match chain.split_first() {
            Some(((comb, compound), _)) => (*comb, Target::Compound(compound)),
            None => match anchor {
                None => return Outcome::Matched,
                Some((comb, node)) => (comb, Target::Node(node)),
            },
        };
        let rest = chain.get(1..).unwrap_or_default();
        let not_found = match comb {
            Combinator::AdjacentSibling | Combinator::GeneralSibling => {
                Outcome::RestartFromClosestDescendant
            }
            _ => Outcome::NotMatchedGlobally,
        };
        let mut next = self.step(el, comb);
        loop {
            let Some(candidate) = next else {
                return not_found;
            };
            // A compound of a `:has()` argument sits below the anchor (or
            // below its later siblings): a climb that reaches the anchor
            // — or, from its siblings, their parent — has no candidate
            // left (Selectors 4 §4.5).
            if let (Target::Compound(_), Some((lead, a))) = (target, anchor)
                && self.is_past_anchor(candidate, lead, a)
            {
                return not_found;
            }
            cx.caches.count_chain_step();
            let result = match target {
                Target::Node(node) if candidate == node => Outcome::Matched,
                Target::Node(_) => Outcome::RestartFromClosestLaterSibling,
                Target::Compound(compound) if self.matches_compound(candidate, compound, cx) => {
                    self.match_chain(candidate, rest, anchor, cx)
                }
                Target::Compound(_) => Outcome::RestartFromClosestLaterSibling,
            };
            match (result, comb) {
                (Outcome::Matched | Outcome::NotMatchedGlobally, _)
                | (_, Combinator::AdjacentSibling) => return result,
                (_, Combinator::Child) => return Outcome::RestartFromClosestDescendant,
                (Outcome::RestartFromClosestDescendant, Combinator::GeneralSibling) => {
                    return result;
                }
                // A descendant combinator tries the next ancestor; a
                // subsequent-sibling one the next earlier sibling.
                _ => {}
            }
            next = self.step(candidate, comb);
        }
    }

    /// Whether a chain step reaching `candidate` has left the region a
    /// `:has()` argument led by `lead` from `anchor` can occupy: the
    /// anchor's subtree for a descendant / child lead, its later siblings
    /// and their subtrees for a sibling one.
    fn is_past_anchor(&self, candidate: NodeId, lead: Combinator, anchor: NodeId) -> bool {
        candidate == anchor
            || (matches!(
                lead,
                Combinator::AdjacentSibling | Combinator::GeneralSibling
            ) && self.get_node(anchor).and_then(|n| n.parent) == Some(candidate))
    }

    /// The next candidate `comb` relates `el` to: its parent for a
    /// descendant / child combinator, its previous *element* sibling for
    /// a sibling combinator (Selectors 4 §14.3 / §14.4).
    fn step(&self, el: NodeId, comb: Combinator) -> Option<NodeId> {
        match comb {
            Combinator::Descendant | Combinator::Child => self.get_node(el)?.parent,
            Combinator::AdjacentSibling | Combinator::GeneralSibling => {
                self.prev_element_sibling_id(el)
            }
        }
    }

    pub(super) fn matches_compound(
        &self,
        id: NodeId,
        compound: &CompoundSelector,
        cx: &mut Cx<'_>,
    ) -> bool {
        let Some(node) = self.get_node(id) else {
            return false;
        };
        let NodeData::Element {
            tag,
            attrs,
            classes,
            ..
        } = &node.data
        else {
            // A non-element is matched only as the scoping root — the
            // document, for a prelude-less `@scope` in a sheet with no
            // owner node (CSS Cascade 6 §2.5.1).
            return cx.scope == Some(id) && compound.simples.iter().all(names_only_scope);
        };
        for s in &compound.simples {
            match s {
                SimpleSelector::Universal => {}
                SimpleSelector::Type(t) => {
                    if tag != t {
                        return false;
                    }
                }
                SimpleSelector::Id(v) => {
                    if attrs.get("id").map(String::as_str) != Some(v.as_str()) {
                        return false;
                    }
                }
                SimpleSelector::Class(c) => {
                    if !classes.contains(c) {
                        return false;
                    }
                }
                SimpleSelector::Attribute {
                    name,
                    op,
                    value,
                    case,
                } => {
                    if !match_attribute(attrs, name, *op, value.as_deref(), *case) {
                        return false;
                    }
                }
                SimpleSelector::Not(inner) => {
                    if self.matches_list_cx(id, inner, cx) {
                        return false;
                    }
                }
                SimpleSelector::Is(inner) | SimpleSelector::Where(inner) => {
                    // `:is()` matching — any complex selector in the list must
                    // match this element as its subject. Specificity (zero for
                    // `:where()`) is `ComplexSelector::specificity`'s.
                    if !self.matches_list_cx(id, inner, cx) {
                        return false;
                    }
                }
                SimpleSelector::Pseudo(p) => {
                    if !self.match_pseudo(id, *p, cx) {
                        return false;
                    }
                }
                SimpleSelector::Has(relative) => {
                    if !self.matches_has(id, relative, cx) {
                        return false;
                    }
                }
                SimpleSelector::Lang(ranges) => {
                    let lang = self.language(id).unwrap_or("");
                    if !ranges.iter().any(|r| lang_range_matches(r, lang)) {
                        return false;
                    }
                }
                SimpleSelector::Nth(nth) => {
                    if !self.matches_nth(id, nth, cx) {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub(super) fn prev_element_sibling_id(&self, id: NodeId) -> Option<NodeId> {
        let mut cur = self.get_node(id).and_then(|n| n.prev_sibling);
        while let Some(c) = cur {
            let n = self.get_node(c)?;
            if matches!(n.data, NodeData::Element { .. }) {
                return Some(c);
            }
            cur = n.prev_sibling;
        }
        None
    }

    pub(super) fn next_element_sibling_id(&self, id: NodeId) -> Option<NodeId> {
        let mut cur = self.get_node(id).and_then(|n| n.next_sibling);
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

/// A simple selector that only ever matches the scoping root: `:scope`,
/// or `:is()` / `:where()` over `:scope` alone.
fn names_only_scope(simple: &SimpleSelector) -> bool {
    match simple {
        SimpleSelector::Pseudo(PseudoClass::Scope) => true,
        SimpleSelector::Is(list) | SimpleSelector::Where(list) => {
            // An empty (forgiving) list matches nothing, not the root.
            !list.0.is_empty()
                && list.0.iter().all(|c| {
                    c.ancestors.is_empty() && c.subject.simples.iter().all(names_only_scope)
                })
        }
        _ => false,
    }
}
