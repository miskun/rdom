//! The packer's output half: a committed word placed on the current
//! line or wrapped to the next, the fragments it appends, atomic inline
//! boxes, and settling and breaking lines.

use crate::layout::WhiteSpace;
use rdom_core::NodeId;

use super::super::vertical::{self, AtomRows};
use super::super::{GeneratedFragment, InlineFragment, LineBox};
use super::{LinePacker, Origin};

impl LinePacker<'_> {
    /// Commit the word buffer to the current line (or wrap to a new
    /// line if it doesn't fit). Clears the buffer.
    pub(super) fn commit_word(&mut self) {
        if self.word_buffer.is_empty() {
            return;
        }

        let separator: u16 = if self.pending_space && self.cur_line_width > 0 {
            1
        } else {
            0
        };

        let projected = self
            .cur_line_width
            .saturating_add(separator)
            .saturating_add(self.word_width);
        let must_wrap = projected > self.content_width
            && matches!(self.ws, WhiteSpace::Normal | WhiteSpace::PreWrap);

        if must_wrap && self.line_has_content() {
            self.break_line();
            self.pending_space = false;
            self.pending_space_source = None;
            self.emit_word_to_current_line(0);
        } else {
            self.emit_word_to_current_line(separator);
            self.pending_space = false;
            self.pending_space_source = None;
        }
    }

    pub(super) fn emit_word_to_current_line(&mut self, separator_width: u16) {
        // Emit the separator space (if any) with the provenance of
        // the whitespace that produced it.
        if separator_width > 0 && !self.word_buffer.is_empty() {
            let (sep_origin, sep_source_offset) = self.pending_space_source.unwrap_or_else(|| {
                let g = &self.word_buffer[0];
                (g.origin, g.source_offset)
            });
            self.append_fragment(sep_origin, sep_source_offset, " ", 1);
        }

        // Group consecutive same-origin graphemes into fragments. A
        // change of origin starts a new fragment.
        let mut idx = 0;
        while idx < self.word_buffer.len() {
            let g0 = &self.word_buffer[idx];
            let origin = g0.origin;
            let source_offset = g0.source_offset;
            let mut text = String::new();
            let mut width: u16 = 0;
            while idx < self.word_buffer.len() {
                let g = &self.word_buffer[idx];
                if g.origin != origin {
                    break;
                }
                text.push_str(g.text);
                width = width.saturating_add(g.width);
                idx += 1;
            }
            self.append_fragment(origin, source_offset, &text, width);
        }

        self.word_buffer.clear();
        self.word_width = 0;
        self.emitted_any = true;
    }

    /// Append a fragment to the current line. Merges with the
    /// previous fragment when its (owner, text_node) match AND the
    /// byte ranges are contiguous — keeps fragment counts low and
    /// preserves correct source mapping. Generated content goes to the
    /// line's generated list instead.
    pub(super) fn append_fragment(
        &mut self,
        origin: Origin,
        source_offset: usize,
        text: &str,
        width: u16,
    ) {
        let x = self.cur_line_width;
        self.cur_line_width = self.cur_line_width.saturating_add(width);
        if let Some(slot) = origin.generated {
            if let Some(last) = self.cur_generated.last_mut()
                && last.host == origin.owner
                && last.slot == slot
                && last.x + last.width == x
            {
                last.text.push_str(text);
                last.width = last.width.saturating_add(width);
                return;
            }
            self.cur_generated.push(GeneratedFragment {
                host: origin.owner,
                slot,
                x,
                width,
                text: text.to_string(),
            });
            return;
        }
        let Origin {
            owner, text_node, ..
        } = origin;
        if let Some(last) = self.cur_fragments.last_mut() {
            let contiguous = last.source_byte_offset + last.text.len() == source_offset;
            if last.node == owner
                && last.text_node == text_node
                && contiguous
                && last.x + last.width == x
            {
                last.text.push_str(text);
                last.width = last.width.saturating_add(width);
                return;
            }
        }
        self.cur_fragments.push(InlineFragment {
            node: owner,
            text_node,
            source_byte_offset: source_offset,
            x,
            y: 0,
            width,
            height: 1,
            text: text.to_string(),
            atomic: false,
        });
    }

    /// True once anything — text, an atom, generated content — sits on
    /// the current line; a word that does not fit then wraps.
    pub(super) fn line_has_content(&self) -> bool {
        !self.cur_fragments.is_empty() || !self.cur_generated.is_empty()
    }

    /// Push an **atomic inline-block** fragment — a
    /// `Display::InlineBlock` element participating in IFC as a
    /// single inline-level atom (CSS 2.1 §10.8).
    ///
    /// Commits any pending word + flushes the pending whitespace
    /// separator so the atom sits at the natural inline-flow
    /// cursor. Wrap behavior: atoms wrap-aware via the same `\u{a0}`-
    /// proxy mechanism as text — emit a width-`width` placeholder
    /// grapheme to lean on the existing wrap logic, then upgrade
    /// the just-pushed fragment to `atomic = true`.
    ///
    /// `rows` is the atom's block-axis geometry; the line it lands on
    /// grows to hold it when the line is settled (`vertical`).
    pub(in crate::render::inline) fn push_atomic_inline_block(
        &mut self,
        node: NodeId,
        width: u16,
        rows: AtomRows,
    ) {
        if !self.word_buffer.is_empty() {
            self.commit_word();
        }
        // Honor `pending_space` — a collapsed whitespace between
        // preceding text and this atom MUST emit a separator
        // fragment, otherwise `<p>hi <button>X</button> ok</p>`
        // renders as "hi[ X ] ok" instead of "hi [ X ] ok".
        // Skip the separator at IFC start (cur_line_width == 0)
        // to keep the leading-whitespace trim invariant.
        let separator: u16 = if self.pending_space && self.cur_line_width > 0 {
            1
        } else {
            0
        };
        // Wrap if the atom (plus separator) doesn't fit on the
        // current line and there's already content on the line.
        let projected = self
            .cur_line_width
            .saturating_add(separator)
            .saturating_add(width);
        if projected > self.content_width && self.cur_line_width > 0 {
            self.break_line();
            self.pending_space = false;
            self.pending_space_source = None;
        } else if separator > 0 {
            let (sep_origin, sep_offset) = self
                .pending_space_source
                .unwrap_or((Origin::text(node, node), 0));
            self.append_fragment(sep_origin, sep_offset, " ", 1);
            self.pending_space = false;
            self.pending_space_source = None;
        }
        let x = self.cur_line_width;
        self.cur_atoms.push((self.cur_fragments.len(), rows));
        self.cur_fragments.push(InlineFragment {
            node,
            text_node: node, // sentinel — atom has no source text node
            source_byte_offset: 0,
            x,
            y: 0,
            width,
            height: rows.height,
            text: String::new(),
            atomic: true,
        });
        self.cur_line_width = self.cur_line_width.saturating_add(width);
        // Atoms behave like a committed word — any whitespace that
        // FOLLOWS them must again emit a separator (we just emitted
        // visible content, so `emitted_any` must be true).
        self.emitted_any = true;
    }

    pub(super) fn break_line(&mut self) {
        let mut fragments = std::mem::take(&mut self.cur_fragments);
        let generated = std::mem::take(&mut self.cur_generated);
        let (baseline, height) = vertical::settle_line(&mut fragments, &self.cur_atoms);
        self.cur_atoms.clear();
        let width = self.cur_line_width;
        self.cur_line_width = 0;
        let top = self.cur_top;
        self.cur_top = top.saturating_add(height);
        self.lines.push(LineBox {
            fragments,
            generated,
            width,
            top,
            height,
            baseline,
        });
    }

    /// Flush any pending word and the current line. Drops trailing
    /// pending_space (trailing-whitespace trim at IFC end).
    pub(in crate::render::inline) fn finish(&mut self) {
        if !self.word_buffer.is_empty() {
            self.commit_word();
        }
        self.pending_space = false;
        self.pending_space_source = None;
        if self.line_has_content() {
            self.break_line();
        }
    }
}
