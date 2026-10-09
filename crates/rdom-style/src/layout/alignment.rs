//! The Box Alignment values (CSS Box Alignment 3 §4): one keyword
//! vocabulary, [`Align`], shared by `justify-content`, `align-content`,
//! `justify-items`, `align-items`, `justify-self` and `align-self` —
//! each property's grammar takes a subset of it (`parse::values`) — and
//! the value they compute to, [`Alignment`], which adds the overflow
//! position (`safe` / `unsafe`, §4.4) and `justify-items`' `legacy`.

/// A Box Alignment keyword (CSS Box Alignment 3 §4.1–§4.3, and `auto` /
/// `normal`). Which keywords a property takes is its grammar's business
/// ([`Alignment::is_valid_for`]); the layout reads each in the axis it
/// aligns. `#[non_exhaustive]`: CSS Anchor Positioning's `anchor-center`
/// joins it (C15-ANCHOR), so a match outside this crate needs a `_` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
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
    /// `anchor-center` (CSS Anchor Positioning 1 §3.4, a `<self-position>`
    /// of the `*-self` and `*-items` properties): an absolutely positioned
    /// box centred on its default anchor; `center` for any other box.
    AnchorCenter,
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

/// The six Box Alignment properties (CSS Box Alignment 3 §5–§6), each
/// taking its own subset of the [`Align`] keywords.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlignProperty {
    /// `justify-content` (§5.2).
    JustifyContent,
    /// `align-content` (§5.1).
    AlignContent,
    /// `justify-items` (§6.2).
    JustifyItems,
    /// `align-items` (§6.3).
    AlignItems,
    /// `justify-self` (§6.1).
    JustifySelf,
    /// `align-self` (§6.1).
    AlignSelf,
}

impl AlignProperty {
    /// The property's CSS name.
    pub const fn name(self) -> &'static str {
        match self {
            AlignProperty::JustifyContent => "justify-content",
            AlignProperty::AlignContent => "align-content",
            AlignProperty::JustifyItems => "justify-items",
            AlignProperty::AlignItems => "align-items",
            AlignProperty::JustifySelf => "justify-self",
            AlignProperty::AlignSelf => "align-self",
        }
    }

    /// Which keywords the property's grammar takes — the one table the
    /// parser (`parse::values::align`) and [`Alignment::is_valid_for`]
    /// read.
    pub(crate) const fn grammar(self) -> Grammar {
        match self {
            // `normal | <content-distribution> | <overflow-position>?
            // [ <content-position> | left | right ]` (§5.2).
            AlignProperty::JustifyContent => Grammar {
                auto: false,
                baseline: false,
                distribution: true,
                self_positions: false,
                left_right: true,
                legacy: false,
            },
            // `normal | <baseline-position> | <content-distribution> |
            // <overflow-position>? <content-position>` (§5.1).
            AlignProperty::AlignContent => Grammar {
                auto: false,
                baseline: true,
                distribution: true,
                self_positions: false,
                left_right: false,
                legacy: false,
            },
            // `normal | stretch | <baseline-position> |
            // <overflow-position>? [ <self-position> | left | right ] |
            // legacy | legacy && [ left | right | center ]` (§6.2).
            AlignProperty::JustifyItems => Grammar {
                auto: false,
                baseline: true,
                distribution: false,
                self_positions: true,
                left_right: true,
                legacy: true,
            },
            // `normal | stretch | <baseline-position> |
            // <overflow-position>? <self-position>` (§6.3).
            AlignProperty::AlignItems => Grammar {
                auto: false,
                baseline: true,
                distribution: false,
                self_positions: true,
                left_right: false,
                legacy: false,
            },
            // `auto | normal | stretch | <baseline-position> |
            // <overflow-position>? [ <self-position> | left | right ]`
            // (§6.1).
            AlignProperty::JustifySelf => Grammar {
                auto: true,
                baseline: true,
                distribution: false,
                self_positions: true,
                left_right: true,
                legacy: false,
            },
            // `auto | <'align-items'>` (§6.1).
            AlignProperty::AlignSelf => Grammar {
                auto: true,
                baseline: true,
                distribution: false,
                self_positions: true,
                left_right: false,
                legacy: false,
            },
        }
    }
}

/// Which keywords a Box Alignment property takes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Grammar {
    /// `auto` (the `*-self` properties).
    pub(crate) auto: bool,
    /// `<baseline-position>`: `baseline`, `first baseline`, `last baseline`.
    pub(crate) baseline: bool,
    /// `<content-distribution>`: `space-between | space-around |
    /// space-evenly | stretch`.
    pub(crate) distribution: bool,
    /// `<self-position>`'s `self-start | self-end` beside the
    /// `<content-position>`s `center | start | end | flex-start | flex-end`.
    pub(crate) self_positions: bool,
    /// `left | right` (the inline-axis properties).
    pub(crate) left_right: bool,
    /// `legacy | legacy && [ left | right | center ]` (`justify-items`).
    pub(crate) legacy: bool,
}

impl Alignment {
    /// Whether `property`'s grammar takes this value — what its CSS text
    /// would parse to. The typed setters (`TuiStyle::align_self`, …)
    /// refuse a value it does not take, as a CSS parser drops the
    /// declaration.
    pub fn is_valid_for(self, property: AlignProperty) -> bool {
        let g = property.grammar();
        if self.legacy {
            return g.legacy
                && self.overflow == OverflowAlign::Default
                && matches!(
                    self.keyword,
                    Align::Normal | Align::Left | Align::Right | Align::Center
                );
        }
        let positional = match self.keyword {
            Align::Center | Align::Start | Align::End | Align::FlexStart | Align::FlexEnd => true,
            Align::SelfStart | Align::SelfEnd | Align::AnchorCenter => g.self_positions,
            Align::Left | Align::Right => g.left_right,
            _ => false,
        };
        if self.overflow != OverflowAlign::Default {
            return positional;
        }
        positional
            || match self.keyword {
                Align::Normal | Align::Stretch => true,
                Align::Auto => g.auto,
                Align::Baseline | Align::LastBaseline => g.baseline,
                Align::SpaceBetween | Align::SpaceAround | Align::SpaceEvenly => g.distribution,
                _ => false,
            }
    }
}

impl From<Align> for Alignment {
    fn from(keyword: Align) -> Self {
        Self::new(keyword)
    }
}

#[cfg(test)]
#[path = "alignment_tests.rs"]
mod tests;
