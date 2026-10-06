//! The values of the CSS Text properties (CSS Text 3 / 4) and their
//! computed group, [`TextStyle`]: white-space processing, wrapping and
//! line breaking. Every one of them inherits.

/// `white-space-collapse` (CSS Text 4 §4.1): whether and how white space
/// inside the element is collapsed. Inherited; initial `collapse`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhiteSpaceCollapse {
    /// Sequences of spaces, tabs and segment breaks collapse to one
    /// space (segment breaks transformed, §4.1.1).
    #[default]
    Collapse,
    /// White space is preserved; segment breaks are forced line breaks.
    Preserve,
    /// Spaces and tabs collapse; segment breaks are preserved as forced
    /// line breaks.
    PreserveBreaks,
    /// Spaces and tabs are preserved; segment breaks become spaces.
    PreserveSpaces,
    /// As `preserve`, but preserved spaces take up space at the end of a
    /// line instead of hanging, and every one is a soft wrap
    /// opportunity.
    BreakSpaces,
}

impl WhiteSpaceCollapse {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Collapse => "collapse",
            Self::Preserve => "preserve",
            Self::PreserveBreaks => "preserve-breaks",
            Self::PreserveSpaces => "preserve-spaces",
            Self::BreakSpaces => "break-spaces",
        }
    }

    /// Whether spaces and tabs are collapsible (§4.1.1).
    pub const fn collapses_spaces(self) -> bool {
        matches!(self, Self::Collapse | Self::PreserveBreaks)
    }

    /// Whether a segment break is a forced line break rather than
    /// white space (§4.1.1).
    pub const fn preserves_breaks(self) -> bool {
        matches!(
            self,
            Self::Preserve | Self::PreserveBreaks | Self::BreakSpaces
        )
    }
}

/// `text-wrap-mode` (CSS Text 4 §6.1): whether lines may wrap at soft
/// wrap opportunities. Inherited; initial `wrap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextWrapMode {
    /// Lines may break at soft wrap opportunities.
    #[default]
    Wrap,
    /// Lines do not break at soft wrap opportunities.
    Nowrap,
}

impl TextWrapMode {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Wrap => "wrap",
            Self::Nowrap => "nowrap",
        }
    }
}

/// `text-wrap-style` (CSS Text 4 "Selecting How to Wrap"): how a block
/// chooses among its soft wrap opportunities. Inherited; initial `auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextWrapStyle {
    /// Greedy line breaking: each line takes what fits.
    #[default]
    Auto,
    /// Lines of even length (groups of up to six lines in rdom).
    Balance,
    /// Breaks that do not look ahead, so editing leaves earlier lines.
    Stable,
    /// Better layout over speed: rdom avoids a one-word last line.
    Pretty,
    /// Avoid an excessively short last line (rdom: as `pretty`).
    AvoidShortLastLine,
}

impl TextWrapStyle {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Balance => "balance",
            Self::Stable => "stable",
            Self::Pretty => "pretty",
            Self::AvoidShortLastLine => "avoid-short-last-line",
        }
    }
}

/// The `white-space` shorthand's keywords (CSS Text 4 §3): each one a
/// pair of [`WhiteSpaceCollapse`] and [`TextWrapMode`] — the longhands
/// it sets ([`longhands`](Self::longhands)). The cascade computes the
/// longhands, not this; it is the builder's and the serializer's
/// vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhiteSpace {
    /// `collapse wrap`.
    #[default]
    Normal,
    /// `preserve nowrap`.
    Pre,
    /// `preserve wrap` (HTML `<textarea>`'s default).
    PreWrap,
    /// `preserve-breaks wrap`.
    PreLine,
    /// `collapse nowrap`.
    NoWrap,
    /// `break-spaces wrap`.
    BreakSpaces,
}

impl WhiteSpace {
    /// The longhand values the keyword sets (CSS Text 4 §3's table).
    pub const fn longhands(self) -> (WhiteSpaceCollapse, TextWrapMode) {
        use TextWrapMode::{Nowrap, Wrap};
        use WhiteSpaceCollapse as C;
        match self {
            Self::Normal => (C::Collapse, Wrap),
            Self::Pre => (C::Preserve, Nowrap),
            Self::PreWrap => (C::Preserve, Wrap),
            Self::PreLine => (C::PreserveBreaks, Wrap),
            Self::NoWrap => (C::Collapse, Nowrap),
            Self::BreakSpaces => (C::BreakSpaces, Wrap),
        }
    }

    /// The keyword a pair of longhand values spells, if one does.
    pub fn from_longhands(collapse: WhiteSpaceCollapse, mode: TextWrapMode) -> Option<Self> {
        [
            Self::Normal,
            Self::Pre,
            Self::PreWrap,
            Self::PreLine,
            Self::NoWrap,
            Self::BreakSpaces,
        ]
        .into_iter()
        .find(|w| w.longhands() == (collapse, mode))
    }

    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Pre => "pre",
            Self::PreWrap => "pre-wrap",
            Self::PreLine => "pre-line",
            Self::NoWrap => "nowrap",
            Self::BreakSpaces => "break-spaces",
        }
    }
}

