//! Units a math expression or a value can carry besides rdom's unitless
//! cell and the percentage (CSS Values 4 §6 – §7) — lengths and angles —
//! each with its terminal meaning.

use super::{CalcExpr, CalcKind, CalcOp, ResolveCtx};

/// A dimension's unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CalcUnit {
    /// `ch` — the advance of "0": one column on a monospaced grid
    /// (Values 4 §6.1.1).
    Ch,
    /// `lh` — the element's computed `line-height` in rows (Values 4
    /// §6.1.1); in `line-height` itself, its parent's. Resolved at
    /// computed-value time ([`UnitContext`]); one row where no context
    /// gives it.
    Lh,
    /// `rlh` — the root element's computed `line-height` in rows (Values 4
    /// §6.1.1); in the root's own `line-height`, the initial one row.
    Rlh,
    /// A viewport-percentage unit (`vw`, `svh`, `dvmax`, …): 1% of the
    /// terminal on an axis (Values 4 §6.1.2).
    Viewport(ViewportUnit),
    /// A container-relative unit (`cqw`, `cqh`, `cqi`, `cqb`, `cqmin`,
    /// `cqmax`): 1% of the nearest size query container's content box on
    /// the axis, the small viewport's where there is none (CSS
    /// Conditional 5 §6.6). Resolved at computed-value time
    /// ([`UnitContext::with_container`]).
    Container(ViewportAxis),
    /// `deg` — 1/360 of a turn (Values 4 §7.1).
    Deg,
    /// `grad` — 1/400 of a turn (Values 4 §7.1).
    Grad,
    /// `rad` — 1/(2π) of a turn (Values 4 §7.1).
    Rad,
    /// `turn` (Values 4 §7.1).
    Turn,
    /// `px` — the CSS pixel (Values 4 §6.2). Only in the math functions
    /// of the properties whose lengths are pixels — a border width, a
    /// radius, a shadow length (DESIGN "Pixel lengths select, cells
    /// measure") — which `parse_pixel_calc` reads with every
    /// pixel-family unit normalized to it; [`CalcUnit::parse`] never
    /// gives it, as a cell length has no pixels.
    Px,
}

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
/// ([`CalcExpr::absolutize_in`], `ComputedStyle::resolve_context_units`,
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
    fn container_percent(&self, axis: ViewportAxis) -> (f64, UnitReads) {
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

/// A viewport-percentage unit: which viewport size (`sv*` / `lv*` /
/// `dv*` / plain) and which axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportUnit {
    pub size: ViewportSize,
    pub axis: ViewportAxis,
}

/// The viewport size a unit names (CSS Values 4 §6.1.2.1). A terminal
/// has no retractable browser chrome, so all four are the terminal's
/// size; the spelling is kept for serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportSize {
    /// `vw`, `vh`, … — the UA-default viewport.
    Default,
    /// `svw`, … — the small viewport.
    Small,
    /// `lvw`, … — the large viewport.
    Large,
    /// `dvw`, … — the dynamic viewport.
    Dynamic,
}

/// The viewport axis a unit measures (CSS Values 4 §6.1.2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportAxis {
    /// `*w` — the width.
    Width,
    /// `*h` — the height.
    Height,
    /// `*i` — the inline axis: the width in horizontal-tb.
    Inline,
    /// `*b` — the block axis: the height in horizontal-tb.
    Block,
    /// `*min` — the smaller of width and height.
    Min,
    /// `*max` — the larger of width and height.
    Max,
}

