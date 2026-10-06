//! Letter and word spacing (CSS Text 3 §9) in whole cells: blank cells
//! painted after a typographic character unit — rendered text its source
//! map ties to that unit, so the caret, hit-testing and selection step
//! over a unit and its spacing together and copy never sees them.
//!
//! - §9.2 `letter-spacing`: after every grapheme of text, a preserved
//!   space or a collapsed separator, but not after the last one on a line
//!   ("must not be applied at the beginning or at the end of a line") —
//!   the spacing ending a line's content is dropped when it is settled,
//!   and a word is fitted without its own trailing spacing; none inside a
//!   cursive script, whose letters join.
//! - §9.1 `word-spacing`: after each word-separator character (a space,
//!   collapsed or preserved, a no-break space, and the other separators
//!   §9.1 lists).
//! - Not after a tab (it already reaches its tab stop), a zero-width unit
//!   or an atom.

use std::borrow::Cow;

use super::super::source_map::SourceMap;
use super::{LinePacker, Origin};

impl LinePacker<'_> {
    /// The spacing after a unit of the current run rendering a word
    /// separator (`separator`) or another character `c`.
    pub(super) fn spacing_after(&self, c: char, separator: bool) -> u16 {
        let letter = if is_cursive(c) {
            0
        } else {
            self.run.letter_spacing
        };
        let word = if separator { self.run.word_spacing } else { 0 };
        letter.saturating_add(word)
    }

    /// The cells of the pending collapsed separator, its spacing included
    /// — none at the start of a line.
    pub(super) fn separator_width(&self) -> u16 {
        if self.pending_space && self.cur_line_width > 0 {
            1 + self.pending_space_spacing
        } else {
            0
        }
    }

    /// Emit the pending separator, `width` cells ([`Self::separator_width`]),
    /// at `origin` / `source_offset`: one space and its spacing, mapped to
    /// the one source unit.
    pub(super) fn push_separator(&mut self, origin: Origin, source_offset: usize, width: u16) {
        let text = super::emit::spaces(width);
        let map = (width > 1).then(|| SourceMap::new(vec![(1, u32::from(width))]));
        self.append_fragment(origin, source_offset, &text, width, map);
        self.cur_trailing_spacing = width.saturating_sub(1);
    }

    /// §9.2: no spacing at the end of a line — drop the spacing ending the
    /// current line's content (not under spaces that hang, which take
    /// their spacing with them).
    pub(super) fn drop_trailing_spacing(&mut self) {
        let n = std::mem::take(&mut self.cur_trailing_spacing);
        if n == 0 || self.cur_hang > 0 {
            return;
        }
        let line_end = i32::from(self.cur_line_width);
        let end = |x: i32, w: u16| x + i32::from(w);
        let trim = |text: &mut String| {
            let keep = text.len().saturating_sub(usize::from(n));
            text.truncate(keep);
        };
        if let Some(f) = self.cur_fragments.iter_mut().rfind(|f| !f.atomic)
            && end(f.x, f.width) == line_end
        {
            trim(&mut f.text);
            f.width = f.width.saturating_sub(n);
            if let Some(map) = f.map.as_mut()
                && let Some(unit) = map.units_mut().last_mut()
            {
                unit.1 = unit.1.saturating_sub(u32::from(n));
            }
        } else if let Some(g) = self.cur_generated.iter_mut().rfind(|g| !g.is_atom())
            && end(g.x, g.width) == line_end
        {
            trim(&mut g.text);
            g.width = g.width.saturating_sub(n);
        } else {
            return;
        }
        self.cur_line_width = self.cur_line_width.saturating_sub(n);
    }
}

/// `text` followed by `n` cells of spacing.
pub(super) fn spaced<'a>(text: Cow<'a, str>, n: u16) -> Cow<'a, str> {
    if n == 0 {
        return text;
    }
    let mut out = text.into_owned();
    out.extend(std::iter::repeat_n(' ', usize::from(n)));
    Cow::Owned(out)
}

/// A word-separator character (CSS Text 3 §9.1): the space and the
/// no-break space, the Ethiopic word space, the Aegean, Ugaritic and
/// Phoenician word separators.
pub(super) fn is_word_separator(c: char) -> bool {
    matches!(
        c,
        ' ' | '\u{A0}' | '\u{1361}' | '\u{10100}' | '\u{10101}' | '\u{1039F}' | '\u{1091F}'
    )
}

/// A character of a cursive script (CSS Text 3 §9.2: letter spacing is
/// not applied to them) — Arabic, Syriac, N'Ko, Mandaic, Mongolian,
/// Phags-pa and their presentation forms.
fn is_cursive(c: char) -> bool {
    matches!(
        c,
        '\u{0600}'..='\u{06FF}'
            | '\u{0700}'..='\u{074F}'
            | '\u{0750}'..='\u{077F}'
            | '\u{07C0}'..='\u{07FF}'
            | '\u{0840}'..='\u{085F}'
            | '\u{0860}'..='\u{086F}'
            | '\u{0870}'..='\u{08FF}'
            | '\u{1800}'..='\u{18AF}'
            | '\u{A840}'..='\u{A87F}'
            | '\u{FB50}'..='\u{FDFF}'
            | '\u{FE70}'..='\u{FEFF}'
    )
}
