//! Positioning — containing-block resolution + phase-2 placement
//! for `position: absolute | fixed` elements (M2).
//!
//! Containing-block resolution:
//!
//! - `position: fixed` → always the initial containing block (the
//!   root viewport).
//! - `position: absolute` → the nearest ancestor whose
//!   `position` is `relative | absolute | fixed`, or the viewport
//!   if none.
//! - `position: relative` / `static` → returns the parent's
//!   layout rect; used for the §5 static-position fallback.
//!
//! Phase-2 placement walks the tree in document order and, for
//! every element with `position: absolute | fixed`, resolves its
//! containing block, computes the placed rect from
//! `top/right/bottom/left` + `width/height`, writes it into
//! `TuiExt.layout`, and re-runs `layout_node` on the subtree so
//! the element's own children flow inside the placed rect.
//!
//! An axis whose two insets are both `auto` starts at the element's
//! **static position** (CSS 2.1 §10.3.7 / §10.6.4): phase-1 block,
//! inline and flex layout record it through [`record_static_position`]
//! at the point in the flow where the element's hypothetical box would
//! have gone, and [`compute_placed_rect`] reads it back.

use std::collections::HashMap;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{PseudoSlot, StaticPosition, TuiExt};
use crate::layout::{Display, LayoutRect, Length, Position, Size};
use crate::node::TuiNodeExt;
use crate::render::inline::InlineLayout;
use crate::style::ComputedStyle;

/// Resolve the containing block rect for `id`, given the root
/// viewport. The element's own `position` decides:
///
/// - `Fixed` → viewport.
/// - `Absolute` → ancestor walk; first positioned (relative,
///   absolute, fixed) ancestor's layout rect; viewport on miss.
/// - `Relative` / `Static` → returns the parent's content area
///   (or viewport if no parent), matching the in-flow position.
///   (Used by phase-2 callers that ask "where would this be in
///   flow?" for static-position resolution; see §5 of the spec.)
pub(crate) fn containing_block(dom: &Dom<TuiExt>, id: NodeId, viewport: LayoutRect) -> LayoutRect {
    let position = computed_position(dom, id);

    if position == Position::Fixed {
        return viewport;
    }

    if position == Position::Absolute {
        let mut cur = parent_id(dom, id);
        while let Some(p) = cur {
            let pp = computed_position(dom, p);
            if matches!(
                pp,
                Position::Relative | Position::Absolute | Position::Fixed
            ) {
                return layout_rect(dom, p).unwrap_or(viewport);
            }
            cur = parent_id(dom, p);
        }
        return viewport;
    }

    // Static / Relative: containing block = parent's layout rect
    // (or viewport if no parent in the layout tree yet).
    parent_id(dom, id)
        .and_then(|p| layout_rect(dom, p))
        .unwrap_or(viewport)
}

pub(super) fn computed_position(dom: &Dom<TuiExt>, id: NodeId) -> Position {
    dom.node(id)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .map(|c| c.position)
        .unwrap_or_default()
}

pub(super) fn layout_rect(dom: &Dom<TuiExt>, id: NodeId) -> Option<LayoutRect> {
    dom.node(id).ext().map(|e| e.layout)
}

pub(super) fn parent_id(dom: &Dom<TuiExt>, id: NodeId) -> Option<NodeId> {
    dom.node(id).parent_node().map(|p| p.id())
}

// ── Static position (CSS 2.1 §10.3.7 / §10.6.4) ────────────────

/// Record where `id` would sit if it were `position: static`. Called
/// by phase-1 layout for each out-of-flow positioned child, in the
/// coordinate space of the parent's content area (scroll applied).
pub(super) fn record_static_position(dom: &mut Dom<TuiExt>, id: NodeId, x: i32, y: i32) {
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.static_position = Some(StaticPosition { x, y });
    }
}

