//! What the units resolved at computed-value time read (CSS Values 4
//! §6.1, CSS Conditional 5 §6.6): the viewport, the line heights and the
//! query container sizes a resolution takes ([`UnitContext`]), and the
//! sizes it reports having read ([`UnitReads`]).

use super::units::{ViewportAxis, ViewportSize, ViewportUnit};

/// The terminal's size in cells: the viewport the viewport-percentage
/// units are percentages of (CSS Values 4 §6.1.2: the initial
/// containing block).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Viewport {
    pub cols: u16,
    pub rows: u16,
}

impl Viewport {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }
}

/// Which context sizes resolving a value at computed-value time read: the
/// viewport (a viewport-percentage length, CSS Values 4 §6.1.2, or a
/// container-relative one with no query container) and a query
/// container's size (a container-relative length, CSS Conditional 5
/// §6.6). Returned beside the value by every unit resolver
/// ([`CalcExpr::absolutize_in`](super::CalcExpr::absolutize_in), `ComputedStyle::resolve_context_units`,
/// `LineHeight::computed`, …), so a backend learns what a style depends
/// on — a resize restyles only what read the viewport — from the
/// resolution itself, with no state outside it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct UnitReads {
    /// A length read the viewport's size.
    pub viewport: bool,
    /// A length read a query container's size.
    pub container: bool,
}

impl UnitReads {
    /// Nothing read.
    pub const NONE: UnitReads = UnitReads {
        viewport: false,
        container: false,
    };
    /// The viewport read.
    pub const VIEWPORT: UnitReads = UnitReads {
        viewport: true,
        container: false,
    };
    /// A query container read.
    pub const CONTAINER: UnitReads = UnitReads {
        viewport: false,
        container: true,
    };

    /// Whether anything was read.
    pub fn any(self) -> bool {
        self.viewport || self.container
    }
}

impl std::ops::BitOr for UnitReads {
    type Output = UnitReads;
    fn bitor(self, rhs: UnitReads) -> UnitReads {
        UnitReads {
            viewport: self.viewport || rhs.viewport,
            container: self.container || rhs.container,
        }
    }
}

impl std::ops::BitOrAssign for UnitReads {
    fn bitor_assign(&mut self, rhs: UnitReads) {
        *self = *self | rhs;
    }
}

/// What the units resolved at computed-value time are relative to (CSS
/// Values 4 §6.1): the viewport (the viewport-percentage units) and the
/// line heights (`lh`, `rlh`), in rows.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct UnitContext {
    /// The viewport the viewport-percentage units are percentages of.
    pub viewport: Viewport,
    /// Rows one `lh` is: the element's used line height (its parent's
    /// in `line-height` itself).
    pub lh: f64,
    /// Rows one `rlh` is: the root element's used line height.
    pub rlh: f64,
    /// The cells of the nearest query container's content box the
    /// container-relative units of the inline axis resolve against
    /// (`None`: no such container — the small viewport's).
    pub container_inline: Option<f64>,
    /// The same on the block axis.
    pub container_block: Option<f64>,
}

impl UnitContext {
    /// `viewport`, with the line heights one row each (`line-height:
    /// normal`).
    pub fn new(viewport: Viewport) -> Self {
        Self {
            viewport,
            lh: 1.0,
            rlh: 1.0,
            container_inline: None,
            container_block: None,
        }
    }

    /// This context with the query container sizes the
    /// container-relative units resolve against, per axis (CSS
    /// Conditional 5 §6.6).
    pub fn with_container(mut self, inline: Option<f64>, block: Option<f64>) -> Self {
        self.container_inline = inline;
        self.container_block = block;
        self
    }

    /// The cells 1% of the query container is on `axis`, the small
    /// viewport's on an axis with none, and what that read.
    pub(super) fn container_percent(&self, axis: ViewportAxis) -> (f64, UnitReads) {
        let one = |size: Option<f64>, fallback: ViewportAxis| match size {
            Some(cells) => (cells / 100.0, UnitReads::CONTAINER),
            None => (
                ViewportUnit {
                    size: ViewportSize::Small,
                    axis: fallback,
                }
                .percent_of(self.viewport),
                UnitReads::VIEWPORT,
            ),
        };
        let inline = || one(self.container_inline, ViewportAxis::Width);
        let block = || one(self.container_block, ViewportAxis::Height);
        match axis {
            ViewportAxis::Width | ViewportAxis::Inline => inline(),
            ViewportAxis::Height | ViewportAxis::Block => block(),
            ViewportAxis::Min | ViewportAxis::Max => {
                let ((i, a), (b, c)) = (inline(), block());
                let v = if axis == ViewportAxis::Min {
                    i.min(b)
                } else {
                    i.max(b)
                };
                (v, a | c)
            }
        }
    }

    /// This context with `lh` and `rlh` rows for the line-height units.
    pub fn with_line_heights(mut self, lh: f64, rlh: f64) -> Self {
        self.lh = lh;
        self.rlh = rlh;
        self
    }
}
