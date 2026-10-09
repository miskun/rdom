//! The inline formatting context's half of the hit test: which inline
//! fragment's owner a point on a line of an IFC block is — through atomic
//! inlines, `pointer-events: none` and hidden inlines, and inert ones —
//! and the inline ancestors on the path between the block and it.

use rdom_core::{Dom, NodeId};

use super::descend::{has_inert_attribute, hit_in_flow_element};
use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::node::TuiNodeExt;
use crate::render::Rect;
use crate::render::box_tree::slot::parent;

/// Search the inline content of the IFC block `id` for `(x, y)`: the
/// inline fragment's owner under it, with its inline ancestors, pushed on
/// `path` (the block itself is the caller's). Returns whether a
/// descendant was appended.
pub(super) fn hit_inline_content(
    dom: &Dom<TuiExt>,
    id: NodeId,
    x: u16,
    y: u16,
    content_clip: Rect,
    viewport: Rect,
    path: &mut Vec<NodeId>,
) -> bool {
    let Some(layout) = dom.node(id).ext().and_then(|e| e.inline_layout.as_ref()) else {
        return false;
    };
    // Text rows are addressed through the *scrolled* content rect so
    // a scrolled IFC block resolves the owner visible on that row
    // (paint and the caret use the same rect).
    let outer = dom.node(id).layout_rect().unwrap_or_default();
    let inner = crate::render::inline::scrolled_content_rect(dom, id).unwrap_or(outer);
    let lines = Lines {
        block: id,
        layout,
        content: inner,
    };
    hit_lines(dom, &lines, x, y, (content_clip, viewport), path)
}

/// Search the lines of the anonymous block boxes of `id` — a block
/// container with block-level children, or the document root's initial
/// containing block — for `(x, y)` (CSS 2.1 §9.2.1.1: an inline run
/// among blocks is in a line box of an anonymous block box): as
/// [`hit_inline_content`] does an IFC block's.
pub(super) fn hit_anonymous_lines(
    dom: &Dom<TuiExt>,
    id: NodeId,
    x: u16,
    y: u16,
    clips: (Rect, Rect),
    path: &mut Vec<NodeId>,
) -> bool {
    if !clips.0.contains(x, y) {
        return false;
    }
    let (px, py) = (i32::from(x), i32::from(y));
    crate::render::box_tree::icb::anonymous_blocks(dom, id)
        .iter()
        // A `::before` / `::after` box is its host's (`pseudo`).
        .filter(|anon| anon.generated.is_none())
        .find(|anon| {
            let r = anon.rect;
            px >= r.x
                && px < r.x + i32::from(r.width)
                && py >= r.y
                && py < r.y + i32::from(r.height)
        })
        .is_some_and(|anon| {
            let lines = Lines {
                block: id,
                layout: &anon.inline_layout,
                content: anon.rect,
            };
            hit_lines(dom, &lines, x, y, clips, path)
        })
}

/// Line boxes of the block container `block`: an IFC block's own, or one
/// of its anonymous block boxes', laid out from `content`.
struct Lines<'a> {
    block: NodeId,
    layout: &'a crate::render::inline::InlineLayout,
    content: LayoutRect,
}

/// The owner of the inline fragment of `lines` under `(x, y)`, with its
/// inline ancestors, pushed on `path` (the block itself is the
/// caller's).
fn hit_lines(
    dom: &Dom<TuiExt>,
    lines: &Lines<'_>,
    x: u16,
    y: u16,
    (content_clip, viewport): (Rect, Rect),
    path: &mut Vec<NodeId>,
) -> bool {
    let id = lines.block;
    let Some((owner, atomic)) = hit_fragment(dom, lines, x, y) else {
        return false;
    };
    // An inline in an `inert` subtree below the block is inert (HTML
    // §6.3.1): the point is on the block's own line, as for a
    // `pointer-events: none` inline.
    let (owner, atomic) = match outermost_inert_below(dom, id, owner) {
        Some(inert) => (parent(dom, inert).unwrap_or(id), false),
        None => (owner, atomic),
    };
    // An atomic inline is a box (CSS 2.1 §9.2.2): it is hit as one —
    // its content searched, its own `visibility` / `pointer-events`
    // applied — under the inline ancestors it sits in.
    if atomic {
        let mark = path.len();
        let hit = hit_in_flow_element(dom, owner, x, y, content_clip, viewport, path);
        if hit && let Some(parent) = parent(dom, owner) {
            let chain = inline_ancestors(dom, id, parent);
            path.splice(mark..mark, chain);
        }
        return hit;
    }
    // `pointer-events: none` on an inline is transparent: the hit
    // resolves to the nearest ancestor (up to and including the
    // block) that accepts pointer events. If none does — the block
    // itself is transparent — the point falls through.
    let mut target = owner;
    // A hidden inline draws no text: the point is on what is beneath.
    while is_pointer_transparent(dom, target)
        || !crate::render::visibility::shows(dom, target, crate::ext::StyleSlot::Host)
    {
        if target == id {
            return false;
        }
        target = match parent(dom, target) {
            Some(p) => p,
            None => return false,
        };
    }
    if target == id {
        // The block's own text (or a transparent inline resolving
        // to it): the block was pushed by the caller, when it is
        // not transparent itself.
        return false;
    }
    append_inline_ancestors(dom, id, target, path);
    true
}

