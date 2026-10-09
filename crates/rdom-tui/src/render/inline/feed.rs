//! Feeding a block container's box tree to the line packer (CSS 2.1
//! §9.2.1.1, CSS Display 3 §2.4): its text, the inline boxes it descends
//! into with their `::before` / `::after`, its hard breaks, and each
//! atomic inline as one box — shared by layout (`compute_inline_layout`,
//! `pack_run`) and intrinsic measurement (`measure`).

use rdom_core::{Dom, NodeId, NodeType};

use super::packer::{BoxAlign, BoxRows, LinePacker};
use super::run_style::RunStyle;
use super::{RunPseudos, generated, vertical};
use crate::ext::{PseudoSlot, TuiExt};
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;
use crate::style::ComputedStyle;

/// The CSS Text values of the text directly in the element `owner`:
/// its computed ones (they apply to text, CSS Text 3 §3).
pub(super) fn run_of(dom: &Dom<TuiExt>, owner: NodeId) -> RunStyle {
    dom.node(owner)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .map(|c| RunStyle::of(c))
        .unwrap_or_default()
}

/// The rows of a box styled `style` and its alignment in its line (CSS
/// 2.1 §10.8.1): its line height, its `vertical-align`.
fn box_of(style: Option<&ComputedStyle>) -> (BoxRows, BoxAlign) {
    style.map_or_else(Default::default, |c| {
        (
            BoxRows::of(&c.text.line_height),
            BoxAlign::of(&c.vertical_align),
        )
    })
}

/// `id`'s inline box ([`box_of`]).
fn element_box(dom: &Dom<TuiExt>, id: NodeId) -> (BoxRows, BoxAlign) {
    box_of(dom.node(id).computed())
}

/// `host`'s `slot` pseudo-element's inline box ([`box_of`]).
fn pseudo_box(dom: &Dom<TuiExt>, host: NodeId, slot: PseudoSlot) -> (BoxRows, BoxAlign) {
    box_of(dom.node(host).computed_pseudo(slot))
}

/// Feed `host`'s `slot` pseudo-element's `text` as the inline box it is
/// (CSS 2.1 §12.1), styled by the pseudo-element.
fn push_pseudo_text<'a>(
    dom: &'a Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
    text: &'a str,
    packer: &mut LinePacker<'a>,
) {
    let (rows, align) = pseudo_box(dom, host, slot);
    packer.enter_box(rows, align);
    // An inline list-item `::before` / `::after`: its marker first.
    if let Some(marker) = super::markers::inline_pseudo_marker(dom, host, slot) {
        push_marker(dom, marker, packer);
    }
    packer.push_generated(host, slot, text, pseudo_run(dom, host, slot));
    packer.leave_box();
}

/// The CSS Text values of `host`'s `slot` pseudo-element's text.
pub(super) fn pseudo_run(dom: &Dom<TuiExt>, host: NodeId, slot: PseudoSlot) -> RunStyle {
    dom.node(host)
        .computed_pseudo(slot)
        .map(RunStyle::of)
        .unwrap_or_default()
}

/// Feed `block`'s whole inline content to `packer`: its `::before`, its
/// subtree, its `::after` — floated ones placed beside the lines.
pub(super) fn fill_block<'a>(dom: &'a Dom<TuiExt>, block: NodeId, packer: &mut LinePacker<'a>) {
    push_pseudo(dom, block, PseudoSlot::Before, packer, true);
    walk_subtree(dom, block, packer);
    push_pseudo(dom, block, PseudoSlot::After, packer, true);
}

/// Push `host`'s `slot` pseudo-element if it joins `host`'s own inline
/// content (see [`generated`]) — a float only with `floats` (a block
/// container's floats are items of its box sequence, fed in its runs).
/// `::before` first pushes the markers of the list items whose first
/// line this is (`markers`).
fn push_pseudo<'a>(
    dom: &'a Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
    packer: &mut LinePacker<'a>,
    floats: bool,
) {
    if slot == PseudoSlot::Before {
        for marker in super::markers::line_markers(dom, host) {
            push_marker(dom, marker, packer);
        }
    }
    if let Some(kind) = generated::inline_pseudo(dom, host, slot.into()) {
        push_pseudo_box(dom, host, slot, kind, packer, floats);
    }
}