/// `word-break` (CSS Text 3 §5.2): soft wrap opportunities between
/// letters. Inherited; initial `normal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WordBreak {
    /// Words break by their customary rules.
    #[default]
    Normal,
    /// Letters, numbers and alphabetic characters break as ideographs:
    /// a soft wrap opportunity between any two.
    BreakAll,
    /// No soft wrap opportunity between letters, numbers or ideographs:
    /// CJK text breaks only at spaces and punctuation.
    KeepAll,
    /// Legacy: `normal` with `overflow-wrap: anywhere`.
    BreakWord,
}

impl WordBreak {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::BreakAll => "break-all",
            Self::KeepAll => "keep-all",
            Self::BreakWord => "break-word",
        }
    }
}

/// `overflow-wrap` (legacy name `word-wrap`, CSS Text 3 §5.5): whether an
/// otherwise unbreakable string too long for its line may break at an
/// arbitrary point. Inherited; initial `normal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverflowWrap {
    /// Lines break only at soft wrap opportunities.
    #[default]
    Normal,
    /// Break anywhere to avoid overflow; the breaks do not count for the
    /// min-content size.
    BreakWord,
    /// Break anywhere to avoid overflow; the breaks count for the
    /// min-content size.
    Anywhere,
}

impl OverflowWrap {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::BreakWord => "break-word",
            Self::Anywhere => "anywhere",
        }
    }
}

/// `line-break` (CSS Text 3 §5.3): the strictness of the line-breaking
/// rules, for CJK punctuation and small kana. Inherited; initial `auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineBreak {
    /// The UA's choice: `normal` in rdom.
    #[default]
    Auto,
    /// The least restrictive rules.
    Loose,
    /// The common rules.
    Normal,
    /// The most stringent rules.
    Strict,
    /// A soft wrap opportunity around every typographic character unit.
    Anywhere,
}

impl LineBreak {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Loose => "loose",
            Self::Normal => "normal",
            Self::Strict => "strict",
            Self::Anywhere => "anywhere",
        }
    }
}

/// `hyphens` (CSS Text 3 §6.1): hyphenation. Inherited; initial
/// `manual`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Hyphens {
    /// Words are not hyphenated, not even at soft hyphens.
    None,
    /// Words break only at hyphenation opportunities in the text: soft
    /// hyphens (U+00AD).
    #[default]
    Manual,
    /// The UA may hyphenate by its own resources: rdom has no
    /// hyphenation dictionary, so `auto` is `manual` (DIVERGENCES).
    Auto,
}

impl Hyphens {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Manual => "manual",
            Self::Auto => "auto",
        }
    }
}

/// `tab-size` (CSS Text 3 §4.2): the distance between tab stops — a
/// number of spaces, or a length. Inherited; initial `8`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabSize {
    /// A multiple of the advance width of the space: in a terminal, cells.
    Number(f32),
    /// A length, in cells.
    Length(f32),
}

impl Default for TabSize {
    fn default() -> Self {
        TabSize::Number(8.0)
    }
}

impl TabSize {
    /// The tab size in whole cells: a space is one cell, so a number and
    /// a length alike round onto the grid (half to even, as every
    /// fractional length does — DIVERGENCES §1).
    pub fn cells(self) -> u16 {
        let (TabSize::Number(v) | TabSize::Length(v)) = self;
        crate::calc::to_cells(f64::from(v)).clamp(0, i32::from(u16::MAX)) as u16
    }
}

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

/// The computed CSS Text properties of an element
/// ([`ComputedStyle::text`](crate::ComputedStyle::text)). All of them
/// inherit, so the cascade copies the group from the parent whole.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextStyle {
    /// `white-space-collapse` (CSS Text 4 §4.1).
    pub white_space_collapse: WhiteSpaceCollapse,
    /// `text-wrap-mode` (CSS Text 4 §6.1).
    pub text_wrap_mode: TextWrapMode,
    /// `word-break` (CSS Text 3 §5.2).
    pub word_break: WordBreak,
    /// `overflow-wrap` / `word-wrap` (CSS Text 3 §5.5).
    pub overflow_wrap: OverflowWrap,
    /// `line-break` (CSS Text 3 §5.3).
    pub line_break: LineBreak,
    /// `hyphens` (CSS Text 3 §6.1).
    pub hyphens: Hyphens,
    /// `tab-size` (CSS Text 3 §4.2).
    pub tab_size: TabSize,
    /// `text-transform` (CSS Text 3 §2.1).
    pub text_transform: TextTransform,
    /// `text-indent` (CSS Text 3 §8.1).
    pub text_indent: TextIndent,
    /// `text-align-all` (CSS Text 3 §6.2), the `text-align` shorthand's
    /// first longhand.
    pub text_align_all: TextAlign,
    /// `text-align-last` (CSS Text 3 §6.3).
    pub text_align_last: TextAlignLast,
    /// `text-justify` (CSS Text 3 §6.4).
    pub text_justify: TextJustify,
    /// `text-wrap-style` (CSS Text 4), the `text-wrap` shorthand's second
    /// longhand.
    pub text_wrap_style: TextWrapStyle,
}

impl TextStyle {
    /// The `white-space` keyword the longhands spell, if one does.
    pub fn white_space(&self) -> Option<WhiteSpace> {
        WhiteSpace::from_longhands(self.white_space_collapse, self.text_wrap_mode)
    }

    /// Whether lines may break at soft wrap opportunities.
    pub fn wraps(&self) -> bool {
        self.text_wrap_mode == TextWrapMode::Wrap
    }
}
