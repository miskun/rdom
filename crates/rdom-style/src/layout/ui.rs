//! The CSS Basic User Interface 4 values: the outline (§5) — and
//! [`UiStyle`], the computed group of the user-interface properties.

use super::{BorderStyle, BorderWidth, PaintLength};

/// `outline-style` (CSS UI 4 §5.2): `auto | <outline-line-style>`, every
/// `<line-style>` but `hidden` (rdom's own `half-block` included in
/// neither). Not inherited; initial `none`.
///
/// Closed (DESIGN): the painter maps each one to a glyph set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum OutlineStyle {
    /// No outline (the initial value).
    #[default]
    None,
    /// The user agent's focus-ring look: in rdom a light single line with
    /// rounded corners, `outline-color: auto` drawing it in the accent
    /// color.
    Auto,
    Solid,
    Double,
    Dashed,
    Dotted,
    Ridge,
    Outset,
    Groove,
    Inset,
}

impl OutlineStyle {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, OutlineStyle)] = &[
        ("none", OutlineStyle::None),
        ("auto", OutlineStyle::Auto),
        ("solid", OutlineStyle::Solid),
        ("double", OutlineStyle::Double),
        ("dashed", OutlineStyle::Dashed),
        ("dotted", OutlineStyle::Dotted),
        ("ridge", OutlineStyle::Ridge),
        ("outset", OutlineStyle::Outset),
        ("groove", OutlineStyle::Groove),
        ("inset", OutlineStyle::Inset),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        Self::KEYWORDS
            .iter()
            .find(|(_, s)| *s == self)
            .map_or("none", |(k, _)| k)
    }

    /// The border line the ring draws with — `auto` a solid one — or
    /// `None` for `none`.
    pub fn line(self) -> Option<BorderStyle> {
        Some(match self {
            OutlineStyle::None => return None,
            OutlineStyle::Auto | OutlineStyle::Solid => BorderStyle::Solid,
            OutlineStyle::Double => BorderStyle::Double,
            OutlineStyle::Dashed => BorderStyle::Dashed,
            OutlineStyle::Dotted => BorderStyle::Dotted,
            OutlineStyle::Ridge => BorderStyle::Ridge,
            OutlineStyle::Outset => BorderStyle::Outset,
            OutlineStyle::Groove => BorderStyle::Groove,
            OutlineStyle::Inset => BorderStyle::Inset,
        })
    }
}

/// `outline-color` (CSS UI 4 §5.3): `auto | <color>`. `auto` is the
/// accent color under `outline-style: auto` and `currentcolor` otherwise;
/// a color stays a [`TuiColor`](crate::TuiColor) and resolves at paint
/// against the element, as `caret-color` does. Not inherited; initial
/// `auto`.
///
/// Closed (DESIGN): a color or the keyword.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum OutlineColor {
    #[default]
    Auto,
    Color(crate::TuiColor),
}

/// The computed CSS UI 4 properties of an element
/// ([`ComputedStyle::ui`](crate::ComputedStyle::ui)): the outline's four
/// longhands. None of them inherits.
///
/// Closed (DESIGN), as the other style groups: a new field fails a
/// destructuring pattern. `Default` is the initial values.
#[derive(Debug, Clone, PartialEq)]
pub struct UiStyle {
    /// `outline-style` (§5.2).
    pub outline_style: OutlineStyle,
    /// `outline-width` (§5.3): a `<line-width>`, viewport units resolved;
    /// it selects the ring's glyph weight as a border width does.
    pub outline_width: BorderWidth,
    /// `outline-color` (§5.3).
    pub outline_color: OutlineColor,
    /// `outline-offset` (§5.4), viewport units resolved: whole cells
    /// outside the border edge, a pixel length one cell its way
    /// ([`PaintLength::offset_cells`]).
    pub outline_offset: PaintLength,
}

impl Default for UiStyle {
    fn default() -> Self {
        UiStyle {
            outline_style: OutlineStyle::None,
            outline_width: BorderWidth::Medium,
            outline_color: OutlineColor::Auto,
            outline_offset: PaintLength::Cells(0.0),
        }
    }
}