/// Push `host`'s `slot` pseudo-element as the box it is in this inline
/// content (CSS Pseudo 4 §2, `generated::InlinePseudo`): its text, an
/// atomic inline, or — with `floats` — a float.
fn push_pseudo_box<'a>(
    dom: &'a Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
    kind: generated::InlinePseudo<'a>,
    packer: &mut LinePacker<'a>,
    floats: bool,
) {
    match kind {
        generated::InlinePseudo::Text(text) => push_pseudo_text(dom, host, slot, text, packer),
        generated::InlinePseudo::Atom => push_generated_atom(dom, host, slot, packer),
        generated::InlinePseudo::Float if floats => {
            push_float(dom, BoxItem::Generated(host, slot), packer);
        }
        generated::InlinePseudo::Float => {}
    }
}

/// Push `host`'s `slot` pseudo-element, which joins the inline content
/// it is met in, as the box it is ([`push_pseudo_box`]).
fn push_met_pseudo<'a>(
    dom: &'a Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
    packer: &mut LinePacker<'a>,
) {
    if let Some(kind) = generated::inline_pseudo(dom, host, slot.into()) {
        push_pseudo_box(dom, host, slot, kind, packer, true);
    }
}

/// Feed the run `direct_children` of `parent` to `packer`, with the
/// pseudo-elements `pseudos` asks for ([`pack_run`]).
pub(super) fn fill_run<'a>(
    dom: &'a Dom<TuiExt>,
    parent: NodeId,
    direct_children: &[BoxItem],
    pseudos: RunPseudos,
    packer: &mut LinePacker<'a>,
) {
    if pseudos.before {
        push_pseudo(dom, parent, PseudoSlot::Before, packer, false);
    }
    for &item in direct_children {
        let child_id = match item {
            BoxItem::Node(n) => n,
            // The `::before` / `::after` of a box-less child that holds
            // a block box — inline-level boxes of this flow, hosted by it
            // — or `parent`'s own floated one.
            BoxItem::Generated(host, slot) => {
                push_met_pseudo(dom, host, slot, packer);
                continue;
            }
        };
        let child = dom.node(child_id);
        // A text node directly in a box-less child that holds a block
        // box is owned by that child (its parent), not by `parent`.
        let owner = crate::render::box_tree::slot::parent(dom, child_id).unwrap_or(parent);
        match child.node_type() {
            NodeType::Text => {
                if let Some(data) = child.node_value() {
                    // Its anonymous inline box inherits from that child
                    // (CSS Display 3 §2.5), line height included.
                    let anonymous = owner != parent;
                    if anonymous {
                        // `vertical-align` is not inherited: the anonymous
                        // box sits on the baseline.
                        let (rows, _) = element_box(dom, owner);
                        packer.enter_box(rows, BoxAlign::Baseline);
                    }
                    packer.push_text(owner, child_id, data, run_of(dom, owner));
                    if anonymous {
                        packer.leave_box();
                    }
                }
            }
            NodeType::Element => {
                if crate::render::layout_pass::float::float_side(dom, child_id).is_some() {
                    push_float(dom, item, packer);
                    continue;
                }
                if child.tag_name() == Some("br") {
                    packer.push_hard_break(child_id);
                    continue;
                }
                if child.tag_name() == Some("wbr") {
                    packer.push_break_opportunity();
                    continue;
                }
                // An atomic inline participates as one box — see
                // `walk_subtree` for the rationale.
                if child.computed().is_some_and(|c| c.is_atomic_inline()) {
                    push_atom(dom, child_id, packer);
                    continue;
                }
                walk_inline_box(dom, child_id, packer);
            }
            _ => {}
        }
    }
    if pseudos.after {
        push_pseudo(dom, parent, PseudoSlot::After, packer, false);
    }
}

