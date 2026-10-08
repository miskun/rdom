//! The cascade hook (CSS Transitions 1 §3): compare each element style's
//! before-change and after-change values, longhand by longhand, and
//! start, replace or cancel the transitions the changes call for — and
//! bring each element style's CSS animations in line with its
//! `animation-*` lists (`css`, CSS Animations 1 §4).

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
///
/// An element that was not rendered before — never styled, or `display:
/// none` itself or under an ancestor — has no before-change style (CSS
/// Transitions 1 §3), so its values change at once; it has no starting
/// style here ([`diff_and_register_with`] gives one).
pub fn diff_and_register(dom: &mut Dom<TuiExt>, registry: &mut AnimationRegistry, now: Instant) {
    diff_and_register_with(dom, registry, now, &|_, _| None);
}

/// [`diff_and_register`], an element newly rendered starting its
/// transitions from `starting` — its starting style (CSS Transitions 2
/// §3, `@starting-style`), `None` when it has none.
pub fn diff_and_register_with(
    dom: &mut Dom<TuiExt>,
    registry: &mut AnimationRegistry,
    now: Instant,
    starting: &dyn Fn(&Dom<TuiExt>, NodeId) -> Option<ComputedStyle>,
) {
    let starting = |dom: &Dom<TuiExt>, id: NodeId, slot: StyleSlot| {
        (slot == StyleSlot::Host)
            .then(|| starting(dom, id))
            .flatten()
    };
    diff(dom, registry, now, &starting, None);
}

/// [`diff_and_register_with`] under the cascade's sheets: the starting
/// styles they give, and the CSS animations their `@keyframes` rules run
/// (CSS Animations 1 §4) — what an `App`'s frame does.
pub(crate) fn diff_and_register_in(
    dom: &mut Dom<TuiExt>,
    registry: &mut AnimationRegistry,
    now: Instant,
    inputs: super::CssInputs<'_>,
) {
    let starting = |dom: &Dom<TuiExt>, id: NodeId, slot: StyleSlot| {
        crate::style::cascade::starting_style(dom, (inputs.sheets, inputs.registry), id, slot)
    };
    diff(dom, registry, now, &starting, Some(inputs));
}

/// A box's starting style (`@starting-style`), `None` when it has none.
type StartingStyle<'a> = dyn Fn(&Dom<TuiExt>, NodeId, StyleSlot) -> Option<ComputedStyle> + 'a;

