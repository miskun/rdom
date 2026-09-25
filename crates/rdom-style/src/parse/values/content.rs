//! Generated content: `content` (strings, `attr()`, `counter()`) and
//! the `counter-reset` / `counter-increment` operation lists.

use crate::Content;
use crate::parse::token::Token;

/// `content`: `none` | `normal` | `[ <string> | attr(<ident>) |
/// counter(<ident> [, <counter-style>]) ]+` (CSS Generated Content 3
/// §1.2, the subset rdom renders). Several items concatenate.
pub fn parse_content(value: &[Token]) -> Option<Content> {
    use crate::counters::CounterStyle;
    if let [Token::Ident(kw)] = value
        && (kw.eq_ignore_ascii_case("none") || kw.eq_ignore_ascii_case("normal"))
    {
        return Some(Content::None);
    }
    let mut parts = Vec::new();
    let mut i = 0;
    while i < value.len() {
        match &value[i] {
            Token::String(s) => {
                parts.push(Content::Str(s.clone()));
                i += 1;
            }
            Token::Function(name) if name.eq_ignore_ascii_case("attr") => {
                let [Token::Ident(arg), Token::RParen] = value.get(i + 1..i + 3)? else {
                    return None;
                };
                parts.push(Content::Attr(arg.clone()));
                i += 3;
            }
            Token::Function(name) if name.eq_ignore_ascii_case("counter") => {
                let Token::Ident(counter) = value.get(i + 1)? else {
                    return None;
                };
                let (style, used) = match value.get(i + 2)? {
                    Token::RParen => (CounterStyle::Decimal, 3),
                    Token::Comma => {
                        let Token::Ident(style) = value.get(i + 3)? else {
                            return None;
                        };
                        if !matches!(value.get(i + 4), Some(Token::RParen)) {
                            return None;
                        }
                        (CounterStyle::parse(style)?, 5)
                    }
                    _ => return None,
                };
                parts.push(Content::Counter {
                    name: counter.clone(),
                    style,
                });
                i += used;
            }
            _ => return None,
        }
    }
    match parts.len() {
        0 => None,
        1 => parts.pop(),
        _ => Some(Content::Concat(parts)),
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
            (Some(Token::Number(n)), _) => {
                v = *n;
                i += 1;
            }
            (Some(Token::Delim('-')), Some(Token::Number(n))) => {
                v = -*n;
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
