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
//!
//! A `::before` / `::after` that is a list item has a marker of its own
//! (`::before::marker`, CSS Pseudo-Elements 4 §4): it rides the first of
//! the lines the pseudo-element's box packs (`inline::pack_generated`),
//! an outside one hung beside that box ([`place_outside_of_box`]).
//!
//! An inline list item — `display: inline list-item`, an element or a
//! `::before` / `::after` — has its marker as its own first inline box
//! ([`inline_marker`], [`inline_pseudo_marker`]): §3.5 makes `outside`
//! equivalent to `inside` for an inline box.

use rdom_core::{Dom, NodeId};

use super::InlineLayout;
use super::generated::{
    before_is_inline_content, is_block_flow_container, is_block_level, line_bearing_child,
};
use crate::ext::{PseudoSlot, TuiExt};
use crate::layout::{Display, LayoutRect, ListStylePosition, MarkerSide, TextDirection};
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;

#[cfg(test)]
#[path = "markers_tests.rs"]
mod tests;

/// A list item's marker: the item (the host, for a list-item `::before`
/// / `::after`), the marker's slot (`Marker`, `BeforeMarker` or
/// `AfterMarker`), its text (its `content`) and whether it hangs outside.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Marker<'a> {
    pub(crate) item: NodeId,
    pub(crate) slot: PseudoSlot,
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
        slot: PseudoSlot::Marker,
        text,
        outside: computed.list_style_position == ListStylePosition::Outside,
    })
}

/// The marker of `host`'s `slot` pseudo-element (`Before` / `After`) when
/// that box is a list item whose `::before::marker` / `::after::marker`
/// has content (CSS Pseudo-Elements 4 §4, CSS Lists 3 §3.1) — a block
/// container, which packs lines of its own, as [`marker`] asks of an
/// element.
pub(crate) fn pseudo_marker(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
) -> Option<Marker<'_>> {
    let marker_slot = match slot {
        PseudoSlot::Before => PseudoSlot::BeforeMarker,
        PseudoSlot::After => PseudoSlot::AfterMarker,
        _ => return None,
    };
    let pseudo = dom.node(host).computed_pseudo(slot)?;
    if pseudo.display != Display::Block || !pseudo.flow.is_block_flow() {
        return None;
    }
    list_item_pseudo_marker(dom, host, slot, marker_slot)
}

/// The marker of `host`'s `slot` pseudo-element (`Before` / `After`) when
/// that box is an inline list item (`display: inline list-item`) whose
/// nested marker has content: the pseudo-element's first inline box,
/// always `inside` (CSS Lists 3 §3.5: "If the list item is an inline
/// box, this value is equivalent to `inside`").
pub(crate) fn inline_pseudo_marker(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
) -> Option<Marker<'_>> {
    let marker_slot = match slot {
        PseudoSlot::Before => PseudoSlot::BeforeMarker,
        PseudoSlot::After => PseudoSlot::AfterMarker,
        _ => return None,
    };
    if dom.node(host).computed_pseudo(slot)?.display != Display::Inline {
        return None;
    }
    let marker = list_item_pseudo_marker(dom, host, slot, marker_slot)?;
    Some(Marker {
        outside: false,
        ..marker
    })
}

/// The `marker_slot` marker of `host`'s `slot` pseudo-element when that
/// box is a list item whose nested marker has content, whatever its
/// display.
fn list_item_pseudo_marker(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
    marker_slot: PseudoSlot,
) -> Option<Marker<'_>> {
    let ext = dom.node(host).ext()?;
    let pseudo = ext.computed_pseudo(slot)?;
    if !pseudo.list_item {
        return None;
    }
    let text = ext.computed_pseudo(marker_slot)?.content.as_deref()?;
    Some(Marker {
        item: host,
        slot: marker_slot,
        text,
        outside: pseudo.list_style_position == ListStylePosition::Outside,
    })
}

/// `item`'s marker when it is an inline list item (`display: inline
/// list-item`, an inline box) whose `::marker` has content (CSS Display 3
/// §2.3, CSS Lists 3 §3.1): its first inline box, always `inside`
/// (§3.5: `outside` "is equivalent to `inside`" for an inline box).
pub(crate) fn inline_marker(dom: &Dom<TuiExt>, item: NodeId) -> Option<Marker<'_>> {
    let node = dom.node(item);
    let computed = node.computed()?;
    if !computed.list_item || computed.display != Display::Inline {
        return None;
    }
    let text = node.computed_marker()?.content.as_deref()?;
    Some(Marker {
        item,
        slot: PseudoSlot::Marker,
        text,
        outside: false,
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

/// Whether `item`'s `slot` outside marker hangs on the right: `marker-side:
/// match-self` takes the list item's `direction`, `match-parent` its
/// parent's (CSS Lists 3 §3.6) — for the marker of a list-item `::before`
/// / `::after`, the pseudo-element's and its host's.
pub(crate) fn hangs_right(dom: &Dom<TuiExt>, item: NodeId, slot: PseudoSlot) -> bool {
    let node = dom.node(item);
    let (list_item, parent) = match slot {
        PseudoSlot::BeforeMarker | PseudoSlot::AfterMarker => {
            let pseudo = match slot {
                PseudoSlot::BeforeMarker => PseudoSlot::Before,
                _ => PseudoSlot::After,
            };
            (node.computed_pseudo(pseudo), node.computed())
        }
        _ => (
            node.computed(),
            crate::render::box_tree::slot::parent(dom, item).and_then(|p| dom.node(p).computed()),
        ),
    };
    let Some(computed) = list_item else {
        return false;
    };
    let direction = match computed.marker_side {
        MarkerSide::MatchParent => parent.map_or(TextDirection::Ltr, |c| c.text_direction),
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
    place(dom, layout, content_x, |host| dom.node(host).layout_rect());
}

/// [`place_outside`] for the lines a list-item `::before` / `::after`
/// packs itself (`inline::pack_generated`): its marker hangs beside
/// `border_box`, the pseudo-element's box, in the coordinates of
/// `content_x`, its lines' content edge.
pub(crate) fn place_outside_of_box(
    dom: &Dom<TuiExt>,
    layout: &mut InlineLayout,
    content_x: i32,
    border_box: LayoutRect,
) {
    place(dom, layout, content_x, |_| Some(border_box));
}

/// Give each outside marker on `layout`'s lines its column against its
/// list item's border box, `item_box` of the marker's host.
fn place(
    dom: &Dom<TuiExt>,
    layout: &mut InlineLayout,
    content_x: i32,
    item_box: impl Fn(NodeId) -> Option<LayoutRect>,
) {
    for g in layout.lines.iter_mut().flat_map(|l| l.generated.iter_mut()) {
        let Some(outside) = g.outside else {
            continue;
        };
        let Some(item) = item_box(g.host) else {
            continue;
        };
        let base = if hangs_right(dom, g.host, g.slot) {
            item.x + i32::from(item.width) - content_x
        } else {
            item.x - content_x - i32::from(outside.width)
        };
        g.x = base + outside.offset;
    }
}