/// `true` for an element that phase 1 leaves out of the flow because
/// phase 2 places it: `position: absolute | fixed` and not
/// `display: none` (which generates no box at all).
pub(super) fn is_out_of_flow_positioned(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    if node.node_type() != NodeType::Element {
        return false;
    }
    node.ext()
        .and_then(|e| e.computed.as_ref())
        .is_some_and(|c| {
            c.display != Display::None && matches!(c.position, Position::Absolute | Position::Fixed)
        })
}

/// The direct children of `parent` that [`is_out_of_flow_positioned`],
/// in document order.
pub(super) fn out_of_flow_positioned_children(dom: &Dom<TuiExt>, parent: NodeId) -> Vec<NodeId> {
    dom.node(parent)
        .child_nodes()
        .map(|c| c.id())
        .filter(|&c| is_out_of_flow_positioned(dom, c))
        .collect()
}

/// Group a flow's out-of-flow positioned children by the in-flow
/// sibling that follows them: `before[k]` are the positioned children
/// immediately ahead of in-flow child `k`; `trailing` are those after
/// the last in-flow child. Block layout records each group's static
/// position from its flow cursor just before it places `k`.
pub(super) fn static_anchors(
    dom: &Dom<TuiExt>,
    children: &[NodeId],
) -> (HashMap<NodeId, Vec<NodeId>>, Vec<NodeId>) {
    let mut before: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let mut bucket: Vec<NodeId> = Vec::new();
    for &c in children {
        if is_out_of_flow_positioned(dom, c) {
            bucket.push(c);
        } else if super::is_in_flow(dom, c) && !bucket.is_empty() {
            before.insert(c, std::mem::take(&mut bucket));
        }
    }
    (before, bucket)
}

/// Static position of an out-of-flow child of an inline formatting
/// context. An inline-level hypothetical box continues the line after
/// the preceding in-flow content; a block-level one starts the next
/// line at the content's left edge (CSS 2.1 §9.2.1.1 + §10.3.7). With
/// no preceding in-flow content the box sits at the IFC's origin.
///
/// `layout` is the IFC's packed lines and `origin` the rect they were
/// packed into (the block's content area or the anonymous box's rect).
pub(super) fn static_position_in_ifc(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    child: NodeId,
    layout: &InlineLayout,
    origin: LayoutRect,
) -> (i32, i32) {
    // Fragments are owned by the (possibly nested) node that carries
    // their text; attribute each to the direct child of `parent` it
    // sits under so it can be ordered against `child`.
    let sibling_index: HashMap<NodeId, usize> = dom
        .node(parent)
        .child_nodes()
        .enumerate()
        .map(|(i, c)| (c.id(), i))
        .collect();
    let Some(&child_index) = sibling_index.get(&child) else {
        return (origin.x, origin.y);
    };
    let top_level_index = |mut node: NodeId| -> Option<usize> {
        loop {
            if let Some(&i) = sibling_index.get(&node) {
                return Some(i);
            }
            node = dom.node(node).parent_node()?.id();
        }
    };
    // The end of the preceding content: the furthest `(line, x)` any
    // ahead item reaches (items on a line are left to right, generated
    // runs interleaved with the fragments).
    let mut last: Option<(usize, i32)> = None;
    for (line_idx, line) in layout.lines.iter().enumerate() {
        // Generated content hosted by a child's subtree is ordered with
        // that child (an inline element's pseudos sit at its start /
        // end); otherwise it is the parent's own (or a list marker
        // riding this first line): its `::before` is ahead of every
        // child, its `::after` after them all.
        for g in &line.generated {
            let ahead = match top_level_index(g.host) {
                Some(i) => i < child_index,
                None => g.slot == PseudoSlot::Before,
            };
            if ahead {
                last = last.max(Some((line_idx, i32::from(g.x) + i32::from(g.width))));
            }
        }
        for f in &line.fragments {
            let owner = top_level_index(f.text_node).or_else(|| top_level_index(f.node));
            if owner.is_some_and(|i| i < child_index) {
                last = last.max(Some((line_idx, i32::from(f.x) + i32::from(f.width))));
            }
        }
    }
    let inline_level = dom
        .node(child)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .is_some_and(|c| matches!(c.display, Display::Inline | Display::InlineBlock));
    match last {
        Some((line, end)) if inline_level => (origin.x + end, origin.y + line as i32),
        Some((line, _)) => (origin.x, origin.y + line as i32 + 1),
        None => (origin.x, origin.y),
    }
}

