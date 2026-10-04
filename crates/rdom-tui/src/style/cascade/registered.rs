//! Registered custom properties in the cascade (CSS Properties and
//! Values API 1 §2): the registrations of one cascade run, and what
//! they do to an element's custom-property map — the initial value,
//! `inherits: false`, and syntax validation at computed-value time.

use std::collections::{HashMap, HashSet};

use rdom_style::PropertyRegistration;

use crate::style::Stylesheet;

type Map = HashMap<String, String>;

/// The custom properties registered by the sheets of one cascade run,
/// by name; a later registration of a name replaces an earlier one
/// (sheets in cascade order — an `App`'s `CSS.registerProperty` sheet
/// comes last).
#[derive(Default)]
pub(super) struct Registry(HashMap<String, PropertyRegistration>);

impl Registry {
    pub(super) fn new(sheets: &[&Stylesheet]) -> Self {
        let mut map = HashMap::new();
        for sheet in sheets {
            for reg in sheet.registered_properties() {
                map.insert(reg.name.clone(), reg.clone());
            }
        }
        Registry(map)
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(super) fn get(&self, name: &str) -> Option<&PropertyRegistration> {
        self.0.get(name)
    }

    /// The value of the CSS-wide keyword `initial` / `unset` for a
    /// registered property: `Some(Some(v))` its initial value (`unset`
    /// on one that does not inherit), `Some(None)` when it has none;
    /// `None` when `name` is not registered or `unset` inherits.
    pub(super) fn keyword_value(&self, name: &str, unset: bool) -> Option<Option<String>> {
        let reg = self.get(name)?;
        if unset && reg.inherits {
            return None;
        }
        Some(reg.initial_value.clone())
    }

    /// The registered properties `declared` does not cover: a property
    /// that does not inherit restarts at its initial value; one with no
    /// value at all gets it (§2.1). Writes only what changes.
    pub(super) fn settle_undeclared(&self, map: &mut std::rc::Rc<Map>, declared: &HashSet<&str>) {
        for (name, reg) in &self.0 {
            if declared.contains(name.as_str()) {
                continue;
            }
            let current = map.get(name);
            let want = if reg.inherits {
                current.or(reg.initial_value.as_ref())
            } else {
                reg.initial_value.as_ref()
            };
            set(map, name, current.cloned(), want.cloned());
        }
    }

    /// The computed value of custom property `name` given its
    /// substituted `value` (`None`: guaranteed-invalid): unchanged when
    /// `name` is not registered or the value matches its syntax; else
    /// invalid at computed-value time, so `unset` — `inherited`'s value
    /// when it inherits, the initial value otherwise (§2.4). The cascade
    /// runs it inside `var()` resolution, so a property that reads
    /// `name` substitutes this result.
    pub(super) fn computed_value(
        &self,
        name: &str,
        value: Option<String>,
        inherited: &Map,
    ) -> Option<String> {
        let Some(reg) = self.get(name) else {
            return value;
        };
        if value.as_deref().is_some_and(|v| reg.syntax.matches(v)) {
            return value;
        }
        if reg.inherits {
            inherited.get(name).or(reg.initial_value.as_ref()).cloned()
        } else {
            reg.initial_value.clone()
        }
    }

    /// The sheet-level variables (the root's parent) before their
    /// `var()`s resolve: every registered property they lack gets its
    /// initial value, so a dependent can read it (§2.1). Validation runs
    /// during resolution ([`computed_value`](Self::computed_value)).
    pub(super) fn seed_root(&self, map: &mut Map) {
        for (name, reg) in &self.0 {
            if let (false, Some(v)) = (map.contains_key(name), &reg.initial_value) {
                map.insert(name.clone(), v.clone());
            }
        }
    }
}

/// Make `map[name]` `want`, copying the map only on a change.
fn set(map: &mut std::rc::Rc<Map>, name: &str, current: Option<String>, want: Option<String>) {
    if current == want {
        return;
    }
    let map = std::rc::Rc::make_mut(map);
    match want {
        Some(v) => {
            map.insert(name.to_string(), v);
        }
        None => {
            map.remove(name);
        }
    }
}