/// Recursively walk `id`'s descendants in document order, feeding
/// every text node's graphemes to `packer`. Descends into
/// `display: inline` elements (their pseudo-elements included — see
/// [`walk_inline_box`]); `<br>` emits a hard line break.
///
/// Non-element children (comments, fragments) are passed through
/// their descendant element walk.
fn walk_subtree<'a>(dom: &'a Dom<TuiExt>, id: NodeId, packer: &mut LinePacker<'a>) {
    use crate::layout::Display;
    for child in crate::render::box_tree::children(dom, id) {
        let child = dom.node(child);
        match child.node_type() {
            NodeType::Text => {
                // Owner is `id` — the direct element parent. Text
                // node's id goes in too for source-offset tracking.
                if let Some(data) = child.node_value() {
                    packer.push_text(id, child.id(), data, run_of(dom, id));
                }
            }
            NodeType::Element => {
                use crate::layout::Position;
                let (display, position) = child
                    .ext()
                    .and_then(|e| e.computed.as_ref())
                    .map(|c| (c.display, c.position))
                    .unwrap_or((Display::Block, Position::Static));
                // Out-of-flow descendants contribute nothing to the
                // inline formatting context: `display: none` generates
                // no box, and `position: absolute|fixed` boxes are
                // placed independently by phase-2 positioning. Skipping
                // them keeps their text out of an ancestor's inline run
                // — e.g. a collapsed tree branch (`[role=group]` set to
                // `display: none`) must not leak "hidden-child" into the
                // parent treeitem's text, and a chip with an absolutely-
                // positioned dropdown must pack only the chip's own text.
                if display == Display::None
                    || matches!(position, Position::Absolute | Position::Fixed)
                {
                    continue;
                }
                // A float leaves the line (CSS 2.1 §9.5): placed beside it.
                if crate::render::layout_pass::float::float_side(dom, child.id()).is_some() {
                    push_float(dom, BoxItem::Node(child.id()), packer);
                    continue;
                }
                // <br> is a hard break. Matches HTML's baked-in
                // behavior; recognized by tag name rather than by a
                // Display variant to avoid complicating the cascade
                // for a one-element special case.
                if child.tag_name() == Some("br") {
                    packer.push_hard_break(child.id());
                    continue;
                }
                // HTML `<wbr>`: "a line break opportunity".
                if child.tag_name() == Some("wbr") {
                    packer.push_break_opportunity();
                    continue;
                }
                // CSS 2.1 §10.8: an atomic inline (`inline-block`,
                // `inline-flex`, `ComputedStyle::is_atomic_inline`)
                // participates in IFC as a single atomic inline-
                // level box. Don't recurse into it — the packer
                // emits one fragment of its width and rows, the layout
                // pass lays the element out at that rect and paint
                // paints it there as a box, at its turn in the line.
                if child.computed().is_some_and(|c| c.is_atomic_inline()) {
                    push_atom(dom, child.id(), packer);
                    continue;
                }
                walk_inline_box(dom, child.id(), packer);
            }
            _ => {}
        }
    }
}

/// Feed one in-flow inline element: its marker when it is an inline
/// list item, its static `::before`, its content, its static `::after`.
/// CSS 2.1 §12.1: the pseudo-elements are the element's first / last
/// inline children, so they pack at its start / end, in its line flow
/// (they wrap, and the text beside them shifts); CSS Pseudo-Elements 4
/// §3.1 puts `::marker` before `::before`. They land in
/// [`LineBox::generated`], hosted by the element.
fn walk_inline_box<'a>(dom: &'a Dom<TuiExt>, id: NodeId, packer: &mut LinePacker<'a>) {
    let (rows, align) = element_box(dom, id);
    packer.enter_box(rows, align);
    if let Some(marker) = super::markers::inline_marker(dom, id) {
        push_marker(dom, marker, packer);
    }
    push_met_pseudo(dom, id, PseudoSlot::Before, packer);
    walk_subtree(dom, id, packer);
    push_met_pseudo(dom, id, PseudoSlot::After, packer);
    packer.leave_box();
}

/// Push the float `item` met in the inline content (CSS 2.1 §9.5). An
/// intrinsic width measurement packs it as an unbreakable box its margin
/// box wide — beside the text on one line for max-content, alone for
/// min-content (CSS Sizing 3 §5.1) — and lays nothing out.
fn push_float(dom: &Dom<TuiExt>, item: BoxItem, packer: &mut LinePacker<'_>) {
    if packer.is_measuring() {
        let width = crate::render::layout_pass::float::size::outer_contribution(
            dom,
            item,
            packer.content_width() > 0,
        );
        let rows = vertical::AtomRows::UNMEASURED;
        match item {
            BoxItem::Node(id) => {
                packer.push_atomic_inline_block(id, width, rows, BoxAlign::Baseline, (0, 0));
            }
            BoxItem::Generated(host, slot) => {
                packer.push_generated_atom(host, slot, width, rows, BoxAlign::Baseline, (0, 0));
            }
        }
        return;
    }
    packer.push_float(item);
}

