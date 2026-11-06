//! The paint recorder (`crate::capture`): an optional observer on a
//! [`Buffer`] that turns the paint pass's writes into [`PaintOp`]s.
//!
//! Every write to a cell's content or border state goes through one of
//! the buffer's funnels — `cell_mut`, `mark` / `mark_at`, the compositing
//! and color-mapping loops — and each calls [`Buffer::touch`]: with no
//! recorder, one branch; with one, the cell joins a dirty list. The paint
//! pass names whose paint it is running with [`Buffer::painting`] (an
//! element and a [`PaintPart`]); each time that changes, the recorder
//! compares the dirty cells with what it last saw and records one
//! operation per change, attributed to the part that was running.
//!
//! A layer — a copy of a region painted further and composited back
//! (`opacity`, a filter, a translucent color) — gets a recorder of its own
//! one level deeper ([`Buffer::copy_region_into`]); compositing it back
//! ([`Buffer::composite_layer`]) adopts its operations and records the
//! blend.

use rdom_core::NodeId;

use super::{BorderCell, Buffer};
use crate::capture::{OpKind, PaintOp, PaintPart};
use crate::render::{Cell, Rect};

/// The recorder a buffer carries while traced. Not content: a clone of the
/// buffer starts without one.
#[derive(Default)]
pub(crate) struct Tracer(pub(super) Option<Box<Recorder>>);

impl Clone for Tracer {
    fn clone(&self) -> Self {
        Tracer(None)
    }
}

impl std::fmt::Debug for Tracer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.is_some() {
            "Tracer(on)"
        } else {
            "Tracer(off)"
        })
    }
}

/// What a traced buffer records.
pub(crate) struct Recorder {
    /// The buffer's area when recording started (indices are into it).
    area: Rect,
    /// The cells whose operations are recorded.
    region: Rect,
    /// Whose paint is running: the innermost last.
    stack: Vec<(Option<NodeId>, PaintPart)>,
    /// How many layers deep this buffer is.
    depth: u8,
    /// Cells written since the last flush, and a mark per cell.
    dirty: Vec<u32>,
    is_dirty: Vec<bool>,
    /// Each cell's content and border state as last recorded.
    cells: Vec<Cell>,
    borders: Vec<BorderCell>,
    quads: Vec<u8>,
    /// Per cell and direction, the element whose contribution the
    /// direction's state last took (the joiner's `joined`).
    dir_nodes: Vec<[Option<NodeId>; 5]>,
    /// While a layer composites back: its alpha.
    blend: Option<f32>,
    ops: Vec<PaintOp>,
}

/// The `dir_nodes` slot of the half-block quadrants.
const QUADS: usize = 4;

impl Recorder {
    /// A recorder over `buf`'s current cells, recording `region`.
    fn over(
        buf: &Buffer,
        region: Rect,
        depth: u8,
        stack: Vec<(Option<NodeId>, PaintPart)>,
    ) -> Self {
        let len = buf.content.len();
        Recorder {
            area: buf.area,
            region: buf.area.intersection(region),
            stack,
            depth,
            dirty: Vec::new(),
            is_dirty: vec![false; len],
            cells: buf.content.clone(),
            borders: buf.border_dirs.clone(),
            quads: buf.half_block_quads.clone(),
            dir_nodes: vec![[None; 5]; len],
            blend: None,
            ops: Vec::new(),
        }
    }

    /// Note cell `i` as written.
    #[inline]
    fn touch(&mut self, i: usize) {
        if !self.is_dirty[i] {
            self.is_dirty[i] = true;
            self.dirty.push(i as u32);
        }
    }

    /// Whose paint is running.
    fn current(&self) -> (Option<NodeId>, PaintPart) {
        self.stack
            .last()
            .copied()
            .unwrap_or((None, PaintPart::Unattributed))
    }