/// Record the static position of every out-of-flow positioned child
/// of the IFC `parent` (see [`static_position_in_ifc`]).
pub(super) fn record_static_positions_in_ifc(
    dom: &mut Dom<TuiExt>,
    parent: NodeId,
    layout: &InlineLayout,
    origin: LayoutRect,
) {
    for n in out_of_flow_positioned_children(dom, parent) {
        let (x, y) = static_position_in_ifc(dom, parent, n, layout, origin);
        record_static_position(dom, n, x, y);
    }
}

// ── Relative shift (M2 §12.6) ──────────────────────────────────

/// `position: relative` shifts the element's painted rect by
/// `top` / `left` (or `right` / `bottom` if the corresponding
/// edge is `Auto` and the opposite edge is `Cells`). Returns the
/// shifted rect.
///
/// Applied inside `layout_node` before writing the rect, so the
/// element's own children flow inside the shifted rect. Siblings
/// are unaffected — the parent's flex / IFC distribution had
/// already finalized their positions before this element's
/// `layout_node` runs.
///
/// Per CSS, when both edges of an axis are specified, `top` /
/// `left` win and `bottom` / `right` are ignored. A percentage
/// `top` / `bottom` computes to `auto` unless `parent_height_definite`
/// (CSS 2.1 §9.3.2: the containing block's height must be specified
/// explicitly).
pub(super) fn apply_relative_shift(
    computed: &ComputedStyle,
    rect: LayoutRect,
    parent: LayoutRect,
    parent_height_definite: bool,
) -> LayoutRect {
    if computed.position != Position::Relative {
        return rect;
    }
    // Relative offsets resolve percentages against the parent's
    // content box on the matching axis (`top`/`bottom` → height,
    // `left`/`right` → width). Per CSS 2.1 §9.4.3.
    let dx =
        resolve_length_offset(&computed.left, parent.width as i32, false).unwrap_or_else(|| {
            resolve_length_offset(&computed.right, parent.width as i32, true).unwrap_or(0)
        });
    let vertical = |len: &Length| -> Length {
        match len {
            Length::Calc(expr) if !parent_height_definite && expr.contains_percent() => {
                Length::Auto
            }
            other => other.clone(),
        }
    };
    let dy = resolve_length_offset(&vertical(&computed.top), parent.height as i32, false)
        .unwrap_or_else(|| {
            resolve_length_offset(&vertical(&computed.bottom), parent.height as i32, true)
                .unwrap_or(0)
        });
    LayoutRect::new(
        rect.x.saturating_add(dx),
        rect.y.saturating_add(dy),
        rect.width,
        rect.height,
    )
}

/// Resolve a `Length` value to a signed integer offset given the
/// axis basis. `negate` flips the sign (used for the `bottom`/
/// `right` insets which point inward from the opposite edge).
/// Returns `None` for `Length::Auto`.
fn resolve_length_offset(len: &Length, basis: i32, negate: bool) -> Option<i32> {
    let cells = match len {
        Length::Auto => return None,
        Length::Cells(n) => *n,
        Length::Calc(expr) => expr.resolve(&rdom_style::calc::ResolveCtx::new(basis)),
    };
    Some(if negate { -cells } else { cells })
}

// ── Phase 2 placement ───────────────────────────────────────────

/// After phase-1 flex layout completes, walk the tree in document
/// order and place every `position: absolute | fixed` element
/// against its containing block. For each placed element, re-run
/// `layout_node` on the subtree so the element's own children flow
/// inside the placed rect.
///
/// Document-order walk guarantees that an outer positioned element
/// is placed before any positioned descendants — so when a nested
/// absolute resolves its containing block, the outer's
/// `TuiExt.layout` is already populated.
pub(super) fn place_positioned(dom: &mut Dom<TuiExt>, viewport: LayoutRect) {
    let positioned = collect_positioned(dom, dom.root());
    for id in positioned {
        let cb = containing_block(dom, id, viewport);
        let computed = dom
            .node(id)
            .computed_rc()
            .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
        let placed = compute_placed_rect(dom, id, &computed, cb);
        super::layout_node(dom, id, placed, cb.width);
    }
}

