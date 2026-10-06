//! CSS Text Decoration 3 / 4: the line decorations (`text-decoration` and
//! its longhands), the underline's placement properties, and the
//! decorations in effect on an element's text once propagation is done
//! (§2.1).

use crate::Color;

/// `text-decoration-line: none | [underline || overline || line-through
/// || blink]` (CSS Text Decoration 4 §2.1). Not inherited (decorations
/// propagate instead, [`AppliedDecorations`]); initial `none`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct TextDecorationLine {
    /// A line below the text.
    pub underline: bool,
    /// A line above the text.
    pub overline: bool,
    /// A line through the middle of the text.
    pub line_through: bool,
    /// The text blinks.
    pub blink: bool,
}

impl TextDecorationLine {
    /// `none`.
    pub const NONE: Self = TextDecorationLine {
        underline: false,
        overline: false,
        line_through: false,
        blink: false,
    };
    /// `underline`.
    pub const UNDERLINE: Self = TextDecorationLine {
        underline: true,
        ..Self::NONE
    };
    /// `overline`.
    pub const OVERLINE: Self = TextDecorationLine {
        overline: true,
        ..Self::NONE
    };
    /// `line-through`.
    pub const LINE_THROUGH: Self = TextDecorationLine {
        line_through: true,
        ..Self::NONE
    };

    /// No line.
    pub const fn is_none(self) -> bool {
        !(self.underline || self.overline || self.line_through || self.blink)
    }

    /// The keywords, in the grammar's order; `none` for no line.
    pub fn keywords(self) -> String {
        let words: Vec<&str> = [
            (self.underline, "underline"),
            (self.overline, "overline"),
            (self.line_through, "line-through"),
            (self.blink, "blink"),
        ]
        .into_iter()
        .filter_map(|(on, w)| on.then_some(w))
        .collect();
        if words.is_empty() {
            "none".to_string()
        } else {
            words.join(" ")
        }
    }
}

impl From<super::TextDecoration> for TextDecorationLine {
    fn from(d: super::TextDecoration) -> Self {
        match d {
            super::TextDecoration::None => Self::NONE,
            super::TextDecoration::Underline => Self::UNDERLINE,
            super::TextDecoration::LineThrough => Self::LINE_THROUGH,
        }
    }
}

/// `text-decoration-style: solid | double | dotted | dashed | wavy` (CSS
/// Text Decoration 4 §2.3). Not inherited; initial `solid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum TextDecorationStyle {
    #[default]
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

impl TextDecorationStyle {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Solid => "solid",
            Self::Double => "double",
            Self::Dotted => "dotted",
            Self::Dashed => "dashed",
            Self::Wavy => "wavy",
        }
    }
}

/// `text-decoration-thickness: auto | from-font | <length-percentage>`
/// (CSS Text Decoration 4 §2.5). Not inherited; initial `auto`. Parsed
/// and kept for the CSSOM; the terminal draws its lines one thickness
/// (DIVERGENCES §1).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TextDecorationThickness {
    #[default]
    Auto,
    FromFont,
    Length(super::PaintLength),
}

/// `text-underline-offset: auto | <length-percentage>` (CSS Text
/// Decoration 4 §4.2). Inherited; initial `auto`. Parsed, not drawn.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TextUnderlineOffset {
    #[default]
    Auto,
    Length(super::PaintLength),
}

/// `text-underline-position: auto | from-font | [under || [left |
/// right]]` (CSS Text Decoration 4 §4.1). Inherited; initial `auto`.
/// Parsed, not drawn: the terminal places the underline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct TextUnderlinePosition {
    /// `from-font` (with no other keyword).
    pub from_font: bool,
    /// `under`.
    pub under: bool,
    /// `left` (`Some(false)`) or `right` (`Some(true)`).
    pub side: Option<bool>,
}

impl TextUnderlinePosition {
    /// `auto`.
    pub const AUTO: Self = TextUnderlinePosition {
        from_font: false,
        under: false,
        side: None,
    };

