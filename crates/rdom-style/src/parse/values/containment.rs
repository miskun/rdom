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

/// `contain`: `none | strict | content | [ [ size | inline-size ] ||
/// layout || style || paint ]` (CSS Containment 2 §2, Containment 3).
pub fn parse_contain(tokens: &[Token]) -> Option<crate::layout::Contain> {
    use crate::layout::Contain;
    let idents: Vec<String> = tokens
        .iter()
        .map(|t| match t {
            Token::Ident(s) => Some(s.to_ascii_lowercase()),
            _ => None,
        })
        .collect::<Option<_>>()?;
    match idents.as_slice() {
        [] => return None,
        [one] if one == "none" => return Some(Contain::NONE),
        [one] if one == "strict" => return Some(Contain::STRICT),
        [one] if one == "content" => return Some(Contain::CONTENT),
        _ => {}
    }
    let mut out = Contain::NONE;
    for word in &idents {
        let slot = match word.as_str() {
            "size" if !out.inline_size => &mut out.size,
            "inline-size" if !out.size => &mut out.inline_size,
            "layout" => &mut out.layout,
            "style" => &mut out.style,
            "paint" => &mut out.paint,
            _ => return None,
        };
        if std::mem::replace(slot, true) {
            return None;
        }
    }
    Some(out)
}

/// `will-change`: `auto | <animateable-feature>#`, a feature
/// `scroll-position`, `contents` or a `<custom-ident>` other than
/// `will-change`, `none`, `all`, `auto`, `scroll-position`, `contents` and
/// the CSS-wide keywords (CSS Will Change 1 §2).
pub fn parse_will_change(tokens: &[Token]) -> Option<crate::layout::WillChange> {
    if let [Token::Ident(s)] = tokens
        && s.eq_ignore_ascii_case("auto")
    {
        return Some(crate::layout::WillChange::auto());
    }
    let mut features = Vec::new();
    for item in tokens.split(|t| *t == Token::Comma) {
        let [Token::Ident(name)] = item else {
            return None;
        };
        let lower = name.to_ascii_lowercase();
        let keyword = matches!(lower.as_str(), "scroll-position" | "contents");
        let excluded = [
            "will-change",
            "none",
            "all",
            "auto",
            "initial",
            "inherit",
            "unset",
            "revert",
            "revert-layer",
            "default",
        ]
        .contains(&lower.as_str());
        if excluded {
            return None;
        }
        // A keyword or a property name is ASCII case-insensitive; any
        // other ident is kept as written.
        let known = keyword || crate::property_dispatch::property_names().contains(&lower.as_str());
        features.push(Arc::<str>::from(if known { lower } else { name.clone() }));
    }
    Some(crate::layout::WillChange::new(features))
}
