//! Tree helpers of the layout pass: the element children a container
//! lays out (fragments and `display: contents` elements unwrapped), the
//! in-flow predicate, the geometry reset of `display: none`
//! subtrees and box-less elements, and moving a laid-out subtree.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;

/// Direct *element* children of `id` in the box tree
/// (`box_tree::children`: a `<details>`'s `::details-content` box among
/// them), document order. Text/Comment
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
/// fragments) always do; an element does when it's neither `display: none`,
/// out-of-flow positioned (`absolute` / `fixed`) nor a float (CSS 2.1 §9.3:
/// "an element is called out of flow if it is floated, absolutely
/// positioned, or is the root element"; `float::float_side`). The single source of truth
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
            && (c.float == crate::layout::Float::None
                || super::float::float_side(dom, id).is_none()))
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
            .unwrap_or(false);
        if hidden {
            collapse_subtree_geometry(dom, child);
        }
    }
}

/// Recursively reset `layout` / `content_layout` to the zero rect for `id` and
/// every element descendant. Used to collapse a `display:none` subtree.
pub(super) fn collapse_subtree_geometry(dom: &mut Dom<TuiExt>, id: NodeId) {
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
    let children: Vec<NodeId> = crate::render::box_tree::children(dom, id).collect();
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
/// anonymous block boxes, scroll extent and offsets, grid lines — when it has none
/// (`display: none` / `contents`, CSS Display 3 §2.5). Layout writes
/// these only on a node it lays out, so a box-less node would otherwise
/// keep its box days' values, and caret, hit-test and focus code reading
/// them would act on a box that no longer exists.
pub(super) fn clear_box_state(ext: &mut TuiExt, rect: LayoutRect) {
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
    ext.grid_lines = None;
    ext.floated_pseudos = None;
    ext.positioned_pseudos = None;
}

/// Move `id`'s laid-out subtree by `(dx, dy)`: every element's rects,
/// its anonymous block boxes, its positioned pseudo-elements and its
/// recorded static position — what a layout at the moved origin would
/// have written, since layout is translation-invariant. Used where a
/// box moves after its subtree and its positioned descendants were laid
/// out (`position: sticky`): a `fixed` descendant's subtree stays where
/// it is — its containing block is the viewport (CSS Position 3 §2.1),
/// which the move does not touch. [`shift_content`] moves a block's
/// content after it was measured (`align-content`), `fixed` boxes too
/// (phase 2 places them again, from the static positions moved here).
/// Every other position layout keeps is relative to one of these — line
/// boxes and fragments to their content box, a grid's lines to its
/// `content_layout` (C7G-LINES-SHIFT) — so moving these moves it all.
pub(super) fn shift_subtree(dom: &mut Dom<TuiExt>, id: NodeId, dx: i32, dy: i32) {
    shift(dom, id, dx, dy, Keep::Fixed);
}

/// Move `id`'s box and its laid-out subtree by `(dx, dy)`, `fixed`
/// descendants included — phase 2 places them again from the static
/// positions moved here: the relative offset (CSS 2.1 §9.4.3) applied
/// after `id` was laid out in flow.
pub(super) fn shift_box(dom: &mut Dom<TuiExt>, id: NodeId, dx: i32, dy: i32) {
    shift(dom, id, dx, dy, Keep::None);
}

/// What a subtree shift leaves in place.
#[derive(Clone, Copy, PartialEq)]
enum Keep {
    None,
    Fixed,
}

fn shift(dom: &mut Dom<TuiExt>, id: NodeId, dx: i32, dy: i32, keep: Keep) {
    if keep == Keep::Fixed
        && dom
            .node(id)
            .ext()
            .and_then(|e| e.computed.as_ref())
            .is_some_and(|c| c.position == crate::layout::Position::Fixed)
    {
        return;
    }
    let shift = |r: LayoutRect| LayoutRect::new(r.x + dx, r.y + dy, r.width, r.height);
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.layout = shift(ext.layout);
        ext.content_layout = shift(ext.content_layout);
        // Its positioned pseudo-elements move with it, as its positioned
        // descendants do: a `fixed` one stays where `keep` says.
        let fixed: Vec<bool> = ext
            .positioned_pseudo_boxes()
            .iter()
            .map(|a| {
                keep == Keep::Fixed
                    && a.generated
                        .and_then(|g| ext.computed_pseudo(g.slot))
                        .is_some_and(|c| c.position == crate::layout::Position::Fixed)
            })
            .collect();
        let floated = ext.floated_pseudos.as_deref_mut().into_iter().flatten();
        let positioned = ext
            .positioned_pseudos
            .as_deref_mut()
            .into_iter()
            .flatten()
            .zip(fixed)
            .filter_map(|(a, fixed)| (!fixed).then_some(a));
        for anon in ext
            .anonymous_blocks
            .iter_mut()
            .chain(floated)
            .chain(positioned)
        {
            anon.rect = shift(anon.rect);
            if let Some(g) = anon.generated.as_mut() {
                g.border_box = shift(g.border_box);
            }
        }
        if let Some(p) = ext.static_position.as_mut() {
            p.x += dx;
            p.y += dy;
        }
    }
    shift_children(dom, id, dx, dy, keep);
}

/// Move the laid-out content of `id` — its anonymous block boxes and
/// its children's subtrees, not its own box — by `dy` rows.
pub(super) fn shift_content(dom: &mut Dom<TuiExt>, id: NodeId, dy: i32) {
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        let floated = ext.floated_pseudos.as_deref_mut().into_iter().flatten();
        for anon in ext.anonymous_blocks.iter_mut().chain(floated) {
            anon.rect.y += dy;
            if let Some(g) = anon.generated.as_mut() {
                g.border_box.y += dy;
            }
        }
    }
    shift_children(dom, id, 0, dy, Keep::None);
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

fn shift_children(dom: &mut Dom<TuiExt>, id: NodeId, dx: i32, dy: i32, keep: Keep) {
    let children: Vec<NodeId> = crate::render::box_tree::children(dom, id).collect();
    for c in children {
        shift(dom, c, dx, dy, keep);
    }
}

fn collect_element_children(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    for child in crate::render::box_tree::children(dom, id) {
        match dom.node(child).node_type() {
            NodeType::Element if crate::render::box_tree::is_contents(dom, child) => {
                collect_element_children(dom, child, out)
            }
            NodeType::Element => out.push(child),
            NodeType::Fragment => collect_element_children(dom, child, out),
            // Text, comments, and any later node kind (`NodeType` is
            // `#[non_exhaustive]`) are not element children.
            _ => {}
        }
    }
}
