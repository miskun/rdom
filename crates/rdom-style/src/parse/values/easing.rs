//! `<easing-function>` values (CSS Easing 1 / 2): the keywords,
//! `linear(<stops>)`, `cubic-bezier()` and `steps()`, alone and in
//! comma-separated lists.

use super::transition::split_on_top_level_commas;
use crate::parse::token::Token;
use crate::transition::{LinearStop, StepPosition, TimingFunction};

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
