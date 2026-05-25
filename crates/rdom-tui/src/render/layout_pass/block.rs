//! Block layout pass — CSS 2.1 §10 normal flow.
//!
//! Given a block container (`flow: Block`) and its in-flow children,
//! stacks the children vertically in document order at their natural
//! heights. No distribution, no shrink-to-fit; container overflows
//! below its content box if children don't fit.
//!
//! Width resolution follows CSS 2.1 §10.3.3 — the seven-term sum
//! `margin-left + border-left + padding-left + width + padding-right
//! + border-right + margin-right` must equal the containing-block
//! width. Auto margins absorb leftover horizontal space (the
//! `margin: 0 auto` centering pattern).
//!
//! Height: each child takes its declared `height` (`Fixed`), its
//! resolved percentage (`Percent`), or its intrinsic content height
//! (`Auto`). Min/max clamping applies after computing the size.
//!
//! **Scope (BFC-1 through phase 4):**
//! - Width formula + auto margins + min/max clamp (phase 2).
//! - Plain vertical stacking (no margin collapse — phase 5).
//! - Anonymous box generation around inline-level children (phase 3),
//!   including atomic inline-block packing (phase 3.5b).
//! - Live dispatch from `layout_children` via cascaded `Flow::Block`
//!   (phase 4.1); border-collapse parent-edge inset + scroll cursor
//!   offset mirror flex behavior so the two modes agree.
//! - Strict percent-height-needs-definite-parent — phase 6 will
//!   tighten this; for now percent resolves against the container.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{
    Direction, LayoutRect, MarginValue, Size, clamp_size, compute_content_area_collapsed,
};
use crate::node::TuiNodeExt;
use crate::render::inline::compute_inline_layout_for_run;
use crate::render::layout_pass::intrinsic::intrinsic_size;
use crate::style::ComputedStyle;

use super::layout_node;

