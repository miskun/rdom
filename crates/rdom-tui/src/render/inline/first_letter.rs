//! `::first-letter` (CSS Pseudo-Elements 4 §2.3): the first typographic
//! letter unit of a block container's first formatted line — the first
//! letter or number, with the punctuation that precedes and follows it
//! (§2.3.2) — in a `::before`'s generated text or the DOM's, wherever the
//! line starts.
//!
//! The DOM text is never split. The letter is found in the content before
//! packing ([`letter`]): a span of source bytes per text it lies in. The
//! packer (`packer::first`) takes those graphemes in under the letter's
//! style — a fragment of their own, `InlineFragment::first_letter` /
//! `GeneratedFragment::first_letter`, which paint styles ([`effective`])
//! — or, when the letter floats, leaves them out of the line and puts
//! the float there: a generated box (`PseudoSlot::FirstLetter`) whose
//! text is the letter's ([`text`]), laid out and painted as a floated
//! `::before` is.

use rdom_core::{Dom, NodeId, NodeType};
use unicode_segmentation::UnicodeSegmentation;

use super::run_style::RunStyle;
use crate::ext::{PseudoSlot, TuiExt};
use crate::layout::{Display, Float, Position, TextTransform};
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;
use crate::style::ComputedStyle;

/// Where a piece of the letter is: a text node, or a host's generated
/// content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LetterSource {
    Text(NodeId),
    Generated(NodeId, PseudoSlot),
}

/// One piece of the letter: `[start, end)` source bytes of `source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LetterSpan {
    pub(crate) source: LetterSource,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl LetterSpan {
    /// Whether the grapheme at byte `offset` of `source` is in it.
    pub(crate) fn holds(&self, source: LetterSource, offset: usize) -> bool {
        self.source == source && (self.start..self.end).contains(&offset)
    }
}

/// The host whose `::first-letter` styles the first line of `hosts`
/// (innermost first, `first_line::hosts`): the innermost with one.
pub(crate) fn host_of(dom: &Dom<TuiExt>, hosts: &[NodeId]) -> Option<NodeId> {
    hosts.iter().copied().find(|&h| {
        dom.node(h)
            .ext()
            .is_some_and(|e| e.computed_first_letter().is_some())
    })
}

/// The first typographic letter unit of the first line of `block`'s flow
/// (§2.3.2): leading white space skipped, then punctuation, one letter or
/// number, and the punctuation after it — in `block`'s `::before` text,
/// its text and the inline boxes it holds, in order; an atom, a block or
/// a forced break first, or punctuation followed by white space, leaves
/// none. Empty when there is none.
pub(crate) fn letter(dom: &Dom<TuiExt>, block: NodeId) -> Vec<LetterSpan> {
    let mut scan = Scan::default();
    let _ = scan.element(dom, block);
    match scan.state {
        State::Trail | State::Done => scan.spans,
        State::Lead | State::Failed => Vec::new(),
    }
}

/// The letter's text, piece by piece, for its own box (a floated letter's
/// lines): the source slices `spans` name.
pub(crate) fn text<'d>(dom: &'d Dom<TuiExt>, spans: &[LetterSpan]) -> Vec<&'d str> {
    spans
        .iter()
        .filter_map(|s| {
            let source = match s.source {
                LetterSource::Text(id) => dom.node(id).node_value()?,
                LetterSource::Generated(host, slot) => {
                    dom.node(host).computed_pseudo(slot)?.content.as_deref()?
                }
            };
            source.get(s.start..s.end)
        })
        .collect()
}

