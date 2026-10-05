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