/// Lay out `id`'s in-flow children per CSS 2.1 §10. Partitions
/// children into runs of consecutive block-level vs inline-level
/// nodes; block runs get individual block layout; inline runs
/// fold into **anonymous block boxes** (CSS 2.1 §9.2.1.1) that
/// each establish their own IFC.
///
/// Stores anonymous boxes on the parent's `TuiExt.anonymous_blocks`
/// — paint / hit-test / selection iterate this Vec alongside the
/// singular `inline_layout` field.
pub(super) fn layout_block_children(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    container: LayoutRect,
    parent_computed: &ComputedStyle,
) {
    // Collect ALL direct child nodes (text + element). Block layout
    // distinguishes inline-level (text + Display::Inline/InlineBlock
    // elements) from block-level (Display::Block elements) — text
    // nodes are inline-level participants in an anonymous block per
    // CSS 2.1 §9.2.1.1 rule 2.
    let raw_children: Vec<NodeId> = dom.node(id).child_nodes().map(|c| c.id()).collect();
    if raw_children.is_empty() {
        return;
    }

    // Filter out-of-flow elements; text nodes are always in flow.
    // The `child_range` indices below are into THIS filtered list.
    let in_flow: Vec<(usize, NodeId)> = raw_children
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, c)| is_in_flow(dom, *c))
        .collect();
    if in_flow.is_empty() {
        // Clear any stale anonymous boxes from a previous layout —
        // matches flex's `ext.inline_layout = None` reset.
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.anonymous_blocks.clear();
        }
        return;
    }

    // Partition into runs. A run is a contiguous sequence of in-flow
    // children that share a level (block or inline). When the level
    // flips, the run closes and a new one opens. Comments and
    // fragments are treated as inline-level (no effect on layout
    // beyond breaking adjacency).
    let mut runs: Vec<Run> = Vec::new();
    for (orig_idx, child_id) in &in_flow {
        let kind = child_level(dom, *child_id);
        match runs.last_mut() {
            Some(last) if last.kind == kind => {
                last.children.push(*child_id);
                last.child_range.1 = orig_idx + 1;
            }
            _ => runs.push(Run {
                kind,
                children: vec![*child_id],
                child_range: (*orig_idx, orig_idx + 1),
            }),
        }
    }

    // Parent-child border-collapse inset (CSS 2.1 §17.6.3 +
    // BFC-1 invariant): when this container is `border-collapse:
    // collapse` with its own border, `layout_node` already expanded
    // its content area to extend into the border ring. That's
    // correct ONLY when the first/last child has its own border to
    // share the cell with. Content-bearing children (no border)
    // would land on the parent's painted border row. Apply the
    // same per-edge inset flex uses so the two layout modes agree.
    let in_flow_ids: Vec<NodeId> = in_flow.iter().map(|(_, id)| *id).collect();
    let (top_inset, bot_inset, left_inset, right_inset) =
        super::flex::collapse_parent_edge_insets(dom, &in_flow_ids, parent_computed);
    let container = LayoutRect::new(
        container.x + left_inset as i32,
        container.y + top_inset as i32,
        container.width.saturating_sub(left_inset + right_inset),
        container.height.saturating_sub(top_inset + bot_inset),
    );

    let containing_block_width = container.width;
    // Apply this container's scroll_y to the starting cursor (mirrors
    // `flex::layout_flex_children`'s `container.y - scroll_main`). The
    // scroll itself lives on the parent's `ext.scroll_y`; the flex pass
    // reads it via the *children's* `parent_scroll` helper, which we
    // reuse here so block and flex agree on the offset.
    let scroll_y = super::parent_scroll(dom, &in_flow_ids, crate::layout::Direction::Column);
    let mut y_cursor: i32 = container.y - scroll_y;
    let mut anon_blocks: Vec<AnonymousIfc> = Vec::new();

    for run in &runs {
        match run.kind {
            RunKind::Block => {
                for &child in &run.children {
                    y_cursor = lay_out_block_child(
                        dom,
                        child,
                        container,
                        containing_block_width,
                        y_cursor,
                    );
                }
            }
            RunKind::Inline => {
                // Anonymous block box wrapping this inline run. Its
                // IFC packs the run's children at the container's
                // content width. Height = packed line count.
                let inline_layout =
                    compute_inline_layout_for_run(dom, id, &run.children, containing_block_width);
                let height = inline_layout.height();
                let rect = LayoutRect::new(container.x, y_cursor, containing_block_width, height);
                // Layout atomic inline-block children at their
                // fragment rects. This both writes their layout
                // rects (so hit-test descends into them — e.g.
                // `<form><button>Go</button></form>` button clicks
                // route to the button, not the form) and recurses
                // into their subtrees (so `<button>`'s own inner
                // text-only layout, pseudos, etc. get computed).
                layout_atomic_inline_blocks(dom, &inline_layout, rect);
                anon_blocks.push(AnonymousIfc {
                    rect,
                    inline_layout,
                    child_range: run.child_range,
                });
                y_cursor += height as i32;
            }
        }
    }

    // Write anon boxes to the parent. Empty Vec is the normal state
    // for pure-block containers — clears any stale entries from a
    // previous layout where the tree may have had different shape.
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.anonymous_blocks = anon_blocks;
    }
}

/// Recurse layout into atomic inline-block fragments — write each
/// atom's layout rect (so hit-test descends) and call `layout_node`
/// to lay out the atom's own subtree (its own inner inline_layout,
/// pseudo positioning, descendants).
///
/// `anon_rect` is the anonymous block box's rect (or the singular
/// IFC's content rect). Fragment x is offset from `anon_rect.x`;
/// fragment line index gives the y row.
fn layout_atomic_inline_blocks(
    dom: &mut Dom<TuiExt>,
    inline_layout: &crate::render::inline::InlineLayout,
    anon_rect: LayoutRect,
) {
    // Snapshot the atomic-fragment placements first — we can't
    // mutate-borrow `dom` while iterating an `&InlineLayout`
    // borrowed from it.
    let mut atoms: Vec<(NodeId, LayoutRect)> = Vec::new();
    for (line_idx, line) in inline_layout.lines.iter().enumerate() {
        let line_y = anon_rect.y + line_idx as i32;
        for fragment in &line.fragments {
            if !fragment.atomic {
                continue;
            }
            let atom_rect =
                LayoutRect::new(anon_rect.x + fragment.x as i32, line_y, fragment.width, 1);
            atoms.push((fragment.node, atom_rect));
        }
    }
    for (id, rect) in atoms {
        layout_node(dom, id, rect);
    }
}

