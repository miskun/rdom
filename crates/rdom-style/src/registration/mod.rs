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
//! `*`, `<length>` (cells, units and math functions), `<number>`,
//! `<integer>`, `<percentage>`, `<length-percentage>`, `<angle>`,
//! `<color>`, `<time>`, `<custom-ident>` and literal identifiers, each
//! optionally with the `+` (space-separated) or `#` (comma-separated)
//! multiplier, combined with `|`. `<resolution>`, `<image>`, `<url>`,
//! `<transform-function>` and `<transform-list>` have no terminal
//! value parser; a syntax naming one is rejected, so the registration
//! is invalid (Properties and Values 1 §5.4: an unsupported syntax is a
//! syntax error).

mod computed;

pub use computed::length_percentage_text;

use crate::parse::token::{Token, tokenize};
use crate::parse::values::{
    Range, integer, looks_like_calc, number, parse_angle, parse_color_at, parse_length, percentage,
};

/// One component of a registered syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SyntaxComponent {
    Length,
    /// `<angle>` (CSS Values 4 §7.1).
    Angle,
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

/// Why a `syntax` string does not parse (Properties and Values 1 §5).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PropertySyntaxError {
    /// A data type rdom has no value parser for (`<resolution>`, `<image>`,
    /// …; §5.4 treats an unsupported syntax as a syntax error).
    UnsupportedComponent(String),
    /// Neither a data type name nor a keyword (`a b`, `initial`, an
    /// empty alternative).
    InvalidComponent(String),
}

impl std::fmt::Display for PropertySyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PropertySyntaxError::UnsupportedComponent(c) => {
                write!(f, "unsupported syntax component `{c}`")
            }
            PropertySyntaxError::InvalidComponent(c) => {
                write!(f, "invalid syntax component `{c}`")
            }
        }
    }
}

impl std::error::Error for PropertySyntaxError {}

/// Why a custom property cannot be registered (Properties and Values 1
/// §3 `CSS.registerProperty`, §4 `@property`). Every variant is the web
/// API's `SyntaxError` except [`AlreadyRegistered`](Self::AlreadyRegistered),
/// its `InvalidModificationError`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegisterPropertyError {
    /// The name (without dashes) is registered already
    /// (`InvalidModificationError`).
    AlreadyRegistered(String),
    /// Not a custom property name (`--x`).
    InvalidName(String),
    /// The `syntax` does not parse.
    InvalidSyntax(PropertySyntaxError),
    /// A syntax other than `*` needs an initial value.
    MissingInitialValue,
    /// The initial value does not match the syntax.
    InitialValueMismatch(String),
    /// The initial value is not computationally independent (it holds
    /// `var()`).
    NotComputationallyIndependent(String),
}

impl RegisterPropertyError {
    /// Is this the web API's `InvalidModificationError` (rather than a
    /// `SyntaxError`)?
    pub fn is_invalid_modification(&self) -> bool {
        matches!(self, RegisterPropertyError::AlreadyRegistered(_))
    }
}

impl std::fmt::Display for RegisterPropertyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegisterPropertyError::AlreadyRegistered(name) => {
                write!(f, "--{name} is already registered")
            }
            RegisterPropertyError::InvalidName(name) => {
                write!(f, "`{name}` is not a custom property name")
            }
            RegisterPropertyError::InvalidSyntax(e) => e.fmt(f),
            RegisterPropertyError::MissingInitialValue => {
                f.write_str("an initial value is required")
            }
            RegisterPropertyError::InitialValueMismatch(v) => {
                write!(f, "the initial value `{v}` does not match the syntax")
            }
            RegisterPropertyError::NotComputationallyIndependent(_) => {
                f.write_str("the initial value is not computationally independent")
            }
        }
    }
}

impl std::error::Error for RegisterPropertyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RegisterPropertyError::InvalidSyntax(e) => Some(e),
            _ => None,
        }
    }
}

impl From<PropertySyntaxError> for RegisterPropertyError {
    fn from(e: PropertySyntaxError) -> Self {
        RegisterPropertyError::InvalidSyntax(e)
    }
}

