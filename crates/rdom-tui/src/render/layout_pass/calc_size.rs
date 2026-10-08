//! `calc-size()` in layout (CSS Values 5 §10): a `width` / `height` of
//! `calc-size(basis, size * factor + offset)` — authored, or a running
//! transition between a sizing keyword and a length (§11) — is the size
//! its basis resolves to, scaled and offset.
//!
//! Layout runs more than once when the document has one
//! (`doc_flags::has_calc_sizes`): first with each such property at its
//! basis (`auto`, `min-content`, …), which sizes the box as every layout
//! mode sizes that keyword, then with it at the length the sum gives from
//! that size — so flex, grid, block and positioned layout, and every
//! parent measuring the box, see an ordinary length. A box's basis is its
//! size with its content as it is, so `calc-size()` boxes nested in one
//! another resolve inside out: the innermost first, each level in a pass
//! of its own after the one that measured it (C12G-CARRYOVER) — a
//! document whose `calc-size()` boxes do not nest lays out twice, one
//! nested `d` deep `d + 2` times. A box whose sum gives back its basis's
//! size (`calc-size(auto, size)`, the authored accordion at rest) keeps
//! its basis layout, and a level with no box changing size takes no pass
//! (C13G-CALC-SIZE-AUTHORED): the extra passes are paid only while a sum
//! moves a size — an `interpolate-size` transition running — so an
//! authored, resting document lays out once. The computed styles are put
//! back afterwards. A document without one lays out once, with no walk.

use std::rc::Rc;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{BoxSizing, Size};
use crate::render::Rect;
use crate::style::ComputedStyle;

/// Lay `dom` out with `pass` (one whole-tree layout), twice when a box
/// is `calc-size()`d, once more for each level such boxes nest.
pub(super) fn lay_out(dom: &mut Dom<TuiExt>, viewport: Rect, pass: fn(&mut Dom<TuiExt>, Rect)) {
    let sized = if crate::style::doc_flags::has_calc_sizes(dom) {
        collect(dom)
    } else {
        Vec::new()
    };
    if sized.is_empty() {
        // None is left (an `interpolate-size` transition ended): stop
        // paying for the walk.
        crate::style::doc_flags::clear_calc_sizes(dom);
        pass(dom, viewport);
        return;
    }
    for (id, original, _) in &sized {
        put(dom, *id, Rc::new(at_basis(original)));
    }
    pass(dom, viewport);
    let deepest = sized.iter().map(|(_, _, level)| *level).max().unwrap_or(0);
    for level in (0..=deepest).rev() {
        // A box whose sum gives back the size its basis laid it out at
        // (`calc-size(auto, size)` at rest) keeps that layout: only the
        // boxes that change size are put at their length, and a level
        // with none is not laid out again.
        let moved: Vec<(NodeId, Rc<ComputedStyle>)> = sized
            .iter()
            .filter(|(_, _, l)| *l == level)
            .filter_map(|(id, original, _)| {
                resolved(dom, *id, original).map(|style| (*id, Rc::new(style)))
            })
            .collect();
        if moved.is_empty() {
            continue;
        }
        for (id, style) in moved {
            put(dom, id, style);
        }
        pass(dom, viewport);
    }
    for (id, original, _) in sized {
        put(dom, id, original);
    }
}

/// The boxes (in the box tree: a `::details-content` box too) whose
/// `width` or `height` is a `calc-size()`, with their computed styles and
/// how many such boxes are their ancestors (0 for the outermost).
fn collect(dom: &Dom<TuiExt>) -> Vec<(NodeId, Rc<ComputedStyle>, usize)> {
    let mut out = Vec::new();
    let mut stack = vec![(dom.root(), 0)];
    while let Some((id, level)) = stack.pop() {
        let mut below = level;
        if dom.node(id).node_type() == NodeType::Element
            && let Some(style) = dom.node(id).ext().and_then(|e| e.computed.clone())
            && crate::style::doc_flags::is_calc_sized(&style)
        {
            out.push((id, style, level));
            below += 1;
        }
        stack.extend(crate::render::box_tree::children(dom, id).map(|c| (c, below)));
    }
    out
}

fn put(dom: &mut Dom<TuiExt>, id: NodeId, style: Rc<ComputedStyle>) {
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.computed = Some(style);
    }
}

/// `style` with each `calc-size()` at its basis.
fn at_basis(style: &ComputedStyle) -> ComputedStyle {
    let mut out = style.clone();
    for size in [&mut out.width, &mut out.height] {
        if let Size::CalcSize(c) = size {
            *size = c.basis_size();
        }
    }
    out
}

/// `style` with each `calc-size()` at the length its sum gives from the
/// size the last pass laid the box out at — in the box `box-sizing`
/// measures — a percentage in the offset against the containing block;
/// `None` when every such length is that size (an identity sum, as
/// `calc-size(auto, size)`): laid out again, the box would be as it is.
fn resolved(dom: &Dom<TuiExt>, id: NodeId, style: &ComputedStyle) -> Option<ComputedStyle> {
    let ext = dom.node(id).ext()?;
    // The content box: the box less its border, padding and any
    // scrollbar gutter — what `auto` resolved to in `content-box` terms.
    let measured = match style.box_sizing {
        BoxSizing::BorderBox => ext.layout,
        BoxSizing::ContentBox => ext.content_layout,
    };
    let block = crate::render::box_tree::box_parent(dom, id)
        .and_then(|p| dom.node(p).ext().map(|e| e.content_layout))
        .unwrap_or(ext.layout);
    let mut out = style.clone();
    let mut changed = false;
    for (size, basis, percent) in [
        (&mut out.width, measured.width, block.width),
        (&mut out.height, measured.height, block.height),
    ] {
        if let Size::CalcSize(c) = size {
            let length = c.resolve(basis, percent);
            changed |= length != basis;
            *size = Size::Fixed(length);
        }
    }
    changed.then_some(out)
}