    /// The value's CSS spelling.
    pub fn keywords(self) -> String {
        if self.from_font {
            return "from-font".to_string();
        }
        let side = self.side.map(|right| if right { "right" } else { "left" });
        match (self.under, side) {
            (false, None) => "auto".to_string(),
            (true, None) => "under".to_string(),
            (false, Some(s)) => s.to_string(),
            (true, Some(s)) => format!("under {s}"),
        }
    }
}

/// `text-decoration-skip-ink: auto | none | all` (CSS Text Decoration 4
/// §3.2). Inherited; initial `auto`. Parsed, not drawn: skipping ink
/// needs glyph outlines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum TextDecorationSkipInk {
    #[default]
    Auto,
    None,
    All,
}

impl TextDecorationSkipInk {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::None => "none",
            Self::All => "all",
        }
    }
}

/// An element's computed line decoration properties
/// ([`ComputedStyle::text_decoration`](crate::ComputedStyle::text_decoration)),
/// the longhands of the `text-decoration` shorthand. None inherits.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextDecorations {
    /// `text-decoration-line`.
    pub line: TextDecorationLine,
    /// `text-decoration-style`.
    pub style: TextDecorationStyle,
    /// `text-decoration-color`, resolved (its initial `currentcolor` the
    /// element's `color`).
    pub color: Color,
    /// `text-decoration-thickness`.
    pub thickness: TextDecorationThickness,
}

/// One line decoration drawn on an element's text: the style and color
/// of the decorating box that asked for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AppliedLine {
    pub style: TextDecorationStyle,
    pub color: Color,
}

/// The decorations drawn on an element's text (CSS Text Decoration 4
/// §2.1): its own and those propagated from its ancestors — "when
/// specified on or propagated to an inline box, that box applies the
/// decoration to all the text in that box, and propagates it to any
/// in-flow children" — each "drawn with the decorating box's color and
/// style". Not propagated into an atomic inline's contents or to an
/// out-of-flow box. A terminal draws one line of each kind per cell, so
/// where several boxes ask for the same kind the innermost one's wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct AppliedDecorations {
    pub underline: Option<AppliedLine>,
    pub overline: Option<AppliedLine>,
    pub line_through: Option<AppliedLine>,
    pub blink: bool,
}

impl AppliedDecorations {
    /// No decoration.
    pub const NONE: Self = AppliedDecorations {
        underline: None,
        overline: None,
        line_through: None,
        blink: false,
    };

    /// These decorations with the lines a box decorated by `own` asks
    /// for drawn in its style and color.
    pub fn with(self, own: &TextDecorations) -> Self {
        let line = AppliedLine {
            style: own.style,
            color: own.color,
        };
        let pick = |on: bool, outer: Option<AppliedLine>| if on { Some(line) } else { outer };
        AppliedDecorations {
            underline: pick(own.line.underline, self.underline),
            overline: pick(own.line.overline, self.overline),
            line_through: pick(own.line.line_through, self.line_through),
            blink: self.blink || own.line.blink,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §2.1: an inner box's decorations add to the propagated ones, its
    /// own color and style on the lines it asks for.
    #[test]
    fn own_decorations_join_the_propagated_ones() {
        let red = Color::Rgb(255, 0, 0);
        let blue = Color::Rgb(0, 0, 255);
        let outer = AppliedDecorations::NONE.with(&TextDecorations {
            line: TextDecorationLine::UNDERLINE,
            style: TextDecorationStyle::Wavy,
            color: red,
            ..Default::default()
        });
        let inner = outer.with(&TextDecorations {
            line: TextDecorationLine::LINE_THROUGH,
            color: blue,
            ..Default::default()
        });
        assert_eq!(
            inner.underline,
            Some(AppliedLine {
                style: TextDecorationStyle::Wavy,
                color: red
            })
        );
        assert_eq!(inner.line_through.map(|l| l.color), Some(blue));
        assert_eq!(inner.overline, None);
        // The innermost box asking for a line draws it.
        let over = outer.with(&TextDecorations {
            line: TextDecorationLine::UNDERLINE,
            color: blue,
            ..Default::default()
        });
        assert_eq!(over.underline.map(|l| l.color), Some(blue));
        assert_eq!(TextDecorationLine::UNDERLINE.keywords(), "underline");
        assert_eq!(TextDecorationLine::NONE.keywords(), "none");
    }
}
