//! The declarations one element's cascade ladder applies ([`Declarations`]:
//! its matched rules, inline style, presentational hints and keyframe
//! blocks) and their `var()`-substituted forms ([`Substituted`], CSS
//! Variables 1 §3), prepared with the element's custom properties
//! ([`prepare`]).

use std::collections::HashMap;

use super::{Plan, Source, Step};
use crate::style::{ComputedStyle, Rule, RuleOrigin, TuiStyle};

/// The declarations of one element's cascade: its matched rules
/// (sorted ascending by specificity, then source order), each rule's
/// layer rank (parallel to `sorted`), its inline style, and — when any
/// of them holds `var()` — their substituted forms.
#[derive(Clone, Copy)]
pub(in crate::style::cascade) struct Declarations<'a> {
    pub sorted: &'a [&'a Rule],
    pub ranks: &'a [u32],
    pub inline: Option<&'a TuiStyle>,
    /// The element's presentational hints ([`Source::Hint`]).
    pub hints: Option<&'a TuiStyle>,
    /// A keyframe's declaration blocks ([`Source::Animation`]), in order.
    pub animation: &'a [&'a TuiStyle],
    pub substituted: Option<&'a Substituted>,
    /// The element's `direction`, which picks each rule's
    /// [`Rule::directional_overlay`].
    pub direction: crate::layout::TextDirection,
}

impl<'a> Declarations<'a> {
    pub(in crate::style::cascade) fn new(
        sorted: &'a [&'a Rule],
        ranks: &'a [u32],
        inline: Option<&'a TuiStyle>,
    ) -> Self {
        Declarations {
            sorted,
            ranks,
            inline,
            hints: None,
            animation: &[],
            substituted: None,
            direction: crate::layout::TextDirection::Ltr,
        }
    }

    /// These declarations with the element's presentational hints.
    pub(in crate::style::cascade) fn with_hints(self, hints: Option<&'a TuiStyle>) -> Self {
        Declarations { hints, ..self }
    }

    /// These declarations with a keyframe's blocks
    /// ([`Source::Animation`]).
    pub(in crate::style::cascade) fn with_animation(self, animation: &'a [&'a TuiStyle]) -> Self {
        Declarations { animation, ..self }
    }

    /// Whether a block holds an inline-axis flow-relative property (CSS
    /// Logical 1), which maps by the element's `direction`.
    pub(in crate::style::cascade) fn has_directional(self) -> bool {
        let any = |s: &TuiStyle| s.pending.iter().any(|d| d.directional);
        self.sorted.iter().any(|r| any(&r.style))
            || self.inline.is_some_and(any)
            || self.animation.iter().any(|s| any(s))
    }

    /// No matched rule, no hint and no inline style.
    pub(in crate::style::cascade) fn is_empty(self) -> bool {
        self.sorted.is_empty()
            && self.inline.is_none()
            && self.hints.is_none()
            && self.animation.is_empty()
    }

    /// These declarations with `var()` substituted (`None`: none held
    /// `var()`) for an element of `direction`.
    pub(in crate::style::cascade) fn with(
        self,
        substituted: Option<&'a Substituted>,
        direction: crate::layout::TextDirection,
    ) -> Self {
        Declarations {
            substituted,
            direction,
            ..self
        }
    }

    /// The `i`th matched rule's declaration blocks: the rule's own, then
    /// its kept declarations — substituted per element when they hold
    /// `var()` / `attr()`, else the rule's prebuilt form for the
    /// element's direction (`Rule::directional_overlay`) — applying
    /// both in order is applying the whole block.
    fn rule_blocks(self, i: usize) -> impl Iterator<Item = &'a TuiStyle> + 'a {
        let rule = self.sorted[i];
        let overlay = self
            .substituted
            .and_then(|s| s.rules[i].as_ref())
            .or_else(|| rule.directional_overlay(self.direction));
        std::iter::once(&rule.style).chain(overlay.into_iter().flatten())
    }

    /// The inline style's declaration blocks, the same way.
    fn inline_blocks(self) -> impl Iterator<Item = &'a TuiStyle> + 'a {
        let overlay = self.substituted.and_then(|s| s.inline.as_ref());
        self.inline.into_iter().chain(overlay.into_iter().flatten())
    }

    /// The keyframe blocks, the same way (each replays its kept
    /// declarations, substituted when they need it).
    fn animation_blocks(self) -> impl Iterator<Item = &'a TuiStyle> + 'a {
        self.animation
            .iter()
            .enumerate()
            .flat_map(move |(i, block)| {
                let overlay = self
                    .substituted
                    .and_then(|s| s.animation.get(i))
                    .and_then(Option::as_ref);
                std::iter::once(*block).chain(overlay.into_iter().flatten())
            })
    }

    /// Every declaration block `step` applies, in order.
    pub(in crate::style::cascade) fn of(
        self,
        step: &Step,
    ) -> impl Iterator<Item = &'a TuiStyle> + 'a {
        let source = step.source;
        let rules = (0..self.sorted.len())
            .filter(move |&i| {
                let r = self.sorted[i];
                match source {
                    Source::UserAgent => r.origin == RuleOrigin::UserAgent,
                    Source::Author(layer) => {
                        r.origin == RuleOrigin::Author && self.ranks[i] == layer
                    }
                    Source::Hint | Source::Inline | Source::Animation => false,
                }
            })
            .flat_map(move |i| self.rule_blocks(i));
        let hints = self.hints.filter(|_| source == Source::Hint);
        let inline = (source == Source::Inline)
            .then(|| self.inline_blocks())
            .into_iter()
            .flatten();
        let animation = (source == Source::Animation)
            .then(|| self.animation_blocks())
            .into_iter()
            .flatten();
        hints
            .into_iter()
            .chain(rules)
            .chain(inline)
            .chain(animation)
    }

    /// Every declaration block: hints, rules, inline, then the keyframe's.
    pub(in crate::style::cascade) fn all(self) -> impl Iterator<Item = &'a TuiStyle> + 'a {
        self.hints.into_iter().chain(
            (0..self.sorted.len())
                .flat_map(move |i| self.rule_blocks(i))
                .chain(self.inline_blocks())
                .chain(self.animation_blocks()),
        )
    }
}

