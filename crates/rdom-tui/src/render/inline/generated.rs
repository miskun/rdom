//! Where a host's generated content (`::before` / `::after`) is laid
//! out — the one set of predicates the packer, block layout, intrinsic
//! sizing and the margin-collapse walker share.
//!
//! CSS 2.1 §12.1: a static `::before` / `::after` is an inline box, the
//! host's first / last child. An *inline* host's pseudos simply pack at
//! its start / end in the enclosing inline flow (`walk_inline_box`);
//! paint tags them with the host's link and hit-testing routes their
//! cells to the host. For a block host, three placements follow:
//!
//! 1. **In the host's own inline flow** — the host is an IFC block or a
//!    pure-text leaf, or its first (last) in-flow content is
//!    inline-level and so sits in its first (last) anonymous block box.
//!    The packer lays the pseudo out with that content.
//! 2. **A line of its own** — the host's first (last) in-flow content
//!    is a block-level child: CSS 2.1 §9.2.1.1 wraps the inline pseudo
//!    in an anonymous block box before (after) that child
//!    ([`own_line_pseudos`]).
//! 3. **On a descendant's first line, as a list marker** — rdom has no
//!    `::marker` (DIVERGENCES): the UA numbers and bullets `<li>`
//!    through `li::before`. A browser places a list item's marker on
//!    the first line box of the item, even when that line belongs to a
//!    block child (`<li><p>Step</p></li>` shows "1. Step"), so an
//!    `<li>`'s `::before` whose first in-flow content is block-level
//!    rides the first line of the block descendant that holds it
//!    ([`marker_line_holder`] / [`deferred_markers`]).

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{StyleSlot, TuiExt};
use crate::layout::{Display, Flow, Position, WhiteSpace};
use crate::node::TuiNodeExt;

/// The text of `host`'s `slot` pseudo-element when it is a static
/// (`position: static`) box with `content`. Positioned pseudo-elements
/// are laid out and painted on their own (`positioned_pseudos`).
pub(crate) fn static_pseudo_text(dom: &Dom<TuiExt>, host: NodeId, slot: StyleSlot) -> Option<&str> {
    let node = dom.node(host);
    let computed = match slot {
        StyleSlot::Before => node.computed_before(),
        StyleSlot::After => node.computed_after(),
        StyleSlot::Host => None,
    }?;
    if computed.position != Position::Static {
        return None;
    }
    computed.content.as_deref()
}

/// The static pseudo text that joins `host`'s *own* inline content —
/// [`static_pseudo_text`], minus a list marker that rides a
/// descendant's first line instead ([`marker_line_holder`]).
pub(crate) fn own_inline_pseudo_text(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: StyleSlot,
) -> Option<&str> {
    if slot == StyleSlot::Before && marker_line_holder(dom, host).is_some() {
        return None;
    }
    static_pseudo_text(dom, host, slot)
}

/// Which of `host`'s pseudo-elements take a line of their own (CSS 2.1
/// §9.2.1.1): the `::before` when the host's first in-flow content is a
/// block-level child, the `::after` when its last is. Only for a
/// block-flow container with visible generated text; a list marker that
/// rides a descendant's line is not one.
pub(crate) fn own_line_pseudos(dom: &Dom<TuiExt>, host: NodeId) -> super::RunPseudos {
    if !is_block_flow_container(dom, host) {
        return super::RunPseudos::default();
    }
    let visible =
        |slot| own_inline_pseudo_text(dom, host, slot).is_some_and(|t| !t.trim().is_empty());
    let block_edge =
        |from_end| line_bearing_child(dom, host, from_end).is_some_and(|c| is_block_level(dom, c));
    super::RunPseudos {
        before: visible(StyleSlot::Before) && block_edge(false),
        after: visible(StyleSlot::After) && block_edge(true),
    }
}

/// The block descendant whose first line carries `host`'s list marker:
/// `Some` when `host` is an `<li>` with a static `::before` whose first
/// in-flow content is a block-level child holding a line box. `None`
/// otherwise — the `::before` then takes one of the other placements.
pub(crate) fn marker_line_holder(dom: &Dom<TuiExt>, host: NodeId) -> Option<NodeId> {
    if dom.node(host).tag_name() != Some("li")
        || static_pseudo_text(dom, host, StyleSlot::Before).is_none()
        || !is_block_flow_container(dom, host)
    {
        return None;
    }
    let first = line_bearing_child(dom, host, false)?;
    if !is_block_level(dom, first) {
        return None;
    }
    first_line_holder(dom, first)
}