impl ViewportUnit {
    const SIZES: [(&'static str, ViewportSize); 4] = [
        ("", ViewportSize::Default),
        ("s", ViewportSize::Small),
        ("l", ViewportSize::Large),
        ("d", ViewportSize::Dynamic),
    ];
    const AXES: [(&'static str, ViewportAxis); 6] = [
        ("vw", ViewportAxis::Width),
        ("vh", ViewportAxis::Height),
        ("vi", ViewportAxis::Inline),
        ("vb", ViewportAxis::Block),
        ("vmin", ViewportAxis::Min),
        ("vmax", ViewportAxis::Max),
    ];

    /// The unit spelled `unit`, ASCII case-insensitive.
    fn parse(unit: &str) -> Option<ViewportUnit> {
        let unit = unit.to_ascii_lowercase();
        Self::SIZES.iter().find_map(|(prefix, size)| {
            let rest = unit.strip_prefix(prefix)?;
            Self::AXES
                .iter()
                .find(|(name, _)| *name == rest)
                .map(|(_, axis)| ViewportUnit {
                    size: *size,
                    axis: *axis,
                })
        })
    }

    /// The unit's CSS spelling.
    pub fn css_name(self) -> &'static str {
        const NAMES: [[&str; 6]; 4] = [
            ["vw", "vh", "vi", "vb", "vmin", "vmax"],
            ["svw", "svh", "svi", "svb", "svmin", "svmax"],
            ["lvw", "lvh", "lvi", "lvb", "lvmin", "lvmax"],
            ["dvw", "dvh", "dvi", "dvb", "dvmin", "dvmax"],
        ];
        let size = Self::SIZES.iter().position(|(_, s)| *s == self.size);
        let axis = Self::AXES.iter().position(|(_, a)| *a == self.axis);
        NAMES[size.unwrap_or(0)][axis.unwrap_or(0)]
    }

    /// The cells 1% of `viewport` is on this unit's axis — every
    /// viewport-percentage length resolves here.
    fn percent_of(self, viewport: Viewport) -> f64 {
        let (w, h) = (f64::from(viewport.cols), f64::from(viewport.rows));
        let extent = match self.axis {
            ViewportAxis::Width | ViewportAxis::Inline => w,
            ViewportAxis::Height | ViewportAxis::Block => h,
            ViewportAxis::Min => w.min(h),
            ViewportAxis::Max => w.max(h),
        };
        extent / 100.0
    }
}

/// The container-relative units (CSS Conditional 5 §6.6) and their axes.
const CONTAINER_UNITS: [(&str, ViewportAxis); 6] = [
    ("cqw", ViewportAxis::Width),
    ("cqh", ViewportAxis::Height),
    ("cqi", ViewportAxis::Inline),
    ("cqb", ViewportAxis::Block),
    ("cqmin", ViewportAxis::Min),
    ("cqmax", ViewportAxis::Max),
];

impl CalcUnit {
    /// The unit of a dimension token, ASCII case-insensitive (CSS
    /// Values 4 §6: unit identifiers are case-insensitive). `None` for a
    /// unit rdom does not take (`px`, `em`, … — DIVERGENCES §1).
    pub fn parse(unit: &str) -> Option<CalcUnit> {
        const TABLE: &[(&str, CalcUnit)] = &[
            ("ch", CalcUnit::Ch),
            ("lh", CalcUnit::Lh),
            ("rlh", CalcUnit::Rlh),
            ("deg", CalcUnit::Deg),
            ("grad", CalcUnit::Grad),
            ("rad", CalcUnit::Rad),
            ("turn", CalcUnit::Turn),
        ];
        TABLE
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(unit))
            .map(|(_, u)| *u)
            .or_else(|| ViewportUnit::parse(unit).map(CalcUnit::Viewport))
            .or_else(|| {
                CONTAINER_UNITS
                    .iter()
                    .find(|(n, _)| n.eq_ignore_ascii_case(unit))
                    .map(|(_, a)| CalcUnit::Container(*a))
            })
    }

