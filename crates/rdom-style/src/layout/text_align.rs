//! The values of the CSS Text properties of what a line's text renders as
//! and where it sits (CSS Text 3 / 4): `text-transform`, `text-indent`,
//! `text-align` and its longhands, `text-justify`. Every one of them
//! inherits.

/// The case component of `text-transform` (CSS Text 3 §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextCase {
    /// No case mapping.
    #[default]
    None,
    /// The first typographic letter unit of each word in titlecase.
    Capitalize,
    /// All letters in uppercase (full Unicode mapping).
    Uppercase,
    /// All letters in lowercase (full Unicode mapping).
    Lowercase,
}

/// `text-transform` (CSS Text 3 §2.1, Text 4 §2.1): `none | [capitalize
/// | uppercase | lowercase] || full-width || full-size-kana | math-auto`.
/// Inherited; initial `none`. A rendering transform: copy and editing
/// work in the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextTransform {
    /// The case mapping.
    pub case: TextCase,
    /// Typographic character units in their full-width forms.
    pub full_width: bool,
    /// Small kana as full-size kana.
    pub full_size_kana: bool,
    /// A text node of one character as mathematical italic (MathML Core
    /// §4.2); the grammar takes it alone.
    pub math_auto: bool,
}

impl TextTransform {
    /// `none`.
    pub const NONE: TextTransform = TextTransform {
        case: TextCase::None,
        full_width: false,
        full_size_kana: false,
        math_auto: false,
    };

    /// Whether the transform changes nothing.
    pub fn is_none(self) -> bool {
        self == Self::NONE
    }
}

/// `text-indent` (CSS Text 3 §8.1): `<length-percentage> && hanging? &&
/// each-line?`. Inherited; initial `0`. The length is a margin at the
/// start edge of the affected line boxes — a percentage of the block
/// container's own inline size — and either sign.
#[derive(Debug, Clone, PartialEq)]
pub struct TextIndent {
    /// The indent: cells, or a `calc()` / percentage resolved against
    /// the block's content width. `Length::Auto` is not in the grammar
    /// and indents nothing.
    pub length: crate::layout::Length,
    /// Every line but the ones `text-indent` would otherwise affect.
    pub hanging: bool,
    /// The lines after a forced line break too.
    pub each_line: bool,
}

impl Default for TextIndent {
    fn default() -> Self {
        TextIndent::cells(0)
    }
}

impl TextIndent {
    /// An indent of `n` cells, no keyword.
    pub fn cells(n: i32) -> Self {
        TextIndent {
            length: crate::layout::Length::Cells(n),
            hanging: false,
            each_line: false,
        }
    }

    /// The indent in cells, a percentage of `width` (the block's content
    /// width).
    pub fn resolve(&self, width: u16) -> i32 {
        self.length.cells(i32::from(width)).unwrap_or(0)
    }
}

/// `text-align-all` (CSS Text 3 §6.2), and the keywords `text-align`
/// (§6.1) sets it to: the inline alignment of a block container's lines.
/// Inherited; initial `start`. `MatchParent` computes to the parent's
/// value with `start` / `end` resolved against the parent's `direction`
/// (`left` or `right`); at the root, to `start`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    /// The line box's start edge.
    #[default]
    Start,
    /// The line box's end edge.
    End,
    /// The line-left edge.
    Left,
    /// The line-right edge.
    Right,
    /// Centered in the line box.
    Center,
    /// Justified per `text-justify` to fill the line box.
    Justify,
    /// The parent's alignment, `start` / `end` made physical.
    MatchParent,
}

impl TextAlign {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
            Self::Left => "left",
            Self::Right => "right",
            Self::Center => "center",
            Self::Justify => "justify",
            Self::MatchParent => "match-parent",
        }
    }

    /// `start` / `end` as the physical side they are under `direction:
    /// rtl` when `rtl` (CSS Writing Modes 4 §2.1); others as they are.
    pub const fn physical(self, rtl: bool) -> Self {
        match (self, rtl) {
            (Self::Start, false) | (Self::End, true) => Self::Left,
            (Self::Start, true) | (Self::End, false) => Self::Right,
            (other, _) => other,
        }
    }
}

/// `text-align-last` (CSS Text 3 §6.3): the alignment of a block's last
/// line and of each line before a forced break. Inherited; initial
/// `auto` — `text-align-all`'s, `start` when that is `justify`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlignLast {
    /// `text-align-all`'s alignment, `start` for `justify`.
    #[default]
    Auto,
    /// As `text-align`'s `start`.
    Start,
    /// As `text-align`'s `end`.
    End,
    /// As `text-align`'s `left`.
    Left,
    /// As `text-align`'s `right`.
    Right,
    /// As `text-align`'s `center`.
    Center,
    /// As `text-align`'s `justify`.
    Justify,
    /// As `text-align`'s `match-parent`.
    MatchParent,
}

impl TextAlignLast {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self.align() {
            None => "auto",
            Some(a) => a.keyword(),
        }
    }

    /// The alignment it names, `None` for `auto`.
    pub const fn align(self) -> Option<TextAlign> {
        Some(match self {
            Self::Auto => return None,
            Self::Start => TextAlign::Start,
            Self::End => TextAlign::End,
            Self::Left => TextAlign::Left,
            Self::Right => TextAlign::Right,
            Self::Center => TextAlign::Center,
            Self::Justify => TextAlign::Justify,
            Self::MatchParent => TextAlign::MatchParent,
        })
    }

    /// The `text-align-last` value naming `align`.
    pub const fn of(align: TextAlign) -> Self {
        match align {
            TextAlign::Start => Self::Start,
            TextAlign::End => Self::End,
            TextAlign::Left => Self::Left,
            TextAlign::Right => Self::Right,
            TextAlign::Center => Self::Center,
            TextAlign::Justify => Self::Justify,
            TextAlign::MatchParent => Self::MatchParent,
        }
    }
}

/// `text-justify` (CSS Text 3 §6.4): the justification method.
/// Inherited; initial `auto`. `distribute` parses as `inter-character`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextJustify {
    /// The UA's method: rdom expands word separators and the gaps beside
    /// CJK characters.
    #[default]
    Auto,
    /// No justification opportunities.
    None,
    /// Word separators only.
    InterWord,
    /// Between every pair of adjacent typographic character units.
    InterCharacter,
}

impl TextJustify {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::None => "none",
            Self::InterWord => "inter-word",
            Self::InterCharacter => "inter-character",
        }
    }
}
