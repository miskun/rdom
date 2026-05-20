//! `Dom<Ext>` — the arena.
//!
//! Owns all nodes in a flat `Vec<Option<Node<Ext>>>`. Tree structure is
//! expressed via `NodeId` fields inside each `Node`. Slots freed by
//! `remove_child` are recycled LIFO via a free list.

use std::collections::{BTreeMap, BTreeSet};

use crate::accessor::NodeRef;
use crate::dispatch::ListenerStore;
use crate::error::{DomError, Result};
use crate::indexes::Indexes;
use crate::node::{Node, NodeData, NodeType};
use crate::node_id::NodeId;
use crate::node_list::NodeList;
use crate::observer::{InteractionKind, Mutation, ObserverStore};
use crate::selection::{Position, Range};

/// The root of a DOM tree. One instance per document.
///
/// Not `Clone`: event listeners are stored as `Box<dyn FnMut>` closures
/// which cannot be cloned. Use `clone_node(root, true)` if you need a
/// structural copy of the tree (attrs/classes/children — listeners are
/// explicitly excluded, matching browser semantics).
///
/// `Ext: 'static` is required because event listeners and mutation
/// observers are stored as `Box<dyn … + 'static>` trait objects —
/// non-'static Ext types couldn't be boxed that way.
#[derive(Debug)]
pub struct Dom<Ext: 'static = ()> {
    /// Arena storage. `None` = freed slot awaiting reuse.
    pub(crate) nodes: Vec<Option<Node<Ext>>>,
    /// Free-slot indices, LIFO (cache-friendly reuse).
    pub(crate) free: Vec<u32>,
    /// The root node. Created at `Dom::new`; identity is stable for the
    /// lifetime of the `Dom`.
    pub(crate) root: NodeId,
    /// O(1) indexes for id / tag / class lookups. Kept in sync with every
    /// mutation via `hook_register` / `hook_unregister` and the attr/class
    /// accessors in `attrs.rs`.
    pub(crate) indexes: Indexes,
    /// Event listener side-storage. Only nodes with at least one listener
    /// appear in the map — sparse trees pay nothing.
    pub(crate) listeners: ListenerStore<Ext>,
    /// Current interaction state — consulted by the selector matcher for
    /// `:hover` and `:focus` pseudo-classes. `None` means "nothing
    /// hovered" / "nothing focused". Mutators (`set_hovered` /
    /// `set_focused`) are where Phase 7.4's MutationObserver will hook
    /// in to fire `InteractionChanged` records.
    pub(crate) hovered: Option<NodeId>,
    pub(crate) focused: Option<NodeId>,
    /// The element that currently owns the pointer, via
    /// `set_pointer_capture`. While set, the runtime routes every
    /// `mousemove`, drag, and `mouseup` to this element regardless of
    /// which element the cursor is over. Auto-released on the next
    /// `mouseup` (browser-faithful). No mutation record fires — this
    /// state doesn't affect cascade / selectors.
    pub(crate) pointer_capture: Option<NodeId>,
    /// Document-level text selection. `None` = nothing selected.
    /// Mutations fire `Mutation::SelectionChanged`. Paint observers
    /// use those records to refresh the `::selection` overlay.
    pub(crate) selection: Option<crate::Selection>,
    /// Mutation observers. Fires `Mutation` records on every DOM change.
    pub(crate) observers: ObserverStore<Ext>,
    /// Re-entrancy guard: true while an observer callback is running.
    /// Mutations attempted during that window panic with a clear message.
    pub(crate) is_observing: bool,
}

impl<Ext: Default> Default for Dom<Ext> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Ext: Default> Dom<Ext> {
    /// Create a new arena with a `<document-fragment>` root. Use
    /// `with_root_tag` if you want the root to be a specific element tag.
    pub fn new() -> Self {
        let root_node: Node<Ext> = Node::new(NodeData::Fragment);
        let nodes = vec![Some(root_node)];
        let root = NodeId::from_index(0);
        Self {
            nodes,
            free: Vec::new(),
            root,
            indexes: Indexes::default(),
            listeners: ListenerStore::default(),
            hovered: None,
            focused: None,
            pointer_capture: None,
            selection: None,
            observers: ObserverStore::default(),
            is_observing: false,
        }
    }
}

impl<Ext> Dom<Ext> {
    /// The node currently flagged as hovered (see `:hover`). `None` when
    /// nothing is hovered. Matching consults this field directly.
    pub fn hovered(&self) -> Option<NodeId> {
        self.hovered
    }

