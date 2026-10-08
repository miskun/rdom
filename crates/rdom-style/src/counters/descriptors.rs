//! Parsing counter styles (CSS Counter Styles 3): a `<counter-style>`
//! value (a name or `symbols()`, §5) and the descriptors of an
//! `@counter-style` rule (§3), from tokens. `rdom-css` reads the rule's
//! prelude and block; this module owns the grammar.

use std::sync::Arc;

use super::rule::{CounterRange, CounterStyleRule, SpeakAs, System};
use super::style::{CounterStyle, is_counter_style_name};
use crate::parse::token::Token;

/// A `<counter-style>` at the start of `tokens` (§3, §5): a
/// `<counter-style-name>` identifier, or `symbols( <symbols-type>?
/// <string>+ )`. The style and the tokens it used.
pub fn parse_counter_style(tokens: &[Token]) -> Option<(CounterStyle, usize)> {
    match tokens.first()? {
        Token::Ident(name) => Some((CounterStyle::parse(name)?, 1)),
        Token::Function(f) if f.eq_ignore_ascii_case("symbols") => {
            let end = tokens.iter().position(|t| *t == Token::RParen)?;
            let args = &tokens[1..end];
            let (system, strings) = match args.first()? {
                Token::Ident(kw) => (symbols_type(kw)?, &args[1..]),
                _ => (System::Symbolic, args),
            };
            let symbols = strings
                .iter()
                .map(|t| match t {
                    Token::String(s) => Some(s.as_str()),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?;
            Some((CounterStyle::symbols(system, &symbols)?, end + 1))
        }
        _ => None,
    }
}

/// `<symbols-type>` (§5): the systems `symbols()` takes.
fn symbols_type(kw: &str) -> Option<System> {
    Some(match kw.to_ascii_lowercase().as_str() {
        "cyclic" => System::Cyclic,
        "numeric" => System::Numeric,
        "alphabetic" => System::Alphabetic,
        "symbolic" => System::Symbolic,
        "fixed" => System::Fixed(1),
        _ => return None,
    })
}

/// Why [`apply_descriptor`] dropped a declaration (§3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DescriptorError {
    /// No `@counter-style` descriptor has that name.
    Unknown,
    /// The value does not match the descriptor's grammar.
    InvalidValue,
}

impl std::fmt::Display for DescriptorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            DescriptorError::Unknown => "unknown descriptor",
            DescriptorError::InvalidValue => "invalid descriptor value",
        })
    }
}

impl std::error::Error for DescriptorError {}

/// Why [`check_rule`] finds that an `@counter-style` rule defines nothing
/// (§3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CounterStyleRuleError {
    /// The name may not be defined: not a `<counter-style-name>` (a
    /// CSS-wide keyword, `default`), or `none` or one of the styles
    /// authors cannot override (`decimal`, `disc`, `square`, `circle`,
    /// `disclosure-open`, `disclosure-closed`).
    ReservedName,
    /// The system lacks the symbols it needs, or `extends` comes with
    /// symbols ([`CounterStyleRule::is_valid`]).
    SymbolsDoNotSuitSystem,
}

impl std::fmt::Display for CounterStyleRuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CounterStyleRuleError::ReservedName => "the name cannot name a counter style",
            CounterStyleRuleError::SymbolsDoNotSuitSystem => "the symbols do not suit the system",
        })
    }
}

impl std::error::Error for CounterStyleRuleError {}

/// Apply one `descriptor: value` of an `@counter-style` block to `rule`
/// (§3). `Err` with the reason when the descriptor is unknown or its
/// value invalid — the declaration is then ignored, as CSS ignores an
/// invalid declaration (CSS Syntax 3 §5.4.6).
pub fn apply_descriptor(
    rule: &mut CounterStyleRule,
    descriptor: &str,
    value: &[Token],
) -> Result<(), DescriptorError> {
    let invalid = || DescriptorError::InvalidValue;
    match descriptor.to_ascii_lowercase().as_str() {
        "system" => rule.system = Some(system(value).ok_or_else(invalid)?),
        "symbols" => {
            let symbols = symbol_list(value).ok_or_else(invalid)?;
            rule.symbols = Some(symbols.into());
        }
        "additive-symbols" => {
            rule.additive_symbols = Some(additive_symbols(value).ok_or_else(invalid)?);
        }
        "negative" => {
            let symbols = symbol_list(value)
                .filter(|s| s.len() <= 2)
                .ok_or_else(invalid)?;
            let mut it = symbols.into_iter();
            let before = it.next().unwrap_or_default();
            rule.negative = Some((before, it.next().unwrap_or_default()));
        }
        "prefix" => rule.prefix = Some(single_symbol(value).ok_or_else(invalid)?),
        "suffix" => rule.suffix = Some(single_symbol(value).ok_or_else(invalid)?),
        "range" => rule.range = Some(range(value).ok_or_else(invalid)?),
        "pad" => rule.pad = Some(pad(value).ok_or_else(invalid)?),
        "fallback" => match value {
            [Token::Ident(name)] if is_counter_style_name(name) => {
                rule.fallback = CounterStyle::named(name).name().map(Arc::from);
            }
            _ => return Err(invalid()),
        },
        "speak-as" => rule.speak_as = Some(speak_as(value).ok_or_else(invalid)?),
        _ => return Err(DescriptorError::Unknown),
    }
    Ok(())
}

