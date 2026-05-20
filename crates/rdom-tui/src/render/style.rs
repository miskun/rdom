//! `Style` — paint-time style carried by each `Cell` in the buffer.
//!
//! Distinct from `TuiStyle` (author input with `Option<Value<T>>`) and
//! from `ComputedStyle` (post-cascade concrete values). `Style` is the
//! low-level per-cell representation the renderer writes to the screen.
//!
//! Two quirks worth knowing:
//!
//! - `fg` and `bg` are `Option<Color>` — `None` means "don't touch the
//!   current color slot", distinct from `Some(Color::Reset)` which
//!   explicitly resets. This lets styles compose without clobbering
//!   each other's color.
//! - Modifiers are split into `add_modifier` (bits to set) and
//!   `sub_modifier` (bits to unset). A style that "adds bold and
//!   removes italic" sets `BOLD` in `add_modifier` and `ITALIC` in
//!   `sub_modifier`. When patched onto a previous style, add wins
//!   over sub, then sub removes from whatever remains. This mirrors
//!   how the browser handles style patches on inline spans.

use super::{Color, Modifier};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub add_modifier: Modifier,
    pub sub_modifier: Modifier,
}

impl Style {
    /// Fresh style with no color changes, no modifier changes. Patching
    /// this onto anything leaves it untouched.
    pub fn new() -> Self {
        Self::default()
    }

    /// Alias of `new()` for readability at call sites that want to
    /// emphasize "no-op style".
    pub fn reset() -> Self {
        Self {
            fg: Some(Color::Reset),
            bg: Some(Color::Reset),
            add_modifier: Modifier::empty(),
            sub_modifier: Modifier::all(),
        }
    }

    pub fn fg(mut self, c: Color) -> Self {
        self.fg = Some(c);
        self
    }

    pub fn bg(mut self, c: Color) -> Self {
        self.bg = Some(c);
        self
    }

    /// Add modifier bits. Also clears them from `sub_modifier` so the
    /// net effect is "turn these on."
    pub fn add_modifier(mut self, m: Modifier) -> Self {
        self.sub_modifier.remove(m);
        self.add_modifier |= m;
        self
    }

    /// Remove modifier bits. Also clears them from `add_modifier` so
    /// the net effect is "turn these off."
    pub fn remove_modifier(mut self, m: Modifier) -> Self {
        self.add_modifier.remove(m);
        self.sub_modifier |= m;
        self
    }

