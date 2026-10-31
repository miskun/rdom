//! Paint layers and collapse groups of the border table
//! (LAYER-BORDERS).
//!
//! Border contributions are not glyphs: the joiner turns them into
//! glyphs after the whole paint, so the table itself must keep paint
//! order where paint order decides — which, in the browser, is
//! everywhere but a collapsing table's grid.
//!
//! - **Across stacking contexts.** CSS 2.1 Appendix E paints each
//!   stacking context atomically, in order, so a border written in one
//!   covers every border an earlier one left in that cell — exactly as
//!   a later glyph or an opaque background does. The paint walk names
//!   the stacking context it paints in ([`Buffer::enter_border_layer`]);
//!   each cell remembers the layer its border state belongs to, and the
//!   first write from another layer clears the cell's border state
//!   before it lands ([`Buffer::claim`]).
//! - **Inside one stacking context.** Each direction of a cell is owned
//!   by the collapse group that wrote it ([`BorderGroup`]). A write from
//!   the owning group resolves with what is there by CSS Tables 3 §11.5
//!   (`BorderDirState::merge`); a write from another group — painted
//!   later — takes the direction over. The cell's other directions keep
//!   their lines, so the joiner still joins them into one junction
//!   glyph, drawn in the color of the group painted last
//!   ([`DirOwner::seq`]).
//!
//! A buffer the walk never enters a layer of — a consumer's own, or one
//! built in a test — keeps every write in [`BorderLayer::ROOT`] and, through
//! the public `add_border_dir`, in [`BorderGroup::EXTERNAL`]: everything
//! meets in one table resolved by §11.5, as before.

use rdom_core::NodeId;

use super::{BorderCell, BorderContribution, BorderDirState, Buffer};

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

/// The collapse group whose shared edges a border write resolves with
/// (CSS Tables 3 §11.5) — opaque, compared only for equality: a table's
/// grid, a `border-collapse: collapse` container with its direct
/// children, or any other box on its own.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct BorderGroup(u64);

impl BorderGroup {
    /// The writes of the public `Buffer::add_border_dir`: one group, so
    /// a consumer's own writes resolve by §11.5 among themselves.
    pub(crate) const EXTERNAL: Self = Self(0);

    /// The group rooted at the element `id`.
    pub(crate) fn of_element(id: NodeId) -> Self {
        Self(u64::from(id.as_u32()) + 1)
    }

    /// The group of `host`'s generated box in pseudo-element slot `slot`
    /// (one box of its own).
    pub(crate) fn of_pseudo(host: NodeId, slot: u8) -> Self {
        Self(((u64::from(slot) + 1) << 32) | (u64::from(host.as_u32()) + 1))
    }
}

/// Which group owns one direction of a cell, and when it last wrote it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DirOwner {
    /// The collapse group whose writes resolve this direction.
    pub(crate) group: BorderGroup,
    /// The paint order of the group's last write here: of the groups
    /// whose lines meet in a cell, the one painted last colors the
    /// junction.
    pub(crate) seq: u32,
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

    /// The layer the border state at `(x, y)` belongs to;
    /// [`BorderLayer::ROOT`] out of bounds.
    pub(crate) fn border_layer_at(&self, x: u16, y: u16) -> BorderLayer {
        self.index_of(x, y)
            .map(|i| self.border_layers[i])
            .unwrap_or_default()
    }

    /// The owner of direction `dir` at `(x, y)` — meaningful while the
    /// direction holds state (a winner or a kill).
    pub(crate) fn border_owner_at(&self, x: u16, y: u16, dir: usize) -> DirOwner {
        self.index_of(x, y)
            .map(|i| self.border_owners[i][dir])
            .unwrap_or_default()
    }

    /// Add a border contribution of `group` for `(x, y)` in direction
    /// `dir`, in the current layer: from the group that owns the
    /// direction it resolves by CSS Tables 3 §11.5; from any other it
    /// takes the direction over — it painted later. A `none`
    /// contribution writes nothing, and a `hidden` one — which draws
    /// nothing — only kills its own group's edge: over another box's
    /// line it leaves that line be. Out of bounds, nothing.
    pub(crate) fn add_border(
        &mut self,
        x: u16,
        y: u16,
        dir: usize,
        contribution: BorderContribution,
        group: BorderGroup,
    ) {
        debug_assert!(dir < 4, "BorderCell index out of range");
        let Some(i) = self.index_of(x, y) else {
            return;
        };
        self.mark(i, super::coverage::BORDER);
        if contribution.style.is_none() {
            return;
        }
        let foreign = self.border_layers[i] != self.border_layer && self.holds_border(i);
        let held = {
            let state = self.border_dirs[i][dir];
            state.winner.is_some() || state.killed
        };
        let taken = held && self.border_owners[i][dir].group != group;
        if contribution.style.is_hidden() && (foreign || taken) {
            return;
        }
        self.claim(i);
        self.border_seq = self.border_seq.wrapping_add(1);
        let seq = self.border_seq;
        let state = &mut self.border_dirs[i][dir];
        if taken {
            *state = BorderDirState::default();
        }
        state.merge(contribution);
        self.border_owners[i][dir] = DirOwner { group, seq };
    }

    /// True when cell `i` holds any border state: a direction's winner or
    /// kill, or half-block quadrants.
    fn holds_border(&self, i: usize) -> bool {
        self.half_block_quads[i] != 0
            || self.border_dirs[i]
                .iter()
                .any(|d| d.winner.is_some() || d.killed)
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
        self.border_owners[i] = Default::default();
        self.half_block_quads[i] = 0;
        self.border_layers[i] = layer;
    }

    /// Carry on the paint order of `layer`, a copy of a region of this
    /// buffer painted further and composited back.
    pub(super) fn follow_border_seq(&mut self, layer: &Buffer) {
        self.border_seq = self.border_seq.max(layer.border_seq);
    }
}
