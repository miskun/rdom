//! Generated content (CSS 2.1 §12.1, CSS Pseudo 4 §2): a `::before` /
//! `::after` run of text where the packer put it, an atomic one's box and
//! content at its turn in its line, a floated one's box in its stacking
//! context's float layer and a positioned one's on its positioned layer —
//! each in its pseudo-element's style (which holds a running transition's
//! values).

use rdom_core::{Dom, NodeId};

use super::{FlowPlacement, anchor_href_for, paint_inline_layout};
use crate::ext::{PseudoSlot, TuiExt};
use crate::layout::LayoutRect;
use crate::node::TuiNodeExt;
use crate::render::paint_pass::text::{
    glyph_style_from_computed, paint_text_from, style_from_computed,
};
use crate::render::{Buffer, Rect};

/// Paint one generated-content run at its packed cell, in the style of
/// its host's pseudo-element (running transitions included) — without
/// its background when it is the content of the pseudo-element's own box
/// (`own_box`), which painted that — tagged with the host's enclosing
/// `<a href>` link, if any; on a first formatted line (`first`, its
/// blocks innermost first) in the line's `::first-line` style. The run
/// starts at its logical x (`origin_x` plus its own) even when that is
/// left of the clip — `paint_text_from` skips the clipped prefix.
pub(super) fn paint_generated(
    dom: &Dom<TuiExt>,
    generated: &crate::render::inline::GeneratedFragment,
    (origin_x, y, own_box, first): (i32, u16, bool, Option<&[NodeId]>),
    clip_left: u16,
    right: u16,
    buf: &mut Buffer,
) {
    let computed = dom.node(generated.host).computed_pseudo(generated.slot);
    let Some(computed) = computed else {
        return;
    };
    if !crate::render::visibility::shows(dom, generated.host, generated.slot.into()) {
        return;
    }
    // On a first formatted line (`first`), a `::before` / `::after` is in
    // the line's fictional `::first-line` box (CSS Pseudo-Elements 4
    // §2.2.1); a list marker is not.
    let line_style = first
        .filter(|_| matches!(generated.slot, PseudoSlot::Before | PseudoSlot::After))
        .and_then(|hosts| crate::render::inline::first_line::effective(dom, hosts, computed));
    let computed = line_style.as_ref().unwrap_or(computed);
    // The first letter of its block's first line, in generated text
    // (§2.3), in its `::first-letter` style.
    let letter_style = generated
        .first_letter
        .and_then(|host| crate::render::inline::first_letter::effective(dom, host, computed));
    let computed = letter_style.as_ref().unwrap_or(computed);
    // A box of its own painted its background under the glyphs once (CSS
    // Backgrounds 3 §3.10). Running transitions' values are in the style.
    let style = if own_box {
        glyph_style_from_computed(computed)
    } else {
        style_from_computed(computed)
    };
    let x = origin_x + generated.x;
    let end = paint_text_from(buf, x, y, clip_left, right, &generated.text, style);
    // A pseudo-element is part of its host: an `<a href>`'s (or its
    // descendant's) generated cells belong to the link.
    if let Some(href) = anchor_href_for(dom, generated.host) {
        let start = x.max(i32::from(clip_left));
        let end = end.min(i32::from(right));
        if end > start {
            buf.set_link_range(start as u16, y, (end - start) as u16, Some(&href));
        }
    }
}

/// Paint an atomic `::before` / `::after` (CSS Pseudo 4 §2) at
/// `border_box`: its box — background, border — then the content laid
/// out inside it, atomically (CSS 2.1 Appendix E, 7.2.1.4.1.1).
pub(super) fn paint_generated_atom(
    dom: &Dom<TuiExt>,
    generated: &crate::render::inline::GeneratedFragment,
    atom: &crate::render::inline::GeneratedAtom,
    border_box: LayoutRect,
    buf: &mut Buffer,
    clip: Rect,
    viewport: Rect,
) {
    let g = crate::ext::GeneratedBox::new(generated.host, generated.slot, border_box);
    let Some((at, lines)) = atom.content.as_ref() else {
        crate::render::paint_pass::generated_box::paint_box(dom, g, border_box, buf, clip);
        return;
    };
    let lines_at = LayoutRect::new(
        border_box.x + at.x,
        border_box.y + at.y,
        at.width,
        at.height,
    );
    paint_generated_box(dom, g, (lines_at, lines), buf, clip, viewport);
}

/// Paint a `::before` / `::after` laid out as a box of its own — an atom,
/// a float — `g`, its lines `lines` at the rect given: its background and
/// border, then its lines.
fn paint_generated_box(
    dom: &Dom<TuiExt>,
    g: crate::ext::GeneratedBox,
    (lines_at, lines): (LayoutRect, &crate::render::inline::InlineLayout),
    buf: &mut Buffer,
    clip: Rect,
    viewport: Rect,
) {
    crate::render::paint_pass::generated_box::paint_box(dom, g, lines_at, buf, clip);
    let at = FlowPlacement {
        inner: lines_at,
        bg_dedup_owner: g.host,
        boxed: Some((g.host, g.slot)),
    };
    let marking = super::Marking::of_generated(dom, g, lines_at, lines, None);
    paint_inline_layout(dom, lines, at, marking.as_ref(), buf, clip, viewport);
}

/// Paint the `k`-th floated `::before` / `::after` `owner`'s formatting
/// context run placed (CSS 2.1 §9.5, Appendix E step 5), atomically.
pub(in crate::render::paint_pass) fn paint_floated_pseudo(
    dom: &Dom<TuiExt>,
    owner: NodeId,
    k: usize,
    buf: &mut Buffer,
    clip: Rect,
    viewport: Rect,
) {
    let Some(anon) = dom
        .node(owner)
        .ext()
        .and_then(|e| e.floated_pseudos.as_deref())
        .and_then(|f| f.get(k))
    else {
        return;
    };
    if let Some(g) = anon.generated {
        paint_generated_box(
            dom,
            g,
            (anon.rect, &anon.inline_layout),
            buf,
            clip,
            viewport,
        );
    }
}

/// Paint `host`'s `k`-th absolutely or fixed positioned `::before` /
/// `::after` (CSS Pseudo 4 §2) on its stacking context's positioned
/// layer, as a positioned element paints: its box, then its lines.
pub(in crate::render::paint_pass) fn paint_positioned_pseudo(
    dom: &Dom<TuiExt>,
    host: NodeId,
    k: usize,
    buf: &mut Buffer,
    clip: Rect,
    viewport: Rect,
) {
    let Some(anon) = dom
        .node(host)
        .ext()
        .and_then(|e| e.positioned_pseudo_boxes().get(k))
    else {
        return;
    };
    if let Some(g) = anon.generated {
        paint_generated_box(
            dom,
            g,
            (anon.rect, &anon.inline_layout),
            buf,
            clip,
            viewport,
        );
    }
}
