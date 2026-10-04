//! The applicators of the background and border properties whose
//! computed value is not their declared value as written (CSS
//! Backgrounds 3): `background-clip`, whose final layer clips the
//! color; the border styles and widths, which make the used border.

use super::apply::{Keywords, Resolved, matches_pass};
use crate::style::{ComputedStyle, ImportantMask, TuiStyle, Value};
use rdom_style::layout::{BorderWidth, Sides, VisualBox};

/// Each side's computed `border-*-width`, for the CSS-wide keywords'
/// source styles.
const BORDER_WIDTH_FIELDS: Sides<fn(&ComputedStyle) -> &BorderWidth> = Sides::new(
    |c| &c.border_width.top,
    |c| &c.border_width.right,
    |c| &c.border_width.bottom,
    |c| &c.border_width.left,
);

/// Each side's `border-*-width` `!important` bit.
const BORDER_WIDTH_MASKS: Sides<ImportantMask> = Sides::new(
    ImportantMask::BORDER_TOP_WIDTH,
    ImportantMask::BORDER_RIGHT_WIDTH,
    ImportantMask::BORDER_BOTTOM_WIDTH,
    ImportantMask::BORDER_LEFT_WIDTH,
);

/// The used border (CSS Backgrounds 3 §4.3): the cascaded styles with
/// every zero-width side removed. Runs once the ladder has run and the
/// viewport units are resolved.
pub(super) fn finalize_used_border(working: &mut ComputedStyle) {
    working.border = working.border_style.with_widths(&working.border_width);
}

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
    apply_mapped(
        &mut working.border_style,
        &style.border,
        style.important.contains(ImportantMask::BORDER),
        important_pass,
        kw,
        |c| &c.border_style,
        |b| *b,
    );
    let widths = working
        .border_width
        .each_mut()
        .into_iter()
        .zip(style.border_width.each())
        .zip(BORDER_WIDTH_FIELDS.to_array())
        .zip(BORDER_WIDTH_MASKS.to_array());
    for (((target, value), field), mask) in widths {
        apply_mapped(
            target,
            value,
            style.important.contains(mask),
            important_pass,
            kw,
            field,
            BorderWidth::clone,
        );
    }
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
