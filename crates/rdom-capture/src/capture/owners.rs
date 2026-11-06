//! The owners layer: which element's paint decided what each cell shows
//! (`specs/CAPTURE.md` §Paint trace) — the paint the cell shows, looking
//! through layers (`PaintTrace::painter_at`): a write, the border joiner's
//! glyph with the elements whose borders met there, and the element whose
//! layer it reached the frame through (`via`).
//!
//! Rows are runs of cells with the same owner, tiling the row as the grid's
//! runs do; a cell no paint wrote has no `node` and the part `"none"`.

use rdom_tui::capture::PaintTrace;
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
    /// The element whose layer — `opacity`, a filter, a translucent color
    /// — the paint reached the frame through, when not `node` itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
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
    let width = usize::from(area.width);
    let mut rows = Vec::with_capacity(usize::from(area.height));
    for y in 0..usize::from(area.height) {
        let mut runs: Vec<OwnerRun> = Vec::new();
        for x in 0..width {
            let (cx, cy) = (area.x + x as u16, area.y + y as u16);
            let painter = trace.painter_at(cx, cy);
            let via = trace
                .top_at(cx, cy)
                .filter(|top| painter.is_some_and(|p| p.depth > 0 && p.node != top.node))
                .and_then(|top| top.node);
            let (node, part, joined) = match painter {
                Some(op) => (
                    op.node.map(&mut name),
                    op.part.name().to_string(),
                    op.joined.iter().map(|n| name(*n)).collect(),
                ),
                None => (None, "none".to_string(), Vec::new()),
            };
            let via = via.map(&mut name);
            match runs.last_mut() {
                Some(r)
                    if r.node == node && r.part == part && r.joined == joined && r.via == via =>
                {
                    r.w += 1
                }
                _ => runs.push(OwnerRun {
                    x: x as u16,
                    w: 1,
                    node,
                    part,
                    joined,
                    via,
                }),
            }
        }
        rows.push(runs);
    }
    (Owners { rows }, named)
}
