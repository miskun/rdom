//! The CSS Multi-column Layout 1 values (`column-count`, `column-width`,
//! `column-span`, `column-fill`; the rule's style, width and color are the
//! border's types) and the CSS Fragmentation 3 values (`break-before` /
//! `-after` / `-inside`, `orphans`, `widows`, `box-decoration-break`) —
//! with [`MulticolStyle`] and [`FragmentationStyle`], their computed
//! groups.

use super::{BorderStyle, BorderWidth};
use crate::TuiColor;

/// `column-count` (CSS Multi-column 1 §3.2): `auto | <integer [1,∞]>`.
///
/// Closed (DESIGN): the column-box pseudo-algorithm (§3.4) reads both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum ColumnCount {
    /// The width decides the count (the initial value).
    #[default]
    Auto,
    /// At most this many columns, never 0.
    Count(u32),
}

/// `column-width` (CSS Multi-column 1 §3.1): `auto | <length [0,∞]>`, in
/// whole cells — a pixel or font-relative length is geometry and invalid
/// (DESIGN "Pixel lengths select, cells measure"); a percentage is not in
/// the grammar.
///
/// Closed (DESIGN): the column-box pseudo-algorithm (§3.4) reads each.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ColumnWidth {
    /// The count decides the width (the initial value).
    #[default]
    Auto,
    /// The optimal column width, in cells.
    Cells(u16),
    /// A math function with a viewport or line-height unit, made cells by
    /// the cascade (`ComputedStyle::resolve_context_units`); layout never
    /// meets one.
    Calc(std::sync::Arc<crate::calc::CalcExpr>),
}

impl ColumnWidth {
    /// The width in cells; `None` for `auto`. A math function resolves
    /// with no percentage basis (it has no percentage).
    pub fn cells(&self) -> Option<u16> {
        match self {
            ColumnWidth::Auto => None,
            ColumnWidth::Cells(n) => Some(*n),
            ColumnWidth::Calc(e) => Some(super::sizing::resolve_u16(e, 0)),
        }
    }
}

/// `column-span` (CSS Multi-column 1 §6.1): `none | all`.
///
/// Open (`#[non_exhaustive]`): CSS Multi-column 2 adds `<integer>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[non_exhaustive]
pub enum ColumnSpan {
    /// In its column (the initial value).
    #[default]
    None,
    /// Across every column of its multi-column container, which it splits
    /// into the column sets before and after it.
    All,
}

impl ColumnSpan {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, ColumnSpan)] =
        &[("none", ColumnSpan::None), ("all", ColumnSpan::All)];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        keyword_of(Self::KEYWORDS, self)
    }
}

/// `column-fill` (CSS Multi-column 1 §7.1): `auto | balance |
/// balance-all`.
///
/// Closed (DESIGN): the fill the column layout makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum ColumnFill {
    /// Fill each column in turn, where the height is constrained.
    Auto,
    /// Balance the content across the columns (the initial value).
    #[default]
    Balance,
    /// Balance in every fragment (in continuous media, as `balance`).
    BalanceAll,
}

impl ColumnFill {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, ColumnFill)] = &[
        ("auto", ColumnFill::Auto),
        ("balance", ColumnFill::Balance),
        ("balance-all", ColumnFill::BalanceAll),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        keyword_of(Self::KEYWORDS, self)
    }
}

/// `break-before` / `break-after` (CSS Fragmentation 3 §3.1, Fragmentation
/// 4 §3.1): `auto | avoid | always | all | avoid-page | page | left |
/// right | recto | verso | avoid-column | column | avoid-region | region`.
///
/// Closed (DESIGN): the break rules (§4.4) decide each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum BreakBetween {
    /// Neither forces nor forbids a break (the initial value).
    #[default]
    Auto,
    Avoid,
    Always,
    All,
    AvoidPage,
    Page,
    Left,
    Right,
    Recto,
    Verso,
    AvoidColumn,
    Column,
    AvoidRegion,
    Region,
}

impl BreakBetween {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, BreakBetween)] = &[
        ("auto", BreakBetween::Auto),
        ("avoid", BreakBetween::Avoid),
        ("always", BreakBetween::Always),
        ("all", BreakBetween::All),
        ("avoid-page", BreakBetween::AvoidPage),
        ("page", BreakBetween::Page),
        ("left", BreakBetween::Left),
        ("right", BreakBetween::Right),
        ("recto", BreakBetween::Recto),
        ("verso", BreakBetween::Verso),
        ("avoid-column", BreakBetween::AvoidColumn),
        ("column", BreakBetween::Column),
        ("avoid-region", BreakBetween::AvoidRegion),
        ("region", BreakBetween::Region),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        keyword_of(Self::KEYWORDS, self)
    }

    /// Whether the value forces a break between columns: `column`, and
    /// `always` / `all`, whose break is the innermost fragmentation
    /// context's (§3.1, Fragmentation 4 §3.1). A page or region break
    /// forces none in a column — continuous media has no pages, and rdom
    /// no regions.
    pub fn forces_column(self) -> bool {
        matches!(
            self,
            BreakBetween::Column | BreakBetween::Always | BreakBetween::All
        )
    }

    /// Whether the value avoids a break between columns: `avoid` and
    /// `avoid-column`.
    pub fn avoids_column(self) -> bool {
        matches!(self, BreakBetween::Avoid | BreakBetween::AvoidColumn)
    }
}