fn collect_positioned(dom: &Dom<TuiExt>, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk_for_positioned(dom, id, &mut out);
    out
}

fn walk_for_positioned(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    if dom.node(id).node_type() == NodeType::Element {
        let pos = computed_position(dom, id);
        if matches!(pos, Position::Absolute | Position::Fixed) {
            out.push(id);
        }
    }
    for child in dom.node(id).child_nodes() {
        match child.node_type() {
            NodeType::Element | NodeType::Fragment => {
                walk_for_positioned(dom, child.id(), out);
            }
            _ => {}
        }
    }
}

/// Compute the placed rect for an absolute/fixed element given its
/// computed style and resolved containing block.
///
/// Width / height resolve in this order:
/// - `Size::Fixed(n)` → `n`.
/// - `Size::Flex(_)` → fills the containing block on that axis.
/// - `Size::Percent(p)` → `cb_axis * p / 100`. Resolves against the
///   *containing block* — for absolute/fixed positioning, that's the
///   nearest positioned ancestor (or the viewport for `fixed`).
/// - `Size::Auto`:
///   - When both edges of the axis are `Cells`, derive from
///     `cb_axis - left - right` (or `cb_axis - top - bottom`).
///   - Otherwise shrink-to-fit the content (CSS 2.1 §10.3.7 /
///     §10.6.4): the element's intrinsic size on that axis, height
///     measured at the resolved width. A tooltip positioned with
///     only `top` / `left` is therefore as wide as its text, not 0.
///
/// X / Y resolve from the offsets via [`axis_position_anchored`];
/// an axis with both insets `auto` takes `TuiExt::static_position`.
fn compute_placed_rect(
    dom: &Dom<TuiExt>,
    id: NodeId,
    c: &ComputedStyle,
    cb: LayoutRect,
) -> LayoutRect {
    use crate::layout::Direction;
    // Resolve width/height — percentage AND Calc both resolve
    // against the containing-block's matching axis. The intrinsic
    // measurement only runs when an `auto` axis is not pinned by both
    // edges (it walks the subtree).
    let width = resolve_size_axis(&c.width, cb.width, &c.left, &c.right, cb.width, || {
        super::intrinsic::intrinsic_size(dom, id, Direction::Row, cb.width, cb.width)
    });
    let height = resolve_size_axis(&c.height, cb.height, &c.top, &c.bottom, cb.height, || {
        super::intrinsic::intrinsic_size(dom, id, Direction::Column, width, cb.width)
    });

    // M5.3b — absolute element centering via `margin: auto` between
    // resolved insets. CSS rule: when both axis insets are `Cells`
    // (non-auto) AND the corresponding axis margins are both `Auto`,
    // distribute remaining space equally to both margins — i.e.
    // center the element between the insets.
    use crate::layout::MarginValue;
    let (cx_left, cx_right) = (c.margin.left.clone(), c.margin.right.clone());
    let (cy_top, cy_bottom) = (c.margin.top.clone(), c.margin.bottom.clone());
    // Margin percent / calc resolves against the containing-block
    // width on ALL four sides (CSS 2.1 §8.3).
    let margin_cb_w = cb.width;

    let basis_w = cb.width as i32;
    let basis_h = cb.height as i32;

    // An axis with both insets `auto` starts at the static position
    // phase 1 recorded (CSS 2.1 §10.3.7 / §10.6.4). It is `None` only
    // for an element whose parent has not been laid out yet; the
    // containing block's start stands in then.
    let static_pos = dom.node(id).ext().and_then(|e| e.static_position);

    let x = if length_to_cells_opt(&c.left, basis_w).is_some()
        && length_to_cells_opt(&c.right, basis_w).is_some()
        && matches!(cx_left, MarginValue::Auto)
        && matches!(cx_right, MarginValue::Auto)
    {
        // Center horizontally between left + right insets.
        let left = length_to_cells_opt(&c.left, basis_w).unwrap_or(0);
        let right = length_to_cells_opt(&c.right, basis_w).unwrap_or(0);
        let span = basis_w.saturating_sub(left + right);
        let extra = span.saturating_sub(width as i32).max(0);
        cb.x + left + extra / 2
    } else {
        let base = match static_pos {
            Some(sp) if matches!((&c.left, &c.right), (Length::Auto, Length::Auto)) => sp.x,
            _ => axis_position_anchored(&c.left, &c.right, cb.x, cb.width, width),
        };
        let start_margin = match &cx_left {
            MarginValue::Cells(n) => *n as i32,
            MarginValue::Auto => 0,
            MarginValue::Calc(_) => cx_left.resolve(margin_cb_w) as i32,
        };
        base + start_margin
    };
    let y = if length_to_cells_opt(&c.top, basis_h).is_some()
        && length_to_cells_opt(&c.bottom, basis_h).is_some()
        && matches!(cy_top, MarginValue::Auto)
        && matches!(cy_bottom, MarginValue::Auto)
    {
        let top = length_to_cells_opt(&c.top, basis_h).unwrap_or(0);
        let bottom = length_to_cells_opt(&c.bottom, basis_h).unwrap_or(0);
        let span = basis_h.saturating_sub(top + bottom);
        let extra = span.saturating_sub(height as i32).max(0);
        cb.y + top + extra / 2
    } else {
        let base = match static_pos {
            Some(sp) if matches!((&c.top, &c.bottom), (Length::Auto, Length::Auto)) => sp.y,
            _ => axis_position_anchored(&c.top, &c.bottom, cb.y, cb.height, height),
        };
        let start_margin = match &cy_top {
            MarginValue::Cells(n) => *n as i32,
            MarginValue::Auto => 0,
            MarginValue::Calc(_) => cy_top.resolve(margin_cb_w) as i32,
        };
        base + start_margin
    };
    LayoutRect::new(x, y, width, height)
}

