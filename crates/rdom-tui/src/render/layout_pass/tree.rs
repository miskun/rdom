//! Tree helpers of the layout pass: the element children a container
//! lays out (fragments and `display: contents` elements unwrapped), the
//! in-flow predicate, the geometry reset of `display: none`
//! subtrees and box-less elements, and moving a laid-out subtree.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;

/// Direct *element* children of `id`, document order. Text/Comment
/// are skipped (they have no TuiExt and flow inline via intrinsic
/// measurement). Fragment children and `display: contents` children
/// (CSS Display 3 §2.5: no box of their own) are unwrapped — their
/// element children are returned as if they were direct children of
/// `id`.
pub(crate) fn element_children_of(dom: &Dom<TuiExt>, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    collect_element_children(dom, id, &mut out);
    out
}

/// True iff `id` participates in normal flow. Non-elements (text, comments,
/// fragments) always do; an element does when it's neither `display: none` nor
/// out-of-flow positioned (`absolute` / `fixed`). The single source of truth
/// for the "skip out-of-flow children" filter shared by block + flex layout and
/// the scroll-content walk (`DRY-1`), by the margin-collapse predicates, intrinsic sizing, paint and hit-test.
pub(crate) fn is_in_flow(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    if node.node_type() != NodeType::Element {
        return true; // text, comments, fragments
    }
    let Some(c) = node.ext().and_then(|e| e.computed.as_ref()) else {
        return true;
    };
    use crate::layout::{Display, Position};
    // A box-less element has no box to take out of flow (CSS Display 3
    // §2.5): its `position` applies to nothing.
    c.display == Display::Contents
        || (c.display != Display::None
            && !matches!(c.position, Position::Absolute | Position::Fixed)
            && !is_collapsed_table_row(dom, id))
}

/// A `visibility: collapse` table row (CSS 2.1 §17.5.5): removed from
/// the table's flow — the rows close up — while its cells still size
/// the columns (`runtime::builtins::table` sizes them from every row).
/// rdom's table rows are the `<tr>` children of a `<table>` or its row
/// groups (DIVERGENCES: tables are flex rows).
pub(crate) fn is_collapsed_table_row(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    // The visibility first: it is a field read, and almost never
    // `collapse`; the tag compare runs for every in-flow element.
    node.ext()
        .and_then(|e| e.computed.as_ref())
        .is_some_and(|c| c.visibility == crate::layout::Visibility::Collapse)
        && node.tag_name() == Some("tr")
        && node
            .parent_node()
            .is_some_and(|p| matches!(p.tag_name(), Some("table" | "thead" | "tbody" | "tfoot")))
}

/// Zero the layout geometry of every `display:none` child subtree of `id`.
/// In-flow layout filters `display:none` children out, so they'd otherwise
/// retain the rect from when they were last visible (LAYOUT-DISPLAY-NONE-STALE-
/// RECT). A `display:none` box generates no box, so its rect — and every
/// descendant's, since the subtree isn't laid out — must read zero.
///
/// A `display: contents` child has no box either (CSS Display 3 §2.5):
/// its own rects read zero, at `origin` (the container's content box),
/// while its children are laid out as the container's.
pub(crate) fn collapse_hidden_children(dom: &mut Dom<TuiExt>, id: NodeId, origin: LayoutRect) {
    zero_contents_children(dom, id, origin);
    for child in element_children_of(dom, id) {
        let hidden = dom
            .node(child)
            .ext()
            .and_then(|e| e.computed.as_ref())
            .map(|c| c.display == crate::layout::Display::None)
            .unwrap_or(false)
            || is_collapsed_table_row(dom, child);
        if hidden {
            collapse_subtree_geometry(dom, child);
        }
    }
}

/// Recursively reset `layout` / `content_layout` to the zero rect for `id` and
/// every element descendant. Used to collapse a `display:none` subtree.
fn collapse_subtree_geometry(dom: &mut Dom<TuiExt>, id: NodeId) {
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        if ext.layout == LayoutRect::default() && ext.content_layout == LayoutRect::default() {
            // Already collapsed — and so is everything below it (we always zero
            // top-down), so stop early. Keeps steady-state hidden subtrees O(1).
            return;
        }
        clear_box_state(ext, LayoutRect::default());
    }
    for child in element_children_of(dom, id) {
        collapse_subtree_geometry(dom, child);
    }
}

/// Give every `display: contents` child of `id` (through nested ones) a
/// zero rect at `origin`.
fn zero_contents_children(dom: &mut Dom<TuiExt>, id: NodeId, origin: LayoutRect) {
    let children: Vec<NodeId> = dom.node(id).child_nodes().map(|c| c.id()).collect();
    for child in children {
        if crate::render::box_tree::is_contents(dom, child) {
            if let Some(ext) = dom.node_mut(child).ext_mut() {
                clear_box_state(ext, LayoutRect::new(origin.x, origin.y, 0, 0));
            }
            zero_contents_children(dom, child, origin);
        }
    }
}

