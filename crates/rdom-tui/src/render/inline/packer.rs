//! `LinePacker` — the stateful line-packing machine that walks
//! graphemes and emits `InlineFragment`s into `LineBox`es.
//!
//! Internal to `inline/`. The public face is
//! [`compute_inline_layout`](super::compute_inline_layout) in
//! `inline/mod.rs`.
//!
//! ## State invariants
//!
//! - `word_buffer` holds graphemes since the last break opportunity;
//!   it's committed on whitespace, hyphen, CJK boundary, or end.
//! - `pending_space` is true while a collapsed-whitespace separator
//!   is buffered; it emits a " " fragment at the next word commit
//!   unless we hit a wrap (leading whitespace is trimmed).
//! - `emitted_any` is the global "has any visible grapheme shipped?"
//!   flag — leading whitespace at IFC start is dropped by checking
//!   this.
//!
//! ## Source tracking
//!
//! Every `PendingGrapheme` records the source `text_node` and byte
//! offset in that node's data. Fragments inherit this from their
//! first grapheme, enabling `position_at` in the runtime to map
//! screen cells back to node+offset for selection.

use rdom_core::NodeId;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::layout::WhiteSpace;

use super::{InlineFragment, LineBox};

/// One grapheme awaiting commit, with every piece of provenance we
/// need to rebuild a source position later.
pub(super) struct PendingGrapheme {
    /// Direct element parent of the source text node.
    owner: NodeId,
    /// The source text node itself.
    text_node: NodeId,
    /// Byte offset of this grapheme's start in `text_node`'s data.
    source_offset: usize,
    /// The grapheme string.
    text: String,
    /// Visible width of the grapheme.
    width: u16,
}

pub(super) struct LinePacker {
    content_width: u16,
    ws: WhiteSpace,

    lines: Vec<LineBox>,

    /// Committed content on the current (still-accumulating) line.
    cur_fragments: Vec<InlineFragment>,
    cur_line_width: u16,

    /// Accumulated since the last break opportunity — not yet
    /// committed to the current line.
    word_buffer: Vec<PendingGrapheme>,
    word_width: u16,

    /// A collapsed whitespace is buffered between the last committed
    /// content and the pending word. Emit a single space before the
    /// word when we commit (if the word stays on the same line).
    pending_space: bool,

    /// Source provenance of the whitespace that produced
    /// `pending_space`. Used as the separator fragment's
    /// owner/text_node/offset, so a click on the space between "a"
    /// and "<b>bold</b>" routes to the enclosing `<p>` (the
    /// whitespace's text-node parent) rather than to `<b>`.
    pending_space_source: Option<(NodeId, NodeId, usize)>,

    /// Whether any visible grapheme has been emitted yet in this IFC.
    /// False = at IFC start; suppresses leading whitespace.
    emitted_any: bool,
}

impl LinePacker {
    pub(super) fn new(content_width: u16, ws: WhiteSpace) -> Self {
        Self {
            content_width,
            ws,
            lines: Vec::new(),
            cur_fragments: Vec::new(),
            cur_line_width: 0,
            word_buffer: Vec::new(),
            word_width: 0,
            pending_space: false,
            pending_space_source: None,
            emitted_any: false,
        }
    }

    pub(super) fn take_lines(&mut self) -> Vec<LineBox> {
        std::mem::take(&mut self.lines)
    }

    /// Feed a whole text-node's string in one shot. Walks graphemes
    /// with byte-precise source tracking.
    pub(super) fn push_text(&mut self, owner: NodeId, text_node: NodeId, text: &str) {
        let mut source_offset = 0usize;
        for g in text.graphemes(true) {
            self.push_grapheme(owner, text_node, source_offset, g);
            source_offset += g.len();
        }
    }

    /// Force a line break. Pushes any pending word and breaks the
    /// current line. Used by `<br>` and by `\n` under
    /// `WhiteSpace::Pre`.
    pub(super) fn push_hard_break(&mut self, _owner: NodeId) {
        if !self.word_buffer.is_empty() {
            self.commit_word();
        }
        self.pending_space = false;
        self.pending_space_source = None;
        // Always emit a line — even an empty current line becomes a
        // blank row. Matches `<p>a<br><br>b</p>` producing three
        // rows ("a", blank, "b").
        self.break_line();
    }

