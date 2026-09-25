//! Transition values: animatable-property names, easing functions
//! (keywords, `cubic-bezier()`, `steps()`), `<time>` durations, the
//! comma-separated longhand lists and the `transition` shorthand.

use crate::parse::token::Token;
use crate::transition::{AnimatableProperty, StepPosition, TimingFunction, TransitionProperty};

/// Map a CSS property name to its `AnimatableProperty` slot.
/// Returns `None` for non-animatable / unknown names so the
/// caller can warn.
pub fn parse_animatable_property(name: &str) -> Option<AnimatableProperty> {
    Some(match name.to_ascii_lowercase().as_str() {
        "color" => AnimatableProperty::Color,
        "background-color" => AnimatableProperty::BackgroundColor,
        "border-color" => AnimatableProperty::BorderColor,
        "width" => AnimatableProperty::Width,
        "height" => AnimatableProperty::Height,
        "padding" => AnimatableProperty::Padding,
        "gap" => AnimatableProperty::Gap,
        "top" => AnimatableProperty::Top,
        "right" => AnimatableProperty::Right,
        "bottom" => AnimatableProperty::Bottom,
        "left" => AnimatableProperty::Left,
        "z-index" => AnimatableProperty::ZIndex,
        _ => return None,
    })
}

/// Parse a single property keyword (`all` / `none` / named).
pub fn parse_transition_property_keyword(name: &str) -> Option<TransitionProperty> {
    match name.to_ascii_lowercase().as_str() {
        "all" => Some(TransitionProperty::All),
        "none" => Some(TransitionProperty::None),
        other => match parse_animatable_property(other) {
            Some(ap) => Some(TransitionProperty::Named(ap)),
            // Any other `<custom-ident>` is valid and inert (Transitions
            // L1 §2.1), whether or not it names a property rdom knows.
            None => Some(TransitionProperty::Discrete(other.to_string())),
        },
    }
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
        Token::Function(name) if name.eq_ignore_ascii_case("steps") => {
            let mut i = start + 1;
            let count = match value.get(i)? {
                Token::Number(n) if *n >= 1 => *n as u32,
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

/// A `<number>` at `value[start]`, with an optional leading `-` /
/// `+` delimiter. Returns the value and the tokens consumed.
fn signed_number_at(value: &[Token], start: usize) -> Option<(f64, usize)> {
    let (sign, i) = match value.get(start)? {
        Token::Delim('-') => (-1.0, start + 1),
        Token::Delim('+') => (1.0, start + 1),
        _ => (1.0, start),
    };
    let n = match value.get(i)? {
        Token::Number(n) => f64::from(*n),
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
    if out.is_empty() { None } else { Some(out) }
}

/// Parse a comma-separated list of `<time>` values into ms.
pub fn parse_time_list(value: &[Token]) -> Option<Vec<u32>> {
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
/// token (`Number` or `Float`) followed by the unit ident.
pub fn parse_time_ms(tokens: &[Token]) -> Option<u32> {
    let (n, unit) = match tokens {
        [Token::Number(n), Token::Ident(unit)] => (f64::from(*n), unit),
        [Token::Float(f), Token::Ident(unit)] => (*f, unit),
        _ => return None,
    };
    if n < 0.0 {
        return None;
    }
    let ms = match unit.as_str() {
        "ms" => n,
        "s" => n * 1000.0,
        _ => return None,
    };
    let ms = ms.round();
    (ms <= f64::from(u32::MAX)).then_some(ms as u32)
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
pub struct TransitionShorthandRule {
    property: TransitionProperty,
    duration: u32,
    timing: TimingFunction,
    delay: u32,
}

pub fn parse_transition_shorthand(value: &[Token]) -> Option<Vec<TransitionShorthandRule>> {
    let segments = split_on_top_level_commas(value);
    let mut out = Vec::with_capacity(segments.len());
    for seg in segments {
        out.push(parse_transition_shorthand_single(seg)?);
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
    let mut delay: Option<u32> = None;
    let mut timing: Option<TimingFunction> = None;

    let mut i = 0;
    while i < value.len() {
        // Try a `<time>` first — it spans 2..=4 tokens.
        if let Some((ms, consumed)) = try_parse_time_at(value, i) {
            if duration.is_none() {
                duration = Some(ms);
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
fn try_parse_time_at(value: &[Token], start: usize) -> Option<(u32, usize)> {
    // A `<time>` is always one numeric token plus its unit ident.
    if start + 2 <= value.len()
        && let Some(ms) = parse_time_ms(&value[start..start + 2])
    {
        return Some((ms, 2));
    }
    None
}

pub fn unzip_transition_rules(
    rules: &[TransitionShorthandRule],
) -> (
    Vec<TransitionProperty>,
    Vec<u32>,
    Vec<TimingFunction>,
    Vec<u32>,
) {
    let mut props = Vec::with_capacity(rules.len());
    let mut durs = Vec::with_capacity(rules.len());
    let mut timings = Vec::with_capacity(rules.len());
    let mut delays = Vec::with_capacity(rules.len());
    for r in rules {
        props.push(r.property.clone());
        durs.push(r.duration);
        timings.push(r.timing);
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

    /// Transitions L1 §2.1: any `<custom-ident>` is a valid, inert
    /// `transition-property`.
    #[test]
    fn transition_property_accepts_any_custom_ident() {
        assert_eq!(
            parse_transition_property_list(&t("display, foo")),
            Some(vec![
                TransitionProperty::Discrete("display".into()),
                TransitionProperty::Discrete("foo".into()),
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
