//! The look checks (`specs/CAPTURE.md` §Audits, Look): the ones about
//! color ([`paint`]: contrast, palette, depth-loss, profile) and the ones
//! about layout ([`layout`]: truncation, occlusion, alignment, rhythm).

mod layout;
mod paint;

pub(in crate::audit) use layout::{alignment, occlusion, rhythm, truncation};
pub(in crate::audit) use paint::{contrast, depth_loss, palette, profile};

use crate::profile::Rgb;

pub(super) fn hex([r, g, b]: Rgb) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

pub(super) fn round(v: f64, places: i32) -> f64 {
    let p = 10f64.powi(places);
    (v * p).round() / p
}

/// Whether a cell is text: an element's or a pseudo-element's, selected or
/// highlighted text, an ellipsis — or a control's chrome showing a letter,
/// a digit, punctuation or a bullet (a closed `<select>`'s label, a
/// password's dots). Other glyphs are graphics: borders, marks.
pub(super) fn is_text(part: &str, glyph: &str) -> bool {
    matches!(part, "text" | "selection" | "highlight" | "text-overflow")
        || part.starts_with("pseudo:")
        || (part == "ua-chrome"
            && glyph
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric() || c.is_ascii_punctuation() || c == '•'))
}

/// Block elements and shades: graphics whose background never shows.
pub(super) fn is_block(glyph: &str) -> bool {
    glyph
        .chars()
        .next()
        .is_some_and(|c| ('\u{2580}'..='\u{259F}').contains(&c))
}
