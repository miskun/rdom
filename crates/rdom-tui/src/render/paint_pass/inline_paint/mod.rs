//! Inline paint paths — `::before` / own text / `::after` for
//! non-IFC elements, and fragment-driven IFC paint for blocks that
//! establish an inline formatting context.
//!
//! - [`paint_inline_content`] — the classic paint path. Used by
//!   every non-IFC element. Concatenates direct Text-node children
//!   as "own text," wraps with `::before` and `::after` pseudo
//!   content.
//! - [`paint_ifc`] — the IFC path. Reads the pre-computed
//!   `InlineLayout` from `TuiExt` and paints each fragment with its
//!   owner element's cascaded style at `(content.x + fragment.x,
//!   content.y + line_index)`.
//!
//! ## Module layout
//!
//! - `mod.rs` — the two entry points above, [`paint_anonymous_blocks`],
//!   and the shared line walker (`paint_inline_layout`). A static
//!   `::before` / `::after` in an inline flow is packed by the layout
//!   pass ([`GeneratedFragment`](crate::render::inline::GeneratedFragment));
//!   paint draws it where the packer put it.
//! - [`single_row`] — the single-row painter (`::before` + body +
//!   `::after` on one row) behind chrome substitution and the
//!   no-own-text fallback.
//! - [`chrome`] — the chrome-substitution contract ([`ChromeText`],
//!   [`InlineChromeFn`]) and the one call that asks the built-ins for
//!   an element's replacement text. Paint never reads builtin state
//!   itself.
//! - [`caret`] — the collapsed-selection caret overlay.
//! - [`selection_overlay`] — the `::selection` highlight over the
//!   cells of a fragment that fall inside the current selection range.

mod caret;
mod chrome;
mod selection_overlay;
mod single_row;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::node::TuiNodeExt;
use crate::render::{Buffer, Rect};
use crate::style::ComputedStyle;

use super::text::{
    advance_text_by_cells, glyph_style_from_computed, paint_text, paint_text_from, pseudo_style,
    style_from_computed,
};
use chrome::inline_chrome;
use selection_overlay::apply_selection_overlay;
use single_row::paint_single_row_chrome;

pub(super) use caret::paint_caret_if_editable;
pub(crate) use chrome::{ChromeText, InlineChromeFn};

