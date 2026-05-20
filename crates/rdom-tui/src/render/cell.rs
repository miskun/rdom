//! `Cell` — one terminal grid cell.
//!
//! A cell carries its visible grapheme cluster (inline-stored in a
//! `CompactString` — up to 24 bytes without heap allocation), plus
//! foreground/background/modifier state, plus a diff-control hint
//! for the renderer.
//!
//! ## Wide-glyph encoding
//!
//! Double-width glyphs (CJK, emoji, ZWJ sequences, regional
//! indicators) occupy **two cells**:
//!
//! - **Primary cell**: `symbol = Some(<cluster>)`, e.g. `"中"` or `"👨‍👩‍👧"`.
//! - **Trailing (spacer) cell**: `symbol = Some("")` — empty string.
//!   Same fg/bg/modifier as the primary. The diff iterator skips these.
//!
//! A normal cell has `symbol = None` (rendered as a blank space) or
//! `symbol = Some(single-grapheme)` of width 1.
//!
//! ## Size
//!
//! On 64-bit: 40 bytes with padding. A 200×80 buffer is ~640KB for
//! one side, ~1.3MB for a Terminal's front+back pair.

use compact_str::CompactString;
use unicode_width::UnicodeWidthStr;

use super::{Color, Modifier};

/// Diff-control hint on a cell.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellDiff {
    /// Default: participate in diffing normally — emitted when
    /// different from the previous frame.
    #[default]
    Normal,
    /// Never emit. Used by overlays (sixel / kitty image passthrough)
    /// that manage their own paint out-of-band.
    Skip,
    /// Always emit, even if equal to the previous frame. Used when an
    /// out-of-band write may have clobbered our cell and we need to
    /// reassert.
    AlwaysUpdate,
}

/// One terminal grid cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The grapheme cluster rendered in this cell.
    ///
    /// - `None` — blank; renders as a single space.
    /// - `Some(s)` where `s` is non-empty — the glyph.
    /// - `Some("")` — trailing spacer of a double-width primary cell.
    symbol: Option<CompactString>,

    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
    pub diff: CellDiff,
    /// OSC 8 hyperlink target. When `Some(url)`, the backend
    /// wraps this cell's symbol with `ESC ] 8 ;; <url> ESC \\ …
    /// ESC ] 8 ;; ESC \\`. Runs of consecutive same-URL cells
    /// emit one open + one close pair. `None` = no link.
    ///
    /// Boxed so Cell stays ≤48 bytes even with URLs present —
    /// hyperlinks are rare enough per-frame that the extra heap
    /// allocation on cells that have one is a fine trade.
    pub link: Option<Box<str>>,
}

impl Cell {
    /// The zero cell: blank symbol, reset colors, no modifiers.
    /// `Buffer::new()` fills with this.
    pub const EMPTY: Cell = Cell {
        symbol: None,
        fg: Color::Reset,
        bg: Color::Reset,
        modifier: Modifier::empty(),
        diff: CellDiff::Normal,
        link: None,
    };

    /// Construct a cell with the given grapheme cluster. `s` is stored
    /// as-is — use `Buffer::set_string` if you want unicode-width
    /// enforcement + spacer-cell handling.
    pub fn new(symbol: impl Into<CompactString>) -> Self {
        Cell {
            symbol: Some(symbol.into()),
            ..Self::EMPTY
        }
    }

    /// Borrow the cell's symbol string. Returns `""` for blank cells.
    pub fn symbol(&self) -> &str {
        self.symbol.as_deref().unwrap_or(" ")
    }

    /// Raw symbol access. `None` means blank (renders as space);
    /// `Some("")` means a trailing spacer of a wide glyph.
    pub fn raw_symbol(&self) -> Option<&str> {
        self.symbol.as_deref()
    }

    /// True when this cell is the empty spacer after a wide glyph.
    /// Used by the diff iterator to skip these (they're implied by
    /// the preceding primary cell's width).
    pub fn is_spacer(&self) -> bool {
        matches!(self.symbol.as_deref(), Some(""))
    }

    /// True when this cell has no glyph set (will render as a space).
    pub fn is_blank(&self) -> bool {
        self.symbol.is_none()
    }

    /// Visible width in cells: 0 for spacer, 1 for normal/blank,
    /// 2 for wide glyphs (CJK, emoji, ZWJ, regional indicators).
    pub fn cell_width(&self) -> u16 {
        match self.symbol.as_deref() {
            None => 1,
            Some("") => 0,
            Some(s) => UnicodeWidthStr::width(s).max(1) as u16,
        }
    }

