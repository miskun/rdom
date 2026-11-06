//! An element's boxes as the last layout placed them (CSS Box Model 3 §2,
//! CSS Overflow 3 §3): [`box_model`].

use rdom_core::{NodeId, NodeType};

use crate::TuiDom;
use crate::layout::{Display, LayoutRect};
use crate::node::TuiNodeExt;

/// An element's boxes, in viewport cells, as the last layout placed and
/// the frame painted them.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BoxModel {
    /// The border box.
    pub border: LayoutRect,
    /// The padding box: the border box less the border.
    pub padding: LayoutRect,
    /// The content box, where the layout placed one (a block or flex
    /// container's; an inline box has none of its own).
    pub content: Option<LayoutRect>,
    /// The scrollport — the padding box less the scrollbar gutters — and
    /// the scroll state, for a scroll container.
    pub scroll: Option<ScrollBox>,
    /// The border box's fragments, for a box a fragmented flow split
    /// (one per fragmentainer); empty for a box in one piece.
    pub fragments: Vec<LayoutRect>,
}

/// A scroll container's scrollport and scroll state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScrollBox {
    /// The scrollport.
    pub port: LayoutRect,
    /// How far it is scrolled, `(left, top)`.
    pub offset: (i32, i32),
    /// The scrollable overflow's size, `(width, height)`.
    pub size: (u32, u32),
}

/// `id`'s boxes from the last layout; `None` for a node that generates no
/// box (not an element, `display: none` or `contents`, inside skipped
/// contents) or was never laid out.
pub fn box_model(dom: &TuiDom, id: NodeId) -> Option<BoxModel> {
    let node = dom.node(id);
    if node.node_type() != NodeType::Element || !crate::node::is_rendered(dom, id) {
        return None;
    }
    let computed = node.computed()?;
    if matches!(computed.display, Display::None | Display::Contents) {
        return None;
    }
    let ext = node.ext()?;
    // A non-atomic inline box has no rect of its own, only fragments in
    // its block's lines: its border box is their bounding box (CSSOM View
    // §6.1, as `getBoundingClientRect()`).
    let inline = computed.display == Display::Inline && !computed.is_atomic_inline();
    let border = if inline {
        crate::render::inline::inline_box_rect(dom, id)?
    } else {
        ext.layout
    };
    let padding =
        crate::render::layout_pass::geometry::compute_padding_box(border, computed.border);
    // An inline box has no content box of its own (CSS Display 3 §2):
    // only a block, flex, grid or atomic box's layout places one.
    let content = if inline {
        None
    } else {
        node.content_layout_rect()
    };
    let scroll = computed.is_scroll_container().then(|| ScrollBox {
        port: crate::render::layout_pass::scrollport::scrollport_of(ext, computed),
        offset: (ext.scroll_x, ext.scroll_y),
        size: (
            u32::try_from(ext.scroll_content_width).unwrap_or(u32::MAX),
            u32::try_from(ext.scroll_content_height).unwrap_or(u32::MAX),
        ),
    });
    let fragments = ext
        .box_fragments()
        .map(|f| f.drawn(ext.layout).map(|d| d.rows).collect())
        .unwrap_or_default();
    Some(BoxModel {
        border,
        padding,
        content,
        scroll,
        fragments,
    })
}
