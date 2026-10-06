//! Generated content: `content` (CSS Generated Content 3 §2) and
//! the `counter-reset` / `counter-increment` operation lists.

use crate::counters::CounterStyle;
use crate::parse::token::Token;
use crate::parse::values::numeric::clamp_i32;
use crate::{Content, QuoteKind};

/// `content` (CSS Generated Content 3 §2): `normal | none |
/// <content-list> [ / [ <string> | <counter> | attr() ]+ ]?`, with the
/// `<content-list>` items rdom renders — `<string>`, `<counter>`
/// (`counter()` / `counters()`, CSS Lists 3 §4.3) and `<quote>` (§2.2).
/// Several items concatenate; the alt text after `/` is kept beside them
/// ([`Content::WithAlt`]). `attr()` and `var()` are no items here: they
/// are arbitrary substitution functions (CSS Values 5 §8.7, CSS
/// Variables 1 §3), substituted by the cascade before this grammar runs,
/// an untyped `attr()` as a string. Images (`url()`, gradients) and the
/// paged-media items are not parsed (DIVERGENCES §1).
pub fn parse_content(value: &[Token]) -> Option<Content> {
    if let [Token::Ident(kw)] = value {
        if kw.eq_ignore_ascii_case("none") {
            return Some(Content::None);
        }
        if kw.eq_ignore_ascii_case("normal") {
            return Some(Content::Normal);
        }
    }
    let slash = value.iter().position(|t| matches!(t, Token::Delim('/')));
    let (list, alt) = match slash {
        Some(at) => (&value[..at], Some(&value[at + 1..])),
        None => (value, None),
    };
    let content = parse_items(list, true)?;
    match alt {
        None => Some(content),
        Some(alt) => Some(Content::WithAlt {
            content: Box::new(content),
            alt: Box::new(parse_items(alt, false)?),
        }),
    }
}

/// One or more content items; `<quote>` keywords only when `quotes`
/// (the alt text takes strings and counters alone).
fn parse_items(value: &[Token], quotes: bool) -> Option<Content> {
    let mut parts = Vec::new();
    let mut i = 0;
    while i < value.len() {
        let (item, used) = match &value[i] {
            Token::String(s) => (Content::Str(s.clone()), 1),
            Token::Ident(kw) if quotes => (Content::Quote(QuoteKind::parse(kw)?), 1),
            Token::Function(name) if name.eq_ignore_ascii_case("counter") => {
                parse_counter(&value[i + 1..])?
            }
            Token::Function(name) if name.eq_ignore_ascii_case("counters") => {
                parse_counters(&value[i + 1..])?
            }
            _ => return None,
        };
        parts.push(item);
        i += used;
    }
    match parts.len() {
        0 => None,
        1 => parts.pop(),
        _ => Some(Content::Concat(parts)),
    }
}

/// `counter(` `<counter-name> [, <counter-style>]? )` after the function
/// token: the item and the tokens it used, the function token included.
fn parse_counter(args: &[Token]) -> Option<(Content, usize)> {
    let Token::Ident(name) = args.first()? else {
        return None;
    };
    let (style, used) = trailing_style(&args[1..])?;
    let item = Content::Counter {
        name: name.clone(),
        style,
    };
    Some((item, 2 + used))
}

/// `counters(` `<counter-name>, <string> [, <counter-style>]? )`.
fn parse_counters(args: &[Token]) -> Option<(Content, usize)> {
    let (Token::Ident(name), Token::Comma, Token::String(separator)) =
        (args.first()?, args.get(1)?, args.get(2)?)
    else {
        return None;
    };
    let (style, used) = trailing_style(&args[3..])?;
    let item = Content::Counters {
        name: name.clone(),
        separator: separator.clone(),
        style,
    };
    Some((item, 4 + used))
}

/// `)` or `, <counter-style> )`: the style (`decimal` when absent) and
/// the tokens used, the `)` included.
fn trailing_style(rest: &[Token]) -> Option<(CounterStyle, usize)> {
    match rest.first()? {
        Token::RParen => Some((CounterStyle::decimal(), 1)),
        Token::Comma => {
            let (style, used) = crate::counters::parse_counter_style(&rest[1..])?;
            if !matches!(rest.get(1 + used), Some(Token::RParen)) {
                return None;
            }
            Some((style, used + 2))
        }
        _ => None,
    }
}

/// `counter-reset` / `counter-increment` / `counter-set` (CSS Lists 3
/// §4.2–§4.3): `none` | `[ <counter-name> <integer>? ]+`, with `default`
/// as the implied integer (0 for reset and set, 1 for increment); when
/// `reversed` (reset only), an item may also be `reversed(<counter-name>)
/// <integer>?`, whose missing integer the cascade computes.
pub fn parse_counter_ops(
    value: &[Token],
    default: i32,
    reversed: bool,
) -> Option<Vec<crate::counters::CounterOp>> {
    use crate::counters::CounterOp;
    if let [Token::Ident(kw)] = value
        && kw.eq_ignore_ascii_case("none")
    {
        return Some(Vec::new());
    }
    // A `<counter-name>` is a `<custom-ident>` other than `none` (§4.1).
    let counter_name = |t: Option<&Token>| match t {
        Some(Token::Ident(name)) if !name.eq_ignore_ascii_case("none") => Some(name.clone()),
        _ => None,
    };
    let mut ops = Vec::new();
    let mut i = 0;
    while i < value.len() {
        let (name, is_reversed) = match &value[i] {
            Token::Function(f) if reversed && f.eq_ignore_ascii_case("reversed") => {
                let name = counter_name(value.get(i + 1))?;
                if !matches!(value.get(i + 2), Some(Token::RParen)) {
                    return None;
                }
                i += 3;
                (name, true)
            }
            t => {
                let name = counter_name(Some(t))?;
                i += 1;
                (name, false)
            }
        };
        // A counter value is an `<integer>`, clamped to rdom's range (CSS
        // Values 4 §5.1).
        let given = match (value.get(i), value.get(i + 1)) {
            (Some(Token::Number(n)), _) => {
                i += 1;
                Some(clamp_i32(*n))
            }
            (Some(Token::Delim('-')), Some(Token::Number(n))) => {
                i += 2;
                Some(clamp_i32(-*n))
            }
            _ => None,
        };
        ops.push(if is_reversed {
            CounterOp::reversed(name, given)
        } else {
            CounterOp::new(name, given.unwrap_or(default))
        });
    }
    if ops.is_empty() { None } else { Some(ops) }
}

/// `quotes` (CSS Generated Content 3 §2.1): `auto | none | match-parent |
/// [ <string> <string> ]+`.
pub fn parse_quotes(value: &[Token]) -> Option<crate::Quotes> {
    use crate::{QuotePair, Quotes};
    if let [Token::Ident(kw)] = value {
        return match kw.to_ascii_lowercase().as_str() {
            "auto" => Some(Quotes::Auto),
            "none" => Some(Quotes::None),
            "match-parent" => Some(Quotes::MatchParent),
            _ => None,
        };
    }
    if value.is_empty() || !value.len().is_multiple_of(2) {
        return None;
    }
    let pairs = value
        .chunks(2)
        .map(|pair| match pair {
            [Token::String(open), Token::String(close)] => Some(QuotePair {
                open: open.clone(),
                close: close.clone(),
            }),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Quotes::Pairs(pairs.into()))
}
