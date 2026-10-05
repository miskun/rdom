//! Blockification (CSS Display 3 §2.7): the computed `display` of a flex
//! item — a child box of a flex container (CSS Flexbox §4) — is
//! block-level. Elements (`walk`) and `::before` / `::after` (`pseudo`)
//! ask [`children_are_flex_items`] of their parent and [`blockify`].

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Display, Flow};
use crate::style::ComputedStyle;

/// CSS Display 3 §2.7: blockify a box whose parent box is a flex
/// container (CSS Flexbox §4: "the `display` value of a flex item is
/// blockified"). An inline-level outer display type becomes `block`, the
/// inner type kept — `inline` → `block`, `inline-flex` → `flex`, `inline
/// flow-root` → `flow-root` — and `inline-block`, rdom's `inline
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
        Display::Block | Display::None | Display::Contents => {}
    }
}

/// Whether the children of `parent` (styled `parent_computed`) are flex
/// items: their parent box — `parent`, or for a box-less (`display:
/// contents`, CSS Display 3 §2.5) parent its nearest ancestor with a
/// box — is a flex container. A non-element parent (the document root's
/// fragment) is no flex container: rdom's viewport column only stands in
/// for a browser's `<body>`, whose children are not flex items.
pub(super) fn children_are_flex_items(
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
            return computed.flow == Flow::Flex;
        }
        // The ancestors' styles are written: the walk is top-down.
        let Some(next) = dom.node(id).parent_node().map(|p| p.id()) else {
            return false;
        };
        let Some(next_computed) = dom.node(next).ext().and_then(|e| e.computed.as_deref()) else {
            return false;
        };
        id = next;
        computed = next_computed;
    }
}
