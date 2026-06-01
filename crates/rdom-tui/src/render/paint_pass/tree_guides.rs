//! Tree guide-line paint pass — draws the `│ ├ └` connectors for
//! the ARIA tree pattern (`<ul role=tree>` / `<li role=treeitem>` /
//! `<ul role=group>`).
//!
//! ## Why this reuses the border substrate
//!
//! Tree guides ARE box-drawing lines, so rather than hand-roll
//! glyph selection this pass emits per-direction
//! [`BorderContribution`]s into `buf.border_dirs` exactly like
//! [`super::border::paint_border`] does, then lets
//! [`super::border_join::join_borders`] turn the 4-direction masks
//! into the right glyph (`├` = N+E+S, `└` = N+E, `│` = N+S, `─` =
//! E+W) via its `SOLID_TABLE`. This pass therefore runs AFTER the
//! main paint walk and BEFORE the joiner (see `paint_dom`).
//!
//! ## Geometry (2 cells per level)
//!
//! Each nesting level is 2 cells. The indent comes from the
//! treeitem's own `padding-left: TREE_INDENT` (the `▼ `/`▶ ` arrow
//! field) — `[role=group]` adds none. So an item's box sits 2 cells
//! right of its parent's box, and its connector is painted in the
//! cell 2 to the LEFT of its box (`item.layout.x - TREE_INDENT`),
//! which lands directly under the parent item's arrow column.
//! Per row: `[ancestor trunks `│ `][connector `├ `/`└ `][arrow
//! `▼ `/`▶ ` or 2 blanks][label]`. Ancestor trunks (`│`) are drawn
//! at each ancestor's connector column when that ancestor has a
//! following sibling — "does the line continue past me" falls out
//! of the sibling list, no per-row computed state.
//!
//! Direct children of `[role=tree]` are depth 0: arrow + label
//! only, no connector/trunk. Connectors begin one level in.
//!
//! This pass reads layout rects + DOM structure only — it never
//! recomputes layout or cascade. Glyph choice stays in the generic
//! joiner. Mirrors the unguarded full-tree collection that
//! `paint_modal_backdrops` already does for dialogs.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{CornerStyle, Display};
use crate::node::TuiNodeExt;
use crate::render::buffer::{BorderContribution, BorderSide, DIR_E, DIR_N, DIR_S};
use crate::render::{Buffer, Rect, Style};
use crate::style::Color;
use rdom_style::layout::BorderStyle;

/// Cells of indent per nesting level. MUST match the
/// `[role=treeitem] { padding-left: N }` value in the UA stylesheet
/// (`rdom-style/src/ua.rs`) — that padding is the per-level step,
/// and a child's connector is painted `TREE_INDENT` cells left of
/// its box.
const TREE_INDENT: i32 = 2;

/// Entry point — paint guides for every `[role=tree]` in the
/// document. Called from `paint_dom` between the main walk and the
/// border joiner.
pub(super) fn paint_tree_guides(dom: &Dom<TuiExt>, buf: &mut Buffer, clip: Rect) {
    let mut trees = Vec::new();
    collect_trees(dom, dom.root(), &mut trees);
    for tree in trees {
        // The guide pass is a standalone walk, so — unlike `paint_node`
        // — it doesn't inherit the per-`overflow` clip a scroll
        // container imposes on its descendants. Re-derive it: narrow
        // the viewport clip to the scrollport of every clipping
        // ancestor. Without this a tree taller than an `overflow:
        // auto` ancestor paints its `│` trunk straight through the
        // container's bottom edge (see the regression test).
        let tree_clip = clip_for_tree(dom, tree, clip);
        // Full-width span of the tree's content box — row highlights
        // fill from here to the right edge so the selected/cursor bg
        // runs under the guide gutter, not just the indented box.
        let span = dom
            .node(tree)
            .content_layout_rect()
            .map(|r| (r.x, r.x + r.width as i32))
            .unwrap_or((tree_clip.x as i32, tree_clip.right() as i32));
        for item in treeitem_children(dom, tree) {
            paint_item(dom, item, buf, tree_clip, &[], span);
        }
    }
}

