//! Where a host's generated content (`::before` / `::after`) is laid
//! out — the one set of predicates the packer, block layout, intrinsic
//! sizing and the margin-collapse walker share.
//!
//! CSS 2.1 §12.1: a static `::before` / `::after` is an inline box, the
//! host's first / last child. An *inline* host's pseudos simply pack at
//! its start / end in the enclosing inline flow (`walk_inline_box`);
//! paint tags them with the host's link and hit-testing routes their
//! cells to the host. For a block host, two placements follow:
//!
//! 1. **In the host's own inline flow** — the host is an IFC block or a
//!    pure-text leaf, or its first (last) in-flow content is
//!    inline-level and so sits in its first (last) anonymous block box.
//!    The packer lays the pseudo out with that content.
//! 2. **A line of its own** — the host's first (last) in-flow content
//!    is a block-level child: CSS 2.1 §9.2.1.1 wraps the inline pseudo
//!    in an anonymous block box before (after) that child
//!    ([`own_line_pseudos`]).
//!
//! A list item's `::marker` is laid out by [`super::markers`]: it rides
//! the item's first line box, wherever that line is.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{StyleSlot, TuiExt};
use crate::layout::{Display, Position};
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;

/// The computed style of `host`'s `slot` pseudo-element (`None` for the
/// host itself).
fn pseudo_style(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: StyleSlot,
) -> Option<&crate::style::ComputedStyle> {
    if slot == StyleSlot::Host {
        return None;
    }
    dom.node(host).ext()?.computed_for(slot).map(|c| &**c)
}

/// The text of `host`'s `slot` pseudo-element when it is a static
/// (`position: static`) box with `content` — none under `display: none`,
/// which generates no box (CSS 2.1 §12.1). Positioned pseudo-elements
/// are laid out and painted on their own (`positioned_pseudos`).
pub(crate) fn static_pseudo_text(dom: &Dom<TuiExt>, host: NodeId, slot: StyleSlot) -> Option<&str> {
    let computed = pseudo_style(dom, host, slot)?;
    if computed.position != Position::Static || computed.display == Display::None {
        return None;
    }
    computed.content.as_deref()
}

/// Whether `host`'s `slot` pseudo-element is a block-level box of its
/// host's block flow (CSS 2.1 §12.1, CSS Pseudo 4 §2: it is rendered "as
/// if it were a real element", its `display` included): a static one
/// with `content` — `""` makes an empty box — whose computed `display`
/// is block-level (`block`, `flow-root`, `flex`, `grid`) and that does
/// not float, in a host whose children lay out in block flow. It is the
/// host's first (last) block-level box, laid out by the block pass
/// (`layout_pass::block::generated`), never packed into a line.
pub(crate) fn is_block_pseudo(dom: &Dom<TuiExt>, host: NodeId, slot: StyleSlot) -> bool {
    let node = dom.node(host);
    let Some(hc) = node.computed() else {
        return false;
    };
    if !hc.flow.is_block_flow() || !matches!(hc.display, Display::Block | Display::InlineBlock) {
        return false;
    }
    let computed = pseudo_style(dom, host, slot);
    computed.is_some_and(|c| {
        c.display == Display::Block
            && c.float == crate::layout::Float::None
            && static_pseudo_text(dom, host, slot).is_some()
    })
}

/// Which of `host`'s pseudo-elements are block-level boxes
/// ([`is_block_pseudo`]).
pub(crate) fn block_pseudos(dom: &Dom<TuiExt>, host: NodeId) -> super::RunPseudos {
    super::RunPseudos {
        before: is_block_pseudo(dom, host, StyleSlot::Before),
        after: is_block_pseudo(dom, host, StyleSlot::After),
    }
}

