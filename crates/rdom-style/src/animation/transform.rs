//! Interpolation of the transform values (CSS Transforms 2 §11 "Transform
//! function primitives and derivatives", §12 "Interpolation of
//! transforms", §15 "Addition of transforms").
//!
//! `translate` interpolates component-wise and `none` as zero; `rotate`
//! by its angle about a shared axis; `scale` factor by factor, `none` as
//! one. Two `transform` lists whose functions pair up — the same length,
//! each pair a translation or one inert function — interpolate function
//! by function, `none` as the other list's identities; any other pair
//! would interpolate by its matrices (§12.2), which a grid has only the
//! translation of: rdom interpolates the lists' summed translations and
//! switches their inert functions at the midpoint (DIVERGENCES §2).
//! Addition: translations and angles sum, factors multiply, lists
//! concatenate.

use super::length::add_lp;
use super::value::{Animate, Cx, discrete, lerp};
use crate::layout::{
    PaintLength, Rotate, Scale, TransformFunction, TransformList, TransformOrigin, Translate,
    TranslateFunction,
};

/// A z offset, summed (no common measure between cells and pixels).
fn add_depth(a: &PaintLength, b: &PaintLength) -> Option<PaintLength> {
    match (a, b) {
        (PaintLength::Cells(x), PaintLength::Cells(y)) => Some(PaintLength::Cells(x + y)),
        (PaintLength::Px(x), PaintLength::Px(y)) => Some(PaintLength::Px(x + y)),
        _ => None,
    }
}

impl Animate for Translate {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        Some(Translate {
            x: self.x.animate(&to.x, p, cx)?,
            y: self.y.animate(&to.y, p, cx)?,
            z: self.z.animate(&to.z, p, cx)?,
        })
    }
    fn add(&self, other: &Self, _: &Cx) -> Option<Self> {
        Some(Translate {
            x: add_lp(&self.x, &other.x)?,
            y: add_lp(&self.y, &other.y)?,
            z: add_depth(&self.z, &other.z)?,
        })
    }
}

/// `translate`: `none` is a zero translation (Transforms 2 §6.1).
impl Animate for Option<Translate> {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        if self.is_none() && to.is_none() {
            return Some(None);
        }
        let (a, b) = (
            self.clone().unwrap_or_default(),
            to.clone().unwrap_or_default(),
        );
        Some(Some(a.animate(&b, p, cx)?))
    }
    fn add(&self, other: &Self, cx: &Cx) -> Option<Self> {
        match (self, other) {
            (Some(a), Some(b)) => Some(Some(a.add(b, cx)?)),
            (a, None) => Some(a.clone()),
            (None, b) => Some(b.clone()),
        }
    }
}

/// `rotate` (Transforms 2 §6.2): about one axis the angle interpolates —
/// `none` as no turn about the other's — two axes do not (rdom draws no
/// rotation, so their slerp, §6.2's, would change nothing a cell shows).
impl Animate for Option<Rotate> {
    fn animate(&self, to: &Self, p: f64, _: &Cx) -> Option<Self> {
        let zero = |r: &Rotate| Rotate { degrees: 0.0, ..*r };
        let (a, b) = match (self, to) {
            (None, None) => return Some(None),
            (Some(a), Some(b)) => (*a, *b),
            (None, Some(b)) => (zero(b), *b),
            (Some(a), None) => (*a, zero(a)),
        };
        (a.axis == b.axis).then(|| {
            Some(Rotate {
                axis: a.axis,
                degrees: lerp(a.degrees, b.degrees, p),
            })
        })
    }
    fn add(&self, other: &Self, _: &Cx) -> Option<Self> {
        match (self, other) {
            (Some(a), Some(b)) if a.axis == b.axis => Some(Some(Rotate {
                axis: a.axis,
                degrees: a.degrees + b.degrees,
            })),
            (Some(_), Some(_)) => None,
            (a, None) => Some(*a),
            (None, b) => Some(*b),
        }
    }
}

/// `scale` (§6.3): factor by factor, `none` as 1; factors multiply.
impl Animate for Option<Scale> {
    fn animate(&self, to: &Self, p: f64, _: &Cx) -> Option<Self> {
        if self.is_none() && to.is_none() {
            return Some(None);
        }
        let one = Scale {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        };
        let (a, b) = (self.unwrap_or(one), to.unwrap_or(one));
        Some(Some(Scale {
            x: lerp(a.x, b.x, p),
            y: lerp(a.y, b.y, p),
            z: lerp(a.z, b.z, p),
        }))
    }
    fn add(&self, other: &Self, _: &Cx) -> Option<Self> {
        Some(match (self, other) {
            (Some(a), Some(b)) => Some(Scale {
                x: a.x * b.x,
                y: a.y * b.y,
                z: a.z * b.z,
            }),
            (a, None) => *a,
            (None, b) => *b,
        })
    }
}

