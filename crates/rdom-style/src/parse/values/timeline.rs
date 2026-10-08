//! The scroll-driven animation values (Scroll-driven Animations 1 §2–§4):
//! timeline names, axes and insets, their shorthands, `timeline-scope`,
//! the `scroll()` / `view()` functions of `animation-timeline`, and the
//! `animation-range` boundaries.

use super::length::parse_length;
use super::numeric::components;
use super::transition::split_on_top_level_commas;
use crate::keyframes::{
    AnimationTimeline, RangeBoundary, TimelineAxis, TimelineInset, TimelineName, TimelineRangeName,
    TimelineScope, TimelineScroller,
};
use crate::layout::Length;
use crate::parse::token::Token;

/// A comma-separated list of `one` (at least one entry).
fn list<T>(value: &[Token], one: impl Fn(&[Token]) -> Option<T>) -> Option<Vec<T>> {
    split_on_top_level_commas(value)
        .into_iter()
        .map(one)
        .collect::<Option<Vec<T>>>()
        .filter(|l| !l.is_empty())
}

/// A `<dashed-ident>` (CSS Values 4 §4.2): `--` and at least one more
/// code point; case-sensitive.
pub(crate) fn dashed_ident(token: &Token) -> Option<std::sync::Arc<str>> {
    match token {
        Token::Ident(s) if s.starts_with("--") && s.len() > 2 => Some(s.as_str().into()),
        _ => None,
    }
}

fn axis(token: &Token) -> Option<TimelineAxis> {
    match token {
        Token::Ident(s) => TimelineAxis::from_keyword(s),
        _ => None,
    }
}

fn name(seg: &[Token]) -> Option<TimelineName> {
    match seg {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("none") => Some(TimelineName::None),
        [t] => dashed_ident(t).map(TimelineName::Named),
        _ => None,
    }
}

/// `scroll-timeline-name` / `view-timeline-name`: `[none | <dashed-ident>]#`.
pub(crate) fn parse_timeline_name_list(value: &[Token]) -> Option<Vec<TimelineName>> {
    list(value, name)
}

/// `scroll-timeline-axis` / `view-timeline-axis`: `[block | inline | x | y]#`.
pub(crate) fn parse_timeline_axis_list(value: &[Token]) -> Option<Vec<TimelineAxis>> {
    list(value, |seg| match seg {
        [t] => axis(t),
        _ => None,
    })
}

/// One inset: `auto | <length-percentage>`.
fn inset_side(seg: &[Token]) -> Option<Length> {
    match seg {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(Length::Auto),
        _ => parse_length(seg).filter(|l| *l != Length::Auto),
    }
}

/// `[auto | <length-percentage>]{1,2}` from its components: one value is
/// both insets.
fn inset(parts: &[&[Token]]) -> Option<TimelineInset> {
    match parts {
        [a] => {
            let a = inset_side(a)?;
            Some(TimelineInset {
                start: a.clone(),
                end: a,
            })
        }
        [a, b] => Some(TimelineInset {
            start: inset_side(a)?,
            end: inset_side(b)?,
        }),
        _ => None,
    }
}

/// `view-timeline-inset: [[auto | <length-percentage>]{1,2}]#`.
pub(crate) fn parse_timeline_inset_list(value: &[Token]) -> Option<Vec<TimelineInset>> {
    list(value, |seg| inset(&components(seg)?))
}

/// One `scroll-timeline` piece: `<name> <axis>?`.
pub(crate) fn parse_scroll_timeline(value: &[Token]) -> Option<Vec<(TimelineName, TimelineAxis)>> {
    list(value, |seg| match seg {
        [n] => Some((name(std::slice::from_ref(n))?, TimelineAxis::Block)),
        [n, a] => Some((name(std::slice::from_ref(n))?, axis(a)?)),
        _ => None,
    })
}

/// One `view-timeline` piece: `<name> [<axis> || <inset>]?`.
pub(crate) fn parse_view_timeline(
    value: &[Token],
) -> Option<Vec<(TimelineName, TimelineAxis, TimelineInset)>> {
    list(value, |seg| {
        let parts = components(seg)?;
        let (first, rest) = parts.split_first()?;
        let name = name(first)?;
        let (axis, inset) = axis_and_inset(rest)?;
        Some((name, axis, inset))
    })
}

/// `[<axis> || <inset>]?` from its components (the axis first or last).
fn axis_and_inset(parts: &[&[Token]]) -> Option<(TimelineAxis, TimelineInset)> {
    let one_axis = |p: &[Token]| match p {
        [t] => axis(t),
        _ => None,
    };
    let auto = || TimelineInset {
        start: Length::Auto,
        end: Length::Auto,
    };
    match parts {
        [] => Some((TimelineAxis::Block, auto())),
        [first, rest @ ..] if one_axis(first).is_some() => {
            let inset = if rest.is_empty() {
                auto()
            } else {
                inset(rest)?
            };
            Some((one_axis(first)?, inset))
        }
        [init @ .., last] if one_axis(last).is_some() => Some((one_axis(last)?, inset(init)?)),
        _ => Some((TimelineAxis::Block, inset(parts)?)),
    }
}

