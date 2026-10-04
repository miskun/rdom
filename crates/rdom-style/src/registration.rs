//! Registered custom properties (CSS Properties and Values API 1):
//! `@property` and `CSS.registerProperty`.
//!
//! A [`PropertyRegistration`] gives a custom property a syntax, an
//! inheritance flag and an initial value. The cascade then
//!
//! - starts the property at its initial value (and, when it does not
//!   inherit, resets it at every element);
//! - validates its value — after `var()` substitution — against the
//!   syntax, treating a mismatch as invalid at computed-value time
//!   (the property is `unset`: inherited or initial);
//! - animates it as its syntax type when that type interpolates
//!   ([`PropertySyntax::interpolation`]).
//!
//! The syntax components rdom checks are those its value parsers can:
//! `*`, `<length>` (cells and `calc()`), `<number>`, `<integer>`,
//! `<percentage>`, `<length-percentage>`, `<color>`, `<time>`,
//! `<custom-ident>` and literal identifiers, each optionally with the
//! `+` (space-separated) or `#` (comma-separated) multiplier, combined
//! with `|`. `<angle>`, `<resolution>`, `<image>`, `<url>`,
//! `<transform-function>` and `<transform-list>` have no terminal
//! value parser; a syntax naming one is rejected, so the registration
//! is invalid (Properties and Values 1 §5.4: an unsupported syntax is a
//! syntax error).

use crate::parse::token::{Token, tokenize};
use crate::parse::values::{looks_like_calc, parse_color, parse_color_at, parse_length};

/// One component of a registered syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SyntaxComponent {
    Length,
    Number,
    Integer,
    Percentage,
    LengthPercentage,
    Color,
    Time,
    CustomIdent,
    /// A literal keyword (`auto` in `<length> | auto`), matched
    /// case-sensitively.
    Ident(String),
}

/// How many times a component repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Multiplier {
    One,
    /// `+`: one or more, space-separated.
    SpaceList,
    /// `#`: one or more, comma-separated.
    CommaList,
}

/// A parsed `syntax` descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PropertySyntax {
    /// `*`: any token sequence (an unregistered custom property).
    Universal,
    /// `a | b | …`.
    Alternatives(Vec<(SyntaxComponent, Multiplier)>),
}

impl PropertySyntax {
    /// Parse a syntax string (Properties and Values 1 §5).
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        if text == "*" {
            return Ok(PropertySyntax::Universal);
        }
        let mut alternatives = Vec::new();
        for part in text.split('|') {
            let part = part.trim();
            let (body, multiplier) = match part.strip_suffix('+') {
                Some(b) => (b, Multiplier::SpaceList),
                None => match part.strip_suffix('#') {
                    Some(b) => (b, Multiplier::CommaList),
                    None => (part, Multiplier::One),
                },
            };
            let component = match body {
                "<length>" => SyntaxComponent::Length,
                "<number>" => SyntaxComponent::Number,
                "<integer>" => SyntaxComponent::Integer,
                "<percentage>" => SyntaxComponent::Percentage,
                "<length-percentage>" => SyntaxComponent::LengthPercentage,
                "<color>" => SyntaxComponent::Color,
                "<time>" => SyntaxComponent::Time,
                "<custom-ident>" => SyntaxComponent::CustomIdent,
                b if b.starts_with('<') => {
                    return Err(format!("unsupported syntax component `{b}`"));
                }
                b if is_keyword(b) => SyntaxComponent::Ident(b.to_string()),
                b => return Err(format!("invalid syntax component `{b}`")),
            };
            alternatives.push((component, multiplier));
        }
        Ok(PropertySyntax::Alternatives(alternatives))
    }

    /// Does `value` (a custom property's value text, `var()` already
    /// substituted) match the syntax?
    pub fn matches(&self, value: &str) -> bool {
        match self {
            PropertySyntax::Universal => true,
            PropertySyntax::Alternatives(alts) => {
                let Ok(tokens) = tokenize(value) else {
                    return false;
                };
                alts.iter().any(|(c, m)| matches_term(c, *m, &tokens))
            }
        }
    }

    /// The type a value of this syntax interpolates as, when it is a
    /// single color or number-like component (Properties and Values 1
    /// §6.2); `None`: the property animates discretely.
    pub fn interpolation(&self) -> Option<&SyntaxComponent> {
        match self {
            PropertySyntax::Alternatives(alts) => match alts.as_slice() {
                [(c, Multiplier::One)] => matches!(
                    c,
                    SyntaxComponent::Color
                        | SyntaxComponent::Number
                        | SyntaxComponent::Integer
                        | SyntaxComponent::Length
                        | SyntaxComponent::Percentage
                )
                .then_some(c),
                _ => None,
            },
            PropertySyntax::Universal => None,
        }
    }
}

