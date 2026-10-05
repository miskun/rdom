//! `LinePacker` — the stateful line-packing machine that walks
//! graphemes and emits `InlineFragment`s into `LineBox`es.
//!
//! Internal to `inline/`. The public face is
//! [`compute_inline_layout`](super::compute_inline_layout) in
//! `inline/mod.rs`.
//!
//! ## State invariants
//!
//! - `word_buffer` holds graphemes since the last soft wrap opportunity
//!   that lets the line wrap (one under `text-wrap-mode: nowrap` does
//!   not end it: its content is measured whole); it's committed at such
//!   an opportunity (white space, hyphen, CJK boundary), an atom, a
//!   forced break, or the end.
//! - `pending_space` is true while a collapsed-whitespace separator
//!   is buffered; it emits a " " fragment at the next word commit
//!   unless we hit a wrap (leading whitespace is trimmed). Collapsible
//!   white space under `nowrap` is no opportunity, so it sits inside the
//!   word buffer instead, collapsed to one space.
//! - The current run's CSS Text values (`run`, a `RunStyle`) are the
//!   text's own: each text node is pushed with its parent element's.
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
//! - `mod.rs` — the packer's state, its modes, and the band floats
//!   leave each line.
//! - `intake` — taking text in: the white space processing rules and
//!   the soft wrap opportunities, filling the word buffer.
//! - `fragments` — a placed word's graphemes as fragments, with the
//!   source maps their rendering needs; the hyphen a soft hyphen shows.
//! - `emit` — committing a word to the current line or the next,
//!   fragments, atomic inlines, and settling and breaking lines.

use std::borrow::Cow;

use rdom_core::NodeId;

use super::breaking::BreakClass;
use super::run_style::RunStyle;
use super::transform::CaseContext;
use super::vertical::{AtomAt, AtomRows};
use super::{GeneratedFragment, InlineFragment, LineBox};
use crate::ext::PseudoSlot;
use crate::layout::OverflowWrap;
use crate::render::box_tree::BoxItem;
use crate::render::layout_pass::float::lines::LineExclusions;

mod emit;
mod fragments;
mod intake;

/// One grapheme awaiting commit, with every piece of provenance we
/// need to rebuild a source position later.
pub(super) struct PendingGrapheme<'a> {
    /// Where the grapheme comes from — a DOM text node or a host's
    /// generated content.
    origin: Origin,
    /// Byte offset of this grapheme's start in the source string (the
    /// text node's data, or the pseudo-element's `content`).
    source_offset: usize,
    /// The grapheme's source bytes.
    source_len: usize,
    /// What it renders as: the source grapheme (borrowed from the text
    /// node's data), or what layout made of it.
    text: Cow<'a, str>,
    /// Visible width of `text`.
    width: u16,
    /// What white-space processing made of it.
    kind: GraphemeKind,
    /// `text` is not the source grapheme byte for byte: its fragment
    /// needs a `SourceMap`.
    mapped: bool,
    /// The `overflow-wrap` of its text: whether an otherwise unbreakable
    /// word too long for its line may break after it (CSS Text 3 §5.5).
    split: OverflowWrap,
}

/// What a buffered grapheme is to line breaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphemeKind {
    /// Text.
    Text,
    /// A collapsed space with no soft wrap opportunity after it (its text
    /// is `nowrap`): removed at a line's start or end, one space between
    /// words (CSS Text 3 §4.1.1, §4.1.2). `segment_break` as
    /// `WhiteSpaceClass::Collapsible`'s.
    Collapsible { segment_break: bool },
    /// A preserved space or tab; `hangs` when it hangs at the end of a
    /// line (CSS Text 3 §4.1.2, `RunStyle::hangs_spaces`); a tab holds its
    /// text's tab size in cells (§4.2).
    Preserved { hangs: bool, tab: Option<u16> },
    /// A soft hyphen (CSS Text 3 §6.1): nothing, or — `shows`, its text's
    /// `hyphens` not `none` — a hyphen when the line breaks after it.
    SoftHyphen { shows: bool },
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

