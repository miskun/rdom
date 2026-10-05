//! The Box Alignment properties' grammars (CSS Box Alignment 3 §5–§6):
//! each property takes a subset of the shared keywords ([`Align`]), and
//! one parser reads any of them from its grammar
//! (`AlignProperty::grammar`, the table `Alignment::is_valid_for` reads).

use crate::layout::alignment::Grammar;
use crate::layout::{Align, AlignProperty, Alignment, OverflowAlign};
use crate::parse::token::Token;

const JUSTIFY_CONTENT: Grammar = AlignProperty::JustifyContent.grammar();
const ALIGN_CONTENT: Grammar = AlignProperty::AlignContent.grammar();
const ALIGN_ITEMS: Grammar = AlignProperty::AlignItems.grammar();
const JUSTIFY_SELF: Grammar = AlignProperty::JustifySelf.grammar();
const JUSTIFY_ITEMS: Grammar = AlignProperty::JustifyItems.grammar();
const ALIGN_SELF: Grammar = AlignProperty::AlignSelf.grammar();

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
    if g.legacy
        && let Some(legacy) = parse_legacy(value)
    {
        return Some(legacy);
    }
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

/// `legacy`, or `legacy` with `left | right | center` in either order.
fn parse_legacy(value: &[Token]) -> Option<Alignment> {
    let is_legacy = |t: &Token| keyword(t).is_some_and(|k| k.eq_ignore_ascii_case("legacy"));
    let side = |t: &Token| {
        let k = keyword(t)?.to_ascii_lowercase();
        Some(match k.as_str() {
            "left" => Align::Left,
            "right" => Align::Right,
            "center" => Align::Center,
            _ => return None,
        })
    };
    let keyword = match value {
        [one] if is_legacy(one) => Align::Normal,
        [a, b] if is_legacy(a) => side(b)?,
        [a, b] if is_legacy(b) => side(a)?,
        _ => return None,
    };
    Some(Alignment {
        keyword,
        overflow: OverflowAlign::Default,
        legacy: true,
    })
}

/// `justify-content` (CSS Box Alignment 3 §5.2).
pub fn parse_justify_content(value: &[Token]) -> Option<Alignment> {
    parse(value, &JUSTIFY_CONTENT)
}

/// `align-content` (CSS Box Alignment 3 §5.1).
pub fn parse_align_content(value: &[Token]) -> Option<Alignment> {
    parse(value, &ALIGN_CONTENT)
}

/// `align-items` (CSS Box Alignment 3 §6.3).
pub fn parse_align_items(value: &[Token]) -> Option<Alignment> {
    parse(value, &ALIGN_ITEMS)
}

/// `align-self` (CSS Box Alignment 3 §6.1).
pub fn parse_align_self(value: &[Token]) -> Option<Alignment> {
    parse(value, &ALIGN_SELF)
}

/// `justify-self` (CSS Box Alignment 3 §6.1).
pub fn parse_justify_self(value: &[Token]) -> Option<Alignment> {
    parse(value, &JUSTIFY_SELF)
}

/// `justify-items` (CSS Box Alignment 3 §6.2).
pub fn parse_justify_items(value: &[Token]) -> Option<Alignment> {
    parse(value, &JUSTIFY_ITEMS)
}

/// A `place-*` shorthand (§5.5, §6.4, §6.5): `<align> <justify>?`. The
/// first longhand takes the first one or two tokens; an omitted second
/// copies it — a `<baseline-position>` as `start` when `baseline_start`
/// (`place-content`: `justify-content` has no baseline). Returns
/// `(align, justify)`.
fn parse_place(
    value: &[Token],
    align: fn(&[Token]) -> Option<Alignment>,
    justify: fn(&[Token]) -> Option<Alignment>,
    baseline_start: bool,
) -> Option<(Alignment, Alignment)> {
    if let Some(a) = align(value) {
        let copied = if baseline_start && matches!(a.keyword, Align::Baseline | Align::LastBaseline)
        {
            Alignment::new(Align::Start)
        } else {
            a
        };
        return justify(value).or(Some(copied)).map(|j| (a, j));
    }
    (1..value.len()).find_map(|k| Some((align(&value[..k])?, justify(&value[k..])?)))
}

/// `place-content: <'align-content'> <'justify-content'>?` (§5.5).
pub fn parse_place_content(value: &[Token]) -> Option<(Alignment, Alignment)> {
    parse_place(value, parse_align_content, parse_justify_content, true)
}

/// `place-items: <'align-items'> <'justify-items'>?` (§6.4).
pub fn parse_place_items(value: &[Token]) -> Option<(Alignment, Alignment)> {
    parse_place(value, parse_align_items, parse_justify_items, false)
}

/// `place-self: <'align-self'> <'justify-self'>?` (§6.5).
pub fn parse_place_self(value: &[Token]) -> Option<(Alignment, Alignment)> {
    parse_place(value, parse_align_self, parse_justify_self, false)
}

/// A `place-*` shorthand's CSS text: one value when the second is what
/// the first alone would give it (CSSOM §6.7.2's shortest form).
pub fn serialize_place(align: Alignment, justify: Alignment, baseline_start: bool) -> String {
    let implied =
        if baseline_start && matches!(align.keyword, Align::Baseline | Align::LastBaseline) {
            Alignment::new(Align::Start)
        } else {
            align
        };
    if justify == implied {
        serialize_alignment(align)
    } else {
        format!(
            "{} {}",
            serialize_alignment(align),
            serialize_alignment(justify)
        )
    }
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