    /// The node currently flagged as focused (see `:focus`).
    pub fn focused(&self) -> Option<NodeId> {
        self.focused
    }

    /// The node that currently owns the pointer via
    /// [`set_pointer_capture`](Self::set_pointer_capture). `None`
    /// (the default) means routing uses hit-testing as usual.
    pub fn pointer_capture(&self) -> Option<NodeId> {
        self.pointer_capture
    }

    /// The document's text selection, if any. `None` means no
    /// selection (not even a caret). Mutated via
    /// [`set_selection`](Self::set_selection) which fires
    /// `Mutation::SelectionChanged` for paint observers.
    pub fn selection(&self) -> Option<&crate::Selection> {
        self.selection.as_ref()
    }

    /// The current selection normalized to a document-ordered
    /// `Range` — `start` precedes `end` per `compare_document_position`.
    /// Useful for paint + copy walks that need ordered traversal.
    ///
    /// Returns `None` when nothing is selected OR when the
    /// anchor/focus nodes are disconnected (shouldn't happen in
    /// practice, but handled defensively).
    pub fn selection_range(&self) -> Option<crate::Range> {
        let sel = self.selection.as_ref()?;
        let a = sel.anchor;
        let f = sel.focus;
        if a.node == f.node {
            // Same node: order by offset.
            let (start, end) = if a.offset <= f.offset { (a, f) } else { (f, a) };
            return Some(crate::Range::ordered_unchecked(start, end));
        }
        use crate::position::DocumentPosition;
        let pos = self.compare_document_position(a.node, f.node);
        if pos.contains(DocumentPosition::DISCONNECTED) {
            return None;
        }
        // CONTAINS/PRECEDING: a comes before b (a is ancestor-or-sibling-before).
        // CONTAINED_BY/FOLLOWING: a comes after b.
        let a_first =
            pos.contains(DocumentPosition::FOLLOWING) || pos.contains(DocumentPosition::CONTAINS);
        let (start, end) = if a_first { (a, f) } else { (f, a) };
        Some(crate::Range::ordered_unchecked(start, end))
    }
}

