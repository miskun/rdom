//! Feeding a block container's box tree to the line packer (CSS 2.1
//! §9.2.1.1, CSS Display 3 §2.4): its text, the inline boxes it descends
//! into with their `::before` / `::after`, its hard breaks, and each
//! atomic inline as one box — shared by layout (`compute_inline_layout`,
//! `pack_run`) and intrinsic measurement (`measure`).

use rdom_core::{Dom, NodeId, NodeType};

use super::packer::LinePacker;
use super::{RunPseudos, generated, vertical};
use crate::ext::{PseudoSlot, StyleSlot, TuiExt};
use crate::layout::WhiteSpace;
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;

/// The `white-space` of the block container `id`, whose inline
/// formatting context it governs.
pub(super) fn white_space(dom: &Dom<TuiExt>, id: NodeId) -> WhiteSpace {
    dom.node(id)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .map(|c| c.white_space)
        .unwrap_or(WhiteSpace::Normal)
}

/// Feed `block`'s whole inline content to `packer`: its `::before`, its
/// subtree, its `::after`.
pub(super) fn fill_block<'a>(dom: &'a Dom<TuiExt>, block: NodeId, packer: &mut LinePacker<'a>) {
    push_pseudo(dom, block, PseudoSlot::Before, packer);
    walk_subtree(dom, block, packer);
    push_pseudo(dom, block, PseudoSlot::After, packer);
}

/// Push `host`'s `slot` pseudo-element if it joins `host`'s own inline
/// content (see [`generated`]). `::before` first pushes the markers of
/// the list items whose first line this is.
fn push_pseudo<'a>(
    dom: &'a Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
    packer: &mut LinePacker<'a>,
) {
    if slot == PseudoSlot::Before {
        for item in generated::deferred_markers(dom, host) {
            if let Some(text) = generated::static_pseudo_text(dom, item, StyleSlot::Before) {
                packer.push_generated(item, PseudoSlot::Before, text);
            }
        }
    }
    if let Some(text) = generated::own_inline_pseudo_text(dom, host, slot.into()) {
        packer.push_generated(host, slot, text);
    }
}

/// Feed the run `direct_children` of `parent` to `packer`, with the
/// pseudo-elements `pseudos` asks for ([`pack_run`]).
pub(super) fn fill_run<'a>(
    dom: &'a Dom<TuiExt>,
    parent: NodeId,
    direct_children: &[BoxItem],
    pseudos: RunPseudos,
    packer: &mut LinePacker<'a>,
) {
    if pseudos.before {
        push_pseudo(dom, parent, PseudoSlot::Before, packer);
    }
    for &item in direct_children {
        let child_id = match item {
            BoxItem::Node(n) => n,
            // The `::before` / `::after` of a box-less child that holds
            // a block box: an inline box of this flow, hosted by it.
            BoxItem::Generated(host, slot) => {
                if let Some(text) = crate::render::box_tree::generated_text(dom, host, slot) {
                    packer.push_generated(host, slot, text);
                }
                continue;
            }
        };
        let child = dom.node(child_id);
        // A text node directly in a box-less child that holds a block
        // box is owned by that child (its parent), not by `parent`.
        let owner = child.parent_node().map_or(parent, |p| p.id());
        match child.node_type() {
            NodeType::Text => {
                if let Some(data) = child.node_value() {
                    packer.push_text(owner, child_id, data);
                }
            }
            NodeType::Element => {
                if crate::render::layout_pass::float::float_side(dom, child_id).is_some() {
                    push_float(dom, child_id, packer);
                    continue;
                }
                if child.tag_name() == Some("br") {
                    packer.push_hard_break(child_id);
                    continue;
                }
                // An atomic inline participates as one box — see
                // `walk_subtree` for the rationale.
                if child
                    .computed()
                    .is_some_and(crate::render::box_tree::is_atomic_inline)
                {
                    push_atom(dom, child_id, packer);
                    continue;
                }
                walk_inline_box(dom, child_id, packer);
            }
            _ => {}
        }
    }
    if pseudos.after {
        push_pseudo(dom, parent, PseudoSlot::After, packer);
    }
}

