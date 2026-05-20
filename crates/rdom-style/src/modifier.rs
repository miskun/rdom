//! `Modifier` — text-decoration bitflags.
//!
//! Eight SGR-backed effects. Each is an independent bit — a cell can
//! be bold + underlined + italic at the same time. Composition via
//! `|` / `|=`, intersection via `&`, negation via `remove()`.
//!
//! Serialization to SGR codes (`\x1b[1m`, `\x1b[4m`, ...) lives in
//! the backend layer.

use rdom_core::bitflags_like;

bitflags_like! {
    /// Text-decoration bitflags. Compose multiple effects with `|`.
    ///
    /// Terminal support varies:
    /// - Bold, Underlined, Reversed, Hidden, Crossed — widely supported.
    /// - Italic — most modern terminals; iTerm2, WezTerm, Alacritty yes; some old emulators no.
    /// - Slow/RapidBlink — rare, distracting; users often disable.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Modifier(u16) {
        BOLD         = 1 << 0;
        // Bits 1 and 6 are unused. Bit 1 was DIM (SGR-2),
        // dropped pre-publish; bit 6 was REVERSED (SGR-7), dropped
        // when the caret switched to explicit fg/bg pairs instead
        // of relying on the terminal's reverse-video toggle. Both
        // gaps stay so the remaining bit values don't shift.
        ITALIC       = 1 << 2;
        UNDERLINED   = 1 << 3;
        SLOW_BLINK   = 1 << 4;
        RAPID_BLINK  = 1 << 5;
        HIDDEN       = 1 << 7;
        CROSSED_OUT  = 1 << 8;
    }
}

impl Modifier {
    /// Clear `other`'s bits from `self` in place. Matches the
    /// `bitflags` crate convention.
    pub fn remove(&mut self, other: Modifier) {
        *self = Modifier(self.bits() & !other.bits());
    }

    /// Set `other`'s bits in `self` in place. Equivalent to `*self |= other`
    /// but reads better at call sites.
    pub fn insert(&mut self, other: Modifier) {
        *self |= other;
    }

    /// Overwrite the bits in `mask` with the bits of `other` (masked).
    /// `value` must be a subset of `mask`.
    pub fn set(&mut self, mask: Modifier, value: bool) {
        if value {
            self.insert(mask);
        } else {
            self.remove(mask);
        }
    }

    /// Difference as a value: bits set in `self` but not in `other`.
    pub fn difference(self, other: Modifier) -> Modifier {
        Modifier(self.bits() & !other.bits())
    }

    /// Symmetric difference as a value: bits set in either but not both.
    pub fn symmetric_difference(self, other: Modifier) -> Modifier {
        Modifier(self.bits() ^ other.bits())
    }

    /// True when `self` and `other` share at least one bit. Complement
    /// of `contains`: `contains` requires ALL of `other`'s bits;
    /// `intersects` requires ANY.
    pub fn intersects(self, other: Modifier) -> bool {
        (self.bits() & other.bits()) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        assert_eq!(Modifier::default(), Modifier::empty());
        assert!(Modifier::default().is_empty());
    }

    #[test]
    fn single_flag_contains_itself() {
        assert!(Modifier::BOLD.contains(Modifier::BOLD));
        assert!(!Modifier::BOLD.contains(Modifier::ITALIC));
    }

    #[test]
    fn bitor_combines() {
        let m = Modifier::BOLD | Modifier::ITALIC;
        assert!(m.contains(Modifier::BOLD));
        assert!(m.contains(Modifier::ITALIC));
        assert!(!m.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn bitor_assign_combines() {
        let mut m = Modifier::BOLD;
        m |= Modifier::ITALIC;
        assert!(m.contains(Modifier::BOLD));
        assert!(m.contains(Modifier::ITALIC));
    }

    #[test]
    fn bitand_intersects() {
        let m = (Modifier::BOLD | Modifier::ITALIC) & Modifier::ITALIC;
        assert_eq!(m, Modifier::ITALIC);
    }

    #[test]
    fn remove_strips_bits_in_place() {
        let mut m = Modifier::BOLD | Modifier::ITALIC;
        m.remove(Modifier::ITALIC);
        assert_eq!(m, Modifier::BOLD);
    }

    #[test]
    fn remove_non_present_is_noop() {
        let mut m = Modifier::BOLD;
        m.remove(Modifier::ITALIC);
        assert_eq!(m, Modifier::BOLD);
    }

    #[test]
    fn insert_sets_bits() {
        let mut m = Modifier::BOLD;
        m.insert(Modifier::ITALIC);
        assert!(m.contains(Modifier::BOLD));
        assert!(m.contains(Modifier::ITALIC));
    }

    #[test]
    fn set_on_inserts_off_removes() {
        let mut m = Modifier::BOLD;
        m.set(Modifier::ITALIC, true);
        assert!(m.contains(Modifier::ITALIC));
        m.set(Modifier::BOLD, false);
        assert!(!m.contains(Modifier::BOLD));
    }

    #[test]
    fn difference_returns_new() {
        let a = Modifier::BOLD | Modifier::ITALIC;
        let d = a.difference(Modifier::ITALIC);
        // `a` unchanged (returned a new value).
        assert!(a.contains(Modifier::ITALIC));
        assert_eq!(d, Modifier::BOLD);
    }

    #[test]
    fn symmetric_difference() {
        let a = Modifier::BOLD | Modifier::ITALIC;
        let b = Modifier::ITALIC | Modifier::UNDERLINED;
        let sd = a.symmetric_difference(b);
        // Bits: BOLD (in a, not b), UNDERLINED (in b, not a). ITALIC is in both.
        assert!(sd.contains(Modifier::BOLD));
        assert!(sd.contains(Modifier::UNDERLINED));
        assert!(!sd.contains(Modifier::ITALIC));
    }

    #[test]
    fn all_seven_flags_distinct() {
        let flags = [
            Modifier::BOLD,
            Modifier::ITALIC,
            Modifier::UNDERLINED,
            Modifier::SLOW_BLINK,
            Modifier::RAPID_BLINK,
            Modifier::HIDDEN,
            Modifier::CROSSED_OUT,
        ];
        let combined: Modifier = flags.iter().copied().fold(Modifier::empty(), |a, b| a | b);
        // Every flag individually is set in the combined mask.
        for f in &flags {
            assert!(combined.contains(*f));
        }
        // And each flag is a distinct bit.
        for (i, a) in flags.iter().enumerate() {
            for b in flags.iter().skip(i + 1) {
                assert!((*a & *b).is_empty(), "modifier bits overlap: {a:?} & {b:?}");
            }
        }
    }

    #[test]
    fn all_contains_every_flag() {
        let every = Modifier::all();
        let flags = [
            Modifier::BOLD,
            Modifier::ITALIC,
            Modifier::UNDERLINED,
            Modifier::SLOW_BLINK,
            Modifier::RAPID_BLINK,
            Modifier::HIDDEN,
            Modifier::CROSSED_OUT,
        ];
        for f in &flags {
            assert!(every.contains(*f));
        }
    }

    #[test]
    fn from_bits_truncate_ignores_unknown() {
        // bits above 1<<8 aren't defined; from_bits_truncate drops them.
        let m = Modifier::from_bits_truncate(0b1111_1111_1111_1111);
        assert_eq!(m, Modifier::all());
    }
}