/// Look up the inline fragment under `(x, y)` in `lines`. Returns the
/// fragment's owner element (the direct element parent of the
/// underlying text — typically `<code>`, `<b>`,
/// or the IFC block itself when the text is a direct child; an atomic
/// inline for its fragment) and whether it is an atom.
fn hit_fragment(dom: &Dom<TuiExt>, lines: &Lines<'_>, x: u16, y: u16) -> Option<(NodeId, bool)> {
    let (ifc_block, layout, content) = (lines.block, lines.layout, lines.content);

    // A relatively positioned or sticky run moved off its line paints
    // over the lines (CSS 2.1 §9.4.3): it is hit first, for its host —
    // the block itself when it is the block's own.
    let (row_i, x_i) = (y as i32 - content.y, x as i32 - content.x);
    for line in &layout.lines {
        let moved = line.generated.iter().find(|g| {
            let (dx, dy) = g.offset;
            (dx, dy) != (0, 0)
                && !g.is_atom()
                && row_i == i32::from(line.top) + i32::from(g.y) + dy
                && x_i >= g.x + dx
                && x_i < g.x + dx + i32::from(g.width)
        });
        if let Some(g) = moved {
            return is_descendant(dom, g.host, ifc_block).then_some((g.host, false));
        }
    }

    // The line box spanning the row (CSS 2.1 §10.8: a line is as tall
    // as its tallest atom).
    let row = u16::try_from(y as i32 - content.y).ok()?;
    // Local x within content — negative left of it, where an overflowing
    // `rtl` line's start sits.
    let x_local = x as i32 - content.x;
    let line = &layout.lines[layout.line_at(x_local, row)?];

    for fragment in &line.fragments {
        if x_local >= fragment.x
            && x_local < fragment.x + i32::from(fragment.width)
            && line.covers(fragment, row)
        {
            return Some((fragment.node, fragment.atomic));
        }
    }
    // A pseudo-element is part of its host's box: a generated cell of
    // an inline descendant's `::before` / `::after` targets that host
    // (a click on an `<a>`'s marker follows the link), unless the
    // pseudo itself is `pointer-events: none`. The block's own pseudos
    // (and list markers) resolve to the block, which the caller holds.
    line.generated
        .iter()
        .filter(|g| g.offset == (0, 0))
        .find(|g| x_local >= g.x && x_local < g.x + i32::from(g.width))
        .filter(|g| is_descendant(dom, g.host, ifc_block))
        .filter(|g| {
            let pseudo = dom.node(g.host).computed_pseudo(g.slot);
            pseudo.is_none_or(|c| c.pointer_events != crate::layout::PointerEvents::None)
                && crate::render::visibility::shows(dom, g.host, g.slot.into())
        })
        .map(|g| (g.host, false))
}

/// `id` is a strict descendant of `ancestor`.
fn is_descendant(dom: &Dom<TuiExt>, id: NodeId, ancestor: NodeId) -> bool {
    let mut cur = parent(dom, id);
    while let Some(n) = cur {
        if n == ancestor {
            return true;
        }
        cur = parent(dom, n);
    }
    false
}

/// The outermost element from `owner` (inclusive) up to `block`
/// (exclusive) that carries the `inert` attribute, if any.
fn outermost_inert_below(dom: &Dom<TuiExt>, block: NodeId, owner: NodeId) -> Option<NodeId> {
    let mut found = None;
    let mut cur = Some(owner);
    while let Some(n) = cur
        && n != block
    {
        if has_inert_attribute(dom, n) {
            found = Some(n);
        }
        cur = parent(dom, n);
    }
    found
}

/// `id` is `pointer-events: none`.
fn is_pointer_transparent(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    dom.node(id)
        .computed()
        .is_some_and(|c| c.pointer_events == crate::layout::PointerEvents::None)
}

/// Walk the ancestor chain from `owner` up to (but not including)
/// `ifc_block`. Append each to `path` in outer → inner order so the
/// final path stays document-ordered.
fn append_inline_ancestors(
    dom: &Dom<TuiExt>,
    ifc_block: NodeId,
    owner: NodeId,
    path: &mut Vec<NodeId>,
) {
    path.extend(inline_ancestors(dom, ifc_block, owner));
}

/// `owner` and its ancestors below `ifc_block`, outer → inner, without
/// the `pointer-events: none` ones (see [`append_inline_ancestors`]).
fn inline_ancestors(dom: &Dom<TuiExt>, ifc_block: NodeId, owner: NodeId) -> Vec<NodeId> {
    // Collect inner → outer first, then reverse. A transparent inline
    // ancestor is never on the path.
    let mut chain = Vec::new();
    let mut cur = owner;
    while cur != ifc_block {
        if !is_pointer_transparent(dom, cur) {
            chain.push(cur);
        }
        match parent(dom, cur) {
            Some(parent) => cur = parent,
            None => break, // defensive — should never trigger in a well-formed tree
        }
    }
    chain.reverse();
    chain
}
