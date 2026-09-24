//! `HitTestExt` — point → element lookup, the foundation of mouse
//! routing.
//!
//! `Dom::hit_test(x, y)` returns the deepest element whose painted
//! area contains `(x, y)`. `hit_test_path(x, y)` returns the full
//! ancestor chain (outer → inner), matching the browser's
//! `composedPath()` for a synthetic `MouseEvent` at that point.
//!
//! ## Algorithm
//!
//! The walk mirrors paint in reverse (`render::stacking`, CSS 2.1
//! Appendix E). For a stacking context: try the child contexts with
//! positive `z-index` (highest first), then the `z-index: auto | 0`
//! layer in reverse tree order, then the root's in-flow content
//! (children in reverse document order, or the inline fragment under
//! the point for an inline formatting context), then the negative
//! layer, and finally the root's own box. Every layer entry carries
//! the clip that applies to it (CSS 2.1 §11.1.1), so a positioned box
//! clipped by an ancestor's overflow is not hittable outside it, and
//! an element that clips its children ends the descent when the point
//! sits on its padding or border.
//!
//! The path is the full DOM ancestor chain of the hit, root-most
//! first, whichever layer found it — the ancestors between a context
//! root and a layer entry are inserted when the entry hits.
//!
//! ## `pointer-events`
//!
//! `pointer-events: none` makes an element transparent: it is never
//! the hit target, its subtree is still searched for `auto`
//! descendants, and otherwise the point falls through to what is
//! beneath. Inherited, like the web.

use rdom_core::{Dom, NodeId, NodeType, Position};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::node::TuiNodeExt;
use crate::render::Rect;
use crate::render::inline::{InlineFragment, has_inline_layout};
use crate::render::stacking::{
    LayerEntry, children_clip, collect_layers, creates_stacking_context, is_positioned,
};
use crate::runtime::selection::user_select;
use crate::style::ComputedStyle;

/// Extension trait adding hit-test lookup to `Dom<TuiExt>`.
pub trait HitTestExt {
    /// The deepest element whose painted area contains `(x, y)`.
    /// Uses the last-painted-wins rule: when two siblings overlap,
    /// the later one wins. Returns `None` if no element covers the
    /// point (e.g., empty viewport).
    ///
    /// For IFC blocks the lookup descends into the inline layout so
    /// a point landing on text inside a `<code>` fragment returns
    /// the `<code>` element, not the enclosing `<p>`.
    fn hit_test(&self, x: u16, y: u16) -> Option<NodeId>;

    /// The full ancestor chain from root to the deepest hit, in
    /// document order (root-most first, deepest last). Suitable for
    /// event-dispatch targets or browser-style `composedPath()`
    /// walks. Empty when nothing hit.
    fn hit_test_path(&self, x: u16, y: u16) -> Vec<NodeId>;

    /// Map a screen cell `(x, y)` to a DOM text position — a
    /// `(text_node, byte_offset)` pair suitable for
    /// [`Dom::set_selection`].
    ///
    /// A point in empty space — a gap between blocks, above/below all
    /// content, or a non-IFC container whose text lives in descendants —
    /// **snaps to the nearest text position** by vertical distance
    /// (scoped to the deepest hit element's subtree), matching how
    /// browsers resolve `caretPositionFromPoint` and drag-select past
    /// content. Without this, dragging past the bottom edge collapsed
    /// the selection back to the anchor block.
    ///
    /// Returns `None` when:
    /// - `(x, y)` misses every element AND the document root subtree
    ///   has no inline flow to snap to;
    /// - the innermost hit element or one of its ancestors has
    ///   `user-select: none` (chrome, buttons, etc. — the
    ///   selection algorithm skips these subtrees); the nearest-flow
    ///   fallback likewise skips `user-select: none` candidates.
    ///
    /// The returned `offset` is a byte offset into the text
    /// node's data — matches the `Selection` / `Range` API and
    /// Rust string slicing conventions.
    fn position_at(&self, x: u16, y: u16) -> Option<Position>;

    /// The nearest **selectable** text position to `(x, y)` by vertical
    /// distance — `position_at`'s empty-space resolution, exposed for the
    /// drag-extend fallback. Unlike `position_at` it never returns `None`
    /// for a `user-select: none` hit: it skips those candidates and snaps to
    /// the closest selectable flow instead (so dragging a selection *over* a
    /// `user-select: none` bar extends to the text beyond it rather than
    /// collapsing). Scoped to the deepest hit element's subtree, escalating
    /// up the ancestor chain until a subtree has selectable text. `None` only
    /// when nothing selectable exists to snap to.
    fn nearest_selectable_position(&self, x: u16, y: u16) -> Option<Position>;
}

