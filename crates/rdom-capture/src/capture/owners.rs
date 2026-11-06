//! The owners layer: which element's paint decided what each cell shows
//! (`specs/CAPTURE.md` §Paint trace) — the last operation on the frame
//! that wrote the cell: a write, a blend, or the border joiner's glyph
//! with the elements whose borders met there.
//!
//! Rows are runs of cells with the same owner, tiling the row as the grid's
//! runs do; a cell no paint wrote has no `node` and the part `"none"`.

use rdom_tui::capture::{OpKind, PaintTrace};
use rdom_tui::core_api::NodeId;
use serde::{Deserialize, Serialize};

use super::nodes::node_key;

/// Who painted every cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Owners {
    /// One list of runs per row, top to bottom.
    pub rows: Vec<Vec<OwnerRun>>,
}

/// Consecutive cells of a row with the same owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OwnerRun {
    pub x: u16,
    pub w: u16,
    /// The element; absent for the joiner and for unpainted cells.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    /// Which part of its paint: `"background"`, `"text"`,
    /// `"border-join"`, `"pseudo:before"`, … (`"none"`: unpainted).
    pub part: String,
    /// For a border glyph: the elements whose borders met there.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub joined: Vec<String>,
}

/// The owner of each cell of `trace`'s area, and every element named.
pub(crate) fn owners(trace: &PaintTrace) -> (Owners, Vec<NodeId>) {
    let area = trace.area();
    let mut named: Vec<NodeId> = Vec::new();
    let mut name = |id: NodeId| {
        if !named.contains(&id) {
            named.push(id);
        }
        node_key(id)
    };
    // The top operation of each cell, found in one pass.
    let width = usize::from(area.width);
    let mut top = vec![None; width * usize::from(area.height)];
    for op in trace.ops() {
        if op.depth == 0 && op.kind != OpKind::Border {
            let (x, y) = (usize::from(op.x - area.x), usize::from(op.y - area.y));
            top[y * width + x] = Some(op);
        }
    }
    let mut rows = Vec::with_capacity(usize::from(area.height));
    for y in 0..usize::from(area.height) {
        let mut runs: Vec<OwnerRun> = Vec::new();
        for x in 0..width {
            let (node, part, joined) = match top[y * width + x] {
                Some(op) => (
                    op.node.map(&mut name),
                    op.part.name().to_string(),
                    op.joined.iter().map(|n| name(*n)).collect(),
                ),
                None => (None, "none".to_string(), Vec::new()),
            };
            match runs.last_mut() {
                Some(r) if r.node == node && r.part == part && r.joined == joined => r.w += 1,
                _ => runs.push(OwnerRun {
                    x: x as u16,
                    w: 1,
                    node,
                    part,
                    joined,
                }),
            }
        }
        rows.push(runs);
    }
    (Owners { rows }, named)
}
