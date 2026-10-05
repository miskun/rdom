//! The Box Alignment properties' grammars (CSS Box Alignment 3 §5–§6):
//! each property takes a subset of the shared keywords ([`Align`]), and
//! one parser reads any of them from its [`Grammar`].

use crate::layout::{Align, Alignment, OverflowAlign};
use crate::parse::token::Token;

/// Which keywords a Box Alignment property takes.
struct Grammar {
    /// `auto` (the `*-self` properties).
    auto: bool,
    /// `<baseline-position>`: `baseline`, `first baseline`, `last baseline`.
    baseline: bool,
    /// `<content-distribution>`: `space-between | space-around |
    /// space-evenly | stretch`.
    distribution: bool,
    /// `<self-position>`'s `self-start | self-end` beside the
    /// `<content-position>`s `center | start | end | flex-start | flex-end`.
    self_positions: bool,
    /// `left | right` (the inline-axis properties).
    left_right: bool,
}

/// `justify-content: normal | <content-distribution> |
/// <overflow-position>? [ <content-position> | left | right ]` (§5.2).
const JUSTIFY_CONTENT: Grammar = Grammar {
    auto: false,
    baseline: false,
    distribution: true,
    self_positions: false,
    left_right: true,
};

fn keyword(token: &Token) -> Option<&str> {
    match token {
        Token::Ident(s) => Some(s.as_str()),
        _ => None,
    }
}

/// A positional keyword (`<content-position>`, `<self-position>`,
/// `left` / `right`) the grammar takes.
fn position(name: &str, g: &Grammar) -> Option<Align> {
    let lower = name.to_ascii_lowercase();
    Some(match lower.as_str() {
        "center" => Align::Center,
        "start" => Align::Start,
        "end" => Align::End,
        "flex-start" => Align::FlexStart,
        "flex-end" => Align::FlexEnd,
        "self-start" if g.self_positions => Align::SelfStart,
        "self-end" if g.self_positions => Align::SelfEnd,
        "left" if g.left_right => Align::Left,
        "right" if g.left_right => Align::Right,
        _ => return None,
    })
}

/// A one-keyword value that is not a position.
fn single(name: &str, g: &Grammar) -> Option<Align> {
    let lower = name.to_ascii_lowercase();
    Some(match lower.as_str() {
        "normal" => Align::Normal,
        "stretch" => Align::Stretch,
        "auto" if g.auto => Align::Auto,
        "baseline" if g.baseline => Align::Baseline,
        "space-between" if g.distribution => Align::SpaceBetween,
        "space-around" if g.distribution => Align::SpaceAround,
        "space-evenly" if g.distribution => Align::SpaceEvenly,
        _ => return None,
    })
}

fn parse(value: &[Token], g: &Grammar) -> Option<Alignment> {
    match value {
        [one] => {
            let name = keyword(one)?;
            single(name, g)
                .or_else(|| position(name, g))
                .map(Alignment::new)
        }
        [first, second] => {
            let (a, b) = (keyword(first)?, keyword(second)?);
            if g.baseline && b.eq_ignore_ascii_case("baseline") {
                return if a.eq_ignore_ascii_case("first") {
                    Some(Alignment::new(Align::Baseline))
                } else if a.eq_ignore_ascii_case("last") {
                    Some(Alignment::new(Align::LastBaseline))
                } else {
                    None
                };
            }
            let overflow = if a.eq_ignore_ascii_case("safe") {
                OverflowAlign::Safe
            } else if a.eq_ignore_ascii_case("unsafe") {
                OverflowAlign::Unsafe
            } else {
                return None;
            };
            Some(Alignment {
                keyword: position(b, g)?,
                overflow,
                legacy: false,
            })
        }
        _ => None,
    }
}

/// `justify-content` (CSS Box Alignment 3 §5.2).
pub fn parse_justify_content(value: &[Token]) -> Option<Alignment> {
    parse(value, &JUSTIFY_CONTENT)
}

/// An [`Align`] keyword's CSS text.
pub fn align_keyword(keyword: Align) -> &'static str {
    match keyword {
        Align::Normal => "normal",
        Align::Auto => "auto",
        Align::Stretch => "stretch",
        Align::Start => "start",
        Align::End => "end",
        Align::Center => "center",
        Align::FlexStart => "flex-start",
        Align::FlexEnd => "flex-end",
        Align::SelfStart => "self-start",
        Align::SelfEnd => "self-end",
        Align::Left => "left",
        Align::Right => "right",
        Align::SpaceBetween => "space-between",
        Align::SpaceAround => "space-around",
        Align::SpaceEvenly => "space-evenly",
        Align::Baseline => "baseline",
        Align::LastBaseline => "last baseline",
    }
}

/// A Box Alignment value's CSS text, in its shortest form (CSSOM
/// §6.7.2): `first baseline` is `baseline`; an overflow position leads
/// (`safe center`); `legacy` leads its keyword (`legacy left`), alone for
/// `normal`.
pub fn serialize_alignment(value: Alignment) -> String {
    let keyword = align_keyword(value.keyword);
    let prefix = match value.overflow {
        OverflowAlign::Default => None,
        OverflowAlign::Safe => Some("safe"),
        OverflowAlign::Unsafe => Some("unsafe"),
    };
    match (value.legacy, prefix) {
        (true, _) if value.keyword == Align::Normal => "legacy".to_string(),
        (true, _) => format!("legacy {keyword}"),
        (false, Some(p)) => format!("{p} {keyword}"),
        (false, None) => keyword.to_string(),
    }
}
