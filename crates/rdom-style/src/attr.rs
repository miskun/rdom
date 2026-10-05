//! `attr()` (CSS Values 5 §8.7): an arbitrary substitution function, like
//! `var()`, that substitutes the value of an attribute of the element the
//! style applies to — of a pseudo-element's originating element — at
//! computed-value time, in any property.
//!
//! `attr( <attr-name> <attr-type>? , <declaration-value>? )`:
//!
//! - `type(<syntax>)` parses the attribute against the syntax (the
//!   `@property` syntaxes rdom checks) and substitutes its tokens;
//! - `number` parses it as a `<number-token>`, and a unit (`%`, `ch`,
//!   `deg`, …) as a number then that dimension;
//! - `raw-string`, or no type, substitutes it as a `<string>`, verbatim.
//!
//! A missing attribute or one that does not parse takes the fallback;
//! with none, an untyped `attr()` is the empty string and a typed one the
//! guaranteed-invalid value (the declaration is invalid at computed-value
//! time). The `<url>` taint rule (§8.7.2) has nothing to guard: rdom has
//! no `url()`.

use std::sync::Arc;

use crate::parse::token::{Token, tokenize};

/// An element's attributes by name: what `attr()` reads. `None` when
/// the attribute is absent. Borrowed from the element: a lookup
/// allocates nothing.
pub type AttrLookup<'a> = &'a dyn Fn(&str) -> Option<&'a str>;

/// Is `token` an `attr(` function token?
pub(crate) fn is_attr(token: &Token) -> bool {
    matches!(token, Token::Function(f) if f.eq_ignore_ascii_case("attr"))
}

/// The arguments of an `attr()` (between `attr(` and its `)`) split at
/// the first top-level comma: the name-and-type part and the fallback
/// (`None` without a comma; `Some(&[])` for `attr(x,)`).
pub(crate) fn split_args(args: &[Token]) -> (&[Token], Option<&[Token]>) {
    let mut depth = 0usize;
    for (i, t) in args.iter().enumerate() {
        match t {
            Token::Function(_) | Token::LParen => depth += 1,
            Token::RParen => depth = depth.saturating_sub(1),
            Token::Comma if depth == 0 => return (&args[..i], Some(&args[i + 1..])),
            _ => {}
        }
    }
    (args, None)
}

/// `<attr-args> = attr( <declaration-value>, <declaration-value>? )`
/// with the first argument `<attr-name> <attr-type>?` (CSS Values 5
/// §8.7): a parse-time check. A first argument holding a substitution
/// function (`attr(var(--n))`) is only known once substituted, so it is
/// checked then.
pub(crate) fn valid_args(args: &[Token]) -> bool {
    let head = split_args(args).0;
    !head.is_empty() && (crate::var::contains_substitution(head) || parse_head(head).is_some())
}

/// The parsed first arguments of a declaration's `attr()`s, keyed by the
/// index of each `attr(` token in the declaration's tokens — parsed once
/// when the declaration is stored ([`crate::var::PendingDeclaration`]).
/// A head holding a substitution function is not here: it is parsed
/// when substituted.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct AttrHeads(Arc<[(usize, AttrHead)]>);

impl AttrHeads {
    /// The heads of every `attr()` in `tokens`, nested ones included.
    pub(crate) fn of(tokens: &[Token]) -> Self {
        let mut heads = Vec::new();
        for (at, t) in tokens.iter().enumerate() {
            if !is_attr(t) {
                continue;
            }
            let Some(end) = crate::var::matching_paren(tokens, at) else {
                continue;
            };
            let head = split_args(&tokens[at + 1..end]).0;
            if crate::var::contains_substitution(head) {
                continue;
            }
            if let Some(parsed) = parse_head(head) {
                heads.push((at, parsed));
            }
        }
        AttrHeads(heads.into())
    }

    /// The head of the `attr()` whose function token is at `at`.
    pub(crate) fn get(&self, at: usize) -> Option<&AttrHead> {
        self.0
            .binary_search_by_key(&at, |(i, _)| *i)
            .ok()
            .map(|i| &self.0[i].1)
    }
}

/// What an `attr()` stands for once its first argument is known.
pub(crate) enum Replacement<'t> {
    /// The attribute's value as tokens.
    Tokens(Vec<Token>),
    /// The fallback's tokens, to substitute in turn.
    Fallback(&'t [Token]),
    /// The guaranteed-invalid value: the typed attribute `name` had no
    /// usable value and there is no fallback.
    Invalid(String),
}

/// A parsed first argument: `<attr-name> <attr-type>?`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AttrHead {
    name: String,
    ty: AttrType,
}

/// The parsed `<attr-type>`.
#[derive(Debug, Clone, PartialEq)]
enum AttrType {
    /// `raw-string`, or no type: a `<string>`.
    RawString,
    /// `number`.
    Number,
    /// A unit: a number, then a dimension or percentage in it.
    Unit(String),
    /// `type(<syntax>)`.
    Syntax(crate::registration::PropertySyntax),
}

/// CSS Values 5 §8.7.1 "replace an attr() function": `head` is the
/// first argument with its own substitution functions substituted —
/// `parsed` when it was parsed in advance ([`AttrHeads`]) —, `fallback`
/// the second (unsubstituted), `attrs` the element's attributes (`None`:
/// no element, so no attribute).
pub(crate) fn replace<'t>(
    head: &[Token],
    parsed: Option<&AttrHead>,
    fallback: Option<&'t [Token]>,
    attrs: Option<AttrLookup<'_>>,
) -> Replacement<'t> {
    let owned;
    let parsed = match parsed {
        Some(p) => Some(p),
        None => {
            owned = parse_head(head);
            owned.as_ref()
        }
    };
    let typed = !matches!(
        parsed,
        Some(AttrHead {
            ty: AttrType::RawString,
            ..
        })
    );
    let value = parsed.and_then(|h| resolve(attrs?(&h.name)?, &h.ty));
    match (value, fallback) {
        (Some(tokens), _) => Replacement::Tokens(tokens),
        (None, Some(fallback)) => Replacement::Fallback(fallback),
        (None, None) if !typed => Replacement::Tokens(vec![Token::String(String::new())]),
        (None, None) => Replacement::Invalid(parsed.map(|h| h.name.clone()).unwrap_or_default()),
    }
}