/// The list items whose markers ride `holder`'s first line, outermost
/// first (`<li><ol><li>x` puts both markers on the inner item's line).
/// Empty unless `holder` owns the first line of each such item.
pub(crate) fn deferred_markers(dom: &Dom<TuiExt>, holder: NodeId) -> Vec<NodeId> {
    let mut markers = Vec::new();
    let mut cur = holder;
    // Climb while `cur` is the first line-bearing child of a block-flow
    // parent: only along that path can an ancestor's first line be
    // `holder`'s.
    while let Some(parent) = dom.node(cur).parent_node().map(|p| p.id()) {
        if !is_block_flow_container(dom, parent)
            || line_bearing_child(dom, parent, false) != Some(cur)
        {
            break;
        }
        if marker_line_holder(dom, parent) == Some(holder) {
            markers.push(parent);
        }
        cur = parent;
    }
    markers.reverse();
    markers
}

/// Whether `host`'s first (`from_end = false`) or last in-flow content
/// is inline-level — text or an inline box, which CSS 2.1 §9.2.1.1
/// wraps in an anonymous block holding a line box. Such a line
/// separates `host`'s top (bottom) margin from its first (last) block
/// child's (§8.3.1). Collapsible whitespace-only text holds no line and
/// does not count.
pub(crate) fn inline_content_at_edge(dom: &Dom<TuiExt>, host: NodeId, from_end: bool) -> bool {
    line_bearing_child(dom, host, from_end).is_some_and(|c| !is_block_level(dom, c))
}

/// Which of `host`'s pseudo-elements a run over `direct_children` (an
/// inline run of `host`'s children, as an anonymous block box holds
/// it) carries: the `::before` when the run holds `host`'s first
/// line-bearing child, the `::after` when it holds the last. A host
/// with no line-bearing child gives any run both (the pseudos stand
/// alone).
pub(crate) fn run_pseudos(
    dom: &Dom<TuiExt>,
    host: NodeId,
    direct_children: &[NodeId],
) -> super::RunPseudos {
    let holds_edge = |from_end| {
        line_bearing_child(dom, host, from_end).is_none_or(|c| direct_children.contains(&c))
    };
    super::RunPseudos {
        before: holds_edge(false),
        after: holds_edge(true),
    }
}

/// The first line of `el` is its own when its first line-bearing
/// content is inline-level; a block-level first child passes it down.
/// `None` when no line box is reachable (an empty block, a flex
/// container).
fn first_line_holder(dom: &Dom<TuiExt>, el: NodeId) -> Option<NodeId> {
    if !is_block_flow_container(dom, el) {
        return None;
    }
    let first = line_bearing_child(dom, el, false)?;
    if is_block_level(dom, first) {
        first_line_holder(dom, first)
    } else {
        Some(el)
    }
}

/// `host`'s first (`from_end = false`) or last in-flow child that can
/// hold content of a line: collapsible whitespace-only text and
/// comments generate no line box and are skipped.
fn line_bearing_child(dom: &Dom<TuiExt>, host: NodeId, from_end: bool) -> Option<NodeId> {
    let collapses = dom
        .node(host)
        .computed()
        .is_none_or(|c| matches!(c.white_space, WhiteSpace::Normal | WhiteSpace::NoWrap));
    let bears_line = |c: NodeId| {
        let node = dom.node(c);
        match node.node_type() {
            NodeType::Text => {
                !(collapses
                    && node
                        .node_value()
                        .is_none_or(|t| t.chars().all(char::is_whitespace)))
            }
            NodeType::Element => crate::render::layout_pass::is_in_flow(dom, c),
            _ => false,
        }
    };
    let mut children = dom.node(host).child_nodes().map(|c| c.id());
    if from_end {
        children.filter(|&c| bears_line(c)).last()
    } else {
        children.find(|&c| bears_line(c))
    }
}

/// A `display: block` element whose children flow as blocks — the only
/// container whose first child can pass its first line down.
fn is_block_flow_container(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    dom.node(id)
        .computed()
        .is_some_and(|c| c.display == Display::Block && c.flow == Flow::Block)
}

fn is_block_level(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    node.node_type() == NodeType::Element
        && node.computed().is_none_or(|c| c.display == Display::Block)
}
