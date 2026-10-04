//! The cascade ladder (CSS Cascade 4 §6.1 / Cascade 5 §6.4): the
//! order in which an element's declarations apply, and the rollback
//! states `revert` needs.
//!
//! The ladder is a [`Plan`] of [`Step`]s, each one origin + importance
//! group applied in specificity / source order on top of the previous:
//!
//! 1. UA normal, Author normal, Inline normal,
//! 2. Inline important, Author important, UA important.
//!
//! `!important` inverts origin priority, matching CSS. Don't shortcut
//! the ladder — the inversion is observable and tests depend on it.
//!
//! ## Rollback (`revert`)
//!
//! `revert` (Cascade 4 §7.3) gives a property the value the cascade
//! would produce from the *earlier* origins alone: for an author or
//! inline declaration, the state after UA normal; for a UA declaration,
//! the state before any declaration applied (`unset`). Each step names
//! the step whose *starting* state its `revert` rolls back to
//! ([`Step::revert_to`]). [`Rollback`] computes those states on demand
//! by replaying the plan from the element's base state (initial values
//! plus inherited ones), memoized per step — so an element whose
//! declarations hold no `revert` pays nothing.

use std::cell::OnceCell;

use super::apply::{Initials, Keywords, apply_style};
use super::inherit::inherit_inheritable_from;
use crate::style::{ComputedStyle, Rule, RuleOrigin, TuiStyle};

/// Which declarations one ladder step applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Source {
    /// Matched rules of the user-agent origin.
    UserAgent,
    /// Matched rules of the author origin.
    Author,
    /// The element's `style` attribute.
    Inline,
}

/// One rung of the ladder.
#[derive(Debug, Clone, Copy)]
pub(super) struct Step {
    pub source: Source,
    /// Apply the `!important` declarations (else the normal ones).
    pub important: bool,
    /// The step whose starting state a `revert` in this step rolls
    /// back to: `0` (nothing applied — `unset`) for the UA origin, `1`
    /// (after UA normal) for author and inline declarations.
    pub revert_to: usize,
}

/// The ladder for one element, in application order.
pub(super) struct Plan(Vec<Step>);

impl Plan {
    /// The six-step ladder of Cascade 4 §6.1.
    pub(super) fn new() -> Plan {
        let step = |source, important, revert_to| Step {
            source,
            important,
            revert_to,
        };
        Plan(vec![
            step(Source::UserAgent, false, 0),
            step(Source::Author, false, 1),
            step(Source::Inline, false, 1),
            step(Source::Inline, true, 1),
            step(Source::Author, true, 1),
            step(Source::UserAgent, true, 0),
        ])
    }

    pub(super) fn steps(&self) -> &[Step] {
        &self.0
    }
}

/// The declarations of one element's cascade: its matched rules
/// (sorted ascending by specificity, then source order) and its inline
/// style.
#[derive(Clone, Copy)]
pub(super) struct Declarations<'a> {
    pub sorted: &'a [&'a Rule],
    pub inline: Option<&'a TuiStyle>,
}

impl<'a> Declarations<'a> {
    /// Every declaration block `step` applies, in order.
    pub(super) fn of(self, step: &Step) -> impl Iterator<Item = &'a TuiStyle> + 'a {
        let origin = match step.source {
            Source::UserAgent => Some(RuleOrigin::UserAgent),
            Source::Author => Some(RuleOrigin::Author),
            Source::Inline => None,
        };
        let rules = self
            .sorted
            .iter()
            .filter(move |r| origin == Some(r.origin))
            .map(|r| &r.style);
        let inline = self.inline.filter(|_| origin.is_none());
        rules.chain(inline)
    }
}

/// Memoized rollback states of one element's ladder: `state_before(i)`
/// is the computed style after steps `0..i`, replayed from the base.
pub(super) struct Rollback<'a, S> {
    states: Vec<OnceCell<S>>,
    base: &'a dyn Fn() -> S,
    apply: &'a dyn Fn(&mut S, usize, &Rollback<'a, S>),
}

