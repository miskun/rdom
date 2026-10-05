//! Keyword-valued properties: the generic keyword-table matcher plus
//! the single-keyword enums (`text-decoration`, `overflow`,
//! `scroll-behavior`, `position`).

use crate::layout::{Overflow, Position};
use crate::parse::token::Token;

pub fn parse_keyword<T: Clone>(value: &[Token], table: &[(&str, T)]) -> Option<T> {
    if value.len() != 1 {
        return None;
    }
    let name = match &value[0] {
        Token::Ident(s) => s.as_str(),
        _ => return None,
    };
    for (k, v) in table {
        if name.eq_ignore_ascii_case(k) {
            return Some(v.clone());
        }
    }
    None
}

/// `text-decoration`: one keyword, `underline | line-through | none`
/// (ASCII case-insensitive, CSS Values 4 §2.1).
pub fn parse_text_decoration(value: &[Token]) -> Option<(bool, bool)> {
    parse_keyword(
        value,
        &[
            ("underline", (true, false)),
            ("line-through", (false, true)),
            ("none", (false, false)),
        ],
    )
}

pub fn parse_overflow(value: &[Token]) -> Option<Overflow> {
    parse_keyword(
        value,
        &[
            ("hidden", Overflow::Hidden),
            ("clip", Overflow::Clip),
            ("scroll", Overflow::Scroll),
            ("auto", Overflow::Auto),
            ("visible", Overflow::Visible),
        ],
    )
}

/// `overflow: <overflow>{1,2}` (CSS Overflow 3 §3.1): `(overflow-x,
/// overflow-y)`, one value for both.
pub fn parse_overflow_shorthand(value: &[Token]) -> Option<(Overflow, Overflow)> {
    match super::numeric::components(value)?.as_slice() {
        [x] => parse_overflow(x).map(|o| (o, o)),
        [x, y] => Some((parse_overflow(x)?, parse_overflow(y)?)),
        _ => None,
    }
}

/// `overflow-clip-margin: <visual-box> || <length [0,∞]>` (CSS Overflow
/// 3 §3.2), the length in whole cells: no percentages, and no viewport
/// units (a length needing the viewport is rejected).
pub fn parse_overflow_clip_margin(value: &[Token]) -> Option<crate::layout::OverflowClipMargin> {
    use super::numeric::{LengthPercentage, Range, components, length_percentage};
    let mut visual_box = None;
    let mut margin = None;
    for part in components(value)? {
        if let Some(b) = super::background::visual_box(part) {
            if visual_box.replace(b).is_some() {
                return None;
            }
            continue;
        }
        let cells = match length_percentage(part, Range::NonNegative)? {
            LengthPercentage::Integer(n) => u16::try_from(n).ok()?,
            LengthPercentage::Cells(v) => super::numeric::cells_u16(v),
            LengthPercentage::Expr(_) => return None,
        };
        if margin.replace(cells).is_some() {
            return None;
        }
    }
    let initial = crate::layout::OverflowClipMargin::default();
    Some(crate::layout::OverflowClipMargin::new(
        visual_box.unwrap_or(initial.visual_box),
        margin.unwrap_or(initial.margin),
    ))
}

/// `text-overflow: [ clip | ellipsis | <string> ]{1,2}` (CSS Overflow 4
/// §3). `fade` / `fade()` — a sub-cell gradient — are rejected.
pub fn parse_text_overflow(value: &[Token]) -> Option<crate::layout::TextOverflow> {
    use crate::layout::{TextOverflow, TextOverflowSide};
    let side = |part: &[Token]| match part {
        [Token::String(s)] => Some(TextOverflowSide::Str(s.clone())),
        _ => parse_keyword(
            part,
            &[
                ("clip", TextOverflowSide::Clip),
                ("ellipsis", TextOverflowSide::Ellipsis),
            ],
        ),
    };
    match super::numeric::components(value)?.as_slice() {
        [end] => Some(TextOverflow::one(side(end)?)),
        [left, right] => Some(TextOverflow::two(side(left)?, side(right)?)),
        _ => None,
    }
}

pub fn parse_scroll_behavior(value: &[Token]) -> Option<crate::layout::ScrollBehavior> {
    use crate::layout::ScrollBehavior;
    parse_keyword(
        value,
        &[
            ("auto", ScrollBehavior::Auto),
            ("smooth", ScrollBehavior::Smooth),
        ],
    )
}

pub fn parse_position(value: &[Token]) -> Option<Position> {
    parse_keyword(
        value,
        &[
            ("static", Position::Static),
            ("relative", Position::Relative),
            ("absolute", Position::Absolute),
            ("fixed", Position::Fixed),
            ("sticky", Position::Sticky),
        ],
    )
}
