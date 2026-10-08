//! Relatively positioned and sticky `::before` / `::after` (CSS 2.1
//! §9.4.3, CSS Position 3 §3.4): in flow, laid out where their `display`
//! puts them — a run of a line, an atom, a block-level box, a float, a
//! flex or grid item — and then moved, "without affecting the layout of
//! surrounding boxes", as a relative or sticky element is. The move is
//! applied once the document is laid out and sticky elements are placed:
//! a run or an atom of a line keeps its packed place and records the
//! move (`GeneratedFragment::offset`), which paint and hit-testing add; a
//! box of its own moves its border box and lines.
//!
//! A relative one moves by its insets against its containing block — the
//! content box of the block container whose flow holds it
//! (`relative::relative_offset`); a sticky one by the pin its insets give
//! in the nearest scrollport at or above its host (`sticky::sticky_offset`).

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{PseudoSlot, TuiExt};
use crate::layout::{Display, LayoutRect, Position};
use crate::node::TuiNodeExt;
use crate::render::inline::{InlineLayout, LineBox};

/// Move every relatively positioned and sticky pseudo-element of the
/// document from its in-flow place. Walks only the subtrees whose
/// cascade found a positioned pseudo-element
/// (`TuiExt::tree_has_positioned_pseudo`).
pub(in crate::render::layout_pass) fn offset_in_flow_pseudos(dom: &mut Dom<TuiExt>) {
    let mut owners = Vec::new();
    collect(dom, dom.root(), &mut owners);
    for owner in owners {
        offset_owner(dom, owner);
    }
}

/// The elements under `id` (itself included) whose flows may hold a
/// moved pseudo-element: those in a flagged subtree, outside any
/// `display: none` one.
fn collect(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    for child in crate::render::box_tree::children(dom, id) {
        let child = dom.node(child);
        match child.node_type() {
            NodeType::Fragment => collect(dom, child.id(), out),
            NodeType::Element => {
                let Some(ext) = child.ext() else {
                    continue;
                };
                let hidden = ext
                    .computed
                    .as_ref()
                    .is_some_and(|c| c.display == Display::None);
                if !ext.tree_has_positioned_pseudo || hidden {
                    continue;
                }
                out.push(child.id());
                collect(dom, child.id(), out);
            }
            _ => {}
        }
    }
}

/// Where a moved pseudo-element lies in its owner's layout.
#[derive(Clone, Copy)]
enum At {
    /// The `g`-th generated run or atom of line `line` of the owner's own
    /// lines (`flow: None`) or of its `k`-th anonymous box's.
    Line {
        flow: Option<usize>,
        line: usize,
        g: usize,
    },
    /// The owner's `k`-th anonymous box (a block-level box or a flex or
    /// grid item).
    Anonymous(usize),
    /// The `k`-th floated pseudo-element its run placed.
    Floated(usize),
}

/// Move the relatively positioned and sticky pseudo-elements laid out in
/// the flows of `owner`.
fn offset_owner(dom: &mut Dom<TuiExt>, owner: NodeId) {
    let moves = moves_of(dom, owner);
    if moves.is_empty() {
        return;
    }
    let mut node = dom.node_mut(owner);
    let Some(ext) = node.ext_mut() else {
        return;
    };
    for (at, (dx, dy)) in moves {
        match at {
            At::Line { flow, line, g } => {
                let il = match flow {
                    None => ext.inline_layout.as_mut(),
                    Some(k) => ext
                        .anonymous_blocks
                        .get_mut(k)
                        .map(|a| &mut a.inline_layout),
                };
                if let Some(g) = il
                    .and_then(|il| il.lines.get_mut(line))
                    .and_then(|l| l.generated.get_mut(g))
                {
                    g.offset = (dx, dy);
                }
            }
            At::Anonymous(k) => {
                if let Some(a) = ext.anonymous_blocks.get_mut(k) {
                    shift(a, dx, dy);
                }
            }
            At::Floated(k) => {
                if let Some(a) = ext
                    .floated_pseudos
                    .as_deref_mut()
                    .and_then(|f| f.get_mut(k))
                {
                    shift(a, dx, dy);
                }
            }
        }
    }
}