/// Resolve a `Size` against a basis (parent's matching-axis
/// content dimension). Handles all `Size` variants including
/// `Size::Calc`. For `Size::Auto`, falls back to deriving from
/// the start/end edges when both are non-auto.
fn resolve_size_axis(
    size: &Size,
    cb_extent: u16,
    start: &Length,
    end: &Length,
    edges_basis: u16,
    shrink_to_fit: impl FnOnce() -> u16,
) -> u16 {
    match size {
        Size::Fixed(n) => *n,
        Size::Flex(_) => cb_extent,
        Size::Percent(p) => Size::percent_of(cb_extent as i32, *p).clamp(0, u16::MAX as i32) as u16,
        Size::Calc(expr) => {
            let v = expr.resolve(&rdom_style::calc::ResolveCtx::new(cb_extent as i32));
            v.max(0).min(u16::MAX as i32) as u16
        }
        Size::Auto => {
            let both_edges = length_to_cells_opt(start, edges_basis as i32).is_some()
                && length_to_cells_opt(end, edges_basis as i32).is_some();
            if both_edges {
                axis_size_from_edges(start, end, edges_basis, 0)
            } else {
                shrink_to_fit()
            }
        }
    }
}

/// Resolve a `Length` to `Option<i32>` cells. Wrapper used by
/// the per-axis branches above; `length_to_cells` (in the
/// `Length` resolver section) is a private helper from the same
/// module.
fn length_to_cells_opt(len: &Length, basis: i32) -> Option<i32> {
    length_to_cells(len, basis)
}

// ── Shared offset resolvers (consumed by absolute/fixed element
//    placement AND by positioned-pseudo placement) ───────────────

