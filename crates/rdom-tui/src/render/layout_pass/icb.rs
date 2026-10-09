//! The document laid out in the initial containing block (CSS 2.1 §10.1):
//! a block container the viewport's size that establishes a block
//! formatting context (§9.4.1; `box_tree::icb`).
//!
//! - A root fragment (the default): its children are the ICB's — laid out
//!   in block flow (`block::layout_block_children`), as a browser lays
//!   out `<body>`'s: block boxes stacked with their vertical margins
//!   collapsing (§8.3.1), floats floating (§9.5), inline-level children
//!   and text in the line boxes of anonymous block boxes (§9.2.1.1). Its
//!   height is definite — the viewport's — so a child's percentage height
//!   resolves against it (§10.5); each child's own `auto` height is its
//!   content's (§10.6.3).
//! - An element root (`Dom::with_root_tag`): the root element, laid out as
//!   a block in the ICB — the viewport's width less its margins, its
//!   height its content's unless set (`html { height: 100% }` fills it).
//!
//! Nothing here sizes a top-level box to the viewport: the canvas does
//! that for the root element's background (CSS Backgrounds 3 §2.11.2,
//! `paint_pass::canvas`).

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;

/// Lay out the tree rooted at `root` in the initial containing block
/// `icb` (the viewport).
pub(super) fn lay_out(dom: &mut Dom<TuiExt>, root: NodeId, icb: LayoutRect) {
    match dom.node(root).node_type() {
        NodeType::Element => {
            super::float::enter(dom);
            super::block::layout_root_element(dom, root, icb);
            let _ = super::float::leave(dom);
        }
        NodeType::Fragment => {
            let style = crate::render::box_tree::icb::style(dom);
            super::float::enter(dom);
            let _ = super::block::layout_block_children(dom, root, icb, &style);
            let _ = super::float::leave(dom);
            super::collapse_hidden_children(dom, root, icb);
        }
        _ => {}
    }
}