/// A registered custom property.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PropertyRegistration {
    /// The name without the leading `--`.
    pub name: String,
    pub syntax: PropertySyntax,
    /// Whether the property inherits.
    pub inherits: bool,
    /// The initial value's text; `None` only for the `*` syntax.
    pub initial_value: Option<String>,
}

impl PropertyRegistration {
    /// Validate a registration as `CSS.registerProperty` and `@property`
    /// do (Properties and Values 1 §3, §4): `name` is a custom property
    /// name (`--x`), `syntax` parses, and — unless the syntax is `*` —
    /// `initial_value` is given, matches the syntax and holds no
    /// `var()` (it must be computationally independent).
    pub fn new(
        name: &str,
        syntax: &str,
        inherits: bool,
        initial_value: Option<&str>,
    ) -> Result<Self, String> {
        let bare = name
            .strip_prefix("--")
            .filter(|n| !n.is_empty())
            .ok_or_else(|| format!("`{name}` is not a custom property name"))?;
        let syntax = PropertySyntax::parse(syntax)?;
        let initial_value = initial_value.map(|v| v.trim().to_string());
        if let Some(v) = &initial_value
            && tokenize(v).is_ok_and(|t| crate::var::contains_var(&t))
        {
            return Err("the initial value is not computationally independent".to_string());
        }
        match (&syntax, &initial_value) {
            (PropertySyntax::Universal, _) => {}
            (_, None) => return Err("an initial value is required".to_string()),
            (_, Some(v)) if !syntax.matches(v) => {
                return Err(format!("the initial value `{v}` does not match the syntax"));
            }
            _ => {}
        }
        Ok(PropertyRegistration {
            name: bare.to_string(),
            syntax,
            inherits,
            initial_value,
        })
    }
}

fn is_keyword(s: &str) -> bool {
    rdom_core::css_syntax::would_start_ident(s)
        && rdom_core::css_syntax::consume_ident(s).1 == s.len()
        && ![
            "initial",
            "inherit",
            "unset",
            "revert",
            "revert-layer",
            "default",
        ]
        .iter()
        .any(|k| s.eq_ignore_ascii_case(k))
}

fn matches_term(component: &SyntaxComponent, multiplier: Multiplier, tokens: &[Token]) -> bool {
    match multiplier {
        Multiplier::One => consume(component, tokens, 0) == Some(tokens.len()),
        Multiplier::CommaList => top_level_segments(tokens)
            .iter()
            .all(|seg| !seg.is_empty() && consume(component, seg, 0) == Some(seg.len())),
        Multiplier::SpaceList => {
            let mut at = 0;
            while at < tokens.len() {
                match consume(component, tokens, at) {
                    Some(n) if n > at => at = n,
                    _ => return false,
                }
            }
            at > 0
        }
    }
}

