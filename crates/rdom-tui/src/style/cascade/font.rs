//! The font applicators (CSS Fonts 4): each longhand computes to its
//! declared value into the `ComputedStyle::font` group — `font-weight`'s
//! relative keywords against the parent's weight once the ladder has run —
//! and the bold and italic modifier bits the font draws.

use super::apply::{Keywords, apply_value};
use crate::style::{ComputedStyle, ImportantMask, Modifier, TuiStyle};

/// Apply `style`'s font declarations to `working`, for one ladder pass.
pub(super) fn apply_font(
    working: &mut ComputedStyle,
    style: &TuiStyle,
    important_pass: bool,
    kw: &Keywords<'_>,
) {
    macro_rules! font {
        ($($field:ident: $mask:ident),* $(,)?) => {$(
            apply_value(
                &mut working.font.$field,
                &style.font.$field,
                style.important.contains(ImportantMask::$mask),
                important_pass,
                kw,
                |c| &c.font.$field,
            );
        )*};
    }
    font!(
        weight: FONT_WEIGHT,
        style: FONT_STYLE,
        size: FONT_SIZE,
        family: FONT_FAMILY,
        stretch: FONT_STRETCH,
        variant: FONT_VARIANT,
    );
}

/// `font-weight`'s computed value — `bolder` / `lighter` against the
/// parent's weight (CSS Fonts 4 §2.2) — and the modifier bits the font
/// draws: bold from weight 600, italic for `italic` and a slanted
/// `oblique`.
pub(super) fn finalize_font(working: &mut ComputedStyle, parent: &ComputedStyle) {
    let parent_weight = parent.font.weight();
    let weight = working.font.weight;
    working.font.weight = weight.computed(parent_weight);
    working
        .modifiers
        .set(Modifier::BOLD, weight.is_bold(parent_weight));
    working
        .modifiers
        .set(Modifier::ITALIC, working.font.style.is_italic());
}
