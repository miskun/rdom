//! The packer's output half: a committed word placed on the current
//! line or wrapped to the next, the fragments it appends, atomic inline
//! boxes, and settling and breaking lines.

use rdom_core::NodeId;

use super::super::align::{self, LineGeometry, TextAlignment};
use super::super::boxes::GeneratedAtom;
use super::super::vertical::{self, AtomAt, AtomRows};
use super::super::{GeneratedFragment, InlineFragment, LineBox};
use super::{BoxAlign, BoxRows, GraphemeKind, LineEnd, LinePacker, Op, Origin, PendingGrapheme};
use crate::layout::OverflowWrap;

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
        let mut trailing: Option<(Origin, usize, bool, u16)> = None;
        while let Some(&PendingGrapheme {
            kind: GraphemeKind::Collapsible { segment_break },
            origin,
            source_offset,
            width,
            spacing,
            ..
        }) = self.word_buffer.last()
        {
            self.word_buffer.pop();
            self.word_width = self.word_width.saturating_sub(width);
            let sb = trailing.map_or(segment_break, |(.., b, _)| b || segment_break);
            trailing = Some((origin, source_offset, sb, spacing));
        }
        self.place_word();
        if let Some((origin, source_offset, segment_break, spacing)) = trailing {
            if !self.pending_space {
                self.pending_space = true;
                self.pending_space_source = Some((origin, source_offset));
                self.pending_space_spacing = spacing;
            }
            self.pending_segment_break |= segment_break;
        }
    }

    /// Place the word buffer on the current line, or wrap to a new line
    /// first when it does not fit and may wrap; a word too long for the
    /// line it then starts breaks where its `overflow-wrap` lets it
    /// (CSS Text 3 §5.5, [`Self::split_word`]).
    fn place_word(&mut self) {
        if self.word_buffer.is_empty() {
            return;
        }

        let separator = self.separator_width();
        self.layout_tabs(self.cur_line_width.saturating_add(separator));
        let fit = self.word_fit();
        let projected = self
            .cur_line_width
            .saturating_add(separator)
            .saturating_add(fit);
        let must_wrap = projected > self.line_width() && self.buffer_wraps;

        if must_wrap && self.line_has_content() {
            self.break_line(LineEnd::Soft);
            self.clear_pending_space();
            self.drop_leading_collapsible();
            self.layout_tabs(0);
            self.fit_empty_line(self.word_fit());
            self.split_word();
            self.emit_word_to_current_line(0);
        } else {
            self.fit_empty_line(fit);
            if self.cur_line_width == 0 {
                self.drop_leading_collapsible();
                self.split_word();
            }
            self.emit_word_to_current_line(separator);
            self.clear_pending_space();
        }
    }

    /// Place the word buffer's tabs at their tab stops (CSS Text 3 §4.2):
    /// the word starts `at` cells into the current line, whose start is
    /// [`Self::line_origin`] cells from the block's starting content edge;
    /// each tab "lines up the start edge of the next glyph with the next
    /// tab stop", the multiples of its tab size from that edge — none
    /// rendered at a tab size of 0.
    pub(super) fn layout_tabs(&mut self, at: u16) {
        let mut pos = self.line_origin().saturating_add(at);
        let mut changed = false;
        for g in &mut self.word_buffer {
            if let GraphemeKind::Preserved {
                tab: Some(size), ..
            } = g.kind
            {
                let width = if size == 0 { 0 } else { size - pos % size };
                if width != g.width {
                    g.width = width;
                    g.text = spaces(width);
                    g.mapped = width != 1;
                    changed = true;
                }
            }
            pos = pos.saturating_add(g.width);
        }
        if changed {
            self.word_width = self.word_buffer.iter().map(|g| g.width).sum();
        }
    }

    /// The cells the word buffer needs on its line: its width less the
    /// spaces that would hang at the line's end and the spacing ending it,
    /// which a line's end drops, plus the hyphen a soft hyphen ending it
    /// shows if the line breaks there (CSS Text 3 §4.1.2, §6.1, §9.2).
    fn word_fit(&self) -> u16 {
        let (shy, spacing) = match self.word_buffer.last() {
            Some(PendingGrapheme {
                kind: GraphemeKind::SoftHyphen { shows: true },
                ..
            }) => (true, 0),
            Some(PendingGrapheme {
                kind: GraphemeKind::Preserved { hangs: true, .. },
                ..
            })
            | None => (false, 0),
            Some(g) => (false, g.spacing),
        };
        self.word_width
            .saturating_sub(self.word_hang())
            .saturating_sub(spacing)
            .saturating_add(u16::from(shy))
    }

    /// `overflow-wrap: anywhere` / `break-word` (CSS Text 3 §5.5): while
    /// the word buffer, starting an empty line, is wider than the line,
    /// place as many of its graphemes as fit (one at least) — breaking
    /// only after a grapheme whose text allows it — and wrap. A
    /// min-content measurement takes `anywhere`'s breaks only ("soft wrap
    /// opportunities introduced by break-word are not considered").
    fn split_word(&mut self) {
        let min_content = self.is_measuring() && self.content_width() == 0;
        let breaks_after = |g: &PendingGrapheme<'_>| match g.split {
            OverflowWrap::Normal => false,
            OverflowWrap::BreakWord => !min_content,
            OverflowWrap::Anywhere => true,
        };
        while self.word_fit() > self.line_width() && self.buffer_wraps {
            let room = self.line_width();
            let (mut used, mut cut) = (0u16, 0usize);
            for (i, g) in self.word_buffer.iter().enumerate() {
                // The spacing of the line's last grapheme is dropped.
                if used.saturating_add(g.width - g.spacing) > room && cut > 0 {
                    break;
                }
                used = used.saturating_add(g.width);
                if i + 1 < self.word_buffer.len() && breaks_after(g) {
                    cut = i + 1;
                }
            }
            if cut == 0 {
                return;
            }
            let rest = self.word_buffer.split_off(cut);
            let rest_width: u16 = rest.iter().map(|g| g.width).sum();
            self.word_width = self.word_width.saturating_sub(rest_width);
            self.emit_word_to_current_line(0);
            self.break_line(LineEnd::Soft);
            self.word_buffer = rest;
            self.word_width = rest_width;
            self.layout_tabs(0);
            self.fit_empty_line(self.word_fit());
        }
    }

    /// The cells of preserved spaces ending the word buffer that hang at
    /// the end of a line (CSS Text 3 §4.1.2).
    pub(super) fn word_hang(&self) -> u16 {
        self.word_buffer
            .iter()
            .rev()
            .take_while(|g| matches!(g.kind, GraphemeKind::Preserved { hangs: true, .. }))
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
        align: BoxAlign,
    ) {
        self.log(Op::Atom {
            node,
            width,
            rows,
            align,
        });
        let x = self.open_atom(node, width);
        let frame = self.atom_frame(rows, align);
        self.cur_atoms
            .push((AtomAt::Fragment(self.cur_fragments.len()), rows, frame));
        let mut atom = InlineFragment::atom(node, x, width, rows.height);
        atom.frame = frame;
        self.cur_fragments.push(atom);
        self.close_atom(width);
    }

    /// The inline box of an atom `rows` tall aligned by `align`, in the
    /// current one, placed on the current line (CSS 2.1 §10.8.1: its
    /// margin box is its box).
    fn atom_frame(&mut self, rows: AtomRows, align: BoxAlign) -> u32 {
        let box_rows = BoxRows {
            above: rows.above(),
            below: rows.below(),
        };
        let frame = self.frames.add(box_rows, align);
        self.frames.mark(frame);
        frame
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
        align: BoxAlign,
    ) {
        self.log(Op::GeneratedAtom {
            host,
            slot,
            width,
            rows,
            align,
        });
        let x = self.open_atom(host, width);
        let frame = self.atom_frame(rows, align);
        self.cur_atoms
            .push((AtomAt::Generated(self.cur_generated.len()), rows, frame));
        let mut atom = GeneratedFragment::text(host, slot, x, "");
        atom.width = width;
        atom.frame = frame;
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
        let separator = self.separator_width();
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
                .unwrap_or((Origin::text(owner, owner, self.frames.current()), 0));
            self.push_separator(sep_origin, sep_offset, separator);
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
        self.cur_trailing_spacing = 0;
        self.cur_ends_in_shy = false;
        self.last_class = None;
        self.emitted_any = true;
        self.after_atom = true;
    }

    /// Settle the current line — its rows (`vertical`), its content at
    /// the inline-start edge of its band, its hanging spaces (CSS Text 3
    /// §4.1.2: at a soft wrap they hang, at a forced break or the end only
    /// where they overflow) — and open the next one.
    pub(super) fn break_line(&mut self, end: LineEnd) {
        // §9.2: no letter spacing at the end of a line.
        self.drop_trailing_spacing();
        // §6.1: a line broken at a soft hyphen shows a hyphen — not when
        // it breaks at a collapsed space after it, or is forced.
        if end == LineEnd::Soft && self.cur_ends_in_shy && !self.pending_space {
            self.show_hyphen();
        }
        self.cur_ends_in_shy = false;
        let mut fragments = std::mem::take(&mut self.cur_fragments);
        let mut generated = std::mem::take(&mut self.cur_generated);
        let settled = self.frames.settle();
        let frames = &self.frames;
        vertical::settle_line(&mut fragments, &mut generated, &self.cur_atoms, |f| {
            frames.row(&settled, f)
        });
        let (baseline, height) = (settled.baseline(), settled.height);
        self.cur_atoms.clear();
        let width = self.cur_line_width;
        let hang = match end {
            LineEnd::Soft => self.cur_hang,
            LineEnd::Forced => self.cur_hang.min(width.saturating_sub(self.line_width())),
        };
        self.cur_line_width = 0;
        self.cur_hang = 0;
        // CSS Text 3 §6: the content placed in its line box — the band,
        // past the `text-indent` — by `text-align`, justified by
        // `text-justify` (`align`). An intrinsic measurement reads widths
        // only: its lines stay at their start.
        let indent = self.cur_indent;
        let geometry = LineGeometry {
            band: self.band,
            indent,
            rtl: self.rtl,
            width,
            hang,
            last: end == LineEnd::Forced,
            has_tab: std::mem::take(&mut self.cur_has_tab),
        };
        let alignment = if self.measuring {
            TextAlignment::default()
        } else {
            self.align
        };
        let width = align::place_line(&mut fragments, &mut generated, geometry, alignment);
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
            indent,
            ends_clamp: false,
        });
        self.line_groups.push(self.cur_group);
        if end == LineEnd::Forced {
            self.cur_group += 1;
        }
        self.cur_indent = self.indent.of_line(false, end == LineEnd::Forced);
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

/// `n` spaces: a tab's rendering, a separator's with its spacing.
pub(super) fn spaces(n: u16) -> std::borrow::Cow<'static, str> {
    const SPACES: &str = "                                                                ";
    match SPACES.get(..usize::from(n)) {
        Some(s) => std::borrow::Cow::Borrowed(s),
        None => std::borrow::Cow::Owned(" ".repeat(usize::from(n))),
    }
}