fn diff(
    dom: &mut Dom<TuiExt>,
    registry: &mut AnimationRegistry,
    now: Instant,
    starting: &StartingStyle<'_>,
    css: Option<super::CssInputs<'_>>,
) {
    let ids = collect_element_ids(dom, dom.root());
    let preferred = crate::style::CascadeExt::color_scheme(dom);
    // Whether each element was rendered at the last style update, read
    // before any snapshot below moves on (parents come first).
    let rendered = was_rendered(dom, &ids);
    // CSS animations: the sheets changed since the last pass (every
    // animation re-resolves its `@keyframes`), and which elements are
    // rendered now (a hidden one runs none).
    let css = css.map(|inputs| {
        let sheets_changed = registry.css_stamp != inputs.stamp();
        registry.css_stamp = inputs.stamp();
        let animated: std::collections::HashSet<NodeId> =
            registry.css.iter().map(|a| a.node).collect();
        (inputs, sheets_changed, is_rendered(dom, &ids), animated)
    });
    for id in ids {
        let was_rendered = rendered.get(&id).copied().unwrap_or(false);
        for slot in [StyleSlot::Host, StyleSlot::Before, StyleSlot::After] {
            let changed = snapshot(dom, id, slot);
            // A `::before` / `::after` that generated a box at the last
            // style change and generates none now.
            let gone =
                slot != StyleSlot::Host && changed.is_none() && stopped_generating(dom, id, slot);
            if let Some((inputs, sheets_changed, rendered_now, animated)) = &css {
                let now_rendered = rendered_now.get(&id).copied().unwrap_or(false);
                update_css(
                    dom,
                    registry,
                    (*inputs, preferred, now),
                    (id, slot),
                    animated.contains(&id),
                    changed.is_some() || gone || *sheets_changed || now_rendered != was_rendered,
                    now_rendered,
                );
            }
            if gone {
                // CSS Transitions 1 §3: its transitions are cancelled.
                for l in registry.running_on(id, slot) {
                    registry.cancel(id, slot, l, now);
                }
                continue;
            }
            let Some((prev, curr)) = changed else {
                continue;
            };
            let scheme = curr.color_scheme.used(preferred);
            // A box with no before-change style — an element not rendered
            // at the last style update, a `::before` / `::after` that
            // generated no box then — starts its transitions only from a
            // starting style (`curr` is rendered, or it has nothing to
            // show).
            let newly_rendered = !was_rendered || (slot != StyleSlot::Host && prev.is_none());
            if newly_rendered {
                if curr.display != crate::layout::Display::None
                    && rule::any(&curr)
                    && let Some(start) = starting(dom, id, slot)
                {
                    diff_style(registry, id, slot, &Rc::new(start), &curr, scheme, now);
                }
                continue;
            }
            let Some(prev) = prev else {
                continue;
            };
            if slot == StyleSlot::Host {
                registry.diff_custom(dom, id, &prev, &curr, now);
            }
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
        // A pair that does not interpolate transitions only under
        // `transition-behavior: allow-discrete` (CSS Transitions 2 §3.1),
        // stepping by its type's rule; otherwise it changes at once, and a
        // transition of it that was running stops.
        let discrete_allowed =
            rule.behavior == crate::style::transition::TransitionBehavior::AllowDiscrete;
        let transitions = l.interpolable(prev, curr) || (discrete_allowed && l.is_animatable());
        if !transitions {
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

/// `(previous, new)` cascaded styles of `slot` when the new one exists
/// and the cascade replaced the style (an unchanged one shares its
/// allocation); the previous one is `None` before the first.
fn snapshot(
    dom: &Dom<TuiExt>,
    id: NodeId,
    slot: StyleSlot,
) -> Option<(Option<Rc<ComputedStyle>>, Rc<ComputedStyle>)> {
    let ext = dom.node(id).ext()?;
    let prev = match slot {
        StyleSlot::Host => ext.computed_prev.as_ref(),
        StyleSlot::Before => ext.computed_before_prev(),
        StyleSlot::After => ext.computed_after_prev(),
        _ => None,
    };
    let curr = ext.cascaded_for(slot)?;
    if prev.is_some_and(|p| Rc::ptr_eq(p, curr)) {
        return None;
    }
    Some((prev.cloned(), curr.clone()))
}

/// Whether `slot` of `id` had a previous style (a box at the last style
/// change) and has none now.
fn stopped_generating(dom: &Dom<TuiExt>, id: NodeId, slot: StyleSlot) -> bool {
    let Some(ext) = dom.node(id).ext() else {
        return false;
    };
    let prev = match slot {
        StyleSlot::Before => ext.computed_before_prev(),
        StyleSlot::After => ext.computed_after_prev(),
        _ => None,
    };
    prev.is_some() && ext.cascaded_for(slot).is_none()
}

/// Update the CSS animations of `(id, slot)` when its style, the sheets
/// or its being rendered changed — `animated`: it runs some now.
fn update_css(
    dom: &Dom<TuiExt>,
    registry: &mut AnimationRegistry,
    (inputs, preferred, now): (
        super::CssInputs<'_>,
        rdom_style::color::ColorScheme,
        Instant,
    ),
    (id, slot): (NodeId, StyleSlot),
    animated: bool,
    changed: bool,
    rendered: bool,
) {
    let Some(ext) = dom.node(id).ext() else {
        return;
    };
    let style = ext
        .cascaded_for(slot)
        .filter(|s| rendered && s.display != crate::layout::Display::None);
    let wants = style.is_some_and(|s| !s.animation_name.is_empty());
    if !(animated || wants) || !changed {
        return;
    }
    let scheme = style.map_or(preferred, |s| s.color_scheme.used(preferred));
    registry.update_css(
        dom,
        inputs,
        (id, slot),
        style.map(|s| &**s),
        scheme,
        true,
        now,
    );
}

/// Whether each of `ids` (in tree order, parents first) is rendered now:
/// styled, not `display: none`, and under a box parent that is.
fn is_rendered(dom: &Dom<TuiExt>, ids: &[NodeId]) -> std::collections::HashMap<NodeId, bool> {
    let mut out = std::collections::HashMap::with_capacity(ids.len());
    for &id in ids {
        let own = dom
            .node(id)
            .ext()
            .and_then(|e| e.cascaded_for(StyleSlot::Host))
            .is_some_and(|c| c.display != crate::layout::Display::None);
        let parent = crate::render::box_tree::box_parent(dom, id)
            .and_then(|p| out.get(&p).copied())
            .unwrap_or(true);
        out.insert(id, own && parent);
    }
    out
}

/// Whether each of `ids` (in tree order, parents first) was rendered at
/// the last style update: styled then, its before-change `display` not
/// `none`, and under a box parent that was rendered.
fn was_rendered(dom: &Dom<TuiExt>, ids: &[NodeId]) -> std::collections::HashMap<NodeId, bool> {
    let mut out = std::collections::HashMap::with_capacity(ids.len());
    for &id in ids {
        let own = dom
            .node(id)
            .ext()
            .and_then(before_change_display)
            .is_some_and(|d| d != crate::layout::Display::None);
        let parent = crate::render::box_tree::box_parent(dom, id)
            .and_then(|p| out.get(&p).copied())
            .unwrap_or(true);
        out.insert(id, own && parent);
    }
    out
}

/// The `display` of `ext`'s before-change style (CSS Transitions 1 §3:
/// the previous style with its declarative animations "updated to the
/// current time"): the previous cascade's, or — while a transition or an
/// animation holds it (`allow-discrete` on an exit) — the running value,
/// which the cascade's write-back carried into the computed style
/// (`TuiExt::overlay`). `None` for an element not styled then.
fn before_change_display(ext: &TuiExt) -> Option<crate::layout::Display> {
    let prev = ext.computed_prev.as_deref()?;
    let held = ext
        .presentation_for(StyleSlot::Host)
        .is_some_and(|p| p.animated().iter().any(|l| l.css_name() == "display"));
    Some(match ext.computed.as_deref() {
        Some(now) if held => now.display,
        _ => prev.display,
    })
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
