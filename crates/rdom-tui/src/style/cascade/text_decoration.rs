//! The text decoration applicators (CSS Text Decoration 4 §2): the
//! `text-decoration` longhands — the color resolves with the other colors
//! (`colors.rs`) — and the decorations an element's text is drawn with,
//! propagated from its ancestors (§2.1).

use super::keywords::{Keywords, apply_value};
use crate::layout::{AppliedDecorations, Float, Position};
use crate::style::{ComputedStyle, ImportantMask, TuiStyle};

/// Apply `style`'s `text-decoration-line`, `-style` and `-thickness` to
/// `working`, for one ladder pass. None inherits.
pub(super) fn apply_text_decoration(
    working: &mut ComputedStyle,
    style: &TuiStyle,
    important_pass: bool,
    kw: &Keywords<'_>,
) {
    let d = &style.text_decoration;
    let important = |mask| style.important.contains(mask);
    apply_value(
        &mut working.text_decoration.line,
        &d.line,
        important(ImportantMask::TEXT_DECORATION_LINE),
        important_pass,
        kw,
        |c| &c.text_decoration.line,
    );
    apply_value(
        &mut working.text_decoration.style,
        &d.style,
        important(ImportantMask::TEXT_DECORATION_STYLE),
        important_pass,
        kw,
        |c| &c.text_decoration.style,
    );
    apply_value(
        &mut working.text_decoration.thickness,
        &d.thickness,
        important(ImportantMask::TEXT_DECORATION_THICKNESS),
        important_pass,
        kw,
        |c| &c.text_decoration.thickness,
    );
}

/// The decorations drawn on the text of a box styled `working` whose
/// parent box's text is drawn with `parent`'s (CSS Text Decoration 4
/// §2.1): its own over the propagated ones — none propagated into an
/// atomic inline (its contents are "not affected") or an out-of-flow box
/// (a float, an absolutely positioned box). Runs once the box's
/// `display`, `float` and `position` are final.
pub(super) fn finalize_applied_decorations(
    working: &mut ComputedStyle,
    parent: AppliedDecorations,
) {
    let out_of_flow = working.float != Float::None
        || matches!(working.position, Position::Absolute | Position::Fixed);
    let atomic = working.is_atomic_inline();
    let propagated = if out_of_flow || atomic {
        AppliedDecorations::NONE
    } else {
        parent
    };
    working.applied_decorations = propagated.with(&working.text_decoration);
}