/// Whether `name` may name an `@counter-style` rule and `rule` defines a
/// counter style (§3): the name is a `<counter-style-name>` other than
/// `none` and the styles authors cannot override (`decimal`, `disc`,
/// `square`, `circle`, `disclosure-open`, `disclosure-closed`); the
/// system has the symbols it needs ([`CounterStyleRule::is_valid`]).
pub fn check_rule(name: &str, rule: &CounterStyleRule) -> Result<(), CounterStyleRuleError> {
    const FIXED: [&str; 7] = [
        "none",
        "decimal",
        "disc",
        "square",
        "circle",
        "disclosure-open",
        "disclosure-closed",
    ];
    if !is_counter_style_name(name) || FIXED.iter().any(|f| name.eq_ignore_ascii_case(f)) {
        return Err(CounterStyleRuleError::ReservedName);
    }
    if !rule.is_valid() {
        return Err(CounterStyleRuleError::SymbolsDoNotSuitSystem);
    }
    Ok(())
}

/// `system` (§3.1).
fn system(value: &[Token]) -> Option<System> {
    let (Token::Ident(kw), rest) = value.split_first()? else {
        return None;
    };
    let kw = kw.to_ascii_lowercase();
    match (kw.as_str(), rest) {
        ("cyclic", []) => Some(System::Cyclic),
        ("numeric", []) => Some(System::Numeric),
        ("alphabetic", []) => Some(System::Alphabetic),
        ("symbolic", []) => Some(System::Symbolic),
        ("additive", []) => Some(System::Additive),
        ("fixed", []) => Some(System::Fixed(1)),
        ("fixed", rest) => {
            let (n, used) = integer(rest)?;
            (used == rest.len()).then_some(System::Fixed(i32::try_from(n).ok()?))
        }
        ("extends", [Token::Ident(name)]) if is_counter_style_name(name) => Some(System::Extends(
            Arc::from(CounterStyle::named(name).name()?),
        )),
        _ => None,
    }
}

/// One `<symbol>` (§3.2): a `<string>` or a `<custom-ident>`. Images
/// have no meaning in a cell grid (DIVERGENCES §1).
fn symbol(token: &Token) -> Option<String> {
    match token {
        Token::String(s) | Token::Ident(s) => Some(s.clone()),
        _ => None,
    }
}

/// `<symbol>+`.
fn symbol_list(value: &[Token]) -> Option<Vec<String>> {
    if value.is_empty() {
        return None;
    }
    value.iter().map(symbol).collect()
}

fn single_symbol(value: &[Token]) -> Option<String> {
    match value {
        [t] => symbol(t),
        _ => None,
    }
}

/// An `<integer>` at the start of `tokens` (a `-` is its own token):
/// the value and the tokens used.
fn integer(tokens: &[Token]) -> Option<(i64, usize)> {
    match tokens {
        [Token::Number(n), ..] => Some((*n, 1)),
        [Token::Delim('-'), Token::Number(n), ..] => Some((-*n, 2)),
        [Token::Delim('+'), Token::Number(n), ..] => Some((*n, 2)),
        _ => None,
    }
}

/// `<integer [0,∞]> && <symbol>`, in either order.
fn weight_and_symbol(tokens: &[Token]) -> Option<(u32, String)> {
    if tokens.len() < 2 {
        return None;
    }
    let (weight, symbol_at) = match integer(tokens) {
        Some((n, used)) if used == tokens.len() - 1 => (n, tokens.len() - 1),
        _ => {
            let (n, used) = integer(&tokens[1..])?;
            (used == tokens.len() - 1).then_some(())?;
            (n, 0)
        }
    };
    let weight = u32::try_from(weight).ok()?;
    Some((weight, symbol(tokens.get(symbol_at)?)?))
}

/// `additive-symbols` (§3.2): `[ <integer [0,∞]> && <symbol> ]#`, the
/// weights strictly descending.
fn additive_symbols(value: &[Token]) -> Option<Arc<[(u32, String)]>> {
    let tuples = value
        .split(|t| *t == Token::Comma)
        .map(weight_and_symbol)
        .collect::<Option<Vec<_>>>()?;
    let descending = tuples.windows(2).all(|w| w[0].0 > w[1].0);
    (!tuples.is_empty() && descending).then(|| tuples.into())
}

/// `range` (§3.5): `auto`, or `[ [ <integer> | infinite ]{2} ]#`, each
/// lower bound not above its upper.
fn range(value: &[Token]) -> Option<CounterRange> {
    if let [Token::Ident(kw)] = value
        && kw.eq_ignore_ascii_case("auto")
    {
        return Some(CounterRange::Auto);
    }
    let bound = |tokens: &[Token], low: bool| -> Option<(i64, usize)> {
        match tokens.first()? {
            Token::Ident(kw) if kw.eq_ignore_ascii_case("infinite") => {
                Some((if low { i64::MIN } else { i64::MAX }, 1))
            }
            _ => integer(tokens),
        }
    };
    let ranges = value
        .split(|t| *t == Token::Comma)
        .map(|pair| {
            let (lo, used) = bound(pair, true)?;
            let (hi, used2) = bound(&pair[used..], false)?;
            (used + used2 == pair.len() && lo <= hi).then_some((lo, hi))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(CounterRange::Ranges(ranges.into()))
}

/// `pad` (§3.6): `<integer [0,∞]> && <symbol>`.
fn pad(value: &[Token]) -> Option<(u32, String)> {
    (value.len() == 2 || value.len() == 3)
        .then(|| weight_and_symbol(value))
        .flatten()
}

/// `speak-as` (§3.9).
fn speak_as(value: &[Token]) -> Option<SpeakAs> {
    let [Token::Ident(kw)] = value else {
        return None;
    };
    Some(match kw.to_ascii_lowercase().as_str() {
        "auto" => SpeakAs::Auto,
        "bullets" => SpeakAs::Bullets,
        "numbers" => SpeakAs::Numbers,
        "words" => SpeakAs::Words,
        "spell-out" => SpeakAs::SpellOut,
        _ if is_counter_style_name(kw) => SpeakAs::Style(Arc::from(kw.as_str())),
        _ => return None,
    })
}