/// How a static `::before` / `::after` that is not a block box of its
/// host's flow takes part in the inline content it joins (CSS Pseudo 4
/// §2: its `display` and `float` make its box, as an element's do).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InlinePseudo<'a> {
    /// Its generated text, inline content of the lines (an inline box).
    Text(&'a str),
    /// An atomic inline — `inline-block`, `inline flow-root`,
    /// `inline-flex`, `inline-grid` (CSS Display 3 §2.4): one box in its
    /// line, its text laid out inside it.
    Atom,
    /// A float (CSS 2.1 §9.5): out of the line, beside it.
    Float,
}

/// The inline-level box `host`'s `slot` pseudo-element is, wherever it
/// joins inline content ([`InlinePseudo`]); `None` when it generates no
/// static box or is a block-level box of the host's flow
/// ([`is_block_pseudo`]).
pub(crate) fn inline_pseudo(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: StyleSlot,
) -> Option<InlinePseudo<'_>> {
    let text = static_pseudo_text(dom, host, slot)?;
    let computed = pseudo_style(dom, host, slot)?;
    if is_block_pseudo(dom, host, slot) {
        return None;
    }
    let pslot = match slot {
        StyleSlot::Before => crate::ext::PseudoSlot::Before,
        _ => crate::ext::PseudoSlot::After,
    };
    let item = BoxItem::Generated(host, pslot);
    if crate::render::layout_pass::float::float_side_of(dom, item).is_some() {
        return Some(InlinePseudo::Float);
    }
    if computed.is_atomic_inline() {
        return Some(InlinePseudo::Atom);
    }
    Some(InlinePseudo::Text(text))
}

/// Whether `host`'s `slot` pseudo-element is a float of its flow (CSS 2.1
/// §9.5) — an item of the host's box sequence
/// (`box_tree::box_sequence`), placed by the block pass or the packer.
pub(crate) fn is_float_pseudo(dom: &Dom<TuiExt>, host: NodeId, slot: StyleSlot) -> bool {
    matches!(inline_pseudo(dom, host, slot), Some(InlinePseudo::Float))
}

/// Which of `host`'s pseudo-elements are items of its box sequence
/// (`box_tree::box_sequence`): its block-level boxes ([`is_block_pseudo`])
/// and its floats ([`is_float_pseudo`]).
pub(crate) fn sequence_pseudos(dom: &Dom<TuiExt>, host: NodeId) -> super::RunPseudos {
    let item = |slot| is_block_pseudo(dom, host, slot) || is_float_pseudo(dom, host, slot);
    super::RunPseudos {
        before: item(StyleSlot::Before),
        after: item(StyleSlot::After),
    }
}

/// Which of `host`'s pseudo-elements take a line of their own (CSS 2.1
/// §9.2.1.1): the `::before` when the host's first in-flow content is a
/// block-level child, the `::after` when its last is. Only for a
/// block-flow container with visible generated text (or a list marker
/// riding its own first line, which then has no line box in its content
/// to ride: CSS Lists 3 §3.1, the marker makes one).
pub(crate) fn own_line_pseudos(dom: &Dom<TuiExt>, host: NodeId) -> super::RunPseudos {
    if !is_block_flow_container(dom, host) {
        return super::RunPseudos::default();
    }
    let visible = visible_inline_pseudos(dom, host);
    let block_edge =
        |from_end| line_bearing_child(dom, host, from_end).is_some_and(|c| is_block_level(dom, c));
    super::RunPseudos {
        before: visible.before && block_edge(false),
        after: visible.after && block_edge(true),
    }
}

/// Which of `host`'s `::before` / `::after` are visible inline content —
/// text a line box would hold wherever the pseudo is placed, or an
/// atomic inline (CSS 2.1 §9.4.2: in-flow content makes a line box). A
/// list marker riding `host`'s own first line counts with the `::before`.
pub(crate) fn visible_inline_pseudos(dom: &Dom<TuiExt>, host: NodeId) -> super::RunPseudos {
    // `host`'s own marker, when no line box in its content can take it, is
    // pushed with its `::before` (`feed::push_pseudo`): it makes that line.
    super::RunPseudos {
        before: before_is_inline_content(dom, host) || super::markers::makes_own_line(dom, host),
        after: is_visible_inline(inline_pseudo(dom, host, StyleSlot::After)),
    }
}

