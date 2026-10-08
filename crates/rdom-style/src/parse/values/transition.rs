//! Transition values: property names, `<time>` durations and delays,
//! `transition-behavior`, the comma-separated longhand lists and the
//! `transition` shorthand. The easing functions are `easing.rs`.

use super::easing::parse_timing_function_at;
use crate::parse::token::Token;
use crate::transition::{TimingFunction, TransitionBehavior, TransitionProperty};

/// Parse a single property keyword: `all`, `none`, or any
/// `<custom-ident>` (CSS Transitions 1 §2.1) — a property the dispatch
/// table knows is [`TransitionProperty::Named`], any other name (a custom
/// property, an unknown one) is valid and kept as
/// [`TransitionProperty::Other`].
pub fn parse_transition_property_keyword(name: &str) -> Option<TransitionProperty> {
    Some(TransitionProperty::named(name))
}

/// A `<transition-behavior-value>` keyword (CSS Transitions 2 §3.1).
fn behavior_keyword(name: &str) -> Option<TransitionBehavior> {
    match name.to_ascii_lowercase().as_str() {
        "normal" => Some(TransitionBehavior::Normal),
        "allow-discrete" => Some(TransitionBehavior::AllowDiscrete),
        _ => None,
    }
}

/// `transition-behavior: <transition-behavior-value>#`.
pub fn parse_transition_behavior_list(value: &[Token]) -> Option<Vec<TransitionBehavior>> {
    split_on_top_level_commas(value)
        .into_iter()
        .map(|seg| match seg {
            [Token::Ident(s)] => behavior_keyword(s),
            _ => None,
        })
        .collect()
}

/// Parse a comma-separated list of property keywords.
pub fn parse_transition_property_list(value: &[Token]) -> Option<Vec<TransitionProperty>> {
    let segments = split_on_top_level_commas(value);
    let mut out = Vec::with_capacity(segments.len());
    for seg in segments {
        let name = match seg {
            [Token::Ident(s)] => s.as_str(),
            _ => return None,
        };
        out.push(parse_transition_property_keyword(name)?);
    }
    // §2.1: `none` is valid only as the whole value.
    if out.len() > 1 && out.contains(&TransitionProperty::None) {
        return None;
    }
    if out.is_empty() { None } else { Some(out) }
}

/// Parse a comma-separated list of `<time>` values into ms, any sign —
/// `transition-delay` (CSS Transitions 1 §2.4: a negative delay starts
/// the transition part-way).
pub fn parse_time_list(value: &[Token]) -> Option<Vec<i32>> {
    let segments = split_on_top_level_commas(value);
    let mut out = Vec::with_capacity(segments.len());
    for seg in segments {
        out.push(parse_signed_time_ms(seg)?);
    }
    if out.is_empty() { None } else { Some(out) }
}

/// Parse a comma-separated list of `<time [0s,∞]>` values into ms —
/// `transition-duration` (§2.2: a negative duration is invalid).
pub fn parse_duration_list(value: &[Token]) -> Option<Vec<u32>> {
    let segments = split_on_top_level_commas(value);
    let mut out = Vec::with_capacity(segments.len());
    for seg in segments {
        out.push(parse_time_ms(seg)?);
    }
    if out.is_empty() { None } else { Some(out) }
}

/// Parse a single `<time>` value (`200ms`, `0.5s`, `1.05s`, `0s`) into
/// whole milliseconds, rounding half away from zero. The literal is one
/// `Dimension` token (CSS Syntax 3 §4.3.3).
pub fn parse_time_ms(tokens: &[Token]) -> Option<u32> {
    // CSS Syntax 3 §4.3.13: the numeric part may carry a `+` sign.
    let (n, unit) = match tokens {
        [Token::Dimension { value, unit, .. }]
        | [Token::Delim('+'), Token::Dimension { value, unit, .. }] => (*value, unit),
        _ => return None,
    };
    if n < 0.0 {
        return None;
    }
    // Unit identifiers are ASCII case-insensitive (CSS Values 4 §2.1).
    let ms = if unit.eq_ignore_ascii_case("ms") {
        n
    } else if unit.eq_ignore_ascii_case("s") {
        n * 1000.0
    } else {
        return None;
    };
    let ms = ms.round();
    (ms <= f64::from(u32::MAX)).then_some(ms as u32)
}

/// A `<time>` of any sign (`-500ms` is `Delim('-')` then the
/// dimension, `+500ms` `Delim('+')`), in whole milliseconds.
pub fn parse_signed_time_ms(tokens: &[Token]) -> Option<i32> {
    let (negative, rest) = match tokens {
        [Token::Delim('-'), rest @ ..] => (true, rest),
        _ => (false, tokens),
    };
    let ms = i32::try_from(parse_time_ms(rest)?).ok()?;
    Some(if negative { -ms } else { ms })
}

