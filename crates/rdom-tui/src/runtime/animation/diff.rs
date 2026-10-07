//! The cascade hook: diff each element's previous and new computed
//! style, register the transitions the changes start, and write /
//! clear the animated values in `TuiExt::presentation`.

use std::time::{Duration, Instant};

use rdom_core::{Dom, NodeId, NodeType};

use super::{ActiveAnimation, AnimatedProp, AnimatedValue, AnimationRegistry};
use crate::ext::{StyleSlot, TuiExt};
use crate::style::ComputedStyle;
use crate::style::transition::{TimingFunction, TransitionProperty};

// ── Cascade hook ──────────────────────────────────────────────────

/// Run after `Dom::cascade(&sheet)`. For each element, diff
/// `computed_prev` against `computed`; for each animatable
/// property change covered by an active transition rule, register
/// (or replace) an animation. Then snapshot `computed` →
/// `computed_prev` for next pass.
pub fn diff_and_register(dom: &mut Dom<TuiExt>, registry: &mut AnimationRegistry, now: Instant) {
    let ids = collect_element_ids(dom, dom.root());
    let preferred = crate::style::CascadeExt::color_scheme(dom);
    for id in ids {
        let (prev, curr) = match snapshot(dom, id) {
            Some(pair) => pair,
            None => continue,
        };
        if let Some(prev_style) = prev.as_deref() {
            let curr_style: &ComputedStyle = curr.as_deref().unwrap();
            registry.diff_custom(dom, id, prev_style, curr_style, now);
            for prop in animatable_props_for(curr_style, prev_style) {
                let Some(rule) = lookup_rule(curr_style, prop) else {
                    continue;
                };
                // Zero-duration rule means "no transition";
                // commit the new value immediately.
                if rule.duration_ms == 0 && rule.delay_ms == 0 {
                    continue;
                }
                let from = read_value(prev_style, prop);
                let to = read_value(curr_style, prop);
                if from == to {
                    continue;
                }
                let anim = ActiveAnimation {
                    node: id,
                    slot: StyleSlot::Host,
                    property: prop,
                    from,
                    to,
                    started_at: now,
                    delay: Duration::from_millis(rule.delay_ms as u64),
                    duration: Duration::from_millis(rule.duration_ms as u64),
                    timing: rule.timing,
                    scheme: curr_style.color_scheme.used(preferred),
                    started_dispatched: false,
                };
                registry.register(anim, now);
            }
        }
        // Pseudo-elements: their paint properties transition too, driven
        // by the pseudo's own `transition-*` (`D-M3-3`).
        for slot in [StyleSlot::Before, StyleSlot::After] {
            let Some((prev_p, curr_p)) = snapshot_pseudo(dom, id, slot) else {
                continue;
            };
            for prop in animatable_props_for(&curr_p, &prev_p) {
                if !matches!(
                    prop,
                    AnimatedProp::Fg | AnimatedProp::Bg | AnimatedProp::BorderFg
                ) {
                    continue;
                }
                let Some(rule) = lookup_rule(&curr_p, prop) else {
                    continue;
                };
                if rule.duration_ms == 0 && rule.delay_ms == 0 {
                    continue;
                }
                let from = read_value(&prev_p, prop);
                let to = read_value(&curr_p, prop);
                if from == to {
                    continue;
                }
                registry.register(
                    ActiveAnimation {
                        node: id,
                        slot,
                        property: prop,
                        from,
                        to,
                        started_at: now,
                        delay: Duration::from_millis(rule.delay_ms as u64),
                        duration: Duration::from_millis(rule.duration_ms as u64),
                        timing: rule.timing,
                        scheme: curr_p.color_scheme.used(preferred),
                        started_dispatched: false,
                    },
                    now,
                );
            }
        }
        // Snapshot for next pass.
        let mut node_mut = dom.node_mut(id);
        if let Some(ext) = node_mut.ext_mut() {
            if let Some(curr_clone) = curr {
                ext.computed_prev = Some(curr_clone);
            }
            ext.computed_before_prev = ext.computed_before.clone();
            ext.computed_after_prev = ext.computed_after.clone();
        }
    }
}

/// Run after the per-frame re-cascade that carries registered
/// custom-property transitions to their `var()` consumers
/// ([`AnimationRegistry::take_restyle`]): make that animated result the
/// subtrees' previous style. CSS Transitions 1 §3 compares a style
/// change with the *before-change style*, which includes running
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
                ext.computed_prev = ext.computed.clone();
                ext.computed_before_prev = ext.computed_before.clone();
                ext.computed_after_prev = ext.computed_after.clone();
            }
        }
    }
}