fn shift(a: &mut crate::ext::AnonymousIfc, dx: i32, dy: i32) {
    a.rect.x += dx;
    a.rect.y += dy;
    if let Some(g) = a.generated.as_mut() {
        g.border_box.x += dx;
        g.border_box.y += dy;
    }
}

/// The moves of the relatively positioned and sticky pseudo-elements in
/// `owner`'s flows, with where each lies.
fn moves_of(dom: &Dom<TuiExt>, owner: NodeId) -> Vec<(At, (i32, i32))> {
    let mut out = Vec::new();
    let Some(ext) = dom.node(owner).ext() else {
        return out;
    };
    let cb = ext.content_layout;
    let definite = dom
        .node(owner)
        .computed()
        .is_some_and(|c| c.height.cells(None).is_some());
    let mover = Mover { dom, cb, definite };
    if let Some(il) = ext.inline_layout.as_ref()
        && let Some(origin) = crate::render::inline::scrolled_content_rect(dom, owner)
    {
        mover.lines(il, origin, None, &mut out);
    }
    for (k, anon) in ext.anonymous_blocks.iter().enumerate() {
        match anon.generated {
            Some(g) => {
                if let Some(m) = mover.offset(g.host, g.slot, g.border_box) {
                    out.push((At::Anonymous(k), m));
                }
            }
            None => mover.lines(&anon.inline_layout, anon.rect, Some(k), &mut out),
        }
    }
    for (k, anon) in ext.floated_pseudos().iter().enumerate() {
        if let Some(g) = anon.generated
            && let Some(m) = mover.offset(g.host, g.slot, g.border_box)
        {
            out.push((At::Floated(k), m));
        }
    }
    out
}

/// What a move is computed against: the containing block of the
/// pseudo-elements in one owner's flows, and whether its height is
/// definite (CSS 2.1 §9.3.2: a percentage `top` / `bottom` needs it).
struct Mover<'a> {
    dom: &'a Dom<TuiExt>,
    cb: LayoutRect,
    definite: bool,
}

impl Mover<'_> {
    /// The moves of the generated runs and atoms of `il`'s lines, laid
    /// out at `origin`.
    fn lines(
        &self,
        il: &InlineLayout,
        origin: LayoutRect,
        flow: Option<usize>,
        out: &mut Vec<(At, (i32, i32))>,
    ) {
        for (line_idx, line) in il.lines.iter().enumerate() {
            for (g_idx, g) in line.generated.iter().enumerate() {
                if g.outside.is_some() {
                    continue;
                }
                if let Some(m) = self.offset(g.host, g.slot, natural(line, g, origin)) {
                    out.push((
                        At::Line {
                            flow,
                            line: line_idx,
                            g: g_idx,
                        },
                        m,
                    ));
                }
            }
        }
    }

    /// The move of `host`'s `slot` pseudo-element, laid out in flow at
    /// `natural`: `None` unless it is relatively positioned or sticky.
    fn offset(&self, host: NodeId, slot: PseudoSlot, natural: LayoutRect) -> Option<(i32, i32)> {
        let style = self.dom.node(host).computed_pseudo(slot)?;
        let m = match style.position {
            Position::Relative => super::relative_offset(style, self.cb, self.definite),
            Position::Sticky => {
                let port =
                    crate::render::layout_pass::sticky::nearest_scrollport(self.dom, Some(host))?;
                let host_cb = self
                    .dom
                    .node(host)
                    .ext()
                    .filter(|_| {
                        self.dom
                            .node(host)
                            .computed()
                            .is_some_and(|c| c.display != Display::Inline)
                    })
                    .map_or(self.cb, |e| e.content_layout);
                crate::render::layout_pass::sticky::sticky_offset(style, natural, port, host_cb)
            }
            _ => return None,
        };
        (m != (0, 0)).then_some(m)
    }
}

/// Where the generated run or atom `g` of `line` sits when its flow is
/// laid out at `origin`: an atom's border box, a run's cells on its row.
fn natural(
    line: &LineBox,
    g: &crate::render::inline::GeneratedFragment,
    origin: LayoutRect,
) -> LayoutRect {
    let top = origin.y + i32::from(line.top);
    match g.atom_rows() {
        Some((y, height)) => LayoutRect::new(origin.x + g.x, top + i32::from(y), g.width, height),
        None => LayoutRect::new(origin.x + g.x, top + i32::from(g.y), g.width, 1),
    }
}