/// `::before` + own text + `::after` paint for a non-IFC element.
///
/// Three paths:
///
/// 1. **Chrome substitution** — gauge (`<progress>` / `<meter>`),
///    closed `<select>` dropdown, password mask. These replace own
///    text with a single-row glyph string; layout never multi-line
///    sizes them. Painted as a single row alongside `::before` /
///    `::after`. Which elements substitute, and with what, is the
///    built-ins' business — see [`chrome`].
///
/// 2. **Multi-line own text** — element has an `InlineLayout` in
///    its `ext` (populated by the layout pass for pure-text leaf
///    blocks). Iterate the lines like the IFC path: each
///    [`LineBox`] paints at `inner.y + line_index`. The packer laid
///    `::before` / `::after` out as the first / last inline content,
///    so they wrap with the text.
///
/// 3. **No own text** — element has only `::before` / `::after`
///    chrome. Single-row paint as before.
///
/// `inner` is the element's content rect; `clip` is the current
/// paint clip (overflow-restricted).
pub(super) fn paint_inline_content(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    inner: LayoutRect,
    buf: &mut Buffer,
    clip: Rect,
) {
    // Mixed content (a direct text run AND in-flow block children): the own
    // text run lives in an anonymous block, painted by `paint_anonymous_blocks`.
    // This Path-3 pass would paint the own text a SECOND time (at a different x
    // once `::before` shifts it), duplicating the tail — `<li>Label<ul>` →
    // "Labelel". So bail when anonymous blocks exist; the anon-block pass owns
    // the inline content, the host's `::before` / `::after` included (the
    // layout pass packs them into the first / last anonymous box).
    if dom
        .node(id)
        .ext()
        .is_some_and(|e| !e.anonymous_blocks.is_empty())
    {
        return;
    }

    // Path 1: chrome substitution. Always a single row by construction
    // (gauges and closed dropdowns are height-1 chrome; a password input
    // is height 1 and its displayed text is bullets, not the underlying
    // value — the IFC packer wouldn't know to mask, so it routes around
    // `paint_lines` too).
    let avail_single_row = inner.width;
    if let Some((text, style)) = inline_chrome(dom, id, computed, avail_single_row) {
        paint_single_row_chrome(dom, id, &text, style, inner, buf, clip);
        return;
    }

    // Path 2: line-aware paint when the layout pass populated an
    // `InlineLayout` for this element (pure-text leaf block — e.g.
    // a `<textarea>`, or any element with own text and no element
    // children). Iterate lines and paint fragments and generated
    // content where the packer put them.
    if let Some(layout) = dom
        .node(id)
        .tui_ext()
        .and_then(|e| e.inline_layout.as_ref())
    {
        paint_lines(dom, id, layout, inner, buf, clip);
        return;
    }

    // A block container whose in-flow children the block pass laid out
    // has no own text here, and its `::before` / `::after` were placed
    // by that pass (a line of their own, or a list marker on a
    // descendant's first line — `inline::generated`). Painting them
    // again at its first row would draw them under its first child.
    if computed.flow == crate::layout::Flow::Block
        && dom
            .node(id)
            .child_nodes()
            .any(|c| crate::render::layout_pass::is_in_flow(dom, c.id()))
    {
        return;
    }

    // Path 3: no inline_layout (either because the element is IFC-
    // zeroed as a flex/IFC child OR because it has no own text and
    // no children). Fall back to single-row chrome — render
    // ::before + own_text_content (if any) + ::after at the inner
    // rect. This covers the BFC-1 phase 3.5b atomic inline-block
    // path: `paint_anonymous_blocks` / `paint_ifc` paint the
    // atomic by calling back into `paint_inline_content` at the
    // fragment's rect, but the inline-block child's own
    // `inline_layout` was cleared during the parent IFC's zeroing
    // pass.
    paint_single_row_chrome(
        dom,
        id,
        &own_text_content(dom, id),
        glyph_style_from_computed(computed),
        inner,
        buf,
        clip,
    );
}

/// Line-aware paint for a pure-text leaf block: its `InlineLayout`
/// (generated content included) at `inner`, then the whole-element
/// anchor tag.
fn paint_lines(
    dom: &Dom<TuiExt>,
    id: NodeId,
    layout: &crate::render::inline::InlineLayout,
    inner: LayoutRect,
    buf: &mut Buffer,
    clip: Rect,
) {
    // `inner` is the *scrolled* content rect (see
    // `inline::scrolled_content_rect`): the block shows `inner.height`
    // lines starting at its own `scroll_y`.
    let first_visible_line = dom.node(id).ext().map_or(0, |e| e.scroll_y as i32);
    paint_inline_layout(dom, layout, inner, first_visible_line, id, buf, clip);

    // Anchor href tagging for whole-element anchors (e.g.
    // block-level `<a>` with text content and no inline descendants).
    // Per-fragment anchors are handled inside the loop.
    if let Some(href) = anchor_href_for(dom, id) {
        // Walk every line and tag the painted width.
        for (line_index, line) in layout.lines.iter().enumerate() {
            let line_y = inner.y + line_index as i32;
            if line_y < clip.y as i32 || line_y >= clip.bottom() as i32 {
                continue;
            }
            let start_x = inner.x.max(clip.x as i32) as u16;
            let cells = line.width;
            if cells > 0 {
                buf.set_link_range(start_x, line_y as u16, cells, Some(&href));
            }
        }
    }
}

/// Return the `href` attribute if `id` (or any ancestor) is an
/// `<a href>`. Used by both paint paths to propagate hyperlink
/// info from anchors to the cells painted for their text content
/// — including when an anchor wraps inline descendants like
/// `<a><b>bold</b></a>`.
fn anchor_href_for(dom: &Dom<TuiExt>, id: NodeId) -> Option<String> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        let node = dom.node(n);
        if node.tag_name() == Some("a")
            && let Some(href) = node.get_attribute("href")
        {
            return Some(href.to_string());
        }
        cur = node.parent_node().map(|p| p.id());
    }
    None
}

