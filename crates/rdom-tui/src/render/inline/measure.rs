//! Intrinsic inline sizes (CSS Sizing 3 §5.1): a box's inline content
//! packed by the packer layout uses, at every soft-wrap opportunity
//! (min-content, available width 0) or at none (max-content, an
//! unbounded one), and its widest line read — so measurement cannot
//! drift from what layout wraps: `white-space`, collapsing, forced
//! breaks, generated content, hanging spaces (CSS Text 3 §4.1.2: the
//! ones that hang at a soft wrap count for neither size, the ones that
//! hang only where they overflow count for max-content), and each
//! atomic inline a box its min- or
//! max-content contribution wide (C7G-INLINE-ATOM-MAX,
//! C8G-FLOAT-MEASURE). The packer runs in its measuring mode
//! (`LinePacker::measuring`).

use rdom_core::{Dom, NodeId};

use super::packer::LinePacker;
use super::{RunPseudos, fill_block, fill_run};
use crate::ext::TuiExt;
use crate::render::box_tree::BoxItem;

/// The widest line of the block container `block`'s inline content, its
/// own `::before` / `::after` included, packed `available` wide.
pub(crate) fn widest_line(dom: &Dom<TuiExt>, block: NodeId, available: u16) -> u16 {
    let mut packer = LinePacker::measuring(available).indented(indent_of(dom, block, true));
    fill_block(dom, block, &mut packer);
    widest(packer)
}

/// The widest line of the inline run `items` of `parent` (one
/// anonymous block box's content, CSS 2.1 §9.2.1.1), without `parent`'s
/// pseudo-elements, packed `available` wide; `first` when the run holds
/// `parent`'s first formatted line (its `text-indent` applies).
pub(crate) fn widest_run_line(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    items: &[BoxItem],
    available: u16,
    first: bool,
) -> u16 {
    let mut packer = LinePacker::measuring(available).indented(indent_of(dom, parent, first));
    fill_run(dom, parent, items, RunPseudos::default(), &mut packer);
    widest(packer)
}

/// The widest line of `host`'s `slot` pseudo-element's own inline content
/// — its generated text, or its atom or float as the run packs them —
/// packed alone `available` wide: its text transformed, collapsed, at its
/// tab stops and letter-spaced, as layout packs it (CSS Sizing 3 §5.1).
pub(crate) fn widest_pseudo_line(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: crate::ext::PseudoSlot,
    available: u16,
) -> u16 {
    let pseudos = RunPseudos {
        before: slot == crate::ext::PseudoSlot::Before,
        after: slot == crate::ext::PseudoSlot::After,
    };
    let mut packer = LinePacker::measuring(available);
    fill_run(dom, host, &[], pseudos, &mut packer);
    widest(packer)
}

/// The width of the markers riding `holder`'s first line
/// (`markers::line_markers`) packed `available` wide — an inside marker as
/// the inline text it is, an outside one taking no room — as followed by
/// the line's content: the letter and word spacing their last character
/// takes inside a line counts (CSS Text 3 §9.2 drops it only at the line's
/// end). Measured as the markers followed by a one-cell probe, less the
/// probe packed alone.
pub(crate) fn widest_marker_line(dom: &Dom<TuiExt>, holder: NodeId, available: u16) -> u16 {
    let markers = super::markers::line_markers(dom, holder);
    let Some(last_inside) = markers.iter().rev().find(|m| !m.outside).map(|m| m.item) else {
        return 0;
    };
    let run = super::feed::pseudo_run(dom, last_inside, crate::ext::PseudoSlot::Marker);
    let probe = |packer: &mut LinePacker<'_>| {
        packer.push_generated(last_inside, crate::ext::PseudoSlot::Marker, "x", run);
    };
    let mut with = LinePacker::measuring(available);
    for marker in markers {
        super::feed::push_marker(dom, marker, &mut with);
    }
    probe(&mut with);
    let mut alone = LinePacker::measuring(available);
    probe(&mut alone);
    widest(with).saturating_sub(widest(alone))
}

/// The widest line of the text node `text` packed alone `available` wide,
/// by its parent's CSS Text values; and the rows it packs to.
pub(crate) fn text_node_extent(dom: &Dom<TuiExt>, text: NodeId, available: u16) -> (u16, u16) {
    let parent = dom.node(text).parent_node().map_or(text, |p| p.id());
    let mut packer = LinePacker::measuring(available);
    fill_run(
        dom,
        parent,
        &[BoxItem::Node(text)],
        RunPseudos::default(),
        &mut packer,
    );
    packer.finish();
    let lines = packer.take_lines();
    let rows = lines.last().map_or(0, |l| l.bottom());
    let widest = lines
        .iter()
        .map(|line| line.width - line.hang)
        .max()
        .unwrap_or(0);
    (widest, rows)
}

fn widest(mut packer: LinePacker<'_>) -> u16 {
    packer.finish();
    packer
        .take_lines()
        .iter()
        .map(|line| {
            let width = i32::from(line.width - line.hang) + line.indent;
            width.clamp(0, i32::from(u16::MAX)) as u16
        })
        .max()
        .unwrap_or(0)
}

/// `block`'s `text-indent` for an intrinsic measurement: a percentage of
/// the size being measured is cyclic, so against 0 (CSS Sizing 3 §5.2.1).
fn indent_of(dom: &Dom<TuiExt>, block: NodeId, first: bool) -> super::indent::LineIndent {
    dom.node(block)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .map_or_else(super::indent::LineIndent::default, |c| {
            super::indent::LineIndent::of(&c.text.text_indent, 0, first)
        })
}
