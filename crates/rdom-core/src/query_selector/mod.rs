//! Matcher + public query_selector API.
//!
//! `Dom::query_selector_in(root, selector)` finds the first descendant
//! matching `selector`. `query_selector_all_in` returns every match.
//! `matches` tests a single node. `closest` walks ancestors finding the
//! first match. The DOM-shaped one-arg shortcuts
//! [`Dom::query_selector`](crate::Dom::query_selector) /
//! [`Dom::query_selector_all`](crate::Dom::query_selector_all) live on
//! `Dom` directly and pass `self.root()` as the root_id.
//!
//! Each query method matches with the node it was called on as the
//! scoping root (DOM §4.2.6 "scope-match a selectors string", Selectors 4
//! §8.4): `query_selector_in(div, ":scope > p")` finds `div`'s own `p`
//! children, `matches(id, ":scope")` and `closest(id, ":scope")` are
//! `id`. Each call shares one [`SelectorCaches`] across the elements it
//! tests.
//!
//! The matcher evaluates each `ComplexSelector` right-to-left starting from
//! the candidate element (subject), then walks ancestors/siblings per
//! combinator.

use crate::dom::Dom;
use crate::node::NodeData;
use crate::node_id::NodeId;
use crate::selectors::{self, ParseError, SelectorList};

mod attribute;
pub(crate) mod caches;
mod has;
mod matcher;
mod nth;
mod pseudo;

pub use caches::{CacheWork, SelectorCaches};
use matcher::Cx;
#[cfg(test)]
mod form_state_tests;
#[cfg(test)]
mod has_cost_tests;
#[cfg(test)]
mod has_tests;
#[cfg(test)]
mod linguistic_tests;
#[cfg(test)]
mod tests;

impl<Ext> Dom<Ext> {
    /// Whether `id` matches `:placeholder-shown`: it has a non-empty
    /// `placeholder` attribute and no text content, i.e. it is showing
    /// its placeholder hint. Also what decides whether `::placeholder`
    /// rules style anything.
    pub fn is_placeholder_shown(&self, id: NodeId) -> bool {
        self.get_attribute(id, "placeholder")
            .is_some_and(|v| !v.is_empty())
            && self.text_content(id).is_empty()
    }

    /// Find the first descendant of `root_id` matching `selector`, in
    /// document order. Returns `None` if none matches. Errors if the
    /// selector is malformed.
    ///
    /// The DOM-shaped one-arg form is [`Dom::query_selector`]; this
    /// `_in` form is the explicit-root variant (M4b step 18 rename).
    pub fn query_selector_in(
        &self,
        root_id: NodeId,
        selector: &str,
    ) -> Result<Option<NodeId>, ParseError> {
        let list = selectors::parse(selector)?;
        let mut caches = SelectorCaches::new();
        let mut found = None;
        self.walk_descendants(root_id, &mut |id, data| {
            if found.is_some() {
                return;
            }
            if let NodeData::Element { .. } = data
                && self.matches_list_with(id, &list, Some(root_id), &mut caches)
            {
                found = Some(id);
            }
        });
        Ok(found)
    }

    /// All descendants of `root_id` matching `selector`, in document
    /// order. The DOM-shaped one-arg form is
    /// [`Dom::query_selector_all`]; this `_in` form is the
    /// explicit-root variant (M4b step 18 rename).
    pub fn query_selector_all_in(
        &self,
        root_id: NodeId,
        selector: &str,
    ) -> Result<Vec<NodeId>, ParseError> {
        let list = selectors::parse(selector)?;
        let mut caches = SelectorCaches::new();
        let mut out = Vec::new();
        self.walk_descendants(root_id, &mut |id, data| {
            if matches!(data, NodeData::Element { .. })
                && self.matches_list_with(id, &list, Some(root_id), &mut caches)
            {
                out.push(id);
            }
        });
        Ok(out)
    }

    /// Does `id` match `selector`? Errors on malformed selector.
    pub fn matches(&self, id: NodeId, selector: &str) -> Result<bool, ParseError> {
        let list = selectors::parse(selector)?;
        Ok(self.matches_list_in_scope(id, &list, Some(id)))
    }

    /// Walk from `id` (inclusive) up the tree and return the first ancestor
    /// that matches `selector`. `None` if none does.
    pub fn closest(&self, id: NodeId, selector: &str) -> Result<Option<NodeId>, ParseError> {
        let list = selectors::parse(selector)?;
        let mut caches = SelectorCaches::new();
        let mut cur = Some(id);
        while let Some(c) = cur {
            if matches!(
                self.get_node(c).map(|n| &n.data),
                Some(NodeData::Element { .. })
            ) && self.matches_list_with(c, &list, Some(id), &mut caches)
            {
                return Ok(Some(c));
            }
            cur = self.get_node(c).and_then(|n| n.parent);
        }
        Ok(None)
    }

    // ─── Matcher ─────────────────────────────────────────────────────

    /// Does `id` match any selector in the pre-parsed `list`? Public so
    /// downstream crates (rdom-tui's cascade) can drive rule matching
    /// without re-parsing selector strings on every call.
    pub fn matches_list(&self, id: NodeId, list: &SelectorList) -> bool {
        self.matches_list_in_scope(id, list, None)
    }

    /// [`Self::matches_list`] with `scope` as the scoping root `:scope`
    /// matches (Selectors 4 §14.3) — an `@scope` rule's root (CSS
    /// Cascade 6 §2.5). With `None` there is no scoping root and
    /// `:scope` is `:root`.
    pub fn matches_list_in_scope(
        &self,
        id: NodeId,
        list: &SelectorList,
        scope: Option<NodeId>,
    ) -> bool {
        self.matches_list_with(id, list, scope, &mut SelectorCaches::new())
    }

    /// [`Self::matches_list_in_scope`] with the caches of the matching
    /// pass this call belongs to ([`SelectorCaches`]): a cascade or a
    /// query matching many elements shares one, so `:nth-child()` and
    /// its family index each sibling list once per pass instead of
    /// counting siblings per element. The result is the same with fresh
    /// caches.
    pub fn matches_list_with(
        &self,
        id: NodeId,
        list: &SelectorList,
        scope: Option<NodeId>,
        caches: &mut SelectorCaches,
    ) -> bool {
        caches.sync(self.mutation_epoch);
        self.matches_list_cx(id, list, &mut Cx { scope, caches })
    }
}
