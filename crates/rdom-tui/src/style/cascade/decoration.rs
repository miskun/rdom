//! The applicators of the background and border properties whose
//! computed value is not their declared value as written (CSS
//! Backgrounds 3): `background-clip`, whose final layer clips the
//! color.

use super::apply::{Keywords, Resolved, matches_pass};
use crate::style::{ComputedStyle, ImportantMask, TuiStyle, Value};
use rdom_style::layout::VisualBox;

/// Apply one block's background / border declarations for one ladder
/// pass.
pub(super) fn apply_decoration(
    working: &mut ComputedStyle,
    style: &TuiStyle,
    important_pass: bool,
    kw: &Keywords<'_>,
) {
    apply_mapped(
        &mut working.background_clip,
        &style.background_clip,
        style.important.contains(ImportantMask::BACKGROUND_CLIP),
        important_pass,
        kw,
        |c| &c.background_clip,
        // §3.2: the background color is clipped by the bottom-most
        // (final) layer's clip.
        |layers: &Vec<VisualBox>| layers.last().copied().unwrap_or_default(),
    );
}

/// [`super::apply`]'s keyword resolution for a property whose computed
/// field is derived from the declared value by `compute`.
fn apply_mapped<S, T: Clone>(
    target: &mut T,
    value: &Option<Value<S>>,
    important_prop: bool,
    important_pass: bool,
    kw: &Keywords<'_>,
    field: fn(&ComputedStyle) -> &T,
    compute: impl FnOnce(&S) -> T,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        *target = match kw.resolve(v) {
            Resolved::Specified(x) => compute(x),
            Resolved::From(source) => field(source).clone(),
        };
    }
}
