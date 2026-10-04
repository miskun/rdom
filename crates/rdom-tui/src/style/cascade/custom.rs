//! Custom properties (CSS Variables 1 §2) through the cascade ladder:
//! every matched `--*` declaration folds into the element's own map in
//! ladder order, so a later or more important declaration of the same
//! name wins, before any `var()` consumer runs; then their own `var()`s
//! are substituted (`rdom_style::backend`) and the registered ones settled
//! (`registered.rs`).

use std::collections::{HashMap, HashSet};

use super::ladder::{Declarations, Plan, Rollback, Step};
use super::registered::PropertyRegistry;
use crate::style::ComputedStyle;
use rdom_style::backend::{SubstitutionContext, resolve_custom_properties};
use rdom_style::calc::Viewport;

type Map = HashMap<String, rdom_style::CustomValue>;

/// Fold the element's `--*` declarations into `working.vars`, then
/// apply the registered properties of `registry` (Properties and
/// Values 1 §2). Copy-on-write: an element that declares nothing — and
/// whose registered properties need no reset — keeps sharing its
/// parent's map.
pub(super) fn apply_custom_properties(
    working: &mut ComputedStyle,
    plan: &Plan,
    decls: Declarations<'_>,
    registry: &PropertyRegistry,
    transitions: Option<&Map>,
    attrs: rdom_style::backend::AttrLookup<'_>,
    viewport: Viewport,
) {
    let inherited = working.vars.clone();
    let declared: HashSet<&str> = decls
        .all()
        .flat_map(|s| s.custom_properties.iter().map(|d| d.name.as_str()))
        .collect();
    if !declared.is_empty() {
        let base = || (*inherited).clone();
        let apply = |map: &mut Map, i: usize, rollback: &Rollback<'_, Map>| {
            put_step(
                map,
                &plan.steps()[i],
                decls,
                &inherited,
                rollback,
                registry,
                viewport,
            );
        };
        let rollback = Rollback::new(plan.steps().len(), &base, &apply);
        let map = std::rc::Rc::make_mut(&mut working.vars);
        for step in plan.steps() {
            put_step(map, step, decls, &inherited, &rollback, registry, viewport);
        }
    }
    if !registry.is_empty() {
        registry.settle_undeclared(&mut working.vars, &declared, viewport);
    }
    if !declared.is_empty() {
        // CSS Variables 1 §3: a custom property's own `var()`s substitute
        // here, where it is declared; descendants inherit the result. A
        // registered one is validated as it resolves (Properties and
        // Values 1 §2.4), so its dependents read the computed value.
        let map = std::rc::Rc::make_mut(&mut working.vars);
        // Their `attr()`s read the element's attributes (CSS Values 5 §8.7).
        let cx = SubstitutionContext::new().with_attrs(attrs);
        let mut computed =
            |name: &str, value| registry.computed_value(name, value, &inherited, viewport);
        let cx = if registry.is_empty() {
            cx
        } else {
            cx.with_computed(&mut computed)
        };
        // An invalid one is the guaranteed-invalid value (removed); the
        // cascade does not report why.
        let _invalid = resolve_custom_properties(map, declared.iter().copied(), cx);
    }
    apply_transitions(working, &inherited, &declared, registry, transitions);
}

/// `working.animated_vars`: the cascaded values with the running
/// transitions applied — the parent's animated values of the properties
/// this element inherits rather than sets, then this element's own
/// (`transitions`). `None` when that changes nothing.
fn apply_transitions(
    working: &mut ComputedStyle,
    inherited: &Map,
    declared: &HashSet<&str>,
    registry: &PropertyRegistry,
    transitions: Option<&Map>,
) {
    let parent = working.animated_vars.take();
    if parent.is_none() && transitions.is_none() {
        return;
    }
    let mut animated = (*working.vars).clone();
    if let Some(parent) = &parent {
        for (name, value) in parent.iter() {
            let animating = inherited.get(name) != Some(value);
            let inherits = registry.get(name).is_none_or(|r| r.inherits);
            if animating && inherits && !declared.contains(name.as_str()) {
                animated.insert(name.clone(), value.clone());
            }
        }
    }
    for (name, value) in transitions.into_iter().flatten() {
        animated.insert(name.clone(), value.clone());
    }
    if animated != *working.vars {
        working.animated_vars = Some(std::rc::Rc::new(animated));
    }
}

/// One ladder step's custom-property declarations. CSS Variables 1 §2:
/// the CSS-wide keywords apply to custom properties too — `initial` is
/// the guaranteed-invalid value (the property is undefined), `inherit`
/// / `unset` take the parent's, `revert` / `revert-layer` the value of
/// the step the ladder rolls back to (Cascade 4 §7.3, Cascade 5 §7.4).
fn put_step(
    map: &mut Map,
    step: &Step,
    decls: Declarations<'_>,
    inherited: &Map,
    rollback: &Rollback<'_, Map>,
    registry: &PropertyRegistry,
    viewport: Viewport,
) {
    for style in decls.of(step) {
        for d in &style.custom_properties {
            if d.important != step.important {
                continue;
            }
            let v = d.value.trim();
            // A registered property's `initial` (and, when it does not
            // inherit, `unset`) is its initial value (Properties and
            // Values 1 §2.1).
            let keyword = if v.eq_ignore_ascii_case("initial") {
                registry.keyword_value(&d.name, false, viewport)
            } else if v.eq_ignore_ascii_case("unset") {
                registry.keyword_value(&d.name, true, viewport)
            } else {
                None
            };
            if let Some(value) = keyword {
                match value {
                    Some(value) => map.insert(d.name.clone(), value),
                    None => map.remove(&d.name),
                };
                continue;
            }
            let from = if v.eq_ignore_ascii_case("initial") {
                None
            } else if v.eq_ignore_ascii_case("inherit") || v.eq_ignore_ascii_case("unset") {
                Some(inherited)
            } else if v.eq_ignore_ascii_case("revert") {
                Some(rollback.state_before(step.revert_to))
            } else if v.eq_ignore_ascii_case("revert-layer") {
                Some(rollback.state_before(step.revert_layer_to))
            } else {
                map.insert(d.name.clone(), d.value.clone());
                continue;
            };
            match from.and_then(|m| m.get(&d.name)) {
                Some(value) => {
                    map.insert(d.name.clone(), value.clone());
                }
                None => {
                    map.remove(&d.name);
                }
            }
        }
    }
}