/// The letter of the first formatted line the `::first-letter` of
/// `host` styles: found in the flow of the block holding that line —
/// `host`, or its first block descendant down the chain of first
/// line-bearing blocks (§2.2).
pub(crate) fn letter_of_host(dom: &Dom<TuiExt>, host: NodeId) -> Vec<LetterSpan> {
    let mut block = host;
    while let Some(BoxItem::Node(child)) = super::generated::line_bearing_child(dom, block, false)
        && dom
            .node(child)
            .computed()
            .is_some_and(|c| c.display == Display::Block)
        && !super::generated::own_line_pseudos(dom, block).before
    {
        block = child;
    }
    letter(dom, block)
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum State {
    /// Before the letter: white space skipped, punctuation collected.
    #[default]
    Lead,
    /// After the letter: punctuation collected.
    Trail,
    /// The unit is complete.
    Done,
    /// The line starts with something that is no letter unit.
    Failed,
}

/// The letter search over the line's content, in order.
#[derive(Default)]
struct Scan {
    state: State,
    spans: Vec<LetterSpan>,
}

/// Whether the search is over.
type Stop = std::ops::ControlFlow<()>;

impl Scan {
    fn over(&self) -> Stop {
        if matches!(self.state, State::Done | State::Failed) {
            Stop::Break(())
        } else {
            Stop::Continue(())
        }
    }

    /// The inline content of the element `id`: its `::before` text, its
    /// children, its `::after` text.
    fn element(&mut self, dom: &Dom<TuiExt>, id: NodeId) -> Stop {
        self.pseudo(dom, id, PseudoSlot::Before)?;
        for child in dom.node(id).child_nodes() {
            match child.node_type() {
                NodeType::Text => {
                    let hidden = crate::render::box_tree::is_hidden_text(dom, id, child.id());
                    if !hidden && let Some(data) = child.node_value() {
                        self.text(LetterSource::Text(child.id()), data)?;
                    }
                }
                NodeType::Element => self.child(dom, child.id())?,
                _ => {}
            }
        }
        self.pseudo(dom, id, PseudoSlot::After)
    }

    /// An element child of the line's content: an inline box searched
    /// through, an out-of-flow or floated one skipped, anything else
    /// (an atom, a block, a forced break) the end of the search.
    fn child(&mut self, dom: &Dom<TuiExt>, id: NodeId) -> Stop {
        let Some(c) = dom.node(id).computed() else {
            return self.over();
        };
        let out_of_flow = c.display == Display::None
            || matches!(c.position, Position::Absolute | Position::Fixed)
            || c.float != Float::None;
        if out_of_flow {
            return Stop::Continue(());
        }
        let inline_box = matches!(c.display, Display::Inline | Display::Contents)
            && !c.is_atomic_inline()
            && dom.node(id).tag_name() != Some("br");
        if !inline_box {
            self.state = match self.state {
                State::Trail => State::Done,
                _ => State::Failed,
            };
            return self.over();
        }
        self.element(dom, id)
    }

    /// `host`'s `slot` pseudo-element, when it is inline text of the line.
    fn pseudo(&mut self, dom: &Dom<TuiExt>, host: NodeId, slot: PseudoSlot) -> Stop {
        match super::generated::inline_pseudo(dom, host, slot.into()) {
            Some(super::generated::InlinePseudo::Text(t)) => {
                self.text(LetterSource::Generated(host, slot), t)
            }
            Some(super::generated::InlinePseudo::Atom) => {
                self.state = match self.state {
                    State::Trail => State::Done,
                    _ => State::Failed,
                };
                self.over()
            }
            _ => self.over(),
        }
    }

    /// The graphemes of `text`, from `source`.
    fn text(&mut self, source: LetterSource, text: &str) -> Stop {
        for (offset, g) in text.grapheme_indices(true) {
            let c = g.chars().next().unwrap_or(' ');
            let take = match self.state {
                State::Lead if c.is_whitespace() && self.spans.is_empty() => false,
                State::Lead if is_punctuation(c) => true,
                State::Lead if c.is_alphanumeric() => {
                    self.state = State::Trail;
                    true
                }
                State::Lead => {
                    self.state = State::Failed;
                    return Stop::Break(());
                }
                State::Trail if is_punctuation(c) => true,
                State::Trail => {
                    self.state = State::Done;
                    return Stop::Break(());
                }
                State::Done | State::Failed => return Stop::Break(()),
            };
            if take {
                let end = offset + g.len();
                match self.spans.last_mut() {
                    Some(s) if s.source == source && s.end == offset => s.end = end,
                    _ => self.spans.push(LetterSpan {
                        source,
                        start: offset,
                        end,
                    }),
                }
            }
        }
        Stop::Continue(())
    }
}

/// Whether `c` is punctuation (Unicode's P* categories, §2.3.2) — ASCII
/// punctuation that is no symbol, the Latin-1 and General Punctuation
/// marks, CJK and full-width punctuation.
fn is_punctuation(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_punctuation()
            && !matches!(c, '$' | '+' | '<' | '=' | '>' | '^' | '`' | '|' | '~');
    }
    matches!(c,
        '\u{00A1}' | '\u{00A7}' | '\u{00AB}' | '\u{00B6}' | '\u{00B7}' | '\u{00BB}' | '\u{00BF}'
        | '\u{2010}'..='\u{2027}'
        | '\u{2030}'..='\u{2043}'
        | '\u{2045}'..='\u{2051}'
        | '\u{2053}'..='\u{205E}'
        | '\u{3001}'..='\u{3003}'
        | '\u{3008}'..='\u{3011}'
        | '\u{3014}'..='\u{301F}'
        | '\u{FF01}'..='\u{FF03}'
        | '\u{FF05}'..='\u{FF0A}'
        | '\u{FF0C}'..='\u{FF0F}'
        | '\u{FF1A}' | '\u{FF1B}' | '\u{FF1F}' | '\u{FF20}'
        | '\u{FF3B}'..='\u{FF3D}'
        | '\u{FF3F}' | '\u{FF5B}' | '\u{FF5D}'
    )
}