impl<Ext: 'static> Dom<Ext> {
    /// Set or clear the hovered node. Fires an `InteractionChanged`
    /// mutation record when the value actually changes; no-op when
    /// setting to the current value.
    pub fn set_hovered(&mut self, id: Option<NodeId>) {
        if self.hovered == id {
            return;
        }
        let prev = self.hovered;
        self.hovered = id;
        self.fire_mutation(Mutation::InteractionChanged {
            prev,
            next: id,
            kind: InteractionKind::Hover,
        });
    }

    /// Set or clear the focused node. Fires an `InteractionChanged`
    /// record on change.
    pub fn set_focused(&mut self, id: Option<NodeId>) {
        if self.focused == id {
            return;
        }
        let prev = self.focused;
        self.focused = id;
        self.fire_mutation(Mutation::InteractionChanged {
            prev,
            next: id,
            kind: InteractionKind::Focus,
        });
    }

    /// Claim the pointer for `id`. While set, the runtime routes
    /// `mousemove` / drag / `mouseup` to `id` regardless of where
    /// the cursor lands — critical for drag-select, resize
    /// handles, scrubbing.
    ///
    /// Typical usage from a `mousedown` listener:
    ///
    /// ```ignore
    /// dom.add_event_listener(handle, "mousedown",
    ///     ListenerOptions::default(), |ctx| {
    ///         let target = ctx.event.target.unwrap();
    ///         ctx.dom.set_pointer_capture(target).unwrap();
    ///     })?;
    /// ```
    ///
    /// The capture releases automatically on `mouseup`, or can be
    /// released explicitly via [`release_pointer_capture`].
    ///
    /// Returns `Err(DomError::InvalidNode)` if `id` doesn't exist.
    /// Does **not** fire a mutation record — pointer capture
    /// doesn't affect cascade / selectors (no `:pointer-captured`
    /// pseudo in v1).
    ///
    /// [`release_pointer_capture`]: Self::release_pointer_capture
    pub fn set_pointer_capture(&mut self, id: NodeId) -> crate::Result<()> {
        self.node_or_err(id)?;
        self.pointer_capture = Some(id);
        Ok(())
    }

    /// Release any active pointer capture. Idempotent — no-op when
    /// nothing was captured.
    pub fn release_pointer_capture(&mut self) {
        self.pointer_capture = None;
    }

    /// Set the document selection. `None` clears it.
    ///
    /// Fires `Mutation::SelectionChanged { prev, next }` on change
    /// so paint observers can refresh the `::selection` overlay.
    /// No-op when `next == current selection`.
    pub fn set_selection(&mut self, next: Option<crate::Selection>) {
        if self.selection == next {
            return;
        }
        let prev = self.selection.take();
        self.selection = next;
        self.fire_mutation(Mutation::SelectionChanged { prev, next });
    }

    // ── Dom-level shortcuts ──────────────────────────────────────

    /// Find the first element in the document matching `selector`,
    /// in document order. DOM `Document.querySelector`.
    ///
    /// Document-rooted shortcut for the more general
    /// [`Self::query_selector_in`]. Malformed selectors return
    /// `None` (browser-DOM throws; rdom diverges per §9.1 spec
    /// table).
    pub fn query_selector(&self, selector: &str) -> Option<NodeRef<'_, Ext>> {
        self.query_selector_in(self.root, selector)
            .ok()
            .flatten()
            .map(|id| self.node(id))
    }

    /// All elements in the document matching `selector`, in
    /// document order. DOM `Document.querySelectorAll`.
    ///
    /// Document-rooted shortcut for
    /// [`Self::query_selector_all_in`]. Malformed selectors yield
    /// an empty list.
    pub fn query_selector_all(&self, selector: &str) -> NodeList<'_, Ext> {
        let ids = self
            .query_selector_all_in(self.root, selector)
            .unwrap_or_default();
        NodeList::from_ids(self, ids)
    }

    /// All elements in the document with the given tag name, in
    /// document order. DOM `Document.getElementsByTagName`.
    ///
    /// Returns a snapshot — unlike browser's live HTMLCollection
    /// (Lock #2, parity ledger §25). Tag comparison is case-
    /// sensitive (rdom tags are lowercased at parse time, so
    /// authors pass lowercase).
    pub fn elements_by_tag(&self, tag: &str) -> NodeList<'_, Ext> {
        let mut out: Vec<NodeId> = Vec::new();
        self.walk_descendants(self.root, &mut |id, data| {
            if let NodeData::Element { tag: t, .. } = data
                && t == tag
            {
                out.push(id);
            }
        });
        NodeList::from_ids(self, out)
    }

    /// The document element. DOM `Document.documentElement`.
    ///
    /// When the root is an element (typically `<html>` via
    /// `Dom::with_root_tag("html")`), this is the root itself.
    /// When the root is a Fragment (the default), returns the
    /// first element child of the fragment, or the fragment
    /// itself if it has no element children.
    pub fn document_element(&self) -> NodeRef<'_, Ext> {
        let root = self.root;
        if matches!(
            self.get_node(root).map(|n| &n.data),
            Some(NodeData::Fragment)
        ) && let Some(first_el) = self.node(root).first_element_child()
        {
            return first_el;
        }
        self.node(root)
    }

    /// The currently focused element. DOM `Document.activeElement`.
    /// Alias for [`Self::focused`] returning a `NodeRef`.
    pub fn active_element(&self) -> Option<NodeRef<'_, Ext>> {
        self.focused.map(|id| self.node(id))
    }

    /// `true` iff the document has a focused element. DOM
    /// `Document.hasFocus`. (rdom has a single document; the
    /// browser semantics of "focused window" don't apply.)
    pub fn has_focus(&self) -> bool {
        self.focused.is_some()
    }

    /// Construct an empty `Range` collapsed at the document root,
    /// offset 0. DOM `Document.createRange()`.
    ///
    /// The returned range can be re-anchored via struct-literal
    /// assignment or [`Range::ordered_unchecked`]. Range boundary
    /// setters (`setStart` / `setEnd`) are polish.
    pub fn create_range(&self) -> Range {
        let pos = Position::new(self.root, 0);
        Range::ordered_unchecked(pos, pos)
    }
}

impl<Ext: Default> Dom<Ext> {
    /// Create a new arena with a named element as the root.
    pub fn with_root_tag(tag: &str) -> Self {
        let root_node: Node<Ext> = Node::new(NodeData::Element {
            tag: tag.to_string(),
            attrs: BTreeMap::new(),
            classes: BTreeSet::new(),
            ext: Ext::default(),
        });
        let nodes = vec![Some(root_node)];
        let root = NodeId::from_index(0);
        let mut dom = Self {
            nodes,
            free: Vec::new(),
            root,
            indexes: Indexes::default(),
            listeners: ListenerStore::default(),
            hovered: None,
            focused: None,
            pointer_capture: None,
            selection: None,
            observers: ObserverStore::default(),
            is_observing: false,
        };
        dom.hook_register(root);
        dom
    }

