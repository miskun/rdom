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
//!
//! Graphemes borrow the text node's data (the packer carries the DOM
//! borrow's lifetime); the only allocation per run is one `String`
//! per emitted fragment (`PACKER-STRING-ALLOC-1`).
//!
//! ## Module layout
//!
//! - `mod.rs` — the packer's state and its intake: text, generated
//!   content and hard breaks, grapheme by grapheme, with the
//!   `white-space` and break-opportunity rules.
//! - `emit` — committing a word to the current line or the next,
//!   fragments, atomic inlines, and settling and breaking lines.

use rdom_core::NodeId;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::layout::WhiteSpace;

use super::vertical::{AtomAt, AtomRows};
use super::{GeneratedFragment, InlineFragment, LineBox};
use crate::ext::PseudoSlot;
use crate::render::box_tree::BoxItem;
use crate::render::layout_pass::float::lines::LineExclusions;

mod emit;

/// One grapheme awaiting commit, with every piece of provenance we
/// need to rebuild a source position later.
pub(super) struct PendingGrapheme<'a> {
    /// Where the grapheme comes from — a DOM text node or a host's
    /// generated content.
    origin: Origin,
    /// Byte offset of this grapheme's start in the source string (the
    /// text node's data, or the pseudo-element's `content`).
    source_offset: usize,
    /// The grapheme, borrowed from the source text node's data.
    text: &'a str,
    /// Visible width of the grapheme.
    width: u16,
}

/// Provenance of a run of graphemes. Consecutive graphemes with the
/// same origin (and contiguous offsets) merge into one fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Origin {
    /// Direct element parent of the source text node; the host element
    /// for generated content.
    owner: NodeId,
    /// The source text node; the host element for generated content
    /// (which has no node of its own).
    text_node: NodeId,
    /// `Some(slot)` for a host's `::before` / `::after` content. Such
    /// graphemes become [`GeneratedFragment`]s, never
    /// [`InlineFragment`]s: they take part in line packing but have no
    /// DOM position.
    generated: Option<PseudoSlot>,
}

impl Origin {
    fn text(owner: NodeId, text_node: NodeId) -> Self {
        Origin {
            owner,
            text_node,
            generated: None,
        }
    }
}

pub(super) struct LinePacker<'a> {
    content_width: u16,
    ws: WhiteSpace,

    lines: Vec<LineBox>,

    /// Committed content on the current (still-accumulating) line.
    cur_fragments: Vec<InlineFragment>,
    /// Committed generated content on the current line.
    cur_generated: Vec<GeneratedFragment>,
    /// The atoms on the current line: where they are (an element's in
    /// `cur_fragments`, a pseudo-element's in `cur_generated`) and their
    /// rows (`vertical`).
    cur_atoms: Vec<(AtomAt, AtomRows)>,
    cur_line_width: u16,
    /// The top row of the current line: the rows of the lines above.
    cur_top: u16,

    /// Accumulated since the last break opportunity — not yet
    /// committed to the current line.
    word_buffer: Vec<PendingGrapheme<'a>>,
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
    pending_space_source: Option<(Origin, usize)>,

    /// Whether any visible grapheme has been emitted yet in this IFC.
    /// False = at IFC start; suppresses leading whitespace.
    emitted_any: bool,

    /// The lines are packed for an intrinsic width only
    /// ([`Self::measuring`]).
    measuring: bool,

    /// The floats beside the lines (CSS 2.1 §9.5), when the context has
    /// any to consult: the band each line may use, and where a float met
    /// in the content goes.
    exclusions: Option<&'a mut dyn LineExclusions>,
    /// The current line's band: its start column and width — the whole
    /// content box unless floats shorten it.
    band: (i32, u16),
    /// Floats met on a line with no room left for them, placed at the
    /// next line's top.
    pending_floats: Vec<BoxItem>,
    /// Lines start at the right (inline-start) edge (`direction: rtl`).
    rtl: bool,
}

impl<'a> LinePacker<'a> {
    pub(super) fn new(content_width: u16, ws: WhiteSpace) -> Self {
        Self {
            content_width,
            ws,
            lines: Vec::new(),
            cur_fragments: Vec::new(),
            cur_generated: Vec::new(),
            cur_atoms: Vec::new(),
            cur_line_width: 0,
            cur_top: 0,
            word_buffer: Vec::new(),
            word_width: 0,
            pending_space: false,
            pending_space_source: None,
            emitted_any: false,
            measuring: false,
            exclusions: None,
            band: (0, content_width),
            pending_floats: Vec::new(),
            rtl: false,
        }
    }

    /// Start each line at the right edge of its band — the inline-start
    /// edge under `direction: rtl` (CSS Writing Modes 4 §2.1, CSS Text 3
    /// §7.1's `start`) — a line wider than it overflowing the left edge.
    pub(super) fn starting_right(mut self, rtl: bool) -> Self {
        self.rtl = rtl;
        self
    }

    /// Pack the lines around the floats `exclusions` describes (CSS 2.1
    /// §9.5).
    pub(super) fn around(mut self, exclusions: &'a mut dyn LineExclusions) -> Self {
        self.exclusions = Some(exclusions);
        self.open_line();
        self
    }

    /// The current line's width: its band's.
    pub(super) fn line_width(&self) -> u16 {
        self.band.1
    }