impl HitTestExt for Dom<TuiExt> {
    fn hit_test(&self, x: u16, y: u16) -> Option<NodeId> {
        self.hit_test_path(x, y).last().copied()
    }

    fn hit_test_path(&self, x: u16, y: u16) -> Vec<NodeId> {
        let mut path = Vec::new();
        // Reverse paint order through the stacking contexts (CSS 2.1
        // Appendix E; `render::stacking`). Hit-testing has no viewport
        // of its own: overflow ancestors are the only clips.
        let unclipped = Rect::new(0, 0, u16::MAX, u16::MAX);
        hit_stacking_context(self, self.root(), x, y, unclipped, unclipped, &mut path);
        path
    }

    fn position_at(&self, x: u16, y: u16) -> Option<Position> {
        // Find the inline-flow container under (x, y) — either a
        // classic IFC block (singular `inline_layout`) or one of
        // a parent's anonymous block boxes (BFC-1 phase 3.3). The
        // hit-test path is walked innermost-first; the deepest
        // matching container wins.
        // `hit_test_path` already handles overflow clipping and
        // reverse-document-order
        // stacking; we just need to find the first IFC ancestor
        // on the path.
        let path = self.hit_test_path(x, y);

        // Walk path *innermost-first* — deepest match wins. A
        // singular IFC block (its own `inline_layout`) is the
        // common case; an anonymous block box (a slot in some
        // ancestor's `anonymous_blocks` Vec, populated by the
        // block-layout pass for inline runs amongst block
        // children) is the BFC-1 phase 3 case.
        if let Some(target) = path
            .iter()
            .rev()
            .find_map(|&id| inline_target_at(self, id, y))
        {
            // user-select gate: any ancestor of the hit with
            // `user-select: none` kills the position.
            if user_select::has_none_ancestor(self, *path.last()?) {
                return None;
            }
            return resolve_in_target(self, target, x, y);
        }

        // Empty space inside a `user-select: none` subtree (e.g. a table row's
        // trailing space past its last cell): no caret. Do NOT snap out to the
        // nearest selectable text elsewhere — that would start a text-selection
        // drag from a non-selectable region and hijack the consumer (the grid).
        // This mirrors the contained-case gate above. The drag-extend path
        // (`nearest_selectable_position`) is unaffected — it deliberately skips
        // *over* user-select:none regions to keep extending an existing drag.
        if let Some(&deepest) = path.last()
            && user_select::has_none_ancestor(self, deepest)
        {
            return None;
        }

        // No inline-flow target contains `y`: the point is in empty space —
        // a gap between blocks, above/below all content, or a non-IFC
        // container with text only in descendants. Browsers snap
        // `caretPositionFromPoint` (and drag-select) to the nearest text
        // position rather than returning nothing.
        self.nearest_selectable_position(x, y)
    }

    fn nearest_selectable_position(&self, x: u16, y: u16) -> Option<Position> {
        // Resolve to the closest inline-flow target by vertical distance.
        // Start scoped to the deepest hit element's subtree (so a Page-
        // scrollport gap resolves within that page, not unrelated chrome) and
        // escalate up the ancestor chain until a subtree has text — e.g. a hit
        // landing in an empty sibling spacer, or on a `user-select: none` bar,
        // climbs to the parent that also holds the prose. The search skips
        // `user-select: none` candidates, so the snap never lands on chrome.
        let path = self.hit_test_path(x, y);
        let mut scope = path.last().copied();
        loop {
            let id = scope.unwrap_or_else(|| self.root());
            if let Some(target) = nearest_inline_target_in_subtree(self, id, y) {
                return resolve_in_target(self, target, x, y);
            }
            if id == self.root() {
                return None;
            }
            scope = self.node(id).parent_node().map(|p| p.id());
        }
    }
}

