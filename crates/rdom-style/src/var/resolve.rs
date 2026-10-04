//! Custom-property resolution (CSS Variables 1 §2.3, §3): a custom
//! property's own `var()`s and `attr()`s are substituted where it is
//! declared, against the other custom properties being resolved.

use std::collections::{HashMap, HashSet};

use super::{CustomValue, SubstitutionContext, SubstitutionError, lookup_in, substitute_at};

/// A computed-value step: maps a custom property's substituted value
/// (`None`: guaranteed-invalid) to its computed value, which its
/// dependents then substitute. The cascade passes the registered-property
/// step (CSS Properties and Values API 1 §2.4: a value not matching the
/// syntax is invalid at computed-value time; a matching one computes,
/// a `<length>` to absolute cells).
pub type ComputedStep<'c> = &'c mut dyn FnMut(&str, Option<CustomValue>) -> Option<CustomValue>;

/// Substitute the custom properties `names` of `vars` in place (§2.3,
/// §3): each one's `var()`s resolve against `vars` — another of `names`
/// resolved first — and its `attr()`s against `cx`'s attributes; one
/// that cannot (no value and no fallback, a dependency cycle, which
/// takes every property on it, or an over-long result) is removed: the
/// guaranteed-invalid value. With a computed-value step in `cx`, every
/// one of `names` goes through it, a substitution or not; without, only
/// the ones holding a substitution function are touched.
///
/// Returns the declared properties whose substitution failed, with why
/// — the cascade's explanation of an invalid custom property.
pub fn resolve_custom_properties<'n>(
    vars: &mut HashMap<String, CustomValue>,
    names: impl IntoIterator<Item = &'n str>,
    cx: SubstitutionContext<'_, '_>,
) -> Vec<(String, SubstitutionError)> {
    let pending: Vec<String> = match cx.computed {
        Some(_) => names.into_iter().map(str::to_string).collect(),
        None => names
            .into_iter()
            .filter(|n| vars.get(*n).is_some_and(CustomValue::has_substitution))
            .map(str::to_string)
            .collect(),
    };
    if pending.is_empty() {
        return Vec::new();
    }
    let mut resolver = Resolver {
        pending: pending.iter().cloned().collect(),
        done: HashMap::new(),
        stack: Vec::new(),
        cyclic: HashSet::new(),
        failures: Vec::new(),
        cx,
    };
    for name in &pending {
        // The outcome is recorded in `done`.
        let _ = resolver.resolve(name, vars);
    }
    for (name, value) in resolver.done {
        match value {
            Ok(v) => {
                vars.insert(name, v);
            }
            Err(_) => {
                vars.remove(&name);
            }
        }
    }
    resolver.failures
}

struct Resolver<'a, 'c> {
    /// The names still to substitute.
    pending: HashSet<String>,
    /// Substituted (and computed) values.
    done: HashMap<String, Result<CustomValue, SubstitutionError>>,
    /// The names being substituted, outermost first.
    stack: Vec<String>,
    /// Names found on a dependency cycle.
    cyclic: HashSet<String>,
    /// The declared properties whose substitution failed.
    failures: Vec<(String, SubstitutionError)>,
    cx: SubstitutionContext<'a, 'c>,
}

impl Resolver<'_, '_> {
    fn resolve(
        &mut self,
        name: &str,
        vars: &HashMap<String, CustomValue>,
    ) -> Result<CustomValue, SubstitutionError> {
        if let Some(done) = self.done.get(name) {
            return done.clone();
        }
        if let Some(at) = self.stack.iter().position(|n| n == name) {
            // A cycle: every property from `name` up is on it.
            self.cyclic.extend(self.stack[at..].iter().cloned());
            return Err(SubstitutionError::Cycle(name.to_string()));
        }
        self.stack.push(name.to_string());
        let declared = vars.get(name);
        let result = match declared {
            // No substitution function: the value as declared, untouched.
            Some(v) if !v.has_substitution() => Ok(v.clone()),
            Some(v) => match v.tokens() {
                Some(tokens) => {
                    let attrs = self.cx.attrs;
                    substitute_at(
                        tokens,
                        0,
                        None,
                        &mut |n| {
                            if self.pending.contains(n) {
                                self.resolve(n, vars)
                            } else {
                                lookup_in(vars, n)
                            }
                        },
                        attrs,
                    )
                    // Tokenized here, once: the substitution is the tokens.
                    .map(CustomValue::from_tokens)
                }
                None => Err(SubstitutionError::Undefined(name.to_string())),
            },
            None => Err(SubstitutionError::Undefined(name.to_string())),
        };
        self.stack.pop();
        let cyclic = self.cyclic.contains(name);
        let result = if cyclic {
            Err(SubstitutionError::Cycle(name.to_string()))
        } else {
            result
        };
        if let (Some(_), Err(e)) = (declared, &result) {
            self.failures.push((name.to_string(), e.clone()));
        }
        let value = match self.cx.computed.as_mut() {
            Some(computed) => computed(name, result.ok()),
            None => result.ok(),
        };
        // A dependent reading an invalid property sees why it has no value.
        let value = value.ok_or_else(|| {
            if cyclic {
                SubstitutionError::Cycle(name.to_string())
            } else {
                SubstitutionError::Undefined(name.to_string())
            }
        });
        self.done.insert(name.to_string(), value.clone());
        value
    }
}
