//! The stacking-context walk — `(x, y)` → full ancestor path.
//!
//! This is the reverse-paint traversal behind
//! [`HitTestExt::hit_test_path`](super::HitTestExt::hit_test_path):
//! stacking contexts and their layers (`hit_stacking_context`,
//! `hit_layers`), plain boxes and their in-flow content
//! (`descend_plain`, `hit_content`, `descend_children_reverse`), and
//! `pointer-events` transparency and inertness for boxes. The inline
//! formatting context's half — which fragment's owner a point on a line
//! is — is `inline_hit`.
//! Nothing here knows about text positions; that is `fragment.rs`.
//!
//! The walk is over the box tree: a `<details>`'s `::details-content` box
//! is on the path it builds, between the element and its content
//! (`box_tree::slot`); [`HitTestExt::hit_test_path`](super::HitTestExt::hit_test_path)
//! leaves it out.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::node::TuiNodeExt;
use crate::render::Rect;
use crate::render::box_tree::slot::parent;
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
/// `z-index: auto | 0` layer in reverse tree order, the floats in reverse
/// tree order, the root's in-flow content, child contexts with negative `z-index`, and finally the
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
    let layers = collect_layers(dom, root, (content_clip, clip), viewport);
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
        || hit_layers(dom, root, &layers.floats, x, y, viewport, path)
    {
        root_in_path(path);
        return true;
    }
    let Some((_, outer)) = root_box else {
        // The document root: in-flow content — its children's boxes,
        // then the lines of the initial containing block's anonymous
        // boxes — then the negative layer.
        return descend_children_reverse(dom, root, x, y, (content_clip, clip), viewport, path)
            || super::inline_hit::hit_anonymous_lines(
                dom,
                root,
                x,
                y,
                (content_clip, viewport),
                path,
            )
            || hit_layers(dom, root, &layers.negative, x, y, viewport, path);
    };
    let contains = rect_contains(outer, x, y);
    // A `<select>` picker (in the top layer) is its option list, which
    // overflows the select's own row: its content is hit outside its box.
    let picker = dom.top_layer_kind(root) == Some(rdom_core::TopLayerKind::Picker);
    if (contains || picker) && reach(dom, root, content_clip, clip).contains(x, y) {
        if !transparent {
            path.push(root);
        }
        if hit_content(dom, root, x, y, (content_clip, clip), viewport, path) {
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
        // A layered box inside an `inert` subtree is inert with it (HTML
        // §6.3.1): its context's walk reaches it without passing the
        // ancestor that carries the attribute. A `::details-content` box
        // is outside the DOM tree, which `Dom::is_inert` climbs: it is as
        // inert as its `<details>`.
        let subject = crate::render::box_tree::slot::host_of(dom, e.id).unwrap_or(e.id);
        if dom.is_inert(subject) {
            continue;
        }
        let mark = path.len();
        let hit = if let Some(generated) = e.generated {
            hit_generated(dom, e.id, generated, x, y, e.clip, path)
        } else if e.context {
            hit_stacking_context(dom, e.id, x, y, e.clip, viewport, path)
        } else {
            descend_plain(dom, e.id, x, y, e.clip, viewport, path)
        };
        if hit {
            let mut chain = Vec::new();
            let mut cur = parent(dom, e.id);
            while let Some(id) = cur {
                if id == root {
                    break;
                }
                if dom.node(id).node_type() == NodeType::Element {
                    chain.push(id);
                }
                cur = parent(dom, id);
            }
            chain.reverse();
            path.splice(mark..mark, chain);
            return true;
        }
    }
    false
}

/// Hit-test a generated box a layer holds in place of `owner`: the `k`-th
/// floated `::before` / `::after` of `owner`'s formatting context run, or
/// `owner`'s `k`-th positioned one. A pseudo-element is part of its
/// host's box, so a hit inside its border box (and `clip`) targets the
/// host — `owner` on the path above it — unless the pseudo-element is
/// `pointer-events: none` or not drawn.
fn hit_generated(
    dom: &Dom<TuiExt>,
    owner: NodeId,
    generated: crate::render::stacking::Generated,
    x: u16,
    y: u16,
    clip: Rect,
    path: &mut Vec<NodeId>,
) -> bool {
    use crate::render::stacking::Generated;
    let Some(ext) = dom.node(owner).ext() else {
        return false;
    };
    let anon = match generated {
        Generated::Floated(k) => ext.floated_pseudos().get(k),
        Generated::Positioned(k) => ext.positioned_pseudo_boxes().get(k),
    };
    let Some(g) = anon.and_then(|a| a.generated) else {
        return false;
    };
    if g.host != owner && dom.is_inert(g.host) {
        return false;
    }
    let pseudo = dom.node(g.host).computed_pseudo(g.slot);
    let targets = pseudo.is_some_and(|c| c.pointer_events != crate::layout::PointerEvents::None)
        && crate::render::visibility::shows(dom, g.host, g.slot.into());
    if !targets || !clip.contains(x, y) || !rect_contains(g.border_box, x, y) {
        return false;
    }
    path.push(owner);
    if g.host != owner {
        path.push(g.host);
    }
    true
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
    // A column or column group has a rect for its background (CSS 2.1
    // §17.5.1) but is no target: a point over it is over the cells or
    // the table.
    if crate::render::layout_pass::is_column_box(dom, id) {
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
        return reach(dom, id, content_clip, clip).contains(x, y)
            && hit_content(dom, id, x, y, (content_clip, clip), viewport, path);
    }
    // `visibility: hidden` (CSS Display 3 §4): the box draws nothing and
    // is no target, but a `visible` descendant is — with the hidden
    // element on its ancestor path, as the DOM has it.
    if !crate::render::visibility::shows(dom, id, crate::ext::StyleSlot::Host) {
        let mark = path.len();
        let hit = reach(dom, id, content_clip, clip).contains(x, y)
            && hit_content(dom, id, x, y, (content_clip, clip), viewport, path);
        if hit {
            path.insert(mark, id);
        }
        return hit;
    }

    path.push(id);
    // Overflow clipping (CSS Overflow 3 §3 = padding-box; the same rect
    // paint clips to): outside the scrollport the hit stays on THIS
    // element's padding / border — no descent.
    if reach(dom, id, content_clip, clip).contains(x, y) {
        hit_content(dom, id, x, y, (content_clip, clip), viewport, path);
    }
    true
}

