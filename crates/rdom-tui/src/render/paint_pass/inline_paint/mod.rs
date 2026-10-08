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
//! - `mod.rs` — the two entry points above and [`paint_anonymous_blocks`].
//! - [`flow`] — the shared line walker (`paint_inline_layout`). A static
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
//! - [`highlight_overlay`] — the highlight overlays over the cells of a
//!   fragment inside a registered highlight's range (`::highlight()`) or
//!   the selection's (`::selection`), one path for both.

mod caret;
mod chrome;
#[cfg(test)]
mod cost_tests;
mod flow;
mod generated;
mod highlight_overlay;
mod single_row;
mod text_overflow;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::node::TuiNodeExt;
use crate::render::{Buffer, Rect};
use crate::style::ComputedStyle;

use super::text::glyph_style_from_computed;
use chrome::inline_chrome;
use flow::{FlowPlacement, paint_inline_layout};
use single_row::paint_single_row_chrome;
use text_overflow::Marking;

pub(super) use caret::paint_caret_if_editable;
pub(crate) use chrome::{ChromeText, InlineChromeFn};
pub(in crate::render::paint_pass) use generated::{paint_floated_pseudo, paint_positioned_pseudo};

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
    clips: (Rect, Rect),
) {
    let (clip, viewport) = clips;
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
        // The chrome is the host's own content (CSS Display 3 §4).
        if crate::render::visibility::shows(dom, id, crate::ext::StyleSlot::Host) {
            paint_single_row_chrome(dom, id, &text, style, inner, buf, clip);
        }
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
        paint_lines(dom, id, layout, inner, buf, (clip, viewport));
        return;
    }

    // A block container whose in-flow children the block pass laid out
    // has no own text here, and its `::before` / `::after` were placed
    // by that pass (a line of their own, or a list marker on a
    // descendant's first line — `inline::generated`). Painting them
    // again at its first row would draw them under its first child.
    if computed.flow.is_block_flow()
        && crate::render::box_tree::children(dom, id)
            .any(|c| crate::render::layout_pass::is_in_flow(dom, c))
    {
        return;
    }

    // Path 3: no inline_layout (the element is IFC-zeroed as a flex
    // child, or has no own text and no children). Fall back to
    // single-row chrome — render ::before + own_text_content (if any)
    // + ::after at the inner rect.
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
    (clip, viewport): (Rect, Rect),
) {
    // `inner` is the *scrolled* content rect (see
    // `inline::scrolled_content_rect`).
    let at = FlowPlacement {
        inner,
        bg_dedup_owner: id,
        boxed: None,
    };
    let marking = Marking::of(dom, id, None);
    paint_inline_layout(dom, layout, at, marking.as_ref(), buf, clip, viewport);

    // Anchor href tagging for whole-element anchors (e.g.
    // block-level `<a>` with text content and no inline descendants).
    // Per-fragment anchors are handled inside the loop.
    if let Some(href) = anchor_href_for(dom, id) {
        // Walk every line and tag the painted width.
        for line in &layout.lines {
            let line_y = inner.y + i32::from(line.text_row());
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
    inner: LayoutRect,
    buf: &mut Buffer,
    clip: Rect,
    viewport: Rect,
) {
    let Some(inline_layout) = dom
        .node(id)
        .tui_ext()
        .and_then(|e| e.inline_layout.as_ref())
    else {
        return;
    };
    let at = FlowPlacement {
        inner,
        bg_dedup_owner: id,
        boxed: None,
    };
    let marking = Marking::of(dom, id, None);
    paint_inline_layout(
        dom,
        inline_layout,
        at,
        marking.as_ref(),
        buf,
        clip,
        viewport,
    );
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
    viewport: Rect,
) {
    let Some(ext) = dom.node(container_id).tui_ext() else {
        return;
    };
    // The host's `::before` / `::after` are packed into the first / last
    // anonymous box by the layout pass (CSS 2.1 §9.2.1.1); they arrive
    // here as the layouts' generated fragments.
    for (k, anon) in ext.anonymous_blocks.iter().enumerate() {
        // An anonymous block box's lines are its container's (CSS Overflow
        // 4 §3 applies to the block container's line boxes).
        let marking = match anon.generated {
            // A `::before` / `::after` box's lines are its own block
            // container's.
            Some(g) => Marking::of_generated(dom, g, anon.rect, &anon.inline_layout, Some(k)),
            None => Marking::of(dom, container_id, Some(k)),
        };
        let at = FlowPlacement {
            inner: anon.rect,
            bg_dedup_owner: container_id,
            boxed: anon.generated.map(|g| (g.host, g.slot)),
        };
        paint_inline_layout(
            dom,
            &anon.inline_layout,
            at,
            marking.as_ref(),
            buf,
            clip,
            viewport,
        );
    }
}

/// Concatenate the text content of `id`'s direct Text-node children.
/// Element children are NOT recursed — their content paints
/// separately at their own layout positions.
fn own_text_content(dom: &Dom<TuiExt>, id: NodeId) -> String {
    let mut out = String::new();
    for child in crate::render::box_tree::children(dom, id) {
        let child = dom.node(child);
        if child.node_type() == NodeType::Text
            && let Some(data) = child.node_value()
        {
            out.push_str(data);
        }
    }
    out
}
