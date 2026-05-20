//! `NodeMutHtml` — author-facing HTML setters on `NodeMut`.
//!
//! These methods need the parser, so they can't live on `NodeMut`
//! in `rdom-core` (the dependency direction is parser → core).
//! Authors `use rdom_parser::NodeMutHtml;` to bring them in. The
//! `rdom_tui::prelude` (M4b step 32) re-exports the trait so a
//! TUI consumer gets the full M4 surface with one import.
//!
//! ## Error policy
//!
//! Strict: malformed input propagates `rdom_parser::ParseError`.
//! No recovery. If a real consumer needs spec-faithful recovery
//! (insert what can be parsed, drop the rest), that lands as a
//! polish item.
//!
//! Non-parse failures (e.g. `set_outer_html` called on a
//! parentless receiver) surface as a synthesized `ParseError`
//! carrying a diagnostic message. The trait's single error type
//! keeps call sites simple — the docstring on each method lists
//! the non-parser failure modes.
//!
//! ## Resource hygiene
//!
//! `set_inner_html` drops the existing subtree of the receiver
//! (frees arena slots, removes attached listeners). The §14.2 row
//! promise: an AbortSignal-managed listener registered on a
//! removed child is gone after `set_inner_html` replaces the
//! subtree.

use rdom_core::{AdjacentPosition, NodeId, NodeMut};

use crate::error::{ParseError, Result};
use crate::parser::parse_into;

/// HTML setters on `NodeMut`. Brings the `innerHTML` /
/// `outerHTML` / `insertAdjacentHTML` ergonomics that `rdom-core`
/// can't expose directly (since it can't depend on the parser).
///
/// Implemented for any `NodeMut<'_, Ext>` whose `Ext` is
/// `Default + 'static` (the parser's bound for materializing new
/// elements).
pub trait NodeMutHtml<'a, Ext>
where
    Ext: Default + 'static,
{
    /// Parse `html` and use it as the receiver's children,
    /// replacing whatever was there. DOM `Element.innerHTML`
    /// setter.
    ///
    /// The existing subtree is dropped — arena slots are freed and
    /// listeners (including AbortSignal-controlled ones) are
    /// released.
    ///
    /// **Errors:** any `ParseError` from the underlying parser.
    fn set_inner_html(&mut self, html: &str) -> Result<()>;

    /// Replace the receiver in its parent with the parsed top-
    /// level nodes from `html`. DOM `Element.outerHTML` setter.
    ///
    /// **Consumes `self`** — the receiver is detached and dropped,
    /// so the handle is no longer usable.
    ///
    /// Returns the `NodeId` of the **first** new top-level node;
    /// remaining top-level nodes are spliced in at the same
    /// position.
    ///
    /// **Errors:**
    /// - `ParseError` from the underlying parser.
    /// - `ParseError` (synthesized) if the receiver has no parent
    ///   — outer-HTML replacement requires an attached element.
    /// - `ParseError` (synthesized) if the parsed fragment is
    ///   empty.
    ///
    /// ```compile_fail
    /// use rdom_core::Dom;
    /// use rdom_parser::NodeMutHtml;
    /// let mut dom: Dom = Dom::new();
    /// let root = dom.root();
    /// let el = dom.create_element("div");
    /// dom.append_child(root, el).unwrap();
    /// let nm = dom.node_mut(el);
    /// nm.set_outer_html("<p>x</p>").unwrap();
    /// let _ = nm.id();  // ERROR: nm was consumed
    /// ```
    fn set_outer_html(self, html: &str) -> Result<NodeId>;

    /// Parse `html` and insert the resulting nodes at `position`
    /// relative to the receiver. DOM
    /// `Element.insertAdjacentHTML(position, html)`.
    ///
    /// `BeforeBegin` and `AfterEnd` require a parent — both fail
    /// with a synthesized `ParseError` if the receiver is
    /// parentless. `AfterBegin` and `BeforeEnd` always succeed
    /// when the parse succeeds.
    ///
    /// **Errors:** any `ParseError` from the underlying parser,
    /// plus a synthesized one for the parent-required positions.
    fn insert_adjacent_html(&mut self, position: AdjacentPosition, html: &str) -> Result<()>;
}