/// Resolve a screen cell to a [`Position`] within a known inline-flow
/// `target`. Returns the fragment-exact position when a fragment covers
/// `(x, y)`, else clamps to the nearest valid position on the target's
/// lines (drag past end-of-line / past last-line bottom — see
/// [`clamp_to_line_layout`]).
fn resolve_in_target(dom: &Dom<TuiExt>, target: InlineTarget, x: u16, y: u16) -> Option<Position> {
    let (inline_layout, content) = target.layout_and_rect(dom)?;
    match fragment_at_layout(inline_layout, content, x, y) {
        Some(fragment) => {
            let cell_offset_in_frag = (x as i32 - content.x - fragment.x as i32).max(0) as u16;
            let bytes_into_text = cells_to_bytes(&fragment.text, cell_offset_in_frag);
            Some(Position::new(
                fragment.text_node,
                fragment.source_byte_offset + bytes_into_text,
            ))
        }
        None => clamp_to_line_layout(inline_layout, content, x, y),
    }
}

/// The inline-flow target in `root`'s subtree nearest to `y` by vertical
/// distance — the empty-space fallback for [`HitTestExt::position_at`]. A
/// point inside a target's y-range has distance 0; otherwise it's the gap
/// to the nearest edge. Ties keep the first found in document order.
/// `user-select: none` candidates are skipped so the snap never lands on
/// unselectable chrome. Returns `None` when the subtree has no inline flow.
fn nearest_inline_target_in_subtree(
    dom: &Dom<TuiExt>,
    root: NodeId,
    y: u16,
) -> Option<InlineTarget> {
    let y = y as i32;
    let mut best: Option<(i32, InlineTarget)> = None;
    let mut consider = |target: InlineTarget, top: i32, bottom: i32| {
        if user_select::has_none_ancestor(dom, target.node()) {
            return;
        }
        let dist = if y < top {
            top - y
        } else if y >= bottom {
            y - bottom + 1
        } else {
            0
        };
        if best.is_none_or(|(d, _)| dist < d) {
            best = Some((dist, target));
        }
    };

    // Document-order DFS so ties resolve to the earliest target.
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if has_inline_layout(dom, id)
            && let Some(content) = crate::render::inline::scrolled_content_rect(dom, id)
        {
            consider(
                InlineTarget::Ifc(id),
                content.y,
                content.y + content.height as i32,
            );
        }
        if let Some(ext) = dom.node(id).ext() {
            for (i, anon) in ext.anonymous_blocks.iter().enumerate() {
                consider(
                    InlineTarget::Anonymous {
                        container: id,
                        index: i,
                    },
                    anon.rect.y,
                    anon.rect.y + anon.rect.height as i32,
                );
            }
        }
        // Push children reversed so they pop in document order.
        let kids: Vec<NodeId> = dom.node(id).children().map(|c| c.id()).collect();
        stack.extend(kids.into_iter().rev());
    }
    best.map(|(_, t)| t)
}

/// What kind of inline-flow container is under the hit point.
#[derive(Debug, Clone, Copy)]
enum InlineTarget {
    /// Classic IFC — the block element itself owns the
    /// `inline_layout`. Content rect = the block's content_layout.
    Ifc(NodeId),
    /// Anonymous block box — `container` owns the
    /// `anonymous_blocks` Vec; `index` selects the entry. Content
    /// rect = the entry's `.rect` (no further inset).
    Anonymous { container: NodeId, index: usize },
}

impl InlineTarget {
    /// The owning node — the IFC block itself, or the container that
    /// holds the anonymous block box. Used for the `user-select` gate
    /// on the nearest-flow fallback.
    fn node(self) -> NodeId {
        match self {
            InlineTarget::Ifc(id) => id,
            InlineTarget::Anonymous { container, .. } => container,
        }
    }

    /// Resolve to `(layout, content_rect)`. Borrows from the dom.
    fn layout_and_rect(
        self,
        dom: &Dom<TuiExt>,
    ) -> Option<(&crate::render::inline::InlineLayout, LayoutRect)> {
        match self {
            InlineTarget::Ifc(id) => {
                let ext = dom.node(id).ext()?;
                let layout = ext.inline_layout.as_ref()?;
                let content = crate::render::inline::scrolled_content_rect(dom, id)?;
                Some((layout, content))
            }
            InlineTarget::Anonymous { container, index } => {
                let ext = dom.node(container).ext()?;
                let anon = ext.anonymous_blocks.get(index)?;
                Some((&anon.inline_layout, anon.rect))
            }
        }
    }
}

