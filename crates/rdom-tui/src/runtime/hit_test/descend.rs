//! The stacking-context walk — `(x, y)` → full ancestor path.
//!
//! This is the reverse-paint traversal behind
//! [`HitTestExt::hit_test_path`](super::HitTestExt::hit_test_path):
//! stacking contexts and their layers (`hit_stacking_context`,
//! `hit_layers`), plain boxes and their in-flow content
//! (`descend_plain`, `hit_content`, `descend_children_reverse`), the
//! inline-fragment owner lookup inside an IFC (`hit_fragment`,
//! `append_inline_ancestors`), and `pointer-events` transparency.
//! Nothing here knows about text positions; that is `fragment.rs`.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::node::TuiNodeExt;
use crate::render::Rect;
use crate::render::inline::has_inline_layout;
use crate::render::stacking::{
    LayerEntry, children_clip, collect_layers, creates_stacking_context, is_layered,
};
use crate::style::ComputedStyle;

/// Append nodes to `path` if `(x, y)` lands inside the subtree
/// rooted at `id`. Returns `true` when at least one node was added
/// at this level or deeper (lets the caller skip trying earlier
/// siblings).
/// Hit-test the stacking context rooted at `root` in reverse paint
/// order: child contexts with positive `z-index` (highest first), the
/// `z-index: auto | 0` layer in reverse tree order, the root's in-flow
/// content, child contexts with negative `z-index`, and finally the
/// root's own box. `clip` is the region the context paints into.
pub(super) fn hit_stacking_context(
    dom: &Dom<TuiExt>,
    root: NodeId,
    x: u16,
    y: u16,
    clip: Rect,
    viewport: Rect,
    path: &mut Vec<NodeId>,
) -> bool {
    if !clip.contains(x, y) {
        return false;
    }
    let root_box = element_box(dom, root);
    if dom.node(root).node_type() == NodeType::Element && root_box.is_none() {
        return false; // `display: none`
    }
    let content_clip = root_box
        .as_ref()
        .map_or(clip, |(c, _)| children_clip(dom, root, c, clip));
    let layers = collect_layers(dom, root, content_clip, viewport);
    // A hit inside a layer reports the full ancestor chain: the
    // entries between this root and the hit (`hit_layers`), and the
    // root itself, ahead of what the layer pushed.
    let mark = path.len();
    let transparent = root_box
        .as_ref()
        .is_some_and(|(c, _)| c.pointer_events == crate::layout::PointerEvents::None);
    let root_in_path = |path: &mut Vec<NodeId>| {
        if root_box.is_some() && !transparent {
            path.insert(mark, root);
        }
    };
    if hit_layers(dom, root, &layers.positive, x, y, viewport, path)
        || hit_layers(dom, root, &layers.zero_auto, x, y, viewport, path)
    {
        root_in_path(path);
        return true;
    }
    let Some((_, outer)) = root_box else {
        // The document root: in-flow content, then the negative layer.
        return descend_children_reverse(dom, root, x, y, content_clip, viewport, path)
            || hit_layers(dom, root, &layers.negative, x, y, viewport, path);
    };
    let contains = rect_contains(outer, x, y);
    if contains && content_clip.contains(x, y) {
        if !transparent {
            path.push(root);
        }
        if hit_content(dom, root, x, y, content_clip, viewport, path) {
            return true;
        }
        path.truncate(mark);
    }
    if hit_layers(dom, root, &layers.negative, x, y, viewport, path) {
        root_in_path(path);
        return true;
    }
    // A hidden root (CSS Display 3 §4) is on a descendant's path but is
    // no target of its own.
    let hidden = !crate::render::visibility::shows(dom, root, crate::ext::StyleSlot::Host);
    if contains && !transparent && !hidden {
        path.push(root);
        return true;
    }
    false
}

/// Try the entries of one layer in reverse paint order. On a hit, the
/// ancestors between the context `root` (exclusive) and the entry are
/// inserted ahead of what the entry pushed, so the path stays the full
/// ancestor chain.
fn hit_layers(
    dom: &Dom<TuiExt>,
    root: NodeId,
    entries: &[LayerEntry],
    x: u16,
    y: u16,
    viewport: Rect,
    path: &mut Vec<NodeId>,
) -> bool {
    for e in entries.iter().rev() {
        let mark = path.len();
        let hit = if e.context {
            hit_stacking_context(dom, e.id, x, y, e.clip, viewport, path)
        } else {
            descend_plain(dom, e.id, x, y, e.clip, viewport, path)
        };
        if hit {
            let mut chain = Vec::new();
            let mut cur = dom.node(e.id).parent_node().map(|p| p.id());
            while let Some(id) = cur {
                if id == root {
                    break;
                }
                if dom.node(id).node_type() == NodeType::Element {
                    chain.push(id);
                }
                cur = dom.node(id).parent_node().map(|p| p.id());
            }
            chain.reverse();
            path.splice(mark..mark, chain);
            return true;
        }
    }
    false
}