/// Reset everything `ext`'s element derived from having a box — its
/// rects (to `rect`, a zero-size one), margin-collapse memo, line boxes,
/// anonymous block boxes, scroll extent and offsets — when it has none
/// (`display: none` / `contents`, CSS Display 3 §2.5). Layout writes
/// these only on a node it lays out, so a box-less node would otherwise
/// keep its box days' values, and caret, hit-test and focus code reading
/// them would act on a box that no longer exists.
fn clear_box_state(ext: &mut TuiExt, rect: LayoutRect) {
    ext.layout = rect;
    ext.content_layout = rect;
    ext.layout_dirty = false;
    ext.margin_chain = None;
    ext.inline_layout = None;
    ext.anonymous_blocks.clear();
    ext.scroll_content_width = 0;
    ext.scroll_content_height = 0;
    ext.scroll_x = 0;
    ext.scroll_y = 0;
    ext.scroll_state = None;
    ext.static_position = None;
}

/// Move `id`'s laid-out subtree by `(dx, dy)`: every element's rects,
/// its anonymous block boxes, its positioned pseudo-elements and its
/// recorded static position — what a layout at the moved origin would
/// have written, since layout is translation-invariant. Used where a
/// box moves after its subtree was laid out (`position: sticky`) and
/// where content is aligned after it was measured (`align-content`).
/// Every other position layout keeps is relative to one of these — line
/// boxes and fragments to their content box, a grid's lines to its
/// `content_layout` (C7G-LINES-SHIFT) — so moving these moves it all.
pub(super) fn shift_subtree(dom: &mut Dom<TuiExt>, id: NodeId, dx: i32, dy: i32) {
    let shift = |r: LayoutRect| LayoutRect::new(r.x + dx, r.y + dy, r.width, r.height);
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.layout = shift(ext.layout);
        ext.content_layout = shift(ext.content_layout);
        for anon in &mut ext.anonymous_blocks {
            anon.rect = shift(anon.rect);
            if let Some(g) = anon.generated.as_mut() {
                g.border_box = shift(g.border_box);
            }
        }
        for pseudo in [&mut ext.before_layout, &mut ext.after_layout]
            .into_iter()
            .flatten()
        {
            pseudo.rect = shift(pseudo.rect);
        }
        if let Some(p) = ext.static_position.as_mut() {
            p.x += dx;
            p.y += dy;
        }
    }
    shift_children(dom, id, dx, dy);
}

/// Move the laid-out content of `id` — its anonymous block boxes and
/// its children's subtrees, not its own box — by `dy` rows.
pub(super) fn shift_content(dom: &mut Dom<TuiExt>, id: NodeId, dy: i32) {
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        for anon in &mut ext.anonymous_blocks {
            anon.rect.y += dy;
            if let Some(g) = anon.generated.as_mut() {
                g.border_box.y += dy;
            }
        }
    }
    shift_children(dom, id, 0, dy);
}

/// Move the lines of `id`'s inline formatting context down by `dy`
/// rows (`align-content`, CSS Box Alignment 3 §5.1), with
/// [`shift_content`] moving its atoms' boxes alongside. A line's rows
/// count from the content box's top (`LineBox::top` is unsigned), so
/// the caller never shifts them up (DIVERGENCES §4).
pub(super) fn shift_lines(dom: &mut Dom<TuiExt>, id: NodeId, dy: i32) {
    let Ok(dy) = u16::try_from(dy) else {
        debug_assert!(false, "lines shift down only, by {dy}");
        return;
    };
    if let Some(layout) = dom
        .node_mut(id)
        .ext_mut()
        .and_then(|e| e.inline_layout.as_mut())
    {
        for line in &mut layout.lines {
            line.top = line.top.saturating_add(dy);
        }
    }
}

fn shift_children(dom: &mut Dom<TuiExt>, id: NodeId, dx: i32, dy: i32) {
    let mut child = dom.node(id).first_child().map(|c| c.id());
    while let Some(c) = child {
        shift_subtree(dom, c, dx, dy);
        child = dom.node(c).next_sibling().map(|n| n.id());
    }
}

fn collect_element_children(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    for child in dom.node(id).child_nodes() {
        match child.node_type() {
            NodeType::Element if crate::render::box_tree::is_contents(dom, child.id()) => {
                collect_element_children(dom, child.id(), out)
            }
            NodeType::Element => out.push(child.id()),
            NodeType::Fragment => collect_element_children(dom, child.id(), out),
            // Text, comments, and any later node kind (`NodeType` is
            // `#[non_exhaustive]`) are not element children.
            _ => {}
        }
    }
}