/// Where a point can hit `id`'s content: its content clip — or, for a
/// table with captions, the clip it is hit in, as its captions are
/// (`stacking::child_clip`; its other children are narrowed again each).
fn reach(dom: &Dom<TuiExt>, id: NodeId, content_clip: Rect, clip: Rect) -> Rect {
    if crate::render::stacking::has_unclipped_children(dom, id) {
        clip
    } else {
        content_clip
    }
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
    (content_clip, outer): (Rect, Rect),
    viewport: Rect,
    path: &mut Vec<NodeId>,
) -> bool {
    if has_inline_layout(dom, id) {
        return content_clip.contains(x, y)
            && super::inline_hit::hit_inline_content(dom, id, x, y, content_clip, viewport, path);
    }
    descend_children_reverse(dom, id, x, y, (content_clip, outer), viewport, path)
        || super::inline_hit::hit_anonymous_lines(dom, id, x, y, (content_clip, viewport), path)
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
    (clip, outer): (Rect, Rect),
    viewport: Rect,
    path: &mut Vec<NodeId>,
) -> bool {
    // Reverse paint order: a flex container's items in order-modified
    // document order (CSS Flexbox §5.4).
    for child in crate::render::box_tree::paint_order_children(dom, id).rev() {
        let node = dom.node(child);
        // An `inert` subtree is out of hit-testing whole (HTML §6.3.1).
        if has_inert_attribute(dom, child) {
            continue;
        }
        let mark = path.len();
        let hit = match node.node_type() {
            NodeType::Fragment => {
                descend_children_reverse(dom, child, x, y, (clip, outer), viewport, path)
            }
            // A box-less element (CSS Display 3 §2.5) is never hit
            // itself; its children are where they are, with it on their
            // ancestor path.
            NodeType::Element if crate::render::box_tree::is_contents(dom, child) => {
                let mark = path.len();
                let hit = descend_children_reverse(dom, child, x, y, (clip, outer), viewport, path);
                if hit {
                    path.insert(mark, child);
                }
                hit
            }
            // A list item's outside marker hangs outside its box, painted
            // with its lines: a point on it is the item's (CSS Lists 3
            // §3.5).
            NodeType::Element => {
                let clip = crate::render::stacking::child_clip(dom, id, child, clip, outer);
                hit_in_flow_element(dom, child, x, y, clip, viewport, path)
                    || (super::pseudo::on_outside_marker(dom, child, x, y, clip) && {
                        path.push(child);
                        true
                    })
            }
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
pub(super) fn hit_in_flow_element(
    dom: &Dom<TuiExt>,
    id: NodeId,
    x: u16,
    y: u16,
    clip: Rect,
    viewport: Rect,
    path: &mut Vec<NodeId>,
) -> bool {
    let parent = parent(dom, id).unwrap_or(id);
    match dom.node(id).ext().and_then(|e| e.computed.as_ref()) {
        Some(c) if is_layered(dom, id, parent, c) => false,
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
    let mut cur = parent(dom, child);
    while let Some(p) = cur
        && p != id
    {
        if dom.node(p).node_type() == NodeType::Element {
            path.insert(mark, p);
        }
        cur = parent(dom, p);
    }
}

/// `id` carries the `inert` attribute (HTML §6.3.1).
pub(super) fn has_inert_attribute(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    dom.node(id).has_attribute("inert")
}

#[inline]
fn rect_contains(r: LayoutRect, x: u16, y: u16) -> bool {
    let x = x as i32;
    let y = y as i32;
    x >= r.x && x < r.x + r.width as i32 && y >= r.y && y < r.y + r.height as i32
}
