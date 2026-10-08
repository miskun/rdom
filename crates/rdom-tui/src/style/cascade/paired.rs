//! The paired defaults of `::selection` (CSS Pseudo-Elements 4 §3.4):
//! the UA's highlight `color` and `background-color` are used as a pair —
//! "if the author specifies either, the UA's default for the other is not
//! used" — so `::selection { background-color: yellow }` keeps the text's
//! own color instead of drawing the UA's white over yellow.
//!
//! Run on the cascaded `::selection` style: a pair half the author (or a
//! hint or an inline style, any origin above the UA's) left to the UA is
//! reset to what a highlight that does not set it paints with — `color`
//! the originating element's (highlight inheritance's root, §3.5), and
//! `background-color` transparent.

use super::ladder::{Declarations, Plan, Source};
use crate::style::{ComputedStyle, TuiStyle};

/// Whether `block` declares `color` (`fg`) and `background-color` (`bg`),
/// directly or through a `var()` it holds for later.
fn declares(block: &TuiStyle) -> (bool, bool) {
    let pending = |names: &[&str]| {
        block
            .pending
            .iter()
            .any(|d| names.contains(&d.name.as_str()))
    };
    (
        block.fg.is_some() || pending(&["color"]),
        block.bg.is_some() || pending(&["background-color", "background"]),
    )
}

/// Drop the UA's half of `::selection`'s color pair from `working` when
/// the declarations above the UA origin set only the other half.
pub(super) fn unpair(
    working: &mut ComputedStyle,
    plan: &Plan,
    decls: Declarations<'_>,
    host: &ComputedStyle,
) {
    let (mut fg, mut bg) = (false, false);
    for step in plan
        .steps()
        .iter()
        .filter(|s| s.source != Source::UserAgent)
    {
        for block in decls.of(step) {
            let (f, b) = declares(block);
            fg |= f;
            bg |= b;
        }
    }
    if fg && !bg {
        working.bg = crate::style::Color::TRANSPARENT;
    } else if bg && !fg {
        working.fg = host.fg;
    }
}
