//! Transition values: animatable-property names, easing functions
//! (keywords, `cubic-bezier()`, `steps()`), `<time>` durations, the
//! comma-separated longhand lists and the `transition` shorthand.

use crate::parse::token::Token;
use crate::transition::{LinearStop, StepPosition, TimingFunction, TransitionProperty};

/// Parse a single property keyword: `all`, `none`, or any
/// `<custom-ident>` (CSS Transitions 1 §2.1) — a property the dispatch
/// table knows is [`TransitionProperty::Named`], any other name (a custom
/// property, an unknown one) is valid and kept as
/// [`TransitionProperty::Other`].
pub fn parse_transition_property_keyword(name: &str) -> Option<TransitionProperty> {
    Some(TransitionProperty::named(name))
}

/// Parse a single timing-function keyword (CSS Easing 1 §2.2 / §2.3).
pub fn parse_timing_function_keyword(name: &str) -> Option<TimingFunction> {
    match name.to_ascii_lowercase().as_str() {
        "linear" => Some(TimingFunction::Linear),
        "ease" => Some(TimingFunction::Ease),
        "ease-in" => Some(TimingFunction::EaseIn),
        "ease-out" => Some(TimingFunction::EaseOut),
        "ease-in-out" => Some(TimingFunction::EaseInOut),
        "step-start" => Some(TimingFunction::STEP_START),
        "step-end" => Some(TimingFunction::STEP_END),
        _ => None,
    }
}

/// Parse one `<easing-function>` at `value[start]`: a keyword,
/// `cubic-bezier(x1, y1, x2, y2)` or `steps(n [, <position>])`.
/// Returns the function and the number of tokens consumed.
pub fn parse_timing_function_at(value: &[Token], start: usize) -> Option<(TimingFunction, usize)> {
    match value.get(start)? {
        Token::Ident(name) => parse_timing_function_keyword(name).map(|f| (f, 1)),
        Token::Function(name) if name.eq_ignore_ascii_case("cubic-bezier") => {
            let mut i = start + 1;
            let mut args = [0f32; 4];
            for (k, slot) in args.iter_mut().enumerate() {
                let (n, used) = signed_number_at(value, i)?;
                *slot = n as f32;
                i += used;
                let expect_comma = k < 3;
                match value.get(i)? {
                    Token::Comma if expect_comma => i += 1,
                    Token::RParen if !expect_comma => i += 1,
                    _ => return None,
                }
            }
            let [x1, y1, x2, y2] = args;
            // §2.2.1: the x coordinates must stay within [0, 1].
            if !(0.0..=1.0).contains(&x1) || !(0.0..=1.0).contains(&x2) {
                return None;
            }
            Some((TimingFunction::CubicBezier { x1, y1, x2, y2 }, i - start))
        }
        Token::Function(name) if name.eq_ignore_ascii_case("linear") => {
            let close = matching_paren(value, start)?;
            let stops = parse_linear_stops(&value[start + 1..close])?;
            Some((TimingFunction::LinearStops(stops.into()), close + 1 - start))
        }
        Token::Function(name) if name.eq_ignore_ascii_case("steps") => {
            let mut i = start + 1;
            let count = match value.get(i)? {
                // An `<integer [1,∞]>`, clamped to `u32` (CSS Values 4 §5.1).
                Token::Number(n) if *n >= 1 => u32::try_from(*n).unwrap_or(u32::MAX),
                _ => return None,
            };
            i += 1;
            let position = match value.get(i)? {
                Token::RParen => StepPosition::End,
                Token::Comma => {
                    i += 1;
                    let Token::Ident(pos) = value.get(i)? else {
                        return None;
                    };
                    i += 1;
                    match pos.to_ascii_lowercase().as_str() {
                        "start" | "jump-start" => StepPosition::Start,
                        "end" | "jump-end" => StepPosition::End,
                        "jump-none" => StepPosition::JumpNone,
                        "jump-both" => StepPosition::JumpBoth,
                        _ => return None,
                    }
                }
                _ => return None,
            };
            if !matches!(value.get(i), Some(Token::RParen)) {
                return None;
            }
            i += 1;
            // §2.3: `jump-none` needs at least two steps.
            if position == StepPosition::JumpNone && count < 2 {
                return None;
            }
            Some((TimingFunction::Steps { count, position }, i - start))
        }
        _ => None,
    }
}