/// Resolve size on one axis when both edges are `Cells`, otherwise
/// return `fallback`. Per CSS, an `auto` width on a positioned box
/// only resolves to `cb_extent - start - end` when both edges are
/// specified; one-sided cases fall back to an intrinsic measure
/// (caller passes `0` for elements, content width for pseudos).
pub(super) fn axis_size_from_edges(
    start: &Length,
    end: &Length,
    cb_extent: u16,
    fallback: u16,
) -> u16 {
    // Resolve both edges into Option<i32>. `Auto` → None, others
    // → Some(cells). When both are Some, derive size from the
    // extent minus both insets.
    let basis = cb_extent as i32;
    let s = length_to_cells(start, basis);
    let e = length_to_cells(end, basis);
    match (s, e) {
        (Some(s), Some(e)) => {
            let span = s.saturating_add(e);
            (basis.saturating_sub(span)).max(0) as u16
        }
        _ => fallback,
    }
}

/// Resolve a `Length` to a signed integer cell count given the
/// percent basis (parent's axis extent). Returns `None` for
/// `Length::Auto`. Shared helper for the offset/size resolvers
/// in this module.
fn length_to_cells(len: &Length, basis: i32) -> Option<i32> {
    match len {
        Length::Auto => None,
        Length::Cells(n) => Some(*n),
        Length::Calc(expr) => Some(expr.resolve(&rdom_style::calc::ResolveCtx::new(basis))),
    }
}

/// Resolve start position on one axis using the CSS anchored-offset
/// semantics that govern `position: absolute | fixed`:
///
/// - `(Cells(s), _)` → `cb_start + s` (start edge wins per CSS).
/// - `(Auto, Cells(e))` → `cb_start + cb_extent - e - size` (anchor
///   flips to far edge, going inward).
/// - `(Auto, Auto)` → `cb_start`. Element placement substitutes the
///   recorded static position before reaching this case; pseudo
///   placement and an element without one land at the containing
///   block's start.
pub(super) fn axis_position_anchored(
    start: &Length,
    end: &Length,
    cb_start: i32,
    cb_extent: u16,
    size: u16,
) -> i32 {
    let basis = cb_extent as i32;
    let s = length_to_cells(start, basis);
    let e = length_to_cells(end, basis);
    match (s, e) {
        (Some(s), _) => cb_start.saturating_add(s),
        (None, Some(e)) => cb_start
            .saturating_add(basis)
            .saturating_sub(e)
            .saturating_sub(size as i32),
        _ => cb_start,
    }
}