pub(super) fn paint_ifc(
    dom: &Dom<TuiExt>,
    id: NodeId,
    _block_computed: &ComputedStyle,
    inner: LayoutRect,
    buf: &mut Buffer,
    clip: Rect,
) {
    let Some(inline_layout) = dom
        .node(id)
        .tui_ext()
        .and_then(|e| e.inline_layout.as_ref())
    else {
        return;
    };
    let first_visible_line = dom.node(id).ext().map_or(0, |e| e.scroll_y as i32);
    paint_inline_layout(dom, inline_layout, inner, first_visible_line, id, buf, clip);
    // Caret is painted by `paint_node` once per element that owns
    // an inline-flow container (IFC blocks AND pure-text leaf
    // blocks); the call used to live here, but textareas/inputs go
    // through `paint_inline_content` and never reach this function.
    // The hoist keeps both paint paths consistent.
}

/// Paint each anonymous block box (BFC-1 phase 3) attached to
/// `container_id`'s `TuiExt.anonymous_blocks`. Each anon box was
/// synthesized by `layout_pass::block::layout_block_children` for
/// a run of inline-level children inside a block container that
/// also has block-level children. The anon box carries its own
/// `InlineLayout` + `LayoutRect`; this routine paints each.
///
/// `bg_dedup_owner` is the parent block container — fragments
/// whose `node` matches it get their bg painted by the parent's
/// own `fill_bg`, so they paint with `glyph_style` (one owner per
/// cell bg).
pub(super) fn paint_anonymous_blocks(
    dom: &Dom<TuiExt>,
    container_id: NodeId,
    buf: &mut Buffer,
    clip: Rect,
) {
    let Some(ext) = dom.node(container_id).tui_ext() else {
        return;
    };
    // The host's `::before` / `::after` are packed into the first / last
    // anonymous box by the layout pass (CSS 2.1 §9.2.1.1); they arrive
    // here as the layouts' generated fragments.
    for anon in &ext.anonymous_blocks {
        paint_inline_layout(
            dom,
            &anon.inline_layout,
            anon.rect,
            0,
            container_id,
            buf,
            clip,
        );
    }
}