/// Two translate functions interpolated: the common spelling, else the
/// 3D one when either moves on z, else `translate()` (§11).
fn translate_pair(
    a: (TranslateFunction, &Translate),
    b: (TranslateFunction, &Translate),
    p: f64,
    cx: &Cx,
) -> Option<TransformFunction> {
    let function = if a.0 == b.0 {
        a.0
    } else if matches!(
        a.0,
        TranslateFunction::Translate3d | TranslateFunction::TranslateZ
    ) || matches!(
        b.0,
        TranslateFunction::Translate3d | TranslateFunction::TranslateZ
    ) {
        TranslateFunction::Translate3d
    } else {
        TranslateFunction::Translate
    };
    Some(TransformFunction::Translate {
        function,
        offset: a.1.animate(b.1, p, cx)?,
    })
}

/// Whether two functions pair up for function-by-function interpolation:
/// two translations, or two inert functions of one name.
fn pairs(a: &TransformFunction, b: &TransformFunction) -> bool {
    match (a, b) {
        (TransformFunction::Translate { .. }, TransformFunction::Translate { .. }) => true,
        (TransformFunction::Inert { name: x, .. }, TransformFunction::Inert { name: y, .. }) => {
            x == y
        }
        _ => false,
    }
}

/// One paired step: a translation interpolated, a missing side its
/// identity; an inert function kept — the one shown at `p` when the two
/// differ.
fn step(
    a: Option<&TransformFunction>,
    b: Option<&TransformFunction>,
    p: f64,
    cx: &Cx,
) -> Option<TransformFunction> {
    use TransformFunction::{Inert, Translate as T};
    let zero = Translate::default();
    match (a, b) {
        (
            Some(T {
                function: f,
                offset: o,
            }),
            Some(T {
                function: g,
                offset: q,
            }),
        ) => translate_pair((*f, o), (*g, q), p, cx),
        (Some(T { function, offset }), None) => {
            translate_pair((*function, offset), (*function, &zero), p, cx)
        }
        (None, Some(T { function, offset })) => {
            translate_pair((*function, &zero), (*function, offset), p, cx)
        }
        (Some(x @ Inert { .. }), Some(y @ Inert { .. })) => Some(discrete(x, y, p)),
        (Some(x @ Inert { .. }), None) | (None, Some(x @ Inert { .. })) => Some(x.clone()),
        _ => None,
    }
}

/// The list's translations summed into one offset (percentages kept).
fn summed(list: &TransformList) -> Option<Translate> {
    list.functions()
        .iter()
        .try_fold(Translate::default(), |sum, f| match f {
            TransformFunction::Translate { offset, .. } => Some(Translate {
                x: add_lp(&sum.x, &offset.x)?,
                y: add_lp(&sum.y, &offset.y)?,
                z: add_depth(&sum.z, &offset.z).unwrap_or(sum.z),
            }),
            TransformFunction::Inert { .. } => Some(sum),
        })
}

/// `transform` (Transforms 2 §12): see the module documentation.
impl Animate for TransformList {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        let (a, b) = (self.functions(), to.functions());
        if a.is_empty() && b.is_empty() {
            return Some(TransformList::none());
        }
        let paired = a.is_empty()
            || b.is_empty()
            || (a.len() == b.len() && a.iter().zip(b).all(|(x, y)| pairs(x, y)));
        if paired {
            let n = a.len().max(b.len());
            let functions = (0..n)
                .map(|i| step(a.get(i), b.get(i), p, cx))
                .collect::<Option<Vec<_>>>()?;
            return Some(TransformList::new(functions));
        }
        // §12.2's matrix interpolation, as much of it as a grid draws: the
        // summed translations move, the inert functions switch.
        let offset = summed(self)?.animate(&summed(to)?, p, cx)?;
        let shown = if p < 0.5 { self } else { to };
        let inert = shown
            .functions()
            .iter()
            .filter(|f| matches!(f, TransformFunction::Inert { .. }))
            .cloned();
        let functions = std::iter::once(TransformFunction::Translate {
            function: TranslateFunction::Translate,
            offset,
        })
        .chain(inert)
        .collect();
        Some(TransformList::new(functions))
    }
    /// §15: two lists add by concatenation.
    fn add(&self, other: &Self, _: &Cx) -> Option<Self> {
        let functions: Vec<TransformFunction> = self
            .functions()
            .iter()
            .chain(other.functions())
            .cloned()
            .collect();
        Some(TransformList::new(functions))
    }
}

/// `transform-origin` (Transforms 1 §6): by computed value, each
/// position as a length-percentage (cells and pixels do not mix).
impl Animate for TransformOrigin {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        Some(TransformOrigin::new(
            self.x().animate(&to.x(), p, cx)?,
            self.y().animate(&to.y(), p, cx)?,
            self.z().animate(&to.z(), p, cx)?,
        ))
    }
}
