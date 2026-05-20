//! Border-collapse paint joiner (M5.5c).
//!
//! Runs after the main paint pass completes. `paint_border` has
//! recorded per-cell directional connectivity in `buf.border_mask`
//! as each ring was painted — additively, so when two rings' cells
//! coincide their masks OR together. The joiner reads each non-
//! zero mask and rewrites the cell's symbol from the 16-entry
//! `JUNCTION_TABLE`.
//!
//! Bit assignments: `N=0b0001`, `E=0b0010`, `S=0b0100`, `W=0b1000`.
//! - Mask `0b0110` = E + S = `┌` (top-left corner).
//! - Mask `0b0111` = N + E + S = `├` (T-junction where a vertical
//!   meets a rightward horizontal).
//! - Mask `0b1111` = all four = `┼` (cross).
//!
//! When a parent's bottom-left `└` (N + E) and a child's top-left
//! `┌` (E + S) paint at the same cell, the additive mask becomes
//! N + E + S = `├` — the proper T-junction. This closes the
//! `M5-COLLAPSE-2` coincident-corner gap.
//!
//! **Paint-layer invariant preserved:** reads + writes `cell.symbol`
//! only. Never touches `cell.bg`.
//!
//! **Rounded corners:** `paint_border` writes `╭ ╮ ╰ ╯` for cells
//! that only have one ring participating. Once a second ring's mask
//! ORs in, the joiner rewrites to a square junction — Unicode has
//! no rounded junction glyphs.
//!
//! **Gate:** no element has `border-collapse: collapse` → no-op.
//! Skipped via the cached `TuiExt::tree_has_collapse` flag.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::render::Buffer;

pub(super) fn join_borders(dom: &Dom<TuiExt>, buf: &mut Buffer) {
    if !tree_has_collapse(dom) {
        return;
    }

    let area = buf.area;
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let mask = buf.border_mask_at(x, y);
            if mask == 0 {
                continue;
            }
            let replacement = JUNCTION_TABLE[mask as usize];
            if replacement.is_empty() {
                continue;
            }
            if let Some(cell) = buf.cell_mut(x, y)
                && cell.symbol() != replacement
            {
                cell.set_symbol(replacement);
            }
        }
    }
}

fn tree_has_collapse(dom: &Dom<TuiExt>) -> bool {
    walk(dom, dom.root())
}

fn walk(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    for child in dom.node(id).child_nodes() {
        let hit = match child.node_type() {
            NodeType::Element => child.ext().is_some_and(|e| e.tree_has_collapse),
            NodeType::Fragment => walk(dom, child.id()),
            _ => false,
        };
        if hit {
            return true;
        }
    }
    false
}

/// 16-entry junction lookup. Index encoding: bit0 = N, bit1 = E,
/// bit2 = S, bit3 = W. Empty string means "leave the cell alone".
const JUNCTION_TABLE: [&str; 16] = [
    "",  // 0000 - none
    "│", // 0001 - N only
    "─", // 0010 - E only
    "└", // 0011 - N+E
    "│", // 0100 - S only
    "│", // 0101 - N+S
    "┌", // 0110 - E+S
    "├", // 0111 - N+E+S
    "─", // 1000 - W only
    "┘", // 1001 - N+W
    "─", // 1010 - E+W
    "┴", // 1011 - N+E+W
    "┐", // 1100 - S+W
    "┤", // 1101 - N+S+W
    "┬", // 1110 - E+S+W
    "┼", // 1111 - all four
];
