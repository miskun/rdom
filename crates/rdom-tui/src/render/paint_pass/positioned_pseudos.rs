//! Paint pass for positioned `::before` / `::after` pseudo-elements
//! (M5-now Stage B).
//!
//! Runs after `paint_z_list` so that absolute pseudos can paint above
//! positioned host elements (they share the same flat stacking
//! context). Reads pre-computed `PseudoLayout` rects from
//! `TuiExt::before_layout` / `after_layout`, which the layout pass
//! populated in `layout_pass::positioned_pseudos`.
//!
//! Sort key: `(host.z_index, host doc_order, pseudo_order)` where
//! `pseudo_order` puts `::before` before `::after` for tie-break.
//!
//! Pseudos do NOT participate in hit-test — clicks fall through to
//! the host element.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::render::paint_pass::text::{paint_text, style_from_computed};
use crate::render::{Buffer, Rect};
use crate::style::Color;

use super::layout_rect_to_grid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PseudoEnd {
    Before = 0,
    After = 1,
}

pub(super) fn paint_positioned_pseudos(dom: &Dom<TuiExt>, buf: &mut Buffer, clip: Rect) {
    // Cascade-level early-exit (D-M5N-2). When no positioned pseudos
    // exist in the tree, skip the host walk + sort + iteration
    // entirely. The aggregate flag was written by the cascade pass.
    if !tree_has_any_positioned_pseudo(dom) {
        return;
    }

    let mut list: Vec<(i16, usize, NodeId, PseudoEnd)> = Vec::new();
    let mut order: usize = 0;
    collect(dom, dom.root(), &mut list, &mut order);
    list.sort_by_key(|(z, ord, _, end)| (*z, *ord, *end as u8));

    for (_, _, host_id, end) in list {
        let (rect, style) = {
            let ext = match dom.node(host_id).ext() {
                Some(e) => e,
                None => continue,
            };
            match end {
                PseudoEnd::Before => match (&ext.before_layout, &ext.computed_before) {
                    (Some(layout), Some(style)) => (layout.rect, style.clone()),
                    _ => continue,
                },
                PseudoEnd::After => match (&ext.after_layout, &ext.computed_after) {
                    (Some(layout), Some(style)) => (layout.rect, style.clone()),
                    _ => continue,
                },
            }
        };

        if rect.width == 0 || rect.height == 0 {
            continue;
        }
        let Some(grid) = layout_rect_to_grid(rect, clip) else {
            continue;
        };

        let bg = style.bg;
        let fg = style.fg;
        if bg != Color::Reset {
            for y in grid.y..grid.bottom() {
                for x in grid.x..grid.right() {
                    if let Some(cell) = buf.cell_mut(x, y) {
                        cell.set_bg(bg);
                        if fg != Color::Reset {
                            cell.set_fg(fg);
                        }
                    }
                }
            }
        }
        if let Some(text) = style.content.as_deref()
            && !text.is_empty()
        {
            let _ = paint_text(
                buf,
                grid.x,
                grid.y,
                grid.right(),
                text,
                style_from_computed(&style),
            );
        }
    }
}

/// OR `tree_has_positioned_pseudo` across the root's children
/// (transparently descending through nested Fragments). Mirrors the
/// helper in `layout_pass::positioned_pseudos`; both passes consult
/// the same cascade-written flag.
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

fn collect(
    dom: &Dom<TuiExt>,
    id: NodeId,
    out: &mut Vec<(i16, usize, NodeId, PseudoEnd)>,
    order: &mut usize,
) {
    if dom.node(id).node_type() == NodeType::Element
        && let Some(ext) = dom.node(id).ext()
    {
        let host_z = ext
            .computed
            .as_ref()
            .map(|c| match c.z_index {
                crate::layout::ZIndex::Auto => 0,
                crate::layout::ZIndex::Value(n) => n,
            })
            .unwrap_or(0);
        if ext.before_layout.is_some() {
            out.push((host_z, *order, id, PseudoEnd::Before));
            *order += 1;
        }
        if ext.after_layout.is_some() {
            out.push((host_z, *order, id, PseudoEnd::After));
            *order += 1;
        }
    }
    for child in dom.node(id).child_nodes() {
        match child.node_type() {
            NodeType::Element | NodeType::Fragment => {
                collect(dom, child.id(), out, order);
            }
            _ => {}
        }
    }
}