/// `break-inside` (CSS Fragmentation 3 §3.2): `auto | avoid | avoid-page |
/// avoid-column | avoid-region`.
///
/// Closed (DESIGN): the break rules (§4.4) decide each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum BreakInside {
    /// The initial value.
    #[default]
    Auto,
    Avoid,
    AvoidPage,
    AvoidColumn,
    AvoidRegion,
}

impl BreakInside {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, BreakInside)] = &[
        ("auto", BreakInside::Auto),
        ("avoid", BreakInside::Avoid),
        ("avoid-page", BreakInside::AvoidPage),
        ("avoid-column", BreakInside::AvoidColumn),
        ("avoid-region", BreakInside::AvoidRegion),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        keyword_of(Self::KEYWORDS, self)
    }

    /// Whether the value avoids a column break inside the box.
    pub fn avoids_column(self) -> bool {
        matches!(self, BreakInside::Avoid | BreakInside::AvoidColumn)
    }
}

/// `box-decoration-break` (CSS Fragmentation 3 §5.4): `slice | clone`.
///
/// Closed (DESIGN): the two ways a fragment draws its box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum BoxDecorationBreak {
    /// The box is sliced at a break (the initial value).
    #[default]
    Slice,
    /// Each fragment draws the whole box (rdom draws it as `slice`,
    /// DIVERGENCES).
    Clone,
}

impl BoxDecorationBreak {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, BoxDecorationBreak)] = &[
        ("slice", BoxDecorationBreak::Slice),
        ("clone", BoxDecorationBreak::Clone),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        keyword_of(Self::KEYWORDS, self)
    }
}

fn keyword_of<T: Copy + PartialEq>(keywords: &[(&'static str, T)], v: T) -> &'static str {
    keywords
        .iter()
        .find(|(_, k)| *k == v)
        .map_or("auto", |(name, _)| name)
}

/// The computed CSS Multi-column 1 properties of an element
/// ([`ComputedStyle::multicol`](crate::ComputedStyle::multicol)); none
/// inherit. `column-gap` is the top-level
/// [`column_gap`](crate::ComputedStyle::column_gap), shared with flex and
/// grid.
///
/// Closed (DESIGN), as the other style groups: a new field fails a
/// destructuring pattern. `Default` is the initial values.
#[derive(Debug, Clone, PartialEq)]
pub struct MulticolStyle {
    /// `column-count` (§3.2).
    pub column_count: ColumnCount,
    /// `column-width` (§3.1), viewport units resolved.
    pub column_width: ColumnWidth,
    /// `column-rule-style` (§4.2).
    pub column_rule_style: BorderStyle,
    /// `column-rule-width` (§4.3): a `<line-width>` that selects the
    /// rule's glyph weight, as a border width does.
    pub column_rule_width: BorderWidth,
    /// `column-rule-color` (§4.1), resolved at paint against the element
    /// (`currentcolor` its `color`), as `outline-color` is.
    pub column_rule_color: TuiColor,
    /// `column-span` (§6.1).
    pub column_span: ColumnSpan,
    /// `column-fill` (§7.1).
    pub column_fill: ColumnFill,
}

impl Default for MulticolStyle {
    fn default() -> Self {
        MulticolStyle {
            column_count: ColumnCount::Auto,
            column_width: ColumnWidth::Auto,
            column_rule_style: BorderStyle::None,
            column_rule_width: BorderWidth::Medium,
            column_rule_color: TuiColor::CurrentColor,
            column_span: ColumnSpan::None,
            column_fill: ColumnFill::Balance,
        }
    }
}

impl MulticolStyle {
    /// Whether a block container with these values is a multi-column
    /// container (§2): `column-count` or `column-width` is not `auto`.
    pub fn is_multicol(&self) -> bool {
        self.column_count != ColumnCount::Auto || self.column_width != ColumnWidth::Auto
    }
}

/// The computed CSS Fragmentation 3 properties of an element
/// ([`ComputedStyle::fragmentation`](crate::ComputedStyle::fragmentation)):
/// the breaks, which do not inherit, and `orphans` / `widows`, which do.
///
/// Closed (DESIGN), as the other style groups. `Default` is the initial
/// values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentationStyle {
    /// `break-before` (§3.1).
    pub break_before: BreakBetween,
    /// `break-after` (§3.1).
    pub break_after: BreakBetween,
    /// `break-inside` (§3.2).
    pub break_inside: BreakInside,
    /// `orphans` (§3.3): the fewest lines of a block container left at
    /// the end of a fragment. Inherited; initial 2.
    pub orphans: u32,
    /// `widows` (§3.3): the fewest lines carried to the start of the
    /// next fragment. Inherited; initial 2.
    pub widows: u32,
    /// `box-decoration-break` (§5.4).
    pub box_decoration_break: BoxDecorationBreak,
}

impl Default for FragmentationStyle {
    fn default() -> Self {
        FragmentationStyle {
            break_before: BreakBetween::Auto,
            break_after: BreakBetween::Auto,
            break_inside: BreakInside::Auto,
            orphans: 2,
            widows: 2,
            box_decoration_break: BoxDecorationBreak::Slice,
        }
    }
}