/// Shared body: paint `inline_layout` at `rect` (the IFC's content
/// area in viewport coords). `bg_dedup_owner` is the element whose
/// `fill_bg` already covers fragments owned by it — those fragments
/// paint with `glyph_style`, leaving that bg to its owner.
///
/// Generated fragments (`::before` / `::after`) paint at the cells the
/// packer gave them, in their pseudo-element's style; they never take
/// the selection overlay (they have no DOM position).
fn paint_inline_layout(
    dom: &Dom<TuiExt>,
    inline_layout: &crate::render::inline::InlineLayout,
    inner: LayoutRect,
    first_visible_line: i32,
    bg_dedup_owner: NodeId,
    buf: &mut Buffer,
    clip: Rect,
) {
    // The current selection range (document-ordered) — computed once
    // per IFC paint, reused across fragments. `None` when there's no
    // selection or it's collapsed (caret only, nothing to highlight).
    let selection_range = dom.selection_range().filter(|r| !r.is_collapsed());
    for (line_index, line) in inline_layout.lines.iter().enumerate() {
        let line_y = inner.y + line_index as i32;
        if line_y < clip.y as i32 || line_y >= clip.bottom() as i32 {
            continue;
        }
        // The block shows `inner.height` lines starting at
        // `first_visible_line` (its own scroll offset; 0 for an
        // anonymous box, whose rect is already scrolled). Lines outside
        // that band are above the scrollport or past the content box.
        // `overflow: hidden` on the block is enforced by the caller's
        // clip rect (set in `paint_node` based on overflow mode).
        let li = line_index as i32;
        if li < first_visible_line || li >= first_visible_line + inner.height as i32 {
            continue;
        }

        let line_right = clip
            .right()
            .min(inner.x.saturating_add(inner.width as i32).max(0) as u16);
        for generated in &line.generated {
            paint_generated(
                dom,
                generated,
                inner.x,
                line_y as u16,
                clip.x,
                line_right,
                buf,
            );
        }

        for fragment in &line.fragments {
            let frag_x = inner.x + fragment.x as i32;
            if frag_x >= clip.right() as i32 {
                continue;
            }

            // Atomic inline-block fragments are rendered via the
            // regular inline-content path at the fragment's rect —
            // pseudo content (`<button>`'s `[ ]`), own text, and
            // background all paint through the host element's
            // normal paint pass. Done before the text-fragment
            // body so atom-specific paint doesn't double-touch
            // the text path. See `crate::render::inline::InlineFragment`
            // doc for the contract.
            if fragment.atomic {
                let atom_rect = LayoutRect::new(frag_x, line_y, fragment.width, 1);
                let atom_computed = dom
                    .node(fragment.node)
                    .ext()
                    .and_then(|e| e.computed.clone())
                    .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
                // A positioned atom belongs to its stacking context's
                // positioned layer, which paints it; painting it here
                // too would blend it twice under `opacity`.
                if atom_computed.position != crate::layout::Position::Static {
                    continue;
                }
                // Reuse paint_inline_content: it handles
                // ::before / own text / ::after at the given inner
                // rect.
                paint_inline_content(dom, fragment.node, &atom_computed, atom_rect, buf, clip);
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
            // their own.
            let style = if fragment.node == bg_dedup_owner {
                glyph_style_from_computed(&computed)
            } else {
                style_from_computed(&computed)
            };

            let start_x = frag_x.max(clip.x as i32) as u16;
            let skip = start_x as i32 - frag_x;
            let budget_right = clip
                .right()
                .min(inner.x.saturating_add(inner.width as i32).max(0) as u16);
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
            paint_text(
                buf,
                start_x,
                line_y as u16,
                budget_right,
                text_to_paint,
                style,
            );

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
                    buf.set_link_range(start_x, line_y as u16, written_cells, Some(&href));
                }
            }

            // Selection overlay: REVERSE the fg/bg of any cells that
            // fall inside the current selection range. Keeps the
            // fragment's symbols + base style intact so a re-paint
            // without selection restores the original appearance.
            if let Some(ref sr) = selection_range {
                apply_selection_overlay(dom, buf, line_y as u16, frag_x, clip, fragment, sr);
            }
        }
    }
}

/// Paint one generated-content run at its packed cell, in the style of
/// its host's pseudo-element (transition overrides included), tagged
/// with the host's enclosing `<a href>` link, if any. The run
/// starts at its logical x even when that is left of the clip —
/// `paint_text_from` skips the clipped prefix.
fn paint_generated(
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
        crate::ext::StyleSlot::Before => node.computed_before(),
        crate::ext::StyleSlot::After => node.computed_after(),
        crate::ext::StyleSlot::Host => None,
    };
    let Some(computed) = computed else {
        return;
    };
    let style = pseudo_style(
        computed,
        presentation_of(dom, generated.host, generated.slot),
    );
    let x = origin_x + i32::from(generated.x);
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

/// The in-flight transition overrides for one of `id`'s pseudo-element
/// slots, borrowed; an empty set when the element has no ext.
fn presentation_of(
    dom: &Dom<TuiExt>,
    id: NodeId,
    slot: crate::ext::StyleSlot,
) -> &crate::ext::PresentationStyle {
    static EMPTY: std::sync::LazyLock<crate::ext::PresentationStyle> =
        std::sync::LazyLock::new(crate::ext::PresentationStyle::default);
    dom.node(id)
        .ext()
        .map(|e| e.presentation_for(slot))
        .unwrap_or(&EMPTY)
}

/// Concatenate the text content of `id`'s direct Text-node children.
/// Element children are NOT recursed — their content paints
/// separately at their own layout positions.
fn own_text_content(dom: &Dom<TuiExt>, id: NodeId) -> String {
    let mut out = String::new();
    for child in dom.node(id).child_nodes() {
        if child.node_type() == NodeType::Text
            && let Some(data) = child.node_value()
        {
            out.push_str(data);
        }
    }
    out
}