    /// Create a new Element node. Orphan — not attached to any parent.
    /// Use `append_child` to attach it.
    pub fn create_element(&mut self, tag: &str) -> NodeId {
        self.alloc(Node::new(NodeData::Element {
            tag: tag.to_string(),
            attrs: BTreeMap::new(),
            classes: BTreeSet::new(),
            ext: Ext::default(),
        }))
    }
}

impl<Ext> Dom<Ext> {
    /// Create an Element with explicit extension data (useful when `Ext`
    /// doesn't implement `Default` or you want to pre-populate state).
    pub fn create_element_with_ext(&mut self, tag: &str, ext: Ext) -> NodeId {
        self.alloc(Node::new(NodeData::Element {
            tag: tag.to_string(),
            attrs: BTreeMap::new(),
            classes: BTreeSet::new(),
            ext,
        }))
    }

    /// Create a Text node (content of a text child).
    pub fn create_text_node(&mut self, data: &str) -> NodeId {
        self.alloc(Node::new(NodeData::Text {
            data: data.to_string(),
        }))
    }

    /// Create a Comment node.
    pub fn create_comment(&mut self, data: &str) -> NodeId {
        self.alloc(Node::new(NodeData::Comment {
            data: data.to_string(),
        }))
    }

    /// Create a DocumentFragment. Used as a detachable subtree container;
    /// inserting a fragment unwraps it.
    pub fn create_document_fragment(&mut self) -> NodeId {
        self.alloc(Node::new(NodeData::Fragment))
    }

