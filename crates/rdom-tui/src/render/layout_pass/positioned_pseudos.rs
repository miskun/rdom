//! Phase-3 placement for positioned `::before` / `::after` pseudo-
//! elements (M5-now Stage B).
//!
//! Runs after `place_positioned` (phase 2) so absolute pseudos whose
//! hosts are themselves absolutely positioned can read the host's
//! placed rect as their containing block.
//!
//! Writes `TuiExt.before_layout` / `after_layout`. Does NOT call
//! `layout_node` — pseudos have no `NodeId` of their own.
//!
//! ## Divergences from CSS
//!
//! - **Relative-pseudo natural position** comes from the host's
//!   layout rect (start edge for `::before`, end edge for `::after`)
//!   — not from the inline-formatting cursor like the browser
//!   does. Pseudos with `position: relative` are uncommon enough
//!   that the simplified anchor is acceptable for 0.1.0.
//! - **`Auto` width / height** fall back to the pseudo's intrinsic
//!   `content` size (UAX#11 `UnicodeWidthStr`). CSS would solve
//!   `width = cb_width - left - right` even when only one edge is
//!   specified; rdom only does that when *both* edges are `Cells`.

use rdom_core::{Dom, NodeId, NodeType};
use unicode_width::UnicodeWidthStr;

use crate::ext::{PseudoLayout, TuiExt};
use crate::layout::{Display, LayoutRect, Position};
use crate::style::ComputedStyle;

use super::positioning::{
    axis_position_anchored, axis_position_relative_shift, axis_size_from_edges, computed_position,
    layout_rect, parent_id,
};

pub(super) fn place_positioned_pseudos(dom: &mut Dom<TuiExt>, viewport: LayoutRect) {
    // Cascade-level early-exit (D-M5N-2). If no element in the tree
    // has a positioned pseudo, skip the full-tree walk entirely.
    // Cascade aggregates `tree_has_positioned_pseudo` bottom-up; OR
    // across the root's children to handle Fragment roots.
    if !tree_has_any_positioned_pseudo(dom) {
        return;
    }

    // Hosts that have a `display: none` ancestor are skipped (matches
    // CSS — `display: none` suppresses the entire subtree including
    // generated content). The visibility check walks from the host up.
    let candidates = collect_hosts(dom, dom.root());

    for host in candidates {
        if is_display_none_subtree(dom, host) {
            continue;
        }
        place_one(dom, host, viewport);
    }
}

/// OR `tree_has_positioned_pseudo` across the root's children
/// (transparently descending through nested Fragments). `dom.root()`
/// is itself a Fragment by default and has no `TuiExt`, so the flag
/// is carried by the first element layer beneath it.
fn tree_has_any_positioned_pseudo(dom: &Dom<TuiExt>) -> bool {
    fn walk(dom: &Dom<TuiExt>, id: NodeId) -> bool {
        for child in dom.node(id).child_nodes() {
            let hit = match child.node_type() {
                NodeType::Element => child.ext().is_some_and(|e| e.tree_has_positioned_pseudo),
                NodeType::Fragment => walk(dom, child.id()),
                _ => false,
            };
            if hit {
                return true;
            }
        }
        false
    }
    walk(dom, dom.root())
}

fn collect_hosts(dom: &Dom<TuiExt>, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk(dom, id, &mut out);
    out
}

fn walk(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    if dom.node(id).node_type() == NodeType::Element
        && let Some(ext) = dom.node(id).ext()
    {
        let has_positioned_before = ext
            .computed_before
            .as_ref()
            .is_some_and(|c| c.position != Position::Static);
        let has_positioned_after = ext
            .computed_after
            .as_ref()
            .is_some_and(|c| c.position != Position::Static);
        if has_positioned_before || has_positioned_after {
            out.push(id);
        }
    }
    for child in dom.node(id).child_nodes() {
        match child.node_type() {
            NodeType::Element | NodeType::Fragment => {
                walk(dom, child.id(), out);
            }
            _ => {}
        }
    }
}

fn is_display_none_subtree(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let mut cur = Some(id);
    while let Some(node) = cur {
        if dom
            .node(node)
            .ext()
            .and_then(|e| e.computed.as_ref())
            .map(|c| c.display == Display::None)
            .unwrap_or(false)
        {
            return true;
        }
        cur = parent_id(dom, node);
    }
    false
}

