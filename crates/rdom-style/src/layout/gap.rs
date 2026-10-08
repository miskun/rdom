//! `row-gap` / `column-gap` (CSS Box Alignment 3 §8.1) and
//! `border-spacing`: the space between items, tracks or cells.

/// A `row-gap` / `column-gap` value (CSS Box Alignment 3 §8.1), also
/// `border-spacing`'s: whole cells, or a `calc()` / percentage that
/// resolves at layout time against the container's content size on the
/// gap's axis (indefinite → 0; `CALC-GAP-1`), or `normal` — the initial
/// value, 0 in flex (and grid) layout.
#[derive(Debug, Clone, PartialEq)]
pub enum GapValue {
    Cells(u16),
    Calc(std::sync::Arc<crate::calc::CalcExpr>),
    /// `normal`: 0 in flex layout (§8.1; a multi-column `1em`, which
    /// rdom has no layout for yet).
    Normal,
}

impl Default for GapValue {
    fn default() -> Self {
        GapValue::Cells(0)
    }
}

impl From<u16> for GapValue {
    fn from(cells: u16) -> Self {
        GapValue::Cells(cells)
    }
}

impl From<crate::calc::CalcExpr> for GapValue {
    fn from(expr: crate::calc::CalcExpr) -> Self {
        GapValue::calc(expr)
    }
}

impl GapValue {
    /// A `calc()` value (CSS Values 4 §10): the expression behind an `Arc`,
    /// shared by every style holding the value — an inherited or copied
    /// one clones without allocating.
    pub fn calc(expr: crate::calc::CalcExpr) -> Self {
        GapValue::Calc(std::sync::Arc::new(expr))
    }

    /// Resolve against `basis` (the container's content size on the
    /// gap's axis; `0` when that size is indefinite).
    pub fn resolve(&self, basis: u16) -> u16 {
        match self {
            GapValue::Cells(n) => *n,
            GapValue::Calc(expr) => {
                let v = expr.resolve(&crate::calc::ResolveCtx::new(i32::from(basis)));
                v.clamp(0, i32::from(u16::MAX)) as u16
            }
            GapValue::Normal => 0,
        }
    }

    /// The value as whole cells when it needs no basis.
    pub fn as_cells(&self) -> Option<u16> {
        match self {
            GapValue::Cells(n) => Some(*n),
            GapValue::Calc(_) => None,
            GapValue::Normal => Some(0),
        }
    }
}