/// `(prev, curr)` for a pseudo-element slot when both exist and differ
/// by identity.
fn snapshot_pseudo(
    dom: &Dom<TuiExt>,
    id: NodeId,
    slot: StyleSlot,
) -> Option<(std::rc::Rc<ComputedStyle>, std::rc::Rc<ComputedStyle>)> {
    let ext = dom.node(id).ext()?;
    let (prev, curr) = match slot {
        StyleSlot::Before => (&ext.computed_before_prev, &ext.computed_before),
        StyleSlot::After => (&ext.computed_after_prev, &ext.computed_after),
        // The host is diffed on its own; a marker does not transition.
        _ => return None,
    };
    let (prev, curr) = (prev.as_ref()?, curr.as_ref()?);
    // Rc clones only; an unchanged pseudo (the cascade did not touch this
    // element) shares the allocation and is skipped outright.
    if std::rc::Rc::ptr_eq(prev, curr) {
        return None;
    }
    Some((prev.clone(), curr.clone()))
}

/// A shared handle to a cascade result (`None` before the first cascade).
type StyleSnapshot = Option<std::rc::Rc<ComputedStyle>>;

fn snapshot(dom: &Dom<TuiExt>, id: NodeId) -> Option<(StyleSnapshot, StyleSnapshot)> {
    // Rc clones: this runs for every element every frame.
    let ext = dom.node(id).ext()?;
    Some((ext.computed_prev.clone(), ext.computed.clone()))
}

fn collect_element_ids(dom: &Dom<TuiExt>, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk(dom, id, &mut out);
    out
}

fn walk(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    if dom.node(id).node_type() == NodeType::Element {
        out.push(id);
    }
    for child in dom.node(id).child_nodes() {
        walk(dom, child.id(), out);
    }
}

/// Iterate the animatable properties whose values differ between
/// `prev` and `curr`. Skip properties the cascade hasn't moved.
fn animatable_props_for(curr: &ComputedStyle, prev: &ComputedStyle) -> Vec<AnimatedProp> {
    let mut out = Vec::new();
    if curr.fg != prev.fg {
        out.push(AnimatedProp::Fg);
    }
    if curr.bg != prev.bg {
        out.push(AnimatedProp::Bg);
    }
    if curr.border_color != prev.border_color {
        out.push(AnimatedProp::BorderFg);
    }
    if curr.width != prev.width {
        out.push(AnimatedProp::Width);
    }
    if curr.height != prev.height {
        out.push(AnimatedProp::Height);
    }
    if curr.padding != prev.padding {
        out.push(AnimatedProp::Padding);
    }
    // A `calc()` gap has no cell value until layout; only cell ↔ cell
    // changes interpolate (a calc-bearing change snaps).
    let cells = |s: &ComputedStyle| Some((s.row_gap.as_cells()?, s.column_gap.as_cells()?));
    if (curr.row_gap != prev.row_gap || curr.column_gap != prev.column_gap)
        && cells(curr).is_some()
        && cells(prev).is_some()
    {
        out.push(AnimatedProp::Gap);
    }
    if curr.top != prev.top {
        out.push(AnimatedProp::Top);
    }
    if curr.right != prev.right {
        out.push(AnimatedProp::Right);
    }
    if curr.bottom != prev.bottom {
        out.push(AnimatedProp::Bottom);
    }
    if curr.left != prev.left {
        out.push(AnimatedProp::Left);
    }
    if curr.z_index != prev.z_index {
        out.push(AnimatedProp::ZIndex);
    }
    // CSS Display 3 §4: `visibility` interpolates only when one end is
    // `visible`; between two non-visible values it is discrete.
    if curr.visibility != prev.visibility
        && (curr.visibility.is_visible() || prev.visibility.is_visible())
    {
        out.push(AnimatedProp::Visibility);
    }
    out
}

/// Look up the transition rule for `prop` inside `style`'s four
/// transition longhand lists, applying CSS L1's cycling rule when
/// list lengths differ. Returns `None` when no rule applies (no
/// transition-property entry covers this property, or
/// transition-property is `None`).
fn lookup_rule(style: &ComputedStyle, prop: AnimatedProp) -> Option<MatchedRule> {
    let props = &style.transition_property;
    if props.is_empty() {
        return None;
    }
    // Find the index of the entry covering `prop`.
    let idx = props.iter().position(|p| match p {
        TransitionProperty::All => true,
        TransitionProperty::None | TransitionProperty::Discrete(_) => false,
        TransitionProperty::Named(ap) => AnimatedProp::from_animatable(*ap) == Some(prop),
    })?;
    // None entries disable transitions for the matched property.
    if matches!(props[idx], TransitionProperty::None) {
        return None;
    }
    let durations = &style.transition_duration;
    let timings = &style.transition_timing_function;
    let delays = &style.transition_delay;
    let duration_ms = cycle(durations, idx).copied().unwrap_or(0);
    let timing = cycle(timings, idx).copied().unwrap_or(TimingFunction::Ease);
    let delay_ms = cycle(delays, idx).copied().unwrap_or(0);
    Some(MatchedRule {
        duration_ms,
        timing,
        delay_ms,
    })
}

