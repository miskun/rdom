//! The initial containing block (CSS 2.1 §10.1): the box the document's
//! top-level boxes are laid out in — the viewport's size, a block
//! container holding a block formatting context (§9.4.1).
//!
//! rdom's default root is a document fragment, which has no `TuiExt`:
//! it stands for the document, and its box is the ICB. Its children lay
//! out in block flow in it as a browser's `<body>` children do — block
//! boxes stacked with their margins collapsing, floats, inline-level
//! children and text in line boxes of anonymous block boxes (§9.2.1.1).
//! Those anonymous boxes belong to the ICB and are kept here, as document
//! data, where an element keeps its own in `TuiExt::anonymous_blocks`;
//! [`anonymous_blocks`] is the one read of either.
//!
//! An element root (`Dom::with_root_tag`) is the root element: laid out
//! as a block in a ICB that has no node.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{AnonymousIfc, TuiExt};
use crate::style::ComputedStyle;

/// The anonymous block boxes of the document root's lines (document data
/// for the layout pass and the paints and hit tests after it).
#[derive(Debug, Default)]
struct RootBoxes(Vec<AnonymousIfc>);

/// Whether `id` is the document root fragment: the box of the initial
/// containing block.
pub(crate) fn is_icb(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    id == dom.root() && dom.node(id).node_type() == NodeType::Fragment
}

/// The style the initial containing block lays its children out with: a
/// `flow-root` block container (CSS Display 3 §2: it establishes a block
/// formatting context), every other property initial — the document's
/// children inherit nothing from it either.
pub(crate) fn style() -> ComputedStyle {
    let mut icb = ComputedStyle::initial();
    icb.flow = crate::layout::Flow::FlowRoot;
    icb.establishes_new_bfc = true;
    icb
}

/// The anonymous block boxes `id`'s inline runs are laid out in: an
/// element's own, the document root's kept for the ICB, none for any
/// other node.
pub(crate) fn anonymous_blocks(dom: &Dom<TuiExt>, id: NodeId) -> &[AnonymousIfc] {
    if is_icb(dom, id) {
        return dom
            .document_data::<RootBoxes>()
            .map_or(&[], |b| b.0.as_slice());
    }
    dom.node(id)
        .ext()
        .map_or(&[], |e| e.anonymous_blocks.as_slice())
}

/// Keep `boxes` as `id`'s anonymous block boxes: an element's own, or the
/// document root's for the ICB.
pub(crate) fn set_anonymous_blocks(dom: &mut Dom<TuiExt>, id: NodeId, boxes: Vec<AnonymousIfc>) {
    if is_icb(dom, id) {
        match dom.document_data_mut::<RootBoxes>() {
            Some(kept) => kept.0 = boxes,
            None => {
                dom.set_document_data(RootBoxes(boxes));
            }
        }
    } else if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.anonymous_blocks = boxes;
    }
}
