//! The packer's intake: text, generated content and hard breaks,
//! grapheme by grapheme, through the white space processing rules (CSS
//! Text 3 §4.1, `white_space`) and the soft wrap opportunities, into
//! the word buffer that `emit` commits.

use rdom_core::NodeId;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::super::run_style::RunStyle;
use super::super::white_space::{self, WhiteSpaceClass};
use super::{GraphemeKind, LineEnd, LinePacker, Origin, PendingGrapheme};
use crate::ext::PseudoSlot;
use crate::layout::WhiteSpaceCollapse;

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
        // Always emit a line — even an empty current line becomes a
        // blank row. Matches `<p>a<br><br>b</p>` producing three
        // rows ("a", blank, "b").
        self.break_line(LineEnd::Forced);
    }

    /// Take one grapheme in, per the white space processing rules of its
    /// run (CSS Text 3 §4.1.1, `white_space::classify`).
    fn push_grapheme(&mut self, origin: Origin, source_offset: usize, g: &'a str) {
        match white_space::classify(g, self.run.collapse) {
            WhiteSpaceClass::Control => {}
            WhiteSpaceClass::ForcedBreak => self.push_hard_break(origin.owner),
            WhiteSpaceClass::Collapsible { segment_break } => {
                self.push_collapsible(origin, source_offset, segment_break);
            }
            WhiteSpaceClass::PreservedSpace | WhiteSpaceClass::PreservedTab => {
                // A tab advances one cell (tab stops are C9-TAB-SIZE); a
                // carriage return or a segment break converted to a space
                // is a space (CSS Text 3 §4, Text 4 §4.1).
                let text = if g == " " { g } else { " " };
                let kind = GraphemeKind::Preserved {
                    hangs: self.run.hangs_spaces(),
                };
                self.push_to_word(origin, source_offset, text, 1, kind);
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
                self.push_to_word(origin, source_offset, " ", 1, kind);
            }
        }
        self.last_wraps = false;
    }

    /// A grapheme of text: the separator before it settled by the segment
    /// break transformation rules (CSS Text 3 §4.1.3), the soft wrap
    /// opportunities around it taken.
    fn push_text_grapheme(&mut self, origin: Origin, source_offset: usize, g: &'a str) {
        let w = UnicodeWidthStr::width(g) as u16;
        if w == 0 {
            // Combining marks / ZWJ fragments we don't handle standalone.
            return;
        }
        let first = g.chars().next().unwrap_or(' ');
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
        let wraps = self.run.wraps;
        // CJK (width-2) graphemes: each is its own "word" with break
        // opportunities on both sides; a hyphen breaks after.
        if w == 2 && wraps {
            self.commit_word();
        }
        self.push_to_word(origin, source_offset, g, w, GraphemeKind::Text);
        if wraps && (w == 2 || g == "-") {
            self.commit_word();
        }
        self.last_char = g.chars().last();
        self.last_wraps = wraps;
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

    /// Append a grapheme to the word buffer. The first one records
    /// whether the line may wrap before the word: always after a soft
    /// wrap opportunity, after an atom only when the text wraps.
    fn push_to_word(
        &mut self,
        origin: Origin,
        source_offset: usize,
        text: &'a str,
        width: u16,
        kind: GraphemeKind,
    ) {
        if self.word_buffer.is_empty() {
            self.buffer_wraps = !self.after_atom || self.pending_space || self.run.wraps;
            self.after_atom = false;
        }
        self.word_buffer.push(PendingGrapheme {
            origin,
            source_offset,
            text,
            width,
            kind,
        });
        self.word_width = self.word_width.saturating_add(width);
        if kind != GraphemeKind::Text {
            self.last_wraps = self.run.wraps;
        }
    }

    /// Drop the pending collapsed separator.
    pub(super) fn clear_pending_space(&mut self) {
        self.pending_space = false;
        self.pending_segment_break = false;
        self.pending_space_source = None;
    }
}
