//! The cascade hook (CSS Transitions 1 §3): compare each element style's
//! before-change and after-change values, longhand by longhand, and
//! start, replace or cancel the transitions the changes call for.

use std::rc::Rc;
use std::time::Instant;

use rdom_core::{Dom, NodeId, NodeType};

use super::rule::{self, Coverage};
use super::{ActiveAnimation, AnimationRegistry, Longhand};
use crate::ext::{StyleSlot, TuiExt};
use crate::style::ComputedStyle;

/// Run after `Dom::cascade(&sheet)`. For each element style (the element
/// and its `::before` / `::after`), compare the previous cascaded style
/// with the new one; for each longhand that changed under a
/// `transition-*` rule, start (or replace) its transition. Then keep the
/// new cascaded styles as the previous ones for the next pass.
pub fn diff_and_register(dom: &mut Dom<TuiExt>, registry: &mut AnimationRegistry, now: Instant) {
    let ids = collect_element_ids(dom, dom.root());
    let preferred = crate::style::CascadeExt::color_scheme(dom);
    for id in ids {
        for slot in [StyleSlot::Host, StyleSlot::Before, StyleSlot::After] {
            let Some((prev, curr)) = snapshot(dom, id, slot) else {
                continue;
            };
            if slot == StyleSlot::Host {
                registry.diff_custom(dom, id, &prev, &curr, now);
            }
            let scheme = curr.color_scheme.used(preferred);
            diff_style(registry, id, slot, &prev, &curr, scheme, now);
        }
        // Keep the cascaded styles for the next pass.
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            if let Some(curr) = ext.cascaded_for(StyleSlot::Host).cloned() {
                ext.computed_prev = Some(curr);
            }
            ext.snapshot_pseudo_prev();
        }
    }
}

/// Start, replace or cancel the transitions of one element style (CSS
/// Transitions 1 §3, "Starting of transitions").
fn diff_style(
    registry: &mut AnimationRegistry,
    id: NodeId,
    slot: StyleSlot,
    prev: &Rc<ComputedStyle>,
    curr: &Rc<ComputedStyle>,
    scheme: rdom_style::color::ColorScheme,
    now: Instant,
) {
    let running = registry.running_on(id, slot);
    // The fast path: no rule can start a transition, none runs.
    if running.is_empty() && !rule::any(curr) {
        return;
    }
    let coverage = Coverage::of(curr);
    for l in Longhand::all() {
        let changed = l.differs(prev, curr);
        let rule = coverage.rule(curr, l).filter(rule::Rule::runs);
        let Some(rule) = rule else {
            // §3: a running transition whose property no longer has a
            // matching rule is cancelled.
            if running.contains(&l) {
                registry.cancel(id, slot, l, now);
            }
            continue;
        };
        if !changed {
            continue;
        }
        if !l.interpolable(prev, curr) {
            // A value that does not interpolate changes at once; one that
            // was transitioning stops.
            registry.cancel(id, slot, l, now);
            continue;
        }
        let clock = rule.clock(now);
        registry.register(
            ActiveAnimation {
                node: id,
                slot,
                property: l,
                from: prev.clone(),
                to: curr.clone(),
                started_at: clock.started_at,
                delay: clock.delay,
                duration: clock.duration,
                skipped: clock.skipped,
                timing: rule.timing,
                scheme,
                started_dispatched: false,
            },
            now,
        );
    }
}

/// Run after the per-frame restyle that carries running values to the
/// descendants ([`AnimationRegistry::take_restyle`]): make the restyled
/// cascaded styles the previous ones. CSS Transitions 1 §3 compares a
/// style change with the *before-change style*, which includes running
/// animations at the current time; without this, the next cascade of
/// these elements would diff against the values of the last real style
/// change and start transitions nothing asked for.
pub fn settle_restyled(dom: &mut Dom<TuiExt>, roots: &[NodeId]) {
    for &root in roots {
        if !dom.contains(root) {
            continue;
        }
        for id in collect_element_ids(dom, root) {
            if let Some(ext) = dom.node_mut(id).ext_mut() {
                ext.computed_prev = ext.cascaded_for(StyleSlot::Host).cloned();
                ext.snapshot_pseudo_prev();
            }
        }
    }
}

/// `(previous, new)` cascaded styles of `slot` when both exist and the
/// cascade replaced the style (an unchanged one shares its allocation).
fn snapshot(
    dom: &Dom<TuiExt>,
    id: NodeId,
    slot: StyleSlot,
) -> Option<(Rc<ComputedStyle>, Rc<ComputedStyle>)> {
    let ext = dom.node(id).ext()?;
    let prev = match slot {
        StyleSlot::Host => ext.computed_prev.as_ref(),
        StyleSlot::Before => ext.computed_before_prev(),
        StyleSlot::After => ext.computed_after_prev(),
        _ => None,
    }?;
    let curr = ext.cascaded_for(slot)?;
    if Rc::ptr_eq(prev, curr) {
        return None;
    }
    Some((prev.clone(), curr.clone()))
}

fn collect_element_ids(dom: &Dom<TuiExt>, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk(dom, id, &mut out);
    out
}

/// The elements of the box tree under `id` (`box_tree::children`): a
/// `<details>`'s `::details-content` box transitions as an element does.
fn walk(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    if dom.node(id).node_type() == NodeType::Element {
        out.push(id);
    }
    for child in crate::render::box_tree::children(dom, id) {
        walk(dom, child, out);
    }
}
