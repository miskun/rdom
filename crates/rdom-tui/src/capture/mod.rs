//! What a frame is made of, for tools that explain it
//! (`specs/CAPTURE.md` §Paint trace): every paint operation that touched
//! a cell, in order, with the element and the part of its paint that made
//! it.
//!
//! A traced paint is the frame's own paint pass run again into a fresh
//! buffer with a recorder attached ([`trace_paint`], [`App::trace_paint`]).
//! The recorder watches the buffer's writes and, each time the paint pass
//! moves from one element's part to the next, records what changed: the
//! cell as it is after the operation, and its border state. The paint pass
//! itself is unchanged — tracing observes it — and without a recorder a
//! write costs one branch.
//!
//! - [`PaintOp`] — one operation on one cell.
//! - [`PaintPart`] — which part of an element's paint it was.
//! - [`OpKind`] — a write, a border contribution, a layer composited back
//!   at an alpha, or the border joiner's glyph.
//! - [`PaintTrace`] — every operation of one paint, by cell.
//! - [`box_model`] — an element's boxes as the last layout placed them.
//! - [`explain_style`] / [`StyleTrace`] — every declaration the cascade
//!   applied to an element, in order, with where each is written.
//!
//! [`App::trace_paint`]: crate::App::trace_paint

use rdom_core::NodeId;

use crate::TuiDom;
use crate::ext::StyleSlot;
use crate::render::{Buffer, Cell, PaintExt, Rect};

/// Which part of an element's paint an operation was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PaintPart {
    /// The canvas: the root element's background over the viewport.
    Canvas,
    /// The element's background fill.
    Background,
    /// A `box-shadow`, outer or inset.
    Shadow,
    /// A border contribution (the joiner draws the glyph: [`Self::BorderJoin`]).
    Border,
    /// A multi-column container's column rules.
    ColumnRule,
    /// The outline ring.
    Outline,
    /// The element's content outside any more specific part: a
    /// `<canvas>` callback, a single-row text.
    Content,
    /// A run of the element's text.
    Text,
    /// A pseudo-element's content or box: `::before`, `::after`,
    /// `::marker`, `::first-letter`.
    Generated(StyleSlot),
    /// A built-in's UA chrome standing for the element's text (a gauge, a
    /// closed `<select>`, a password mask).
    Chrome,
    /// The caret of the focused editable element.
    Caret,
    /// The `::selection` overlay.
    Selection,
    /// A `::highlight()` overlay.
    Highlight,
    /// A scrollbar.
    Scrollbar,
    /// A tree's guide lines and row highlight.
    TreeGuide,
    /// A top-layer element's `::backdrop`.
    Backdrop,
    /// The element's group layer — `opacity`, a `filter`, a blend mode —
    /// composited back onto what is beneath; the operations of the paints
    /// inside it precede it, a level deeper.
    Group,
    /// A `text-overflow` marker.
    TextOverflow,
    /// The border joiner turning contributions into glyphs, after every
    /// element painted.
    BorderJoin,
    /// A write the paint pass made outside every element's part — a gap in
    /// the attribution, never expected.
    Unattributed,
}

impl PaintPart {
    /// The part's name in a capture (`"background"`, `"pseudo:before"`).
    pub fn name(self) -> &'static str {
        match self {
            PaintPart::Canvas => "canvas",
            PaintPart::Background => "background",
            PaintPart::Shadow => "shadow",
            PaintPart::Border => "border",
            PaintPart::ColumnRule => "column-rule",
            PaintPart::Outline => "outline",
            PaintPart::Content => "content",
            PaintPart::Text => "text",
            PaintPart::Generated(slot) => match slot {
                StyleSlot::Before => "pseudo:before",
                StyleSlot::After => "pseudo:after",
                StyleSlot::Marker | StyleSlot::BeforeMarker | StyleSlot::AfterMarker => {
                    "pseudo:marker"
                }
                StyleSlot::FirstLetter => "pseudo:first-letter",
                StyleSlot::Host => "content",
            },
            PaintPart::Chrome => "ua-chrome",
            PaintPart::Caret => "caret",
            PaintPart::Selection => "selection",
            PaintPart::Highlight => "highlight",
            PaintPart::Scrollbar => "scrollbar",
            PaintPart::TreeGuide => "tree-guide",
            PaintPart::Backdrop => "backdrop",
            PaintPart::Group => "group",
            PaintPart::TextOverflow => "text-overflow",
            PaintPart::BorderJoin => "border-join",
            PaintPart::Unattributed => "unattributed",
        }
    }
}

/// What an operation did to its cell.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum OpKind {
    /// Wrote the cell: a glyph, colors, attributes.
    Write,
    /// Added or cleared a border contribution, or recolored one (a tint,
    /// a filter, a layer blending back); the glyph comes from the joiner.
    Border,
    /// A layer — an `opacity` / `filter` group, a translucent color —
    /// composited back onto the cell at `alpha`; the layer's own
    /// operations precede it, one level deeper.
    Blend { alpha: f32 },
    /// The border joiner drew the cell's glyph from its contributions.
    Join,
}

/// One paint operation on one cell.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PaintOp {
    /// The cell's column.
    pub x: u16,
    /// The cell's row.
    pub y: u16,
    /// The element whose paint it was; `None` for the joiner and an
    /// unattributed write.
    pub node: Option<NodeId>,
    /// Which part of its paint.
    pub part: PaintPart,
    /// What it did.
    pub kind: OpKind,
    /// How many layers deep it painted: 0 on the frame, 1 in a group's or
    /// a translucent paint's layer, and so on.
    pub depth: u8,
    /// The cell after the operation (in its layer, for `depth > 0`).
    pub cell: Cell,
    /// The cell's border state after the operation, for a
    /// [`OpKind::Border`] operation.
    pub border: Option<BorderState>,
    /// For a [`OpKind::Join`]: the elements whose contributions won the
    /// cell's directions, in direction order (N, E, S, W), without
    /// repeats.
    pub joined: Vec<NodeId>,
}

/// Paint `dom` — cascaded and laid out — into a fresh buffer of `area`, as
/// the frame does, recording every operation on the cells of `region`.
/// The buffer is the frame's paint exactly; the trace explains it.
pub fn trace_paint(dom: &TuiDom, area: Rect, region: Rect) -> (Buffer, PaintTrace) {
    let mut buf = Buffer::empty(area);
    buf.start_trace(region);
    dom.paint_dom(&mut buf, area);
    let ops = buf.finish_trace();
    (buf, PaintTrace::new(area, area.intersection(region), ops))
}

mod border;
mod boxes;
pub(crate) mod style;
mod trace;

pub use border::{BorderLine, BorderState};
pub use boxes::{BoxModel, ScrollBox, box_model};
pub use style::{
    DeclarationOrigin, Declared, Inheritance, RuleRef, StyleTrace, css_wide_keyword, explain_style,
    inheritance_parent, inherits,
};
pub use trace::PaintTrace;

#[cfg(test)]
mod tests;