/// `timeline-scope: none | all | <dashed-ident>#` (§4.2).
pub(crate) fn parse_timeline_scope(value: &[Token]) -> Option<TimelineScope> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("none") => Some(TimelineScope::None),
        [Token::Ident(s)] if s.eq_ignore_ascii_case("all") => Some(TimelineScope::All),
        _ => list(value, |seg| match seg {
            [t] => dashed_ident(t),
            _ => None,
        })
        .map(TimelineScope::Names),
    }
}

/// One `animation-timeline` entry (CSS Animations 2 §3.7): `auto`,
/// `none`, a `<dashed-ident>`, `scroll()` or `view()`.
pub(super) fn animation_timeline(seg: &[Token]) -> Option<AnimationTimeline> {
    match seg {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(AnimationTimeline::Auto),
        [Token::Ident(s)] if s.eq_ignore_ascii_case("none") => Some(AnimationTimeline::None),
        [t @ Token::Ident(_)] => dashed_ident(t).map(AnimationTimeline::Named),
        [Token::Function(f), args @ .., Token::RParen] if f.eq_ignore_ascii_case("scroll") => {
            scroll_args(args)
        }
        [Token::Function(f), args @ .., Token::RParen] if f.eq_ignore_ascii_case("view") => {
            let (axis, inset) = axis_and_inset(&components(args).unwrap_or_default())?;
            Some(AnimationTimeline::View { axis, inset })
        }
        _ => None,
    }
}

/// `scroll( [<scroller> || <axis>]? )` (§2.1.1).
fn scroll_args(args: &[Token]) -> Option<AnimationTimeline> {
    let mut scroller = None;
    let mut axis_v = None;
    for t in args {
        let Token::Ident(s) = t else {
            return None;
        };
        if let Some(sc) = TimelineScroller::from_keyword(s) {
            if scroller.replace(sc).is_some() {
                return None;
            }
        } else if let Some(a) = TimelineAxis::from_keyword(s) {
            if axis_v.replace(a).is_some() {
                return None;
            }
        } else {
            return None;
        }
    }
    Some(AnimationTimeline::Scroll {
        scroller: scroller.unwrap_or_default(),
        axis: axis_v.unwrap_or_default(),
    })
}

/// One range boundary: `normal | <length-percentage> |
/// <timeline-range-name> <length-percentage>?` — the name alone at
/// `default` (0% for a start, 100% for an end).
fn boundary(seg: &[Token], default: f64) -> Option<RangeBoundary> {
    match seg {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("normal") => Some(RangeBoundary::Normal),
        [Token::Ident(s), rest @ ..] if TimelineRangeName::from_keyword(s).is_some() => {
            let offset = if rest.is_empty() {
                Length::calc(crate::calc::CalcExpr::Percent(default))
            } else {
                parse_length(rest).filter(|l| *l != Length::Auto)?
            };
            Some(RangeBoundary::Offset {
                name: TimelineRangeName::from_keyword(s),
                offset,
            })
        }
        _ => Some(RangeBoundary::Offset {
            name: None,
            offset: parse_length(seg).filter(|l| *l != Length::Auto)?,
        }),
    }
}

/// `animation-range-start` (§4.3.1).
pub(crate) fn parse_range_start_list(value: &[Token]) -> Option<Vec<RangeBoundary>> {
    list(value, |seg| boundary(seg, 0.0))
}

/// `animation-range-end` (§4.3.2).
pub(crate) fn parse_range_end_list(value: &[Token]) -> Option<Vec<RangeBoundary>> {
    list(value, |seg| boundary(seg, 100.0))
}

/// `animation-range: [<'animation-range-start'> <'animation-range-end'>?]#`
/// (§4.3): the end omitted is the start's range at 100% when the start
/// names one, else `normal`.
pub(crate) fn parse_animation_range(
    value: &[Token],
) -> Option<Vec<(RangeBoundary, RangeBoundary)>> {
    list(value, |seg| {
        let parts = components(seg)?;
        // The start takes a name and its offset, or one component.
        let starts_with_name = matches!(parts.first(), Some([Token::Ident(s)]) if TimelineRangeName::from_keyword(s).is_some());
        let start_len = if starts_with_name
            && parts.get(1).is_some_and(|p| {
                !matches!(p, [Token::Ident(s)] if TimelineRangeName::from_keyword(s).is_some() || s.eq_ignore_ascii_case("normal"))
            }) {
            2
        } else {
            1
        };
        let start_parts = parts.get(..start_len)?;
        let start = boundary(&start_parts.concat(), 0.0)?;
        let rest: Vec<Token> = parts.get(start_len..)?.concat();
        let end = if rest.is_empty() {
            match &start {
                RangeBoundary::Offset { name: Some(n), .. } => RangeBoundary::Offset {
                    name: Some(*n),
                    offset: Length::calc(crate::calc::CalcExpr::Percent(100.0)),
                },
                _ => RangeBoundary::Normal,
            }
        } else {
            boundary(&rest, 100.0)?
        };
        Some((start, end))
    })
}