    /// The unit's CSS spelling.
    pub fn css_name(self) -> &'static str {
        match self {
            CalcUnit::Ch => "ch",
            CalcUnit::Lh => "lh",
            CalcUnit::Rlh => "rlh",
            CalcUnit::Viewport(v) => v.css_name(),
            CalcUnit::Container(axis) => CONTAINER_UNITS
                .iter()
                .find(|(_, a)| *a == axis)
                .map_or("cqw", |(n, _)| n),
            CalcUnit::Deg => "deg",
            CalcUnit::Grad => "grad",
            CalcUnit::Rad => "rad",
            CalcUnit::Turn => "turn",
            CalcUnit::Px => "px",
        }
    }

    /// The type of a value in this unit.
    pub fn kind(self) -> CalcKind {
        match self {
            CalcUnit::Ch
            | CalcUnit::Lh
            | CalcUnit::Rlh
            | CalcUnit::Viewport(_)
            | CalcUnit::Container(_)
            | CalcUnit::Px => CalcKind::Length,
            CalcUnit::Deg | CalcUnit::Grad | CalcUnit::Rad | CalcUnit::Turn => CalcKind::Angle,
        }
    }

    /// `true` when the value depends on something only known after
    /// parsing (the viewport, the line heights); such a value stays
    /// symbolic until the cascade makes it absolute
    /// ([`CalcExpr::absolutize_in`]).
    pub fn needs_context(self) -> bool {
        match self {
            CalcUnit::Ch
            | CalcUnit::Deg
            | CalcUnit::Grad
            | CalcUnit::Rad
            | CalcUnit::Turn
            | CalcUnit::Px => false,
            CalcUnit::Viewport(_) | CalcUnit::Container(_) | CalcUnit::Lh | CalcUnit::Rlh => true,
        }
    }

    /// `value` in this unit, in the evaluator's canonical unit — cells
    /// for lengths, radians for angles; a pixel expression is all `px`,
    /// so its canonical unit is the pixel.
    pub(super) fn canonical(self, value: f64, cx: &ResolveCtx) -> f64 {
        match self {
            CalcUnit::Viewport(v) => {
                debug_assert!(
                    cx.viewport.is_some(),
                    "`{value}{}` reached layout: a computed-style field \
                     `ComputedStyle::resolve_viewport_units` does not resolve",
                    v.css_name()
                );
                value * v.percent_of(cx.viewport.unwrap_or_default())
            }
            // As a viewport unit: the cascade makes it absolute. One that
            // reaches layout resolves against the small viewport.
            CalcUnit::Container(axis) => {
                debug_assert!(
                    cx.viewport.is_some(),
                    "a container unit reached layout: a computed-style field \
                     `ComputedStyle::resolve_context_units` does not resolve"
                );
                value
                    * ViewportUnit {
                        size: ViewportSize::Small,
                        axis,
                    }
                    .percent_of(cx.viewport.unwrap_or_default())
            }
            // One column. A line-height unit the cascade did not make
            // absolute (a registered custom property's, which is computed
            // before `line-height` is) is one row, `line-height: normal`.
            CalcUnit::Ch | CalcUnit::Lh | CalcUnit::Rlh => value,
            // Angles are radians inside the evaluator.
            CalcUnit::Deg => value.to_radians(),
            CalcUnit::Grad => value * std::f64::consts::PI / 200.0,
            CalcUnit::Rad => value,
            CalcUnit::Turn => value * std::f64::consts::TAU,
            CalcUnit::Px => value,
        }
    }
}

impl CalcExpr {
    /// `true` iff a unit needs a context ([`CalcUnit::needs_context`])
    /// anywhere in the expression.
    pub fn needs_context(&self) -> bool {
        match self {
            CalcExpr::Dimension { unit, .. } => unit.needs_context(),
            CalcExpr::Binary { lhs, rhs, .. } => lhs.needs_context() || rhs.needs_context(),
            CalcExpr::Function { args, .. } => args.iter().any(CalcExpr::needs_context),
            _ => false,
        }
    }

    /// The expression with every viewport-percentage length replaced by
    /// its cells in `viewport`, and the line-height units by one row
    /// each ([`Self::absolutize_in`] with [`UnitContext::new`]).
    pub fn absolutize(&self, viewport: Viewport) -> CalcExpr {
        self.absolutize_in(&UnitContext::new(viewport)).0
    }

    /// The expression with every unit that needs a context replaced by
    /// its cells in `cx` — the computed value (CSS Values 4 §6.1: the
    /// viewport-percentage and font-relative lengths are absolute once
    /// computed) — and the context sizes that read. Percentages stay for
    /// layout.
    pub fn absolutize_in(&self, cx: &UnitContext) -> (CalcExpr, UnitReads) {
        let mut reads = UnitReads::NONE;
        let expr = self.absolutize_reading(cx, &mut reads);
        (expr, reads)
    }

