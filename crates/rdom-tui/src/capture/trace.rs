//! [`PaintTrace`]: every operation of one traced paint, indexed by cell.

use super::{OpKind, PaintOp};
use crate::render::Rect;

/// Every operation of one traced paint.
#[derive(Debug, Clone, Default)]
pub struct PaintTrace {
    area: Rect,
    region: Rect,
    ops: Vec<PaintOp>,
    /// Per cell of `area`, row-major: the indices of its operations.
    by_cell: Vec<Vec<u32>>,
    /// How many cells of the traced repaint differ from the frame it
    /// explains (`App::trace_paint`); 0 when it is the frame.
    frame_mismatch: usize,
}

impl PaintTrace {
    pub(crate) fn new(area: Rect, region: Rect, ops: Vec<PaintOp>) -> Self {
        let width = usize::from(area.width);
        let mut by_cell = vec![Vec::new(); width * usize::from(area.height)];
        for (k, op) in ops.iter().enumerate() {
            if area.contains(op.x, op.y) {
                let i = usize::from(op.y - area.y) * width + usize::from(op.x - area.x);
                by_cell[i].push(k as u32);
            }
        }
        PaintTrace {
            area,
            region,
            ops,
            by_cell,
            frame_mismatch: 0,
        }
    }

    pub(crate) fn with_frame_mismatch(mut self, cells: usize) -> Self {
        self.frame_mismatch = cells;
        self
    }

    /// The area the traced paint covered.
    pub fn area(&self) -> Rect {
        self.area
    }

    /// The cells whose operations were recorded.
    pub fn region(&self) -> Rect {
        self.region
    }

    /// Every operation, in paint order.
    pub fn ops(&self) -> &[PaintOp] {
        &self.ops
    }

    /// How many cells of the traced repaint differ from the frame it
    /// explains: 0 — the trace explains the frame exactly — unless the
    /// repaint saw other state (a `<canvas>` callback that draws
    /// differently each time, a layout not brought up to date).
    pub fn frame_mismatch(&self) -> usize {
        self.frame_mismatch
    }

    /// The operations on the cell at `(x, y)`, in paint order.
    pub fn ops_at(&self, x: u16, y: u16) -> impl Iterator<Item = &PaintOp> {
        let cell = self.area.contains(x, y).then(|| {
            usize::from(y - self.area.y) * usize::from(self.area.width)
                + usize::from(x - self.area.x)
        });
        cell.map(|i| self.by_cell[i].as_slice())
            .unwrap_or_default()
            .iter()
            .map(|&k| &self.ops[k as usize])
    }

    /// The operation that decided what the cell at `(x, y)` shows: its last
    /// operation on the frame that wrote the cell — a write, a blend or the
    /// joiner's glyph — or `None` for a cell no paint wrote.
    pub fn top_at(&self, x: u16, y: u16) -> Option<&PaintOp> {
        self.ops_at(x, y)
            .filter(|op| op.depth == 0 && op.kind != OpKind::Border)
            .last()
    }

    /// The operation whose paint the cell at `(x, y)` shows, looking
    /// through layers: [`top_at`](Self::top_at), or — when that is a layer
    /// blending back — the last write in the layer before it, and so on
    /// down. A text in an `opacity: 0.5` group is its element's, not the
    /// group's.
    pub fn painter_at(&self, x: u16, y: u16) -> Option<&PaintOp> {
        let ops: Vec<&PaintOp> = self.ops_at(x, y).collect();
        let mut end = ops.len();
        let mut depth = 0u8;
        let mut found = None;
        loop {
            let pick = ops[..end]
                .iter()
                .rposition(|op| op.depth == depth && op.kind != OpKind::Border);
            let Some(k) = pick else {
                return found;
            };
            found = Some(ops[k]);
            if !matches!(ops[k].kind, OpKind::Blend { .. }) {
                return found;
            }
            // What the layer painted precedes its blend, one level deeper.
            let inner = ops[..k]
                .iter()
                .rposition(|op| op.depth == depth + 1 && op.kind != OpKind::Border);
            match inner {
                Some(_) => {
                    end = k;
                    depth += 1;
                }
                None => return found,
            }
        }
    }
}