/// The letter's `::first-letter` style and the style it inherits from:
/// `host`'s `::first-line`, or `host`.
fn styles(dom: &Dom<TuiExt>, host: NodeId) -> Option<(&ComputedStyle, &ComputedStyle)> {
    let ext = dom.node(host).ext()?;
    let letter = ext.computed_first_letter().map(|s| &**s)?;
    let parent = ext
        .computed_first_line()
        .map(|s| &**s)
        .or(ext.computed.as_deref())?;
    Some((letter, parent))
}

/// Whether the letter `host`'s `::first-letter` styles floats.
pub(crate) fn floats(dom: &Dom<TuiExt>, host: NodeId) -> bool {
    styles(dom, host).is_some_and(|(l, _)| l.float != Float::None)
}

/// What `::first-letter` does to the letter's text-shaping values: its
/// own where its rules set them (§2.3.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LetterRun {
    transform: Option<TextTransform>,
    letter_spacing: Option<u16>,
    word_spacing: Option<u16>,
}

impl LetterRun {
    /// `host`'s letter's mapping; `None` when it shapes nothing.
    pub(crate) fn of(dom: &Dom<TuiExt>, host: NodeId) -> Option<Self> {
        let (letter, parent) = styles(dom, host)?;
        let (l, p) = (RunStyle::of(letter), RunStyle::of(parent));
        let out = LetterRun {
            transform: (l.transform != p.transform).then_some(l.transform),
            letter_spacing: (l.letter_spacing != p.letter_spacing).then_some(l.letter_spacing),
            word_spacing: (l.word_spacing != p.word_spacing).then_some(l.word_spacing),
        };
        (out.transform.is_some() || out.letter_spacing.is_some() || out.word_spacing.is_some())
            .then_some(out)
    }

    /// `run` inside the letter.
    pub(crate) fn apply(&self, mut run: RunStyle) -> RunStyle {
        if let Some(t) = self.transform {
            run.transform = t;
        }
        if let Some(n) = self.letter_spacing {
            run.letter_spacing = n;
        }
        if let Some(n) = self.word_spacing {
            run.word_spacing = n;
        }
        run
    }
}

/// The style the letter's text, in an element styled `own` (already on
/// its first line's style), paints in: `own` with the color, font weight
/// / style, decorations and background `host`'s `::first-letter` sets —
/// the letter's box sits inside the innermost inline box holding it, so
/// its values win. `None` when it changes none of them.
pub(crate) fn effective(
    dom: &Dom<TuiExt>,
    host: NodeId,
    own: &ComputedStyle,
) -> Option<ComputedStyle> {
    use crate::style::Modifier;
    let (letter, parent) = styles(dom, host)?;
    let font = Modifier::BOLD | Modifier::ITALIC;
    let mut out = own.clone();
    if letter.fg != parent.fg {
        out.fg = letter.fg;
    }
    if letter.modifiers & font != parent.modifiers & font {
        out.modifiers = out.modifiers.without(font) | (letter.modifiers & font);
    }
    if letter.applied_decorations != parent.applied_decorations {
        out.applied_decorations = letter.applied_decorations;
    }
    if crate::render::paint_pass::fills(letter.bg) {
        out.bg = letter.bg;
    }
    (out != *own).then_some(out)
}
