//! Moving what a scroll moves: a scroll container's content, by the
//! change of its scroll offset — every box it is the containing block
//! chain of, since layout is translation-invariant (`tree::shift_subtree`
//! has the model). An absolutely or fixed positioned box whose containing
//! block is outside the moving box does not move with it (CSS Overflow 3
//! §2.2: a scroll container scrolls the boxes it contains), but its static
//! position — its place in the moving flow — does: such a box is handed
//! back to be placed again (`stays`).

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{Display, LayoutRect, Position};
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;
use crate::render::layout_pass::positioning::{
    computed_position, containing_ancestor, fixed_containing_ancestor,
};

/// Move the content of the scroll container `scroller` by `(dx, dy)`:
/// its anonymous boxes, its in-flow and floated pseudo-elements, its
/// children's subtrees and the positioned boxes it contains — not its own
/// box, nor its own lines (kept unscrolled, `inline::scrolled_content_rect`).
pub(super) fn scroll_content(
    dom: &mut Dom<TuiExt>,
    scroller: NodeId,
    dx: i32,
    dy: i32,
    stays: &mut Vec<BoxItem>,
) {
    move_content(dom, scroller, (dx, dy), scroller, false, stays);
}

/// Move the positioned box `id`, placed again `(dx, dy)` from where it was,
/// with its content — its static position stays, as it is its place in its
/// parent's flow.
pub(in crate::render::layout_pass) fn move_box(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    dx: i32,
    dy: i32,
    stays: &mut Vec<BoxItem>,
) {
    move_rects(dom, id, (dx, dy), false);
    move_content(dom, id, (dx, dy), id, true, stays);
}

/// Whether the element `id` is a positioned box whose containing block is
/// not `root` or inside it: it stays when `root`'s content moves.
fn escapes(dom: &Dom<TuiExt>, id: NodeId, root: NodeId) -> bool {
    let position = computed_position(dom, id);
    if !matches!(position, Position::Absolute | Position::Fixed) {
        return false;
    }
    if dom.is_in_top_layer(id) {
        return true;
    }
    let parent = crate::render::box_tree::box_parent(dom, id);
    escapes_from(dom, parent, position == Position::Fixed, root)
}

/// Whether the containing block of a positioned box whose ancestors are
/// `from` and up — `fixed` or absolute — is outside `root`.
fn escapes_from(dom: &Dom<TuiExt>, from: Option<NodeId>, fixed: bool, root: NodeId) -> bool {
    let cb = if fixed {
        fixed_containing_ancestor(dom, from)
    } else {
        containing_ancestor(dom, from)
    };
    cb.is_none_or(|cb| !dom.node(root).contains(cb))
}

/// Move the subtree at `id` inside `root`'s moving content. Layout gives a
/// box with no box of its own a rect at its box parent's content origin
/// (`display: contents`, a non-atomic inline box of an inline formatting
/// context: `layout_pass::dispatch`), which moves when that origin does
/// (`origin_moves`: the parent is not `root`, whose own box stays), writes
/// none for a nested inline box, and none inside a `display: none` subtree
/// — those stay as they are.
fn walk(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    d: (i32, i32),
    root: NodeId,
    stays: &mut Vec<BoxItem>,
    origin_moves: bool,
) {
    if dom.node(id).node_type() != NodeType::Element {
        walk_children(dom, id, d, root, stays, origin_moves);
        return;
    }
    let Some(c) = dom.node(id).computed_rc() else {
        return;
    };
    if c.display == Display::None {
        return;
    }
    if escapes(dom, id, root) {
        // Its place in the moving flow moves; it is placed again.
        if let Some(p) = dom
            .node_mut(id)
            .ext_mut()
            .and_then(|e| e.static_position.as_mut())
        {
            p.x += d.0;
            p.y += d.1;
        }
        stays.push(BoxItem::Node(id));
        return;
    }
    if crate::render::box_tree::is_contents(dom, id) {
        if origin_moves {
            move_rects(dom, id, d, false);
        }
        walk_children(dom, id, d, root, stays, origin_moves);
        return;
    }
    let inline_box = c.display == Display::Inline
        && !c.is_atomic_inline()
        && crate::render::layout_pass::is_in_flow(dom, id);
    if inline_box {
        let in_lines = crate::render::box_tree::box_parent(dom, id)
            .and_then(|p| dom.node(p).ext())
            .is_some_and(|e| e.inline_layout.is_some());
        if in_lines && origin_moves {
            move_rects(dom, id, d, false);
        }
        walk_children(dom, id, d, root, stays, false);
        return;
    }
    move_rects(dom, id, d, true);
    move_content(dom, id, d, root, true, stays);
}