    fn push_grapheme(&mut self, owner: NodeId, text_node: NodeId, source_offset: usize, g: &str) {
        let first = g.chars().next().unwrap_or(' ');

        // Control characters require per-mode handling.
        if first.is_control() {
            match self.ws {
                // Pre and PreWrap both preserve newlines as hard
                // breaks and convert tabs to a single space; only
                // their wrap behavior on regular whitespace differs.
                WhiteSpace::Pre | WhiteSpace::PreWrap => match g {
                    "\n" | "\r\n" => {
                        self.push_hard_break(owner);
                        return;
                    }
                    "\r" => return,
                    "\t" => {
                        // Tab → single space (tab-stop columns are a
                        // separate feature).
                        self.word_buffer.push(PendingGrapheme {
                            owner,
                            text_node,
                            source_offset,
                            text: " ".to_string(),
                            width: 1,
                        });
                        self.word_width = self.word_width.saturating_add(1);
                        return;
                    }
                    _ => return,
                },
                _ => {
                    // Normal / NoWrap: collapse to a single space
                    // separator (same as any ASCII whitespace).
                    if !self.word_buffer.is_empty() {
                        self.commit_word();
                    }
                    if self.emitted_any {
                        self.pending_space = true;
                        self.pending_space_source = Some((owner, text_node, source_offset));
                    }
                    return;
                }
            }
        }

        let w = UnicodeWidthStr::width(g) as u16;
        if w == 0 {
            // Combining marks / ZWJ fragments we don't handle standalone.
            return;
        }

        match self.ws {
            WhiteSpace::Pre => {
                // Verbatim: preserve spaces/tabs. No soft-break
                // opportunities.
                self.word_buffer.push(PendingGrapheme {
                    owner,
                    text_node,
                    source_offset,
                    text: g.to_string(),
                    width: w,
                });
                self.word_width = self.word_width.saturating_add(w);
            }
            WhiteSpace::PreWrap => {
                // Preserve whitespace verbatim (like Pre) AND create a
                // soft-break opportunity at each ASCII space (like
                // Normal). CJK width-2 graphemes also break either
                // side (same as Normal). Hyphens break-after.
                if g == " " {
                    if !self.word_buffer.is_empty() {
                        self.commit_word();
                    }
                    self.word_buffer.push(PendingGrapheme {
                        owner,
                        text_node,
                        source_offset,
                        text: " ".to_string(),
                        width: 1,
                    });
                    self.word_width = self.word_width.saturating_add(1);
                    self.commit_word();
                } else if w == 2 {
                    if !self.word_buffer.is_empty() {
                        self.commit_word();
                    }
                    self.word_buffer.push(PendingGrapheme {
                        owner,
                        text_node,
                        source_offset,
                        text: g.to_string(),
                        width: w,
                    });
                    self.word_width = self.word_width.saturating_add(w);
                    self.commit_word();
                } else {
                    self.word_buffer.push(PendingGrapheme {
                        owner,
                        text_node,
                        source_offset,
                        text: g.to_string(),
                        width: w,
                    });
                    self.word_width = self.word_width.saturating_add(w);
                    if g == "-" {
                        self.commit_word();
                    }
                }
            }
            WhiteSpace::Normal | WhiteSpace::NoWrap => {
                if is_collapsible_whitespace(g) {
                    // Break boundary: commit the pending word; mark a
                    // pending space so the NEXT word emits a leading
                    // space (unless we're at IFC start).
                    if !self.word_buffer.is_empty() {
                        self.commit_word();
                    }
                    if self.emitted_any {
                        self.pending_space = true;
                        self.pending_space_source = Some((owner, text_node, source_offset));
                    }
                    return;
                }

                // CJK (width-2) graphemes: each is its own "word"
                // with break opportunities on both sides.
                if w == 2 {
                    if !self.word_buffer.is_empty() {
                        self.commit_word();
                    }
                    self.word_buffer.push(PendingGrapheme {
                        owner,
                        text_node,
                        source_offset,
                        text: g.to_string(),
                        width: w,
                    });
                    self.word_width = self.word_width.saturating_add(w);
                    self.commit_word();
                    return;
                }

                // Hyphen: break-after.
                self.word_buffer.push(PendingGrapheme {
                    owner,
                    text_node,
                    source_offset,
                    text: g.to_string(),
                    width: w,
                });
                self.word_width = self.word_width.saturating_add(w);
                if g == "-" {
                    self.commit_word();
                }
            }
        }
    }

