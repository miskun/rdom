//! Border values: the `border` shorthand keyword ring, per-side
//! `border-<side>` styles and the current-border read used by the
//! per-side longhands.

use super::keyword::parse_keyword;
use crate::layout::{Border, BorderStyle};
use crate::parse::token::Token;
use crate::{TuiStyle, Value};

/// Read the current border from `style`, defaulting to all-sides-off
/// when nothing is set. Used by the `border-top` / `border-right` /
/// `border-bottom` / `border-left` longhands so consecutive
/// declarations combine instead of overwriting.
pub fn current_border(style: &TuiStyle) -> Border {
    match style.border {
        Some(Value::Specified(b)) => b,
        _ => Border::none(),
    }
}

/// Parse a per-side `border-<side>` or `border-<side>-style` value into
/// a [`BorderStyle`]. Accepts the full CSS keyword set (`none`,
/// `hidden`, `solid`, `double`, `dashed`, `dotted`, `ridge`, `outset`,
/// `groove`, `inset`). `single` and `rounded` are legacy keywords kept
/// for backward compat (both map to `Solid`; `rounded` only affects
/// `CornerStyle` via the shorthand path, not per-side longhands).
/// Unknown values → `None` so the caller emits a warning.
pub fn parse_border_side(value: &[Token]) -> Option<BorderStyle> {
    parse_keyword(
        value,
        &[
            ("none", BorderStyle::None),
            ("hidden", BorderStyle::Hidden),
            ("solid", BorderStyle::Solid),
            ("single", BorderStyle::Solid),
            ("rounded", BorderStyle::Solid),
            ("half-block", BorderStyle::HalfBlock),
            ("double", BorderStyle::Double),
            ("dashed", BorderStyle::Dashed),
            ("dotted", BorderStyle::Dotted),
            ("ridge", BorderStyle::Ridge),
            ("outset", BorderStyle::Outset),
            ("groove", BorderStyle::Groove),
            ("inset", BorderStyle::Inset),
        ],
    )
}

pub fn parse_border(value: &[Token]) -> Option<Border> {
    // CSS `border` shorthand maps to a ring of the same style on all
    // four sides. Single-side legacy keywords (`top`/`bottom`/etc.)
    // produce solid-only-that-side. `rounded` is rdom-specific shape
    // sugar — solid ring + rounded corners.
    parse_keyword(
        value,
        &[
            ("none", Border::none()),
            ("hidden", Border::ring(BorderStyle::Hidden)),
            ("solid", Border::ring(BorderStyle::Solid)),
            ("single", Border::ring(BorderStyle::Solid)),
            ("rounded", Border::rounded()),
            ("half-block", Border::ring(BorderStyle::HalfBlock)),
            ("double", Border::ring(BorderStyle::Double)),
            ("dashed", Border::ring(BorderStyle::Dashed)),
            ("dotted", Border::ring(BorderStyle::Dotted)),
            ("ridge", Border::ring(BorderStyle::Ridge)),
            ("outset", Border::ring(BorderStyle::Outset)),
            ("groove", Border::ring(BorderStyle::Groove)),
            ("inset", Border::ring(BorderStyle::Inset)),
            ("top", Border::top()),
            ("bottom", Border::bottom()),
            ("left", Border::left()),
            ("right", Border::right()),
        ],
    )
}
