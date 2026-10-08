//! The shared line walker: an inline layout's lines painted where its
//! [`FlowPlacement`] says — text runs, generated content and atomic
//! inline boxes on their rows, `text-overflow` cuts and the highlight
//! overlays over them.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::render::{Buffer, Rect};
use crate::style::ComputedStyle;

use super::super::text::{
    advance_text_by_cells, glyph_style_from_computed, paint_text, style_from_computed,
};
use super::anchor_href_for;
use super::generated::{paint_generated, paint_generated_atom};
use super::highlight_overlay::Overlays;
use super::text_overflow::{Marking, cut_line};

/// Where an inline layout paints: `inner` is the inline flow's content
/// area in viewport coords (scrolled with its scroll container), and
/// `bg_dedup_owner` the element whose
/// `fill_bg` already covers fragments owned by it — those fragments
/// paint with `glyph_style`, leaving that bg to its owner.
#[derive(Clone, Copy)]
pub(super) struct FlowPlacement {
    pub(super) inner: LayoutRect,
    pub(super) bg_dedup_owner: NodeId,
    /// The `::before` / `::after` whose own box these lines are the
    /// content of: its box painted its background, so its text takes
    /// none ([`text::pseudo_glyph_style`](super::super::text::pseudo_glyph_style)).
    pub(super) boxed: Option<(NodeId, crate::ext::PseudoSlot)>,
}

