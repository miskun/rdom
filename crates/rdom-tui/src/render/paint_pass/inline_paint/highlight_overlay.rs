//! The highlight overlays (CSS Pseudo-Elements 4 §3, CSS Custom
//! Highlight API 1 §5): the cells of a painted fragment whose source
//! bytes fall in a highlight's ranges are restyled — its symbols kept, so
//! a repaint without the highlight restores them — layer by layer:
//! the registered highlights (`Dom::highlights`) by priority, then
//! registration order (§5.2), each in its `::highlight(name)` style, and
//! the selection last, the topmost layer (§3.5), in its `::selection`
//! style. One path for both.
//!
//! A highlight style applies only the properties a highlight
//! pseudo-element takes (§3.2: color, background, decorations — the rules
//! were cut to them, `TuiStyle::highlight_subset`), and of those only the
//! ones it sets: a highlight that leaves `color` alone keeps the text's
//! own. rdom keeps computed values, so "sets" is "differs from the
//! originating element's" (DIVERGENCES §2). The style is the nearest
//! ancestor's of the text with one for that highlight (§3.5's highlight
//! inheritance, approximated). The used `user-select` (`style::user_select`)
//! excludes text from the selection's layer only.

use std::sync::Arc;

use rdom_core::{Dom, NodeId, Position, Range};

use crate::ext::TuiExt;
use crate::render::inline::InlineFragment;
use crate::render::paint_pass::text::decoration_style;
use crate::render::{Buffer, Rect, Style};
use crate::style::ComputedStyle;

/// What one overlay layer paints in.
#[derive(Debug, Clone)]
enum Source {
    /// The registered highlight of that name.
    Highlight(Arc<str>),
    /// The document selection.
    Selection,
}

/// One range of one layer.
#[derive(Debug, Clone)]
struct Layer {
    range: Range,
    source: Source,
}

/// The overlay layers of one paint, bottom first.
#[derive(Debug, Default)]
pub(super) struct Overlays {
    layers: Vec<Layer>,
}

impl Overlays {
    /// The document's overlays: each registered highlight's ranges, by
    /// priority then registration order, then the selection when it is
    /// not collapsed. `None` when there are none.
    pub(super) fn of(dom: &Dom<TuiExt>) -> Option<Self> {
        let selection = dom.selection_range().filter(|r| !r.is_collapsed());
        let registry = dom.highlights();
        if registry.is_empty() && selection.is_none() {
            return None;
        }
        let mut order: Vec<(i32, usize, &str, &rdom_core::Highlight)> = registry
            .iter()
            .enumerate()
            .map(|(k, (name, h))| (h.priority, k, name, h))
            .collect();
        order.sort_by_key(|&(priority, k, ..)| (priority, k));
        let mut layers = Vec::new();
        for (.., name, h) in order {
            let name: Arc<str> = name.into();
            for range in h.ranges() {
                layers.push(Layer {
                    range: range.clone(),
                    source: Source::Highlight(name.clone()),
                });
            }
        }
        if let Some(range) = selection {
            layers.push(Layer {
                range,
                source: Source::Selection,
            });
        }
        Some(Overlays { layers })
    }

    /// Paint every layer over `fragment`, drawn at `frag_x` on row
    /// `line_y` inside `clip`.
    pub(super) fn paint(
        &self,
        dom: &Dom<TuiExt>,
        buf: &mut Buffer,
        line_y: u16,
        frag_x: i32,
        clip: Rect,
        fragment: &InlineFragment,
    ) {
        for layer in &self.layers {
            paint_layer(dom, buf, line_y, frag_x, clip, fragment, layer);
        }
    }
}

