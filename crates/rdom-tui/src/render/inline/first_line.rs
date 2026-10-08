//! `::first-line` (CSS Pseudo-Elements 4 §2.2): the first formatted line
//! of a block container — its own first line box, or, when its first
//! in-flow content is a block, that block's (and so on down) — styled as
//! if its contents were wrapped in a fictional inline box, the
//! `::first-line` pseudo-element, inside the block and around its inline
//! descendants (§2.2.1's fictional tag sequence).
//!
//! The DOM text is never split. Layout switches style at the line
//! boundary in the packer (`packer::first`): the properties that shape
//! the text — `text-transform`, `letter-spacing`, `word-spacing` — map
//! each run's values while the packer is on the first line
//! ([`FirstLineRun`]), and the word that no longer fits is reshaped in its
//! own style when the line ends. Paint reads the rest — color, the
//! background, the font's weight and style, the decorations — for the
//! line the packer marked (`LineBox::first_line`, [`effective`]).
//!
//! An inline descendant inherits a value from the fictional box when its
//! own is the block's (it set none): rdom keeps computed values, not
//! which of them were inherited, so one that sets the block's value
//! explicitly takes the first line's too (DIVERGENCES §2).

use rdom_core::{Dom, NodeId};

use super::packer::LinePacker;
use super::run_style::RunStyle;
use crate::ext::TuiExt;
use crate::layout::{Display, TextTransform};
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;
use crate::style::ComputedStyle;

/// The block containers whose first formatted line is the first line of
/// the flow packed for `block` — its own lines, or the anonymous box run
/// holding its first formatted line (`first_formatted`) — innermost first:
/// `block`, then each ancestor whose first line-bearing content is the
/// one below it (CSS Pseudo-Elements 4 §2.2). Empty when none of them has
/// a `::first-line` or `::first-letter`, which is what makes the walk
/// worth its cost: it climbs only as far as the outermost one that does.
pub(crate) fn hosts(dom: &Dom<TuiExt>, block: NodeId, first_formatted: bool) -> Vec<NodeId> {
    if !first_formatted {
        return Vec::new();
    }
    let has_first = |id: NodeId| {
        dom.node(id)
            .ext()
            .is_some_and(|e| e.computed_first_line.is_some() || e.computed_first_letter.is_some())
    };
    // The outermost ancestor that could style this line: nothing above it
    // needs the (allocating) first-content check.
    let mut top = None;
    let mut cur = Some(block);
    while let Some(id) = cur {
        if has_first(id) {
            top = Some(id);
        }
        cur = crate::render::box_tree::box_parent(dom, id);
    }
    let Some(top) = top else {
        return Vec::new();
    };
    let mut out = vec![block];
    let mut child = block;
    while child != top {
        let Some(parent) = crate::render::box_tree::box_parent(dom, child) else {
            break;
        };
        if !first_line_passes_down(dom, parent, child) {
            break;
        }
        out.push(parent);
        child = parent;
    }
    out
}

/// Whether `parent`'s first formatted line is its block child `child`'s:
/// `parent` is a block container whose first line-bearing content is
/// `child`, a block-level box, with no line of its own before it (an
/// inline `::before` takes one, CSS 2.1 §9.2.1.1).
fn first_line_passes_down(dom: &Dom<TuiExt>, parent: NodeId, child: NodeId) -> bool {
    let block_level = dom
        .node(child)
        .computed()
        .is_some_and(|c| c.display == Display::Block);
    block_level
        && dom
            .node(parent)
            .computed()
            .is_some_and(crate::style::cascade::is_block_container)
        && !super::generated::own_line_pseudos(dom, parent).before
        && super::generated::line_bearing_child(dom, parent, false) == Some(BoxItem::Node(child))
}

/// The `::first-line` style of the first of `hosts` (innermost first) to
/// have one, and the others outward from it: the styles that apply to
/// the line, outermost first (each nested inside the one before it).
fn line_styles<'d>(
    dom: &'d Dom<TuiExt>,
    hosts: &[NodeId],
) -> Vec<(&'d ComputedStyle, &'d ComputedStyle)> {
    hosts
        .iter()
        .rev()
        .filter_map(|&h| {
            let ext = dom.node(h).ext()?;
            Some((
                ext.computed.as_deref()?,
                ext.computed_first_line.as_deref()?,
            ))
        })
        .collect()
}