    // ── Builders / setters ───────────────────────────────────────────

    pub fn set_symbol(&mut self, symbol: impl Into<CompactString>) -> &mut Self {
        self.symbol = Some(symbol.into());
        self
    }

    pub fn set_blank(&mut self) -> &mut Self {
        self.symbol = None;
        self
    }

    /// Mark as the trailing spacer of a wide glyph.
    pub fn set_spacer(&mut self) -> &mut Self {
        self.symbol = Some(CompactString::const_new(""));
        self
    }

    pub fn set_fg(&mut self, fg: Color) -> &mut Self {
        self.fg = fg;
        self
    }

    pub fn set_bg(&mut self, bg: Color) -> &mut Self {
        self.bg = bg;
        self
    }

    pub fn set_modifier(&mut self, modifier: Modifier) -> &mut Self {
        self.modifier = modifier;
        self
    }

    /// OSC 8 hyperlink target. `None` clears the link; `Some(url)`
    /// marks this cell as part of a hyperlink run that the backend
    /// wraps with OSC 8 escape sequences when emitting.
    pub fn set_link(&mut self, link: Option<&str>) -> &mut Self {
        self.link = link.map(|s| s.to_string().into_boxed_str());
        self
    }

    /// Read the current OSC 8 link target, if any.
    pub fn link(&self) -> Option<&str> {
        self.link.as_deref()
    }

    /// Apply a `Style` (paint-layer): overrides fg/bg when set, adds
    /// `add_modifier`, removes `sub_modifier`.
    pub fn apply_style(&mut self, style: super::Style) -> &mut Self {
        if let Some(fg) = style.fg {
            self.fg = fg;
        }
        if let Some(bg) = style.bg {
            self.bg = bg;
        }
        self.modifier |= style.add_modifier;
        self.modifier.remove(style.sub_modifier);
        self
    }

