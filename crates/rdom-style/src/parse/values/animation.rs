//! `animation-*` values (CSS Animations 1 §4, CSS Animations 2 §3):
//! the comma-separated longhand lists and the `animation` shorthand's
//! pieces. The easing functions are `easing.rs`'s, `<time>` is
//! `transition.rs`'s.

use super::easing::parse_timing_function_at;
use super::transition::{parse_signed_time_ms, parse_time_ms, split_on_top_level_commas};
use crate::keyframes::{
    AnimationComposition, AnimationDirection, AnimationDuration, AnimationFillMode, AnimationName,
    AnimationPlayState, AnimationTimeline, IterationCount,
};
use crate::parse::token::Token;
use crate::transition::TimingFunction;

/// A comma-separated list of `one` (at least one entry).
fn list<T>(value: &[Token], one: impl Fn(&[Token]) -> Option<T>) -> Option<Vec<T>> {
    split_on_top_level_commas(value)
        .into_iter()
        .map(one)
        .collect::<Option<Vec<T>>>()
        .filter(|l| !l.is_empty())
}

/// A `<keyframes-name>` (CSS Animations 1 §3): a `<custom-ident>` — not
/// a CSS-wide keyword, `default` (CSS Values 4 §4.2) or `none` — or a
/// `<string>`. The name keeps its case.
pub fn keyframes_name(token: &Token) -> Option<std::sync::Arc<str>> {
    match token {
        Token::String(s) => Some(s.as_str().into()),
        Token::Ident(s) if is_custom_ident(s) && !s.eq_ignore_ascii_case("none") => {
            Some(s.as_str().into())
        }
        _ => None,
    }
}

/// Whether `s` may be a `<custom-ident>` (CSS Values 4 §4.2): not a
/// CSS-wide keyword nor `default`.
fn is_custom_ident(s: &str) -> bool {
    ![
        "initial",
        "inherit",
        "unset",
        "revert",
        "revert-layer",
        "default",
    ]
    .iter()
    .any(|k| s.eq_ignore_ascii_case(k))
}

/// One `animation-name` entry: `none` or a `<keyframes-name>`.
fn name(seg: &[Token]) -> Option<AnimationName> {
    match seg {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("none") => Some(AnimationName::None),
        [t] => keyframes_name(t).map(AnimationName::Named),
        _ => None,
    }
}

/// `animation-name: [none | <keyframes-name>]#` (§4.1).
pub(crate) fn parse_animation_name_list(value: &[Token]) -> Option<Vec<AnimationName>> {
    list(value, name)
}

/// One `animation-duration`: `auto | <time [0s,∞]>`.
fn duration(seg: &[Token]) -> Option<AnimationDuration> {
    match seg {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(AnimationDuration::Auto),
        _ => parse_time_ms(seg).map(AnimationDuration::Ms),
    }
}

/// `animation-duration: [auto | <time [0s,∞]>]#` (§4.2, CSS Animations
/// 2 §3.3).
pub(crate) fn parse_animation_duration_list(value: &[Token]) -> Option<Vec<AnimationDuration>> {
    list(value, duration)
}

/// One `<single-animation-iteration-count>`: `infinite | <number [0,∞]>`.
fn iteration_count(seg: &[Token]) -> Option<IterationCount> {
    let n = match seg {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("infinite") => {
            return Some(IterationCount::Infinite);
        }
        [Token::Number(n)] => *n as f64,
        [Token::Float(f)] => *f,
        _ => return None,
    };
    (n >= 0.0 && n.is_finite()).then_some(IterationCount::Count(n as f32))
}

/// `animation-iteration-count` (§4.4).
pub(crate) fn parse_iteration_count_list(value: &[Token]) -> Option<Vec<IterationCount>> {
    list(value, iteration_count)
}

/// A list of one keyword type, through its `from_keyword`.
fn keyword_list<T>(value: &[Token], from: impl Fn(&str) -> Option<T>) -> Option<Vec<T>> {
    list(value, |seg| match seg {
        [Token::Ident(s)] => from(s),
        _ => None,
    })
}

/// `animation-direction` (§4.5).
pub(crate) fn parse_direction_list(value: &[Token]) -> Option<Vec<AnimationDirection>> {
    keyword_list(value, AnimationDirection::from_keyword)
}

/// `animation-fill-mode` (§4.8).
pub(crate) fn parse_fill_mode_list(value: &[Token]) -> Option<Vec<AnimationFillMode>> {
    keyword_list(value, AnimationFillMode::from_keyword)
}

/// `animation-play-state` (§4.6).
pub(crate) fn parse_play_state_list(value: &[Token]) -> Option<Vec<AnimationPlayState>> {
    keyword_list(value, AnimationPlayState::from_keyword)
}