    /// Apply `other` on top of `self`. Fields set in `other` override
    /// `self`; unset fields (None colors, empty modifier bits) inherit
    /// from `self`. Adds win over subs and subs win over existing bits
    /// on a per-modifier-bit basis.
    pub fn patch(self, other: Style) -> Style {
        Style {
            fg: other.fg.or(self.fg),
            bg: other.bg.or(self.bg),
            add_modifier: self.add_modifier.difference(other.sub_modifier) | other.add_modifier,
            sub_modifier: self.sub_modifier.difference(other.add_modifier) | other.sub_modifier,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_noop() {
        let s = Style::default();
        assert_eq!(s.fg, None);
        assert_eq!(s.bg, None);
        assert!(s.add_modifier.is_empty());
        assert!(s.sub_modifier.is_empty());
    }

    #[test]
    fn fg_bg_setters() {
        let s = Style::new()
            .fg(Color::Rgb(255, 0, 0))
            .bg(Color::Rgb(0, 0, 0));
        assert_eq!(s.fg, Some(Color::Rgb(255, 0, 0)));
        assert_eq!(s.bg, Some(Color::Rgb(0, 0, 0)));
    }

    #[test]
    fn add_modifier_sets_bit() {
        let s = Style::new().add_modifier(Modifier::BOLD);
        assert!(s.add_modifier.contains(Modifier::BOLD));
        assert!(!s.sub_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn remove_modifier_sets_sub_bit() {
        let s = Style::new().remove_modifier(Modifier::ITALIC);
        assert!(s.sub_modifier.contains(Modifier::ITALIC));
        assert!(!s.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn add_then_remove_same_bit_is_removed() {
        let s = Style::new()
            .add_modifier(Modifier::BOLD)
            .remove_modifier(Modifier::BOLD);
        // Sub wins — the bit was explicitly removed last.
        assert!(!s.add_modifier.contains(Modifier::BOLD));
        assert!(s.sub_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn remove_then_add_same_bit_is_added() {
        let s = Style::new()
            .remove_modifier(Modifier::BOLD)
            .add_modifier(Modifier::BOLD);
        // Add wins — the bit was explicitly added last.
        assert!(s.add_modifier.contains(Modifier::BOLD));
        assert!(!s.sub_modifier.contains(Modifier::BOLD));
    }

    // ── patch ────────────────────────────────────────────────────────

    #[test]
    fn patch_other_fg_overrides_self() {
        let a = Style::new().fg(Color::Rgb(255, 0, 0));
        let b = Style::new().fg(Color::Rgb(0, 0, 255));
        assert_eq!(a.patch(b).fg, Some(Color::Rgb(0, 0, 255)));
    }

    #[test]
    fn patch_none_inherits_self() {
        let a = Style::new().fg(Color::Rgb(255, 0, 0));
        let b = Style::new(); // fg = None
        assert_eq!(a.patch(b).fg, Some(Color::Rgb(255, 0, 0)));
    }

    #[test]
    fn patch_both_none_is_none() {
        assert_eq!(Style::new().patch(Style::new()).fg, None);
    }

    #[test]
    fn patch_accumulates_modifiers() {
        let a = Style::new().add_modifier(Modifier::BOLD);
        let b = Style::new().add_modifier(Modifier::ITALIC);
        let merged = a.patch(b);
        assert!(merged.add_modifier.contains(Modifier::BOLD));
        assert!(merged.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn patch_other_remove_clears_self_add() {
        let a = Style::new().add_modifier(Modifier::BOLD | Modifier::ITALIC);
        let b = Style::new().remove_modifier(Modifier::BOLD);
        let merged = a.patch(b);
        assert!(!merged.add_modifier.contains(Modifier::BOLD));
        assert!(merged.add_modifier.contains(Modifier::ITALIC));
        assert!(merged.sub_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn patch_other_add_clears_self_sub() {
        let a = Style::new().remove_modifier(Modifier::BOLD);
        let b = Style::new().add_modifier(Modifier::BOLD);
        let merged = a.patch(b);
        assert!(merged.add_modifier.contains(Modifier::BOLD));
        assert!(!merged.sub_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn patch_chain_preserves_order() {
        // (fg=red, +bold) patched with (+italic) patched with (fg=blue, -bold)
        let merged = Style::new()
            .fg(Color::Rgb(255, 0, 0))
            .add_modifier(Modifier::BOLD)
            .patch(Style::new().add_modifier(Modifier::ITALIC))
            .patch(
                Style::new()
                    .fg(Color::Rgb(0, 0, 255))
                    .remove_modifier(Modifier::BOLD),
            );
        assert_eq!(merged.fg, Some(Color::Rgb(0, 0, 255)));
        assert!(!merged.add_modifier.contains(Modifier::BOLD));
        assert!(merged.add_modifier.contains(Modifier::ITALIC));
        assert!(merged.sub_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn reset_style_wipes_everything_when_patched_on() {
        let a = Style::new()
            .fg(Color::Rgb(255, 0, 0))
            .bg(Color::Rgb(0, 0, 0))
            .add_modifier(Modifier::BOLD | Modifier::ITALIC);
        let merged = a.patch(Style::reset());
        assert_eq!(merged.fg, Some(Color::Reset));
        assert_eq!(merged.bg, Some(Color::Reset));
        assert!(merged.add_modifier.is_empty());
    }

    #[test]
    fn copy_semantics() {
        let a = Style::new().fg(Color::Rgb(255, 0, 0));
        let b = a; // Copy
        assert_eq!(a, b);
    }
}
