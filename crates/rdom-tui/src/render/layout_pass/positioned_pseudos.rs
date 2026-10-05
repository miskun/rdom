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
//! - **Size** follows CSS 2.1 §10.3.7 / §10.6.4 as for a positioned
//!   element (`positioning::resolve_size_axis`): a declared `width` /
//!   `height` (cells, a percentage of the containing block, `calc()`)
//!   is the size; `auto` spans between both insets when both are set,
//!   else is the `content`'s size (UAX#11 `UnicodeWidthStr`, one row
//!   per line). A relative pseudo's insets only shift it (§9.4.3).

use rdom_core::{Dom, NodeId, NodeType};
use unicode_width::UnicodeWidthStr;

use crate::ext::{PseudoLayout, TuiExt};
use crate::layout::{Display, LayoutRect, Position, clamp_size};
use crate::render::layout_pass::box_sizing::Sizer;
use crate::style::ComputedStyle;

use super::positioning::{
    axis_position_anchored, computed_position, padding_box, parent_id, relative_offset,
    resolve_size_axis,
};
use crate::layout::Length;

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
            ext.computed_before.as_deref().cloned(),
            ext.computed_after.as_deref().cloned(),
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
            // CSS 2.1 §10.1: the padding edge of the positioned box.
            if matches!(
                host_position,
                Position::Relative | Position::Absolute | Position::Fixed
            ) {
                return padding_box(dom, host).unwrap_or(host_rect);
            }
            let mut cur = parent_id(dom, host);
            while let Some(p) = cur {
                let pp = computed_position(dom, p);
                if matches!(
                    pp,
                    Position::Relative | Position::Absolute | Position::Fixed
                ) {
                    return padding_box(dom, p).unwrap_or(viewport);
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
/// resolve via [`resolve_size_axis`] with the pseudo's intrinsic
/// content size as the shrink-to-fit size. Position then routes through
/// [`axis_position_anchored`] for absolute/fixed, or [`relative_offset`]
/// from the host's edge for `Position::Relative` (see module docs for the natural-position
/// divergence from CSS).
fn compute_placed_rect(
    style: &ComputedStyle,
    cb: LayoutRect,
    host_rect: LayoutRect,
    end: PseudoEnd,
) -> LayoutRect {
    let intrinsic_w = pseudo_content_width(style);
    let intrinsic_h = pseudo_content_height(style);

    // CSS 2.1 §10.3.7 / §10.6.4, as for a positioned element: the
    // declared size, else the span between both insets, else the
    // content's size. A relative box only shifts (§9.4.3): its insets
    // never size it.
    let relative = style.position == Position::Relative;
    let insets = |start: &Length, end: &Length| -> (Length, Length) {
        if relative {
            (Length::Auto, Length::Auto)
        } else {
            (start.clone(), end.clone())
        }
    };
    let (left, right) = insets(&style.left, &style.right);
    let (top, bottom) = insets(&style.top, &style.bottom);
    let width = resolve_size_axis(
        &style.width,
        Sizer::horizontal(style, cb.width),
        cb.width,
        &left,
        &right,
        cb.width,
        // A pseudo-element's content is one string: every keyword is its
        // width, as the shrink-to-fit size is.
        |_, _| intrinsic_w,
    );
    let height = resolve_size_axis(
        &style.height,
        Sizer::vertical(style, cb.width),
        cb.height,
        &top,
        &bottom,
        cb.height,
        |_, _| intrinsic_h,
    );
    // CSS 2.1 §10.4 / §10.7: clamped by `max-*`, then `min-*`; a keyword
    // bound is the content's size, as a keyword size is.
    let bound = |sizer: Sizer, cells: Option<u16>, keyword: bool, content: u16| {
        if keyword {
            Some(content)
        } else {
            sizer.outer_opt(cells)
        }
    };
    let (hs, vs) = (
        Sizer::horizontal(style, cb.width),
        Sizer::vertical(style, cb.width),
    );
    let basis_w = Some(cb.width);
    let basis_h = Some(cb.height);
    let width = hs.floor(clamp_size(
        width,
        bound(
            hs,
            style.min_width.cells(basis_w),
            style.min_width.intrinsic().is_some(),
            intrinsic_w,
        ),
        bound(
            hs,
            style.max_width.cells(basis_w),
            style.max_width.intrinsic().is_some(),
            intrinsic_w,
        ),
    ));
    let height = vs.floor(clamp_size(
        height,
        bound(
            vs,
            style.min_height.cells(basis_h),
            style.min_height.intrinsic().is_some(),
            intrinsic_h,
        ),
        bound(
            vs,
            style.max_height.cells(basis_h),
            style.max_height.intrinsic().is_some(),
            intrinsic_h,
        ),
    ));

    if relative {
        // Relative pseudo: natural anchor is the host's start edge
        // for `::before`, host's far edge - intrinsic_w for `::after`.
        // The insets then shift it from there, as they shift a relative
        // element (`relative_offset`: the inline-start inset wins).
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
        let (dx, dy) = relative_offset(style, cb, true);
        LayoutRect::new(
            natural_x.saturating_add(dx),
            natural_y.saturating_add(dy),
            width,
            height,
        )
    } else {
        let x = axis_position_anchored(&style.left, &style.right, cb.x, cb.width, width);
        let y = axis_position_anchored(&style.top, &style.bottom, cb.y, cb.height, height);
        LayoutRect::new(x, y, width, height)
    }
}

#[cfg(test)]
#[path = "positioned_pseudos_tests.rs"]
mod tests;
