//! The second half of an element's cascade, after its children
//! (`walk::cascade_subtree`): its `::after` — which sees their counter
//! increments — and the bottom-up aggregates, written back with the
//! pseudo-elements' styles under their running transitions.

use std::rc::Rc;

use rdom_core::{Dom, NodeId};

use super::counters::has_ops;
use super::matching::Slot;
use super::pseudo::compute_pseudo_style;
use super::walk::{
    CounterState, ElementCx, FreshElement, Scratch, Sheets, SubtreeFlags, compute_box,
};
use crate::ext::TuiExt;
use crate::layout::Position;
use crate::style::{ComputedStyle, PseudoElementTarget};

/// After the element `id`'s children: its `::after` (which sees their
/// counter increments) and the bottom-up aggregates, written back. Not
/// inlined into the recursion, as `walk::style_element` is not.
#[inline(never)]
pub(super) fn finish_element<'a>(
    dom: &mut Dom<TuiExt>,
    sheets: &Sheets<'a>,
    id: NodeId,
    styled: FreshElement,
    mut flags: SubtreeFlags,
    counters: &mut CounterState,
    scratch: &mut Scratch<'a>,
) -> SubtreeFlags {
    let FreshElement {
        computed,
        computed_before,
        recorded,
        mut recorder,
        mut reads_counters,
        restyle,
    } = styled;
    let cached = recorded.as_deref().filter(|_| restyle);
    // `::after` comes after the children in tree order, and its own
    // marker after it (CSS Pseudo-Elements 4 §4).
    let (computed_after, after_marker) = {
        let mut cx = ElementCx {
            dom: &*dom,
            sheets,
            id,
            counters: &mut *counters,
            scratch: &mut *scratch,
        };
        // An element skipping its contents (CSS Containment 2 §4) has no
        // `::after`: it is part of them.
        let after = if crate::style::content_visibility::skips(cx.dom, id, &computed) {
            None
        } else {
            compute_box(&mut cx, Slot::After, cached, &mut recorder, |cx, rules| {
                compute_pseudo_style(cx, &computed, &[PseudoElementTarget::After], rules)
            })
        };
        let marker =
            super::early_pseudos::after_marker(&mut cx, after.as_ref(), cached, &mut recorder);
        (after, marker)
    };
    reads_counters |= counters.take_read();
    counters.exit(id);
    let own_has_positioned_pseudo = computed_before
        .as_deref()
        .is_some_and(|c| c.position != Position::Static)
        || computed_after
            .as_ref()
            .is_some_and(|c| c.position != Position::Static);
    flags.has_positioned_pseudo |= own_has_positioned_pseudo;

    flags.has_counters |= reads_counters
        || has_ops(&computed)
        || dom
            .node(id)
            .ext()
            .and_then(|e| e.computed_marker().map(|s| &**s))
            .is_some_and(has_ops)
        || computed_before.as_deref().is_some_and(has_ops)
        || computed_after.as_ref().is_some_and(has_ops);

    // Write the bottom-up aggregates.
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        counters.note_ops(ext.computed_after.as_deref(), computed_after.as_ref());
        write_pseudo(ext, crate::ext::StyleSlot::Before, computed_before);
        write_pseudo(
            ext,
            crate::ext::StyleSlot::After,
            computed_after.map(Rc::new),
        );
        let marker = after_marker.map(std::rc::Rc::new);
        ext.update_pseudo(marker.is_some(), |p| p.after_marker = marker);
        ext.tree_has_positioned_pseudo = flags.has_positioned_pseudo;
        ext.tree_has_collapse = flags.has_collapse;
        ext.tree_has_counters = flags.has_counters;
        ext.reads_counters = reads_counters;
        ext.matched = Some(recorder.finish(sheets));
    }
    super::details::mirror_flags(dom, id);
    flags
}

/// Store a pseudo-element's new style, under the values of its running
/// transitions (`TuiExt::overlay`).
fn write_pseudo(ext: &mut TuiExt, slot: crate::ext::StyleSlot, style: Option<Rc<ComputedStyle>>) {
    let Some(style) = style else {
        // No box: nothing of it is kept, its running values included
        // (C12G-PSEUDO-GONE); the transition hook cancels what ran.
        ext.drop_pseudo(slot);
        return;
    };
    let overlaid = ext.overlay(slot, &style).map(Rc::new);
    ext.set_cascaded(slot, style, overlaid);
}
