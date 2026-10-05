//! The packer's output half: a committed word placed on the current
//! line or wrapped to the next, the fragments it appends, atomic inline
//! boxes, and settling and breaking lines.

use crate::layout::WhiteSpace;
use rdom_core::NodeId;

use super::super::boxes::GeneratedAtom;
use super::super::vertical::{self, AtomAt, AtomRows};
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
        let must_wrap = projected > self.line_width()
            && matches!(self.ws, WhiteSpace::Normal | WhiteSpace::PreWrap);

        if must_wrap && self.line_has_content() {
            self.break_line();
            self.pending_space = false;
            self.pending_space_source = None;
            self.fit_empty_line(self.word_width);
            self.emit_word_to_current_line(0);
        } else {
            self.fit_empty_line(self.word_width);
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
        let x = i32::from(self.cur_line_width);
        self.cur_line_width = self.cur_line_width.saturating_add(width);
        if let Some(slot) = origin.generated {
            if let Some(last) = self.cur_generated.last_mut()
                && last.host == origin.owner
                && last.slot == slot
                && last.atom.is_none()
                && last.x + i32::from(last.width) == x
            {
                last.text.push_str(text);
                last.width = last.width.saturating_add(width);
                return;
            }
            let mut run = GeneratedFragment::text(origin.owner, slot, x, text);
            run.width = width;
            self.cur_generated.push(run);
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
                && last.x + i32::from(last.width) == x
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
    /// `rows` is the atom's block-axis geometry; the line it lands on
    /// grows to hold it when the line is settled (`vertical`).
    pub(in crate::render::inline) fn push_atomic_inline_block(
        &mut self,
        node: NodeId,
        width: u16,
        rows: AtomRows,
    ) {
        let x = self.open_atom(node, width);
        self.cur_atoms
            .push((AtomAt::Fragment(self.cur_fragments.len()), rows));
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
        self.close_atom(width);
    }

    /// Push an atomic inline `::before` / `::after` (CSS Display 3 §2.4,
    /// CSS Pseudo 4 §2) — `host`'s `slot` pseudo-element, `width` cells
    /// wide with `rows` — as one generated fragment holding its box,
    /// placed in the line as an element's atom is
    /// ([`Self::push_atomic_inline_block`]). Its content is laid out
    /// inside it by the layout pass.
    pub(in crate::render::inline) fn push_generated_atom(
        &mut self,
        host: NodeId,
        slot: crate::ext::PseudoSlot,
        width: u16,
        rows: AtomRows,
    ) {
        let x = self.open_atom(host, width);
        self.cur_atoms
            .push((AtomAt::Generated(self.cur_generated.len()), rows));
        let mut atom = GeneratedFragment::text(host, slot, x, "");
        atom.width = width;
        atom.atom = Some(Box::new(GeneratedAtom {
            y: 0,
            height: rows.height(),
            content: None,
        }));
        self.cur_generated.push(atom);
        self.close_atom(width);
    }

    /// Make room for an atom `width` cells wide at the inline-flow
    /// cursor: commit any pending word and flush the pending whitespace
    /// separator (a collapsed space between preceding text and the atom
    /// must emit one, otherwise `<p>hi <button>X</button> ok</p>` renders
    /// as "hi[ X ] ok"; none at the IFC start, the leading-whitespace trim
    /// invariant), wrapping first when the atom (plus separator) does not
    /// fit beside what the line holds. Its x on the line. `owner` sources
    /// a separator with no recorded provenance.
    fn open_atom(&mut self, owner: NodeId, width: u16) -> i32 {
        if !self.word_buffer.is_empty() {
            self.commit_word();
        }
        let separator: u16 = if self.pending_space && self.cur_line_width > 0 {
            1
        } else {
            0
        };
        let projected = self
            .cur_line_width
            .saturating_add(separator)
            .saturating_add(width);
        if projected > self.line_width() && self.cur_line_width > 0 {
            self.break_line();
            self.pending_space = false;
            self.pending_space_source = None;
        } else if separator > 0 {
            let (sep_origin, sep_offset) = self
                .pending_space_source
                .unwrap_or((Origin::text(owner, owner), 0));
            self.append_fragment(sep_origin, sep_offset, " ", 1);
            self.pending_space = false;
            self.pending_space_source = None;
        }
        self.fit_empty_line(width);
        i32::from(self.cur_line_width)
    }

    /// The atom just pushed takes its `width`: whitespace that follows it
    /// again emits a separator (visible content was emitted).
    fn close_atom(&mut self, width: u16) {
        self.cur_line_width = self.cur_line_width.saturating_add(width);
        self.emitted_any = true;
    }

    /// Settle the current line — its rows (`vertical`), its content at
    /// the inline-start edge of its band — and open the next one.
    pub(super) fn break_line(&mut self) {
        let mut fragments = std::mem::take(&mut self.cur_fragments);
        let mut generated = std::mem::take(&mut self.cur_generated);
        let (baseline, height) =
            vertical::settle_line(&mut fragments, &mut generated, &self.cur_atoms);
        self.cur_atoms.clear();
        let width = self.cur_line_width;
        self.cur_line_width = 0;
        // `text-align: start` (CSS Text 3 §7.1; rdom has no `text-align`
        // yet, C9-TEXT-ALIGN): flush with the band's left edge, or under
        // `rtl` its right one — a line wider than the band starting left
        // of it and overflowing its left (end) edge.
        let (start, band_width) = self.band;
        let shift = if self.rtl {
            start + i32::from(band_width) - i32::from(width)
        } else {
            start
        };
        if shift != 0 {
            for f in &mut fragments {
                f.x += shift;
            }
            for g in &mut generated {
                g.x += shift;
            }
        }
        let top = self.cur_top;
        self.cur_top = top.saturating_add(height);
        // Kept only when floats shortened the line box: otherwise its
        // edges are the content box's.
        let band = (self.band != (0, self.content_width())).then_some(self.band);
        self.lines.push(LineBox {
            fragments,
            generated,
            width,
            top,
            height,
            baseline,
            band,
        });
        self.open_line();
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
