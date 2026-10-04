//! The cascade ladder (CSS Cascade 4 §6.1 / Cascade 5 §6.4): the
//! order in which an element's declarations apply, and the rollback
//! states `revert` needs.
//!
//! The ladder is a [`Plan`] of [`Step`]s, each one origin + importance
//! (+ cascade layer) group applied in specificity / source order on
//! top of the previous:
//!
//! 1. UA normal, Author normal (one step per layer, unlayered last),
//!    Inline normal,
//! 2. Author important (layers reversed), Inline important, UA
//!    important.
//!
//! `!important` inverts origin priority, matching CSS. Don't shortcut
//! the ladder — the inversion is observable and tests depend on it.
//! Within the author origin, the element-attached `style` attribute
//! wins over rule declarations at *both* importances (Cascade 4 §6.1
//! "Element-Attached Styles"), and that criterion sorts above cascade
//! layers (Cascade 5 §6.1), so inline sits after every author layer in
//! the normal half and after every author layer in the important half.
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
//! declarations hold no `revert` pays nothing. `revert-layer` (Cascade
//! 5 §7.4) reads the same memo, at the step's own start
//! ([`Step::revert_layer_to`]).

use std::cell::OnceCell;

use super::apply::{Initials, Keywords, apply_style};
use super::inherit::inherit_inheritable_from;
use crate::style::{ComputedStyle, Rule, RuleOrigin, TuiStyle};

/// Which declarations one ladder step applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Source {
    /// Matched rules of the user-agent origin.
    UserAgent,
    /// Matched rules of the author origin in the cascade layer of this
    /// rank (`rdom_style::LayerOrder`; `UNLAYERED` for unlayered
    /// rules).
    Author(u32),
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
    /// The step whose starting state a `revert-layer` rolls back to:
    /// this step's own start (the cascade without this layer and the
    /// ones above it, Cascade 5 §7.4) for author and inline
    /// declarations, `0` for the UA origin, which has no layers.
    pub revert_layer_to: usize,
}

/// The ladder for one element, in application order.
pub(super) struct Plan(Vec<Step>);

impl Plan {
    /// The ladder of CSS Cascade 5 §6.1 for an element whose matched
    /// author rules sit in the layers of `author_ranks` (any order,
    /// duplicates allowed): UA normal; author normal, one step per
    /// layer from the lowest rank up, unlayered last; inline normal;
    /// author important with the layer order reversed (unlayered
    /// first); inline important; UA important.
    pub(super) fn new(author_ranks: impl IntoIterator<Item = u32>) -> Plan {
        let mut ranks: Vec<u32> = author_ranks.into_iter().collect();
        ranks.sort_unstable();
        ranks.dedup();
        let mut steps = Vec::with_capacity(2 * ranks.len() + 4);
        let mut push = |source, important| {
            let own = steps.len();
            let (revert_to, revert_layer_to) = match source {
                Source::UserAgent => (0, 0),
                Source::Author(_) | Source::Inline => (1, own),
            };
            steps.push(Step {
                source,
                important,
                revert_to,
                revert_layer_to,
            });
        };
        push(Source::UserAgent, false);
        for &rank in &ranks {
            push(Source::Author(rank), false);
        }
        push(Source::Inline, false);
        for &rank in ranks.iter().rev() {
            push(Source::Author(rank), true);
        }
        push(Source::Inline, true);
        push(Source::UserAgent, true);
        Plan(steps)
    }

    pub(super) fn steps(&self) -> &[Step] {
        &self.0
    }
}

/// The declarations of one element's cascade: its matched rules
/// (sorted ascending by specificity, then source order), each rule's
/// layer rank (parallel to `sorted`), its inline style, and — when any
/// of them holds `var()` — their substituted forms.
#[derive(Clone, Copy)]
pub(super) struct Declarations<'a> {
    pub sorted: &'a [&'a Rule],
    pub ranks: &'a [u32],
    pub inline: Option<&'a TuiStyle>,
    pub substituted: Option<&'a Substituted>,
}

