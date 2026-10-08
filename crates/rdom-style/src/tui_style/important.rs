//! `ImportantMask` — which of a `TuiStyle`'s fields were declared
//! `!important`.
//!
//! One bit per storage field the property table addresses
//! (`property_dispatch::table`'s `define_fields!`): a field's bit is
//! its row, and its named constant (`ImportantMask::FG`, …) is
//! declared on that row, so a new property gets its bit without a
//! hand-picked number and the set grows with the table. The words are
//! private; nothing outside depends on the width.

use crate::property_dispatch::{IMPORTANT_BITS, important_bit_name};

/// Number of `u64` words the bitset needs.
const WORDS: usize = IMPORTANT_BITS.div_ceil(64);

/// One bit per `TuiStyle` field — set when the author wrote
/// `!important` on the declaration that set it (CSS Cascade 4 §6.4).
/// Kept parallel to the fields rather than wrapping each in
/// `(Value<T>, bool)` to keep the hot property accessors cheap.
///
/// Build masks from the named constants (`ImportantMask::FG |
/// ImportantMask::BG`), `property_dispatch::property_mask(name)`, or
/// [`all`](Self::all); test them with [`contains`](Self::contains).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ImportantMask {
    words: [u64; WORDS],
}

impl ImportantMask {
    /// `font-weight`'s bit under its 0.5 name.
    pub const BOLD: Self = Self::FONT_WEIGHT;
    /// `font-style`'s bit under its 0.5 name.
    pub const ITALIC: Self = Self::FONT_STYLE;
    /// The `white-space` shorthand's bits under its 0.5 name: its two
    /// longhands' (CSS Text 4 §3).
    pub const WHITE_SPACE: Self = Self::WHITE_SPACE_COLLAPSE.union(Self::TEXT_WRAP_MODE);
    /// The font longhands' bits (the `font` shorthand sets
    /// `line-height` too).
    pub const FONT: Self = Self::FONT_WEIGHT
        .union(Self::FONT_STYLE)
        .union(Self::FONT_SIZE)
        .union(Self::FONT_FAMILY)
        .union(Self::FONT_STRETCH)
        .union(Self::FONT_VARIANT);

    /// The `text-decoration` shorthand's bits: its four longhands' (CSS
    /// Text Decoration 4 §2.6).
    pub const TEXT_DECORATION: Self = Self::TEXT_DECORATION_LINE
        .union(Self::TEXT_DECORATION_STYLE)
        .union(Self::TEXT_DECORATION_COLOR)
        .union(Self::TEXT_DECORATION_THICKNESS);

    /// No bit set.
    #[inline]
    pub const fn empty() -> Self {
        Self { words: [0; WORDS] }
    }

    /// The bit of the table row `index`. Crate-private: the rows'
    /// named constants are the public way in.
    pub(crate) const fn bit(index: usize) -> Self {
        assert!(index < IMPORTANT_BITS);
        let mut words = [0; WORDS];
        words[index / 64] = 1 << (index % 64);
        Self { words }
    }

    /// Every field's bit.
    pub const fn all() -> Self {
        let mut words = [u64::MAX; WORDS];
        let rem = IMPORTANT_BITS % 64;
        if rem != 0 {
            words[WORDS - 1] = (1 << rem) - 1;
        }
        Self { words }
    }

    /// `self | other`, usable in a `const`.
    pub const fn union(self, other: Self) -> Self {
        let mut words = self.words;
        let mut i = 0;
        while i < WORDS {
            words[i] |= other.words[i];
            i += 1;
        }
        Self { words }
    }

    /// Clear the bits of `other` from `self`.
    pub const fn without(self, other: Self) -> Self {
        let mut words = self.words;
        let mut i = 0;
        while i < WORDS {
            words[i] &= !other.words[i];
            i += 1;
        }
        Self { words }
    }