/// The `var()`-substituted declarations of one element (CSS Variables 1
/// §3), parallel to [`Declarations::sorted`]: per block, only its
/// pending declarations substituted ([`TuiStyle::substituted_pending`]),
/// applied after the block itself — no copy of the block. `None` where a
/// block holds no `var()`.
pub(in crate::style::cascade) struct Substituted {
    /// Per block, its normal then its `!important` kept declarations
    /// ([`TuiStyle::substituted_pending_split`]).
    rules: Vec<Option<[TuiStyle; 2]>>,
    inline: Option<[TuiStyle; 2]>,
    /// Per keyframe block (`Declarations::animation`).
    animation: Vec<Option<[TuiStyle; 2]>>,
}

impl Substituted {
    /// Substitute the blocks of `decls` that hold `var()` from `vars`
    /// (the element's custom properties); `None` when none does, which
    /// is the common case and costs one scan. A rule whose kept
    /// declarations are only direction-mapped (no `var()` / `attr()`)
    /// needs nothing here: its prebuilt form applies
    /// (`Rule::directional_overlay`). An inline style is per element
    /// anyway, and replays here.
    pub(in crate::style::cascade) fn new(
        decls: Declarations<'_>,
        vars: &crate::style::VarMap,
        attrs: rdom_style::backend::AttrLookup<'_>,
        direction: crate::layout::TextDirection,
    ) -> Option<Self> {
        let any = decls.sorted.iter().any(|r| r.style.needs_substitution())
            || decls.inline.is_some_and(TuiStyle::has_pending)
            || decls.animation.iter().any(|s| s.has_pending());
        if !any {
            return None;
        }
        let cx = rdom_style::backend::SubstitutionContext::new()
            .with_attrs(attrs)
            .with_direction(direction);
        let rule = |s: &TuiStyle| {
            s.needs_substitution()
                .then(|| s.substituted_pending_split(vars, &cx))
        };
        Some(Substituted {
            rules: decls.sorted.iter().map(|r| rule(&r.style)).collect(),
            inline: decls.inline.and_then(|s| {
                s.has_pending()
                    .then(|| s.substituted_pending_split(vars, &cx))
            }),
            animation: decls
                .animation
                .iter()
                .map(|s| {
                    s.has_pending()
                        .then(|| s.substituted_pending_split(vars, &cx))
                })
                .collect(),
        })
    }
}

/// The custom properties of `decls` folded into `working.vars`, then
/// the declarations' `var()`s substituted from them and their `attr()`s
/// from `attrs` (the element's — for a pseudo-element, its originating
/// element's — attributes, CSS Values 5 §8.7), and their inline-axis
/// flow-relative properties mapped by `working.text_direction` (CSS
/// Logical 1 §4) — the inputs of [`apply_cascade_ladder`].
pub(in crate::style::cascade) fn prepare(
    working: &mut ComputedStyle,
    plan: &Plan,
    decls: Declarations<'_>,
    registry: &super::super::registered::PropertyRegistry,
    transitions: Option<&HashMap<String, rdom_style::CustomValue>>,
    attrs: rdom_style::backend::AttrLookup<'_>,
    units: super::super::registered::ViewportUse<'_>,
) -> Option<Substituted> {
    // CSS Variables 1 §2 — same ladder, folded into the element's own
    // map before any `var()` consumer runs.
    super::super::custom::apply_custom_properties(
        working,
        plan,
        decls,
        registry,
        transitions,
        attrs,
        units,
    );
    let vars = working.animated_vars.as_ref().unwrap_or(&working.vars);
    Substituted::new(decls, vars, attrs, working.text_direction)
}
