//! `rgb()` / `rgba()` (CSS Color 4 §5.1).
//!
//! ```text
//! rgb() = rgb( [<number> | <percentage> | none]{3} [ / [<alpha-value> | none] ]? )
//!       | rgb( <percentage>#{3} , <alpha-value>? )
//!       | rgb( <number>#{3} , <alpha-value>? )
//! ```
//!
//! `rgba()` is the same function. A channel number is `0..=255`, a
//! percentage `0%..=100%` of 255; out-of-range values clamp.

use super::channel::{Arguments, Channel, ChannelType, alpha, split_arguments};
use crate::color::{AbsoluteColor, ColorSpace};
use crate::parse::token::Token;

/// Parse the arguments of `rgb()` / `rgba()`.
pub(super) fn parse(args: &[Token]) -> Option<AbsoluteColor> {
    let (channels, alpha_component, legacy) = match split_arguments(args)? {
        Arguments::Modern { channels, alpha } => (channels, alpha, false),
        Arguments::Legacy { channels, alpha } => (channels, alpha, true),
    };
    let [r, g, b] = channels.as_slice() else {
        return None;
    };
    let channels = [Channel::parse(r)?, Channel::parse(g)?, Channel::parse(b)?];
    if legacy {
        let ty = channels[0].ty();
        if ty == ChannelType::Other || channels.iter().any(|c| c.ty() != ty) {
            return None;
        }
    }
    let alpha = match alpha_component {
        Some(component) => alpha(component, !legacy)?,
        None => Some(1.0),
    };
    // Out-of-range values clamp at parsed-value time.
    let coords = channels.map(|c| c.resolve(255.0).map(|v| v.clamp(0.0, 255.0) / 255.0));
    Some(AbsoluteColor {
        space: ColorSpace::Srgb,
        coords,
        alpha: alpha.map(|a| a.clamp(0.0, 1.0)),
    })
}
