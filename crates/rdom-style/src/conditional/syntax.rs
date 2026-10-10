//! Component values (CSS Syntax 3 §5.4.8) for the conditional preludes,
//! and the shared `not` / `and` / `or` grammar over them.
//!
//! A prelude is tokenized once ([`Prelude::parse`]) and grouped into a
//! tree whose blocks are the parenthesized groups and functions; a
//! condition's leaves are those blocks. Token spans are kept so a leaf
//! can be cut out as written (`<general-enclosed>`, a `@supports`
//! declaration) and so whitespace between two tokens can be seen (`>=`
//! is one comparison only without it).

use crate::parse::Token;
use crate::parse::token::{SpannedTokens, tokenize_spans};

use super::Condition;

/// A tokenized prelude.
pub(crate) struct Prelude<'a> {
    pub(crate) text: &'a str,
    pub(crate) tokens: SpannedTokens,
}

/// One component value: a token, or a block — `( … )` or a function —
/// holding its contents.
#[derive(Debug, Clone)]
pub(crate) enum Cv {
    Token(usize),
    /// A `(` block: the `(` token's index, its contents, the `)`'s index
    /// (`None` when the input ended first).
    Paren {
        open: usize,
        inner: Vec<Cv>,
        close: Option<usize>,
    },
    /// A function: its token's index (`Token::Function(name)`), its
    /// arguments, the `)`'s index.
    Function {
        at: usize,
        inner: Vec<Cv>,
        close: Option<usize>,
    },
}

impl Cv {
    /// The first and last token indices it covers.
    pub(crate) fn range(&self) -> (usize, usize) {
        match self {
            Cv::Token(i) => (*i, *i),
            Cv::Paren { open, inner, close } => {
                (*open, close.unwrap_or_else(|| last_index(inner, *open)))
            }
            Cv::Function { at, inner, close } => {
                (*at, close.unwrap_or_else(|| last_index(inner, *at)))
            }
        }
    }
}

fn last_index(inner: &[Cv], start: usize) -> usize {
    inner.last().map_or(start, |c| c.range().1)
}

impl<'a> Prelude<'a> {
    /// Tokenize `text`; `None` when it does not tokenize (an
    /// unterminated string or comment).
    /// `None`, too, when its blocks nest deeper than
    /// [`MAX_CONDITION_NESTING`](super::MAX_CONDITION_NESTING): grouping
    /// and the condition grammar recurse once per block (C16G-DEPTH-CAPS).
    pub(crate) fn parse(text: &'a str) -> Option<Self> {
        let tokens = tokenize_spans(text, 1, 1).ok()?;
        let mut depth = 0usize;
        for token in &tokens.tokens {
            match token {
                Token::LParen | Token::Function(_) => {
                    depth += 1;
                    if depth > super::MAX_CONDITION_NESTING {
                        return None;
                    }
                }
                Token::RParen => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        Some(Prelude { text, tokens })
    }

    pub(crate) fn token(&self, i: usize) -> &Token {
        &self.tokens.tokens[i]
    }

    /// The component values of the whole prelude.
    pub(crate) fn values(&self) -> Vec<Cv> {
        let mut i = 0;
        let (values, _) = self.group(&mut i, false);
        values
    }

    /// Group tokens from `*i` until the input ends or — `nested` — a
    /// `)` closes the block (its index returned).
    fn group(&self, i: &mut usize, nested: bool) -> (Vec<Cv>, Option<usize>) {
        let mut out = Vec::new();
        while *i < self.tokens.tokens.len() {
            let at = *i;
            *i += 1;
            match &self.tokens.tokens[at] {
                Token::RParen if nested => return (out, Some(at)),
                Token::LParen => {
                    let (inner, close) = self.group(i, true);
                    out.push(Cv::Paren {
                        open: at,
                        inner,
                        close,
                    });
                }
                Token::Function(_) => {
                    let (inner, close) = self.group(i, true);
                    out.push(Cv::Function { at, inner, close });
                }
                _ => out.push(Cv::Token(at)),
            }
        }
        (out, None)
    }

    /// The source text of `cv`, as written.
    pub(crate) fn text_of(&self, cv: &Cv) -> &'a str {
        let (a, b) = cv.range();
        &self.text[self.tokens.spans[a].start..self.tokens.spans[b].end]
    }

    /// The source text of `values`, as written (empty for none).
    pub(crate) fn text_of_all(&self, values: &[Cv]) -> &'a str {
        match (values.first(), values.last()) {
            (Some(first), Some(last)) => {
                let a = first.range().0;
                let b = last.range().1;
                &self.text[self.tokens.spans[a].start..self.tokens.spans[b].end]
            }
            _ => "",
        }
    }

