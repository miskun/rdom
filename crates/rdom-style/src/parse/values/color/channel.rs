//! Color-function arguments (CSS Color 4 §4.1): channel values
//! (`<number>` / `<percentage>` / `none`), the `<alpha-value>`, and the
//! split of an argument list into the modern (space and `/`) or legacy
//! (comma) form.

use crate::calc::{CalcExpr, CalcKind, ResolveCtx};
use crate::parse::token::Token;
use crate::parse::values::calc::{looks_like_calc, parse_math};
use crate::parse::values::numeric::components;

/// One channel argument as written.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Channel {
    /// A `<number>` (a literal, or a math function of numbers).
    Number(f64),
    /// A `<percentage>` (a literal, or a math function of
    /// percentages), as written: 50% is 50.
    Percent(f64),
    /// A math function mixing numbers and percentages
    /// (`calc(50% + 10)`): the percentages resolve against the
    /// channel's reference range.
    Mixed(CalcExpr),
    /// `none`: a missing component (§4.4).
    None,
}

/// Whether a channel was written as a number or a percentage (the
/// legacy syntax requires one type throughout).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChannelType {
    Number,
    Percent,
    Other,
}

impl Channel {
    /// Parse one component value. `None` when it is no channel.
    pub fn parse(component: &[Token]) -> Option<Channel> {
        let (negative, rest) = match component {
            [Token::Delim('-'), rest @ ..] => (true, rest),
            _ => (false, component),
        };
        let sign = if negative { -1.0 } else { 1.0 };
        Some(match rest {
            [Token::Ident(s)] if !negative && s.eq_ignore_ascii_case("none") => Channel::None,
            [Token::Number(n)] => Channel::Number(sign * f64::from(*n)),
            [Token::Float(f)] => Channel::Number(sign * *f),
            [Token::Percentage(p)] => Channel::Percent(sign * *p),
            _ if !negative && looks_like_calc(rest) => {
                let expr = parse_math(rest)?;
                if expr.needs_context() {
                    return None;
                }
                match expr.kind()? {
                    CalcKind::Number => {
                        Channel::Number(nan_zero(expr.resolve_f64(&ResolveCtx::new(0))))
                    }
                    CalcKind::Percent => {
                        Channel::Percent(nan_zero(expr.resolve_f64(&ResolveCtx::new(100))))
                    }
                    // A sum of numbers and percentages types as a
                    // length; one holding a real length does not type
                    // as a number once percentages count as numbers.
                    _ if expr.kind_as_number() == Some(CalcKind::Number) => Channel::Mixed(expr),
                    _ => return None,
                }
            }
            _ => return None,
        })
    }

    /// The channel's type for the legacy syntax.
    pub fn ty(&self) -> ChannelType {
        match self {
            Channel::Number(_) => ChannelType::Number,
            Channel::Percent(_) => ChannelType::Percent,
            Channel::Mixed(_) | Channel::None => ChannelType::Other,
        }
    }

    /// The value, a percentage taken against `reference` (the number
    /// 100% maps to); `None` when missing.
    pub fn resolve(&self, reference: f64) -> Option<f64> {
        match self {
            Channel::Number(n) => Some(*n),
            Channel::Percent(p) => Some(p / 100.0 * reference),
            Channel::Mixed(expr) => Some(nan_zero(
                percents_as_numbers(expr, reference).resolve_f64(&ResolveCtx::new(0)),
            )),
            Channel::None => None,
        }
    }
}

/// NaN is 0 (CSS Values 4 §10.9).
fn nan_zero(v: f64) -> f64 {
    if v.is_nan() { 0.0 } else { v }
}

/// `expr` with every percentage replaced by the number it stands for
/// against `reference`.
fn percents_as_numbers(expr: &CalcExpr, reference: f64) -> CalcExpr {
    match expr {
        CalcExpr::Percent(p) => CalcExpr::Number(p / 100.0 * reference),
        CalcExpr::Binary { op, lhs, rhs } => CalcExpr::binary(
            *op,
            percents_as_numbers(lhs, reference),
            percents_as_numbers(rhs, reference),
        ),
        CalcExpr::Function { func, args } => CalcExpr::function(
            *func,
            args.iter()
                .map(|a| percents_as_numbers(a, reference))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// A color function's arguments, split by form.
pub(super) enum Arguments<'a> {
    /// `a b c [/ alpha]`: the channel components and the alpha one.
    Modern {
        channels: Vec<&'a [Token]>,
        alpha: Option<&'a [Token]>,
    },
    /// `a, b, c[, alpha]` (only the functions with a legacy form).
    Legacy {
        channels: Vec<&'a [Token]>,
        alpha: Option<&'a [Token]>,
    },
}

/// Split `args` (the tokens between the function token and its `)`).
/// `None` when the list is malformed: a comma list with an entry that
/// is not exactly one component, more than one `/`, or a `/` with
/// nothing after it.
pub(super) fn split_arguments(args: &[Token]) -> Option<Arguments<'_>> {
    if top_level_comma(args) {
        let mut parts = Vec::new();
        for part in split_top_level(args, &Token::Comma) {
            let comps = components(part)?;
            let [one] = comps.as_slice() else {
                return None;
            };
            parts.push(*one);
        }
        let alpha = if parts.len() == 4 { parts.pop() } else { None };
        return Some(Arguments::Legacy {
            channels: parts,
            alpha,
        });
    }
    let comps = components(args)?;
    let slashes: Vec<usize> = comps
        .iter()
        .enumerate()
        .filter(|(_, c)| **c == [Token::Delim('/')])
        .map(|(i, _)| i)
        .collect();
    match slashes.as_slice() {
        [] => Some(Arguments::Modern {
            channels: comps,
            alpha: None,
        }),
        [i] if *i + 2 == comps.len() => Some(Arguments::Modern {
            channels: comps[..*i].to_vec(),
            alpha: Some(comps[*i + 1]),
        }),
        _ => None,
    }
}

/// True when `args` holds a comma outside any nested function.
fn top_level_comma(args: &[Token]) -> bool {
    let mut depth = 0usize;
    for t in args {
        match t {
            Token::Function(_) | Token::LParen => depth += 1,
            Token::RParen => depth = depth.saturating_sub(1),
            Token::Comma if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

/// `args` cut at every top-level `sep`.
fn split_top_level<'a>(args: &'a [Token], sep: &Token) -> Vec<&'a [Token]> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (i, t) in args.iter().enumerate() {
        match t {
            Token::Function(_) | Token::LParen => depth += 1,
            Token::RParen => depth = depth.saturating_sub(1),
            t if depth == 0 && t == sep => {
                out.push(&args[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&args[start..]);
    out
}

/// Parse an `<alpha-value>` (a number, or a percentage of 1) — or
/// `none` where `allow_none` — into the alpha, missing as `None`.
/// The outer `Option` is `None` when the component is no alpha.
pub(super) fn alpha(component: &[Token], allow_none: bool) -> Option<Option<f64>> {
    let ch = Channel::parse(component)?;
    if ch == Channel::None && !allow_none {
        return None;
    }
    Some(ch.resolve(1.0))
}