/// Lay out a single block-level child. Returns the new y cursor.
fn lay_out_block_child(
    dom: &mut Dom<TuiExt>,
    child: NodeId,
    container: LayoutRect,
    containing_block_width: u16,
    y_cursor: i32,
) -> i32 {
    let computed = dom
        .node(child)
        .computed()
        .cloned()
        .unwrap_or_else(ComputedStyle::initial);

    let resolved = resolve_block_width(&computed, containing_block_width);
    let height = resolve_block_height(dom, child, &computed, resolved.width, container.height);

    let top_margin = vertical_margin(&computed.margin.top);
    let bottom_margin = vertical_margin(&computed.margin.bottom);

    let outer_x = container.x + resolved.margin_left as i32;
    let outer_y = y_cursor + top_margin as i32;
    let outer_rect = LayoutRect::new(outer_x, outer_y, resolved.width, height);

    layout_node(dom, child, outer_rect);

    outer_rect.bottom() + bottom_margin as i32
}

/// One run of consecutive children sharing a level (block-level or
/// inline-level). Block runs get per-child block layout; inline
/// runs fold into one anonymous block per CSS 2.1 §9.2.1.1.
struct Run {
    kind: RunKind,
    /// Direct-child NodeIds in document order.
    children: Vec<NodeId>,
    /// Indices into the parent's raw `child_nodes()` order, as
    /// `[start, end)`. Stored on the resulting `AnonymousIfc` so
    /// paint / hit-test can map back to surrounding context.
    child_range: (usize, usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunKind {
    Block,
    Inline,
}

/// Classify a direct child as block-level vs inline-level. Text
/// nodes are always inline-level; element children depend on their
/// `Display`. Per CSS 2.1 §9.2: only `Block` elements are
/// block-level; `Inline` and `InlineBlock` are inline-level (the
/// inline-block participates in IFC as an atomic box per phase
/// 3.5's planned inline-block-in-IFC packing).
fn child_level(dom: &Dom<TuiExt>, id: NodeId) -> RunKind {
    let node = dom.node(id);
    match node.node_type() {
        NodeType::Text => RunKind::Inline,
        NodeType::Element => {
            let display = node
                .ext()
                .and_then(|e| e.computed.as_ref())
                .map(|c| c.display)
                .unwrap_or(crate::layout::Display::Block);
            match display {
                crate::layout::Display::Inline | crate::layout::Display::InlineBlock => {
                    RunKind::Inline
                }
                crate::layout::Display::Block | crate::layout::Display::None => RunKind::Block,
            }
        }
        // Comments, fragments — treat as inline-level (effectively
        // invisible; they don't break runs).
        _ => RunKind::Inline,
    }
}

/// True iff the node participates in normal flow. Text nodes are
/// always in flow; element children are in flow when not
/// `display: none` and not absolutely positioned.
fn is_in_flow(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    if node.node_type() != NodeType::Element {
        return true; // text, comments, fragments
    }
    let Some(c) = node.ext().and_then(|e| e.computed.as_ref()) else {
        return true;
    };
    use crate::layout::{Display, Position};
    c.display != Display::None && !matches!(c.position, Position::Absolute | Position::Fixed)
}

/// Result of CSS 2.1 §10.3.3 width resolution for a single block
/// child: the resolved `margin-left`, `width`, and `margin-right`
/// in cells, with auto values resolved and over-constrained cases
/// normalized.
#[derive(Debug, Clone, Copy)]
struct ResolvedWidth {
    margin_left: i16,
    width: u16,
    #[allow(dead_code)] // phase 2: cursor doesn't use the right margin
    // because horizontal block placement is left-anchored; phase 5
    // (margin collapse) doesn't touch horizontal margins either.
    // Kept on the struct for symmetry + future use (right-anchored
    // direction support).
    margin_right: i16,
}

/// CSS 2.1 §10.3.3 — "Block-level, non-replaced elements in normal
/// flow." Resolves the width-and-horizontal-margin equation:
///
/// ```text
///   ML + W_outer + MR = CB
/// ```
///
/// where `W_outer` is the child's border-box width (the
/// `Size::Fixed(N)` value used here, NOT the CSS-strict content
/// width — rdom stores outer rects in `LayoutRect`, matching the
/// flex layout pass's convention). `CB` is the containing block's
/// content width.
///
/// Auto values absorb leftover space; over-constrained widths
/// (LTR) override `margin-right` to make the equation balance.
///
/// **Divergence note.** CSS 2.1 strict defines `width` as the
/// content-box size, with padding + border added on top. rdom
/// follows the flex pass's `width = outer` convention so authors
/// see one definition of `width` across both layout modes.
/// Documented in `DIVERGENCES.md` under "Values" — `box-sizing:
/// border-box` is the implicit default.
fn resolve_block_width(computed: &ComputedStyle, containing_block_width: u16) -> ResolvedWidth {
    let cb = containing_block_width as i32;

    // Declared width and margins in their raw forms.
    let width_decl = &computed.width;
    let ml_decl = computed.margin.left;
    let mr_decl = computed.margin.right;

    // Resolve the declared width to a concrete cell count when
    // possible. `Auto` stays "needs computation" — we drive it
    // from the leftover after margins.
    let declared_width: Option<i32> = resolve_size_to_cells(width_decl, cb);

    let ml_auto = matches!(ml_decl, MarginValue::Auto);
    let mr_auto = matches!(mr_decl, MarginValue::Auto);
    let ml_cells = match ml_decl {
        MarginValue::Auto => 0i32,
        MarginValue::Cells(n) => n as i32,
    };
    let mr_cells = match mr_decl {
        MarginValue::Auto => 0i32,
        MarginValue::Cells(n) => n as i32,
    };

    let (ml_final, width_final, mr_final): (i32, i32, i32) =
        match (declared_width, ml_auto, mr_auto) {
            // Width auto — any auto margins resolve to 0; width absorbs
            // leftover. (Note: width here is outer/border-box, NOT
            // CSS-strict content width.)
            (None, _, _) => {
                let w = cb - ml_cells - mr_cells;
                (ml_cells, w.max(0), mr_cells)
            }
            // Width fixed, both margins auto → center.
            (Some(w), true, true) => {
                let leftover = cb - w;
                let half = leftover.div_euclid(2);
                // The odd cell goes to the right margin — matches the
                // common browser behavior for odd-leftover centering.
                (half, w, leftover - half)
            }
            // Width fixed, only ML auto → ML absorbs leftover.
            (Some(w), true, false) => {
                let ml = cb - w - mr_cells;
                (ml, w, mr_cells)
            }
            // Width fixed, only MR auto → MR absorbs leftover.
            (Some(w), false, true) => {
                let mr = cb - w - ml_cells;
                (ml_cells, w, mr)
            }
            // Over-constrained (LTR): the declared MR is silently
            // overridden so the equation balances.
            (Some(w), false, false) => {
                let mr = cb - w - ml_cells;
                (ml_cells, w, mr)
            }
        };

    // Apply min/max-width clamp. CSS 2.1 §10.4: clamp the resolved
    // width by max-width first, then min-width (min wins over max).
    // After clamping, if the width changed, re-distribute the
    // leftover to whichever margins were auto.
    let clamped_width = clamp_width(width_final, &computed.min_width, computed.max_width, cb);
    let (ml_clamped, mr_clamped) = if clamped_width != width_final {
        let leftover = cb - clamped_width;
        match (ml_auto, mr_auto) {
            (true, true) => {
                let half = leftover.div_euclid(2);
                (half, leftover - half)
            }
            (true, false) => (leftover - mr_cells, mr_cells),
            (false, true) => (ml_cells, leftover - ml_cells),
            (false, false) => (ml_cells, leftover - ml_cells),
        }
    } else {
        (ml_final, mr_final)
    };

    ResolvedWidth {
        margin_left: ml_clamped.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        width: clamped_width.max(0).min(u16::MAX as i32) as u16,
        margin_right: mr_clamped.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
    }
}

/// Resolve a `Size` to a definite cell count when possible. Returns
/// `None` for `Size::Auto` (caller chooses the fallback). `Flex`
/// in block context is treated as `Auto` — flex factors only mean
/// something inside a flex container, and the spec says block-level
/// flex children resolve their main size from `flex-basis` (which
/// for the `flex: <N>` shorthand is 0%).
fn resolve_size_to_cells(size: &Size, basis: i32) -> Option<i32> {
    match size {
        Size::Auto | Size::Flex(_) => None,
        Size::Fixed(n) => Some(*n as i32),
        Size::Percent(p) => Some((basis * *p as i32) / 100),
        Size::Calc(expr) => {
            let v = expr.resolve(&rdom_style::calc::ResolveCtx::new(basis));
            Some(v)
        }
    }
}

fn clamp_width(
    width: i32,
    min: &Option<crate::layout::MinSize>,
    max: Option<u16>,
    _basis: i32, // reserved for percent-min/max in a later phase
) -> i32 {
    let min_cells: Option<i32> = match min {
        Some(crate::layout::MinSize::Cells(n)) => Some(*n as i32),
        Some(crate::layout::MinSize::Auto) | None => None, // phase 2: Auto floors are spec-correctly 0 for block; phase 5/6 may revisit
    };
    let max_cells = max.map(|n| n as i32);
    let after_max = match max_cells {
        Some(m) => width.min(m),
        None => width,
    };
    match min_cells {
        Some(m) => after_max.max(m),
        None => after_max.max(0),
    }
}

/// CSS 2.1 §10.6.3 — block-level non-replaced element height. For
/// phase 2 this is the simple version: `Auto` → intrinsic content
/// height; `Fixed` → declared; `Percent` → percent of container
/// (the "percent needs definite parent" rule lands in phase 6).
///
/// Two separate budgets:
/// - `resolved_width` is the child's content width — fed to
///   `intrinsic_size` as the cross-axis budget so descendant text
///   wraps against the box that will hold it.
/// - `container_height` is the containing-block's content height —
///   used as the base for `height: <pct>%` resolution and for
///   `calc()` with percent terms.
fn resolve_block_height(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    resolved_width: u16,
    container_height: u16,
) -> u16 {
    let raw = match &computed.height {
        Size::Auto | Size::Flex(_) => {
            // Intrinsic content height — walk the child's subtree.
            // Block items aren't "flex items" in this pass; Flex
            // here means the shorthand was used in a non-flex
            // context, treated as Auto.
            //
            // cross_budget passed to intrinsic_size = the WIDTH
            // descendants will be laid out into (Direction::Column
            // queries height; cross axis is row/width). That's the
            // child's own resolved width — text wraps to it.
            intrinsic_size(dom, id, Direction::Column, resolved_width)
        }
        Size::Fixed(n) => *n,
        Size::Percent(p) => {
            ((container_height as u32 * *p as u32) / 100).min(u16::MAX as u32) as u16
        }
        Size::Calc(expr) => {
            let v = expr.resolve(&rdom_style::calc::ResolveCtx::new(container_height as i32));
            v.max(0).min(u16::MAX as i32) as u16
        }
    };

    // Clamp by min-height / max-height. Min:auto on block elements
    // resolves to 0 per CSS 2.1 (block boxes have no content-min
    // floor — that's a flex-only concept from Flexbox §4.5).
    let min_cells: Option<u16> = match computed.min_height {
        Some(crate::layout::MinSize::Cells(n)) => Some(n),
        Some(crate::layout::MinSize::Auto) | None => None,
    };
    clamp_size(raw, min_cells, computed.max_height)
}

/// Convert a `MarginValue` to its effective cell contribution on
/// the BLOCK axis (top/bottom). `Auto` on the block axis collapses
/// to 0 per CSS 2.1 §8.3 — only inline-axis auto margins absorb
/// leftover space; block-axis autos don't.
fn vertical_margin(m: &MarginValue) -> i16 {
    match m {
        MarginValue::Auto => 0,
        MarginValue::Cells(n) => *n,
    }
}

/// Convenience helper for `compute_content_area_collapsed` style
/// access — currently unused in this module but retained for
/// symmetry with `flex.rs` and to give phase 5 a single import.
#[allow(dead_code)]
fn content_area(outer: LayoutRect, computed: &ComputedStyle) -> LayoutRect {
    compute_content_area_collapsed(
        outer,
        computed.padding,
        computed.border,
        computed.border_collapse,
    )
}
