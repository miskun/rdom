//! The packer's intake: text, generated content and hard breaks,
//! grapheme by grapheme, through the white space processing rules (CSS
//! Text 3 §4.1, `white_space`) and the soft wrap opportunities
//! (`breaking`), into the word buffer that `emit` commits.

use std::borrow::Cow;

use rdom_core::NodeId;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::super::breaking::{self, BreakClass};
use super::super::run_style::RunStyle;
use super::super::white_space::{self, WhiteSpaceClass};
use super::{GraphemeKind, LineEnd, LinePacker, Origin, PendingGrapheme};
use crate::ext::PseudoSlot;
use crate::layout::{OverflowWrap, WhiteSpaceCollapse};

impl<'a> LinePacker<'a> {
    /// Feed a whole text-node's string in one shot, styled `run` (its
    /// parent element's CSS Text values). Walks graphemes with
    /// byte-precise source tracking.
    pub(in crate::render::inline) fn push_text(
        &mut self,
        owner: NodeId,
        text_node: NodeId,
        text: &'a str,
        run: RunStyle,
    ) {
        self.run = run;
        self.push_str(Origin::text(owner, text_node), text);
    }

    /// Feed a host's static `::before` / `::after` content (CSS 2.1
    /// §12.1: an inline box, the host's first / last child), styled `run`
    /// (the pseudo-element's). It packs, collapses and wraps like text,
    /// but lands in [`LineBox::generated`](super::LineBox::generated).
    pub(in crate::render::inline) fn push_generated(
        &mut self,
        host: NodeId,
        slot: PseudoSlot,
        text: &'a str,
        run: RunStyle,
    ) {
        let origin = Origin {
            owner: host,
            text_node: host,
            generated: Some(slot),
        };
        self.run = run;
        self.push_str(origin, text);
    }

    fn push_str(&mut self, origin: Origin, text: &'a str) {
        let mut source_offset = 0usize;
        for g in text.graphemes(true) {
            self.push_grapheme(origin, source_offset, g);
            source_offset += g.len();
        }
    }

    /// Force a line break. Pushes any pending word and breaks the
    /// current line. Used by `<br>` and by a preserved segment break
    /// (CSS Text 3 §4.1.3).
    pub(in crate::render::inline) fn push_hard_break(&mut self, _owner: NodeId) {
        self.commit_word();
        // A collapsible space before a forced break is removed (CSS Text
        // 3 §4.1.1 step 1, §4.1.2).
        self.clear_pending_space();
        self.last_class = None;
        // Always emit a line — even an empty current line becomes a
        // blank row. Matches `<p>a<br><br>b</p>` producing three
        // rows ("a", blank, "b").
        self.break_line(LineEnd::Forced);
    }

    /// A soft wrap opportunity with no character of its own — HTML's
    /// `<wbr>` ("a line break opportunity"): the line may wrap here when
    /// the text before it wraps.
    pub(in crate::render::inline) fn push_break_opportunity(&mut self) {
        if self.last_wraps {
            self.commit_word();
        }
        self.last_class = None;
    }

    /// Take one grapheme in, per the white space processing rules of its
    /// run (CSS Text 3 §4.1.1, `white_space::classify`).
    fn push_grapheme(&mut self, origin: Origin, source_offset: usize, g: &'a str) {
        match white_space::classify(g, self.run.collapse) {
            WhiteSpaceClass::Control => {}
            WhiteSpaceClass::ForcedBreak => self.push_hard_break(origin.owner),
            WhiteSpaceClass::Collapsible { segment_break } => {
                self.push_collapsible(origin, source_offset, segment_break);
                self.last_class = None;
            }
            class @ (WhiteSpaceClass::PreservedSpace | WhiteSpaceClass::PreservedTab) => {
                // A carriage return or a segment break converted to a
                // space is a space (CSS Text 3 §4, Text 4 §4.1); a tab is
                // one cell until its line places it at its tab stop
                // (§4.2, `emit::layout_tabs`).
                let text: &'a str = if g == " " { g } else { " " };
                let tab = (class == WhiteSpaceClass::PreservedTab).then_some(self.run.tab_size);
                let kind = GraphemeKind::Preserved {
                    hangs: self.run.hangs_spaces(),
                    tab,
                };
                let piece = self.piece(origin, source_offset, g, Cow::Borrowed(text), 1, kind);
                self.push_to_word(piece);
                self.last_class = None;
                // `break-spaces`: a soft wrap opportunity after every
                // preserved white space character (CSS Text 3 §3).
                if self.run.collapse == WhiteSpaceCollapse::BreakSpaces && self.run.wraps {
                    self.commit_word();
                }
            }
            WhiteSpaceClass::Text => self.push_text_grapheme(origin, source_offset, g),
        }
    }

    /// A collapsible space, tab or segment break (CSS Text 3 §4.1.1): a
    /// soft wrap opportunity when its text wraps — the pending separator
    /// — else one space inside the word, either way collapsed with its
    /// neighbours.
    fn push_collapsible(&mut self, origin: Origin, source_offset: usize, segment_break: bool) {
        if self.run.wraps {
            self.commit_word();
            if self.emitted_any {
                if !self.pending_space {
                    self.pending_space = true;
                    self.pending_space_source = Some((origin, source_offset));
                }
                self.pending_segment_break |= segment_break;
            }
            self.last_wraps = true;
            return;
        }
        match self.word_buffer.last_mut() {
            Some(PendingGrapheme {
                kind: GraphemeKind::Collapsible { segment_break: sb },
                ..
            }) => *sb |= segment_break,
            None if self.pending_space => self.pending_segment_break |= segment_break,
            None if !self.emitted_any => {}
            _ => {
                let kind = GraphemeKind::Collapsible { segment_break };
                let piece = self.piece(origin, source_offset, " ", Cow::Borrowed(" "), 1, kind);
                self.push_to_word(piece);
            }
        }
        self.last_wraps = false;
    }

