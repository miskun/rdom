//! `DomTokenList` + `DomTokenListMut` — DOM-faithful snapshot
//! wrapper for ordered string-token attributes (today: `class`,
//! tomorrow: `rel`, `sandbox`, …).
//!
//! ## Live vs snapshot
//!
//! Per the M4 scope lock (parity ledger §25 #2), wrapper **shape**
//! ships but **liveness** does not. [`DomTokenList`] takes a
//! snapshot of the tokens at construction; re-call the accessor on
//! the element to refresh. Listeners that need to react to class
//! changes use `MutationObserver`.
//!
//! ## API shape
//!
//! Read side ([`DomTokenList`]) — `len`, `is_empty`, `item(i)`,
//! `value()`, `contains(token)`, `iter()`.
//!
//! Write side ([`DomTokenListMut`]) — `add`, `remove`,
//! `toggle(token, force)`, `replace(old, new)`, `supports(token)`.
//! All mutations route through the existing
//! `add_class` / `remove_class` / `replace_class` paths on
//! `Dom<Ext>` so cascade invalidation, mutation events, and the
//! per-class index stay in lockstep.
//!
//! The author-facing constructors `NodeRef::class_list()` /
//! `NodeMut::class_list_mut()` ship in M4b step 14 / 15; this
//! module's types are public and accept a `NodeMut` in their
//! constructor so tests can exercise the shape today.

use crate::accessor::NodeMut;
use crate::error::Result;

/// Snapshot of an element's ordered class tokens (DOM
/// `Element.classList` read-side view). Re-construct via the
/// element accessor to refresh after a class mutation.
///
/// `tokens` preserves the iteration order of the underlying
/// class set; rdom-core stores classes in a `BTreeSet`, so
/// iteration is alphabetic. Browsers preserve insertion order;
/// this is a documented divergence (the same one `class_list()`
/// has carried since Phase 1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DomTokenList {
    tokens: Vec<String>,
}

impl DomTokenList {
    /// Construct from an iterator of class tokens. Used by the
    /// `NodeRef::class_list()` integration (M4b step 14); exposed
    /// publicly so tests and other consumers can build snapshots
    /// directly.
    pub fn from_tokens(tokens: impl IntoIterator<Item = String>) -> Self {
        Self {
            tokens: tokens.into_iter().collect(),
        }
    }

    /// Number of tokens in the snapshot. DOM `length`.
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// `true` iff the snapshot has no tokens.
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Borrow the token at `index`, or `None` if out of bounds.
    /// DOM `item(i)`.
    pub fn item(&self, index: usize) -> Option<&str> {
        self.tokens.get(index).map(String::as_str)
    }

    /// Re-serialize the snapshot as a space-joined string. DOM
    /// `value` / `toString()`.
    pub fn value(&self) -> String {
        self.tokens.join(" ")
    }

    /// `true` iff the snapshot contains `token`. DOM `contains`.
    pub fn contains(&self, token: &str) -> bool {
        self.tokens.iter().any(|t| t == token)
    }

    /// Iterate token strings in snapshot order.
    pub fn iter(&self) -> impl Iterator<Item = &str> + '_ {
        self.tokens.iter().map(String::as_str)
    }
}

/// Mutable handle to an element's class tokens. Holds a `NodeMut`
/// so mutations route through the existing class-set machinery
/// (`add_class` / `remove_class` / `replace_class`), keeping
/// cascade dirty-tracking and the per-class index in sync.
pub struct DomTokenListMut<'a, Ext: 'static> {
    node: NodeMut<'a, Ext>,
}