/// Split `value` on commas at depth 0 (parens / function args
/// don't get split). Used by every transition list parser.
pub(super) fn split_on_top_level_commas(value: &[Token]) -> Vec<&[Token]> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    for (i, tok) in value.iter().enumerate() {
        match tok {
            Token::LParen | Token::Function(_) => depth += 1,
            Token::RParen if depth > 0 => depth -= 1,
            Token::Comma if depth == 0 => {
                out.push(&value[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&value[start..]);
    out
}

/// Parsed shorthand piece — `transition: <single>` per the CSS L1
/// grammar. Holds all four longhand values for one comma-separated
/// rule.
#[derive(Debug, Clone, PartialEq)]
pub struct TransitionShorthandRule {
    property: TransitionProperty,
    duration: u32,
    timing: TimingFunction,
    delay: i32,
    behavior: TransitionBehavior,
}

impl TransitionShorthandRule {
    /// Its `transition-behavior`.
    pub fn behavior(&self) -> TransitionBehavior {
        self.behavior
    }

    /// The piece's `transition-property` entry.
    pub fn property(&self) -> &TransitionProperty {
        &self.property
    }

    /// Its duration in ms.
    pub fn duration(&self) -> u32 {
        self.duration
    }

    /// Its easing function.
    pub fn timing(&self) -> &TimingFunction {
        &self.timing
    }

    /// Its delay in ms; negative starts the transition part-way.
    pub fn delay(&self) -> i32 {
        self.delay
    }
}

pub fn parse_transition_shorthand(value: &[Token]) -> Option<Vec<TransitionShorthandRule>> {
    let segments = split_on_top_level_commas(value);
    let mut out = Vec::with_capacity(segments.len());
    for seg in segments {
        out.push(parse_transition_shorthand_single(seg)?);
    }
    // §2.5: `none` is valid only in a single transition.
    if out.len() > 1 && out.iter().any(|r| r.property == TransitionProperty::None) {
        return None;
    }
    if out.is_empty() { None } else { Some(out) }
}

/// Parse one comma-separated piece of `transition: …`. Pieces
/// can appear in any order; we walk the tokens detecting which
/// kind each is. A second `<time>` token becomes the delay (CSS
/// L1 rule).
pub fn parse_transition_shorthand_single(value: &[Token]) -> Option<TransitionShorthandRule> {
    let mut property: Option<TransitionProperty> = None;
    let mut duration: Option<u32> = None;
    let mut delay: Option<i32> = None;
    let mut timing: Option<TimingFunction> = None;
    let mut behavior: Option<TransitionBehavior> = None;

    let mut i = 0;
    while i < value.len() {
        // Try a `<time>` first — it spans 2..=4 tokens.
        if let Some((ms, consumed)) = try_parse_time_at(value, i) {
            if duration.is_none() {
                // The first `<time>` is the duration, never negative.
                duration = Some(u32::try_from(ms).ok()?);
            } else if delay.is_none() {
                delay = Some(ms);
            } else {
                return None; // a third <time> in one piece is invalid
            }
            i += consumed;
            continue;
        }
        // Then an easing function (keyword or `cubic-bezier()` /
        // `steps()`), then a property keyword.
        if let Some((t, used)) = parse_timing_function_at(value, i) {
            if timing.is_some() {
                return None;
            }
            timing = Some(t);
            i += used;
            continue;
        }
        match value.get(i)? {
            Token::Ident(name) if behavior_keyword(name).is_some() => {
                if behavior.is_some() {
                    return None;
                }
                behavior = behavior_keyword(name);
                i += 1;
            }
            Token::Ident(name) => {
                if let Some(p) = parse_transition_property_keyword(name) {
                    if property.is_some() {
                        return None;
                    }
                    property = Some(p);
                } else {
                    return None;
                }
                i += 1;
            }
            _ => return None,
        }
    }

    Some(TransitionShorthandRule {
        property: property.unwrap_or(TransitionProperty::All),
        duration: duration.unwrap_or(0),
        timing: timing.unwrap_or(TimingFunction::Ease),
        delay: delay.unwrap_or(0),
        behavior: behavior.unwrap_or_default(),
    })
}

/// Try to parse a `<time>` value starting at `value[start]`.
/// Returns `(ms, tokens_consumed)`.
fn try_parse_time_at(value: &[Token], start: usize) -> Option<(i32, usize)> {
    // A `<time>` is one `Dimension` token, after a `-` or a `+` sign.
    if let (Some(Token::Delim('-' | '+')), Some(Token::Dimension { .. })) =
        (value.get(start), value.get(start + 1))
    {
        return Some((parse_signed_time_ms(value.get(start..start + 2)?)?, 2));
    }
    let ms = parse_time_ms(value.get(start..start + 1)?)?;
    Some((i32::try_from(ms).ok()?, 1))
}

pub fn unzip_transition_rules(
    rules: &[TransitionShorthandRule],
) -> (
    Vec<TransitionProperty>,
    Vec<u32>,
    Vec<TimingFunction>,
    Vec<i32>,
) {
    let mut props = Vec::with_capacity(rules.len());
    let mut durs = Vec::with_capacity(rules.len());
    let mut timings = Vec::with_capacity(rules.len());
    let mut delays = Vec::with_capacity(rules.len());
    for r in rules {
        props.push(r.property.clone());
        durs.push(r.duration);
        timings.push(r.timing.clone());
        delays.push(r.delay);
    }
    (props, durs, timings, delays)
}

#[cfg(test)]
mod number_value_tests {
    use super::super::easing::parse_timing_function_list;
    use super::*;
    use crate::parse::token::tokenize;

    fn t(src: &str) -> Vec<Token> {
        tokenize(src).unwrap()
    }

    #[test]
    fn time_values_round_to_whole_milliseconds() {
        assert_eq!(parse_time_ms(&t("1.05s")), Some(1050));
        assert_eq!(parse_time_ms(&t("0.5s")), Some(500));
        assert_eq!(parse_time_ms(&t("200ms")), Some(200));
        assert_eq!(parse_time_ms(&t("0s")), Some(0));
        assert_eq!(parse_time_ms(&t("1.6ms")), Some(2));
        assert_eq!(parse_time_ms(&t("-1s")), None);
        assert_eq!(parse_time_ms(&t("1.5")), None, "unitless is not a time");
    }

    /// CSS Syntax 3 §4.3.13: a numeric token may carry a `+` sign, so
    /// `+1s` is a `<time>` — in a list, in the shorthand, as a delay
    /// (C12G-MISC).
    #[test]
    fn a_time_may_carry_a_plus_sign() {
        assert_eq!(parse_time_ms(&t("+1s")), Some(1000));
        assert_eq!(parse_time_list(&t("+250ms, -1s")), Some(vec![250, -1000]));
        let rules = parse_transition_shorthand(&t("color +1s ease +2s")).unwrap();
        assert_eq!((rules[0].duration, rules[0].delay), (1000, 2000));
        let pieces =
            super::super::animation::parse_animation_shorthand(&t("spin +1s +2s")).unwrap();
        assert_eq!(pieces.len(), 1);
    }

    /// `D-M3-2`: `cubic-bezier()` and `steps()` parse in the longhand
    /// list and inside the shorthand; x coordinates outside [0, 1] and
    /// `steps(1, jump-none)` are invalid per CSS Easing 1.
    #[test]
    fn easing_functions_parse() {
        use crate::transition::StepPosition;
        let list = |s: &str| parse_timing_function_list(&t(s));
        assert_eq!(
            list("cubic-bezier(0.1, 0.7, 1.0, 0.1)"),
            Some(vec![TimingFunction::CubicBezier {
                x1: 0.1,
                y1: 0.7,
                x2: 1.0,
                y2: 0.1
            }])
        );
        assert_eq!(
            list("cubic-bezier(0, -2, 1, 3)"),
            Some(vec![TimingFunction::CubicBezier {
                x1: 0.0,
                y1: -2.0,
                x2: 1.0,
                y2: 3.0
            }]),
            "y is unbounded"
        );
        assert_eq!(list("cubic-bezier(1.5, 0, 1, 1)"), None, "x outside [0, 1]");
        assert_eq!(list("cubic-bezier(0, 0, 1)"), None, "arity");
        assert_eq!(
            list("steps(4)"),
            Some(vec![TimingFunction::Steps {
                count: 4,
                position: StepPosition::End
            }])
        );
        assert_eq!(
            list("steps(4, jump-none)"),
            Some(vec![TimingFunction::Steps {
                count: 4,
                position: StepPosition::JumpNone
            }])
        );
        assert_eq!(list("steps(1, jump-none)"), None);
        assert_eq!(list("steps(0)"), None);
        assert_eq!(
            list("step-start, ease"),
            Some(vec![TimingFunction::STEP_START, TimingFunction::Ease])
        );
        let rules = parse_transition_shorthand(&t("width 1s steps(3, start) 0.5s")).unwrap();
        assert_eq!(
            rules[0].timing,
            TimingFunction::Steps {
                count: 3,
                position: StepPosition::Start
            }
        );
        assert_eq!((rules[0].duration, rules[0].delay), (1000, 500));
    }

    /// Transitions L1 §2.1: any `<custom-ident>` is a valid
    /// `transition-property`; a known property is named, any other kept.
    #[test]
    fn transition_property_accepts_any_custom_ident() {
        assert_eq!(
            parse_transition_property_list(&t("display, foo")),
            Some(vec![
                TransitionProperty::Named("display"),
                TransitionProperty::Other("foo".into()),
            ])
        );
    }

    #[test]
    fn transition_shorthand_takes_decimal_durations() {
        let rules = parse_transition_shorthand(&t("width 0.25s ease-in 0.1s")).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].duration, 250);
        assert_eq!(rules[0].delay, 100);
    }
}