    /// A new line starts at row `cur_top`: the floats waiting for it are
    /// placed at its top, and its band read.
    pub(super) fn open_line(&mut self) {
        let Some(ex) = self.exclusions.as_deref_mut() else {
            return;
        };
        for item in std::mem::take(&mut self.pending_floats) {
            ex.place_float(item, self.cur_top, None);
        }
        self.band = ex.band(self.cur_top);
    }

    /// A float met in the inline content (CSS 2.1 §9.5.1): placed on the
    /// current line when it fits beside what the line holds — the band
    /// narrows, and the line's content shifts past a left float when the
    /// line is settled — else at the next line's top. Without exclusions
    /// (an intrinsic measurement) it is not packed.
    pub(in crate::render::inline) fn push_float(&mut self, item: BoxItem) {
        let used = self.line_has_content().then_some(self.cur_line_width);
        let Some(ex) = self.exclusions.as_deref_mut() else {
            return;
        };
        if ex.place_float(item, self.cur_top, used) {
            self.band = ex.band(self.cur_top);
        } else {
            self.pending_floats.push(item);
        }
    }

    /// An empty line too narrow for `width` cells beside the floats moves
    /// down past them until it is wide enough or no float shortens it
    /// (CSS 2.1 §9.5).
    pub(super) fn fit_empty_line(&mut self, width: u16) {
        if self.line_has_content() {
            return;
        }
        while width > self.band.1 && self.band.1 < self.content_width {
            let Some(ex) = self.exclusions.as_deref_mut() else {
                return;
            };
            let Some(next) = ex.next_change(self.cur_top) else {
                return;
            };
            self.cur_top = next;
            self.band = ex.band(next);
        }
    }

    /// A packer for an intrinsic inline size (CSS Sizing 3 §5.1): the
    /// lines' widths are all that is read, so an atom's rows are not
    /// measured, and its containing block's width — the size being
    /// computed — is a cyclic percentage basis, 0 (§5.2.1).
    pub(super) fn measuring(content_width: u16, ws: WhiteSpace) -> Self {
        Self {
            measuring: true,
            ..Self::new(content_width, ws)
        }
    }

    /// Whether the lines are packed for an intrinsic width only.
    pub(super) fn is_measuring(&self) -> bool {
        self.measuring
    }

    /// The content width the lines are packed against — the IFC
    /// block's content width, the containing block of its atoms.
    pub(super) fn content_width(&self) -> u16 {
        self.content_width
    }

    pub(super) fn take_lines(&mut self) -> Vec<LineBox> {
        std::mem::take(&mut self.lines)
    }

    /// Feed a whole text-node's string in one shot. Walks graphemes
    /// with byte-precise source tracking.
    pub(super) fn push_text(&mut self, owner: NodeId, text_node: NodeId, text: &'a str) {
        self.push_str(Origin::text(owner, text_node), text);
    }

    /// Feed a host's static `::before` / `::after` content (CSS 2.1
    /// §12.1: an inline box, the host's first / last child). It packs,
    /// collapses and wraps like text, but lands in
    /// [`LineBox::generated`](super::LineBox::generated).
    pub(super) fn push_generated(&mut self, host: NodeId, slot: PseudoSlot, text: &'a str) {
        let origin = Origin {
            owner: host,
            text_node: host,
            generated: Some(slot),
        };
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

    fn push_grapheme(&mut self, origin: Origin, source_offset: usize, g: &'a str) {
        let first = g.chars().next().unwrap_or(' ');

        // Control characters require per-mode handling.
        if first.is_control() {
            match self.ws {
                // Pre and PreWrap both preserve newlines as hard
                // breaks and convert tabs to a single space; only
                // their wrap behavior on regular whitespace differs.
                WhiteSpace::Pre | WhiteSpace::PreWrap => match g {
                    "\n" | "\r\n" => {
                        self.push_hard_break(origin.owner);
                        return;
                    }
                    "\r" => return,
                    "\t" => {
                        // Tab → single space (tab-stop columns are a
                        // separate feature).
                        self.word_buffer.push(PendingGrapheme {
                            origin,
                            source_offset,
                            text: " ",
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
                        self.pending_space_source = Some((origin, source_offset));
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
                    origin,
                    source_offset,
                    text: g,
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
                        origin,
                        source_offset,
                        text: " ",
                        width: 1,
                    });
                    self.word_width = self.word_width.saturating_add(1);
                    self.commit_word();
                } else if w == 2 {
                    if !self.word_buffer.is_empty() {
                        self.commit_word();
                    }
                    self.word_buffer.push(PendingGrapheme {
                        origin,
                        source_offset,
                        text: g,
                        width: w,
                    });
                    self.word_width = self.word_width.saturating_add(w);
                    self.commit_word();
                } else {
                    self.word_buffer.push(PendingGrapheme {
                        origin,
                        source_offset,
                        text: g,
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
                        self.pending_space_source = Some((origin, source_offset));
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
                        origin,
                        source_offset,
                        text: g,
                        width: w,
                    });
                    self.word_width = self.word_width.saturating_add(w);
                    self.commit_word();
                    return;
                }

                // Hyphen: break-after.
                self.word_buffer.push(PendingGrapheme {
                    origin,
                    source_offset,
                    text: g,
                    width: w,
                });
                self.word_width = self.word_width.saturating_add(w);
                if g == "-" {
                    self.commit_word();
                }
            }
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