/// `tokens` split on the commas outside parentheses.
fn top_level_segments(tokens: &[Token]) -> Vec<&[Token]> {
    let mut out = Vec::new();
    let (mut depth, mut start) = (0usize, 0usize);
    for (i, t) in tokens.iter().enumerate() {
        match t {
            Token::Function(_) | Token::LParen => depth += 1,
            Token::RParen => depth = depth.saturating_sub(1),
            Token::Comma if depth == 0 => {
                out.push(&tokens[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&tokens[start..]);
    out
}

/// Match one `component` at `tokens[at..]`; the index after it.
fn consume(component: &SyntaxComponent, tokens: &[Token], at: usize) -> Option<usize> {
    let rest = &tokens[at..];
    let signed = |accept: &dyn Fn(&Token) -> bool| match rest {
        [Token::Delim('-'), t, ..] if accept(t) => Some(at + 2),
        [t, ..] if accept(t) => Some(at + 1),
        _ => None,
    };
    match component {
        SyntaxComponent::Number => signed(&|t| matches!(t, Token::Number(_) | Token::Float(_))),
        SyntaxComponent::Integer => signed(&|t| matches!(t, Token::Number(_))),
        SyntaxComponent::Percentage => signed(&|t| matches!(t, Token::Percentage(_))),
        SyntaxComponent::Length | SyntaxComponent::LengthPercentage => {
            let percent = *component == SyntaxComponent::LengthPercentage;
            if let Some(end) = signed(&|t| {
                matches!(t, Token::Number(_)) || (percent && matches!(t, Token::Percentage(_)))
            }) {
                return Some(end);
            }
            // `calc()`: through its closing parenthesis.
            let end = calc_end(rest)?;
            let calc = &rest[..end];
            (looks_like_calc(calc)
                && parse_length(calc).is_some()
                && (percent || !calc.iter().any(|t| matches!(t, Token::Percentage(_)))))
            .then_some(at + end)
        }
        SyntaxComponent::Color => {
            if parse_color(rest).is_some() {
                return Some(tokens.len());
            }
            let (_, used) = parse_color_at(tokens, at)?;
            Some(at + used)
        }
        SyntaxComponent::Time => {
            crate::parse::values::parse_time_ms(rest.get(..1)?).map(|_| at + 1)
        }
        SyntaxComponent::CustomIdent => match rest.first()? {
            Token::Ident(s) if is_keyword(s) => Some(at + 1),
            _ => None,
        },
        SyntaxComponent::Ident(k) => match rest.first()? {
            Token::Ident(s) if s == k => Some(at + 1),
            _ => None,
        },
    }
}

/// The length of a leading function call (through its `)`).
fn calc_end(tokens: &[Token]) -> Option<usize> {
    if !matches!(tokens.first()?, Token::Function(_)) {
        return None;
    }
    let mut depth = 0usize;
    for (i, t) in tokens.iter().enumerate() {
        match t {
            Token::Function(_) | Token::LParen => depth += 1,
            Token::RParen => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn syntax(s: &str) -> PropertySyntax {
        PropertySyntax::parse(s).unwrap()
    }

    /// Properties and Values 1 §5: the supported components, with
    /// multipliers and alternatives.
    #[test]
    fn syntax_matching() {
        assert!(syntax("<color>").matches("red"));
        assert!(syntax("<color>").matches("rgb(1, 2, 3)"));
        assert!(!syntax("<color>").matches("12"));
        assert!(syntax("<length>").matches("5"));
        assert!(syntax("<length>").matches("-2"));
        assert!(syntax("<length>").matches("calc(2 + 3)"));
        assert!(!syntax("<length>").matches("50%"));
        assert!(syntax("<length-percentage>").matches("50%"));
        assert!(syntax("<number>").matches("0.5"));
        assert!(!syntax("<integer>").matches("0.5"));
        assert!(syntax("<time>").matches("2s"));
        assert!(syntax("<length> | auto").matches("auto"));
        assert!(!syntax("<length> | auto").matches("none"));
        assert!(syntax("<custom-ident>").matches("foo"));
        assert!(!syntax("<custom-ident>").matches("inherit"));
        assert!(syntax("<number>+").matches("1 2 3"));
        assert!(syntax("<color>#").matches("red, rgb(0, 0, 255)"));
        assert!(!syntax("<color>#").matches("red blue"));
        assert!(syntax("*").matches("anything { at all }"));
    }

    /// §5.4: a component rdom cannot check makes the syntax invalid.
    #[test]
    fn unsupported_components_are_rejected() {
        for s in [
            "<angle>",
            "<image>",
            "<url>",
            "<transform-list>",
            "<nope>",
            "a b",
        ] {
            assert!(PropertySyntax::parse(s).is_err(), "{s}");
        }
    }

    /// §3: the registration rules `CSS.registerProperty` checks.
    #[test]
    fn registration_validation() {
        assert!(PropertyRegistration::new("--c", "<color>", false, Some("red")).is_ok());
        assert!(PropertyRegistration::new("c", "<color>", false, Some("red")).is_err());
        assert!(PropertyRegistration::new("--c", "<color>", false, None).is_err());
        assert!(PropertyRegistration::new("--c", "<color>", false, Some("12")).is_err());
        assert!(PropertyRegistration::new("--c", "<color>", false, Some("var(--d)")).is_err());
        assert!(PropertyRegistration::new("--c", "*", true, None).is_ok());
    }
}