    /// Record what the dirty cells' state says changed since last seen,
    /// against `buf`'s content and border tables, in cell order.
    fn flush(&mut self, content: &[Cell], borders: &[BorderCell], quads: &[u8]) {
        if self.dirty.is_empty() {
            return;
        }
        let mut dirty = std::mem::take(&mut self.dirty);
        dirty.sort_unstable();
        let (node, part) = self.current();
        let width = usize::from(self.area.width.max(1));
        for &i in &dirty {
            let i = i as usize;
            self.is_dirty[i] = false;
            let x = self.area.x + (i % width) as u16;
            let y = self.area.y + (i / width) as u16;
            let recorded = self.region.contains(x, y);
            if borders[i] != self.borders[i] || quads[i] != self.quads[i] {
                // A direction changes hands when its winning contribution
                // does — not when a filter, a tint or a blend recolors it.
                for (d, (now, seen)) in borders[i].iter().zip(&self.borders[i]).enumerate() {
                    if !same_contribution(now, seen) {
                        self.dir_nodes[i][d] = node;
                    }
                }
                if quads[i] != self.quads[i] {
                    self.dir_nodes[i][QUADS] = node;
                }
                self.borders[i] = borders[i];
                self.quads[i] = quads[i];
                if recorded && part != PaintPart::BorderJoin {
                    self.ops.push(self.op(
                        x,
                        y,
                        node,
                        part,
                        OpKind::Border,
                        &content[i],
                        Some(crate::capture::BorderState::of(&borders[i], quads[i])),
                    ));
                }
            }
            if content[i] != self.cells[i] {
                self.cells[i] = content[i].clone();
                if recorded {
                    let kind = match (part, self.blend) {
                        (_, Some(alpha)) => OpKind::Blend { alpha },
                        (PaintPart::BorderJoin, None) => OpKind::Join,
                        _ => OpKind::Write,
                    };
                    let mut op = self.op(x, y, node, part, kind, &content[i], None);
                    if kind == OpKind::Join {
                        op.joined = self.joined(i, &borders[i], quads[i]);
                    }
                    self.ops.push(op);
                }
            }
        }
        dirty.clear();
        self.dirty = dirty;
    }

    #[allow(clippy::too_many_arguments)]
    fn op(
        &self,
        x: u16,
        y: u16,
        node: Option<NodeId>,
        part: PaintPart,
        kind: OpKind,
        cell: &Cell,
        border: Option<crate::capture::BorderState>,
    ) -> PaintOp {
        PaintOp {
            x,
            y,
            node,
            part,
            kind,
            depth: self.depth,
            cell: cell.clone(),
            border,
            joined: Vec::new(),
        }
    }

    /// The elements whose contributions hold cell `i`'s visible
    /// directions and quadrants, in direction order, without repeats.
    fn joined(&self, i: usize, border: &BorderCell, quads: u8) -> Vec<NodeId> {
        let mut out: Vec<NodeId> = Vec::new();
        let visible = (0..4).filter(|&d| border[d].is_visible());
        let slots = visible.chain((quads != 0).then_some(QUADS));
        for slot in slots {
            if let Some(n) = self.dir_nodes[i][slot]
                && !out.contains(&n)
            {
                out.push(n);
            }
        }
        out
    }
}

/// Whether two states of a direction hold the same contribution, its
/// color aside: a recolor (a filter, a tint, a layer blending back) keeps
/// the element whose border it is.
fn same_contribution(a: &super::BorderDirState, b: &super::BorderDirState) -> bool {
    let key = |s: &super::BorderDirState| {
        s.winner
            .map(|w| (w.style, w.weight, w.priority, w.corner_style, w.side))
    };
    a.killed == b.killed && key(a) == key(b)
}

impl Buffer {
    /// Note cell `i` as written, for the recorder if one is attached.
    #[inline]
    pub(crate) fn touch(&mut self, i: usize) {
        if let Some(r) = self.tracer.0.as_deref_mut() {
            r.touch(i);
        }
    }

    /// Whether a recorder is attached.
    pub(crate) fn is_traced(&self) -> bool {
        self.tracer.0.is_some()
    }

    /// Attach a recorder that records the operations on `region`'s cells.
    pub(crate) fn start_trace(&mut self, region: Rect) {
        self.tracer.0 = Some(Box::new(Recorder::over(self, region, 0, Vec::new())));
    }

    /// Detach the recorder and return its operations, in paint order.
    pub(crate) fn finish_trace(&mut self) -> Vec<PaintOp> {
        self.flush_trace();
        self.tracer.0.take().map(|r| r.ops).unwrap_or_default()
    }

    /// Record what changed since the last flush.
    fn flush_trace(&mut self) {
        if let Some(r) = self.tracer.0.as_deref_mut() {
            r.flush(&self.content, &self.border_dirs, &self.half_block_quads);
        }
    }

