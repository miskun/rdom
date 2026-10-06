//! Where a box's first and last baselines sit among its content rows
//! (CSS 2.1 §10.8.1: an inline-block's baseline is "the baseline of its
//! last line box in the normal flow"; CSS Box Alignment 3 §9.1 a box's
//! first and last baselines): a line box's glyph row, which leading
//! (`line-height`) moves away from the line's first and last rows.

use rdom_core::{Dom, NodeId};

use super::InlineLayout;
use crate::ext::TuiExt;
use crate::node::TuiNodeExt;
use crate::render::layout_pass::box_sizing::Sizer;
use crate::style::ComputedStyle;

impl InlineLayout {
    /// The glyph rows of its first and last lines, from its top; `None`
    /// with no line.
    pub(crate) fn baselines(&self) -> Option<(u16, u16)> {
        let first = self.lines.first()?.text_row();
        let last = self.lines.last()?.text_row();
        Some((first, last))
    }
}

/// Rows from `id`'s content top to its first baseline and from its last
/// baseline to its content bottom, its content box `width` cells wide:
/// the leading of its first and last lines — of the first and last
/// in-flow block children's, through their padding and border, in a
/// block container. `(0, 0)` where no line in it is taller than a row
/// (the content's first and last rows are its baselines), and in a flex
/// or grid container, whose baselines are its items' (C6-ALIGN).
pub(crate) fn insets(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    width: u16,
) -> (u16, u16) {
    if computed.flow.is_flex_or_grid() || !has_tall_lines(dom, id) {
        return (0, 0);
    }
    if !crate::render::box_tree::holds_block_box(dom, id) {
        let layout = super::compute_inline_layout(dom, id, width);
        return match layout.baselines() {
            Some((first, last)) => (first, layout.height().saturating_sub(last + 1)),
            None => (0, 0),
        };
    }
    let children: Vec<NodeId> = crate::render::layout_pass::element_children_of(dom, id)
        .into_iter()
        .filter(|&c| crate::render::layout_pass::is_in_flow(dom, c))
        .collect();
    let child_insets = |child: NodeId| {
        let c = dom.node(child).computed()?;
        let sizer = Sizer::horizontal(c, width);
        let (lead, trail) = insets(dom, child, c, width.saturating_sub(sizer.chrome()));
        let top = c
            .border
            .top
            .cells()
            .saturating_add(c.padding.top.resolve(width));
        let bottom = c
            .border
            .bottom
            .cells()
            .saturating_add(c.padding.bottom.resolve(width));
        Some((top.saturating_add(lead), bottom.saturating_add(trail)))
    };
    let lead = children
        .first()
        .and_then(|&c| child_insets(c))
        .map_or(0, |(lead, _)| lead);
    let trail = children
        .last()
        .and_then(|&c| child_insets(c))
        .map_or(0, |(_, trail)| trail);
    (lead, trail)
}

/// Whether a line in `id` may be taller than a row: its own line height,
/// or a descendant's or pseudo-element's, is.
fn has_tall_lines(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let tall = |c: Option<&ComputedStyle>| c.is_some_and(|c| c.text.line_height.rows() > 1);
    let node = dom.node(id);
    if tall(node.computed()) || tall(node.computed_before()) || tall(node.computed_after()) {
        return true;
    }
    node.child_nodes().any(|child| {
        child.node_type() == rdom_core::NodeType::Element && has_tall_lines(dom, child.id())
    })
}
