//! The packer's output half: a committed word placed on the current
//! line or wrapped to the next, the fragments it appends, atomic inline
//! boxes, and settling and breaking lines.

use rdom_core::NodeId;

use super::super::boxes::GeneratedAtom;
use super::super::vertical::{self, AtomAt, AtomRows};
use super::super::{GeneratedFragment, InlineFragment, LineBox};
use super::{GraphemeKind, LineEnd, LinePacker, Origin, PendingGrapheme};

impl LinePacker<'_> {
    /// Commit the word buffer to the current line (or wrap to a new
    /// line if it doesn't fit). Clears the buffer.
    ///
    /// Collapsed spaces ending the buffer become the pending separator;
    /// one starting it goes when the word starts a line (CSS Text 3
    /// §4.1.2). Preserved spaces ending it that hang are not measured for
    /// fit.
    pub(super) fn commit_word(&mut self) {
        // The collapsed space ending the word separates it from what
        // follows: pending once the word is placed.
        let mut trailing: Option<(Origin, usize, bool)> = None;
        while let Some(&PendingGrapheme {
            kind: GraphemeKind::Collapsible { segment_break },
            origin,
            source_offset,
            width,
            ..
        }) = self.word_buffer.last()
        {
            self.word_buffer.pop();
            self.word_width = self.word_width.saturating_sub(width);
            let sb = trailing.map_or(segment_break, |(.., b)| b || segment_break);
            trailing = Some((origin, source_offset, sb));
        }
        self.place_word();
        if let Some((origin, source_offset, segment_break)) = trailing {
            if !self.pending_space {
                self.pending_space = true;
                self.pending_space_source = Some((origin, source_offset));
            }
            self.pending_segment_break |= segment_break;
        }
    }

    /// Place the word buffer on the current line, or wrap to a new line
    /// first when it does not fit and may wrap.
    fn place_word(&mut self) {
        if self.word_buffer.is_empty() {
            return;
        }

        let separator: u16 = if self.pending_space && self.cur_line_width > 0 {
            1
        } else {
            0
        };
        let fit = self.word_width.saturating_sub(self.word_hang());
        let projected = self
            .cur_line_width
            .saturating_add(separator)
            .saturating_add(fit);
        let must_wrap = projected > self.line_width() && self.buffer_wraps;

        if must_wrap && self.line_has_content() {
            self.break_line(LineEnd::Soft);
            self.clear_pending_space();
            self.drop_leading_collapsible();
            self.fit_empty_line(self.word_width.saturating_sub(self.word_hang()));
            self.emit_word_to_current_line(0);
        } else {
            self.fit_empty_line(fit);
            if self.cur_line_width == 0 {
                self.drop_leading_collapsible();
            }
            self.emit_word_to_current_line(separator);
            self.clear_pending_space();
        }
    }

    /// The cells of preserved spaces ending the word buffer that hang at
    /// the end of a line (CSS Text 3 §4.1.2).
    fn word_hang(&self) -> u16 {
        self.word_buffer
            .iter()
            .rev()
            .take_while(|g| g.kind == GraphemeKind::Preserved { hangs: true })
            .map(|g| g.width)
            .sum()
    }

    /// A collapsed space starting the word buffer goes when the word
    /// starts a line (CSS Text 3 §4.1.2).
    fn drop_leading_collapsible(&mut self) {
        if let Some(&PendingGrapheme {
            kind: GraphemeKind::Collapsible { .. },
            width,
            ..
        }) = self.word_buffer.first()
        {
            self.word_buffer.remove(0);
            self.word_width = self.word_width.saturating_sub(width);
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
        let hang = self.word_hang();

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
        self.cur_hang = hang;
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
        self.commit_word();
        let separator: u16 = if self.pending_space && self.cur_line_width > 0 {
            1
        } else {
            0
        };
        let projected = self
            .cur_line_width
            .saturating_add(separator)
            .saturating_add(width);
        // The opportunity before the atom wraps after a collapsed space
        // or text that wraps (`text-wrap-mode`).
        let wraps = self.pending_space || self.last_wraps;
        if projected > self.line_width() && self.cur_line_width > 0 && wraps {
            self.break_line(LineEnd::Soft);
            self.clear_pending_space();
        } else if separator > 0 {
            let (sep_origin, sep_offset) = self
                .pending_space_source
                .unwrap_or((Origin::text(owner, owner), 0));
            self.append_fragment(sep_origin, sep_offset, " ", 1);
            self.clear_pending_space();
        }
        self.fit_empty_line(width);
        i32::from(self.cur_line_width)
    }

    /// The atom just pushed takes its `width`: whitespace that follows it
    /// again emits a separator (visible content was emitted).
    fn close_atom(&mut self, width: u16) {
        self.cur_line_width = self.cur_line_width.saturating_add(width);
        self.cur_hang = 0;
        self.emitted_any = true;
        self.after_atom = true;
    }

    /// Settle the current line — its rows (`vertical`), its content at
    /// the inline-start edge of its band, its hanging spaces (CSS Text 3
    /// §4.1.2: at a soft wrap they hang, at a forced break or the end only
    /// where they overflow) — and open the next one.
    pub(super) fn break_line(&mut self, end: LineEnd) {
        let mut fragments = std::mem::take(&mut self.cur_fragments);
        let mut generated = std::mem::take(&mut self.cur_generated);
        let (baseline, height) =
            vertical::settle_line(&mut fragments, &mut generated, &self.cur_atoms);
        self.cur_atoms.clear();
        let width = self.cur_line_width;
        let (start, band_width) = self.band;
        let hang = match end {
            LineEnd::Soft => self.cur_hang,
            LineEnd::Forced => self.cur_hang.min(width.saturating_sub(band_width)),
        };
        self.cur_line_width = 0;
        self.cur_hang = 0;
        // `text-align: start` (CSS Text 3 §7.1; rdom has no `text-align`
        // yet, C9-TEXT-ALIGN): flush with the band's left edge, or under
        // `rtl` its right one — a line wider than the band starting left
        // of it and overflowing its left (end) edge.
        let shift = if self.rtl {
            start + i32::from(band_width) - i32::from(width - hang)
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
            hang,
        });
        self.open_line();
    }

    /// Flush any pending word and the current line. Drops trailing
    /// pending_space (trailing-whitespace trim at IFC end).
    pub(in crate::render::inline) fn finish(&mut self) {
        self.commit_word();
        self.clear_pending_space();
        if self.line_has_content() {
            self.break_line(LineEnd::Forced);
        }
    }
}
