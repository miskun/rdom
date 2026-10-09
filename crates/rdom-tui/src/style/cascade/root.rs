//! The root fragment as the CSS root element (C14G-ROOT-ELEMENT;
//! DIVERGENCES §2, `Dom::root()`). A fragment root stands for the
//! document and for its root element: selectors match it as an element
//! with no name, attributes or classes (rdom-core), and the cascade styles
//! it before its children — `:root { color: … }` reaches the page by
//! inheritance, a `:root` custom property inside `@media` follows its
//! condition, `rlh` reads its line height, and its background is the
//! canvas's (`render::paint_pass::canvas`).
//!
//! Its box is the initial containing block (`render::box_tree::icb`), so
//! the style kept for it is the ICB's — a `flow-root` block, every box
//! property initial — with the root's inherited properties, custom
//! properties and background: what its children inherit, what the ICB
//! lays its inline content out with, and what the canvas paints. Its
//! margins, padding, border, sizes, `display` and the like do not apply,
//! nor do transitions (it is no element the animation engine tracks). It
//! is kept as document data, read through `TuiNodeExt::computed` on the
//! root like an element's.

use std::rc::Rc;

use rdom_core::{Dom, NodeId, NodeType};

use super::element::compute_element_style;
use super::matching::Rules;
use super::walk::{CounterState, ElementCx, Scratch, Sheets};
use crate::ext::TuiExt;
use crate::style::ComputedStyle;

/// The root fragment's style (document data).
#[derive(Debug)]
struct RootStyle(Rc<ComputedStyle>);

/// Whether `id` is the tree's root fragment.
pub(crate) fn is_root_fragment(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    id == dom.root() && dom.node(id).node_type() == NodeType::Fragment
}

/// The root fragment's style as last cascaded (module doc); `None` before
/// the first cascade, and for an element root (whose style is its own).
pub(crate) fn style(dom: &Dom<TuiExt>) -> Option<&Rc<ComputedStyle>> {
    dom.document_data::<RootStyle>().map(|r| &r.0)
}

/// The style of the initial containing block before any cascade: a
/// `flow-root` block container (CSS Display 3 §2: it establishes a block
/// formatting context, CSS 2.1 §9.4.1), every other property initial.
pub(crate) fn icb_style() -> ComputedStyle {
    let mut icb = ComputedStyle::initial();
    icb.flow = crate::layout::Flow::FlowRoot;
    icb.establishes_new_bfc = true;
    icb
}

/// Cascade the root fragment against `sheets` from `seed` (the initial
/// values and the sheets' `define_var` variables): its rules matched and
/// its ladder run as an element's, then kept as the ICB's style (module
/// doc). Returns what its children inherit. `true` with it when the
/// root's line height moved — every `rlh` must follow.
pub(super) fn cascade<'a>(
    dom: &mut Dom<TuiExt>,
    sheets: &Sheets<'a>,
    seed: &ComputedStyle,
    counters: &mut CounterState,
    scratch: &mut Scratch<'a>,
) -> (Rc<ComputedStyle>, bool) {
    let id = dom.root();
    let computed = {
        let mut cx = ElementCx {
            dom: &*dom,
            sheets,
            id,
            counters,
            scratch,
        };
        compute_element_style(&mut cx, seed, None, Rules::Match)
    };
    let mut used = icb_style();
    super::inherit::inherit_inheritable_from(&mut used, &computed);
    // The background is the canvas's (CSS Backgrounds 3 §2.11.2).
    used.bg = computed.bg;
    let moved =
        style(dom).is_some_and(|prev| prev.text.line_height.rows() != used.text.line_height.rows());
    let used = Rc::new(used);
    match dom.document_data_mut::<RootStyle>() {
        Some(slot) => slot.0 = used.clone(),
        None => {
            dom.set_document_data(RootStyle(used.clone()));
        }
    }
    (used, moved)
}
