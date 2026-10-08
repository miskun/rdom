//! A list item's marker (CSS Lists 3 §3): where its `::marker` is laid
//! out. The marker rides the item's first line box — the item's own, or
//! the first line of the block descendant that holds it
//! (`<li><p>Step</p></li>` reads "1. Step"; `<li><ol><li>x` puts both
//! markers on the inner item's line) — and is pushed to the packer with
//! the `::before` of that line's block (`feed::push_pseudo`):
//!
//! - `inside` (§3.5): as the line's first inline box, generated text
//!   like a `::before`'s;
//! - `outside`: as a box beside the line, which takes no room in it — its
//!   end at the item's inline-start border edge, the side `marker-side`
//!   names (§3.6). The packer keeps its fragments apart from the line's
//!   content (`push_outside_marker`); the layout pass gives them their
//!   column once the item's box is placed ([`place_outside`]).

use rdom_core::{Dom, NodeId};

use super::InlineLayout;
use super::generated::{
    before_is_inline_content, is_block_flow_container, is_block_level, line_bearing_child,
};
use crate::ext::TuiExt;
use crate::layout::{Display, ListStylePosition, MarkerSide, TextDirection};
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;

#[cfg(test)]
#[path = "markers_tests.rs"]
mod tests;

/// A list item's marker: the item, the marker's text (its `::marker`
/// `content`) and whether it hangs outside.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Marker<'a> {
    pub(crate) item: NodeId,
    pub(crate) text: &'a str,
    pub(crate) outside: bool,
}

/// `item`'s marker, when it is a list item (`display: list-item`) of block
/// flow whose `::marker` has content (CSS Lists 3 §3.1–§3.2). (A list item
/// is never a flex or grid container: CSS Display 3 §2.3 makes `list-item`
/// with `flex` or `grid` invalid.)
pub(crate) fn marker(dom: &Dom<TuiExt>, item: NodeId) -> Option<Marker<'_>> {
    let node = dom.node(item);
    let computed = node.computed()?;
    if !computed.list_item || computed.display == Display::None {
        return None;
    }
    if !is_block_flow_container(dom, item) {
        return None;
    }
    let text = node.computed_marker()?.content.as_deref()?;
    Some(Marker {
        item,
        text,
        outside: computed.list_style_position == ListStylePosition::Outside,
    })
}

/// The block whose first line `item`'s marker rides: the block that holds
/// the item's first line box — `item` itself when it has none to reach
/// (the marker then makes the item's first line).
pub(crate) fn marker_line_holder(dom: &Dom<TuiExt>, item: NodeId) -> Option<NodeId> {
    marker(dom, item)?;
    Some(first_line_holder(dom, item).unwrap_or(item))
}

/// Whether `item`'s marker rides `item`'s own first line — its first
/// line box is its own, or it has none and the marker makes one.
pub(crate) fn makes_own_line(dom: &Dom<TuiExt>, item: NodeId) -> bool {
    marker_line_holder(dom, item) == Some(item)
}

/// The block holding `el`'s first line box: `el` when its first inline
/// content — its `::before`, or its first line-bearing child — is
/// inline-level; a block-level first child passes the line down. `None`
/// when no line box is reachable (an empty block, a flex container).
fn first_line_holder(dom: &Dom<TuiExt>, el: NodeId) -> Option<NodeId> {
    if !is_block_flow_container(dom, el) {
        return None;
    }
    if before_is_inline_content(dom, el) {
        return Some(el);
    }
    match line_bearing_child(dom, el, false)? {
        BoxItem::Node(first) if is_block_level(dom, BoxItem::Node(first)) => {
            first_line_holder(dom, first)
        }
        _ => Some(el),
    }
}

/// The markers riding `holder`'s first line, outermost item first:
/// `holder`'s own, and those of the ancestors whose first line it holds.
///
/// A document with no list item has none to place: the climb is skipped
/// (`style::doc_flags`), and each step finds the parent's first
/// line-bearing child from its start, so a row among thousands of
/// siblings costs a step, not a walk of them (C10G-MARKER-COST).
pub(crate) fn line_markers(dom: &Dom<TuiExt>, holder: NodeId) -> Vec<Marker<'_>> {
    if !crate::style::doc_flags::has_list_items(dom) {
        return Vec::new();
    }
    let mut items = Vec::new();
    if marker_line_holder(dom, holder) == Some(holder) {
        items.push(holder);
    }
    // Climb while `cur` is the first line-bearing child of a block-flow
    // parent whose own `::before` does not come first: only along that
    // path can an ancestor's first line be `holder`'s.
    let mut cur = holder;
    while let Some(parent) = crate::render::box_tree::box_parent(dom, cur) {
        #[cfg(test)]
        tests::CLIMB_STEPS.with(|c| c.set(c.get() + 1));
        if !is_block_flow_container(dom, parent)
            || before_is_inline_content(dom, parent)
            || line_bearing_child(dom, parent, false) != Some(BoxItem::Node(cur))
        {
            break;
        }
        if marker_line_holder(dom, parent) == Some(holder) {
            items.push(parent);
        }
        cur = parent;
    }
    items.reverse();
    items.into_iter().filter_map(|i| marker(dom, i)).collect()
}

/// Whether `item`'s outside marker hangs on the right: `marker-side:
/// match-self` takes the item's `direction`, `match-parent` its parent's
/// (CSS Lists 3 §3.6).
pub(crate) fn hangs_right(dom: &Dom<TuiExt>, item: NodeId) -> bool {
    let node = dom.node(item);
    let Some(computed) = node.computed() else {
        return false;
    };
    let direction = match computed.marker_side {
        MarkerSide::MatchParent => node
            .parent_node()
            .and_then(|p| p.computed().map(|c| c.text_direction))
            .unwrap_or(TextDirection::Ltr),
        _ => computed.text_direction,
    };
    direction == TextDirection::Rtl
}

/// `text`, laid out left to right as the bidi algorithm orders an
/// isolated marker hanging on the right of right-to-left text (UAX #9
/// L2): its runs of letters and digits kept, the runs reversed — `"10. "`
/// reads `" .10"`, `"• "` reads `" •"`. rdom draws text left to right.
pub(crate) fn visual_rtl(text: &str) -> String {
    let mut runs: Vec<String> = Vec::new();
    let mut word = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() {
            word.push(c);
        } else {
            if !word.is_empty() {
                runs.push(std::mem::take(&mut word));
            }
            runs.push(c.to_string());
        }
    }
    if !word.is_empty() {
        runs.push(word);
    }
    runs.reverse();
    runs.concat()
}

/// Give the outside markers on `layout`'s lines their column (CSS Lists 3
/// §3.5): each marker's end at its item's inline-start border edge — its
/// start at the right edge when it hangs right — `content_x` being the
/// absolute column of `layout`'s content edge. The item's box is placed
/// before the lines of its first line's block are packed (a box's rect is
/// written before its children are laid out).
pub(crate) fn place_outside(dom: &Dom<TuiExt>, layout: &mut InlineLayout, content_x: i32) {
    for g in layout.lines.iter_mut().flat_map(|l| l.generated.iter_mut()) {
        let Some(outside) = g.outside else {
            continue;
        };
        let Some(item) = dom.node(g.host).layout_rect() else {
            continue;
        };
        let base = if hangs_right(dom, g.host) {
            item.x + i32::from(item.width) - content_x
        } else {
            item.x - content_x - i32::from(outside.width)
        };
        g.x = base + outside.offset;
    }
}
