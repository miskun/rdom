//! The UA palette: the colors the user-agent sheet paints its chrome
//! with, defined once so every rule (and every color derived from the
//! chrome) agrees.

use super::{Color, named};

/// Field background tint — subtle dark warm gray. On dark
/// terminals it reads as a soft pillow under input/textarea text
/// (just visible enough to mark the field affordance). On light
/// terminals it's a dark rectangle.
pub(crate) const FIELD_BG: Color = Color::Rgb(0x1f, 0x21, 0x23);
/// Muted text — #7F868B. Cool gray that reads as supporting
/// prose against both light and dark surfaces. Used for
/// `:disabled`, placeholder, `<small>`, `<abbr>`, blockquote
/// text, scrollbar glyphs, helper text, etc.
pub(crate) const TEXT_MUTED: Color = Color::Rgb(0x7F, 0x86, 0x8B);
/// Default border — #3B4042. Subtle gray for box-drawing borders
/// and `<hr>` rules — distinct from `TEXT_MUTED` so a border
/// next to muted text still reads as chrome rather than as more
/// text.
pub(crate) const BORDER_DEFAULT: Color = Color::Rgb(0x3B, 0x40, 0x42);
/// Accent — dodgerblue (#1E90FF). Vivid on both light and dark
/// terminals. Replaces every former `ACCENT` use:
/// link fg, kbd, button fg, dialog border, focus glyph, select
/// chevron, range/progress bar accent, selected-option bg.
pub(crate) const ACCENT: Color = named::DODGERBLUE;