/// `animation-composition` (CSS Animations 2 §3.2).
pub(crate) fn parse_composition_list(value: &[Token]) -> Option<Vec<AnimationComposition>> {
    keyword_list(value, AnimationComposition::from_keyword)
}

/// `animation-timeline: <single-animation-timeline>#` (CSS Animations 2
/// §3.7; the entries are `timeline.rs`'s).
pub(crate) fn parse_timeline_list(value: &[Token]) -> Option<Vec<AnimationTimeline>> {
    list(value, super::timeline::animation_timeline)
}

/// One piece of the `animation` shorthand (CSS Animations 1 §4.9): the
/// components it sets, each unset one its longhand's initial value.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct AnimationPiece {
    pub name: Option<AnimationName>,
    pub duration: Option<AnimationDuration>,
    pub timing: Option<TimingFunction>,
    pub delay: Option<i32>,
    pub iterations: Option<IterationCount>,
    pub direction: Option<AnimationDirection>,
    pub fill: Option<AnimationFillMode>,
    pub play_state: Option<AnimationPlayState>,
}

/// `animation: <single-animation>#`.
pub(crate) fn parse_animation_shorthand(value: &[Token]) -> Option<Vec<AnimationPiece>> {
    list(value, animation_piece)
}

/// One `<single-animation>`: its components in any order. A `<time>`
/// is the duration, then the delay; a keyword that could be several
/// components' goes to the first of them still unset, the name last
/// (§4.9: "a value which is a keyword for another property is treated
/// as that property's unless it was already set").
fn animation_piece(value: &[Token]) -> Option<AnimationPiece> {
    fn set<T>(slot: &mut Option<T>, v: T) -> Option<()> {
        if slot.is_some() {
            return None;
        }
        *slot = Some(v);
        Some(())
    }
    let mut p = AnimationPiece::default();
    let mut i = 0;
    while i < value.len() {
        // A `<time>`: one dimension, after a `-` or a `+` sign.
        let time_len = match (value.get(i), value.get(i + 1)) {
            (Some(Token::Delim('-' | '+')), Some(Token::Dimension { .. })) => 2,
            (Some(Token::Dimension { .. }), _) => 1,
            _ => 0,
        };
        if time_len > 0 {
            let seg = &value[i..i + time_len];
            if p.duration.is_none() {
                p.duration = Some(AnimationDuration::Ms(parse_time_ms(seg)?));
            } else {
                set(&mut p.delay, parse_signed_time_ms(seg)?)?;
            }
            i += time_len;
            continue;
        }
        if p.timing.is_none()
            && let Some((t, used)) = parse_timing_function_at(value, i)
        {
            p.timing = Some(t);
            i += used;
            continue;
        }
        let token = &value[i];
        i += 1;
        if let Some(n) = iteration_count(std::slice::from_ref(token))
            && p.iterations.is_none()
        {
            p.iterations = Some(n);
            continue;
        }
        let Token::Ident(kw) = token else {
            set(&mut p.name, AnimationName::Named(keyframes_name(token)?))?;
            continue;
        };
        if kw.eq_ignore_ascii_case("auto") && p.duration.is_none() {
            p.duration = Some(AnimationDuration::Auto);
        } else if let Some(d) =
            AnimationDirection::from_keyword(kw).filter(|_| p.direction.is_none())
        {
            p.direction = Some(d);
        } else if let Some(f) = AnimationFillMode::from_keyword(kw).filter(|_| p.fill.is_none()) {
            p.fill = Some(f);
        } else if let Some(s) =
            AnimationPlayState::from_keyword(kw).filter(|_| p.play_state.is_none())
        {
            p.play_state = Some(s);
        } else {
            set(&mut p.name, name(std::slice::from_ref(token))?)?;
        }
    }
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::token::tokenize;

    fn piece(src: &str) -> Option<AnimationPiece> {
        animation_piece(&tokenize(src).unwrap())
    }

    /// §4.9: a negative `<time>` can only be the delay.
    #[test]
    fn a_negative_first_time_is_invalid() {
        assert!(piece("-1s slide").is_none());
        assert_eq!(piece("1s -1s slide").and_then(|p| p.delay), Some(-1000));
    }

    /// `none` alone is a fill mode first — the name stays `none` either way.
    #[test]
    fn none_fills_first() {
        let p = piece("none").unwrap();
        assert_eq!(p.fill, Some(AnimationFillMode::None));
        assert_eq!(p.name, None);
        let p = piece("none none").unwrap();
        assert_eq!(p.name, Some(AnimationName::None));
    }
}