    /// Commit the word buffer to the current line (or wrap to a new
    /// line if it doesn't fit). Clears the buffer.
    fn commit_word(&mut self) {
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

        if must_wrap && !self.cur_fragments.is_empty() {
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

    fn emit_word_to_current_line(&mut self, separator_width: u16) {
        // Emit the separator space (if any) with the provenance of
        // the whitespace that produced it.
        if separator_width > 0 && !self.word_buffer.is_empty() {
            let (sep_owner, sep_text_node, sep_source_offset) =
                self.pending_space_source.unwrap_or_else(|| {
                    let g = &self.word_buffer[0];
                    (g.owner, g.text_node, g.source_offset)
                });
            self.append_fragment(sep_owner, sep_text_node, sep_source_offset, " ", 1);
        }

        // Group consecutive same-(owner, text_node) graphemes into
        // fragments. A change in either starts a new fragment.
        let mut idx = 0;
        while idx < self.word_buffer.len() {
            let g0 = &self.word_buffer[idx];
            let owner = g0.owner;
            let text_node = g0.text_node;
            let source_offset = g0.source_offset;
            let mut text = String::new();
            let mut width: u16 = 0;
            while idx < self.word_buffer.len() {
                let g = &self.word_buffer[idx];
                if g.owner != owner || g.text_node != text_node {
                    break;
                }
                text.push_str(&g.text);
                width = width.saturating_add(g.width);
                idx += 1;
            }
            self.append_fragment(owner, text_node, source_offset, &text, width);
        }

        self.word_buffer.clear();
        self.word_width = 0;
        self.emitted_any = true;
    }

    /// Append a fragment to the current line. Merges with the
    /// previous fragment when its (owner, text_node) match AND the
    /// byte ranges are contiguous — keeps fragment counts low and
    /// preserves correct source mapping.
    fn append_fragment(
        &mut self,
        owner: NodeId,
        text_node: NodeId,
        source_offset: usize,
        text: &str,
        width: u16,
    ) {
        if let Some(last) = self.cur_fragments.last_mut() {
            let contiguous = last.source_byte_offset + last.text.len() == source_offset;
            if last.node == owner && last.text_node == text_node && contiguous {
                last.text.push_str(text);
                last.width = last.width.saturating_add(width);
                self.cur_line_width = self.cur_line_width.saturating_add(width);
                return;
            }
        }
        let x = self.cur_line_width;
        self.cur_fragments.push(InlineFragment {
            node: owner,
            text_node,
            source_byte_offset: source_offset,
            x,
            width,
            text: text.to_string(),
            atomic: false,
        });
        self.cur_line_width = self.cur_line_width.saturating_add(width);
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
    pub(super) fn push_atomic_inline_block(&mut self, node: NodeId, width: u16) {
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
            let (sep_owner, sep_text_node, sep_offset) =
                self.pending_space_source.unwrap_or((node, node, 0));
            self.append_fragment(sep_owner, sep_text_node, sep_offset, " ", 1);
            self.pending_space = false;
            self.pending_space_source = None;
        }
        let x = self.cur_line_width;
        self.cur_fragments.push(InlineFragment {
            node,
            text_node: node, // sentinel — atom has no source text node
            source_byte_offset: 0,
            x,
            width,
            text: String::new(),
            atomic: true,
        });
        self.cur_line_width = self.cur_line_width.saturating_add(width);
        // Atoms behave like a committed word — any whitespace that
        // FOLLOWS them must again emit a separator (we just emitted
        // visible content, so `emitted_any` must be true).
        self.emitted_any = true;
    }

    fn break_line(&mut self) {
        let fragments = std::mem::take(&mut self.cur_fragments);
        let width = self.cur_line_width;
        self.cur_line_width = 0;
        self.lines.push(LineBox { fragments, width });
    }

    /// Flush any pending word and the current line. Drops trailing
    /// pending_space (trailing-whitespace trim at IFC end).
    pub(super) fn finish(&mut self) {
        if !self.word_buffer.is_empty() {
            self.commit_word();
        }
        self.pending_space = false;
        self.pending_space_source = None;
        if !self.cur_fragments.is_empty() {
            self.break_line();
        }
    }
}

/// Whitespace characters collapsed under `WhiteSpace::Normal` /
/// `NoWrap`. Matches CSS: ASCII space, tab, LF, CR (plus CRLF as a
/// grapheme cluster). NBSP (U+00A0) is NOT collapsed.
#[inline]
fn is_collapsible_whitespace(grapheme: &str) -> bool {
    matches!(grapheme, " " | "\t" | "\n" | "\r" | "\r\n")
}