/// Overlay `layer`'s style on the cells of `fragment` whose source bytes
/// fall within its range. Preserves the underlying symbols.
fn paint_layer(
    dom: &Dom<TuiExt>,
    buf: &mut Buffer,
    line_y: u16,
    frag_x: i32,
    clip: Rect,
    fragment: &InlineFragment,
    layer: &Layer,
) {
    let Some((byte_start, byte_end)) = range_bytes_in(dom, &layer.range, fragment.text_node) else {
        return;
    };

    // Intersect with the fragment's source byte window.
    let frag_start = fragment.source_byte_offset;
    let frag_end = fragment.source_byte_offset + fragment.source_len();
    let local_start = byte_start.max(frag_start);
    let local_end = byte_end.min(frag_end);
    if local_start >= local_end {
        return;
    }

    // Byte offsets within the fragment's own text.
    let off_start = local_start - frag_start;
    let off_end = local_end - frag_start;

    // Map byte offsets → visible cell offsets inside the fragment.
    let cell_start = fragment.cells_before_source(off_start);
    let cell_end = fragment.cells_before_source(off_end);
    if cell_start >= cell_end {
        return;
    }

    let style = match &layer.source {
        Source::Selection => {
            // `user-select: none` content is excluded from a selection's
            // highlight (and from copy) even when the selection spans
            // across it — e.g. dragging from a title down through a
            // `user-select: none` chrome bar into the body must not paint
            // the bar. Browsers skip such content; so do we. Checked after
            // the byte-range tests: it walks the ancestors, and most
            // fragments lie outside the selection.
            if crate::style::user_select::is_unselectable(dom, fragment.text_node) {
                return;
            }
            // The UA's `*::selection` rule gives every element a style;
            // an author who removes it gets no overlay.
            nearest(dom, fragment.text_node, |e| e.computed_selection.as_deref())
        }
        Source::Highlight(name) => nearest(dom, fragment.text_node, |e| e.computed_highlight(name)),
    };
    let Some((highlight, host)) = style else {
        return;
    };
    let overlay = overlay_style(highlight, host);
    for c in cell_start..cell_end {
        // A cell left of the screen (an overflowing `rtl` line) is not
        // painted.
        let Ok(x) = u16::try_from(frag_x + c as i32) else {
            continue;
        };
        if x < clip.x || x >= clip.right() {
            continue;
        }
        buf.set_style(x, line_y, overlay);
    }
}

/// The paint a highlight styled `highlight`, on text of an element styled
/// `host`, lays over a cell: its color, background and decorations where
/// it sets them (CSS Pseudo-Elements 4 §3.2).
fn overlay_style(highlight: &ComputedStyle, host: &ComputedStyle) -> Style {
    let mut style = Style::new();
    if highlight.fg != host.fg && highlight.fg != crate::style::Color::Reset {
        style = style.fg(highlight.fg);
    }
    if crate::render::paint_pass::fills(highlight.bg) {
        style = style.bg(highlight.bg);
    }
    if highlight.applied_decorations != host.applied_decorations {
        style = decoration_style(style, highlight);
    }
    style
}

/// The style `of` gives the nearest element at or above `text_node`'s
/// parent that has one, with that element's own style.
fn nearest<'d>(
    dom: &'d Dom<TuiExt>,
    text_node: NodeId,
    of: impl Fn(&'d TuiExt) -> Option<&'d ComputedStyle>,
) -> Option<(&'d ComputedStyle, &'d ComputedStyle)> {
    let mut cur = dom.node(text_node).parent_node().map(|p| p.id());
    while let Some(id) = cur {
        if let Some(ext) = dom.node(id).ext()
            && let Some(style) = of(ext)
        {
            return Some((style, ext.computed.as_deref()?));
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
}

/// Byte range within `text_node`'s data that falls inside the
/// document-ordered `range`. `None` when `text_node` sits
/// fully outside the range. Handles:
///
/// - range entirely within one text node (start == end == text_node)
/// - range starts in `text_node` (end is elsewhere in the tree)
/// - range ends in `text_node` (start is elsewhere)
/// - `text_node` is strictly between start and end in document order —
///   entire text node is selected
fn range_bytes_in(dom: &Dom<TuiExt>, range: &Range, text_node: NodeId) -> Option<(usize, usize)> {
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

    // Whole-node membership is a boundary-point comparison (DOM §5.2):
    // the text is selected when `start <= (text, 0)` and
    // `(text, len) <= end`. This orders an element position `(el, k)`
    // by its offset against the child index, so a range that ends at
    // `(parent, 0)` does not swallow the text under `parent`'s children.
    use std::cmp::Ordering;
    let after_start = matches!(
        dom.compare_boundary_points(range.start, Position::new(text_node, 0)),
        Some(Ordering::Less | Ordering::Equal)
    );
    let before_end = matches!(
        dom.compare_boundary_points(Position::new(text_node, text_len), range.end),
        Some(Ordering::Less | Ordering::Equal)
    );
    if after_start && before_end {
        Some((0, text_len))
    } else {
        None
    }
}