/// Resolve start position on one axis using `position: relative`
/// shift semantics (the box stays at its natural anchor and only
/// shifts by `start` or `-end`):
///
/// - `(Cells(s), _)` → `anchor + s`.
/// - `(Auto, Cells(e))` → `anchor - e`.
/// - `(Auto, Auto)` → `anchor`.
///
/// Used by positioned-pseudo placement (`positioned_pseudos.rs`)
/// when the pseudo's cascaded `position` is `Relative`. Element
/// `position: relative` uses a separate path
/// ([`apply_relative_shift`]) because element placement reads `top`
/// / `left` / `right` / `bottom` as a *delta* against the in-flow
/// rect, not against a containing block.
pub(super) fn axis_position_relative_shift(
    start: &Length,
    end: &Length,
    anchor: i32,
    basis: i32,
) -> i32 {
    let s = length_to_cells(start, basis);
    let e = length_to_cells(end, basis);
    match (s, e) {
        (Some(s), _) => anchor.saturating_add(s),
        (None, Some(e)) => anchor.saturating_sub(e),
        _ => anchor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Length, ZIndex};
    use crate::render::rect::Rect;
    use crate::style::Value;
    use crate::style::{ComputedStyle, Stylesheet, TuiStyle};
    use crate::{CascadeExt, LayoutExt, TuiDom};

    fn build_dom_with_positioned_chain(positions: &[Position]) -> (TuiDom, Vec<NodeId>) {
        // Build a vertical chain root → child[0] → child[1] → ...
        // with the requested computed `position` on each child.
        let mut dom: TuiDom = TuiDom::new();
        let mut ids = Vec::with_capacity(positions.len());
        let root = dom.root();
        let mut parent = root;
        for (i, _p) in positions.iter().enumerate() {
            let id = dom.create_element("div");
            dom.node_mut(id).set_id(&format!("n{i}")).unwrap();
            dom.append_child(parent, id).unwrap();
            ids.push(id);
            parent = id;
        }

        // Author rules: `#nN { position: <p>; }` for each.
        let mut sheet = Stylesheet::bare();
        for (i, p) in positions.iter().enumerate() {
            sheet = sheet.rule_unchecked(&format!("#n{i}"), TuiStyle::new().position(*p));
        }
        dom.cascade(&sheet);
        // Run layout to populate rects.
        let viewport = Rect::new(0, 0, 100, 50);
        dom.layout_dom(viewport);
        (dom, ids)
    }

    fn viewport() -> LayoutRect {
        LayoutRect::new(0, 0, 100, 50)
    }

    #[test]
    fn absolute_with_no_positioned_ancestor_returns_viewport() {
        let (dom, ids) = build_dom_with_positioned_chain(&[
            Position::Static,
            Position::Static,
            Position::Absolute,
        ]);
        let cb = containing_block(&dom, ids[2], viewport());
        assert_eq!(cb, viewport());
    }

    #[test]
    fn absolute_inside_relative_uses_relative_parent() {
        let (dom, ids) = build_dom_with_positioned_chain(&[
            Position::Static,
            Position::Relative,
            Position::Absolute,
        ]);
        let parent_rect = dom.node(ids[1]).ext().unwrap().layout;
        let cb = containing_block(&dom, ids[2], viewport());
        assert_eq!(cb, parent_rect);
    }

    #[test]
    fn absolute_skips_static_ancestors_to_find_relative() {
        let (dom, ids) = build_dom_with_positioned_chain(&[
            Position::Relative, // grandparent
            Position::Static,   // parent (skipped)
            Position::Absolute, // self
        ]);
        let grandparent_rect = dom.node(ids[0]).ext().unwrap().layout;
        let cb = containing_block(&dom, ids[2], viewport());
        assert_eq!(cb, grandparent_rect);
    }

    #[test]
    fn absolute_inside_absolute_uses_absolute_parent() {
        let (dom, ids) = build_dom_with_positioned_chain(&[
            Position::Static,
            Position::Absolute,
            Position::Absolute,
        ]);
        let parent_rect = dom.node(ids[1]).ext().unwrap().layout;
        let cb = containing_block(&dom, ids[2], viewport());
        assert_eq!(cb, parent_rect);
    }

    #[test]
    fn fixed_always_uses_viewport_even_with_relative_ancestor() {
        let (dom, ids) = build_dom_with_positioned_chain(&[
            Position::Static,
            Position::Relative,
            Position::Fixed,
        ]);
        let cb = containing_block(&dom, ids[2], viewport());
        // Fixed ignores ancestors; viewport always wins.
        assert_eq!(cb, viewport());
    }

    #[test]
    fn fixed_uses_viewport_when_no_ancestors_positioned() {
        let (dom, ids) =
            build_dom_with_positioned_chain(&[Position::Static, Position::Static, Position::Fixed]);
        let cb = containing_block(&dom, ids[2], viewport());
        assert_eq!(cb, viewport());
    }

    #[test]
    fn static_returns_parent_layout() {
        let (dom, ids) = build_dom_with_positioned_chain(&[
            Position::Static,
            Position::Static,
            Position::Static,
        ]);
        let parent_rect = dom.node(ids[1]).ext().unwrap().layout;
        let cb = containing_block(&dom, ids[2], viewport());
        assert_eq!(cb, parent_rect);
    }

    /// Document existence; not a behavioral assertion. M2 callers
    /// invoke `containing_block` only on absolute/fixed; static
    /// behavior is documented for completeness.
    #[allow(dead_code)]
    fn _types_compile() {
        let _: ComputedStyle = ComputedStyle::initial();
        let _: Value<Length> = Value::Specified(Length::Auto);
        let _: ZIndex = ZIndex::Auto;
    }
}
