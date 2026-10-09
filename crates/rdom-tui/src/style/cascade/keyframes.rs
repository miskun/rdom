//! The cascade's side of CSS animations (CSS Animations 1 §3): which
//! `@keyframes` rule a name resolves to under a sheet set, and a
//! keyframe's computed values.
//!
//! A keyframe's values are computed like the element's own — the same
//! matched rules, inheritance, `var()`, `currentcolor`, relative units —
//! with the keyframe's declaration blocks applied in the animation
//! origin (CSS Cascade 5 §6.1): over every normal declaration, under
//! every `!important` one, so an important declaration of a property
//! keeps its value however the animation runs. The engine
//! (`runtime::animation::css`) computes them when an animation starts or
//! its element's style changes, never per frame.

use std::rc::Rc;

use rdom_core::{Dom, NodeId};

use super::matching::{Recorder, Rules};
use super::registered::PropertyRegistry;
use super::walk::{self, CounterState, ElementCx, Scratch, Sheets};
use crate::ext::{StyleSlot, TuiExt};
use crate::style::{ComputedStyle, KeyframesRule, PseudoElementTarget, Stylesheet, TuiStyle};

/// The `@keyframes` rule `name` resolves to under `stylesheets` (CSS
/// Animations 1 §3): the last rule of that name — by cascade layer
/// (unlayered last), then sheet, then source order (CSS Cascade 5
/// §6.4.3). Rules of one name never merge.
pub(crate) fn keyframes_rule<'s>(
    dom: &Dom<TuiExt>,
    (stylesheets, registry): (&'s [&'s Stylesheet], &Rc<PropertyRegistry>),
    name: &str,
) -> Option<&'s KeyframesRule> {
    sheets(dom, stylesheets, registry).keyframes_rule(name)
}

/// `id`'s style for `slot` with a keyframe's `blocks` applied in the
/// animation origin; `None` when the slot has no box (a `::before`
/// without content) or `id` is not an element.
pub(crate) fn keyframe_style(
    dom: &Dom<TuiExt>,
    (stylesheets, registry): (&[&Stylesheet], &Rc<PropertyRegistry>),
    id: NodeId,
    slot: StyleSlot,
    blocks: &[&TuiStyle],
) -> Option<ComputedStyle> {
    let sheets = sheets(dom, stylesheets, registry).with_animation(blocks);
    let mut scratch = Scratch::default();
    let mut counters = CounterState::default();
    let mut recorder = Recorder::new(None, false);
    match slot {
        StyleSlot::Host => {
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
            Some(walk::compute_box(
                &mut cx,
                super::matching::Slot::Element,
                None,
                &mut recorder,
                |cx, rules| super::element::compute_element_style(cx, &parent, parent_id, rules),
            ))
        }
        StyleSlot::Before | StyleSlot::After => {
            let host = dom.node(id).ext()?.computed.clone()?;
            let target = if slot == StyleSlot::Before {
                PseudoElementTarget::Before
            } else {
                PseudoElementTarget::After
            };
            let mut cx = ElementCx {
                dom,
                sheets: &sheets,
                id,
                counters: &mut counters,
                scratch: &mut scratch,
            };
            super::pseudo::compute_pseudo_style(&mut cx, &host, &[target], Rules::Match)
        }
        // No other slot animates (`StyleSlot`'s doc, DIVERGENCES §4).
        _ => None,
    }
}

/// The sheet set of one cascade over `stylesheets`.
fn sheets<'a>(
    dom: &Dom<TuiExt>,
    stylesheets: &'a [&'a Stylesheet],
    registry: &Rc<PropertyRegistry>,
) -> Sheets<'a> {
    Sheets::new(
        stylesheets,
        registry.clone(),
        super::media::document_media(dom),
    )
}
