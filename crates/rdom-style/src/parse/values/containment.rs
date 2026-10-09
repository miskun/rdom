//! The query-container values (CSS Conditional 5 §6.1–§6.3):
//! `container-type`, `container-name` and the `container` shorthand.

use std::sync::Arc;

use crate::layout::{ContainerName, ContainerSize, ContainerType};
use crate::parse::token::Token;

/// `container-type`: `normal | [ [ size | inline-size ] || scroll-state ]`.
pub fn parse_container_type(tokens: &[Token]) -> Option<ContainerType> {
    let idents: Vec<String> = tokens
        .iter()
        .map(|t| match t {
            Token::Ident(s) => Some(s.to_ascii_lowercase()),
            _ => None,
        })
        .collect::<Option<_>>()?;
    match idents.as_slice() {
        [one] if one == "normal" => return Some(ContainerType::default()),
        [] => return None,
        _ => {}
    }
    let mut out = ContainerType::default();
    let mut size_seen = false;
    for word in &idents {
        match word.as_str() {
            "size" | "inline-size" if !size_seen => {
                size_seen = true;
                out.size = if word == "size" {
                    ContainerSize::Size
                } else {
                    ContainerSize::InlineSize
                };
            }
            "scroll-state" if !out.scroll_state => out.scroll_state = true,
            _ => return None,
        }
    }
    Some(out)
}

/// `container-name`: `none | <custom-ident>+`, the names excluding `none`,
/// `and`, `not`, `or` (§6.2) and the CSS-wide keywords and `default`
/// (CSS Values 4 §3.2).
pub fn parse_container_name(tokens: &[Token]) -> Option<ContainerName> {
    if let [Token::Ident(s)] = tokens
        && s.eq_ignore_ascii_case("none")
    {
        return Some(ContainerName::none());
    }
    if tokens.is_empty() {
        return None;
    }
    let names = tokens
        .iter()
        .map(|t| match t {
            Token::Ident(s) if is_container_name(s) => Some(Arc::<str>::from(s.as_str())),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ContainerName::new(names))
}

/// Whether `s` may name a container (§6.2).
fn is_container_name(s: &str) -> bool {
    ![
        "none",
        "and",
        "not",
        "or",
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

/// `container`: `<'container-name'> [ / <'container-type'> ]?` (§6.3);
/// an omitted type is `normal`.
pub fn parse_container(tokens: &[Token]) -> Option<(ContainerName, ContainerType)> {
    let slash = tokens.iter().position(|t| *t == Token::Delim('/'));
    let (name, kind) = match slash {
        Some(i) => (&tokens[..i], Some(&tokens[i + 1..])),
        None => (tokens, None),
    };
    let name = parse_container_name(name)?;
    let kind = match kind {
        Some(k) => parse_container_type(k)?,
        None => ContainerType::default(),
    };
    Some((name, kind))
}