impl<'a> Declarations<'a> {
    pub(super) fn new(
        sorted: &'a [&'a Rule],
        ranks: &'a [u32],
        inline: Option<&'a TuiStyle>,
    ) -> Self {
        Declarations {
            sorted,
            ranks,
            inline,
            substituted: None,
        }
    }

    /// These declarations with `var()` substituted (`None`: none held
    /// `var()`).
    pub(super) fn with(self, substituted: Option<&'a Substituted>) -> Self {
        Declarations {
            substituted,
            ..self
        }
    }

    /// The `i`th matched rule's declarations, substituted.
    fn rule_style(self, i: usize) -> &'a TuiStyle {
        self.substituted
            .and_then(|s| s.rules[i].as_ref())
            .unwrap_or(&self.sorted[i].style)
    }

    fn inline_style(self) -> Option<&'a TuiStyle> {
        match self.substituted.and_then(|s| s.inline.as_ref()) {
            Some(style) => Some(style),
            None => self.inline,
        }
    }

    /// Every declaration block `step` applies, in order.
    pub(super) fn of(self, step: &Step) -> impl Iterator<Item = &'a TuiStyle> + 'a {
        let source = step.source;
        let rules = (0..self.sorted.len())
            .filter(move |&i| {
                let r = self.sorted[i];
                match source {
                    Source::UserAgent => r.origin == RuleOrigin::UserAgent,
                    Source::Author(layer) => {
                        r.origin == RuleOrigin::Author && self.ranks[i] == layer
                    }
                    Source::Inline => false,
                }
            })
            .map(move |i| self.rule_style(i));
        let inline = self.inline_style().filter(|_| source == Source::Inline);
        rules.chain(inline)
    }

    /// Every declaration block, rules then inline.
    pub(super) fn all(self) -> impl Iterator<Item = &'a TuiStyle> + 'a {
        (0..self.sorted.len())
            .map(move |i| self.rule_style(i))
            .chain(self.inline_style())
    }
}

/// The `var()`-substituted declaration blocks of one element (CSS
/// Variables 1 §3), parallel to [`Declarations::sorted`]; `None` where
/// a block holds no `var()`.
pub(super) struct Substituted {
    rules: Vec<Option<TuiStyle>>,
    inline: Option<TuiStyle>,
}

impl Substituted {
    /// Substitute the blocks of `decls` that hold `var()` from `vars`
    /// (the element's custom properties); `None` when none does, which
    /// is the common case and costs one scan.
    pub(super) fn new(decls: Declarations<'_>, vars: &crate::style::VarMap) -> Option<Self> {
        let any = decls.sorted.iter().any(|r| r.style.has_pending())
            || decls.inline.is_some_and(TuiStyle::has_pending);
        if !any {
            return None;
        }
        let sub = |s: &TuiStyle| s.has_pending().then(|| s.substituted(vars));
        Some(Substituted {
            rules: decls.sorted.iter().map(|r| sub(&r.style)).collect(),
            inline: decls.inline.and_then(sub),
        })
    }
}

/// The custom properties of `decls` folded into `working.vars`, then
/// the declarations' `var()`s substituted from them — the inputs of
/// [`apply_cascade_ladder`].
pub(super) fn prepare(
    working: &mut ComputedStyle,
    plan: &Plan,
    decls: Declarations<'_>,
) -> Option<Substituted> {
    // CSS Variables 1 §2 — same ladder, folded into the element's own
    // map before any `var()` consumer runs.
    super::custom::apply_custom_properties(working, plan, decls);
    Substituted::new(decls, &working.vars)
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
    // Custom properties are already in `working.vars` (`prepare`).
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
        revert_layer: &|| rollback.state_before(step.revert_layer_to),
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
        let plan = Plan::new([]);
        for step in plan.steps() {
            let want = match step.source {
                Source::UserAgent => 0,
                Source::Author(_) | Source::Inline => 1,
            };
            assert_eq!(step.revert_to, want, "{step:?}");
        }
        assert_eq!(plan.steps()[0].source, Source::UserAgent);
        assert!(!plan.steps()[0].important);
    }

    /// Cascade 5 §6.4: normal author steps run layer by layer, lowest
    /// rank first and unlayered last; important ones in the reverse
    /// order; inline follows every author step of its importance
    /// (Cascade 4 §6.1 element-attached styles sort above layers);
    /// `revert-layer` rolls back to the step's own start.
    #[test]
    fn layers_order_the_author_steps() {
        const U: u32 = rdom_style::LayerOrder::UNLAYERED;
        let plan = Plan::new([U, 3, 1, 3]);
        let order: Vec<(Source, bool)> = plan
            .steps()
            .iter()
            .map(|s| (s.source, s.important))
            .collect();
        assert_eq!(
            order,
            vec![
                (Source::UserAgent, false),
                (Source::Author(1), false),
                (Source::Author(3), false),
                (Source::Author(U), false),
                (Source::Inline, false),
                (Source::Author(U), true),
                (Source::Author(3), true),
                (Source::Author(1), true),
                (Source::Inline, true),
                (Source::UserAgent, true),
            ]
        );
        for (i, step) in plan.steps().iter().enumerate() {
            let want = if step.source == Source::UserAgent {
                0
            } else {
                i
            };
            assert_eq!(step.revert_layer_to, want, "{step:?}");
        }
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