impl PropertySyntax {
    /// Parse a syntax string (Properties and Values 1 §5).
    pub fn parse(text: &str) -> Result<Self, PropertySyntaxError> {
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
                "<angle>" => SyntaxComponent::Angle,
                "<number>" => SyntaxComponent::Number,
                "<integer>" => SyntaxComponent::Integer,
                "<percentage>" => SyntaxComponent::Percentage,
                "<length-percentage>" => SyntaxComponent::LengthPercentage,
                "<color>" => SyntaxComponent::Color,
                "<time>" => SyntaxComponent::Time,
                "<custom-ident>" => SyntaxComponent::CustomIdent,
                b if b.starts_with('<') => {
                    return Err(PropertySyntaxError::UnsupportedComponent(b.to_string()));
                }
                b if is_keyword(b) => SyntaxComponent::Ident(b.to_string()),
                b => return Err(PropertySyntaxError::InvalidComponent(b.to_string())),
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
            PropertySyntax::Alternatives(_) => {
                tokenize(value).is_ok_and(|tokens| self.matches_tokens(&tokens))
            }
        }
    }

    /// [`matches`](Self::matches) for a value already tokenized.
    pub(crate) fn matches_tokens(&self, tokens: &[Token]) -> bool {
        match self {
            PropertySyntax::Universal => true,
            PropertySyntax::Alternatives(alts) => {
                alts.iter().any(|(c, m)| matches_term(c, *m, tokens))
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
                        | SyntaxComponent::Angle
                        | SyntaxComponent::Number
                        | SyntaxComponent::Integer
                        | SyntaxComponent::Length
                        | SyntaxComponent::LengthPercentage
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
    ) -> Result<Self, RegisterPropertyError> {
        let bare = name
            .strip_prefix("--")
            .filter(|n| !n.is_empty())
            .ok_or_else(|| RegisterPropertyError::InvalidName(name.to_string()))?;
        let syntax = PropertySyntax::parse(syntax)?;
        let initial_value = initial_value.map(|v| v.trim().to_string());
        if let Some(v) = &initial_value
            && tokenize(v).is_ok_and(|t| crate::var::contains_substitution(&t))
        {
            return Err(RegisterPropertyError::NotComputationallyIndependent(
                v.clone(),
            ));
        }
        match (&syntax, &initial_value) {
            (PropertySyntax::Universal, _) => {}
            (_, None) => return Err(RegisterPropertyError::MissingInitialValue),
            (_, Some(v)) if !syntax.matches(v) => {
                return Err(RegisterPropertyError::InitialValueMismatch(v.clone()));
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
    // A math function at `rest` that `valid` accepts: the index after it.
    let math = |rest: &[Token], valid: &dyn Fn(&[Token]) -> bool| {
        let end = calc_end(rest)?;
        (looks_like_calc(&rest[..end]) && valid(&rest[..end])).then_some(at + end)
    };
    let signed = |accept: &dyn Fn(&Token) -> bool| match rest {
        [Token::Delim('-'), t, ..] if accept(t) => Some(at + 2),
        [t, ..] if accept(t) => Some(at + 1),
        _ => None,
    };
    match component {
        // A math function of the component's type is one too (CSS Values
        // 4 §10): `<number>` / `<integer>` (rounded) take a `<number>`
        // calculation, `<percentage>` one of percentages alone.
        SyntaxComponent::Number => signed(&|t| matches!(t, Token::Number(_) | Token::Float(_)))
            .or_else(|| math(rest, &|c| number(c, Range::Any).is_some())),
        SyntaxComponent::Integer => signed(&|t| matches!(t, Token::Number(_)))
            .or_else(|| math(rest, &|c| integer(c).is_some())),
        SyntaxComponent::Percentage => signed(&|t| matches!(t, Token::Percentage(_)))
            .or_else(|| math(rest, &|c| percentage(c).is_some())),
        SyntaxComponent::Length | SyntaxComponent::LengthPercentage => {
            let percent = *component == SyntaxComponent::LengthPercentage;
            if let Some(end) = signed(&|t| {
                matches!(t, Token::Number(_))
                    || (percent && matches!(t, Token::Percentage(_)))
                    || matches!(t, Token::Dimension { unit, .. }
                        if crate::calc::CalcUnit::parse(unit).is_some_and(|u| u.kind().is_length()))
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
        SyntaxComponent::Angle => {
            let end = match rest {
                [Token::Delim('-'), Token::Dimension { .. }, ..] => 2,
                [Token::Dimension { .. }, ..] => 1,
                _ => calc_end(rest)?,
            };
            parse_angle(&rest[..end]).map(|_| at + end)
        }
        // One parse: the color at `at`, through as many tokens as it uses
        // (all of them when it is the whole rest).
        SyntaxComponent::Color => parse_color_at(tokens, at).map(|(_, used)| at + used),
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
            "<resolution>",
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

    /// `C1G-TYPED-ERRORS` — Properties and Values 1 §3 / §5: each way a
    /// registration fails is its own variant; all are the web API's
    /// `SyntaxError` except a second registration
    /// (`InvalidModificationError`, raised by the `App`).
    #[test]
    fn registration_errors_are_typed() {
        use PropertySyntaxError as S;
        use RegisterPropertyError as E;
        assert_eq!(
            PropertySyntax::parse("<image>"),
            Err(S::UnsupportedComponent("<image>".into()))
        );
        assert_eq!(
            PropertySyntax::parse("a b"),
            Err(S::InvalidComponent("a b".into()))
        );
        assert_eq!(
            PropertyRegistration::new("c", "<color>", false, Some("red")),
            Err(E::InvalidName("c".into()))
        );
        assert_eq!(
            PropertyRegistration::new("--c", "<url>", false, Some("red")),
            Err(E::InvalidSyntax(S::UnsupportedComponent("<url>".into())))
        );
        assert_eq!(
            PropertyRegistration::new("--c", "<color>", false, None),
            Err(E::MissingInitialValue)
        );
        assert_eq!(
            PropertyRegistration::new("--c", "<color>", false, Some("12")),
            Err(E::InitialValueMismatch("12".into()))
        );
        assert_eq!(
            PropertyRegistration::new("--c", "<color>", false, Some("var(--d)")),
            Err(E::NotComputationallyIndependent("var(--d)".into()))
        );
        // The messages `@property` warnings carry.
        assert_eq!(
            E::InitialValueMismatch("12".into()).to_string(),
            "the initial value `12` does not match the syntax"
        );
        assert_eq!(
            E::AlreadyRegistered("c".into()).to_string(),
            "--c is already registered"
        );
        assert!(E::AlreadyRegistered("c".into()).is_invalid_modification());
        assert!(!E::MissingInitialValue.is_invalid_modification());
    }
}
