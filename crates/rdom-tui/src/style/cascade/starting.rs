//! An element's starting style (CSS Transitions 2 §3, `@starting-style`):
//! the style an element with no before-change style — newly rendered —
//! takes for its transitions to start from. It is the element's style
//! computed with the `@starting-style` rules applying too, inheriting from
//! its parent's computed style; it is never stored, the transition engine
//! reads it once (`runtime::animation::diff_and_register_with`).

use std::rc::Rc;

use rdom_core::{Dom, NodeId};

use super::matching::Recorder;
use super::registered::PropertyRegistry;
use super::walk::{self, CounterState, ElementCx, Scratch, Sheets};
use crate::ext::TuiExt;
use crate::style::{ComputedStyle, PseudoElementTarget, Stylesheet};

/// `id`'s starting style under `stylesheets` (with their registrations),
/// or `None` when no `@starting-style` rule matches it — then it has no
/// starting style, and its newly rendered values change at once.
pub(crate) fn starting_style(
    dom: &Dom<TuiExt>,
    (stylesheets, registry): (&[&Stylesheet], &Rc<PropertyRegistry>),
    id: NodeId,
) -> Option<ComputedStyle> {
    let sheets = Sheets::new(
        stylesheets,
        registry.clone(),
        super::document_viewport(dom),
        super::document_color_scheme(dom),
    )
    .with_starting_style();
    if !sheets.has_starting_rules() {
        return None;
    }
    let mut scratch = Scratch::default();
    let mut counters = CounterState::default();
    scratch.gather(
        dom,
        &sheets,
        id,
        &[PseudoElementTarget::None],
        super::matching::Rules::Match,
    );
    if !scratch.matched_any(|rule| rule.starting_style) {
        return None;
    }
    let merged_vars = walk::merge_root_vars(dom, &sheets);
    let parent = super::subtrees::parent_computed_for(dom, id, &merged_vars);
    let parent_id = dom.node(id).parent_node().map(|p| p.id());
    let mut cx = ElementCx {
        dom,
        sheets: &sheets,
        id,
        counters: &mut counters,
        scratch: &mut scratch,
    };
    let mut recorder = Recorder::new(None, false);
    Some(walk::compute_box(
        &mut cx,
        super::matching::Slot::Element,
        None,
        &mut recorder,
        |cx, rules| super::element::compute_element_style(cx, &parent, parent_id, rules),
    ))
}
