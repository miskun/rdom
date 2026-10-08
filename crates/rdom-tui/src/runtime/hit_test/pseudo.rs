//! Which pseudo-element is under a point —
//! [`HitTestExt::hit_test_pseudo`](super::HitTestExt::hit_test_pseudo),
//! what `::before:hover` reads (Selectors 4 §3.6.3).
//!
//! A pseudo-element has no node: the element hit test (`descend`)
//! resolves a point on one to its host, which it names as the deepest
//! element (CSS Pseudo 4 §2). Given that element, this finds the
//! generated box itself among those the layout recorded: a positioned or
//! floated box, a block-level one, or a run or atom on a line. Two
//! pseudo-elements sit on lines of a block other than their host's box:
//! a list item's `::marker` rides a descendant's first line, and a
//! block's `::first-letter` its first line — their host may be an
//! ancestor of the hit. A list-item `::before` / `::after`'s own marker
//! rides that box's lines. The innermost pseudo-element under the point
//! wins (a `::first-letter` inside a `::before`'s text).

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{AnonymousIfc, PseudoSlot, TuiExt};
use crate::layout::LayoutRect;
use crate::node::TuiNodeExt;
use crate::render::inline::{GeneratedFragment, InlineLayout, LineBox};

/// The pseudo-element under `(x, y)`, given the element the hit test
/// found there (`target`): its host and slot.
pub(crate) fn pseudo_at(
    dom: &Dom<TuiExt>,
    target: NodeId,
    x: u16,
    y: u16,
) -> Option<(NodeId, PseudoSlot)> {
    let (x, y) = (i32::from(x), i32::from(y));
    // Boxes of their own: positioned ones on the host, floated and
    // block-level ones on a box at or above it (the formatting context
    // run, the box parent of a box-less host).
    let own = dom.node(target).ext()?;
    if let Some(hit) = own
        .positioned_pseudo_boxes()
        .iter()
        .rev()
        .find_map(|anon| in_box(dom, anon, target, x, y))
    {
        return Some(hit);
    }
    for id in ancestors_or_self(dom, target) {
        let Some(ext) = dom.node(id).ext() else {
            continue;
        };
        let boxes = ext.floated_pseudos().iter().chain(&ext.anonymous_blocks);
        if let Some(hit) = boxes.rev().find_map(|anon| in_box(dom, anon, target, x, y)) {
            return Some(hit);
        }
    }
    // Runs and atoms on lines: the lines of every block at or above the
    // hit, and of the block a list item's marker rides.
    for id in ancestors_or_self(dom, target) {
        if let Some(hit) = in_lines_of(dom, id, target, x, y) {
            return Some(hit);
        }
    }
    let holder = crate::render::inline::markers::marker_line_holder(dom, target)?;
    (holder != target)
        .then(|| in_lines_of(dom, holder, target, x, y))
        .flatten()
}

/// Whether `(x, y)`, inside `clip`, is on the outside `::marker` of the
/// list item `item` (CSS Lists 3 §3.5): a box of the item hung outside its
/// principal box, on the row of the first line it rides — `item`'s own or
/// a descendant's. The element hit test asks this at the item's turn when
/// the point misses the item's box, so a click on a bullet targets the
/// item, as in a browser (C10G-MARKER-HIT). Free in a document with no
/// list item.
pub(super) fn on_outside_marker(
    dom: &Dom<TuiExt>,
    item: NodeId,
    x: u16,
    y: u16,
    clip: crate::render::Rect,
) -> bool {
    use crate::render::inline::markers;
    if !crate::style::doc_flags::has_list_items(dom) || !clip.contains(x, y) {
        return false;
    }
    if !markers::marker(dom, item).is_some_and(|m| m.outside) {
        return false;
    }
    let Some(holder) = markers::marker_line_holder(dom, item) else {
        return false;
    };
    let (x, y) = (i32::from(x), i32::from(y));
    in_lines_of(dom, holder, item, x, y) == Some((item, PseudoSlot::Marker))
}

/// `id` and its element ancestors, innermost first.
fn ancestors_or_self(dom: &Dom<TuiExt>, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
    std::iter::successors(Some(id), move |&cur| {
        dom.node(cur)
            .parent_node()
            .filter(|p| p.node_type() == NodeType::Element)
            .map(|p| p.id())
    })
}

/// `anon`, a generated box of `target`'s containing `(x, y)` — or the
/// marker of its own that a list-item `::before` / `::after` carries on
/// its lines (CSS Pseudo-Elements 4 §4), beside the box when outside: the
/// innermost pseudo-element there.
fn in_box(
    dom: &Dom<TuiExt>,
    anon: &AnonymousIfc,
    target: NodeId,
    x: i32,
    y: i32,
) -> Option<(NodeId, PseudoSlot)> {
    let g = anon.generated?;
    if g.host != target {
        return None;
    }
    let nested = in_layout(dom, &anon.inline_layout, anon.rect, target, (x, y), false);
    if let Some(hit @ (_, PseudoSlot::BeforeMarker | PseudoSlot::AfterMarker)) = nested {
        return Some(hit);
    }
    (contains(g.border_box, x, y) && targets(dom, g.host, g.slot)).then_some((g.host, g.slot))
}