impl<'a, Ext> NodeMutHtml<'a, Ext> for NodeMut<'a, Ext>
where
    Ext: Default + 'static,
{
    fn set_inner_html(&mut self, html: &str) -> Result<()> {
        let id = self.id();
        let dom = self.dom_mut();
        while let Some(first) = dom.node(id).first_child().map(|n| n.id()) {
            let _ = dom.drop_subtree(first);
        }
        parse_into(dom, html, id)?;
        Ok(())
    }

    fn set_outer_html(self, html: &str) -> Result<NodeId> {
        let id = self.id();
        let dom = self.into_dom_mut();

        let parent = match dom.node(id).parent_node() {
            Some(p) => p.id(),
            None => return Err(synth_error("set_outer_html: receiver has no parent")),
        };

        // Parse into a fresh staging fragment in the same arena so
        // the parsed nodes are adoptable into `parent` without
        // copying.
        let staging = dom.create_document_fragment();
        let parsed = parse_into(dom, html, staging)?;
        if parsed.is_empty() {
            let _ = dom.drop_subtree(staging);
            return Err(synth_error("set_outer_html: parsed fragment is empty"));
        }

        // Splice each parsed top-level node into the parent at
        // self's position, in document order.
        for &child in &parsed {
            dom.insert_before(parent, child, Some(id))
                .map_err(|e| synth_error(&format!("set_outer_html: {e}")))?;
        }

        // Staging fragment is now empty; the original receiver
        // subtree is dropped (frees listeners + arena slots).
        let _ = dom.drop_subtree(staging);
        let _ = dom.drop_subtree(id);

        Ok(parsed[0])
    }

    fn insert_adjacent_html(&mut self, position: AdjacentPosition, html: &str) -> Result<()> {
        let id = self.id();
        let dom = self.dom_mut();
        let parent_of_id = dom.node(id).parent_node().map(|n| n.id());
        let needs_parent = matches!(
            position,
            AdjacentPosition::BeforeBegin | AdjacentPosition::AfterEnd
        );
        if needs_parent && parent_of_id.is_none() {
            return Err(synth_error(
                "insert_adjacent_html: BeforeBegin/AfterEnd require a parent",
            ));
        }

        // Compute a fixed (insertion_parent, reference) pair so a
        // single insert_before loop preserves parse order across
        // all four positions. Spec semantics fall out of the
        // reference choice — see below.
        let (insertion_parent, reference) = match position {
            AdjacentPosition::BeforeBegin => (parent_of_id.unwrap(), Some(id)),
            AdjacentPosition::AfterBegin => (id, dom.node(id).first_child().map(|n| n.id())),
            AdjacentPosition::BeforeEnd => (id, None),
            AdjacentPosition::AfterEnd => (
                parent_of_id.unwrap(),
                dom.node(id).next_sibling().map(|n| n.id()),
            ),
        };

        let staging = dom.create_document_fragment();
        let parsed = parse_into(dom, html, staging)?;

        for child in parsed {
            dom.insert_before(insertion_parent, child, reference)
                .map_err(|e| synth_error(&format!("insert_adjacent_html: {e}")))?;
        }

        let _ = dom.drop_subtree(staging);
        Ok(())
    }
}