    /// Every bit of `other` is set in `self`.
    pub const fn contains(self, other: Self) -> bool {
        let mut i = 0;
        while i < WORDS {
            if self.words[i] & other.words[i] != other.words[i] {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Some bit of `other` is set in `self`.
    pub const fn intersects(self, other: Self) -> bool {
        let mut i = 0;
        while i < WORDS {
            if self.words[i] & other.words[i] != 0 {
                return true;
            }
            i += 1;
        }
        false
    }

    /// No bit set.
    pub const fn is_empty(self) -> bool {
        let mut i = 0;
        while i < WORDS {
            if self.words[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }

    /// How many fields are marked.
    pub fn count(self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// The set rows, ascending.
    fn indices(self) -> impl Iterator<Item = usize> {
        (0..IMPORTANT_BITS).filter(move |&i| self.words[i / 64] & (1 << (i % 64)) != 0)
    }
}

impl std::fmt::Debug for ImportantMask {
    /// `ImportantMask(FG | BORDER_TOP_COLOR)` — the constants' names.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ImportantMask(")?;
        for (n, i) in self.indices().enumerate() {
            if n > 0 {
                f.write_str(" | ")?;
            }
            f.write_str(important_bit_name(i))?;
        }
        f.write_str(")")
    }
}

impl std::ops::BitOr for ImportantMask {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

impl std::ops::BitOrAssign for ImportantMask {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
    }
}

impl std::ops::BitAnd for ImportantMask {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: Self) -> Self {
        let mut words = self.words;
        for (w, r) in words.iter_mut().zip(rhs.words) {
            *w &= r;
        }
        Self { words }
    }
}

impl std::ops::BitAndAssign for ImportantMask {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl ImportantMask {
    /// The five `transition-*` longhands' bits — what the `transition`
    /// shorthand (and `TuiStyle::transitions_important`) marks.
    pub const TRANSITIONS: Self = Self::TRANSITION_PROPERTY
        .union(Self::TRANSITION_DURATION)
        .union(Self::TRANSITION_TIMING_FUNCTION)
        .union(Self::TRANSITION_DELAY)
        .union(Self::TRANSITION_BEHAVIOR);
    /// The ten `animation-*` longhands' bits — what the `animation`
    /// shorthand (and `TuiStyle::animations_important`) marks.
    pub const ANIMATIONS: Self = Self::ANIMATION_NAME
        .union(Self::ANIMATION_DURATION)
        .union(Self::ANIMATION_TIMING_FUNCTION)
        .union(Self::ANIMATION_DELAY)
        .union(Self::ANIMATION_ITERATION_COUNT)
        .union(Self::ANIMATION_DIRECTION)
        .union(Self::ANIMATION_FILL_MODE)
        .union(Self::ANIMATION_PLAY_STATE)
        .union(Self::ANIMATION_COMPOSITION)
        .union(Self::ANIMATION_TIMELINE);
}

#[cfg(test)]
mod tests {
    use super::ImportantMask as M;

    #[test]
    fn operations_span_every_word() {
        let last = M::bit(super::IMPORTANT_BITS - 1);
        let m = M::FG | last;
        assert!(m.contains(M::FG) && m.contains(last));
        assert_eq!(m.count(), 2);
        assert_eq!(m.without(M::FG), last);
        assert_eq!(m & last, last);
        assert!(M::all().contains(m));
        assert_eq!(M::all().count(), super::IMPORTANT_BITS);
        assert!(M::empty().is_empty() && !m.is_empty());
        assert_eq!(format!("{:?}", M::FG | M::BG), "ImportantMask(FG | BG)");
    }

    /// C5G-API-EDGES: `intersects` — any bit in common; and the
    /// `flex-direction` bit is named for its property, beside
    /// `TEXT_DIRECTION` (CSS `direction`).
    #[test]
    fn intersects_and_the_flex_direction_bit() {
        let m = M::FG | M::BG;
        assert!(m.intersects(M::BG | M::WIDTH));
        assert!(!m.intersects(M::WIDTH));
        assert!(!M::empty().intersects(M::all()));
        assert_eq!(
            crate::property_dispatch::property_mask("flex-direction"),
            Some(M::FLEX_DIRECTION | M::FLEX_REVERSE)
        );
        assert_eq!(
            crate::property_dispatch::property_mask("direction"),
            Some(M::TEXT_DIRECTION)
        );
        assert_eq!(
            format!("{:?}", M::FLEX_DIRECTION),
            "ImportantMask(FLEX_DIRECTION)"
        );
    }
}
