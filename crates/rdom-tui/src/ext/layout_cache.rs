//! Layout results cached on an element: positioned pseudo-element
//! rects, the static position of an out-of-flow box, the per-pass
//! margin-chain memo and the anonymous block boxes of mixed content.

use crate::layout::{LayoutRect, Position};
use crate::render::inline::InlineLayout;

/// Layout state for a positioned `::before` / `::after` pseudo-
/// element. Carries the rect (where the pseudo paints) plus the
/// cascaded `position` (so paint can route static pseudos through
/// the inline-append path and non-static pseudos through the
/// positioned-pseudo paint pass). Populated by the layout pass's
/// `place_positioned_pseudos` phase.
///
/// Static-position pseudos (the default) do NOT populate this —
/// they paint inline via the inline-content path. Only
/// `Position::Relative | Absolute | Fixed` produces a slot here.
///
/// Consumers reading this for debug snapshots or hit-test work
/// should note the divergence on `TuiExt::before_layout` /
/// `after_layout` — positioned pseudo rects do not participate
/// in hit-testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PseudoLayout {
    pub rect: LayoutRect,
    pub position: Position,
}

/// The **static position** of an out-of-flow positioned element
/// (CSS 2.1 §10.3.7 / §10.6.4): where its top-left corner would be
/// if it were `position: static`, in the same coordinate space as
/// [`TuiExt::layout`](super::TuiExt::layout). Phase-1 layout records it at the point in the
/// parent's flow where the element's hypothetical box would have
/// gone; phase-2 placement reads it for every axis whose two insets
/// are both `auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticPosition {
    pub x: i32,
    pub y: i32,
}

/// Memoized CSS 2.1 §8.3.1 outer-margin chains of one block, valid for
/// one containing-block width and one layout pass
/// (`BFC1-PERF-MARGIN-CHAIN-1`). Placing a block walks its first- /
/// last-child collapse chain; without the memo every level of a deep
/// chain re-walked the levels below it when its own turn came. Each
/// accumulator is `(largest positive margin, most negative margin)`.
///
/// Invariant: a chain walk only visits in-flow block-level children of
/// a block container that does not establish a new formatting context
/// (it stops at inline content, at a BFC and at padding / borders), and
/// every such child is laid out by `layout_node` later in the same
/// pass, which clears its entry — so no entry outlives the pass
/// (checked in debug builds at the end of `layout_dom`). An entry is
/// used only for the containing-block width it was computed against;
/// a width that differs (a scrollbar gutter, a border-collapse inset)
/// makes the walk recompute, never misread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MarginChainMemo {
    /// The width the chain's percentages were resolved against.
    pub containing_block_width: u16,
    /// The chain surfacing at the block's outer top edge.
    pub outer_top: Option<(i16, i16)>,
    /// The chain surfacing at the block's outer bottom edge.
    pub outer_bottom: Option<(i16, i16)>,
}

/// One synthesized **anonymous block box** wrapping a run of
/// inline-level children inside a block container. Per CSS 2.1
/// §9.2.1.1, when a block-flow container has mixed block + inline
/// children, the inline runs are wrapped in anonymous boxes that
/// each establish their own IFC.
///
/// Anonymous boxes have no `NodeId` (they're layout-pass ephemera
/// allocated per cascade). Their `inline_layout` carries text
/// fragments owned by real source nodes; hit-test and selection
/// resolve through those owners. `child_range` records the
/// document-order indices (within the parent's box-tree children: its
/// child nodes, with each `display: contents` child that holds a block
/// box replaced by its own children and generated text, CSS Display 3
/// §2.5) the anon box wraps. The host's static `::before` /
/// `::after` are packed into the first / last box's `inline_layout`
/// (as `LineBox::generated`); a pseudo whose host starts / ends with a
/// block-level child gets a box of its own, with an empty
/// `child_range`.
///
/// A flex container's anonymous flex items (CSS Flexbox §4) are stored
/// here too: a run of its text, or one `::before` / `::after`, whose
/// box is its own ([`generated`](Self::generated)).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct AnonymousIfc {
    /// Where this box's lines sit: its content box (for an anonymous
    /// block box, its whole box — it has no padding or border). Width =
    /// parent content width; height = inline_layout.height().
    pub rect: LayoutRect,
    /// IFC packing of the wrapped inline run.
    pub inline_layout: InlineLayout,
    /// Indices into the parent's box-tree children covered by this
    /// anonymous box, as `[start, end)` — its `child_nodes()` unless a
    /// `display: contents` child holds a block box. Hit-test and
    /// selection use this to map a fragment to its surrounding DOM
    /// neighbors.
    pub child_range: (usize, usize),
    /// The `::before` / `::after` flex item this box is, with its border
    /// box (`rect` is its content box); `None` for an anonymous box.
    pub generated: Option<GeneratedBox>,
}

impl AnonymousIfc {
    /// A box whose lines sit at `rect`.
    pub fn new(
        rect: LayoutRect,
        inline_layout: InlineLayout,
        child_range: (usize, usize),
        generated: Option<GeneratedBox>,
    ) -> Self {
        Self {
            rect,
            inline_layout,
            child_range,
            generated,
        }
    }

    /// The box's border box: a generated item's own, else `rect`.
    pub fn border_box(&self) -> LayoutRect {
        self.generated.map_or(self.rect, |g| g.border_box)
    }
}

/// A `::before` / `::after` laid out as a flex item (CSS Flexbox §4): a
/// box of its own, styled by its computed style (`TuiExt::computed_before`
/// / `computed_after` of `host`), whose content is its [`AnonymousIfc`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct GeneratedBox {
    /// The pseudo-element's originating element.
    pub host: rdom_core::NodeId,
    /// Which pseudo-element.
    pub slot: super::PseudoSlot,
    /// Its border box, in the coordinates of `TuiExt::layout`.
    pub border_box: LayoutRect,
}

impl GeneratedBox {
    /// The `slot` pseudo-element of `host`, at `border_box`.
    pub fn new(host: rdom_core::NodeId, slot: super::PseudoSlot, border_box: LayoutRect) -> Self {
        Self {
            host,
            slot,
            border_box,
        }
    }
}