    /// The root node of this arena.
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// Does the arena currently hold this id?
    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes
            .get(id.index())
            .map(|slot| slot.is_some())
            .unwrap_or(false)
    }

    /// How many live nodes are in the arena (excludes freed slots).
    pub fn len(&self) -> usize {
        self.nodes.len() - self.free.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // ── Internal ─────────────────────────────────────────────────────

    /// Allocate a slot for `node`. Reuses a freed slot if available.
    /// Registers the node in the id/tag/class indexes if it's an Element.
    pub(crate) fn alloc(&mut self, node: Node<Ext>) -> NodeId {
        let new_id = if let Some(idx) = self.free.pop() {
            self.nodes[idx as usize] = Some(node);
            NodeId::from_index(idx as usize)
        } else {
            let idx = self.nodes.len();
            self.nodes.push(Some(node));
            NodeId::from_index(idx)
        };
        self.hook_register(new_id);
        new_id
    }

    /// Mark a slot as freed. Caller must ensure the node is already
    /// detached (unlinked from parent/siblings). Unregisters from every
    /// index and drops any attached listeners before the slot is wiped.
    pub(crate) fn free(&mut self, id: NodeId) {
        let idx = id.index();
        if idx < self.nodes.len() && self.nodes[idx].is_some() {
            self.hook_unregister(id);
            self.drop_listeners(id);
            self.nodes[idx] = None;
            self.free.push(idx as u32);
        }
    }

    /// Shared-ref node access; `None` if slot is freed or out of bounds.
    pub(crate) fn get_node(&self, id: NodeId) -> Option<&Node<Ext>> {
        self.nodes.get(id.index()).and_then(|slot| slot.as_ref())
    }

    /// Mutable node access.
    pub(crate) fn get_node_mut(&mut self, id: NodeId) -> Option<&mut Node<Ext>> {
        self.nodes
            .get_mut(id.index())
            .and_then(|slot| slot.as_mut())
    }

    /// Node access that errors on invalid id (use when the caller expects
    /// it to exist — e.g. caller just created it, or it's a parent/child
    /// pointer we trust).
    pub(crate) fn node_or_err(&self, id: NodeId) -> Result<&Node<Ext>> {
        self.get_node(id).ok_or(DomError::InvalidNode(id))
    }

    pub(crate) fn node_mut_or_err(&mut self, id: NodeId) -> Result<&mut Node<Ext>> {
        self.get_node_mut(id).ok_or(DomError::InvalidNode(id))
    }

    /// Check whether `ancestor` is an ancestor of `descendant` (or the
    /// same node). Used for cycle detection in tree mutations.
    pub(crate) fn is_ancestor(&self, ancestor: NodeId, descendant: NodeId) -> bool {
        let mut current = Some(descendant);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.get_node(id).and_then(|n| n.parent);
        }
        false
    }

    /// Node type — handy enough to hoist to Dom level.
    pub fn node_type(&self, id: NodeId) -> Option<NodeType> {
        self.get_node(id).map(|n| n.node_type())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_dom_has_fragment_root() {
        let dom: Dom = Dom::new();
        let root = dom.root();
        assert!(dom.contains(root));
        assert_eq!(dom.node_type(root), Some(NodeType::Fragment));
        assert_eq!(dom.len(), 1);
    }

    #[test]
    fn with_root_tag_creates_element_root() {
        let dom: Dom = Dom::with_root_tag("body");
        let root = dom.root();
        let n = dom.get_node(root).unwrap();
        assert_eq!(n.node_type(), NodeType::Element);
        assert_eq!(n.tag_name(), Some("body"));
    }

    #[test]
    fn create_element_is_orphan() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let n = dom.get_node(el).unwrap();
        assert!(n.parent.is_none());
        assert!(n.first_child.is_none());
        assert_eq!(n.node_type(), NodeType::Element);
    }

    #[test]
    fn create_allocates_distinct_ids() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        let c = dom.create_text_node("hi");
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
        assert_eq!(dom.len(), 4); // root + 3
    }

    #[test]
    fn freed_slot_gets_reused() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("a");
        assert!(dom.contains(a));
        dom.free(a);
        assert!(!dom.contains(a));

        // Next allocation reuses the same slot.
        let b = dom.create_element("b");
        assert_eq!(a.index(), b.index());
    }

    #[test]
    fn invalid_id_returns_error() {
        let dom: Dom = Dom::new();
        let ghost = NodeId::from_index(999);
        assert!(matches!(
            dom.node_or_err(ghost).unwrap_err(),
            DomError::InvalidNode(_)
        ));
    }

    #[test]
    fn is_ancestor_detects_self() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("a");
        // Without attachment, the node is its own ancestor (descending
        // from itself) — walker starts at `descendant` and matches.
        assert!(dom.is_ancestor(a, a));
    }

    // ── pointer_capture ──────────────────────────────────────────────

    #[test]
    fn pointer_capture_none_by_default() {
        let dom: Dom = Dom::new();
        assert_eq!(dom.pointer_capture(), None);
    }

    #[test]
    fn set_pointer_capture_records_node() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("handle");
        dom.set_pointer_capture(el).unwrap();
        assert_eq!(dom.pointer_capture(), Some(el));
    }

    #[test]
    fn set_pointer_capture_invalid_node_errors() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("x");
        dom.free(el);
        assert!(dom.set_pointer_capture(el).is_err());
        assert_eq!(dom.pointer_capture(), None);
    }

    #[test]
    fn release_pointer_capture_clears_state() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("handle");
        dom.set_pointer_capture(el).unwrap();
        dom.release_pointer_capture();
        assert_eq!(dom.pointer_capture(), None);
    }

    #[test]
    fn release_pointer_capture_is_idempotent() {
        let mut dom: Dom = Dom::new();
        // Releasing without a prior capture is a no-op, not an error.
        dom.release_pointer_capture();
        dom.release_pointer_capture();
        assert_eq!(dom.pointer_capture(), None);
    }

    #[test]
    fn set_pointer_capture_replaces_previous() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        dom.set_pointer_capture(a).unwrap();
        dom.set_pointer_capture(b).unwrap();
        assert_eq!(dom.pointer_capture(), Some(b));
    }

    // ── selection ────────────────────────────────────────────────────

    #[test]
    fn selection_none_by_default() {
        let dom: Dom = Dom::new();
        assert_eq!(dom.selection(), None);
        assert_eq!(dom.selection_range(), None);
    }

    #[test]
    fn set_selection_stores_value() {
        use crate::{Position, Selection};
        let mut dom: Dom = Dom::new();
        let t = dom.create_text_node("hello");
        let sel = Selection::new(Position::new(t, 1), Position::new(t, 4));
        dom.set_selection(Some(sel));
        assert_eq!(dom.selection().copied(), Some(sel));
    }

    #[test]
    fn set_selection_none_clears() {
        use crate::{Position, Selection};
        let mut dom: Dom = Dom::new();
        let t = dom.create_text_node("hello");
        dom.set_selection(Some(Selection::caret(Position::new(t, 0))));
        dom.set_selection(None);
        assert_eq!(dom.selection(), None);
    }

    #[test]
    fn selection_range_same_node_orders_by_offset() {
        use crate::{Position, Range, Selection};
        let mut dom: Dom = Dom::new();
        let t = dom.create_text_node("hello");
        // Inverted selection (anchor after focus in byte order).
        dom.set_selection(Some(Selection::new(
            Position::new(t, 4),
            Position::new(t, 1),
        )));
        let r = dom.selection_range().unwrap();
        assert_eq!(
            r,
            Range::ordered_unchecked(Position::new(t, 1), Position::new(t, 4))
        );
    }

    #[test]
    fn selection_range_different_nodes_orders_by_document_position() {
        use crate::{Position, Selection};
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let t1 = dom.create_text_node("first");
        let t2 = dom.create_text_node("second");
        dom.append_child(root, t1).unwrap();
        dom.append_child(root, t2).unwrap();

        // Selection from t2 → t1 (inverted).
        dom.set_selection(Some(Selection::new(
            Position::new(t2, 2),
            Position::new(t1, 3),
        )));
        let r = dom.selection_range().unwrap();
        assert_eq!(r.start.node, t1);
        assert_eq!(r.end.node, t2);
    }

    // ── M4b step 18: Dom-level accessor additions ─────────────────────

    #[test]
    fn query_selector_one_arg_runs_from_root() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("p");
        dom.node_mut(a).add_class("hit").unwrap();
        let b = dom.create_element("p");
        dom.node_mut(b).add_class("hit").unwrap();
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        let hit = dom.query_selector(".hit").unwrap();
        assert_eq!(hit.id(), a);
    }

    #[test]
    fn query_selector_one_arg_returns_none_on_invalid_selector() {
        let dom: Dom = Dom::new();
        assert!(dom.query_selector("!!!").is_none());
    }

    #[test]
    fn query_selector_all_one_arg_returns_doc_order_node_list() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("p");
        let b = dom.create_element("p");
        let c = dom.create_element("p");
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        dom.append_child(root, c).unwrap();
        let list = dom.query_selector_all("p");
        assert_eq!(list.len(), 3);
        let ids: Vec<NodeId> = list.iter().map(|n| n.id()).collect();
        assert_eq!(ids, vec![a, b, c]);
    }

    #[test]
    fn elements_by_tag_returns_node_list_in_doc_order() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        let span1 = dom.create_element("span");
        let span2 = dom.create_element("span");
        let p = dom.create_element("p");
        dom.append_child(div, span1).unwrap();
        dom.append_child(div, p).unwrap();
        dom.append_child(div, span2).unwrap();
        dom.append_child(root, div).unwrap();
        let list = dom.elements_by_tag("span");
        let ids: Vec<NodeId> = list.iter().map(|n| n.id()).collect();
        assert_eq!(ids, vec![span1, span2]);
    }

    #[test]
    fn document_element_returns_root_when_element() {
        let dom: Dom<()> = Dom::with_root_tag("html");
        let html_id = dom.root();
        assert_eq!(dom.document_element().id(), html_id);
    }

    #[test]
    fn document_element_returns_first_element_child_when_root_is_fragment() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        // Default root is a Fragment; first element child wins.
        let html = dom.create_element("html");
        let comment = dom.create_comment("note");
        dom.append_child(root, comment).unwrap();
        dom.append_child(root, html).unwrap();
        assert_eq!(dom.document_element().id(), html);
    }

    #[test]
    fn document_element_returns_root_fragment_when_no_element_child() {
        // Edge case: empty fragment root → return the root itself.
        let dom: Dom = Dom::new();
        let root = dom.root();
        assert_eq!(dom.document_element().id(), root);
    }

    #[test]
    fn active_element_and_has_focus_track_focused() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let el = dom.create_element("input");
        dom.append_child(root, el).unwrap();
        assert!(!dom.has_focus());
        assert!(dom.active_element().is_none());
        dom.set_focused(Some(el));
        assert!(dom.has_focus());
        assert_eq!(dom.active_element().map(|n| n.id()), Some(el));
        dom.set_focused(None);
        assert!(!dom.has_focus());
        assert!(dom.active_element().is_none());
    }

    #[test]
    fn create_range_returns_collapsed_at_root() {
        use crate::Position;
        let dom: Dom = Dom::new();
        let r = dom.create_range();
        assert!(r.is_collapsed());
        assert_eq!(r.start, Position::new(dom.root(), 0));
    }
}
