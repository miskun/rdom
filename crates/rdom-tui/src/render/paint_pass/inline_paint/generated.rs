//! Generated content in inline flows (CSS 2.1 §12.1, CSS Pseudo 4 §2): a
//! `::before` / `::after` run of text where the packer put it, an atomic
//! one's box and content at its turn in its line, and a floated one's box
//! in its stacking context's float layer — each in its pseudo-element's
//! style, a running transition's overrides included.

use rdom_core::{Dom, NodeId};

use super::{FlowPlacement, anchor_href_for, paint_inline_layout};
use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::node::TuiNodeExt;
use crate::render::paint_pass::text::{paint_text_from, pseudo_style};
use crate::render::{Buffer, Rect};

/// Paint one generated-content run at its packed cell, in the style of
/// its host's pseudo-element (transition overrides included), tagged
/// with the host's enclosing `<a href>` link, if any. The run
/// starts at its logical x even when that is left of the clip —
/// `paint_text_from` skips the clipped prefix.
pub(super) fn paint_generated(
    dom: &Dom<TuiExt>,
    generated: &crate::render::inline::GeneratedFragment,
    origin_x: i32,
    y: u16,
    clip_left: u16,
    right: u16,
    buf: &mut Buffer,
) {
    let node = dom.node(generated.host);
    let computed = match generated.slot {
        crate::ext::PseudoSlot::Before => node.computed_before(),
        crate::ext::PseudoSlot::After => node.computed_after(),
    };
    let Some(computed) = computed else {
        return;
    };
    if !crate::render::visibility::shows(dom, generated.host, generated.slot.into()) {
        return;
    }
    let style = pseudo_style(
        computed,
        presentation_of(dom, generated.host, generated.slot.into()),
    );
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

/// The in-flight transition overrides for one of `id`'s pseudo-element
/// slots, borrowed; an empty set when the element has no ext.
pub(super) fn presentation_of(
    dom: &Dom<TuiExt>,
    id: NodeId,
    slot: crate::ext::StyleSlot,
) -> &crate::ext::PresentationStyle {
    static EMPTY: std::sync::LazyLock<crate::ext::PresentationStyle> =
        std::sync::LazyLock::new(crate::ext::PresentationStyle::default);
    dom.node(id)
        .ext()
        .and_then(|e| e.presentation_for(slot))
        .unwrap_or(&EMPTY)
}
