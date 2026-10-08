//! The structural pseudo-classes that count siblings (Selectors 4
//! §13.3–§13.4): `:nth-child()`, `:nth-last-child()`, `:nth-of-type()`,
//! `:nth-last-of-type()` and `:first-of-type` / `:last-of-type` /
//! `:only-of-type`, through the pass's nth index
//! ([`SelectorCaches`](super::caches::SelectorCaches)).

use std::collections::HashMap;

use super::caches::NthCount;
use super::matcher::Cx;
use crate::dom::Dom;
use crate::node::NodeData;
use crate::node_id::NodeId;
use crate::selectors::{NthKind, NthSelector, SelectorList};

impl<Ext> Dom<Ext> {
    /// Whether `id` matches `nth`.
    pub(super) fn matches_nth(&self, id: NodeId, nth: &NthSelector, cx: &mut Cx<'_>) -> bool {
        let count = match &nth.of {
            None if matches!(nth.kind, NthKind::OfType | NthKind::LastOfType) => NthCount::OfType,
            None => NthCount::Child,
            Some(of) => {
                if !self.matches_list_cx(id, of, cx) {
                    return false;
                }
                NthCount::Of(std::ptr::from_ref(of) as usize)
            }
        };
        let Some((from_start, from_end)) = self.nth_position(id, count, nth.of.as_ref(), cx) else {
            return false;
        };
        let index = match nth.kind {
            NthKind::Child | NthKind::OfType => from_start,
            NthKind::LastChild | NthKind::LastOfType => from_end,
        };
        nth.matches_index(index)
    }

    /// `:first-of-type` / `:last-of-type` / `:only-of-type`: the
    /// element's index among its type from the start and from the end.
    pub(super) fn type_position(&self, id: NodeId, cx: &mut Cx<'_>) -> Option<(u32, u32)> {
        self.nth_position(id, NthCount::OfType, None, cx)
    }

    /// `id`'s 1-based index among the siblings `count` counts, from the
    /// start and from the end. An element without a parent is the first
    /// and last of its siblings (Selectors 4 §13.3: Level 4 drops the
    /// parent requirement). `of` is the list an `NthCount::Of` names.
    fn nth_position(
        &self,
        id: NodeId,
        count: NthCount,
        of: Option<&SelectorList>,
        cx: &mut Cx<'_>,
    ) -> Option<(u32, u32)> {
        let Some(parent) = self.get_node(id)?.parent else {
            return Some((1, 1));
        };
        let key = (parent, count);
        if !cx.caches.nth.contains_key(&key) {
            let index = self.index_children(parent, count, of, cx);
            cx.caches.nth.insert(key, index);
        }
        let (index, total) = *cx.caches.nth.get(&key)?.get(&id)?;
        Some((index, total - index + 1))
    }

    /// Every child of `parent` that `count` counts, with its 1-based
    /// index and the size of its group: one pass over the children (two
    /// for the per-type totals).
    fn index_children(
        &self,
        parent: NodeId,
        count: NthCount,
        of: Option<&SelectorList>,
        cx: &mut Cx<'_>,
    ) -> HashMap<NodeId, (u32, u32)> {
        let mut index = HashMap::new();
        let mut per_type: HashMap<&str, u32> = HashMap::new();
        let mut counted = 0;
        let mut child = self.get_node(parent).and_then(|n| n.first_child);
        while let Some(c) = child {
            cx.caches.count_nth_sibling();
            let Some(node) = self.get_node(c) else { break };
            child = node.next_sibling;
            let NodeData::Element { tag, .. } = &node.data else {
                continue;
            };
            let at = match count {
                NthCount::Child => {
                    counted += 1;
                    counted
                }
                NthCount::OfType => {
                    let n = per_type.entry(tag.as_str()).or_insert(0);
                    *n += 1;
                    *n
                }
                NthCount::Of(_) => {
                    if !of.is_some_and(|of| self.matches_list_cx(c, of, cx)) {
                        continue;
                    }
                    counted += 1;
                    counted
                }
            };
            index.insert(c, (at, 0));
        }
        for (node, (_, total)) in &mut index {
            *total = match count {
                NthCount::OfType => self.tag_of(*node).map_or(0, |t| per_type[t]),
                _ => counted,
            };
        }
        index
    }

    fn tag_of(&self, id: NodeId) -> Option<&str> {
        match &self.get_node(id)?.data {
            NodeData::Element { tag, .. } => Some(tag),
            _ => None,
        }
    }
}