    /// Whether token `i + 1` follows token `i` with nothing between.
    pub(crate) fn adjacent(&self, i: usize) -> bool {
        self.tokens
            .spans
            .get(i + 1)
            .is_some_and(|next| next.start == self.tokens.spans[i].end)
    }

    /// `cv` is the identifier `name` (ASCII case-insensitive).
    pub(crate) fn is_ident(&self, cv: &Cv, name: &str) -> bool {
        matches!(cv, Cv::Token(i) if matches!(self.token(*i), Token::Ident(s) if s.eq_ignore_ascii_case(name)))
    }

    /// `cv`'s identifier, if it is one.
    pub(crate) fn ident(&self, cv: &Cv) -> Option<&str> {
        match cv {
            Cv::Token(i) => match self.token(*i) {
                Token::Ident(s) => Some(s),
                _ => None,
            },
            _ => None,
        }
    }

    /// Parse `values` as `<X-condition>` (`and_or`: `or` allowed — a
    /// `<media-condition-without-or>` passes `false`): `not <in-parens>`
    /// or `<in-parens> [and <in-parens>]*` or `<in-parens> [or
    /// <in-parens>]*`. Each `<in-parens>` is a `( <X-condition> )`, else a
    /// leaf `leaf` accepts, else a `<general-enclosed>`. `None` when
    /// `values` is not a condition.
    pub(crate) fn condition<L>(
        &self,
        values: &[Cv],
        or_allowed: bool,
        leaf: &impl Fn(&Self, &Cv) -> Option<L>,
    ) -> Option<Condition<L>> {
        let (first, rest) = values.split_first()?;
        if self.is_ident(first, "not") {
            let [operand] = rest else { return None };
            return Some(Condition::Not(Box::new(self.in_parens(operand, leaf)?)));
        }
        let head = self.in_parens(first, leaf)?;
        if rest.is_empty() {
            return Some(head);
        }
        let joiner = match self.ident(&rest[0]) {
            Some(j) if j.eq_ignore_ascii_case("and") => "and",
            Some(j) if j.eq_ignore_ascii_case("or") && or_allowed => "or",
            _ => return None,
        };
        let mut parts = vec![head];
        let mut it = rest.chunks(2);
        for pair in &mut it {
            let [word, operand] = pair else { return None };
            if !self.is_ident(word, joiner) {
                return None;
            }
            parts.push(self.in_parens(operand, leaf)?);
        }
        Some(if joiner == "and" {
            Condition::And(parts)
        } else {
            Condition::Or(parts)
        })
    }

    /// `<X-in-parens>`: a nested condition, a leaf, or a
    /// `<general-enclosed>` — `None` for a bare token.
    fn in_parens<L>(
        &self,
        cv: &Cv,
        leaf: &impl Fn(&Self, &Cv) -> Option<L>,
    ) -> Option<Condition<L>> {
        match cv {
            Cv::Paren { inner, close, .. } => {
                if close.is_some()
                    && let Some(nested) = self.condition(inner, true, leaf)
                {
                    return Some(nested);
                }
                Some(match leaf(self, cv) {
                    Some(l) => Condition::Leaf(l),
                    None => Condition::Unknown(self.text_of(cv).to_string()),
                })
            }
            Cv::Function { .. } => Some(match leaf(self, cv) {
                Some(l) => Condition::Leaf(l),
                None => Condition::Unknown(self.text_of(cv).to_string()),
            }),
            Cv::Token(_) => None,
        }
    }
}

/// Split `values` at top-level commas.
pub(crate) fn split_commas<'v>(prelude: &Prelude<'_>, values: &'v [Cv]) -> Vec<&'v [Cv]> {
    values
        .split(|cv| matches!(cv, Cv::Token(i) if *prelude.token(*i) == Token::Comma))
        .collect()
}