/// `<attr-name> <attr-type>?`. `None` on a parse failure — an
/// `<attr-type>` that is not `type(<syntax>)`, `raw-string`, `number` or
/// a CSS unit. Attributes have no namespace in rdom: `|name` is `name`,
/// `ns|name` never exists.
fn parse_head(head: &[Token]) -> Option<AttrHead> {
    #[cfg(test)]
    probe::HEAD_PARSES.with(|c| c.set(c.get() + 1));
    let (name, rest) = match head {
        [Token::Delim('|'), Token::Ident(name), rest @ ..] => (name, rest),
        [Token::Ident(_), Token::Delim('|'), ..] => return None,
        [Token::Ident(name), rest @ ..] => (name, rest),
        _ => return None,
    };
    let ty = match rest {
        [] => AttrType::RawString,
        [Token::Ident(k)] if k.eq_ignore_ascii_case("raw-string") => AttrType::RawString,
        [Token::Ident(k)] if k.eq_ignore_ascii_case("number") => AttrType::Number,
        [Token::Ident(unit)] if is_known_unit(unit) => AttrType::Unit(unit.clone()),
        [Token::Delim('%')] => AttrType::Unit("%".to_string()),
        [Token::Function(f), syntax @ .., Token::RParen] if f.eq_ignore_ascii_case("type") => {
            AttrType::Syntax(crate::registration::PropertySyntax::parse(&syntax_text(syntax)).ok()?)
        }
        _ => return None,
    };
    Some(AttrHead {
        name: name.clone(),
        ty,
    })
}

/// A `<syntax>`'s tokens back to the text the `@property` syntax parser
/// reads (`<length> | auto`).
fn syntax_text(tokens: &[Token]) -> String {
    let mut out = String::new();
    for t in tokens {
        match t {
            Token::Delim('|') => out.push_str(" | "),
            Token::Delim(c) => out.push(*c),
            Token::Ident(s) => out.push_str(s),
            Token::HexColor(h) => {
                out.push('#');
                out.push_str(h);
            }
            other => out.push_str(&crate::parse::values::render_value(std::slice::from_ref(
                other,
            ))),
        }
    }
    out
}

/// The attribute `value` as `ty`, `None` when it does not parse.
fn resolve(value: &str, ty: &AttrType) -> Option<Vec<Token>> {
    match ty {
        AttrType::RawString => Some(vec![Token::String(value.to_string())]),
        AttrType::Number => number(value),
        AttrType::Unit(unit) => {
            let tokens = number(value)?;
            let (sign, n) = match tokens.as_slice() {
                [Token::Delim('-'), n] => (Some(Token::Delim('-')), n),
                [n] => (None, n),
                _ => return None,
            };
            let (v, integer) = match n {
                Token::Number(i) => (*i as f64, true),
                Token::Float(f) => (*f, false),
                _ => return None,
            };
            let leaf = if unit == "%" {
                Token::Percentage(v)
            } else {
                Token::Dimension {
                    value: v,
                    integer,
                    unit: unit.clone(),
                }
            };
            Some(sign.into_iter().chain([leaf]).collect())
        }
        AttrType::Syntax(syntax) => {
            // CSS parsing applies, once; the attribute's text is not
            // searched for further substitution functions (DIVERGENCES).
            let tokens = tokenize(value).ok()?;
            syntax.matches_tokens(&tokens).then_some(tokens)
        }
    }
}

/// `value`, whitespace-trimmed, as one `<number-token>` (the tokenizer
/// keeps a sign apart: `-3` is `Delim('-')`, `Number(3)`).
fn number(value: &str) -> Option<Vec<Token>> {
    let tokens = tokenize(value.trim()).ok()?;
    match tokens.as_slice() {
        [Token::Number(_) | Token::Float(_)] => Some(tokens),
        [Token::Delim('-'), Token::Number(_) | Token::Float(_)] => Some(tokens),
        [Token::Delim('+'), n @ (Token::Number(_) | Token::Float(_))] => Some(vec![n.clone()]),
        _ => None,
    }
}

/// A CSS unit name (ASCII case-insensitive): those rdom resolves and the
/// others CSS defines, which then fail the property's own grammar —
/// invalid at computed-value time — rather than taking the fallback. Any
/// other identifier is no `<attr-type>` (a parse error).
fn is_known_unit(unit: &str) -> bool {
    const OTHER_CSS_UNITS: &[&str] = &[
        "px", "cm", "mm", "q", "in", "pt", "pc", "em", "rem", "ex", "rex", "cap", "rcap", "ic",
        "ric", "rch", "cqw", "cqh", "cqi", "cqb", "cqmin", "cqmax", "fr", "s", "ms", "hz", "khz",
        "dpi", "dpcm", "dppx", "x",
    ];
    crate::calc::CalcUnit::parse(unit).is_some()
        || OTHER_CSS_UNITS.iter().any(|u| u.eq_ignore_ascii_case(unit))
}

/// Test-only: how many `attr()` heads were parsed on this thread.
#[cfg(test)]
pub(crate) mod probe {
    thread_local! {
        pub static HEAD_PARSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    pub fn take_head_parses() -> usize {
        HEAD_PARSES.with(|c| c.replace(0))
    }
}