/// A pseudo-element of (or, for a marker or first letter, above)
/// `target` on a line of `block`'s — its own inline layout or one of its
/// anonymous block boxes'.
fn in_lines_of(
    dom: &Dom<TuiExt>,
    block: NodeId,
    target: NodeId,
    x: i32,
    y: i32,
) -> Option<(NodeId, PseudoSlot)> {
    let ext = dom.node(block).ext()?;
    // A relatively positioned `::before` / `::after` in these lines is
    // drawn off its line's rows (`GeneratedFragment::offset`): only then
    // is every line looked at.
    let moved = ext.tree_has_positioned_pseudo;
    if let Some(layout) = ext.inline_layout.as_ref()
        && let Some(content) = crate::render::inline::scrolled_content_rect(dom, block)
        && let Some(hit) = in_layout(dom, layout, content, target, (x, y), moved)
    {
        return Some(hit);
    }
    ext.anonymous_blocks
        .iter()
        .filter(|anon| anon.generated.is_none())
        .find_map(|anon| in_layout(dom, &anon.inline_layout, anon.rect, target, (x, y), moved))
}

/// The pseudo-element under `(x, y)` on `layout`'s lines (laid out at
/// `content`) that belongs to `target`: on the line at the point's row
/// (`InlineLayout::line_at_row`), or — when a generated run was `moved`
/// off its line — on any line.
fn in_layout(
    dom: &Dom<TuiExt>,
    layout: &InlineLayout,
    content: LayoutRect,
    target: NodeId,
    (x, y): (i32, i32),
    moved: bool,
) -> Option<(NodeId, PseudoSlot)> {
    let (x, row) = (x - content.x, y - content.y);
    let lines = if moved {
        &layout.lines[..]
    } else {
        match u16::try_from(row).ok().and_then(|r| layout.line_at_row(r)) {
            Some(i) => std::slice::from_ref(&layout.lines[i]),
            None => &[],
        }
    };
    for line in lines {
        #[cfg(test)]
        cost::LINES_SCANNED.with(|c| c.set(c.get() + 1));
        // A first letter in the element's own text (CSS Pseudo 4 §2.3).
        let letter = line
            .fragments
            .iter()
            .filter(|f| x >= f.x && x < f.x + i32::from(f.width))
            .filter(|f| u16::try_from(row).is_ok_and(|r| line.covers(f, r)))
            .find_map(|f| f.first_letter);
        if let Some(block) = letter
            && owns(dom, block, PseudoSlot::FirstLetter, target)
        {
            return Some((block, PseudoSlot::FirstLetter));
        }
        for g in line.generated.iter().filter(|g| on(line, g, x, row)) {
            if let Some(block) = g.first_letter
                && owns(dom, block, PseudoSlot::FirstLetter, target)
            {
                return Some((block, PseudoSlot::FirstLetter));
            }
            if owns(dom, g.host, g.slot, target) {
                return Some((g.host, g.slot));
            }
        }
    }
    None
}

/// `g`, one of `line`'s generated runs or atoms, covers the cell at
/// `(x, row)` of its layout — where it was moved to, if it was
/// (`GeneratedFragment::offset`).
fn on(line: &LineBox, g: &GeneratedFragment, x: i32, row: i32) -> bool {
    let (dx, dy) = g.offset;
    let left = g.x + dx;
    if x < left || x >= left + i32::from(g.width) {
        return false;
    }
    let top = i32::from(line.top) + dy;
    match g.atom_rows() {
        Some((first, count)) => {
            let first = top + i32::from(first);
            row >= first && row < first + i32::from(count)
        }
        None => row == top + i32::from(g.y),
    }
}

/// Whether `host`'s `slot` pseudo-element is the one meant by a point
/// whose hit is `target`: `target` is its host — or, for a marker or a
/// first letter, which ride a descendant's line, inside its host — and
/// it is a pointer target.
fn owns(dom: &Dom<TuiExt>, host: NodeId, slot: PseudoSlot, target: NodeId) -> bool {
    let placed = host == target
        || (matches!(slot, PseudoSlot::Marker | PseudoSlot::FirstLetter)
            && ancestors_or_self(dom, target).any(|a| a == host));
    placed && targets(dom, host, slot)
}

/// `host`'s `slot` pseudo-element is drawn and takes pointer events.
fn targets(dom: &Dom<TuiExt>, host: NodeId, slot: PseudoSlot) -> bool {
    let style = dom.node(host).computed_pseudo(slot);
    style.is_none_or(|c| c.pointer_events != crate::layout::PointerEvents::None)
        && crate::render::visibility::shows(dom, host, slot.into())
}

fn contains(r: LayoutRect, x: i32, y: i32) -> bool {
    x >= r.x && x < r.x + i32::from(r.width) && y >= r.y && y < r.y + i32::from(r.height)
}

/// Test-only counters of the pseudo hit test.
#[cfg(test)]
pub(crate) mod cost {
    thread_local! {
        /// Line boxes the pseudo hit test looked at.
        pub(crate) static LINES_SCANNED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
}
