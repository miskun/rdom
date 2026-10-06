//! The packer's intake log and its replay, for the `text-wrap-style`
//! values that choose breaks by trying more than one layout (`wrap`):
//! every call that feeds the packer is recorded as an [`Op`] — the text
//! borrowed from the DOM, each atom with the width and rows it was
//! measured at — so a replica packer, with its lines capped narrower, is
//! fed the same content without walking or measuring the tree again.

use rdom_core::NodeId;

use super::super::run_style::RunStyle;
use super::super::vertical::AtomRows;
use super::LinePacker;
use crate::ext::PseudoSlot;
use crate::render::box_tree::BoxItem;

/// One call that fed the packer.
#[derive(Debug, Clone, Copy)]
pub(in crate::render::inline) enum Op<'a> {
    Text {
        owner: NodeId,
        text_node: NodeId,
        text: &'a str,
        run: RunStyle,
    },
    Generated {
        host: NodeId,
        slot: PseudoSlot,
        text: &'a str,
        run: RunStyle,
    },
    HardBreak(NodeId),
    Opportunity,
    Atom {
        node: NodeId,
        width: u16,
        rows: AtomRows,
        align: super::BoxAlign,
    },
    GeneratedAtom {
        host: NodeId,
        slot: PseudoSlot,
        width: u16,
        rows: AtomRows,
        align: super::BoxAlign,
    },
    Float(BoxItem),
    /// An inline box opens (`frames`).
    Enter(super::BoxRows, super::BoxAlign),
    /// The inline box last opened ends.
    Leave,
}

/// Caps on the width of lines, below their line box's: per group of lines
/// between forced breaks (`balance`), and on one line (`pretty`).
#[derive(Debug, Clone, Default)]
pub(in crate::render::inline) struct WidthCaps {
    /// Group `g`'s lines take at most `groups[g]` cells (a group past the
    /// end is uncapped).
    pub(in crate::render::inline) groups: Vec<u16>,
    /// Line `.0` takes at most `.1` cells.
    pub(in crate::render::inline) line: Option<(usize, u16)>,
}

impl WidthCaps {
    /// The cap on line `line` of group `group`.
    pub(super) fn cap(&self, group: usize, line: usize) -> u16 {
        let by_group = self.groups.get(group).copied().unwrap_or(u16::MAX);
        match self.line {
            Some((l, cap)) if l == line => by_group.min(cap),
            _ => by_group,
        }
    }
}

impl<'a> LinePacker<'a> {
    /// Record every call that feeds the packer ([`Self::take_ops`]).
    pub(in crate::render::inline) fn recording(mut self) -> Self {
        self.ops = Some(Vec::new());
        self
    }

    /// Record `op` when recording.
    pub(super) fn log(&mut self, op: Op<'a>) {
        if let Some(ops) = self.ops.as_mut() {
            ops.push(op);
        }
    }

    /// The recorded calls.
    pub(in crate::render::inline) fn take_ops(&mut self) -> Vec<Op<'a>> {
        self.ops.take().unwrap_or_default()
    }

    /// The group (lines between forced breaks) of each line packed.
    pub(in crate::render::inline) fn line_groups(&self) -> &[usize] {
        &self.line_groups
    }

    /// Whether a float was met in the content (a replay would place it
    /// again).
    pub(in crate::render::inline) fn met_float(&self) -> bool {
        self.met_float
    }

    /// A packer like this one — its width, direction, indent and
    /// alignment, no floats beside its lines — with `caps` on its lines.
    pub(in crate::render::inline) fn replica(&self, caps: WidthCaps) -> LinePacker<'a> {
        let mut packer = LinePacker::new(self.content_width)
            .with_strut(self.frames.strut())
            .starting_right(self.rtl)
            .indented(self.indent)
            .aligned(self.align);
        packer.caps = caps;
        packer
    }

    /// Feed the recorded calls `ops` again.
    pub(in crate::render::inline) fn replay(&mut self, ops: &[Op<'a>]) {
        for &op in ops {
            match op {
                Op::Text {
                    owner,
                    text_node,
                    text,
                    run,
                } => self.push_text(owner, text_node, text, run),
                Op::Generated {
                    host,
                    slot,
                    text,
                    run,
                } => self.push_generated(host, slot, text, run),
                Op::HardBreak(owner) => self.push_hard_break(owner),
                Op::Opportunity => self.push_break_opportunity(),
                Op::Atom {
                    node,
                    width,
                    rows,
                    align,
                } => self.push_atomic_inline_block(node, width, rows, align),
                Op::GeneratedAtom {
                    host,
                    slot,
                    width,
                    rows,
                    align,
                } => self.push_generated_atom(host, slot, width, rows, align),
                Op::Float(item) => self.push_float(item),
                Op::Enter(rows, align) => self.enter_box(rows, align),
                Op::Leave => self.leave_box(),
            }
        }
    }
}