/// An element's style and outer rect; `None` for non-elements and
/// `display: none` (CSS Display 3 §2.5: no box — the layout pass may
/// leave a stale rect behind, which must not catch clicks).
fn element_box(dom: &Dom<TuiExt>, id: NodeId) -> Option<(std::rc::Rc<ComputedStyle>, LayoutRect)> {
    let node = dom.node(id);
    if node.node_type() != NodeType::Element {
        return None;
    }
    let computed = node
        .computed_rc()
        .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
    if matches!(computed.display, crate::layout::Display::None) {
        return None;
    }
    let outer = node.layout_rect()?;
    Some((computed, outer))
}

/// Hit-test an element as a plain box: its outer rect must contain the
/// point; its content is searched inside its content clip. Used for
/// in-flow elements and for `z-index: auto` positioned boxes, whose
/// positioned descendants belong to the enclosing context's layers.
/// Returns whether the element or a descendant was appended to `path`.
fn descend_plain(
    dom: &Dom<TuiExt>,
    id: NodeId,
    x: u16,
    y: u16,
    clip: Rect,
    viewport: Rect,
    path: &mut Vec<NodeId>,
) -> bool {
    if !clip.contains(x, y) {
        return false;
    }
    let Some((computed, outer)) = element_box(dom, id) else {
        return false;
    };
    if !rect_contains(outer, x, y) {
        return false;
    }
    let content_clip = children_clip(dom, id, &computed, clip);

    // `pointer-events: none`: the element is transparent to the
    // pointer. Its subtree is still searched — a descendant that sets
    // `pointer-events: auto` is a target — but the element itself is
    // never on the path; with no hittable descendant the point falls
    // through to earlier siblings / the parent.
    if computed.pointer_events == crate::layout::PointerEvents::None {
        return content_clip.contains(x, y)
            && hit_content(dom, id, x, y, content_clip, viewport, path);
    }
    // `visibility: hidden` (CSS Display 3 §4): the box draws nothing and
    // is no target, but a `visible` descendant is — with the hidden
    // element on its ancestor path, as the DOM has it.
    if !crate::render::visibility::shows(dom, id, crate::ext::StyleSlot::Host) {
        let mark = path.len();
        let hit =
            content_clip.contains(x, y) && hit_content(dom, id, x, y, content_clip, viewport, path);
        if hit {
            path.insert(mark, id);
        }
        return hit;
    }

    path.push(id);
    // Overflow clipping (CSS Overflow 3 §3 = padding-box; the same rect
    // paint clips to): outside the scrollport the hit stays on THIS
    // element's padding / border — no descent.
    if content_clip.contains(x, y) {
        hit_content(dom, id, x, y, content_clip, viewport, path);
    }
    true
}