    fn absolutize_reading(&self, cx: &UnitContext, reads: &mut UnitReads) -> CalcExpr {
        match self {
            CalcExpr::Dimension {
                value,
                unit: CalcUnit::Viewport(v),
            } => {
                *reads |= UnitReads::VIEWPORT;
                CalcExpr::Number(value * v.percent_of(cx.viewport))
            }
            CalcExpr::Dimension {
                value,
                unit: CalcUnit::Container(axis),
            } => {
                let (percent, read) = cx.container_percent(*axis);
                *reads |= read;
                CalcExpr::Number(value * percent)
            }
            CalcExpr::Dimension {
                value,
                unit: CalcUnit::Lh,
            } => CalcExpr::Number(value * cx.lh),
            CalcExpr::Dimension {
                value,
                unit: CalcUnit::Rlh,
            } => CalcExpr::Number(value * cx.rlh),
            CalcExpr::Binary { op, lhs, rhs } => CalcExpr::binary(
                *op,
                lhs.absolutize_reading(cx, reads),
                rhs.absolutize_reading(cx, reads),
            ),
            CalcExpr::Function { func, args } => CalcExpr::function(
                *func,
                args.iter()
                    .map(|a| a.absolutize_reading(cx, reads))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    /// What resolving the expression's lengths against a bare viewport
    /// (`ResolveCtx::with_viewport`, no query container) reads: the
    /// viewport, when a viewport-percentage or container-relative length
    /// is in it.
    pub fn viewport_reads(&self) -> UnitReads {
        let reads_viewport = |e: &CalcExpr| match e {
            CalcExpr::Dimension { unit, .. } => {
                matches!(unit, CalcUnit::Viewport(_) | CalcUnit::Container(_))
            }
            _ => false,
        };
        if self.any_node(&reads_viewport) {
            UnitReads::VIEWPORT
        } else {
            UnitReads::NONE
        }
    }

    fn any_node(&self, f: &impl Fn(&CalcExpr) -> bool) -> bool {
        f(self)
            || match self {
                CalcExpr::Binary { lhs, rhs, .. } => lhs.any_node(f) || rhs.any_node(f),
                CalcExpr::Function { args, .. } => args.iter().any(|a| a.any_node(f)),
                _ => false,
            }
    }
}

impl CalcExpr {
    /// The expression as `cells + percent%` — `(cells, percent)` — when
    /// it is linear in its percentages: sums of numbers, lengths and
    /// percentages, scaled by numbers (CSS Values 4 §10.10's simplified
    /// sum of a number and a percentage). `None` when a percentage sits
    /// inside a math function (`min(50%, 10)`) or is multiplied by
    /// another, when a viewport unit still needs the viewport, or when
    /// the length part is not finite. What a registered
    /// `<length-percentage>` computes and interpolates through.
    pub fn linear_parts(&self) -> Option<(f64, f64)> {
        if self.needs_context() || !self.is_linear() {
            return None;
        }
        let cells = self.resolve_f64(&ResolveCtx::new(0));
        let percent = self.resolve_f64(&ResolveCtx::new(100)) - cells;
        (cells.is_finite() && percent.is_finite()).then_some((cells, percent))
    }

    fn is_linear(&self) -> bool {
        match self {
            CalcExpr::Binary {
                op: CalcOp::Add | CalcOp::Sub,
                lhs,
                rhs,
            } => lhs.is_linear() && rhs.is_linear(),
            CalcExpr::Binary {
                op: CalcOp::Mul,
                lhs,
                rhs,
            } => {
                (lhs.is_linear() && !rhs.contains_percent())
                    || (!lhs.contains_percent() && rhs.is_linear())
            }
            CalcExpr::Binary {
                op: CalcOp::Div,
                lhs,
                rhs,
            } => lhs.is_linear() && !rhs.contains_percent(),
            CalcExpr::Function { .. } => !self.contains_percent(),
            CalcExpr::NoBound => false,
            CalcExpr::Number(_)
            | CalcExpr::Length(_)
            | CalcExpr::Percent(_)
            | CalcExpr::Dimension { .. } => true,
        }
    }
}