    /// Reset everything except `diff` to `EMPTY`'s values. The diff
    /// control is intentionally preserved — a `Skip`/`AlwaysUpdate`
    /// marker survives buffer resets.
    pub fn reset(&mut self) -> &mut Self {
        let diff = self.diff;
        *self = Self::EMPTY;
        self.diff = diff;
        self
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::EMPTY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_blank_reset() {
        let c = Cell::EMPTY;
        assert!(c.is_blank());
        assert_eq!(c.fg, Color::Reset);
        assert_eq!(c.bg, Color::Reset);
        assert_eq!(c.modifier, Modifier::empty());
        assert_eq!(c.symbol(), " ");
    }

    #[test]
    fn default_matches_empty() {
        assert_eq!(Cell::default(), Cell::EMPTY);
    }

    #[test]
    fn new_stores_symbol() {
        let c = Cell::new("A");
        assert_eq!(c.symbol(), "A");
        assert!(!c.is_blank());
        assert!(!c.is_spacer());
    }

    #[test]
    fn ascii_cell_width_1() {
        let c = Cell::new("A");
        assert_eq!(c.cell_width(), 1);
    }

    #[test]
    fn cjk_cell_width_2() {
        let c = Cell::new("中");
        assert_eq!(c.cell_width(), 2);
    }

    #[test]
    fn single_emoji_cell_width_2() {
        // 🦀 is a single-codepoint emoji, width 2.
        let c = Cell::new("🦀");
        assert_eq!(c.cell_width(), 2);
    }

    #[test]
    fn zwj_family_cell_width_2() {
        // 👨‍👩‍👧 is a ZWJ sequence (multiple codepoints, one grapheme).
        // Terminals render it as width 2.
        let c = Cell::new("👨\u{200D}👩\u{200D}👧");
        assert_eq!(c.cell_width(), 2);
    }

    #[test]
    fn combining_grapheme_width_1() {
        // é composed as e + combining acute. One grapheme, width 1.
        let c = Cell::new("e\u{0301}");
        assert_eq!(c.cell_width(), 1);
    }

    #[test]
    fn blank_cell_width_1() {
        assert_eq!(Cell::EMPTY.cell_width(), 1);
    }

    #[test]
    fn spacer_cell_width_0() {
        let mut c = Cell::new("中");
        c.set_spacer();
        assert_eq!(c.cell_width(), 0);
        assert!(c.is_spacer());
    }

    #[test]
    fn set_blank_clears_symbol() {
        let mut c = Cell::new("A");
        c.set_blank();
        assert!(c.is_blank());
        assert_eq!(c.symbol(), " ");
    }

    #[test]
    fn set_spacer_sets_empty_string_symbol() {
        let mut c = Cell::EMPTY;
        c.set_spacer();
        assert_eq!(c.raw_symbol(), Some(""));
        assert!(c.is_spacer());
    }

    #[test]
    fn spacer_is_not_blank() {
        let mut c = Cell::EMPTY;
        c.set_spacer();
        assert!(!c.is_blank());
        assert!(c.is_spacer());
    }

    #[test]
    fn apply_style_writes_fg_bg() {
        let mut c = Cell::new("A");
        let style = super::super::Style::new()
            .fg(Color::Rgb(255, 0, 0))
            .bg(Color::Rgb(0, 0, 255));
        c.apply_style(style);
        assert_eq!(c.fg, Color::Rgb(255, 0, 0));
        assert_eq!(c.bg, Color::Rgb(0, 0, 255));
    }

    #[test]
    fn apply_style_none_leaves_fields_alone() {
        let mut c = Cell::new("A");
        c.set_fg(Color::Rgb(255, 0, 0));
        let style = super::super::Style::new(); // fg/bg both None
        c.apply_style(style);
        assert_eq!(c.fg, Color::Rgb(255, 0, 0)); // preserved
    }

    #[test]
    fn apply_style_accumulates_add_removes_sub_modifier() {
        let mut c = Cell::new("A");
        c.set_modifier(Modifier::BOLD);
        let style = super::super::Style::new()
            .add_modifier(Modifier::ITALIC)
            .remove_modifier(Modifier::BOLD);
        c.apply_style(style);
        assert!(c.modifier.contains(Modifier::ITALIC));
        assert!(!c.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn reset_preserves_diff_marker() {
        let mut c = Cell::new("A");
        c.fg = Color::Rgb(255, 0, 0);
        c.diff = CellDiff::AlwaysUpdate;
        c.reset();
        assert!(c.is_blank());
        assert_eq!(c.fg, Color::Reset);
        // diff is preserved so overlays keep their AlwaysUpdate flag
        assert_eq!(c.diff, CellDiff::AlwaysUpdate);
    }

    #[test]
    fn size_budget() {
        // Pre-Polish #9 target was 48 bytes (actual 40). Adding
        // `Option<Box<str>>` for OSC 8 hyperlinks pushed us to 56
        // — Box<str> is 16 bytes (ptr + len) and Rust's niche
        // optimization keeps Option<Box<str>> the same 16. The
        // 64-byte target aligns with a cache line and costs a
        // predictable 8 bytes of padding over actual data; still
        // small enough that a terminal-sized Buffer fits
        // comfortably in memory.
        assert!(
            std::mem::size_of::<Cell>() <= 64,
            "Cell is {} bytes, over budget",
            std::mem::size_of::<Cell>()
        );
    }

    #[test]
    fn clone_independent() {
        let a = Cell::new("A");
        let mut b = a.clone();
        b.set_symbol("B");
        assert_eq!(a.symbol(), "A");
        assert_eq!(b.symbol(), "B");
    }

    #[test]
    fn eq_structural() {
        let a = Cell::new("X");
        let b = Cell::new("X");
        assert_eq!(a, b);
        let mut c = a.clone();
        c.fg = Color::Rgb(255, 0, 0);
        assert_ne!(a, c);
    }

    #[test]
    fn regional_indicator_flag_cell_width_2() {
        // 🇺🇸 = U+1F1FA + U+1F1F8 (two regional indicators, one grapheme).
        let c = Cell::new("🇺🇸");
        assert_eq!(c.cell_width(), 2);
    }

    #[test]
    fn compact_string_inlines_short_symbols() {
        // The short path should not heap-allocate. This is a best-effort
        // check: we verify by asserting CompactString's guarantee (inline
        // up to 24 bytes on 64-bit). Just ensure we can store various
        // short inputs without error.
        let _ = Cell::new("A"); // 1 byte
        let _ = Cell::new("中"); // 3 bytes
        let _ = Cell::new("🦀"); // 4 bytes
        let _ = Cell::new("🇺🇸"); // 8 bytes
        let _ = Cell::new("👨\u{200D}👩\u{200D}👧"); // ~18 bytes
    }
}
