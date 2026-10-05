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
    let mut packer = LinePacker::measuring(available);
    fill_block(dom, block, &mut packer);
    widest(packer)
}

/// The widest line of the inline run `items` of `parent` (one
/// anonymous block box's content, CSS 2.1 §9.2.1.1), without `parent`'s
/// pseudo-elements, packed `available` wide.
pub(crate) fn widest_run_line(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    items: &[BoxItem],
    available: u16,
) -> u16 {
    let mut packer = LinePacker::measuring(available);
    fill_run(dom, parent, items, RunPseudos::default(), &mut packer);
    widest(packer)
}

fn widest(mut packer: LinePacker<'_>) -> u16 {
    packer.finish();
    packer
        .take_lines()
        .iter()
        .map(|line| line.width - line.hang)
        .max()
        .unwrap_or(0)
}