fn place_one(dom: &mut Dom<TuiExt>, host: NodeId, viewport: LayoutRect) {
    // Snapshot the pseudo styles + host's own layout BEFORE touching
    // the ext (write borrows below).
    let (before, after, host_rect) = {
        let ext = dom.node(host).ext().expect("host has ext");
        (
            ext.computed_before.clone(),
            ext.computed_after.clone(),
            ext.layout,
        )
    };

    if let Some(before_style) = before
        && before_style.position != Position::Static
    {
        let cb = resolve_containing_block(dom, host, &before_style, viewport, host_rect);
        let rect = compute_placed_rect(&before_style, cb, host_rect, PseudoEnd::Before);
        if let Some(ext) = dom.node_mut(host).ext_mut() {
            ext.before_layout = Some(PseudoLayout {
                rect,
                position: before_style.position,
            });
        }
    }
    if let Some(after_style) = after
        && after_style.position != Position::Static
    {
        let cb = resolve_containing_block(dom, host, &after_style, viewport, host_rect);
        let rect = compute_placed_rect(&after_style, cb, host_rect, PseudoEnd::After);
        if let Some(ext) = dom.node_mut(host).ext_mut() {
            ext.after_layout = Some(PseudoLayout {
                rect,
                position: after_style.position,
            });
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum PseudoEnd {
    Before,
    After,
}

/// Resolve the containing block for a positioned pseudo. The ancestor
/// walk starts at the host element per the spec § "Containing block
/// resolution":
///
/// - `Position::Fixed` → viewport.
/// - `Position::Absolute` → nearest positioned ancestor starting from
///   the host; if the host itself is positioned, the host is the CB.
///   Otherwise walk up.
/// - `Position::Relative` → the host's own layout rect (the pseudo
///   shifts from its natural inline position, which sits inside the
///   host's content area).
fn resolve_containing_block(
    dom: &Dom<TuiExt>,
    host: NodeId,
    pseudo: &ComputedStyle,
    viewport: LayoutRect,
    host_rect: LayoutRect,
) -> LayoutRect {
    match pseudo.position {
        Position::Fixed => viewport,
        Position::Absolute => {
            // Ancestor walk STARTS at the host element. If the host
            // is positioned, host is the CB.
            let host_position = dom
                .node(host)
                .ext()
                .and_then(|e| e.computed.as_ref())
                .map(|c| c.position)
                .unwrap_or_default();
            if matches!(
                host_position,
                Position::Relative | Position::Absolute | Position::Fixed
            ) {
                return host_rect;
            }
            let mut cur = parent_id(dom, host);
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
            viewport
        }
        // Sticky behaves as Relative for the pseudo's containing block
        // rule — its placed rect is still rooted at the host.
        Position::Relative | Position::Static | Position::Sticky => host_rect,
    }
}

fn pseudo_content_width(style: &ComputedStyle) -> u16 {
    style
        .content
        .as_deref()
        .map(|s| UnicodeWidthStr::width(s) as u16)
        .unwrap_or(0)
}

fn pseudo_content_height(style: &ComputedStyle) -> u16 {
    // Single line unless the content carries explicit newlines.
    style
        .content
        .as_deref()
        .map(|s| s.lines().count().max(1) as u16)
        .unwrap_or(1)
}

/// Compute the placed rect for a positioned pseudo. Width / height
/// resolve via [`axis_size_from_edges`] with the pseudo's intrinsic
/// content size as fallback. Position then routes through
/// [`axis_position_anchored`] for absolute/fixed, or
/// [`axis_position_relative_shift`] anchored at the host's edge for
/// `Position::Relative` (see module docs for the natural-position
/// divergence from CSS).
fn compute_placed_rect(
    style: &ComputedStyle,
    cb: LayoutRect,
    host_rect: LayoutRect,
    end: PseudoEnd,
) -> LayoutRect {
    let intrinsic_w = pseudo_content_width(style);
    let intrinsic_h = pseudo_content_height(style);

    let width = axis_size_from_edges(&style.left, &style.right, cb.width, intrinsic_w);
    let height = axis_size_from_edges(&style.top, &style.bottom, cb.height, intrinsic_h);

    if style.position == Position::Relative {
        // Relative pseudo: natural anchor is the host's start edge
        // for `::before`, host's far edge - intrinsic_w for `::after`.
        // The cascaded `top/left/right/bottom` then shift from there.
        let (natural_x, natural_y) = match end {
            PseudoEnd::Before => (host_rect.x, host_rect.y),
            PseudoEnd::After => (
                host_rect
                    .x
                    .saturating_add(host_rect.width as i32)
                    .saturating_sub(intrinsic_w as i32),
                host_rect.y,
            ),
        };
        let x = axis_position_relative_shift(&style.left, &style.right, natural_x, cb.width as i32);
        let y =
            axis_position_relative_shift(&style.top, &style.bottom, natural_y, cb.height as i32);
        LayoutRect::new(x, y, width, height)
    } else {
        let x = axis_position_anchored(&style.left, &style.right, cb.x, cb.width, width);
        let y = axis_position_anchored(&style.top, &style.bottom, cb.y, cb.height, height);
        LayoutRect::new(x, y, width, height)
    }
}