#[derive(Debug, Clone, Copy)]
struct MatchedRule {
    duration_ms: u32,
    timing: TimingFunction,
    delay_ms: u32,
}

fn cycle<T>(list: &[T], idx: usize) -> Option<&T> {
    if list.is_empty() {
        None
    } else {
        Some(&list[idx % list.len()])
    }
}

fn read_value(style: &ComputedStyle, prop: AnimatedProp) -> AnimatedValue {
    match prop {
        AnimatedProp::Fg => AnimatedValue::Color(style.fg),
        AnimatedProp::Bg => AnimatedValue::Color(style.bg),
        AnimatedProp::BorderFg => AnimatedValue::Colors(style.border_color),
        AnimatedProp::Width => AnimatedValue::Size(style.width.clone()),
        AnimatedProp::Height => AnimatedValue::Size(style.height.clone()),
        AnimatedProp::Padding => AnimatedValue::Padding(style.padding.clone()),
        AnimatedProp::Gap => AnimatedValue::Gaps(
            style.row_gap.as_cells().unwrap_or(0),
            style.column_gap.as_cells().unwrap_or(0),
        ),
        AnimatedProp::Top => AnimatedValue::Length(style.top.clone()),
        AnimatedProp::Right => AnimatedValue::Length(style.right.clone()),
        AnimatedProp::Bottom => AnimatedValue::Length(style.bottom.clone()),
        AnimatedProp::Left => AnimatedValue::Length(style.left.clone()),
        AnimatedProp::ZIndex => AnimatedValue::ZIndex(style.z_index),
        AnimatedProp::Visibility => AnimatedValue::Visibility(style.visibility),
    }
}

pub(super) fn write_presentation(
    dom: &mut Dom<TuiExt>,
    node: NodeId,
    slot: StyleSlot,
    prop: AnimatedProp,
    value: AnimatedValue,
) {
    let mut node_mut = dom.node_mut(node);
    let Some(ext) = node_mut.ext_mut() else {
        return;
    };
    let Some(ext) = ext.presentation_for_mut(slot) else {
        return;
    };
    match (prop, value) {
        (AnimatedProp::Fg, AnimatedValue::Color(c)) => ext.fg = Some(c),
        (AnimatedProp::Bg, AnimatedValue::Color(c)) => ext.bg = Some(c),
        (AnimatedProp::BorderFg, AnimatedValue::Colors(c)) => ext.border_color = Some(c),
        (AnimatedProp::Width, AnimatedValue::Size(s)) => ext.width = Some(s),
        (AnimatedProp::Height, AnimatedValue::Size(s)) => ext.height = Some(s),
        (AnimatedProp::Padding, AnimatedValue::Padding(p)) => ext.padding = Some(p),
        (AnimatedProp::Gap, AnimatedValue::Gaps(row, column)) => {
            ext.row_gap = Some(row);
            ext.column_gap = Some(column);
        }
        (AnimatedProp::Top, AnimatedValue::Length(l)) => ext.top = Some(l),
        (AnimatedProp::Right, AnimatedValue::Length(l)) => ext.right = Some(l),
        (AnimatedProp::Bottom, AnimatedValue::Length(l)) => ext.bottom = Some(l),
        (AnimatedProp::Left, AnimatedValue::Length(l)) => ext.left = Some(l),
        (AnimatedProp::ZIndex, AnimatedValue::ZIndex(z)) => ext.z_index = Some(z),
        (AnimatedProp::Visibility, AnimatedValue::Visibility(v)) => ext.visibility = Some(v),
        _ => {}
    }
}

pub(super) fn clear_presentation(
    dom: &mut Dom<TuiExt>,
    node: NodeId,
    slot: StyleSlot,
    prop: AnimatedProp,
) {
    let mut node_mut = dom.node_mut(node);
    let Some(ext) = node_mut.ext_mut() else {
        return;
    };
    if ext.presentation_for(slot).is_none() {
        return;
    }
    let Some(presentation) = ext.presentation_for_mut(slot) else {
        return;
    };
    match prop {
        AnimatedProp::Fg => presentation.fg = None,
        AnimatedProp::Bg => presentation.bg = None,
        AnimatedProp::BorderFg => presentation.border_color = None,
        AnimatedProp::Width => presentation.width = None,
        AnimatedProp::Height => presentation.height = None,
        AnimatedProp::Padding => presentation.padding = None,
        AnimatedProp::Gap => {
            presentation.row_gap = None;
            presentation.column_gap = None;
        }
        AnimatedProp::Top => presentation.top = None,
        AnimatedProp::Right => presentation.right = None,
        AnimatedProp::Bottom => presentation.bottom = None,
        AnimatedProp::Left => presentation.left = None,
        AnimatedProp::ZIndex => presentation.z_index = None,
        AnimatedProp::Visibility => presentation.visibility = None,
    }
    ext.release_empty_presentation(slot);
}
