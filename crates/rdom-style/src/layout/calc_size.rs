//! `calc-size()` and `interpolate-size` (CSS Values 5 §10, §11): a size
//! computed from a sizing keyword's resolved value — what lets `height:
//! auto` animate.

use crate::calc::CalcExpr;

use super::IntrinsicSize;

/// `interpolate-size: numeric-only | allow-keywords` (CSS Values 5 §11):
/// whether a sizing keyword (`auto`, `min-content`, …) interpolates with
/// a length through `calc-size()`. Inherited; initial `numeric-only`;
/// not animatable. Closed (DESIGN): the two values of its grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InterpolateSize {
    /// Only two numeric sizes interpolate (the initial value).
    #[default]
    NumericOnly,
    /// A sizing keyword interpolates with a length too.
    AllowKeywords,
}

/// What `size` stands for in a [`CalcSize`]: the value the sizing keyword
/// resolves to. `#[non_exhaustive]`: CSS Values 5 grows the keywords
/// (`stretch`), and a reader sizes an unknown one as `auto`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CalcSizeBasis {
    /// `auto`: the box's automatic size.
    Auto,
    /// `min-content`, `max-content`, `fit-content`, `fit-content(<l>)`.
    Intrinsic(IntrinsicSize),
}

/// `calc-size(<basis>, <calc-sum>)` (CSS Values 5 §10) whose sum is linear
/// in `size`: `size * factor + offset`, `size` the value `basis`
/// resolves to. Every interpolation between a sizing keyword and a length
/// is one (§11: `calc-size(auto, size * (1 - p) + L * p)`). A `basis` of
/// `any` or of a length folds into a plain size when parsed.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CalcSize {
    pub basis: CalcSizeBasis,
    /// The coefficient of `size`.
    pub factor: f64,
    /// The rest of the sum: a `<length-percentage>` (cells for a bare
    /// number), a percentage resolved as the property's.
    pub offset: CalcExpr,
}

impl CalcSize {
    /// `calc-size(basis, size * factor + offset)`.
    pub fn new(basis: CalcSizeBasis, factor: f64, offset: CalcExpr) -> Self {
        CalcSize {
            basis,
            factor,
            offset,
        }
    }

    /// The size in cells, `size` being `basis_cells` and a percentage in
    /// the offset taking `percent_basis` — whole cells, never negative.
    pub fn resolve(&self, basis_cells: u16, percent_basis: u16) -> u16 {
        let offset = self
            .offset
            .resolve_f64(&crate::calc::ResolveCtx::new(i32::from(percent_basis)));
        let v = self.factor * f64::from(basis_cells) + offset;
        crate::calc::to_cells(v).clamp(0, i32::from(u16::MAX)) as u16
    }

    /// The basis as a size (`auto` or the intrinsic keyword).
    pub fn basis_size(&self) -> super::Size {
        match &self.basis {
            CalcSizeBasis::Auto => super::Size::Auto,
            CalcSizeBasis::Intrinsic(k) => super::Size::Intrinsic(k.clone()),
        }
    }
}
