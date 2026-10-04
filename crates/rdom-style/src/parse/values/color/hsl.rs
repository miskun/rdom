//! `hsl()` / `hsla()` (CSS Color 4 §7.1) and `hwb()` (§8.1).
//!
//! ```text
//! hsl() = hsl( [<hue> | none] [<percentage> | <number> | none]{2} [ / [<alpha-value> | none] ]? )
//!       | hsl( <hue>, <percentage>, <percentage>, <alpha-value>? )
//! hwb() = hwb( [<hue> | none] [<percentage> | <number> | none]{2} [ / [<alpha-value> | none] ]? )
//! ```
//!
//! `hsla()` is `hsl()`. Saturation, lightness, whiteness and
//! blackness are kept as numbers on a 0–100 scale (a percentage is its
//! number); a negative saturation clamps to 0 at parsed-value time.

use super::channel::{Arguments, Channel, ChannelType, alpha, hue, split_arguments};
use crate::color::{AbsoluteColor, ColorSpace};
use crate::parse::token::Token;

/// Parse the arguments of `hsl()` / `hsla()`.
pub(super) fn parse_hsl(args: &[Token]) -> Option<AbsoluteColor> {
    let mut color = parse(args, ColorSpace::Hsl, true)?;
    color.coords[1] = color.coords[1].map(|s| s.max(0.0));
    Some(color)
}

/// Parse the arguments of `hwb()`.
pub(super) fn parse_hwb(args: &[Token]) -> Option<AbsoluteColor> {
    parse(args, ColorSpace::Hwb, false)
}

/// A hue followed by two 0–100 channels, in the modern syntax or —
/// where `legacy_allowed` — the legacy one (percentages only, no
/// `none`).
fn parse(args: &[Token], space: ColorSpace, legacy_allowed: bool) -> Option<AbsoluteColor> {
    let (channels, alpha_component, legacy) = match split_arguments(args)? {
        Arguments::Modern { channels, alpha } => (channels, alpha, false),
        Arguments::Legacy { channels, alpha } if legacy_allowed => (channels, alpha, true),
        Arguments::Legacy { .. } => return None,
    };
    let [h, a, b] = channels.as_slice() else {
        return None;
    };
    let h = hue(h, !legacy)?;
    let [a, b] = [Channel::parse(a)?, Channel::parse(b)?];
    if legacy && (a.ty() != ChannelType::Percent || b.ty() != ChannelType::Percent) {
        return None;
    }
    let alpha = match alpha_component {
        Some(component) => alpha(component, !legacy)?,
        None => Some(1.0),
    };
    Some(AbsoluteColor {
        space,
        coords: [h, a.resolve(100.0), b.resolve(100.0)],
        alpha: alpha.map(|a| a.clamp(0.0, 1.0)),
    })
}