/// Push `host`'s atomic inline `slot` pseudo-element: its width and its
/// rows in the line (`layout_pass::generated_atoms`), as [`push_atom`]
/// pushes an element's.
fn push_generated_atom(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
    packer: &mut LinePacker<'_>,
) {
    let measuring = packer.is_measuring().then(|| packer.content_width() > 0);
    let cb_width = packer.content_width();
    if let Some((width, rows)) =
        crate::render::layout_pass::generated_atoms::measure(dom, host, slot, cb_width, measuring)
    {
        let (_, align) = pseudo_box(dom, host, slot);
        let style = dom.node(host).computed_pseudo(slot);
        let margins = style.map_or((0, 0), |c| horizontal_margins_of(c, cb_width));
        packer.push_generated_atom(host, slot, width, rows, align, margins);
    }
}

/// Push the inline block `id` as an atom: its width and its rows in
/// the line (`vertical`). CSS 2.1 §10.3.9: an inline block's `auto` width
/// is shrink-to-fit, as a float's is (`float::size::FloatBox`) — its
/// max-content width, down to its min-content one in a narrower
/// containing block (the line's content box). An intrinsic width
/// measurement takes its contribution under the measurement's constraint
/// (CSS Sizing 3 §5.2): its min-content or max-content width, its
/// percentages against no basis.
fn push_atom(dom: &Dom<TuiExt>, id: NodeId, packer: &mut LinePacker<'_>) {
    if packer.is_measuring() {
        let max_content = packer.content_width() > 0;
        let width = crate::render::layout_pass::intrinsic::contribution(
            dom,
            id,
            crate::layout::Direction::Row,
            0,
            0,
            max_content,
        );
        let rows = vertical::AtomRows::UNMEASURED;
        let margins = horizontal_margins(dom, id, 0);
        packer.push_atomic_inline_block(id, width, rows, BoxAlign::Baseline, margins);
        return;
    }
    let cb_width = packer.content_width();
    let width =
        crate::render::layout_pass::float::size::FloatBox::of(dom, BoxItem::Node(id), cb_width)
            .width;
    let rows = vertical::atom_rows(dom, id, width, cb_width);
    let (_, align) = element_box(dom, id);
    let margins = horizontal_margins(dom, id, cb_width);
    packer.push_atomic_inline_block(id, width, rows, align, margins);
}

/// The left and right margins of the inline block `id` in cells (CSS 2.1
/// §10.3.9: they apply; `auto` is 0), a percentage against `cb_width` —
/// a negative one as 0, as its vertical margins count in its line
/// (`vertical`; DIVERGENCES §2).
fn horizontal_margins(dom: &Dom<TuiExt>, id: NodeId, cb_width: u16) -> (u16, u16) {
    dom.node(id)
        .computed()
        .map_or((0, 0), |c| horizontal_margins_of(c, cb_width))
}

/// [`horizontal_margins`] of the computed style `c`.
fn horizontal_margins_of(c: &ComputedStyle, cb_width: u16) -> (u16, u16) {
    let cells =
        |m: &crate::layout::MarginValue| u16::try_from(m.resolve(cb_width).max(0)).unwrap_or(0);
    (cells(&c.margin.left), cells(&c.margin.right))
}

/// Push a list item's marker (CSS Lists 3 §3.5) — an element's, or a
/// list-item `::before` / `::after`'s: `inside`, the line's first inline
/// box, as generated text; `outside`, beside the line — in visual order
/// when it hangs right (`markers::visual_rtl`).
pub(super) fn push_marker<'a>(
    dom: &'a Dom<TuiExt>,
    marker: super::markers::Marker<'a>,
    packer: &mut LinePacker<'a>,
) {
    let (item, slot, text) = (marker.item, marker.slot, marker.text);
    if !marker.outside {
        push_pseudo_text(dom, item, slot, text, packer);
        return;
    }
    let run = pseudo_run(dom, item, slot);
    let right = super::markers::hangs_right(dom, item, slot);
    packer.push_outside_marker(item, slot, text, run, right);
}
