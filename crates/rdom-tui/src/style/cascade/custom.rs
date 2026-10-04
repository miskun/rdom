//! Custom properties (CSS Variables 1 §2) through the cascade ladder:
//! every matched `--*` declaration folds into the element's own map in
//! ladder order, so a later or more important declaration of the same
//! name wins, before any `var()` consumer runs.

use std::collections::HashMap;

use super::ladder::{Declarations, Plan, Rollback, Step};
use crate::style::ComputedStyle;

type Map = HashMap<String, String>;

/// Fold the element's `--*` declarations into `working.vars`.
/// Copy-on-write: elements that declare nothing keep sharing their
/// parent's map.
pub(super) fn apply_custom_properties(
    working: &mut ComputedStyle,
    plan: &Plan,
    decls: Declarations<'_>,
) {
    let declares = decls
        .sorted
        .iter()
        .any(|r| !r.style.custom_properties.is_empty())
        || decls
            .inline
            .is_some_and(|s| !s.custom_properties.is_empty());
    if !declares {
        return;
    }
    let inherited = working.vars.clone();
    let base = || (*inherited).clone();
    let apply = |map: &mut Map, i: usize, rollback: &Rollback<'_, Map>| {
        put_step(map, &plan.steps()[i], decls, &inherited, rollback);
    };
    let rollback = Rollback::new(plan.steps().len(), &base, &apply);
    let map = std::rc::Rc::make_mut(&mut working.vars);
    for step in plan.steps() {
        put_step(map, step, decls, &inherited, &rollback);
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
) {
    for style in decls.of(step) {
        for d in &style.custom_properties {
            if d.important != step.important {
                continue;
            }
            let v = d.value.trim();
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