impl<'a, S: Clone> Rollback<'a, S> {
    pub(super) fn new(
        steps: usize,
        base: &'a dyn Fn() -> S,
        apply: &'a dyn Fn(&mut S, usize, &Rollback<'a, S>),
    ) -> Self {
        Rollback {
            states: (0..=steps).map(|_| OnceCell::new()).collect(),
            base,
            apply,
        }
    }

    /// The state after steps `0..i`. Replays step `i - 1` on a clone
    /// of the state before it; a `revert` inside that step only ever
    /// asks for an earlier state, so the recursion terminates.
    pub(super) fn state_before(&self, i: usize) -> &S {
        self.states[i].get_or_init(|| {
            if i == 0 {
                (self.base)()
            } else {
                let mut s = self.state_before(i - 1).clone();
                (self.apply)(&mut s, i - 1, self);
                s
            }
        })
    }
}

/// Walk the cascade ladder once for this element.
pub(super) fn apply_cascade_ladder(
    working: &mut ComputedStyle,
    plan: &Plan,
    decls: Declarations<'_>,
    parent: &ComputedStyle,
) {
    // 0. Custom properties (CSS Variables 1 §2) — same ladder, folded
    //    into the element's own map before any `var()` consumer runs.
    super::custom::apply_custom_properties(working, plan, decls);
    let initial = Initials::default();
    // The base every rollback replays from: initial values, the
    // inherited ones from `parent`, and this element's custom
    // properties — exactly `working` as it stands now.
    let vars = working.vars.clone();
    let base = || {
        let mut b = ComputedStyle::initial();
        inherit_inheritable_from(&mut b, parent);
        b.vars = vars.clone();
        b
    };
    let apply = |state: &mut ComputedStyle, i: usize, rollback: &Rollback<'_, ComputedStyle>| {
        apply_step(state, &plan.steps()[i], decls, parent, &initial, rollback);
    };
    let rollback = Rollback::new(plan.steps().len(), &base, &apply);
    for step in plan.steps() {
        apply_step(working, step, decls, parent, &initial, &rollback);
    }

    // NOTE — CSS Overflow L3's cross-axis rule ("if one axis is
    // not visible, the visible side behaves as auto") is skipped
    // in v1. Browsers apply it because they know content size at
    // layout time and only show the auto scrollbar when needed.
    // rdom-tui v1 can't (we use `scrollbar-gutter: stable`-style
    // always-reserve), so enforcing the rule would surprise
    // authors writing `overflow-y: scroll` and getting an
    // unexpected horizontal gutter. Each axis is independent.
}

fn apply_step(
    working: &mut ComputedStyle,
    step: &Step,
    decls: Declarations<'_>,
    parent: &ComputedStyle,
    initial: &Initials,
    rollback: &Rollback<'_, ComputedStyle>,
) {
    let keywords = Keywords {
        parent,
        initial,
        revert: &|| rollback.state_before(step.revert_to),
    };
    for style in decls.of(step) {
        apply_style(working, style, step.important, &keywords);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cascade 4 §7.3: `revert` in the user-agent origin acts as
    /// `unset` (the state before any declaration, step 0); in the
    /// author origin — rules and inline style alike — it rolls back to
    /// the user-agent origin (the state after UA normal, step 1).
    #[test]
    fn revert_targets_follow_the_origin() {
        let plan = Plan::new();
        for step in plan.steps() {
            let want = match step.source {
                Source::UserAgent => 0,
                Source::Author | Source::Inline => 1,
            };
            assert_eq!(step.revert_to, want, "{step:?}");
        }
        assert_eq!(plan.steps()[0].source, Source::UserAgent);
        assert!(!plan.steps()[0].important);
    }

    /// `state_before(i)` replays steps `0..i` from the base, once each.
    #[test]
    fn rollback_replays_each_step_once() {
        let calls = std::cell::Cell::new(0);
        let base = || vec![0];
        let apply = |s: &mut Vec<usize>, i: usize, _: &Rollback<'_, Vec<usize>>| {
            calls.set(calls.get() + 1);
            s.push(i + 1);
        };
        let rollback = Rollback::new(3, &base, &apply);
        assert_eq!(rollback.state_before(3), &vec![0, 1, 2, 3]);
        assert_eq!(rollback.state_before(1), &vec![0, 1]);
        assert_eq!(calls.get(), 3);
    }
}
