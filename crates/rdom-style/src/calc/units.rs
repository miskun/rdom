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
    /// `lh` — the element's line height: one row, as rdom's line is
    /// until `line-height` lands (C9-LINE-HEIGHT) (Values 4 §6.1.1).
    Lh,
    /// `rlh` — the root's line height: one row (Values 4 §6.1.1).
    Rlh,
    /// A viewport-percentage unit (`vw`, `svh`, `dvmax`, …): 1% of the
    /// terminal on an axis (Values 4 §6.1.2).
    Viewport(ViewportUnit),
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

    /// The cells 1% of `viewport` is on this unit's axis.
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
    }

    /// The unit's CSS spelling.
    pub fn css_name(self) -> &'static str {
        match self {
            CalcUnit::Ch => "ch",
            CalcUnit::Lh => "lh",
            CalcUnit::Rlh => "rlh",
            CalcUnit::Viewport(v) => v.css_name(),
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
            CalcUnit::Ch | CalcUnit::Lh | CalcUnit::Rlh | CalcUnit::Viewport(_) | CalcUnit::Px => {
                CalcKind::Length
            }
            CalcUnit::Deg | CalcUnit::Grad | CalcUnit::Rad | CalcUnit::Turn => CalcKind::Angle,
        }
    }

    /// `true` when the value depends on something only known after
    /// parsing (the viewport); such a value stays symbolic until the
    /// cascade makes it absolute ([`CalcExpr::absolutize`]).
    pub fn needs_context(self) -> bool {
        match self {
            CalcUnit::Ch
            | CalcUnit::Lh
            | CalcUnit::Rlh
            | CalcUnit::Deg
            | CalcUnit::Grad
            | CalcUnit::Rad
            | CalcUnit::Turn
            | CalcUnit::Px => false,
            CalcUnit::Viewport(_) => true,
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
            // One column; one row (the fixed line height) — a cell
            // either way.
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
    /// `true` iff a unit needs the viewport ([`CalcUnit::needs_context`])
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
    /// its cells in `viewport` — the computed value (CSS Values 4
    /// §6.1.2: viewport units are absolute lengths once computed).
    /// Percentages stay for layout.
    pub fn absolutize(&self, viewport: Viewport) -> CalcExpr {
        match self {
            CalcExpr::Dimension {
                value,
                unit: CalcUnit::Viewport(v),
            } => CalcExpr::Number(value * v.percent_of(viewport)),
            CalcExpr::Binary { op, lhs, rhs } => {
                CalcExpr::binary(*op, lhs.absolutize(viewport), rhs.absolutize(viewport))
            }
            CalcExpr::Function { func, args } => {
                CalcExpr::function(*func, args.iter().map(|a| a.absolutize(viewport)).collect())
            }
            other => other.clone(),
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
