//! The conditional group rules of one sheet set, evaluated (CSS
//! Conditional 3 §2): which `@media` rules hold in the document's media
//! environment.
//!
//! The results are computed once per environment and kept on the sheet
//! set (`SheetFacts`): a cascade under the same viewport, scheme and
//! preferences reuses them, and an environment that flips no condition
//! keeps the same results — the same `Rc` — so the matches recorded under
//! them (`matching::MatchedRules`) stay valid, and the `App` can tell a
//! resize that flipped a query from one that did not.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_style::conditional::MediaEnvironment;
use rdom_style::{ConditionId, ConditionKind, Stylesheet};

/// Whether each conditional group rule of each sheet holds — itself and
/// every rule enclosing it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ConditionResults {
    per_sheet: Vec<Box<[bool]>>,
}

impl ConditionResults {
    fn evaluate(list: &[&Stylesheet], env: &MediaEnvironment) -> Self {
        #[cfg(test)]
        probe::EVALUATIONS.with(|c| c.set(c.get() + 1));
        let per_sheet = list
            .iter()
            .map(|sheet| {
                let mut holds: Vec<bool> = Vec::with_capacity(sheet.conditions().len());
                for rule in sheet.conditions() {
                    // A rule is declared after the one enclosing it.
                    let parent = rule.parent.is_none_or(|p| holds[p.index()]);
                    holds.push(parent && own(&rule.kind, env));
                }
                holds.into_boxed_slice()
            })
            .collect();
        ConditionResults { per_sheet }
    }

    /// Whether `condition` of sheet `sheet` holds (`None`: unconditional).
    pub(super) fn holds(&self, sheet: usize, condition: Option<ConditionId>) -> bool {
        match condition {
            None => true,
            Some(c) => self
                .per_sheet
                .get(sheet)
                .and_then(|s| s.get(c.index()))
                .copied()
                .unwrap_or(false),
        }
    }
}

/// Whether one rule's own condition holds.
fn own(kind: &ConditionKind, env: &MediaEnvironment) -> bool {
    match kind {
        ConditionKind::Media(queries) => queries.matches(env),
        // `ConditionKind` is open: a kind this cascade does not know
        // holds nothing, rather than applying rules it cannot test.
        #[allow(unreachable_patterns)]
        _ => false,
    }
}

/// The last results of a sheet set and the environment they were
/// computed in.
#[derive(Debug, Default)]
pub(super) struct ConditionCache {
    last: RefCell<Option<(MediaEnvironment, Rc<ConditionResults>)>>,
}

impl ConditionCache {
    /// The results for `list` in `env`: the cached ones when `env` is
    /// theirs, or when evaluating again changes nothing.
    pub(super) fn get(&self, list: &[&Stylesheet], env: &MediaEnvironment) -> Rc<ConditionResults> {
        let mut last = self.last.borrow_mut();
        if let Some((seen, results)) = last.as_mut() {
            if seen == env {
                return results.clone();
            }
            let fresh = ConditionResults::evaluate(list, env);
            *seen = *env;
            if fresh != **results {
                *results = Rc::new(fresh);
            }
            return results.clone();
        }
        let results = Rc::new(ConditionResults::evaluate(list, env));
        *last = Some((*env, results.clone()));
        results
    }
}

/// Test-only: how many times a sheet set's conditions were evaluated.
#[cfg(test)]
pub(crate) mod probe {
    thread_local! {
        pub static EVALUATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    pub fn take() -> usize {
        EVALUATIONS.with(|c| c.replace(0))
    }
}
