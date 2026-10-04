//! Registered custom properties in the cascade (CSS Properties and
//! Values API 1 §2): the registrations of one cascade run, and what
//! they do to an element's custom-property map — the initial value,
//! `inherits: false`, and syntax validation at computed-value time.

use std::collections::{HashMap, HashSet};

use rdom_style::{CustomValue, PropertyRegistration};

use crate::style::Stylesheet;

type Map = HashMap<String, CustomValue>;

/// One registration, with its initial value tokenized once.
#[derive(Debug)]
struct Entry {
    reg: PropertyRegistration,
    initial: Option<CustomValue>,
}

/// The custom properties registered by a list of sheets, by name; a
/// later registration of a name replaces an earlier one (sheets in
/// cascade order — an `App`'s `CSS.registerProperty` sheet comes last).
///
/// The one registry both the cascade and the transition engine
/// (`runtime::animation`) read. An `App` builds it when its sheets
/// change (`FramePrelude::sheets_changed`) and shares it; the
/// stateless [`CascadeExt`](super::CascadeExt) entry points build one
/// per call.
#[derive(Debug, Default)]
pub(crate) struct PropertyRegistry(HashMap<String, Entry>);

impl PropertyRegistry {
    pub(crate) fn new(sheets: &[&Stylesheet]) -> Self {
        #[cfg(test)]
        probe::BUILDS.with(|c| c.set(c.get() + 1));
        let mut map = HashMap::new();
        for sheet in sheets {
            for reg in sheet.registered_properties() {
                let initial = reg.initial_value.as_deref().map(CustomValue::new);
                map.insert(
                    reg.name.clone(),
                    Entry {
                        reg: reg.clone(),
                        initial,
                    },
                );
            }
        }
        PropertyRegistry(map)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Every registration, by name, in no particular order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &PropertyRegistration)> {
        self.0.iter().map(|(name, e)| (name.as_str(), &e.reg))
    }

    pub(super) fn get(&self, name: &str) -> Option<&PropertyRegistration> {
        self.0.get(name).map(|e| &e.reg)
    }

    /// The value of the CSS-wide keyword `initial` / `unset` for a
    /// registered property: `Some(Some(v))` its initial value (`unset`
    /// on one that does not inherit), `Some(None)` when it has none;
    /// `None` when `name` is not registered or `unset` inherits.
    pub(super) fn keyword_value(&self, name: &str, unset: bool) -> Option<Option<CustomValue>> {
        let e = self.0.get(name)?;
        if unset && e.reg.inherits {
            return None;
        }
        Some(e.initial.clone())
    }

    /// The registered properties `declared` does not cover: a property
    /// that does not inherit restarts at its initial value; one with no
    /// value at all gets it (§2.1). Writes only what changes.
    pub(super) fn settle_undeclared(&self, map: &mut std::rc::Rc<Map>, declared: &HashSet<&str>) {
        for (name, e) in &self.0 {
            if declared.contains(name.as_str()) {
                continue;
            }
            let current = map.get(name);
            let want = if e.reg.inherits {
                current.or(e.initial.as_ref())
            } else {
                e.initial.as_ref()
            };
            // Compared by reference: the common case (an inherited value
            // kept, an initial value already in place) copies nothing.
            if current == want {
                continue;
            }
            let want = want.cloned();
            let map = std::rc::Rc::make_mut(map);
            match want {
                Some(v) => {
                    map.insert(name.clone(), v);
                }
                None => {
                    map.remove(name);
                }
            }
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
        value: Option<CustomValue>,
        inherited: &Map,
    ) -> Option<CustomValue> {
        let Some(e) = self.0.get(name) else {
            return value;
        };
        if value.as_ref().is_some_and(|v| e.reg.syntax.matches(v)) {
            return value;
        }
        if e.reg.inherits {
            inherited.get(name).or(e.initial.as_ref()).cloned()
        } else {
            e.initial.clone()
        }
    }

    /// The sheet-level variables (the root's parent) before their
    /// `var()`s resolve: every registered property they lack gets its
    /// initial value, so a dependent can read it (§2.1). Validation runs
    /// during resolution ([`computed_value`](Self::computed_value)).
    pub(super) fn seed_root(&self, map: &mut Map) {
        for (name, e) in &self.0 {
            if let (false, Some(v)) = (map.contains_key(name), &e.initial) {
                map.insert(name.clone(), v.clone());
            }
        }
    }
}

/// Test-only: how many registries were built on this thread.
#[cfg(test)]
pub(crate) mod probe {
    thread_local! {
        pub static BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    pub fn take_builds() -> usize {
        BUILDS.with(|c| c.replace(0))
    }
}