/// Shared body: paint `inline_layout` where `at` says. Each line's text
/// and generated content sit on its baseline row; each atomic inline
/// block paints as a box at its turn (`stacking_walk::paint_line_atom`),
/// clipped to the rows the flow shows.
///
/// Generated fragments (`::before` / `::after`) paint at the cells the
/// packer gave them, in their pseudo-element's style; they never take
/// the selection overlay (they have no DOM position).
pub(super) fn paint_inline_layout(
    dom: &Dom<TuiExt>,
    inline_layout: &crate::render::inline::InlineLayout,
    at: FlowPlacement,
    marking: Option<&Marking>,
    buf: &mut Buffer,
    clip: Rect,
    viewport: Rect,
) {
    let FlowPlacement {
        inner,
        bg_dedup_owner,
        boxed,
    } = at;
    let own_box = |g: &crate::render::inline::GeneratedFragment| boxed == Some((g.host, g.slot));
    // Lines and atoms paint wherever the packer put them, inside `clip`
    // alone: content past the box is not clipped by it (CSS Overflow 3
    // §3.1 `visible`) — a box that clips passes its overflow clip edge in
    // `clip` (`stacking::children_clip`), a scroll container its padding
    // box, so a scrolled flow shows the rows and columns it scrolled to.
    let atom_clip = clip;
    let outer_clip = clip;
    // The highlight overlays — the registered highlights' ranges and the
    // selection's (document-ordered) — computed once per IFC paint,
    // reused across fragments. `None` when there is nothing to
    // highlight.
    let overlays = Overlays::of(dom);
    for (index, line) in inline_layout.lines.iter().enumerate() {
        let line_y = inner.y + i32::from(line.text_row());
        let visible = |y: i32| y >= clip.y as i32 && y < clip.bottom() as i32;
        let text_visible = visible(line_y);
        // Each run of text paints on its inline box's row (`vertical-align`
        // moves it off the baseline row, CSS 2.1 §10.8.1).
        let row_of = |y: u16| inner.y + i32::from(line.top) + i32::from(y);
        // `text-overflow`'s cut of this line, narrowing the clip it paints
        // its text in (CSS Overflow 4 §3).
        let cut = marking.map(|m| cut_line(line, index, inner.x, m));
        let clip = cut
            .as_ref()
            .map_or(clip, |cut| narrow(clip, cut.left, cut.right));
        let line_right = clip.right();
        // The first formatted line of its blocks: their `::first-line`
        // (CSS Pseudo-Elements 4 §2.2) styles what it holds.
        let first = line.first_line.as_deref();
        for generated in &line.generated {
            let (dx, dy) = generated.offset;
            if let Some(atom) = &generated.atom {
                // An atomic pseudo-element paints as a box at its turn
                // in the line, as an element atom does — moved by its
                // relative or sticky offset.
                let x = inner.x + generated.x + dx;
                let end = x + i32::from(generated.width);
                if cut.as_ref().is_none_or(|cut| cut.keeps(x, end)) {
                    let top = inner.y + i32::from(line.top) + i32::from(atom.y) + dy;
                    let border_box = LayoutRect::new(x, top, generated.width, atom.height);
                    paint_generated_atom(
                        dom, generated, atom, border_box, buf, atom_clip, viewport,
                    );
                }
                continue;
            }
            if (dx, dy) != (0, 0) {
                // Moved from its place in the line: painted after the
                // line's text (below).
                continue;
            }
            let row = row_of(generated.y);
            // An outside list marker hangs beside the line: no
            // `text-overflow` cut of the line reaches it.
            let (left, right) = if generated.outside.is_some() {
                (outer_clip.x, outer_clip.right())
            } else {
                (clip.x, line_right)
            };
            if visible(row) {
                let at = (inner.x, row as u16, own_box(generated), first);
                paint_generated(dom, generated, at, left, right, buf);
            }
        }

        for fragment in &line.fragments {
            let frag_x = inner.x + fragment.x;

            // An atomic inline block paints as a box at its laid-out
            // rect, atomically (CSS 2.1 Appendix E, 7.2.1.4.1.1): its
            // shadows, background, border, then its content — over the
            // line content painted before it. The line is its one
            // painter; a positioned atom is skipped there (its stacking
            // context's layers paint it).
            if fragment.atomic {
                let end = frag_x + i32::from(fragment.width);
                if cut.as_ref().is_none_or(|cut| cut.keeps(frag_x, end)) {
                    super::super::paint_line_atom(dom, fragment.node, buf, atom_clip, viewport);
                }
                continue;
            }
            let row = row_of(fragment.y);
            if !visible(row) || frag_x >= clip.right() as i32 {
                continue;
            }
            // Text is drawn by its owner's `visibility` (CSS Display 3
            // §4; it inherits, so a `visible` span shows inside a hidden
            // block).
            if !crate::render::visibility::shows(dom, fragment.node, crate::ext::StyleSlot::Host) {
                continue;
            }

            let computed = dom
                .node(fragment.node)
                .ext()
                .and_then(|e| e.computed.clone())
                .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
            // Fragments owned by the bg-dedup owner (text directly
            // inside the block / anon box) have their bg painted by
            // the owner's `fill_bg`, which stays the one owner of the
            // cell bg (`glyph_style_from_computed`). Inline-
            // child fragments (`<span>` etc.) DO need their own bg
            // in the glyph style since they have no `fill_bg` of
            // their own. A box-less (`display: contents`) owner has no
            // background to paint (CSS Display 3 §2.5).
            // On a first formatted line, in the style the line's
            // `::first-line` gives it — its background, behind a run with
            // none of its own, painted with the run.
            let first_style = first.and_then(|hosts| {
                crate::render::inline::first_line::effective(dom, hosts, &computed)
            });
            // Its block's first letter takes the `::first-letter` style
            // over that (§2.3.1).
            let first_style = match fragment.first_letter {
                Some(host) => {
                    let line = first_style.as_ref().unwrap_or(&computed);
                    crate::render::inline::first_letter::effective(dom, host, line).or(first_style)
                }
                None => first_style,
            };
            let painted = first_style.as_ref().unwrap_or(&computed);
            let line_bg = first_style.as_ref().is_some_and(|f| f.bg != computed.bg);
            let style = if (fragment.node == bg_dedup_owner
                || computed.display == crate::layout::Display::Contents)
                && !line_bg
            {
                glyph_style_from_computed(painted)
            } else {
                style_from_computed(painted)
            };

            let start_x = frag_x.max(clip.x as i32) as u16;
            let skip = start_x as i32 - frag_x;
            let budget_right = line_right;
            if start_x >= budget_right {
                continue;
            }
            let max_width = budget_right - start_x;

            let text_to_paint: &str = if skip > 0 {
                advance_text_by_cells(&fragment.text, skip as u16)
            } else {
                &fragment.text
            };

            // Route through `paint_text` so painted content occludes any
            // border the joiner would re-derive beneath it (z-aware borders).
            paint_text(buf, start_x, row as u16, budget_right, text_to_paint, style);

            // Polish #9: tag this fragment's cells with the
            // enclosing `<a href>`'s URL, if any. The fragment's
            // owner might be the `<a>` directly or a styled
            // descendant (e.g. `<a><b>bold</b></a>`) — walk up.
            if let Some(href) = anchor_href_for(dom, fragment.node) {
                let written_cells = text_to_paint
                    .chars()
                    .map(|_| 1u16)
                    .sum::<u16>()
                    .min(max_width);
                if written_cells > 0 {
                    buf.set_link_range(start_x, row as u16, written_cells, Some(&href));
                }
            }

            // Highlight overlays (CSS Pseudo-Elements 4 §3): restyle the
            // cells inside a highlight's or the selection's range, the
            // fragment's symbols kept so a repaint without them restores
            // the original appearance.
            if let Some(ref overlays) = overlays {
                overlays.paint(dom, buf, row as u16, frag_x, clip, fragment);
            }
        }
        // A relatively positioned or sticky run (CSS 2.1 §9.4.3) moved
        // off its place: over the line's text, uncut by its
        // `text-overflow`, as a positioned box paints after in-flow
        // content (DIVERGENCES §2).
        for generated in line.generated.iter().filter(|g| g.atom.is_none()) {
            let (dx, dy) = generated.offset;
            let row = row_of(generated.y) + dy;
            if (dx, dy) != (0, 0) && visible(row) {
                let (left, right) = (outer_clip.x, outer_clip.right());
                let at = (inner.x + dx, row as u16, own_box(generated), first);
                paint_generated(dom, generated, at, left, right, buf);
            }
        }
        if let (Some(cut), Some(marking)) = (&cut, marking)
            && text_visible
        {
            for &(x, marker) in &cut.markers {
                super::super::text::paint_text_from(
                    buf,
                    x,
                    line_y as u16,
                    outer_clip.x,
                    outer_clip.right(),
                    marker,
                    marking.style,
                );
            }
        }
    }
}

/// `clip` narrowed to the columns `[left, right)`.
fn narrow(clip: Rect, left: i32, right: i32) -> Rect {
    let x = i32::from(clip.x).max(left);
    let end = i32::from(clip.right()).min(right);
    let (x, end) = (
        x.clamp(0, i32::from(u16::MAX)) as u16,
        end.clamp(0, i32::from(u16::MAX)) as u16,
    );
    Rect::new(x, clip.y, end.saturating_sub(x), clip.height)
}
