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
//! ## Geometry
//!
//! Indentation is owned by the `[role=group]` UA rule
//! (`padding-left: TREE_INDENT`). A group's child treeitems lay out
//! at `group.layout.x + TREE_INDENT`, so an item's own connector
//! lives in its parent group's padding gutter at `item.layout.x -
//! TREE_INDENT` (= the group's left edge), with a `─` lead-in stub
//! in the next cell. Ancestor trunks (`│`) are drawn at each
//! ancestor group's gutter column when that ancestor has a
//! following sibling — "does the line continue past me" falls out
//! of the sibling list, no per-row computed state.
//!
//! Direct children of `[role=tree]` are depth 0: chevron + label
//! only, no trunk. Connectors begin one level in, inside a
//! `[role=group]`.
//!
//! This pass reads layout rects + DOM structure only — it never
//! recomputes layout or cascade. Glyph choice stays in the generic
//! joiner. Mirrors the unguarded full-tree collection that
//! `paint_modal_backdrops` already does for dialogs.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{CornerStyle, Display};
use crate::node::TuiNodeExt;
use crate::render::buffer::{BorderContribution, BorderSide, DIR_E, DIR_N, DIR_S, DIR_W};
use crate::render::{Buffer, Rect, Style};
use crate::style::Color;
use rdom_style::layout::BorderStyle;

/// Cells of indent per nesting level. MUST match the
/// `[role=group] { padding-left: N }` value in the UA stylesheet
/// (`rdom-style/src/ua.rs`); the guide columns are derived from it.
const TREE_INDENT: i32 = 2;

/// Entry point — paint guides for every `[role=tree]` in the
/// document. Called from `paint_dom` between the main walk and the
/// border joiner.
pub(super) fn paint_tree_guides(dom: &Dom<TuiExt>, buf: &mut Buffer, clip: Rect) {
    let mut trees = Vec::new();
    collect_trees(dom, dom.root(), &mut trees);
    for tree in trees {
        for item in treeitem_children(dom, tree) {
            paint_item(dom, item, buf, clip, &[]);
        }
    }
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
) {
    if is_hidden(dom, item) {
        return;
    }
    let Some(rect) = dom.node(item).layout_rect() else {
        return;
    };
    let row_y = rect.y;
    let color = guide_color(dom, item);

    // Disclosure chevron in the treeitem's reserved padding cell
    // (`padding-left: 2` ⇒ label starts at `rect.x + 2`, chevron at
    // `rect.x`). Branches only — a branch is any item with an
    // `aria-expanded` attribute (presence, not child count, so an
    // unloaded lazy branch still shows a chevron). Painted here
    // rather than via `::before` to dodge the mixed-content pseudo
    // gap (TREE-BFC-PSEUDO-1).
    if let Some(expanded) = dom.node(item).get_attribute("aria-expanded") {
        let glyph = if expanded == "true" { "▾" } else { "▸" };
        let fg = dom
            .node(item)
            .computed()
            .map(|c| c.fg)
            .unwrap_or(Color::Reset);
        put_glyph(buf, clip, rect.x, row_y, glyph, fg);
    }

    // Ancestor trunks at this row.
    for &(col, continues) in trunks {
        if continues {
            put(buf, clip, col as i32, row_y, &[DIR_N, DIR_S], color);
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
        let connector: &[usize] = if is_last {
            &[DIR_N, DIR_E]
        } else {
            &[DIR_N, DIR_E, DIR_S]
        };
        put(buf, clip, own_col, row_y, connector, color);
        // `─` lead-in toward the label.
        put(buf, clip, own_col + 1, row_y, &[DIR_E, DIR_W], color);
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
            paint_item(dom, child, buf, clip, &child_trunks);
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
