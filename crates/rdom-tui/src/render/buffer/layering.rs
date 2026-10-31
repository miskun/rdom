//! Paint layers of the border table (LAYER-BORDERS).
//!
//! Border contributions are not glyphs: the joiner turns them into
//! glyphs after the whole paint, so the table itself must keep paint
//! order where paint order decides. CSS 2.1 Appendix E paints each
//! stacking context atomically, in order, so a border written in one
//! stacking context covers every border an earlier one left in that
//! cell — exactly as a later glyph or an opaque background does. Inside
//! one stacking context borders still meet and join.
//!
//! The paint walk names the stacking context it paints in
//! ([`Buffer::enter_border_layer`]); each cell remembers the layer its
//! border state belongs to, and the first write from another layer
//! clears the cell's border state before it lands ([`Buffer::claim`]).
//! A buffer the walk never enters a layer of — a consumer's own, or one
//! built in a test — keeps every write in [`BorderLayer::ROOT`], so
//! everything meets in one table there, as before.

use rdom_core::NodeId;

use super::{BorderCell, Buffer};

/// The stacking context a border write paints in — opaque, compared
/// only for equality. The document's is [`BorderLayer::ROOT`]; a nested
/// context is named by the box that roots it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct BorderLayer(u64);

impl BorderLayer {
    /// The document's stacking context, and the one layer of a buffer
    /// no paint walk enters a layer of.
    pub(crate) const ROOT: Self = Self(0);

    /// The stacking context rooted at the element `id`.
    pub(crate) fn of_element(id: NodeId) -> Self {
        Self(u64::from(id.as_u32()) + 1)
    }

    /// The stacking context rooted at `host`'s `k`-th positioned
    /// pseudo-element box.
    pub(crate) fn of_pseudo(host: NodeId, k: usize) -> Self {
        Self(((k as u64 + 1) << 32) | (u64::from(host.as_u32()) + 1))
    }
}

impl Buffer {
    /// Paint the border writes that follow in `layer`, until
    /// [`Self::leave_border_layer`] restores the layer this returns.
    pub(crate) fn enter_border_layer(&mut self, layer: BorderLayer) -> BorderLayer {
        std::mem::replace(&mut self.border_layer, layer)
    }

    /// Restore the layer [`Self::enter_border_layer`] returned.
    pub(crate) fn leave_border_layer(&mut self, previous: BorderLayer) {
        self.border_layer = previous;
    }

    /// Take cell `i` for a border write in the current layer: a cell
    /// whose border state an earlier stacking context left is cleared
    /// first — the later context covers it (Appendix E).
    pub(super) fn claim(&mut self, i: usize) {
        let layer = self.border_layer;
        if self.border_layers[i] == layer {
            return;
        }
        self.border_dirs[i] = BorderCell::default();
        self.half_block_quads[i] = 0;
        self.border_layers[i] = layer;
    }
}
