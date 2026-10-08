//! [`UserActionState`] — the user-action pseudo-classes that follow a
//! pseudo-element in a selector (`::before:hover`, Selectors 4 §3.6.3).
//!
//! rdom-core's selector AST describes elements and rejects
//! pseudo-elements, which live beside it on the [`Rule`](super::Rule)
//! ([`PseudoElementTarget`](super::PseudoElementTarget)). A
//! pseudo-class after a pseudo-element states something about the
//! pseudo-element, not its originating element — whether the pointer is
//! over *it* — so it cannot join the element's compound either: it rides
//! the rule as this set, and a backend that knows which pseudo-element is
//! hovered or active matches it.

/// A set of user-action pseudo-classes (Selectors 4 §9): every one must
/// hold of the pseudo-element for the rule to apply. Empty for a rule
/// without them (every rule but `::before:hover`-like ones).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct UserActionState {
    bits: u8,
}

impl UserActionState {
    /// No pseudo-class.
    pub const EMPTY: Self = Self { bits: 0 };
    /// `:hover` (Selectors 4 §9.2).
    pub const HOVER: Self = Self { bits: 1 };
    /// `:active` (Selectors 4 §9.4).
    pub const ACTIVE: Self = Self { bits: 1 << 1 };
    /// `:focus` (Selectors 4 §9.5). A pseudo-element is never focused.
    pub const FOCUS: Self = Self { bits: 1 << 2 };
    /// `:focus-visible` (Selectors 4 §9.6). Never holds of a pseudo-element.
    pub const FOCUS_VISIBLE: Self = Self { bits: 1 << 3 };
    /// `:focus-within` (Selectors 4 §9.7). A pseudo-element holds no
    /// focusable element, so it never holds either.
    pub const FOCUS_WITHIN: Self = Self { bits: 1 << 4 };

    /// The focus pseudo-classes, none of which a pseudo-element can match.
    const FOCUS_ANY: u8 = Self::FOCUS.bits | Self::FOCUS_VISIBLE.bits | Self::FOCUS_WITHIN.bits;

    /// `self` and `other` together.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }

    /// Every pseudo-class of `other` is in `self`.
    pub const fn contains(self, other: Self) -> bool {
        self.bits & other.bits == other.bits
    }

    /// No pseudo-class.
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// How many pseudo-classes — what the set adds to a selector's
    /// specificity (Selectors 4 §17: each counts as a pseudo-class).
    pub const fn len(self) -> u16 {
        self.bits.count_ones() as u16
    }

    /// Holds of a pseudo-element that is `hovered` and `active` as the
    /// arguments say: the focus pseudo-classes never do.
    pub const fn matches(self, hovered: bool, active: bool) -> bool {
        if self.bits & Self::FOCUS_ANY != 0 {
            return false;
        }
        (!self.contains(Self::HOVER) || hovered) && (!self.contains(Self::ACTIVE) || active)
    }
}