/// Recursively walk `id`'s descendants in document order, feeding
/// every text node's graphemes to `packer`. Descends into
/// `display: inline` elements (their pseudo-elements included — see
/// [`walk_inline_box`]); `<br>` emits a hard line break.
///
/// Non-element children (comments, fragments) are passed through
/// their descendant element walk.
fn walk_subtree<'a>(dom: &'a Dom<TuiExt>, id: NodeId, packer: &mut LinePacker<'a>) {
    use crate::layout::Display;
    for child in dom.node(id).child_nodes() {
        match child.node_type() {
            NodeType::Text => {
                // Owner is `id` — the direct element parent. Text
                // node's id goes in too for source-offset tracking.
                if let Some(data) = child.node_value() {
                    packer.push_text(id, child.id(), data);
                }
            }
            NodeType::Element => {
                use crate::layout::Position;
                let (display, position) = child
                    .ext()
                    .and_then(|e| e.computed.as_ref())
                    .map(|c| (c.display, c.position))
                    .unwrap_or((Display::Block, Position::Static));
                // Out-of-flow descendants contribute nothing to the
                // inline formatting context: `display: none` generates
                // no box, and `position: absolute|fixed` boxes are
                // placed independently by phase-2 positioning. Skipping
                // them keeps their text out of an ancestor's inline run
                // — e.g. a collapsed tree branch (`[role=group]` set to
                // `display: none`) must not leak "hidden-child" into the
                // parent treeitem's text, and a chip with an absolutely-
                // positioned dropdown must pack only the chip's own text.
                if display == Display::None
                    || matches!(position, Position::Absolute | Position::Fixed)
                {
                    continue;
                }
                // A float leaves the line (CSS 2.1 §9.5): placed beside it.
                if crate::render::layout_pass::float::float_side(dom, child.id()).is_some() {
                    push_float(dom, child.id(), packer);
                    continue;
                }
                // <br> is a hard break. Matches HTML's baked-in
                // behavior; recognized by tag name rather than by a
                // Display variant to avoid complicating the cascade
                // for a one-element special case.
                if child.tag_name() == Some("br") {
                    packer.push_hard_break(child.id());
                    continue;
                }
                // CSS 2.1 §10.8: an atomic inline (`inline-block`,
                // `inline-flex`, `box_tree::is_atomic_inline`)
                // participates in IFC as a single atomic inline-
                // level box. Don't recurse into it — the packer
                // emits one fragment of its width and rows, the layout
                // pass lays the element out at that rect and paint
                // paints it there as a box, at its turn in the line.
                if child
                    .computed()
                    .is_some_and(crate::render::box_tree::is_atomic_inline)
                {
                    push_atom(dom, child.id(), packer);
                    continue;
                }
                walk_inline_box(dom, child.id(), packer);
            }
            _ => {}
        }
    }
}

/// Feed one in-flow inline element: its static `::before`, its
/// content, its static `::after`. CSS 2.1 §12.1: the pseudo-elements
/// are the element's first / last inline children, so they pack at its
/// start / end, in its line flow (they wrap, and the text beside them
/// shifts). They land in [`LineBox::generated`], hosted by the element.
fn walk_inline_box<'a>(dom: &'a Dom<TuiExt>, id: NodeId, packer: &mut LinePacker<'a>) {
    if let Some(text) = generated::static_pseudo_text(dom, id, StyleSlot::Before) {
        packer.push_generated(id, PseudoSlot::Before, text);
    }
    walk_subtree(dom, id, packer);
    if let Some(text) = generated::static_pseudo_text(dom, id, StyleSlot::After) {
        packer.push_generated(id, PseudoSlot::After, text);
    }
}

/// Push the float `id` met in the inline content (CSS 2.1 §9.5). An
/// intrinsic width measurement packs it as an unbreakable box its margin
/// box wide — beside the text on one line for max-content, alone for
/// min-content (CSS Sizing 3 §5.1) — and lays nothing out.
fn push_float(dom: &Dom<TuiExt>, id: NodeId, packer: &mut LinePacker<'_>) {
    if packer.is_measuring() {
        let width = crate::render::layout_pass::float::size::outer_contribution(
            dom,
            id,
            packer.content_width() > 0,
        );
        packer.push_atomic_inline_block(id, width, vertical::AtomRows::UNMEASURED);
        return;
    }
    packer.push_float(id);
}

/// Push the inline block `id` as an atom: its width and its rows in
/// the line (`vertical`).
fn push_atom(dom: &Dom<TuiExt>, id: NodeId, packer: &mut LinePacker<'_>) {
    if packer.is_measuring() {
        // Its own max-content width (CSS 2.1 §10.3.9), its percentages
        // against no basis; its rows are not asked for.
        let width = atomic_inline_block_intrinsic_width(dom, id, 0);
        packer.push_atomic_inline_block(id, width, vertical::AtomRows::UNMEASURED);
        return;
    }
    let cb_width = packer.content_width();
    let width = atomic_inline_block_intrinsic_width(dom, id, cb_width);
    let rows = vertical::atom_rows(dom, id, width, cb_width);
    packer.push_atomic_inline_block(id, width, rows);
}

/// Intrinsic main-axis (row) content width of an inline-block
/// element treated as an atomic IFC box. Includes UA pseudo
/// content (`::before` + `::after`) plus own text/inline content
/// plus padding/border via the existing intrinsic measurement.
fn atomic_inline_block_intrinsic_width(
    dom: &Dom<TuiExt>,
    id: NodeId,
    containing_block_width: u16,
) -> u16 {
    // `intrinsic_size` already factors in pseudo widths +
    // padding + border for Display::InlineBlock — that's the same
    // measurement the flex layout uses to size inline-block flex
    // items. Pass `cross_budget = 0` since IFC packers don't
    // affect inline-block height; only the width matters here.
    // The atom's containing block is the IFC's block container, whose
    // content width is definite: percent padding / margins resolve
    // against it (CSS 2.1 §8.4).
    crate::render::layout_pass::intrinsic::intrinsic_size(
        dom,
        id,
        crate::layout::Direction::Row,
        0,
        containing_block_width,
    )
}