/// Whether `host`'s `::before` is visible inline content: text a line
/// box holds, or an atomic inline.
pub(super) fn before_is_inline_content(dom: &Dom<TuiExt>, host: NodeId) -> bool {
    is_visible_inline(inline_pseudo(dom, host, StyleSlot::Before))
}

fn is_visible_inline(pseudo: Option<InlinePseudo<'_>>) -> bool {
    match pseudo {
        Some(InlinePseudo::Text(t)) => !t.trim().is_empty(),
        Some(InlinePseudo::Atom) => true,
        Some(InlinePseudo::Float) | None => false,
    }
}

/// Which of `host`'s `::before` / `::after` are inline-level participants
/// of the flow they join: [`visible_inline_pseudos`], and floats.
pub(crate) fn inline_level_pseudos(dom: &Dom<TuiExt>, host: NodeId) -> super::RunPseudos {
    let visible = visible_inline_pseudos(dom, host);
    let float = |slot| is_float_pseudo(dom, host, slot);
    super::RunPseudos {
        before: visible.before || float(StyleSlot::Before),
        after: visible.after || float(StyleSlot::After),
    }
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
    direct_children: &[BoxItem],
) -> super::RunPseudos {
    let holds_edge = |from_end| {
        line_bearing_child(dom, host, from_end).is_none_or(|c| direct_children.contains(&c))
    };
    super::RunPseudos {
        before: holds_edge(false),
        after: holds_edge(true),
    }
}

/// `host`'s first (`from_end = false`) or last in-flow box item that
/// can hold content of a line (see [`bears_line`]), in box-tree order
/// (`box_tree::box_sequence`: a generated item of a box-less child
/// holds its text).
pub(super) fn line_bearing_child(
    dom: &Dom<TuiExt>,
    host: NodeId,
    from_end: bool,
) -> Option<BoxItem> {
    let bears = |c: &BoxItem| match *c {
        BoxItem::Node(c) => bears_line(dom, host, c),
        // A float holds no line (CSS 2.1 §9.5).
        BoxItem::Generated(h, slot) => !is_float_pseudo(dom, h, slot.into()),
    };
    let children = crate::render::box_tree::box_sequence(dom, host);
    if from_end {
        children.into_iter().rev().find(bears)
    } else {
        children.into_iter().find(bears)
    }
}

/// Whether `child`, a child node of `host`, can hold content of a line:
/// text other than whitespace that `host`'s `white-space` collapses
/// away, or an in-flow element. Collapsible whitespace-only text,
/// comments and fragments generate no line box (CSS 2.1 §9.2.1.1 /
/// §16.6.1), nor do out-of-flow elements.
pub(crate) fn bears_line(dom: &Dom<TuiExt>, host: NodeId, child: NodeId) -> bool {
    let node = dom.node(child);
    match node.node_type() {
        NodeType::Text => {
            let collapse = dom
                .node(host)
                .computed()
                .map(|c| c.text.white_space_collapse)
                .unwrap_or_default();
            !node.node_value().is_none_or(|t| {
                t.chars()
                    .all(|c| crate::render::inline::is_collapsible_white_space(c, collapse))
            })
        }
        NodeType::Element => crate::render::layout_pass::is_in_flow(dom, child),
        _ => false,
    }
}

/// A `display: block` element whose children flow as blocks — the only
/// container whose first child can pass its first line down.
pub(super) fn is_block_flow_container(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    dom.node(id)
        .computed()
        .is_some_and(|c| c.display == Display::Block && c.flow.is_block_flow())
}

pub(super) fn is_block_level(dom: &Dom<TuiExt>, item: BoxItem) -> bool {
    let BoxItem::Node(id) = item else {
        return false;
    };
    let node = dom.node(id);
    node.node_type() == NodeType::Element
        && node.computed().is_none_or(|c| c.display == Display::Block)
}
