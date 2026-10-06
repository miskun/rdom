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
    if let [Token::Ident(kw)] = value
        && (kw.eq_ignore_ascii_case("none") || kw.eq_ignore_ascii_case("normal"))
    {
        return Some(Content::None);
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
        Token::RParen => Some((CounterStyle::Decimal, 1)),
        Token::Comma => {
            let Token::Ident(style) = rest.get(1)? else {
                return None;
            };
            if !matches!(rest.get(2), Some(Token::RParen)) {
                return None;
            }
            Some((CounterStyle::parse(style)?, 3))
        }
        _ => None,
    }
}

/// `counter-reset` / `counter-increment`: `none` | `[ <ident> <integer>? ]+`
/// with `default` as the implied integer (0 for reset, 1 for increment).
pub fn parse_counter_ops(value: &[Token], default: i32) -> Option<Vec<crate::counters::CounterOp>> {
    use crate::counters::CounterOp;
    if let [Token::Ident(kw)] = value
        && kw.eq_ignore_ascii_case("none")
    {
        return Some(Vec::new());
    }
    let mut ops = Vec::new();
    let mut i = 0;
    while i < value.len() {
        let Token::Ident(name) = &value[i] else {
            return None;
        };
        if name.eq_ignore_ascii_case("none") {
            return None;
        }
        i += 1;
        let mut v = default;
        match (value.get(i), value.get(i + 1)) {
            // A counter value is an `<integer>` (CSS Lists 3 §3.1),
            // clamped to rdom's range (CSS Values 4 §5.1).
            (Some(Token::Number(n)), _) => {
                v = clamp_i32(*n);
                i += 1;
            }
            (Some(Token::Delim('-')), Some(Token::Number(n))) => {
                v = clamp_i32(-*n);
                i += 2;
            }
            _ => {}
        }
        ops.push(CounterOp {
            name: name.clone(),
            value: v,
        });
    }
    if ops.is_empty() { None } else { Some(ops) }
}