/// Narrow `base_clip` to the scrollport (padding-box, per CSS
/// Overflow 3 §3) of every clipping ancestor of `tree`, including the
/// tree itself. Mirrors `paint_node`'s `children_clip` rule so guides
/// painted by this standalone pass respect the same overflow clipping
/// the main walk applies to a scroll container's descendants.
fn clip_for_tree(dom: &Dom<TuiExt>, tree: NodeId, base_clip: Rect) -> Rect {
    use crate::layout::Overflow;
    let mut clip = base_clip;
    let mut cur = Some(tree);
    while let Some(id) = cur {
        if let Some(computed) = dom.node(id).ext().and_then(|e| e.computed.as_ref()) {
            let clips = !matches!(computed.overflow_x, Overflow::Visible)
                || !matches!(computed.overflow_y, Overflow::Visible);
            if clips && let Some(outer) = dom.node(id).layout_rect() {
                let padding_box = rdom_style::layout::compute_padding_box(outer, computed.border);
                clip = match super::layout_rect_to_grid(padding_box, clip) {
                    Some(grid) => clip.intersection(grid),
                    None => return Rect::new(clip.x, clip.y, 0, 0),
                };
            }
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    clip
}

fn collect_trees(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    if role(dom, id) == Some("tree") {
        out.push(id);
    }
    for child in dom.node(id).child_nodes() {
        collect_trees(dom, child.id(), out);
    }
}

/// Paint one treeitem's guides and recurse into its child group.
/// `trunks` is the stack of `(gutter_column, continues)` for every
/// ancestor level that has a connector — a `│` is drawn at each
/// column whose ancestor has a following sibling.
fn paint_item(
    dom: &Dom<TuiExt>,
    item: NodeId,
    buf: &mut Buffer,
    clip: Rect,
    trunks: &[(u16, bool)],
    span: (i32, i32),
) {
    if is_hidden(dom, item) {
        return;
    }
    let Some(rect) = dom.node(item).layout_rect() else {
        return;
    };
    let row_y = rect.y;
    let color = guide_color(dom, item);

    // The item's own label can wrap to multiple rows. Guides span the
    // LABEL rows only — never the child group, whose rows the
    // recursion handles via the ancestor-trunk stack. `label_height`
    // is the gap between this item's top and its (visible) child
    // group's top, or the full item height when there's no group
    // (e.g. a wrapped leaf). Without spanning these rows the `│`
    // trunk breaks wherever a sibling's label wrapped.
    let label_height = match child_group(dom, item) {
        Some(g) if !is_hidden(dom, g) => dom
            .node(g)
            .layout_rect()
            .map(|gr| (gr.y - rect.y).max(1))
            .unwrap_or(rect.height as i32),
        _ => (rect.height as i32).max(1),
    };

    // Row-background highlight. The cascade sets a non-`Reset` `bg`
    // only on the selected (`aria-selected`) / cursor
    // (`[role=tree]:focus [data-rdom-active]`) row, so reading
    // `computed.bg` covers both — and the cursor case is already
    // focus-gated by the selector. Fill the FULL tree-width row
    // (set `bg` only, preserving the label glyphs painted in the
    // main walk and the guide glyphs the joiner draws after). Fill
    // every label row so a wrapped cursor/selected row highlights
    // fully, not just its first line.
    if let Some(bg) = row_highlight(dom, item) {
        for dy in 0..label_height {
            fill_row_bg(buf, clip, span, row_y + dy, bg);
        }
    }

    // Expand/collapse arrow in the treeitem's reserved arrow field
    // (`padding-left: 2` ⇒ label starts at `rect.x + 2`, arrow at
    // `rect.x`). Branches only — a branch is any item with an
    // `aria-expanded` attribute (presence, not child count, so an
    // unloaded lazy branch still shows an arrow). Full-cell `▼`/`▶`,
    // painted in the guide color. Painted here rather than via
    // `::before` to dodge the mixed-content pseudo gap
    // (TREE-BFC-PSEUDO-1). First label row only.
    if let Some(expanded) = dom.node(item).get_attribute("aria-expanded") {
        let glyph = if expanded == "true" { "▼" } else { "▶" };
        put_glyph(buf, clip, rect.x, row_y, glyph, color);
    }

    // Ancestor trunks — `│` at each continuing ancestor's column,
    // drawn across EVERY label row so the trunk doesn't break where
    // this item's label wrapped.
    for &(col, continues) in trunks {
        if continues {
            for dy in 0..label_height {
                put(buf, clip, col as i32, row_y + dy, &[DIR_N, DIR_S], color);
            }
        }
    }

    // Own connector — only for items inside a group (depth >= 1).
    let parent = dom.node(item).parent_node().map(|p| p.id());
    let in_group = parent
        .map(|p| role(dom, p) == Some("group"))
        .unwrap_or(false);
    let own_col = rect.x - TREE_INDENT;
    let mut is_last = false;
    if in_group && let Some(p) = parent {
        is_last = treeitem_children(dom, p).last() == Some(&item);
        // `├` (N+E+S) when a sibling follows, `└` (N+E) when last.
        // The E stub is the connector glyph's own right tick; the
        // next cell stays blank — the connector is `├ ` / `└ `, no dash.
        let connector: &[usize] = if is_last {
            &[DIR_N, DIR_E]
        } else {
            &[DIR_N, DIR_E, DIR_S]
        };
        put(buf, clip, own_col, row_y, connector, color);
        // Continue the `│` down through any wrapped label rows so a
        // non-last item's trunk reaches its child group / next sibling.
        if !is_last {
            for dy in 1..label_height {
                put(buf, clip, own_col, row_y + dy, &[DIR_N, DIR_S], color);
            }
        }
    }

    // Recurse into the child group, extending the trunk stack with
    // this item's column (continues iff this item isn't the last
    // sibling).
    if let Some(group) = child_group(dom, item)
        && !is_hidden(dom, group)
    {
        let mut child_trunks = trunks.to_vec();
        if in_group && own_col >= 0 {
            child_trunks.push((own_col as u16, !is_last));
        }
        for child in treeitem_children(dom, group) {
            paint_item(dom, child, buf, clip, &child_trunks, span);
        }
    }
}

/// Write `dirs` as `Solid` border contributions at `(x, y)`. The
/// joiner turns the accumulated 4-direction mask into the glyph.
fn put(buf: &mut Buffer, clip: Rect, x: i32, y: i32, dirs: &[usize], color: Color) {
    if x < 0 || y < 0 {
        return;
    }
    let (xu, yu) = (x as u16, y as u16);
    if !clip.contains(xu, yu) {
        return;
    }
    for &dir in dirs {
        buf.add_border_dir(
            xu,
            yu,
            dir,
            BorderContribution {
                style: BorderStyle::Solid,
                fg: color,
                priority: 0,
                corner_style: CornerStyle::Square,
                side: BorderSide::Top,
            },
        );
    }
}

/// The row-highlight background for `item`, or `None` when the row
/// isn't highlighted. Sourced from the cascaded `bg` (selected /
/// focus-gated cursor — see the UA `[role=…]` rules).
fn row_highlight(dom: &Dom<TuiExt>, item: NodeId) -> Option<Color> {
    let bg = dom.node(item).computed().map(|c| c.bg)?;
    (bg != Color::Reset).then_some(bg)
}

/// Set `bg` on every cell of `[left, right)` at row `y` (clipped),
/// preserving each cell's symbol/fg — so the highlight runs under
/// the label and the guide glyphs without erasing them.
fn fill_row_bg(buf: &mut Buffer, clip: Rect, span: (i32, i32), y: i32, bg: Color) {
    if y < 0 {
        return;
    }
    let yu = y as u16;
    if yu < clip.y || yu >= clip.bottom() {
        return;
    }
    let x0 = span.0.max(clip.x as i32).max(0) as u16;
    let x1 = span.1.min(clip.right() as i32).max(0) as u16;
    for x in x0..x1 {
        if let Some(cell) = buf.cell_mut(x, yu) {
            cell.set_bg(bg);
        }
    }
}

/// Write a single glyph at `(x, y)` if it falls inside `clip`.
fn put_glyph(buf: &mut Buffer, clip: Rect, x: i32, y: i32, glyph: &str, fg: Color) {
    if x < 0 || y < 0 {
        return;
    }
    let (xu, yu) = (x as u16, y as u16);
    if !clip.contains(xu, yu) {
        return;
    }
    buf.set_stringn(xu, yu, glyph, 1, Style::new().fg(fg));
}

// ── DOM helpers (structure reads only) ──────────────────────────

fn role(dom: &Dom<TuiExt>, id: NodeId) -> Option<&str> {
    dom.node(id).get_attribute("role")
}

fn is_hidden(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    dom.node(id)
        .computed()
        .map(|c| c.display == Display::None)
        .unwrap_or(false)
}

fn guide_color(dom: &Dom<TuiExt>, id: NodeId) -> Color {
    dom.node(id)
        .computed()
        .map(|c| c.border_fg)
        .unwrap_or(Color::Reset)
}

/// Direct element children of `container` with `role=treeitem`.
fn treeitem_children(dom: &Dom<TuiExt>, container: NodeId) -> Vec<NodeId> {
    dom.node(container)
        .child_nodes()
        .filter(|n| n.get_attribute("role") == Some("treeitem"))
        .map(|n| n.id())
        .collect()
}

/// First direct child of `item` with `role=group`.
fn child_group(dom: &Dom<TuiExt>, item: NodeId) -> Option<NodeId> {
    dom.node(item)
        .child_nodes()
        .find(|n| n.get_attribute("role") == Some("group"))
        .map(|n| n.id())
}
