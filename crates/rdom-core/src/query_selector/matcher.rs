//! The matcher: a complex selector right to left from its subject,
//! each compound's simple selectors in turn.

use super::attribute::match_attribute;
use crate::dom::Dom;
use crate::node::NodeData;
use crate::node_id::NodeId;
use crate::selectors::{self, Combinator, CompoundSelector, PseudoClass, SimpleSelector};

impl<Ext> Dom<Ext> {
    pub(super) fn matches_complex(
        &self,
        id: NodeId,
        complex: &selectors::ComplexSelector,
        scope: Option<NodeId>,
    ) -> bool {
        // Subject must match.
        if !self.matches_compound(id, &complex.subject, scope) {
            return false;
        }
        // Walk ancestors/siblings per combinator. Each step's "candidate
        // pointer" represents the node we're trying to match against the
        // next compound on the outward path.
        let mut cur = id;
        for (comb, compound) in &complex.ancestors {
            match comb {
                Combinator::Descendant => {
                    let mut anc = self.get_node(cur).and_then(|n| n.parent);
                    let mut matched = None;
                    while let Some(a) = anc {
                        if self.matches_compound(a, compound, scope) {
                            matched = Some(a);
                            break;
                        }
                        anc = self.get_node(a).and_then(|n| n.parent);
                    }
                    match matched {
                        Some(a) => cur = a,
                        None => return false,
                    }
                }
                Combinator::Child => {
                    let Some(parent) = self.get_node(cur).and_then(|n| n.parent) else {
                        return false;
                    };
                    if !self.matches_compound(parent, compound, scope) {
                        return false;
                    }
                    cur = parent;
                }
                Combinator::AdjacentSibling => {
                    let Some(prev) = self.get_node(cur).and_then(|n| n.prev_sibling) else {
                        return false;
                    };
                    if !self.matches_compound(prev, compound, scope) {
                        return false;
                    }
                    cur = prev;
                }
                Combinator::GeneralSibling => {
                    let mut sib = self.get_node(cur).and_then(|n| n.prev_sibling);
                    let mut matched = None;
                    while let Some(s) = sib {
                        if self.matches_compound(s, compound, scope) {
                            matched = Some(s);
                            break;
                        }
                        sib = self.get_node(s).and_then(|n| n.prev_sibling);
                    }
                    match matched {
                        Some(s) => cur = s,
                        None => return false,
                    }
                }
            }
        }
        true
    }

    fn matches_compound(
        &self,
        id: NodeId,
        compound: &CompoundSelector,
        scope: Option<NodeId>,
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
            return scope == Some(id) && compound.simples.iter().all(names_only_scope);
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
                SimpleSelector::Attribute { name, op, value } => {
                    if !match_attribute(attrs, name, *op, value.as_deref()) {
                        return false;
                    }
                }
                SimpleSelector::Not(inner) => {
                    if self.matches_list_in_scope(id, inner, scope) {
                        return false;
                    }
                }
                SimpleSelector::Is(inner) | SimpleSelector::Where(inner) => {
                    // `:is()` matching — any complex selector in the list must
                    // match this element as its subject. Specificity (zero for
                    // `:where()`) is `ComplexSelector::specificity`'s.
                    if !self.matches_list_in_scope(id, inner, scope) {
                        return false;
                    }
                }
                SimpleSelector::Pseudo(p) => {
                    if !self.match_pseudo(id, *p, scope) {
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
