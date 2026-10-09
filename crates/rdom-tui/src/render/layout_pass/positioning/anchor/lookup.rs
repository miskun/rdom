//! Finding an anchor (CSS Anchor Positioning 1 §2): the element an
//! `anchor-name` names for a querying box — the last *acceptable* one in
//! tree order — and a box's default anchor (`position-anchor`, its
//! implicit anchor under `auto`: a popover's invoker, HTML §6.12).
//!
//! Acceptable (§2.4, rdom's reading): the anchor has a box; it is neither
//! the querying element nor inside it; the querying element is inside
//! every `anchor-scope` that keeps the name around the anchor (§2.2); and
//! the anchor is laid out before the querying box is placed — inside the
//! querying box's containing block (anywhere when that is the viewport),
//! and, where it or an ancestor below that block is itself absolutely
//! positioned, before the querying element in tree order (positioned boxes
//! are placed in tree order).

use rdom_core::{DocumentPosition, Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Display, LayoutRect, Position, PositionAnchor};
use crate::node::TuiNodeExt;

#[cfg(test)]
thread_local! {
    /// Anchor indexes built (cost tests).
    pub(in crate::render::layout_pass) static INDEX_BUILDS: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

/// The elements with an `anchor-name`, in tree order — built once per
/// placement pass, on the first anchored box (a page without anchor
/// positioning builds none).
#[derive(Default)]
pub(in crate::render::layout_pass) struct AnchorIndex {
    built: std::cell::OnceCell<Vec<NodeId>>,
}

impl AnchorIndex {
    fn named(&self, dom: &Dom<TuiExt>) -> &[NodeId] {
        self.built.get_or_init(|| {
            #[cfg(test)]
            INDEX_BUILDS.with(|c| c.set(c.get() + 1));
            dom.descendants(dom.root())
                .filter(|&n| {
                    dom.node(n).node_type() == NodeType::Element
                        && dom
                            .node(n)
                            .tui_ext()
                            .and_then(|e| e.computed.as_deref())
                            .is_some_and(|c| !c.anchor.anchor_name.names().is_empty())
                })
                .collect()
        })
    }

    /// The anchor `name` resolves to for `querying` (its box's containing
    /// block element `cb`, `None` for the viewport): the last acceptable
    /// element so named.
    pub(in crate::render::layout_pass) fn find(
        &self,
        dom: &Dom<TuiExt>,
        querying: Querying,
        cb: Option<NodeId>,
        name: &str,
    ) -> Option<NodeId> {
        self.named(dom).iter().rev().copied().find(|&a| {
            dom.node(a)
                .tui_ext()
                .and_then(|e| e.computed.as_deref())
                .is_some_and(|c| c.anchor.anchor_name.has(name))
                && acceptable(dom, querying, cb, a, Some(name))
        })
    }
}

fn computed(dom: &Dom<TuiExt>, id: NodeId) -> Option<&crate::style::ComputedStyle> {
    dom.node(id).tui_ext()?.computed.as_deref()
}

/// Whether `anchor` may anchor `querying` (module doc); `name` the name it
/// was found by, which an `anchor-scope` may keep from it (`None`: an
/// implicit anchor, which no scope keeps).
fn acceptable(
    dom: &Dom<TuiExt>,
    querying: Querying,
    cb: Option<NodeId>,
    anchor: NodeId,
    name: Option<&str>,
) -> bool {
    let Querying {
        node: querying,
        pseudo,
    } = querying;
    // A box's anchor is not inside it — a pseudo-element's may be inside
    // its host, which is not its box.
    if !pseudo && (anchor == querying || dom.node(querying).contains(anchor)) {
        return false;
    }
    let Some(c) = computed(dom, anchor) else {
        return false;
    };
    if c.display == Display::None || crate::render::box_tree::is_contents(dom, anchor) {
        return false;
    }
    // A `display: none` ancestor leaves it no box.
    let mut up = crate::render::box_tree::box_parent(dom, anchor);
    let mut positioned = matches!(c.position, Position::Absolute | Position::Fixed);
    while let Some(p) = up {
        if Some(p) == cb {
            break;
        }
        if let Some(pc) = computed(dom, p) {
            if pc.display == Display::None {
                return false;
            }
            if let Some(name) = name
                && pc.anchor.anchor_scope.scopes(name)
                && !dom.node(p).contains(querying)
            {
                return false;
            }
            positioned |= matches!(pc.position, Position::Absolute | Position::Fixed);
        }
        up = crate::render::box_tree::box_parent(dom, p);
    }
    // Inside the querying box's containing block (the walk reached it).
    if let Some(cb) = cb
        && !dom.node(cb).contains(anchor)
    {
        return false;
    }
    // The anchor's own scope (§2.2: the scoping element's subtree,
    // itself included).
    if let Some(name) = name
        && c.anchor.anchor_scope.scopes(name)
        && !dom.node(anchor).contains(querying)
    {
        return false;
    }
    // Placed before the querying box.
    !positioned
        || dom
            .compare_document_position(querying, anchor)
            .contains(DocumentPosition::PRECEDING)
}

/// Who asks for an anchor: an element, or a pseudo-element of the host
/// `node` (`pseudo`).
#[derive(Debug, Clone, Copy)]
pub(in crate::render::layout_pass) struct Querying {
    pub(in crate::render::layout_pass) node: NodeId,
    pub(in crate::render::layout_pass) pseudo: bool,
}

/// `querying`'s default anchor (§2.3): its `position-anchor`'s, or under
/// `auto` its implicit anchor — the invoker of a showing popover (a
/// pseudo-element has none).
pub(in crate::render::layout_pass) fn default_anchor(
    dom: &Dom<TuiExt>,
    index: &AnchorIndex,
    querying: Querying,
    cb: Option<NodeId>,
    position_anchor: &PositionAnchor,
) -> Option<NodeId> {
    match position_anchor {
        PositionAnchor::None => None,
        PositionAnchor::Name(name) => index.find(dom, querying, cb, name),
        PositionAnchor::Auto if querying.pseudo => None,
        PositionAnchor::Auto => crate::runtime::builtins::popover::invoker_of(dom, querying.node)
            .filter(|&a| acceptable(dom, querying, cb, a, None)),
    }
}

/// The anchor box's rect: its border box (the fragments' bounding box
/// for a fragmented one).
pub(in crate::render::layout_pass) fn anchor_box(
    dom: &Dom<TuiExt>,
    anchor: NodeId,
) -> Option<LayoutRect> {
    dom.node(anchor).tui_ext().map(|e| e.border_box())
}
