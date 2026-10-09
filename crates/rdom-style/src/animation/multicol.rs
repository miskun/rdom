//! Interpolation of the multi-column and fragmentation values (CSS
//! Multi-column 1 §3, CSS Fragmentation 3 §3.3): `column-count` and
//! `column-width` by computed value, `auto` discrete; `orphans` and
//! `widows` as integers.

use super::value::{Animate, Cx};
use crate::layout::{ColumnCount, ColumnWidth};

/// An `<integer>` interpolates as a real number and rounds, a half up
/// (CSS Values 4 §3.2); `orphans` / `widows` stay at least 1.
impl Animate for u32 {
    fn animate(&self, to: &Self, p: f64, _: &Cx) -> Option<Self> {
        let v = super::value::lerp(f64::from(*self), f64::from(*to), p);
        Some(((v + 0.5).floor()).clamp(1.0, f64::from(u32::MAX)) as u32)
    }
    fn add(&self, other: &Self, _: &Cx) -> Option<Self> {
        Some(self.saturating_add(*other))
    }
}

/// Two counts interpolate as integers; `auto` steps.
impl Animate for ColumnCount {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (ColumnCount::Count(a), ColumnCount::Count(b)) => {
                a.animate(b, p, cx).map(ColumnCount::Count)
            }
            _ => None,
        }
    }
}

/// Two widths interpolate in whole cells; `auto` steps.
impl Animate for ColumnWidth {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self.cells(), to.cells()) {
            (Some(a), Some(b)) if *self != ColumnWidth::Auto && *to != ColumnWidth::Auto => {
                a.animate(&b, p, cx).map(ColumnWidth::Cells)
            }
            _ => None,
        }
    }
}