/// Return the inline-flow target rooted at `id` that contains
/// `y`, if any. Picks the singular IFC when present; otherwise
/// checks each anonymous box on the element for a y-range match.
fn inline_target_at(dom: &Dom<TuiExt>, id: NodeId, y: u16) -> Option<InlineTarget> {
    if has_inline_layout(dom, id) {
        return Some(InlineTarget::Ifc(id));
    }
    let ext = dom.node(id).ext()?;
    if ext.anonymous_blocks.is_empty() {
        return None;
    }
    let y_i = y as i32;
    for (i, anon) in ext.anonymous_blocks.iter().enumerate() {
        let top = anon.rect.y;
        let bottom = anon.rect.y + anon.rect.height as i32;
        if y_i >= top && y_i < bottom {
            return Some(InlineTarget::Anonymous {
                container: id,
                index: i,
            });
        }
    }
    None
}

/// Clamp `(x, y)` to the nearest valid position on the inline layout
/// of `ifc_id`. Used when the hit cell isn't covered by a fragment —
/// drag past end-of-line, click past last-line bottom, etc.
///
/// Rules:
/// - `y < content.y` → first line's start position.
/// - `y >= content.y + content.height` → last line's end position.
/// - In-bounds y, x past line's content → that line's end position.
/// - In-bounds y, line is empty → walk to the nearest non-empty line.
fn clamp_to_line_layout(
    layout: &crate::render::inline::InlineLayout,
    content: crate::layout::LayoutRect,
    x: u16,
    y: u16,
) -> Option<Position> {
    if layout.lines.is_empty() {
        return None;
    }

    // A y-overshoot dominates the x logic: a point above the block snaps to
    // the FIRST line's start, below it to the LAST line's end — regardless of
    // x (matching the drag-extend clamp in `selection::drag`). Only an
    // in-bounds y consults x to pick the position along the line.
    let (line_idx, overshoot_up, overshoot_down) = if (y as i32) < content.y {
        (0, true, false)
    } else if (y as i32) >= content.y + content.height as i32 {
        (layout.lines.len() - 1, false, true)
    } else {
        let raw = (y as i32 - content.y) as usize;
        (raw.min(layout.lines.len() - 1), false, false)
    };

    let target_line = &layout.lines[line_idx];

    // Empty line — try walking out to find a non-empty fragment.
    // Falls back to the last line's last fragment if everything's
    // empty (shouldn't happen for a populated IFC, but defensive).
    if target_line.fragments.is_empty() {
        for line in layout.lines.iter().rev() {
            if let Some(frag) = line.fragments.last() {
                return Some(Position::new(
                    frag.text_node,
                    frag.source_byte_offset + frag.text.len(),
                ));
            }
        }
        return None;
    }

    let first = target_line.fragments.first().unwrap();
    let last = target_line.fragments.last().unwrap();

    // Y overshoot dominates: up → first line start, down → last line end.
    if overshoot_up {
        return Some(Position::new(first.text_node, first.source_byte_offset));
    }
    if overshoot_down {
        return Some(Position::new(
            last.text_node,
            last.source_byte_offset + last.text.len(),
        ));
    }

    // In-bounds y: clamp on x. Past the line's last fragment → end of last
    // fragment. Before the line's first fragment → start of first fragment.
    let line_left = content.x + first.x as i32;
    let line_right = content.x + last.x as i32 + last.width as i32;

    if (x as i32) < line_left {
        Some(Position::new(first.text_node, first.source_byte_offset))
    } else if (x as i32) >= line_right {
        Some(Position::new(
            last.text_node,
            last.source_byte_offset + last.text.len(),
        ))
    } else {
        // Somewhere in the middle of the line but no fragment
        // covered the cell (gap between fragments, shouldn't be
        // common). Clamp to the last fragment's end as a fallback.
        Some(Position::new(
            last.text_node,
            last.source_byte_offset + last.text.len(),
        ))
    }
}