/// Why a line ends — what decides how its trailing preserved spaces
/// hang (CSS Text 3 §4.1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LineEnd {
    /// At a soft wrap opportunity: hanging spaces hang unconditionally.
    Soft,
    /// At a forced break or the end of the content: they hang only
    /// where they overflow (conditionally).
    Forced,
}

pub(super) struct LinePacker<'a> {
    content_width: u16,
    /// The CSS Text values of the text being pushed.
    run: RunStyle,

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
    /// The cells of preserved spaces ending the current line that hang
    /// (CSS Text 3 §4.1.2), part of `cur_line_width`.
    cur_hang: u16,
    /// The top row of the current line: the rows of the lines above.
    cur_top: u16,

    /// Accumulated since the last wrapping soft wrap opportunity — not
    /// yet committed to the current line.
    word_buffer: Vec<PendingGrapheme<'a>>,
    word_width: u16,
    /// The line may wrap before the word buffer's content: false when it
    /// follows an atom in `nowrap` text.
    buffer_wraps: bool,

    /// A collapsed whitespace is buffered between the last committed
    /// content and the pending word. Emit a single space before the
    /// word when we commit (if the word stays on the same line).
    pending_space: bool,
    /// The pending separator holds a segment break, which the segment
    /// break transformation rules may remove (CSS Text 3 §4.1.3).
    pending_segment_break: bool,

    /// Source provenance of the whitespace that produced
    /// `pending_space`. Used as the separator fragment's
    /// owner/text_node/offset, so a click on the space between "a"
    /// and "<b>bold</b>" routes to the enclosing `<p>` (the
    /// whitespace's text-node parent) rather than to `<b>`.
    pending_space_source: Option<(Origin, usize)>,

    /// The last character of text taken in: the context the segment
    /// break transformation rules read.
    last_char: Option<char>,
    /// Whether the text last taken in wraps (`text-wrap-mode`), which
    /// governs the soft wrap opportunity after it.
    last_wraps: bool,
    /// The line breaking class of the text last taken in, when no white
    /// space, forced break or atom came since: the context of the next
    /// grapheme's soft wrap opportunity (`breaking`).
    last_class: Option<BreakClass>,
    /// The current line's content ends with a soft hyphen that shows a
    /// hyphen if the line breaks there (CSS Text 3 §6.1).
    cur_ends_in_shy: bool,
    /// The context `text-transform`'s case mapping reads (`transform`).
    case_ctx: CaseContext,
    /// An atom was the last thing placed.
    after_atom: bool,

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
    pub(super) fn new(content_width: u16) -> Self {
        Self {
            content_width,
            run: RunStyle::default(),
            lines: Vec::new(),
            cur_fragments: Vec::new(),
            cur_generated: Vec::new(),
            cur_atoms: Vec::new(),
            cur_line_width: 0,
            cur_hang: 0,
            cur_top: 0,
            word_buffer: Vec::new(),
            word_width: 0,
            buffer_wraps: true,
            pending_space: false,
            pending_segment_break: false,
            pending_space_source: None,
            last_char: None,
            last_wraps: true,
            last_class: None,
            cur_ends_in_shy: false,
            case_ctx: CaseContext::default(),
            after_atom: false,
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

    /// Cells from the block's starting content edge — the left one, the
    /// right one under `rtl` — to the current line's start: the floats'
    /// share of that side.
    pub(super) fn line_origin(&self) -> u16 {
        let (start, width) = self.band;
        let from_start = if self.rtl {
            i32::from(self.content_width) - (start + i32::from(width))
        } else {
            start
        };
        from_start.clamp(0, i32::from(u16::MAX)) as u16
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
            // The word's tabs count from the moved line's start (CSS
            // Text 3 §4.2).
            self.layout_tabs(0);
        }
    }

    /// A packer for an intrinsic inline size (CSS Sizing 3 §5.1): the
    /// lines' widths are all that is read, so an atom's rows are not
    /// measured, and its containing block's width — the size being
    /// computed — is a cyclic percentage basis, 0 (§5.2.1).
    pub(super) fn measuring(content_width: u16) -> Self {
        Self {
            measuring: true,
            ..Self::new(content_width)
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
}