/// Search an element's content for the point: the inline fragment's
/// owner inside an inline-flow container, otherwise the in-flow
/// children in reverse tree order. Returns whether a descendant was
/// appended to `path`.
fn hit_content(
    dom: &Dom<TuiExt>,
    id: NodeId,
    x: u16,
    y: u16,
    content_clip: Rect,
    viewport: Rect,
    path: &mut Vec<NodeId>,
) -> bool {
    if has_inline_layout(dom, id) {
        // Text rows are addressed through the *scrolled* content rect so
        // a scrolled IFC block resolves the owner visible on that row
        // (paint and the caret use the same rect).
        let outer = dom.node(id).layout_rect().unwrap_or_default();
        let inner = crate::render::inline::scrolled_content_rect(dom, id).unwrap_or(outer);
        let Some((owner, atomic)) = hit_fragment(dom, id, inner, x, y) else {
            return false;
        };
        // An atomic inline is a box (CSS 2.1 §9.2.2): it is hit as one —
        // its content searched, its own `visibility` / `pointer-events`
        // applied — under the inline ancestors it sits in.
        if atomic {
            let mark = path.len();
            let hit = hit_in_flow_element(dom, owner, x, y, content_clip, viewport, path);
            if hit && let Some(parent) = dom.node(owner).parent_node() {
                let chain = inline_ancestors(dom, id, parent.id());
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
            target = match dom.node(target).parent_node() {
                Some(p) => p.id(),
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
        return true;
    }
    descend_children_reverse(dom, id, x, y, content_clip, viewport, path)
}

/// Recurse into the in-flow element children in reverse document
/// order; the first hit wins (matches paint order). Positioned children
/// are skipped — they are tried from their stacking context's layers —
/// and a child that establishes a stacking context without being
/// positioned (`opacity < 1`) is searched as one atomic unit.
fn descend_children_reverse(
    dom: &Dom<TuiExt>,
    id: NodeId,
    x: u16,
    y: u16,
    clip: Rect,
    viewport: Rect,
    path: &mut Vec<NodeId>,
) -> bool {
    // Reverse paint order: a flex container's items in order-modified
    // document order (CSS Flexbox §5.4).
    for child in crate::render::box_tree::paint_order_children(dom, id).rev() {
        let node = dom.node(child);
        let mark = path.len();
        let hit = match node.node_type() {
            NodeType::Fragment => descend_children_reverse(dom, child, x, y, clip, viewport, path),
            // A box-less element (CSS Display 3 §2.5) is never hit
            // itself; its children are where they are, with it on their
            // ancestor path.
            NodeType::Element if crate::render::box_tree::is_contents(dom, child) => {
                let mark = path.len();
                let hit = descend_children_reverse(dom, child, x, y, clip, viewport, path);
                if hit {
                    path.insert(mark, child);
                }
                hit
            }
            NodeType::Element => hit_in_flow_element(dom, child, x, y, clip, viewport, path),
            _ => false,
        };
        if hit {
            // A flex item reordered by `order` comes through its
            // box-less ancestors (`paint_order_children` unwraps them):
            // they stay on its path, outermost first.
            insert_box_less_ancestors(dom, id, child, mark, path);
            return true;
        }
    }
    false
}

/// Hit-test the in-flow element `id` at its turn in its parent's
/// content: a layered box (positioned, or a flex / grid item with a
/// `z-index`) is skipped — it is tried from its stacking context's
/// layers — and one that establishes a stacking context
/// without being positioned (`opacity < 1`) is searched as one atomic
/// unit.
fn hit_in_flow_element(
    dom: &Dom<TuiExt>,
    id: NodeId,
    x: u16,
    y: u16,
    clip: Rect,
    viewport: Rect,
    path: &mut Vec<NodeId>,
) -> bool {
    let parent = dom.node(id).parent_node().map_or(id, |p| p.id());
    match dom.node(id).ext().and_then(|e| e.computed.as_ref()) {
        Some(c) if is_layered(dom, parent, c) => false,
        Some(c) if creates_stacking_context(dom, parent, c) => {
            hit_stacking_context(dom, id, x, y, clip, viewport, path)
        }
        _ => descend_plain(dom, id, x, y, clip, viewport, path),
    }
}

/// Insert at `mark` the element ancestors of `child` below `id` — the
/// `display: contents` elements a box-tree walk unwrapped — outermost
/// first. None for a direct child.
fn insert_box_less_ancestors(
    dom: &Dom<TuiExt>,
    id: NodeId,
    child: NodeId,
    mark: usize,
    path: &mut Vec<NodeId>,
) {
    let mut cur = dom.node(child).parent_node();
    while let Some(p) = cur
        && p.id() != id
    {
        if p.node_type() == NodeType::Element {
            path.insert(mark, p.id());
        }
        cur = p.parent_node();
    }
}

/// Look up the inline fragment under `(x, y)` inside an IFC block's
/// content area. Returns the fragment's owner element (the direct
/// element parent of the underlying text — typically `<code>`, `<b>`,
/// or the IFC block itself when the text is a direct child; an atomic
/// inline for its fragment) and whether it is an atom.
fn hit_fragment(
    dom: &Dom<TuiExt>,
    ifc_block: NodeId,
    content: LayoutRect,
    x: u16,
    y: u16,
) -> Option<(NodeId, bool)> {
    let ext = dom.node(ifc_block).ext()?;
    let layout = ext.inline_layout.as_ref()?;

    // The line box spanning the row (CSS 2.1 §10.8: a line is as tall
    // as its tallest atom).
    let row = u16::try_from(y as i32 - content.y).ok()?;
    let line = &layout.lines[layout.line_at_row(row)?];

    // Local x within content.
    let x_local_i = x as i32 - content.x;
    if x_local_i < 0 {
        return None;
    }
    let x_local = x_local_i as u16;

    for fragment in &line.fragments {
        if x_local >= fragment.x
            && x_local < fragment.x + fragment.width
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
        .find(|g| x_local >= g.x && x_local < g.x + g.width)
        .filter(|g| is_descendant(dom, g.host, ifc_block))
        .filter(|g| {
            let node = dom.node(g.host);
            let pseudo = match g.slot {
                crate::ext::PseudoSlot::Before => node.computed_before(),
                crate::ext::PseudoSlot::After => node.computed_after(),
            };
            pseudo.is_none_or(|c| c.pointer_events != crate::layout::PointerEvents::None)
                && crate::render::visibility::shows(dom, g.host, g.slot.into())
        })
        .map(|g| (g.host, false))
}

/// `id` is a strict descendant of `ancestor`.
fn is_descendant(dom: &Dom<TuiExt>, id: NodeId, ancestor: NodeId) -> bool {
    let mut cur = dom.node(id).parent_node().map(|p| p.id());
    while let Some(n) = cur {
        if n == ancestor {
            return true;
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    false
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
        match dom.node(cur).parent_node() {
            Some(parent) => cur = parent.id(),
            None => break, // defensive — should never trigger in a well-formed tree
        }
    }
    chain.reverse();
    chain
}

#[inline]
fn rect_contains(r: LayoutRect, x: u16, y: u16) -> bool {
    let x = x as i32;
    let y = y as i32;
    x >= r.x && x < r.x + r.width as i32 && y >= r.y && y < r.y + r.height as i32
}
