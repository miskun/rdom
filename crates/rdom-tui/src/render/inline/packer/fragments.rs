//! The packer's fragments: a placed word's graphemes grouped into
//! [`InlineFragment`]s (and generated runs), merged with the line's last
//! fragment when their source is contiguous, each with the `SourceMap`
//! its rendering needs; and the hyphen a line broken at a soft hyphen
//! shows (CSS Text 3 §6.1).

use std::borrow::Cow;

use super::super::source_map::SourceMap;
use super::super::{GeneratedFragment, InlineFragment};
use super::{GraphemeKind, LinePacker, Origin};

impl LinePacker<'_> {
    /// Emit the word buffer onto the current line after a separator
    /// `separator_width` cells wide (none, or one with its spacing).
    pub(super) fn emit_word_to_current_line(&mut self, separator_width: u16) {
        // Emit the separator space (if any) with the provenance of
        // the whitespace that produced it.
        if separator_width > 0 && !self.word_buffer.is_empty() {
            let (sep_origin, sep_source_offset) = self.pending_space_source.unwrap_or_else(|| {
                let g = &self.word_buffer[0];
                (g.origin, g.source_offset)
            });
            self.push_separator(sep_origin, sep_source_offset, separator_width);
        }
        let trailing_spacing = self.word_buffer.last().map_or(0, |g| g.spacing);
        let hang = self.word_hang();
        self.cur_has_tab |= self
            .word_buffer
            .iter()
            .any(|g| matches!(g.kind, GraphemeKind::Preserved { tab: Some(_), .. }));
        let ends_in_shy = matches!(
            self.word_buffer.last().map(|g| g.kind),
            Some(GraphemeKind::SoftHyphen { shows: true })
        );

        // Group consecutive same-origin graphemes into fragments. A
        // change of origin starts a new fragment.
        // The units of a source map are built only for a group that needs
        // one (a grapheme rendered as something else).
        let mut idx = 0;
        while idx < self.word_buffer.len() {
            let g0 = &self.word_buffer[idx];
            let origin = g0.origin;
            let source_offset = g0.source_offset;
            let end = self.word_buffer[idx..]
                .iter()
                .position(|g| g.origin != origin)
                .map_or(self.word_buffer.len(), |n| idx + n);
            let group = &self.word_buffer[idx..end];
            let mut text = String::with_capacity(group.iter().map(|g| g.text.len()).sum());
            let mut width: u16 = 0;
            for g in group {
                text.push_str(&g.text);
                width = width.saturating_add(g.width);
            }
            let map = group.iter().any(|g| g.mapped).then(|| {
                SourceMap::new(
                    group
                        .iter()
                        .map(|g| (g.source_len as u32, g.text.len() as u32))
                        .collect(),
                )
            });
            idx = end;
            self.append_fragment(origin, source_offset, Cow::Owned(text), width, map);
        }

        self.word_buffer.clear();
        self.word_width = 0;
        self.cur_hang = hang;
        self.cur_trailing_spacing = trailing_spacing;
        self.cur_ends_in_shy = ends_in_shy;
        self.emitted_any = true;
    }

    /// Append a fragment to the current line. Merges with the
    /// previous fragment when its (owner, text_node) match AND the
    /// byte ranges are contiguous — keeps fragment counts low and
    /// preserves correct source mapping. Generated content goes to the
    /// line's generated list instead. `map` is the text's source map
    /// when it is not the source verbatim. An owned `text` becomes a new
    /// fragment's without a copy.
    pub(super) fn append_fragment(
        &mut self,
        origin: Origin,
        source_offset: usize,
        text: Cow<'_, str>,
        width: u16,
        map: Option<SourceMap>,
    ) {
        self.frames.mark(origin.frame);
        let x = i32::from(self.cur_line_width);
        self.cur_line_width = self.cur_line_width.saturating_add(width);
        if let Some(slot) = origin.generated {
            if let Some(last) = self.cur_generated.last_mut()
                && last.host == origin.owner
                && last.slot == slot
                && last.frame == origin.frame
                && last.first_letter == origin.letter
                && last.atom.is_none()
                && last.x + i32::from(last.width) == x
            {
                last.text.push_str(&text);
                last.width = last.width.saturating_add(width);
                return;
            }
            let mut run = GeneratedFragment::text(origin.owner, slot, x, text.into_owned());
            run.width = width;
            run.frame = origin.frame;
            run.first_letter = origin.letter;
            self.cur_generated.push(run);
            return;
        }
        let Origin {
            owner, text_node, ..
        } = origin;
        if let Some(last) = self.cur_fragments.last_mut() {
            let contiguous = last.source_byte_offset + last.source_len() == source_offset;
            if last.node == owner
                && last.text_node == text_node
                && last.frame == origin.frame
                && last.first_letter == origin.letter
                && contiguous
                && last.x + i32::from(last.width) == x
            {
                if last.map.is_some() || map.is_some() {
                    let mut joined = last
                        .map
                        .take()
                        .map_or_else(|| SourceMap::verbatim(&last.text), |m| *m);
                    joined.extend(&map.unwrap_or_else(|| SourceMap::verbatim(&text)));
                    last.map = Some(Box::new(joined));
                }
                last.text.push_str(&text);
                last.width = last.width.saturating_add(width);
                return;
            }
        }
        // A new fragment takes the text, moved when it is owned.
        let mut fragment =
            InlineFragment::text(owner, text_node, source_offset, x, text.into_owned());
        fragment.width = width;
        fragment.map = map.map(Box::new);
        fragment.frame = origin.frame;
        fragment.first_letter = origin.letter;
        self.cur_fragments.push(fragment);
    }

    /// The line breaks after a soft hyphen: it shows a hyphen, one cell
    /// at the end of the line's last text (CSS Text 3 §6.1), which
    /// renders the soft hyphen's source bytes.
    pub(super) fn show_hyphen(&mut self) {
        let end = |x: i32, w: u16| x + i32::from(w);
        let last_text = self.cur_fragments.iter_mut().rfind(|f| !f.atomic);
        let last_gen = self.cur_generated.iter_mut().rfind(|g| !g.is_atom());
        let line_end = i32::from(self.cur_line_width);
        if let Some(f) = last_text
            && end(f.x, f.width) == line_end
        {
            f.text.push('-');
            f.width = f.width.saturating_add(1);
            if let Some(map) = f.map.as_mut()
                && let Some(text) = map.text_bytes_mut().last()
            {
                *text += 1;
            }
        } else if let Some(g) = last_gen
            && end(g.x, g.width) == line_end
        {
            g.text.push('-');
            g.width = g.width.saturating_add(1);
        } else {
            return;
        }
        self.cur_line_width = self.cur_line_width.saturating_add(1);
    }
}
