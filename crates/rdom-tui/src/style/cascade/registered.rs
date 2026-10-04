//! Registered custom properties in the cascade (CSS Properties and
//! Values API 1 §2): the registrations of one cascade run, and what
//! they do to an element's custom-property map — the initial value,
//! `inherits: false`, and syntax validation at computed-value time.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use rdom_core::Dom;

use rdom_style::calc::{CalcUnit, Viewport};
use rdom_style::parse::token::Token;
use rdom_style::{CustomValue, PropertyRegistration};

use crate::ext::TuiExt;
use crate::style::Stylesheet;

type Map = HashMap<String, CustomValue>;

/// One registration, with its initial value tokenized — and, unless it
/// holds a viewport unit, computed (§2.4) — once.
#[derive(Debug)]
struct Entry {
    reg: PropertyRegistration,
    initial: Option<CustomValue>,
    /// The initial value holds a viewport unit: it computes against the
    /// document's viewport at each use.
    initial_needs_viewport: bool,
}

impl Entry {
    fn new(reg: &PropertyRegistration) -> Self {
        let initial = reg.initial_value.as_deref().map(CustomValue::new);
        let initial_needs_viewport = initial.as_ref().is_some_and(needs_viewport);
        let initial = match initial {
            Some(v) if !initial_needs_viewport => {
                Some(reg.syntax.computed(&v, Viewport::default()).unwrap_or(v))
            }
            other => other,
        };
        Entry {
            reg: reg.clone(),
            initial,
            initial_needs_viewport,
        }
    }

    /// The computed initial value in `viewport`.
    fn initial(&self, viewport: Viewport) -> Option<Cow<'_, CustomValue>> {
        let initial = self.initial.as_ref()?;
        if !self.initial_needs_viewport {
            return Some(Cow::Borrowed(initial));
        }
        Some(match self.reg.syntax.computed(initial, viewport) {
            Some(v) => Cow::Owned(v),
            None => Cow::Borrowed(initial),
        })
    }
}

/// Does `value` hold a viewport-percentage unit (CSS Values 4 §6.1.2)?
fn needs_viewport(value: &CustomValue) -> bool {
    value.tokens().is_some_and(|tokens| {
        tokens.iter().any(|t| {
            matches!(t, Token::Dimension { unit, .. }
                if CalcUnit::parse(unit).is_some_and(CalcUnit::needs_context))
        })
    })
}

/// The custom properties registered by a list of sheets, by name; a
/// later registration of a name replaces an earlier one (sheets in
/// cascade order — an `App`'s `CSS.registerProperty` sheet comes last).
///
/// The one registry both the cascade and the transition engine
/// (`runtime::animation`) read. An `App` builds it when its sheets
/// change (`FramePrelude::sheets_changed`) and shares it; the
/// stateless [`CascadeExt`](super::CascadeExt) entry points keep one on
/// the document ([`document_registry`]).
#[derive(Debug, Default)]
pub(crate) struct PropertyRegistry(HashMap<String, Entry>);

/// The document-data slot holding the registry of the sheet set the
/// document was last cascaded with through a stateless form, keyed by
/// each sheet's [`Stylesheet::version`] in order.
struct DocumentRegistry {
    key: Vec<u64>,
    registry: Rc<PropertyRegistry>,
}

/// The registry for `sheets`: the document's (kept as document data)
/// while the same sheets, unchanged, come back — so the stateless
/// cascade forms build it once per sheet set, and the match records
/// stamped with it (`matching::MatchedRules`) stay valid between calls —
/// else a new one, which replaces it.
pub(super) fn document_registry(
    dom: &mut Dom<TuiExt>,
    sheets: &[&Stylesheet],
) -> Rc<PropertyRegistry> {
    let versions = || sheets.iter().map(|s| s.version());
    if let Some(slot) = dom.document_data::<DocumentRegistry>()
        && slot.key.iter().copied().eq(versions())
    {
        return slot.registry.clone();
    }
    let registry = Rc::new(PropertyRegistry::new(sheets));
    dom.set_document_data(DocumentRegistry {
        key: versions().collect(),
        registry: registry.clone(),
    });
    registry
}

impl PropertyRegistry {
    pub(crate) fn new(sheets: &[&Stylesheet]) -> Self {
        #[cfg(test)]
        probe::BUILDS.with(|c| c.set(c.get() + 1));
        let mut map = HashMap::new();
        for sheet in sheets {
            for reg in sheet.registered_properties() {
                map.insert(reg.name.clone(), Entry::new(reg));
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
    pub(super) fn keyword_value(
        &self,
        name: &str,
        unset: bool,
        viewport: Viewport,
    ) -> Option<Option<CustomValue>> {
        let e = self.0.get(name)?;
        if unset && e.reg.inherits {
            return None;
        }
        Some(e.initial(viewport).map(Cow::into_owned))
    }

    /// The registered properties `declared` does not cover: a property
    /// that does not inherit restarts at its initial value; one with no
    /// value at all gets it (§2.1). Writes only what changes.
    pub(super) fn settle_undeclared(
        &self,
        map: &mut std::rc::Rc<Map>,
        declared: &HashSet<&str>,
        viewport: Viewport,
    ) {
        for (name, e) in &self.0 {
            if declared.contains(name.as_str()) {
                continue;
            }
            let current = map.get(name);
            let initial = e.initial(viewport);
            let want = if e.reg.inherits {
                current.or(initial.as_deref())
            } else {
                initial.as_deref()
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
    /// `name` is not registered; when the value matches its syntax, the
    /// syntax's computed value in `viewport` — a `<length>` in absolute
    /// cells, a `<length-percentage>` keeping its percentage (§2.4,
    /// `PropertySyntax::computed`); else invalid at computed-value time,
    /// so `unset` — `inherited`'s value when it inherits, the initial
    /// value otherwise (§2.4). The cascade runs it inside `var()`
    /// resolution, so a property that reads `name` substitutes this
    /// result.
    pub(super) fn computed_value(
        &self,
        name: &str,
        value: Option<CustomValue>,
        inherited: &Map,
        viewport: Viewport,
    ) -> Option<CustomValue> {
        let Some(e) = self.0.get(name) else {
            return value;
        };
        if let Some(v) = value
            && e.reg.syntax.matches(v.as_str())
        {
            return Some(e.reg.syntax.computed(&v, viewport).unwrap_or(v));
        }
        if e.reg.inherits {
            inherited
                .get(name)
                .cloned()
                .or_else(|| e.initial(viewport).map(Cow::into_owned))
        } else {
            e.initial(viewport).map(Cow::into_owned)
        }
    }

    /// The sheet-level variables (the root's parent) before their
    /// `var()`s resolve: every registered property they lack gets its
    /// initial value, so a dependent can read it (§2.1). Validation runs
    /// during resolution ([`computed_value`](Self::computed_value)).
    pub(super) fn seed_root(&self, map: &mut Map, viewport: Viewport) {
        for (name, e) in &self.0 {
            if map.contains_key(name) {
                continue;
            }
            if let Some(v) = e.initial(viewport) {
                map.insert(name.clone(), v.into_owned());
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