/// What the first line does to a run's text-shaping values: the
/// `::first-line` values that differ from their hosts' own (the ones a
/// `::first-line` rule set), applied to a run whose value is the
/// block's — inherited, through the fictional tag sequence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FirstLineRun {
    /// The block's own values: a run with them inherits the first line's.
    block: RunStyle,
    transform: Option<TextTransform>,
    letter_spacing: Option<u16>,
    word_spacing: Option<u16>,
}

impl FirstLineRun {
    /// The first line's run mapping for the flow of `block` whose first
    /// line is the first formatted line of `hosts`; `None` when no
    /// `::first-line` there shapes text.
    pub(crate) fn of(dom: &Dom<TuiExt>, block: NodeId, hosts: &[NodeId]) -> Option<Self> {
        let block_style = dom.node(block).computed()?;
        let mut out = FirstLineRun {
            block: RunStyle::of(block_style),
            transform: None,
            letter_spacing: None,
            word_spacing: None,
        };
        for (host, line) in line_styles(dom, hosts) {
            let (h, l) = (RunStyle::of(host), RunStyle::of(line));
            if l.transform != h.transform {
                out.transform = Some(l.transform);
            }
            if l.letter_spacing != h.letter_spacing {
                out.letter_spacing = Some(l.letter_spacing);
            }
            if l.word_spacing != h.word_spacing {
                out.word_spacing = Some(l.word_spacing);
            }
        }
        let shapes =
            out.transform.is_some() || out.letter_spacing.is_some() || out.word_spacing.is_some();
        shapes.then_some(out)
    }

    /// `run` on the first line: each value it inherits from the block
    /// replaced by the first line's.
    pub(crate) fn apply(&self, mut run: RunStyle) -> RunStyle {
        if let Some(t) = self.transform
            && run.transform == self.block.transform
        {
            run.transform = t;
        }
        if let Some(n) = self.letter_spacing
            && run.letter_spacing == self.block.letter_spacing
        {
            run.letter_spacing = n;
        }
        if let Some(n) = self.word_spacing
            && run.word_spacing == self.block.word_spacing
        {
            run.word_spacing = n;
        }
        run
    }
}

/// The style the content of an element styled `own` paints in on a first
/// line of `hosts` (innermost first): `own` with each paint value it
/// inherits from a host — the host's own — replaced by that host's
/// `::first-line` value where a rule set it (§2.2.1: `color`, the font's
/// weight and style, the decorations), and the first line's background
/// behind it where it has none of its own. `None` when no host has a
/// `::first-line` that changes any of them.
pub(crate) fn effective(
    dom: &Dom<TuiExt>,
    hosts: &[NodeId],
    own: &ComputedStyle,
) -> Option<ComputedStyle> {
    use crate::style::Modifier;
    let font = Modifier::BOLD | Modifier::ITALIC;
    let mut out: Option<ComputedStyle> = None;
    for (host, line) in line_styles(dom, hosts) {
        let style = out.get_or_insert_with(|| own.clone());
        if line.fg != host.fg && own.fg == host.fg {
            style.fg = line.fg;
        }
        if line.modifiers & font != host.modifiers & font
            && own.modifiers & font == host.modifiers & font
        {
            style.modifiers = style.modifiers.without(font) | (line.modifiers & font);
        }
        if line.applied_decorations != host.applied_decorations
            && own.applied_decorations == host.applied_decorations
        {
            style.applied_decorations = line.applied_decorations;
        }
        // The fictional box's background (not inherited) is behind every
        // run on the line that has none of its own.
        if crate::render::paint_pass::fills(line.bg) && !crate::render::paint_pass::fills(own.bg) {
            style.bg = line.bg;
        }
    }
    out.filter(|s| s != own)
}

/// `packer`, packing the flow of `block` (`first_formatted`: the one
/// holding its first formatted line), told which blocks' first formatted
/// line its first line is and how their `::first-line` shapes its runs.
pub(super) fn configure<'a>(
    packer: LinePacker<'a>,
    dom: &Dom<TuiExt>,
    block: NodeId,
    first_formatted: bool,
) -> LinePacker<'a> {
    let hosts = hosts(dom, block, first_formatted);
    if hosts.is_empty() {
        return packer;
    }
    let run = FirstLineRun::of(dom, block, &hosts);
    packer.first_line(hosts, run)
}
