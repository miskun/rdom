//! The applicator of the CSS Text properties (CSS Text 3 / 4): each one
//! computes to its declared value, into the `ComputedStyle::text` group.

use super::apply::{Keywords, apply_value};
use crate::layout::{TextAlign, TextAlignLast, TextDirection};
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
        text_transform: TEXT_TRANSFORM,
        text_indent: TEXT_INDENT,
        text_align_all: TEXT_ALIGN_ALL,
        text_align_last: TEXT_ALIGN_LAST,
        text_justify: TEXT_JUSTIFY,
        text_wrap_style: TEXT_WRAP_STYLE,
    );
}

/// `text-align-all` / `text-align-last: match-parent`'s computed value
/// (CSS Text 3 §6.1): the parent's, `start` / `end` resolved against the
/// parent's `direction` to `left` / `right`; on the root element (no
/// element parent), `start`.
pub(super) fn finalize_text_align(working: &mut ComputedStyle, parent: &ComputedStyle, root: bool) {
    let rtl = parent.text_direction == TextDirection::Rtl;
    let text = &mut working.text;
    if text.text_align_all == TextAlign::MatchParent {
        text.text_align_all = if root {
            TextAlign::Start
        } else {
            parent.text.text_align_all.physical(rtl)
        };
    }
    if text.text_align_last == TextAlignLast::MatchParent {
        text.text_align_last = if root {
            TextAlignLast::Start
        } else {
            parent
                .text
                .text_align_last
                .align()
                .map_or(TextAlignLast::Auto, |a| TextAlignLast::of(a.physical(rtl)))
        };
    }
}