/// Synthesize a `ParseError` for non-parse failures. Position is
/// 1,1,0 since there's no source-text location to point at.
fn synth_error(msg: &str) -> ParseError {
    ParseError::new(msg.to_string(), 1, 1, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdom_core::{Dom, ListenerOptions};

    fn child_tags(dom: &Dom, parent: NodeId) -> Vec<String> {
        dom.node(parent)
            .child_nodes()
            .map(|n| n.tag_name().unwrap_or(n.node_name()).to_string())
            .collect()
    }

    fn s(t: &str) -> String {
        t.to_string()
    }

    // ── set_inner_html ────────────────────────────────────────────────

    #[test]
    fn set_inner_html_replaces_children() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("div");
        let old = dom.create_element("span");
        dom.append_child(parent, old).unwrap();
        dom.node_mut(parent)
            .set_inner_html("<p>hi</p><em>bye</em>")
            .unwrap();
        let tags = child_tags(&dom, parent);
        assert_eq!(tags, vec![s("p"), s("em")]);
        // Note: `old`'s arena slot may have been recycled by alloc;
        // dom.contains(old) is not a reliable post-condition. The
        // listener-drop test below covers the resource-hygiene
        // guarantee that matters.
    }

    #[test]
    fn set_inner_html_clears_when_input_empty() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("div");
        let old = dom.create_element("span");
        dom.append_child(parent, old).unwrap();
        dom.node_mut(parent).set_inner_html("").unwrap();
        assert!(!dom.node(parent).has_child_nodes());
        // No re-alloc happens (empty parse), so the slot is still
        // freed.
        assert!(!dom.contains(old));
    }

    #[test]
    fn set_inner_html_drops_abortsignal_listeners_on_removed_children() {
        // §14.2 step 17 row promise: listeners on removed children
        // are released when set_inner_html replaces the subtree.
        //
        // Resource-hygiene check via listener_count: when a node is
        // freed, its listener entry is removed from the store. Even
        // if the arena slot is later recycled by alloc, the
        // recycled-slot's NodeId has no listener entry until a new
        // listener is added against it.
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("div");
        let child = dom.create_element("button");
        dom.append_child(parent, child).unwrap();
        dom.add_event_listener(child, "click", ListenerOptions::default(), |_| {})
            .unwrap();
        assert_eq!(dom.listener_count(child), 1);
        dom.node_mut(parent).set_inner_html("<p>new</p>").unwrap();
        // The original listener is gone. (If the arena slot was
        // recycled for the new <p>, that new node has no listeners.)
        assert_eq!(dom.listener_count(child), 0);
    }

    #[test]
    fn set_inner_html_propagates_parse_error_strictly() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let err = dom
            .node_mut(el)
            .set_inner_html("<div><span></p></div>")
            .unwrap_err();
        // Real parser error, not a synthesized one.
        assert!(err.msg.to_lowercase().contains("mismatch"));
    }

    // ── set_outer_html ────────────────────────────────────────────────

    #[test]
    fn set_outer_html_replaces_self_in_parent_and_returns_first_id() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let target = dom.create_element("span");
        let sibling = dom.create_element("span");
        dom.append_child(root, target).unwrap();
        dom.append_child(root, sibling).unwrap();
        let new_id = dom
            .node_mut(target)
            .set_outer_html("<p>a</p><em>b</em>")
            .unwrap();
        // Returned id is the first parsed top-level.
        assert_eq!(dom.node(new_id).tag_name(), Some("p"));
        let tags = child_tags(&dom, root);
        assert_eq!(
            tags,
            vec!["p".to_string(), "em".to_string(), "span".to_string()]
        );
        assert!(!dom.contains(target));
    }

    #[test]
    fn set_outer_html_errors_when_receiver_has_no_parent() {
        let mut dom: Dom = Dom::new();
        let detached = dom.create_element("div");
        let err = dom
            .node_mut(detached)
            .set_outer_html("<p>x</p>")
            .unwrap_err();
        assert!(err.msg.contains("no parent"));
        // Receiver was not consumed by the impl on the error path? It
        // *was* consumed by Rust's move semantics. We just verify the
        // node is still in the arena.
        assert!(dom.contains(detached));
    }

    #[test]
    fn set_outer_html_errors_on_empty_parsed_fragment() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let target = dom.create_element("div");
        dom.append_child(root, target).unwrap();
        let err = dom.node_mut(target).set_outer_html("").unwrap_err();
        assert!(err.msg.contains("empty"));
        // Receiver stays attached on error.
        assert!(dom.contains(target));
        assert_eq!(dom.node(target).parent_node().map(|p| p.id()), Some(root));
    }

    #[test]
    fn set_outer_html_propagates_parse_error_strictly() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let target = dom.create_element("div");
        dom.append_child(root, target).unwrap();
        let err = dom
            .node_mut(target)
            .set_outer_html("<a><b></a>")
            .unwrap_err();
        assert!(err.msg.to_lowercase().contains("mismatch"));
        // Receiver stays attached on parse error.
        assert!(dom.contains(target));
    }

    // ── insert_adjacent_html ──────────────────────────────────────────

    #[test]
    fn insert_adjacent_html_before_begin() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let target = dom.create_element("span");
        dom.append_child(root, target).unwrap();
        dom.node_mut(target)
            .insert_adjacent_html(AdjacentPosition::BeforeBegin, "<a></a><b></b>")
            .unwrap();
        let tags = child_tags(&dom, root);
        assert_eq!(tags, vec![s("a"), s("b"), s("span")]);
    }

    #[test]
    fn insert_adjacent_html_after_begin() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("div");
        let existing = dom.create_element("span");
        dom.append_child(parent, existing).unwrap();
        dom.node_mut(parent)
            .insert_adjacent_html(AdjacentPosition::AfterBegin, "<a></a><b></b>")
            .unwrap();
        let tags = child_tags(&dom, parent);
        assert_eq!(tags, vec![s("a"), s("b"), s("span")]);
    }

    #[test]
    fn insert_adjacent_html_before_end() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("div");
        let existing = dom.create_element("span");
        dom.append_child(parent, existing).unwrap();
        dom.node_mut(parent)
            .insert_adjacent_html(AdjacentPosition::BeforeEnd, "<a></a><b></b>")
            .unwrap();
        let tags = child_tags(&dom, parent);
        assert_eq!(tags, vec![s("span"), s("a"), s("b")]);
    }

    #[test]
    fn insert_adjacent_html_after_end() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let target = dom.create_element("span");
        let tail = dom.create_element("span");
        dom.append_child(root, target).unwrap();
        dom.append_child(root, tail).unwrap();
        dom.node_mut(target)
            .insert_adjacent_html(AdjacentPosition::AfterEnd, "<a></a><b></b>")
            .unwrap();
        let tags = child_tags(&dom, root);
        assert_eq!(tags, vec![s("span"), s("a"), s("b"), s("span")]);
    }

    #[test]
    fn insert_adjacent_html_before_begin_errors_without_parent() {
        let mut dom: Dom = Dom::new();
        let detached = dom.create_element("div");
        let err = dom
            .node_mut(detached)
            .insert_adjacent_html(AdjacentPosition::BeforeBegin, "<p/>")
            .unwrap_err();
        assert!(err.msg.contains("parent"));
    }

    #[test]
    fn insert_adjacent_html_after_end_errors_without_parent() {
        let mut dom: Dom = Dom::new();
        let detached = dom.create_element("div");
        let err = dom
            .node_mut(detached)
            .insert_adjacent_html(AdjacentPosition::AfterEnd, "<p/>")
            .unwrap_err();
        assert!(err.msg.contains("parent"));
    }

    #[test]
    fn insert_adjacent_html_propagates_parse_error_strictly() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("div");
        let err = dom
            .node_mut(parent)
            .insert_adjacent_html(AdjacentPosition::BeforeEnd, "<a><b></a>")
            .unwrap_err();
        assert!(err.msg.to_lowercase().contains("mismatch"));
    }
}
