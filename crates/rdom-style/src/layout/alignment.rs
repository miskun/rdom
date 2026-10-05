//! The Box Alignment values (CSS Box Alignment 3 §4): one keyword
//! vocabulary, [`Align`], shared by `justify-content`, `align-content`,
//! `justify-items`, `align-items`, `justify-self` and `align-self` —
//! each property's grammar takes a subset of it (`parse::values`) — and
//! the value they compute to, [`Alignment`], which adds the overflow
//! position (`safe` / `unsafe`, §4.4) and `justify-items`' `legacy`.

/// A Box Alignment keyword (CSS Box Alignment 3 §4.1–§4.3, and `auto` /
/// `normal`). Which keywords a property takes is its grammar's business;
/// the layout reads each in the axis it aligns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// `normal`: the initial value of the content and items properties;
    /// its meaning depends on the layout mode (`stretch` for flex items
    /// and lines, `start` for blocks, §5.1 / §6.1).
    #[default]
    Normal,
    /// `auto` (the `*-self` properties): the parent's `*-items` value.
    Auto,
    /// `stretch`.
    Stretch,
    /// `start`: the alignment container's start edge in the axis, by its
    /// writing mode.
    Start,
    /// `end`.
    End,
    /// `center`.
    Center,
    /// `flex-start`: a flex container's main-start / cross-start edge
    /// (`start` outside flex layout).
    FlexStart,
    /// `flex-end`.
    FlexEnd,
    /// `self-start`: the edge of the item's own start side (the
    /// `*-self` / `*-items` properties).
    SelfStart,
    /// `self-end`.
    SelfEnd,
    /// `left`: the physical left edge (the inline axis only; `start`
    /// elsewhere).
    Left,
    /// `right`.
    Right,
    /// `space-between` (the content properties).
    SpaceBetween,
    /// `space-around`.
    SpaceAround,
    /// `space-evenly`.
    SpaceEvenly,
    /// `baseline` / `first baseline`.
    Baseline,
    /// `last baseline`.
    LastBaseline,
}

/// The overflow position (CSS Box Alignment 3 §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverflowAlign {
    /// Neither keyword: rdom aligns as `unsafe`, as browsers do.
    #[default]
    Default,
    /// `safe`: an aligned box that would overflow its alignment
    /// container is aligned as `start` instead.
    Safe,
    /// `unsafe`: the alignment is honored whatever the overflow.
    Unsafe,
}

/// A Box Alignment property's value: its keyword, its overflow position
/// and — `justify-items` only — whether it is `legacy` (`legacy` alone
/// is `Normal` with the flag; `legacy center` is `Center` with it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Alignment {
    pub keyword: Align,
    pub overflow: OverflowAlign,
    pub legacy: bool,
}

impl Alignment {
    /// `normal`, the initial value of the content and items properties.
    pub const NORMAL: Self = Self::new(Align::Normal);
    /// `auto`, the initial value of the `*-self` properties.
    pub const AUTO: Self = Self::new(Align::Auto);
    /// `legacy`, the initial value of `justify-items`.
    pub const LEGACY: Self = Self {
        keyword: Align::Normal,
        overflow: OverflowAlign::Default,
        legacy: true,
    };

    /// `keyword` with no overflow position.
    pub const fn new(keyword: Align) -> Self {
        Self {
            keyword,
            overflow: OverflowAlign::Default,
            legacy: false,
        }
    }

    /// `safe <keyword>`.
    pub const fn safe(keyword: Align) -> Self {
        Self {
            keyword,
            overflow: OverflowAlign::Safe,
            legacy: false,
        }
    }

    /// `unsafe <keyword>`.
    pub const fn unsafe_(keyword: Align) -> Self {
        Self {
            keyword,
            overflow: OverflowAlign::Unsafe,
            legacy: false,
        }
    }
}

impl From<Align> for Alignment {
    fn from(keyword: Align) -> Self {
        Self::new(keyword)
    }
}
