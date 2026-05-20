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

use rdom_core::{DocumentPosition, Dom, NodeId, NodeType, Range};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::node::TuiNodeExt;
use crate::render::inline::{InlineFragment, inline_flow_container};
use crate::render::{Buffer, Rect, Style};
use crate::style::{ComputedStyle, Modifier};

use super::layout_rect_to_grid;
use super::text::{glyph_style_from_computed, paint_text, style_from_computed};

/// `::before` + own text + `::after` paint for a non-IFC element.
///
/// Three paths:
///
/// 1. **Chrome substitution** — gauge (`<progress>` / `<meter>`),
///    closed `<select>` dropdown, password mask. These replace own
///    text with a single-row glyph string; layout never multi-line
///    sizes them. Painted as a single row alongside `::before` /
///    `::after`.
///
/// 2. **Multi-line own text** — element has an `InlineLayout` in
///    its `ext` (populated by the layout pass for pure-text leaf
///    blocks). Iterate the lines like the IFC path: each
///    [`LineBox`] paints at `inner.y + line_index`. `::before` rides
///    line 0; `::after` rides the last line. Honors wrap.
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
    // Path 1: chrome substitution. Always a single row by construction.
    let avail_single_row = inner.width;
    let chrome = if let Some((bar, color)) =
        crate::runtime::builtins::gauge::gauge_text(dom, id, avail_single_row)
    {
        Some((
            bar,
            glyph_style_from_computed(&crate::runtime::builtins::gauge::override_fg(
                computed.clone(),
                color,
            )),
        ))
    } else {
        select_chrome_text(dom, id).map(|chrome| (chrome, glyph_style_from_computed(computed)))
    };

    if let Some((text, style)) = chrome {
        paint_single_row_chrome(dom, id, &text, style, inner, buf, clip);
        return;
    }

    // Path 2: password mask — single-row by construction (password
    // inputs are height: 1) and the displayed text is bullets, not
    // the underlying value. The IFC packer wouldn't know to mask, so
    // route around `paint_lines` and use the single-row chrome path.
    if is_password_input(dom, id) {
        paint_single_row_chrome(
            dom,
            id,
            &mask_if_password(dom, id, own_text_content(dom, id)),
            glyph_style_from_computed(computed),
            inner,
            buf,
            clip,
        );
        return;
    }

    // Path 3: line-aware paint when the layout pass populated an
    // `InlineLayout` for this element (pure-text leaf block — e.g.
    // a `<textarea>`, or any element with own text and no element
    // children). Iterate lines and emit fragments, prepending
    // `::before` to line 0 and appending `::after` to the last line.
    let inline_layout = dom.node(id).ext().and_then(|e| e.inline_layout.clone());
    if let Some(layout) = inline_layout {
        paint_lines(
            dom, id, computed, &layout, inner, buf, clip, /* emit_pseudos = */ true,
        );
        return;
    }

    // Path 4: no own text, just chrome on a single row.
    paint_single_row_chrome(
        dom,
        id,
        "",
        glyph_style_from_computed(computed),
        inner,
        buf,
        clip,
    );
}

/// Single-row paint: `::before` + `body_text` + `::after`. Used by
/// chrome-substitution elements (gauge, select, password mask) and
/// by elements with no own text at all.
fn paint_single_row_chrome(
    dom: &Dom<TuiExt>,
    id: NodeId,
    body_text: &str,
    body_style: Style,
    inner: LayoutRect,
    buf: &mut Buffer,
    clip: Rect,
) {
    let Some(inner_grid) = layout_rect_to_grid(inner, clip) else {
        return;
    };
    let base_y = inner_grid.y;
    let start_x = inner_grid.x;
    let mut cursor_x = start_x;
    let budget_right = inner_grid.right();

    if let Some(before) = dom.node(id).computed_before()
        && before.position == crate::layout::Position::Static
        && let Some(ref text) = before.content
    {
        cursor_x = paint_text(
            buf,
            cursor_x,
            base_y,
            budget_right,
            text,
            style_from_computed(before),
        );
    }

    if !body_text.is_empty() {
        cursor_x = paint_text(buf, cursor_x, base_y, budget_right, body_text, body_style);
    }

    if let Some(after) = dom.node(id).computed_after()
        && after.position == crate::layout::Position::Static
        && let Some(ref text) = after.content
    {
        cursor_x = paint_text(
            buf,
            cursor_x,
            base_y,
            budget_right,
            text,
            style_from_computed(after),
        );
    }

    if let Some(href) = anchor_href_for(dom, id) {
        let width = cursor_x.saturating_sub(start_x);
        if width > 0 {
            buf.set_link_range(start_x, base_y, width, Some(&href));
        }
    }
}