    /// Run `paint` as `node`'s `part`: the writes it makes are recorded as
    /// that part's operations (a nested `painting` takes over for its own).
    /// Without a recorder, just `paint(self)`.
    #[inline]
    pub(crate) fn painting<T>(
        &mut self,
        node: Option<NodeId>,
        part: PaintPart,
        paint: impl FnOnce(&mut Buffer) -> T,
    ) -> T {
        if !self.is_traced() {
            return paint(self);
        }
        self.flush_trace();
        if let Some(r) = self.tracer.0.as_deref_mut() {
            r.stack.push((node, part));
        }
        let out = paint(self);
        self.flush_trace();
        if let Some(r) = self.tracer.0.as_deref_mut() {
            r.stack.pop();
        }
        out
    }

    /// Cell `i`'s border state was put back as `backdrop`'s cell `j` holds
    /// it: take the backdrop's border as seen, with the elements its
    /// directions came from, so the restore moves no border to whoever
    /// is painting.
    pub(super) fn trace_backdrop_border(&mut self, i: usize, backdrop: &Buffer, j: usize) {
        let Some(r) = self.tracer.0.as_deref_mut() else {
            return;
        };
        r.borders[i] = self.border_dirs[i];
        r.quads[i] = self.half_block_quads[i];
        if let Some(b) = backdrop.tracer.0.as_deref() {
            r.dir_nodes[i] = b.dir_nodes[j];
        }
    }

    /// Give `layer` — just copied from this buffer — a recorder one level
    /// deeper, carrying on whose paint is running.
    pub(crate) fn trace_layer(&mut self, layer: &mut Buffer) {
        self.flush_trace();
        let Some(r) = self.tracer.0.as_deref() else {
            layer.tracer.0 = None;
            return;
        };
        let mut child = Recorder::over(layer, r.region, r.depth.saturating_add(1), r.stack.clone());
        // The layer's cells start as copies of ours: so do the elements
        // their border directions came from.
        for y in layer.area.y..layer.area.bottom() {
            for x in layer.area.x..layer.area.right() {
                if let (Some(i), Some(j)) = (self.index_of(x, y), layer.index_of(x, y)) {
                    child.dir_nodes[j] = r.dir_nodes[i];
                }
            }
        }
        layer.tracer.0 = Some(Box::new(child));
    }

    /// Composite `layer` back at `alpha` ([`Self::composite_group`]),
    /// adopting its recorded operations, then recording the blend.
    pub(crate) fn composite_layer(&mut self, layer: &mut Buffer, alpha: f32) {
        if !self.is_traced() {
            self.composite_group(layer, alpha);
            return;
        }
        self.flush_trace();
        layer.flush_trace();
        let mut child = layer.tracer.0.take();
        if let (Some(r), Some(child)) = (self.tracer.0.as_deref_mut(), child.as_deref_mut()) {
            r.ops.append(&mut child.ops);
        }
        if let Some(r) = self.tracer.0.as_deref_mut() {
            r.blend = Some(alpha);
        }
        self.composite_group(layer, alpha);
        self.flush_trace();
        // A direction the layer's contribution holds after compositing —
        // blended or not — is the element's that contributed it in the
        // layer, not the group's.
        if let Some(child) = child.as_deref() {
            let mut adopt: Vec<(usize, usize, usize)> = Vec::new();
            for y in layer.area.y..layer.area.bottom() {
                for x in layer.area.x..layer.area.right() {
                    let (Some(i), Some(j)) = (self.index_of(x, y), layer.index_of(x, y)) else {
                        continue;
                    };
                    for d in 0..4 {
                        if same_contribution(&self.border_dirs[i][d], &layer.border_dirs[j][d]) {
                            adopt.push((i, j, d));
                        }
                    }
                    let quads = self.half_block_quads[i];
                    if quads != 0 && quads == layer.half_block_quads[j] {
                        adopt.push((i, j, QUADS));
                    }
                }
            }
            if let Some(r) = self.tracer.0.as_deref_mut() {
                for (i, j, d) in adopt {
                    r.dir_nodes[i][d] = child.dir_nodes[j][d];
                }
            }
        }
        if let Some(r) = self.tracer.0.as_deref_mut() {
            r.blend = None;
        }
    }
}
