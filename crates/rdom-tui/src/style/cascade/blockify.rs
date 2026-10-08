//! Blockification (CSS Display 3 §2.7): the computed `display` of a flex
//! or grid item — a child box of a flex container (CSS Flexbox §4) or a
//! grid container (CSS Grid 2 §6.1) — is block-level. Elements
//! (`element`) and `::before` / `::after` (`pseudo`) ask
//! [`children_are_items`] of their parent and [`blockify`].

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Display, Flow};
use crate::style::ComputedStyle;

/// CSS Display 3 §2.7: blockify a box whose parent box is a flex or grid
/// container (CSS Flexbox §4, CSS Grid 2 §6.1: "the `display` value of a
/// grid item is blockified"). An inline-level outer display type becomes
/// `block`, the inner type kept — `inline` → `block`, `inline-flex` →
/// `flex`, `inline-grid` → `grid`, `inline flow-root` → `flow-root` — and
/// `inline-block`, rdom's `inline
/// flow-root`, becomes `block flow-root`. `block`, `contents` and `none`
/// are unaffected; the `list-item` flag is kept.
pub(super) fn blockify(working: &mut ComputedStyle) {
    match working.display {
        Display::Inline => working.display = Display::Block,
        Display::InlineBlock => {
            working.display = Display::Block;
            if working.flow == Flow::Block {
                working.flow = Flow::FlowRoot;
            }
        }
        // §2.7: a layout-internal box becomes `block` — block flow, its
        // table role gone (a cell's `flow-root` with it).
        Display::TablePart(_) => {
            working.display = Display::Block;
            working.flow = Flow::Block;
        }
        Display::Block | Display::None | Display::Contents => {}
    }
}

/// CSS Position 4 (top layer): an element in the document's top layer
/// whose `position` is not `absolute` or `fixed` computes to `absolute`
/// — it is laid out against the initial containing block and painted
/// above the document (`layout_pass::positioning`, `paint_pass`), so it
/// is never in flow.
pub(super) fn finalize_top_layer(working: &mut ComputedStyle, in_top_layer: bool) {
    use crate::layout::Position;
    if in_top_layer && !matches!(working.position, Position::Absolute | Position::Fixed) {
        working.position = Position::Absolute;
    }
}

/// CSS 2.1 §9.7: an absolutely positioned box does not float — its
/// computed `float` is `none` — and a floated box is blockified (§9.7's
/// table: an inline-level box becomes the block-level one, as
/// [`blockify`]). A box-less (`display: contents`) element has no box to
/// float; its `float` computes as written and applies to nothing.
pub(super) fn finalize_float(working: &mut ComputedStyle) {
    use crate::layout::{Float, Position};
    if matches!(working.position, Position::Absolute | Position::Fixed) {
        working.float = Float::None;
        // CSS 2.1 §9.7: an absolutely positioned table part is no part
        // of a table — a `block` box (the other display values keep
        // their box, placed by `positioning`).
        if matches!(working.display, crate::layout::Display::TablePart(_)) {
            blockify(working);
        }
    } else if working.float != Float::None {
        blockify(working);
    }
}

/// Whether the children of `parent` (styled `parent_computed`) are flex
/// or grid items: their parent box — `parent`, or for a box-less
/// (`display: contents`, CSS Display 3 §2.5) parent its nearest ancestor
/// with a box — is a flex or grid container. A non-element parent (the
/// document root's fragment) is no flex container: rdom's viewport column
/// only stands in for a browser's `<body>`, whose children are not flex
/// items.
pub(crate) fn children_are_items(
    dom: &Dom<TuiExt>,
    parent: Option<NodeId>,
    parent_computed: &ComputedStyle,
) -> bool {
    let Some(mut id) = parent else {
        return false;
    };
    let mut computed = parent_computed;
    loop {
        if dom.node(id).node_type() != NodeType::Element {
            return false;
        }
        if computed.display != Display::Contents {
            return computed.flow.is_flex_or_grid();
        }
        // The ancestors' styles are written: the walk is top-down. A
        // `display: contents` box passes its parent's role on — its parent
        // in the box tree: a `::details-content` box's is its `<details>`.
        let Some(next) = super::details::box_parent(dom, id) else {
            return false;
        };
        let Some(next_computed) = dom.node(next).ext().and_then(|e| e.computed.as_deref()) else {
            return false;
        };
        id = next;
        computed = next_computed;
    }
}