fn walk_children(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    d: (i32, i32),
    root: NodeId,
    stays: &mut Vec<BoxItem>,
    origin_moves: bool,
) {
    let children: Vec<NodeId> = crate::render::box_tree::children(dom, id).collect();
    for c in children {
        walk(dom, c, d, root, stays, origin_moves);
    }
}

/// Move `id`'s own rects (and its static position, `with_static`).
fn move_rects(dom: &mut Dom<TuiExt>, id: NodeId, (dx, dy): (i32, i32), with_static: bool) {
    let mut node = dom.node_mut(id);
    let Some(ext) = node.ext_mut() else {
        return;
    };
    ext.layout = moved(ext.layout, dx, dy);
    ext.content_layout = moved(ext.content_layout, dx, dy);
    if with_static && let Some(p) = ext.static_position.as_mut() {
        p.x += dx;
        p.y += dy;
    }
}

/// Move what `id` lays out inside its box: its anonymous and floated
/// boxes, the positioned pseudo-elements whose containing block is inside
/// `root`, and its children's subtrees — `own_moves` when its own box moves
/// too (not a scroll container's, whose content alone scrolls).
fn move_content(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    d: (i32, i32),
    root: NodeId,
    own_moves: bool,
    stays: &mut Vec<BoxItem>,
) {
    // Which positioned pseudo-elements stay: their containing block is
    // found from their host up.
    let staying: Vec<bool> = dom.node(id).ext().map_or_else(Vec::new, |ext| {
        ext.positioned_pseudo_boxes()
            .iter()
            .map(|a| {
                a.generated.is_some_and(|g| {
                    let fixed = dom
                        .node(id)
                        .computed_pseudo(g.slot)
                        .is_some_and(|c| c.position == Position::Fixed);
                    escapes_from(dom, Some(id), fixed, root)
                })
            })
            .collect()
    });
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        for anon in ext.anonymous_blocks.iter_mut() {
            move_anon(anon, d);
        }
        for anon in ext.floated_pseudos.as_deref_mut().into_iter().flatten() {
            move_anon(anon, d);
        }
        let positioned = ext.positioned_pseudos.as_deref_mut().into_iter().flatten();
        for (anon, stay) in positioned.zip(staying) {
            match (stay, anon.generated) {
                (true, Some(g)) => stays.push(BoxItem::Generated(g.host, g.slot)),
                _ => move_anon(anon, d),
            }
        }
    }
    // Skipped contents are not laid out (CSS Containment 2 §4).
    if crate::style::content_visibility::skips_contents(dom, id) {
        return;
    }
    walk_children(dom, id, d, root, stays, own_moves);
}

/// Move a box laid out in a flow — its lines sit relative to `rect`.
pub(in crate::render::layout_pass) fn move_anon(anon: &mut AnonymousIfc, (dx, dy): (i32, i32)) {
    anon.rect = moved(anon.rect, dx, dy);
    if let Some(g) = anon.generated.as_mut() {
        g.border_box = moved(g.border_box, dx, dy);
    }
}

fn moved(r: LayoutRect, dx: i32, dy: i32) -> LayoutRect {
    LayoutRect::new(r.x + dx, r.y + dy, r.width, r.height)
}