/// Append nodes to `path` if `(x, y)` lands inside the subtree
/// rooted at `id`. Returns `true` when at least one node was added
/// at this level or deeper (lets the caller skip trying earlier
/// siblings).
/// Hit-test the stacking context rooted at `root` in reverse paint
/// order: child contexts with positive `z-index` (highest first), the
/// `z-index: auto | 0` layer in reverse tree order, the root's in-flow
/// content, child contexts with negative `z-index`, and finally the
/// root's own box. `clip` is the region the context paints into.
fn hit_stacking_context(
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
    if contains && !transparent {
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
        return match hit_fragment(dom, id, inner, x, y) {
            Some(owner) if owner != id => {
                append_inline_ancestors(dom, id, owner, path);
                true
            }
            _ => false,
        };
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
    let child_ids: Vec<NodeId> = dom.node(id).child_nodes().map(|n| n.id()).collect();
    for &child in child_ids.iter().rev() {
        let node = dom.node(child);
        let hit = match node.node_type() {
            NodeType::Fragment => descend_children_reverse(dom, child, x, y, clip, viewport, path),
            NodeType::Element => match node.ext().and_then(|e| e.computed.as_ref()) {
                Some(c) if is_positioned(c) => false,
                Some(c) if creates_stacking_context(c) => {
                    hit_stacking_context(dom, child, x, y, clip, viewport, path)
                }
                _ => descend_plain(dom, child, x, y, clip, viewport, path),
            },
            _ => false,
        };
        if hit {
            return true;
        }
    }
    false
}

/// Look up the inline fragment under `(x, y)` inside an IFC block's
/// content area. Returns the fragment's owner element (the direct
/// element parent of the underlying text — typically `<code>`, `<b>`,
/// or the IFC block itself when the text is a direct child).
fn hit_fragment(
    dom: &Dom<TuiExt>,
    ifc_block: NodeId,
    content: LayoutRect,
    x: u16,
    y: u16,
) -> Option<NodeId> {
    let ext = dom.node(ifc_block).ext()?;
    let layout = ext.inline_layout.as_ref()?;

    // Line index is the y-offset within content.
    let line_index = y as i32 - content.y;
    if line_index < 0 || line_index as usize >= layout.lines.len() {
        return None;
    }
    let line = &layout.lines[line_index as usize];

    // Local x within content.
    let x_local_i = x as i32 - content.x;
    if x_local_i < 0 {
        return None;
    }
    let x_local = x_local_i as u16;

    for fragment in &line.fragments {
        if x_local >= fragment.x && x_local < fragment.x + fragment.width {
            return Some(fragment.node);
        }
    }
    None
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
    // Collect inner → outer first, then reverse.
    let mut chain = Vec::new();
    let mut cur = owner;
    while cur != ifc_block {
        chain.push(cur);
        match dom.node(cur).parent_node() {
            Some(parent) => cur = parent.id(),
            None => break, // defensive — should never trigger in a well-formed tree
        }
    }
    chain.reverse();
    path.extend(chain);
}

#[inline]
fn rect_contains(r: LayoutRect, x: u16, y: u16) -> bool {
    let x = x as i32;
    let y = y as i32;
    x >= r.x && x < r.x + r.width as i32 && y >= r.y && y < r.y + r.height as i32
}

/// Look up the `InlineFragment` under `(x, y)` inside an IFC
/// block's content area. Returns a reference into the block's
/// stored `InlineLayout` — the caller extracts whatever info it
/// needs (owner, text_node, source offset) without cloning.
fn fragment_at_layout(
    layout: &crate::render::inline::InlineLayout,
    content: LayoutRect,
    x: u16,
    y: u16,
) -> Option<&InlineFragment> {
    let line_index = y as i32 - content.y;
    if line_index < 0 || line_index as usize >= layout.lines.len() {
        return None;
    }
    let line = &layout.lines[line_index as usize];

    let x_local_i = x as i32 - content.x;
    if x_local_i < 0 {
        return None;
    }
    let x_local = x_local_i as u16;

    line.fragments
        .iter()
        .find(|&fragment| x_local >= fragment.x && x_local < fragment.x + fragment.width)
        .map(|v| v as _)
}

/// Walk graphemes of `text` counting cell widths; return the byte
/// offset of the grapheme whose cell range contains `target_cells`.
///
/// Cell grain is per-grapheme (1 for ASCII, 2 for CJK, etc.), not
/// byte length. If `target_cells` falls inside a wide grapheme, the
/// returned offset is the grapheme's *start* byte — the click snaps
/// to the left edge of the character. If `target_cells` overshoots
/// the text's total cell width, returns `text.len()`.
fn cells_to_bytes(text: &str, target_cells: u16) -> usize {
    let mut consumed_cells: u16 = 0;
    for (idx, g) in text.grapheme_indices(true) {
        let w = UnicodeWidthStr::width(g) as u16;
        if target_cells < consumed_cells.saturating_add(w) {
            return idx;
        }
        consumed_cells = consumed_cells.saturating_add(w);
    }
    text.len()
}

#[cfg(test)]
mod tests;
