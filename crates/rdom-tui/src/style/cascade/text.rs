//! The applicator of the CSS Text properties (CSS Text 3 / 4): each one
//! computes to its declared value, into the `ComputedStyle::text` group.

use super::apply::{Keywords, apply_value};
use crate::style::{ComputedStyle, ImportantMask, TuiStyle};

/// Apply `style`'s CSS Text declarations to `working`, for one ladder
/// pass.
pub(super) fn apply_text(
    working: &mut ComputedStyle,
    style: &TuiStyle,
    important_pass: bool,
    kw: &Keywords<'_>,
) {
    macro_rules! text {
        ($($field:ident: $mask:ident),* $(,)?) => {$(
            apply_value(
                &mut working.text.$field,
                &style.text.$field,
                style.important.contains(ImportantMask::$mask),
                important_pass,
                kw,
                |c| &c.text.$field,
            );
        )*};
    }
    text!(
        white_space_collapse: WHITE_SPACE_COLLAPSE,
        text_wrap_mode: TEXT_WRAP_MODE,
        word_break: WORD_BREAK,
        overflow_wrap: OVERFLOW_WRAP,
        line_break: LINE_BREAK,
        hyphens: HYPHENS,
        tab_size: TAB_SIZE,
    );
}