    /// A grapheme of text: the separator before it settled by the segment
    /// break transformation rules (CSS Text 3 §4.1.3), the soft wrap
    /// opportunity before it taken (`breaking`). A zero-width space or a
    /// word joiner is only an opportunity or its absence; a soft hyphen
    /// is kept, zero cells wide, to show a hyphen if the line breaks
    /// after it (§6.1).
    fn push_text_grapheme(&mut self, origin: Origin, source_offset: usize, g: &'a str) {
        let w = UnicodeWidthStr::width(g) as u16;
        let first = g.chars().next().unwrap_or(' ');
        let class = breaking::class_of(first, w == 2);
        if w == 0 {
            match class {
                BreakClass::ZeroWidthSpace | BreakClass::Glue => {
                    self.take_opportunity(class);
                    self.last_class = Some(class);
                }
                BreakClass::SoftHyphen => {
                    self.take_opportunity(class);
                    let kind = GraphemeKind::SoftHyphen {
                        shows: self.run.breaks.hyphens != crate::layout::Hyphens::None,
                    };
                    let piece = self.piece(origin, source_offset, g, Cow::Borrowed(""), 0, kind);
                    self.push_to_word(piece);
                    self.last_class = Some(class);
                }
                // Combining marks / ZWJ fragments we don't handle
                // standalone.
                _ => {}
            }
            return;
        }
        self.transform_segment_break(first);
        // `pre-wrap` / `preserve-spaces`: a soft wrap opportunity at the
        // end of a sequence of preserved spaces (CSS Text 3 §4.1.1).
        if self.last_wraps
            && matches!(
                self.word_buffer.last(),
                Some(PendingGrapheme {
                    kind: GraphemeKind::Preserved { .. },
                    ..
                })
            )
        {
            self.commit_word();
        }
        self.take_opportunity(class);
        let piece = self.piece(
            origin,
            source_offset,
            g,
            Cow::Borrowed(g),
            w,
            GraphemeKind::Text,
        );
        self.push_to_word(piece);
        self.last_char = g.chars().last();
        self.last_class = Some(class);
        self.last_wraps = self.run.wraps;
    }

    /// Commit the word before a grapheme of class `next` when the line
    /// may break between the last text taken in and it (CSS Text 3 §5,
    /// `breaking::break_between`) and the text wraps.
    fn take_opportunity(&mut self, next: BreakClass) {
        if let Some(before) = self.last_class
            && self.run.wraps
            && !self.word_buffer.is_empty()
            && breaking::break_between(before, next, self.run.breaks)
        {
            self.commit_word();
        }
    }

    /// A grapheme `g` at `source_offset` of `origin`, rendered as `text`
    /// `width` cells wide: its piece for the word buffer, with the
    /// `overflow-wrap` of its run (none in text that does not wrap, CSS
    /// Text 3 §5.5: "only has an effect when white-space allows
    /// wrapping").
    fn piece(
        &self,
        origin: Origin,
        source_offset: usize,
        g: &str,
        text: Cow<'a, str>,
        width: u16,
        kind: GraphemeKind,
    ) -> PendingGrapheme<'a> {
        let mapped =
            text.len() != g.len() || (text.as_ref() != g && text.graphemes(true).count() != 1);
        PendingGrapheme {
            origin,
            source_offset,
            source_len: g.len(),
            text,
            width,
            kind,
            mapped,
            split: if self.run.wraps {
                self.run.overflow_wrap
            } else {
                OverflowWrap::Normal
            },
        }
    }

    /// The segment break transformation rules (CSS Text 3 §4.1.3): a
    /// collapsed separator made of a segment break, between `last_char`
    /// and `next`, is removed where `white_space::segment_break_removed`
    /// says so.
    fn transform_segment_break(&mut self, next: char) {
        let removed = |before: Option<char>| {
            before.is_some_and(|b| white_space::segment_break_removed(b, next))
        };
        if self.pending_space && self.pending_segment_break && removed(self.last_char) {
            self.clear_pending_space();
        }
        if let Some(PendingGrapheme {
            kind: GraphemeKind::Collapsible {
                segment_break: true,
            },
            ..
        }) = self.word_buffer.last()
            && removed(self.last_char)
        {
            self.word_buffer.pop();
            self.word_width = self.word_width.saturating_sub(1);
        }
    }

    /// Append a piece to the word buffer. The first one records whether
    /// the line may wrap before the word: always after a soft wrap
    /// opportunity, after an atom only when the text wraps.
    fn push_to_word(&mut self, piece: PendingGrapheme<'a>) {
        if self.word_buffer.is_empty() {
            self.buffer_wraps = !self.after_atom || self.pending_space || self.run.wraps;
            self.after_atom = false;
        }
        if piece.kind != GraphemeKind::Text {
            self.last_wraps = self.run.wraps;
        }
        self.word_width = self.word_width.saturating_add(piece.width);
        self.word_buffer.push(piece);
    }

    /// Drop the pending collapsed separator.
    pub(super) fn clear_pending_space(&mut self) {
        self.pending_space = false;
        self.pending_segment_break = false;
        self.pending_space_source = None;
    }
}