impl<'a, Ext: 'static> DomTokenListMut<'a, Ext> {
    /// Construct from a borrowed `NodeMut`. The mutable list shares
    /// the node's lifetime — drop the list to release the borrow.
    pub fn new(node: NodeMut<'a, Ext>) -> Self {
        Self { node }
    }

    /// Add `token` to the class set. Idempotent — no-op if the
    /// token is already present. DOM `add`.
    pub fn add(&mut self, token: &str) -> Result<()> {
        self.node.add_class(token)
    }

    /// Remove `token` from the class set. Returns `true` iff a
    /// token was actually removed (`false` when the token wasn't
    /// present). DOM `remove`.
    pub fn remove(&mut self, token: &str) -> Result<bool> {
        self.node.remove_class(token)
    }

    /// Toggle `token` with optional force.
    ///
    /// - `force = Some(true)` → ensure present; returns `true`.
    /// - `force = Some(false)` → ensure absent; returns `false`.
    /// - `force = None` → flip; returns the post-flip presence.
    ///
    /// DOM `toggle(token, force)`.
    pub fn toggle(&mut self, token: &str, force: Option<bool>) -> Result<bool> {
        let has = self.contains(token);
        match force {
            Some(true) => {
                if !has {
                    self.add(token)?;
                }
                Ok(true)
            }
            Some(false) => {
                if has {
                    self.remove(token)?;
                }
                Ok(false)
            }
            None => {
                if has {
                    self.remove(token)?;
                    Ok(false)
                } else {
                    self.add(token)?;
                    Ok(true)
                }
            }
        }
    }

    /// Replace `old` with `new`. Returns `true` iff the
    /// replacement happened (i.e., `old` was present). DOM
    /// `replace(old, new)`.
    pub fn replace(&mut self, old: &str, new: &str) -> Result<bool> {
        self.node.replace_class(old, new)
    }

    /// `true` iff `token` is in the supported-token list for this
    /// attribute. DOM spec only defines this for specific
    /// element-attribute pairs (`<link>.relList`,
    /// `<iframe>.sandbox`, …); for `classList` and everything else
    /// rdom doesn't track, returns `false`. DOM `supports`.
    pub fn supports(&self, _token: &str) -> bool {
        false
    }

    /// Borrow the current token set without releasing the `NodeMut`.
    pub fn contains(&self, token: &str) -> bool {
        self.node.as_ref().has_class(token)
    }

    /// Number of tokens currently on the element. DOM `length`.
    pub fn len(&self) -> usize {
        self.node.dom.class_list(self.node.id).count()
    }

    /// `true` iff the element has no class tokens.
    pub fn is_empty(&self) -> bool {
        self.node.dom.class_list(self.node.id).next().is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dom;

    // ── DomTokenList (read-side snapshot) ─────────────────────────────

    #[test]
    fn snapshot_from_tokens_preserves_input_order() {
        let list = DomTokenList::from_tokens(vec!["a".into(), "b".into(), "c".into()]);
        assert_eq!(list.len(), 3);
        assert!(!list.is_empty());
        assert_eq!(list.item(0), Some("a"));
        assert_eq!(list.item(1), Some("b"));
        assert_eq!(list.item(2), Some("c"));
        assert_eq!(list.item(3), None);
    }

    #[test]
    fn snapshot_value_is_space_joined() {
        let list = DomTokenList::from_tokens(vec!["foo".into(), "bar".into()]);
        assert_eq!(list.value(), "foo bar");
    }

    #[test]
    fn empty_snapshot_value_is_empty_string() {
        let list = DomTokenList::from_tokens(Vec::<String>::new());
        assert!(list.is_empty());
        assert_eq!(list.value(), "");
    }

    #[test]
    fn snapshot_contains_iter() {
        let list = DomTokenList::from_tokens(vec!["x".into(), "y".into()]);
        assert!(list.contains("x"));
        assert!(list.contains("y"));
        assert!(!list.contains("z"));
        let collected: Vec<&str> = list.iter().collect();
        assert_eq!(collected, ["x", "y"]);
    }

    // ── DomTokenListMut (mutating side) ──────────────────────────────

    fn element_with_classes(classes: &[&str]) -> (Dom, crate::node_id::NodeId) {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        for c in classes {
            dom.add_class(el, c).unwrap();
        }
        (dom, el)
    }

    #[test]
    fn add_is_idempotent() {
        let (mut dom, el) = element_with_classes(&[]);
        let mut list = DomTokenListMut::new(dom.node_mut(el));
        list.add("foo").unwrap();
        list.add("foo").unwrap();
        assert!(list.contains("foo"));
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn remove_returns_true_when_present_false_when_absent() {
        let (mut dom, el) = element_with_classes(&["foo"]);
        let mut list = DomTokenListMut::new(dom.node_mut(el));
        assert!(list.remove("foo").unwrap());
        assert!(!list.remove("foo").unwrap());
        assert!(!list.contains("foo"));
    }

    #[test]
    fn toggle_with_force_true_is_add_idempotent() {
        // Canonical step-10 failing test: `toggle(name, Some(true))`
        // must be force-add — present-or-not, the post-state is
        // present, and the returned boolean is `true`.
        let (mut dom, el) = element_with_classes(&["foo"]);
        let mut list = DomTokenListMut::new(dom.node_mut(el));

        // Already present: force-add returns true, no removal.
        assert!(list.toggle("foo", Some(true)).unwrap());
        assert!(list.contains("foo"));

        // Not present: force-add adds it, returns true.
        assert!(list.toggle("bar", Some(true)).unwrap());
        assert!(list.contains("bar"));
    }

    #[test]
    fn toggle_with_force_false_is_remove_idempotent() {
        let (mut dom, el) = element_with_classes(&["foo"]);
        let mut list = DomTokenListMut::new(dom.node_mut(el));

        // Present: force-remove returns false, token gone.
        assert!(!list.toggle("foo", Some(false)).unwrap());
        assert!(!list.contains("foo"));

        // Not present: force-remove returns false, still gone.
        assert!(!list.toggle("foo", Some(false)).unwrap());
    }

    #[test]
    fn toggle_with_no_force_flips_state() {
        let (mut dom, el) = element_with_classes(&[]);
        let mut list = DomTokenListMut::new(dom.node_mut(el));

        // Not present → toggle → present, returns true.
        assert!(list.toggle("foo", None).unwrap());
        assert!(list.contains("foo"));

        // Present → toggle → absent, returns false.
        assert!(!list.toggle("foo", None).unwrap());
        assert!(!list.contains("foo"));
    }

    #[test]
    fn replace_swaps_existing_token() {
        let (mut dom, el) = element_with_classes(&["foo", "bar"]);
        let mut list = DomTokenListMut::new(dom.node_mut(el));
        assert!(list.replace("foo", "baz").unwrap());
        assert!(!list.contains("foo"));
        assert!(list.contains("baz"));
        assert!(list.contains("bar"));
    }

    #[test]
    fn replace_returns_false_when_old_absent() {
        let (mut dom, el) = element_with_classes(&["foo"]);
        let mut list = DomTokenListMut::new(dom.node_mut(el));
        assert!(!list.replace("missing", "new").unwrap());
        assert!(!list.contains("new"));
    }

    #[test]
    fn supports_always_false_for_classlist() {
        let (mut dom, el) = element_with_classes(&[]);
        let list = DomTokenListMut::new(dom.node_mut(el));
        // classList has no supported-token list; spec requires false.
        assert!(!list.supports("foo"));
        assert!(!list.supports("anything"));
    }

    #[test]
    fn len_and_is_empty_reflect_current_state() {
        let (mut dom, el) = element_with_classes(&[]);
        let mut list = DomTokenListMut::new(dom.node_mut(el));
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);

        list.add("a").unwrap();
        list.add("b").unwrap();
        assert!(!list.is_empty());
        assert_eq!(list.len(), 2);
    }
}