/// Line-aware paint shared between IFC blocks and pure-text leaf
/// blocks. Iterates `layout.lines`, painting each fragment at its
/// computed `(x, line_index)`.
///
/// When `emit_pseudos` is true, the element's static-position
/// `::before` content rides line 0 (before fragments) and `::after`
/// content rides the last line (after fragments). Set to `false` by
/// the IFC path today — pseudo emission for IFC blocks is deferred
/// to a future slice (see `TECH_DEBT.md`).
#[allow(clippy::too_many_arguments)]
fn paint_lines(
    dom: &Dom<TuiExt>,
    id: NodeId,
    block_computed: &ComputedStyle,
    layout: &crate::render::inline::InlineLayout,
    inner: LayoutRect,
    buf: &mut Buffer,
    clip: Rect,
    emit_pseudos: bool,
) {
    let selection_range = dom.selection_range().filter(|r| !r.is_collapsed());

    let last_line_index = layout.lines.len().saturating_sub(1);

    for (line_index, line) in layout.lines.iter().enumerate() {
        let line_y = inner.y + line_index as i32;
        if line_y < clip.y as i32 || line_y >= clip.bottom() as i32 {
            continue;
        }
        if line_y < inner.y || line_y >= inner.y + inner.height as i32 {
            continue;
        }

        let mut leading_cursor: Option<u16> = None;

        // ::before on line 0.
        if emit_pseudos
            && line_index == 0
            && let Some(before) = dom.node(id).computed_before()
            && before.position == crate::layout::Position::Static
            && let Some(ref text) = before.content
        {
            let line_left = inner.x.max(clip.x as i32) as u16;
            let line_right = clip
                .right()
                .min((inner.x + inner.width as i32).max(0) as u16);
            let new_cursor = paint_text(
                buf,
                line_left,
                line_y as u16,
                line_right,
                text,
                style_from_computed(before),
            );
            leading_cursor = Some(new_cursor.saturating_sub(line_left));
        }

        for fragment in &line.fragments {
            let mut frag_x = inner.x + fragment.x as i32;
            // Shift fragments on line 0 over by the ::before width
            // (only meaningful for the pure-text path where line 0
            // fragment.x starts at 0).
            if let Some(shift) = leading_cursor {
                frag_x = frag_x.saturating_add(shift as i32);
            }
            if frag_x >= clip.right() as i32 {
                continue;
            }

            let computed = dom
                .node(fragment.node)
                .ext()
                .and_then(|e| e.computed.as_ref())
                .cloned()
                .unwrap_or_else(|| block_computed.clone());
            let style = if fragment.node == id {
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

            buf.set_stringn(start_x, line_y as u16, text_to_paint, max_width, style);

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

            if let Some(ref sr) = selection_range {
                apply_selection_overlay(dom, buf, line_y as u16, frag_x, clip, fragment, sr);
            }
        }

        // ::after on the last line, after the line's fragments.
        if emit_pseudos
            && line_index == last_line_index
            && let Some(after) = dom.node(id).computed_after()
            && after.position == crate::layout::Position::Static
            && let Some(ref text) = after.content
        {
            // Account for the ::before prefix on line 0 — fragments
            // are shifted by `leading_cursor`, so the line's visual
            // extent on screen ends at `inner.x + leading_cursor +
            // line.width`, not `inner.x + line.width`.
            let line_visual_end = line.width as i32 + leading_cursor.map(|c| c as i32).unwrap_or(0);
            let after_x = inner.x + line_visual_end;
            if after_x < clip.right() as i32 {
                let start_x = after_x.max(clip.x as i32) as u16;
                let budget_right = clip
                    .right()
                    .min((inner.x + inner.width as i32).max(0) as u16);
                if start_x < budget_right {
                    paint_text(
                        buf,
                        start_x,
                        line_y as u16,
                        budget_right,
                        text,
                        style_from_computed(after),
                    );
                }
            }
        }
    }

    // Anchor href tagging for whole-element anchors (e.g.
    // block-level `<a>` with text content and no inline descendants).
    // Per-fragment anchors are handled inside the loop.
    if emit_pseudos && let Some(href) = anchor_href_for(dom, id) {
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

/// Paint the inline flow of an IFC block from its pre-computed
/// `InlineLayout` (populated during the layout pass). Each fragment
/// paints with its owner element's cascaded style.
///
/// Phase D: line wrapping is honored — fragments span multiple lines
/// when `compute_inline_layout` produced a multi-line layout.
pub(super) fn paint_ifc(
    dom: &Dom<TuiExt>,
    id: NodeId,
    _block_computed: &ComputedStyle,
    inner: LayoutRect,
    buf: &mut Buffer,
    clip: Rect,
) {
    let Some(inline_layout) = dom.node(id).ext().and_then(|e| e.inline_layout.clone()) else {
        return;
    };

    // The current selection range (document-ordered) — computed once
    // per IFC paint, reused across fragments. `None` when there's no
    // selection or it's collapsed (caret only, nothing to highlight).
    let selection_range = dom.selection_range().filter(|r| !r.is_collapsed());

    for (line_index, line) in inline_layout.lines.iter().enumerate() {
        let line_y = inner.y + line_index as i32;
        if line_y < clip.y as i32 || line_y >= clip.bottom() as i32 {
            continue;
        }
        // Stop if this line sits past the block's content height —
        // `overflow: hidden` on the block is enforced by the caller's
        // clip rect (set in `paint_node` based on overflow mode).
        if line_y < inner.y || line_y >= inner.y + inner.height as i32 {
            continue;
        }

        for fragment in &line.fragments {
            let frag_x = inner.x + fragment.x as i32;
            if frag_x >= clip.right() as i32 {
                continue;
            }

            let computed = dom
                .node(fragment.node)
                .ext()
                .and_then(|e| e.computed.as_ref())
                .cloned()
                .unwrap_or_else(ComputedStyle::initial);
            // Fragments owned by the IFC block itself (text directly
            // inside the block) have their bg painted by the block's
            // `fill_bg`; using `style_from_computed` here would
            // double-apply the bg under opacity. Inline-child
            // fragments (`<span>` etc.) DO need their own bg in the
            // glyph style since they have no `fill_bg` of their own.
            let style = if fragment.node == id {
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

            buf.set_stringn(start_x, line_y as u16, text_to_paint, max_width, style);

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

    // Caret is painted by `paint_node` once per element that owns
    // an inline-flow container (IFC blocks AND pure-text leaf
    // blocks); the call used to live here, but textareas/inputs go
    // through `paint_inline_content` and never reach this function.
    // The hoist keeps both paint paths consistent.
}

/// If the focused element is editable and its collapsed-selection
/// caret falls inside the inline-flow container `id`, paint a
/// REVERSED cell at the caret's `(cell_x, cell_y)`. No-op otherwise.
pub(super) fn paint_caret_if_editable(dom: &Dom<TuiExt>, buf: &mut Buffer, id: NodeId, clip: Rect) {
    use crate::layout::{CaretColor, CaretTextColor};

    // Must have a collapsed selection (caret), not a range.
    let sel = match dom.selection() {
        Some(s) if s.is_collapsed() => s,
        _ => return,
    };
    let Some(focused) = dom.focused() else {
        return;
    };
    if crate::node::nearest_editable_ancestor(dom, focused).is_none() {
        return;
    };
    let caret_ifc = inline_flow_container(dom, sel.focus.node);
    if caret_ifc != Some(id) {
        return;
    }
    // Resolve caret colors from cascade. `caret-color: transparent`
    // suppresses paint; `Auto` means "derive from the underlying
    // cell's existing fg/bg" (classic swap visual).
    let computed = match dom.node(focused).ext().and_then(|e| e.computed.as_ref()) {
        Some(c) => c,
        None => return,
    };
    if matches!(computed.caret_color, CaretColor::Transparent) {
        return;
    }
    let Some((x, y)) = crate::runtime::editing::caret::cell_of_position(dom, sel.focus) else {
        return;
    };
    if x < clip.x || x >= clip.right() || y < clip.y || y >= clip.bottom() {
        return;
    }

    // `Auto` caret colors derive from the focused element's
    // CASCADED `color` and `background-color` — NOT the cell's
    // currently-painted values. For an empty cell (no glyph painted
    // yet), the painted fg is `Color::Reset`, which would give an
    // invisible caret. Using cascade values gives the predictable
    // "swap text-color and bg-color" visual that authors expect.
    //
    // Fallback: if the cascade resolved to `Color::Reset` (no
    // explicit value, terminal-default), the caret would be
    // invisible (the cell paints as default-on-default). Substitute
    // a sensible high-contrast default — White for the bg-side,
    // Black for the fg-side — so an unstyled textarea/input still
    // shows a visible caret. Authors override via `caret-color` /
    // `caret-text-color`.
    let resolve_reset_fg = |c: crate::Color| match c {
        crate::Color::Reset => crate::Color::Rgb(0xFF, 0xFF, 0xFF),
        other => other,
    };
    let resolve_reset_bg = |c: crate::Color| match c {
        crate::Color::Reset => crate::Color::Rgb(0x00, 0x00, 0x00),
        other => other,
    };
    let cascaded_fg = resolve_reset_fg(computed.fg);
    let cascaded_bg = resolve_reset_bg(computed.bg);
    let under_mod = buf.cell(x, y).map(|c| c.modifier).unwrap_or_default();

    let caret_bg = match &computed.caret_color {
        CaretColor::Auto => cascaded_fg,
        CaretColor::Transparent => return, // already handled above
        CaretColor::Color(tc) => match tc {
            crate::TuiColor::Literal(c) => *c,
            crate::TuiColor::Var { .. } => cascaded_fg,
        },
    };
    let caret_fg = match &computed.caret_text_color {
        CaretTextColor::Auto => cascaded_bg,
        CaretTextColor::Color(tc) => match tc {
            crate::TuiColor::Literal(c) => *c,
            crate::TuiColor::Var { .. } => cascaded_bg,
        },
    };

    let mut new_style = Style::new().fg(caret_fg).bg(caret_bg);
    // Preserve non-color modifiers (bold/italic etc.) that were on
    // the underlying cell so the caret doesn't strip them. `Modifier`
    // is a bitflag — re-add each set bit.
    for m in [
        Modifier::BOLD,
        Modifier::ITALIC,
        Modifier::UNDERLINED,
        Modifier::SLOW_BLINK,
        Modifier::RAPID_BLINK,
        Modifier::HIDDEN,
        Modifier::CROSSED_OUT,
    ] {
        if under_mod.contains(m) {
            new_style = new_style.add_modifier(m);
        }
    }
    buf.set_style(x, y, new_style);
}

/// Overlay the selection style on cells of `fragment` whose source
/// bytes fall within `range`. Preserves the underlying symbols.
///
/// Author `::selection` styling: walks up from the fragment's text
/// node to the nearest ancestor element with `computed_selection`
/// set and uses that style (fg/bg/modifiers). When no ancestor has
/// a cascaded selection style, falls back to the v1 default
/// transparent overlay (no visual change). The UA's
/// `*::selection { bg: #394B7E; fg: white }` rule ensures the
/// fallback rarely fires.
fn apply_selection_overlay(
    dom: &Dom<TuiExt>,
    buf: &mut Buffer,
    line_y: u16,
    frag_x: i32,
    clip: Rect,
    fragment: &InlineFragment,
    range: &Range,
) {
    let Some((byte_start, byte_end)) = selection_byte_range_in(dom, range, fragment.text_node)
    else {
        return;
    };

    // Intersect with the fragment's source byte window.
    let frag_start = fragment.source_byte_offset;
    let frag_end = fragment.source_byte_offset + fragment.text.len();
    let local_start = byte_start.max(frag_start);
    let local_end = byte_end.min(frag_end);
    if local_start >= local_end {
        return;
    }

    // Byte offsets within the fragment's own text.
    let off_start = local_start - frag_start;
    let off_end = local_end - frag_start;

    // Map byte offsets → visible cell offsets inside the fragment.
    let cell_start = cells_before_byte(&fragment.text, off_start);
    let cell_end = cells_before_byte(&fragment.text, off_end);
    if cell_start >= cell_end {
        return;
    }

    // Author `::selection` cascade overrides the UA default.
    // The UA `*::selection { bg: #394B7E; fg: white }` rule means
    // every focusable always has a computed_selection style — so
    // this fallback only fires if an author *explicitly* removes
    // the UA rule via `*::selection { background-color: initial; }`
    // or similar. In that case we paint nothing (Style::new()).
    let overlay = match nearest_selection_style(dom, fragment.text_node) {
        Some(c) => style_from_computed(c),
        None => Style::new(),
    };
    for c in cell_start..cell_end {
        let x = (frag_x + c as i32) as u16;
        if x < clip.x || x >= clip.right() {
            continue;
        }
        buf.set_style(x, line_y, overlay);
    }
}

/// Walk up from `text_node` to the nearest ancestor element whose
/// cascade produced a `::selection` computed style. Returns `None`
/// if no ancestor has one.
fn nearest_selection_style(dom: &Dom<TuiExt>, text_node: NodeId) -> Option<&ComputedStyle> {
    let mut cur = dom.node(text_node).parent_node().map(|p| p.id());
    while let Some(id) = cur {
        if let Some(ext) = dom.node(id).ext()
            && let Some(sel) = ext.computed_selection.as_ref()
        {
            return Some(sel);
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
}

/// Byte range within `text_node`'s data that falls inside the
/// document-ordered selection `range`. `None` when `text_node` sits
/// fully outside the range. Handles:
///
/// - range entirely within one text node (start == end == text_node)
/// - range starts in `text_node` (end is elsewhere in the tree)
/// - range ends in `text_node` (start is elsewhere)
/// - `text_node` is strictly between start and end in document order —
///   entire text node is selected
fn selection_byte_range_in(
    dom: &Dom<TuiExt>,
    range: &Range,
    text_node: NodeId,
) -> Option<(usize, usize)> {
    let is_start = range.start.node == text_node;
    let is_end = range.end.node == text_node;

    if is_start && is_end {
        return Some((range.start.offset, range.end.offset));
    }
    let text_len = dom
        .node(text_node)
        .node_value()
        .map(|s| s.len())
        .unwrap_or(0);
    if is_start {
        return Some((range.start.offset, text_len));
    }
    if is_end {
        return Some((0, range.end.offset));
    }

    let after_start = {
        let p = dom.compare_document_position(range.start.node, text_node);
        p.contains(DocumentPosition::FOLLOWING) || p.contains(DocumentPosition::CONTAINS)
    };
    let before_end = {
        let p = dom.compare_document_position(range.end.node, text_node);
        p.contains(DocumentPosition::PRECEDING) || p.contains(DocumentPosition::CONTAINS)
    };
    if after_start && before_end {
        Some((0, text_len))
    } else {
        None
    }
}

/// Count visible cells before byte offset `target` in `text`. If
/// `target` falls between graphemes, returns the cells up to that
/// boundary. Mid-grapheme targets round up to the next boundary
/// (shouldn't happen — selection offsets land on grapheme edges).
fn cells_before_byte(text: &str, target: usize) -> u16 {
    let mut cells: u16 = 0;
    for (idx, g) in text.grapheme_indices(true) {
        if idx >= target {
            return cells;
        }
        cells = cells.saturating_add(UnicodeWidthStr::width(g) as u16);
    }
    cells
}

/// Skip `cells` worth of grapheme width from the front of `text`,
/// returning the remaining slice. Used when a fragment starts off
/// the left edge of the paint clip.
fn advance_text_by_cells(text: &str, cells: u16) -> &str {
    let mut consumed: u16 = 0;
    let mut byte_pos: usize = 0;
    for (idx, g) in text.grapheme_indices(true) {
        if consumed >= cells {
            byte_pos = idx;
            return &text[byte_pos..];
        }
        let w = UnicodeWidthStr::width(g) as u16;
        consumed = consumed.saturating_add(w);
        byte_pos = idx + g.len();
    }
    &text[byte_pos..]
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

/// Return the selected-option label to paint as a closed
/// `<select>` dropdown's own text — the option elements
/// themselves are hidden by UA `display: none`, so the runtime
/// echoes the selected label into the select's content area
/// instead. The dropdown affordance (`▾` chevron at the right
/// edge) is supplied separately by the UA
/// `select:not([multiple]):not([size]):not([data-rdom-open])::after`
/// rule, NOT prepended here.
///
/// `None` for any non-select, for listbox-mode selects (which
/// paint their options directly), and for open dropdowns
/// (where the option children paint themselves via the normal
/// traversal).
fn select_chrome_text(dom: &Dom<TuiExt>, id: NodeId) -> Option<String> {
    if dom.node(id).tag_name() != Some("select") {
        return None;
    }
    if !crate::runtime::builtins::select::is_dropdown(dom, id) {
        return None;
    }
    if crate::runtime::builtins::select::is_open(dom, id) {
        return None;
    }
    let selected = crate::runtime::builtins::select::selected_options(dom, id);
    let label = selected
        .first()
        .map(|&opt| crate::runtime::builtins::select::option_label(dom, opt))
        .unwrap_or_default();
    Some(label)
}

/// True iff `id` is `<input type="password">`. Tagged separately
/// from `mask_if_password` so the paint dispatch can branch on it
/// without doing the masking work twice.
fn is_password_input(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    node.tag_name() == Some("input") && node.get_attribute("type") == Some("password")
}

/// If `id` is `<input type="password">`, replace every grapheme
/// cluster in `text` with the bullet character `•`. Otherwise
/// return `text` unchanged. The bullet is a single-cell glyph in
/// monospace fonts, so the masked width matches the original
/// grapheme-count (not display-width — wide chars mask to a
/// single bullet, matching browser behavior).
fn mask_if_password(dom: &Dom<TuiExt>, id: NodeId, text: String) -> String {
    if !is_password_input(dom, id) {
        return text;
    }
    text.graphemes(true).map(|_| "\u{2022}").collect()
}
