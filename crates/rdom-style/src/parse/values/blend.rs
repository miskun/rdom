//! The compositing properties (Compositing and Blending 1 §3.2, §3.4,
//! §5.2; Compositing 2 §9): `mix-blend-mode`, `background-blend-mode`,
//! `isolation`.

use std::borrow::Cow;

use super::numeric::split_commas;
use crate::layout::{BlendMode, Isolation};
use crate::parse::token::Token;

fn mode(value: &[Token]) -> Option<BlendMode> {
    let [Token::Ident(k)] = value else {
        return None;
    };
    BlendMode::KEYWORDS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(k))
        .map(|(_, m)| *m)
}

/// `mix-blend-mode`: `<blend-mode> | plus-darker | plus-lighter`.
pub fn parse_mix_blend_mode(value: &[Token]) -> Option<BlendMode> {
    mode(value)
}

/// `background-blend-mode`: `<blend-mode>#`.
pub fn parse_background_blend_mode(value: &[Token]) -> Option<Cow<'static, [BlendMode]>> {
    let modes = split_commas(value)?
        .into_iter()
        .map(|v| mode(v).filter(|m| m.is_blend_mode()))
        .collect::<Option<Vec<_>>>()?;
    Some(Cow::Owned(modes))
}

/// `isolation`: `auto | isolate`.
pub fn parse_isolation(value: &[Token]) -> Option<Isolation> {
    match value {
        [Token::Ident(k)] if k.eq_ignore_ascii_case("auto") => Some(Isolation::Auto),
        [Token::Ident(k)] if k.eq_ignore_ascii_case("isolate") => Some(Isolation::Isolate),
        _ => None,
    }
}