/// The index of the `)` closing the function opened at `value[open]`.
fn matching_paren(value: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, t) in value.iter().enumerate().skip(open) {
        match t {
            Token::Function(_) | Token::LParen => depth += 1,
            Token::RParen => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// `linear(<linear-stop-list>)`'s arguments (CSS Easing 2 §2.1): each a
/// `<number>` output with zero, one or two `<percentage>` inputs, at
/// least two — canonicalized (§2.1.1): a missing first / last input is
/// 0% / 100%, an input below a previous one is raised to it, and runs of
/// missing inputs are spread evenly between their neighbours.
fn parse_linear_stops(args: &[Token]) -> Option<Vec<LinearStop>> {
    let mut points: Vec<(f32, Option<f32>)> = Vec::new();
    let segments = split_on_top_level_commas(args);
    if segments.len() < 2 {
        return None;
    }
    for seg in segments {
        let mut output = None;
        let mut inputs = Vec::new();
        let mut i = 0;
        while i < seg.len() {
            if let Some((n, used)) = signed_number_at(seg, i)
                && output.is_none()
            {
                output = Some(n as f32);
                i += used;
                continue;
            }
            let (sign, at) = match seg.get(i)? {
                Token::Delim('-') => (-1.0, i + 1),
                _ => (1.0, i),
            };
            match seg.get(at)? {
                Token::Percentage(p) if inputs.len() < 2 => {
                    inputs.push((sign * p / 100.0) as f32);
                    i = at + 1;
                }
                _ => return None,
            }
        }
        let output = output?;
        if inputs.is_empty() {
            points.push((output, None));
        }
        for input in inputs {
            points.push((output, Some(input)));
        }
    }
    let last = points.len() - 1;
    if points[0].1.is_none() {
        points[0].1 = Some(0.0);
    }
    if points[last].1.is_none() {
        points[last].1 = Some(1.0);
    }
    let mut largest = f32::NEG_INFINITY;
    for p in points.iter_mut() {
        if let Some(input) = p.1.as_mut() {
            *input = input.max(largest);
            largest = *input;
        }
    }
    let mut i = 0;
    while i < points.len() {
        if points[i].1.is_some() {
            i += 1;
            continue;
        }
        let run_start = i;
        while points[i].1.is_none() {
            i += 1;
        }
        let (from, to) = (points[run_start - 1].1?, points[i].1?);
        let steps = (i - run_start + 1) as f32;
        for (k, p) in points[run_start..i].iter_mut().enumerate() {
            p.1 = Some(from + (to - from) * (k + 1) as f32 / steps);
        }
    }
    Some(
        points
            .into_iter()
            .map(|(output, input)| LinearStop::new(input.unwrap_or(0.0), output))
            .collect(),
    )
}

/// A `<number>` at `value[start]`, with an optional leading `-` /
/// `+` delimiter. Returns the value and the tokens consumed.
fn signed_number_at(value: &[Token], start: usize) -> Option<(f64, usize)> {
    let (sign, i) = match value.get(start)? {
        Token::Delim('-') => (-1.0, start + 1),
        Token::Delim('+') => (1.0, start + 1),
        _ => (1.0, start),
    };
    let n = match value.get(i)? {
        Token::Number(n) => *n as f64,
        Token::Float(f) => *f,
        _ => return None,
    };
    Some((sign * n, i - start + 1))
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

/// Parse a comma-separated list of easing functions.
pub fn parse_timing_function_list(value: &[Token]) -> Option<Vec<TimingFunction>> {
    let segments = split_on_top_level_commas(value);
    let mut out = Vec::with_capacity(segments.len());
    for seg in segments {
        let (f, used) = parse_timing_function_at(seg, 0)?;
        if used != seg.len() {
            return None;
        }
        out.push(f);
    }
    if out.is_empty() { None } else { Some(out) }
}

/// Parse a single `<time>` value (`200ms`, `0.5s`, `1.05s`, `0s`) into
/// whole milliseconds, rounding half away from zero. The literal is one
/// `Dimension` token (CSS Syntax 3 §4.3.3).
pub fn parse_time_ms(tokens: &[Token]) -> Option<u32> {
    let (n, unit) = match tokens {
        [Token::Dimension { value, unit, .. }] => (*value, unit),
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
/// dimension), in whole milliseconds.
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
fn split_on_top_level_commas(value: &[Token]) -> Vec<&[Token]> {
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
}

impl TransitionShorthandRule {
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
    })
}

/// Try to parse a `<time>` value starting at `value[start]`.
/// Returns `(ms, tokens_consumed)`.
fn try_parse_time_at(value: &[Token], start: usize) -> Option<(i32, usize)> {
    // A `<time>` is one `Dimension` token, after a `-` when negative.
    if let (Some(Token::Delim('-')), Some(Token::Dimension { .. })) =
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
