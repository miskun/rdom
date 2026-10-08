//! CSS Animations data model — the `@keyframes` rule (CSS Animations 1
//! §3) and the values of the `animation-*` longhands (§4; CSS
//! Animations 2 §3) the cascade carries into `ComputedStyle`.
//!
//! A sheet keeps every `@keyframes` rule as written, with its cascade
//! layer ([`Stylesheet::keyframes`](crate::Stylesheet::keyframes)); a
//! backend resolves a name to the last rule of that name in layer order
//! and the rule to its keyframes ([`KeyframesRule::resolve`]): one per
//! offset, the blocks that share an offset cascading in source order.
//! When an animation runs — and how its keyframe values are computed
//! and composited — is the backend's (`rdom-tui`'s `runtime::animation`).

use std::sync::Arc;

use crate::TuiStyle;
use crate::transition::TimingFunction;

mod timeline;
mod values;

pub use timeline::{
    RangeBoundary, TimelineAxis, TimelineInset, TimelineName, TimelineRangeName, TimelineScope,
    TimelineScroller,
};
pub use values::{
    AnimationComposition, AnimationDirection, AnimationDuration, AnimationFillMode, AnimationName,
    AnimationPlayState, AnimationTimeline, IterationCount,
};

/// An `@keyframes` rule (CSS Animations 1 §3): a name and its keyframe
/// blocks in source order.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct KeyframesRule {
    /// The `<keyframes-name>`, as written (case-sensitive).
    pub name: Arc<str>,
    /// The cascade layer the rule sits in (`@layer`, CSS Cascade 5
    /// §6.4), as declared in its own sheet; `None` when unlayered.
    pub layer: Option<crate::LayerId>,
    /// The conditional group rule it sits in (`@media`, CSS Conditional 3
    /// §2): it defines its name only while that holds.
    pub condition: Option<crate::ConditionId>,
    /// The keyframe blocks, in source order.
    pub keyframes: Vec<Keyframe>,
}

/// One keyframe block: its selectors and its declarations.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Keyframe {
    /// The `<keyframe-selector>#` — at least one.
    pub selectors: Vec<KeyframeSelector>,
    /// The block's declarations; `!important` ones are dropped by the
    /// parser (§3: they are ignored).
    pub style: TuiStyle,
}

/// One `<keyframe-selector>`: `from` (0%), `to` (100%), a percentage, or
/// a timeline range name and a percentage of it (Scroll-driven
/// Animations 1 §4.4).
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct KeyframeSelector {
    /// The offset: in `[0, 1]` of the animation, or — with `range` — any
    /// fraction of that range.
    offset: f32,
    range: Option<TimelineRangeName>,
}

impl KeyframeSelector {
    /// The selector at `offset`, a fraction in `[0, 1]`; `None` outside
    /// it (§3: a keyframe selector outside 0%–100% is invalid).
    pub fn at(offset: f32) -> Option<KeyframeSelector> {
        (0.0..=1.0).contains(&offset).then_some(KeyframeSelector {
            offset,
            range: None,
        })
    }

    /// The selector at `offset` (a fraction, any finite one) of the
    /// timeline range `range` (`entry 20%`, Scroll-driven Animations 1
    /// §4.4): it may fall outside the animation's own range.
    pub fn in_range(range: TimelineRangeName, offset: f32) -> Option<KeyframeSelector> {
        offset.is_finite().then_some(KeyframeSelector {
            offset,
            range: Some(range),
        })
    }

    /// Its offset: of the animation, or of [`range`](Self::range).
    pub fn offset(&self) -> f32 {
        self.offset
    }

    /// The timeline range its offset is in, if it names one.
    pub fn range(&self) -> Option<TimelineRangeName> {
        self.range
    }
}

impl Keyframe {
    /// The block for `selectors` (one at least) with `style`.
    pub fn new(selectors: Vec<KeyframeSelector>, style: TuiStyle) -> Keyframe {
        Keyframe { selectors, style }
    }

    /// The block's `animation-timing-function` (§3: the easing from this
    /// keyframe to the next one), if it declares one.
    pub fn easing(&self) -> Option<&TimingFunction> {
        match self.style.animation_timing_function.as_ref()? {
            crate::Value::Specified(list) => list.first(),
            _ => None,
        }
    }

    /// The block's `animation-composition` (CSS Animations 2 §3.2: the
    /// keyframe's composite operation), if it declares one.
    pub fn composition(&self) -> Option<AnimationComposition> {
        match self.style.animation_composition.as_ref()? {
            crate::Value::Specified(list) => list.first().copied(),
            _ => None,
        }
    }
}

/// The keyframe at one offset of a resolved rule
/// ([`KeyframesRule::resolve`]).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ResolvedKeyframe<'a> {
    /// The offset: in `[0, 1]` of the animation, or of `range`.
    pub offset: f32,
    /// The timeline range the offset is in, if the selector names one.
    pub range: Option<TimelineRangeName>,
    /// The blocks whose selectors name this offset, in source order: they
    /// cascade, a later block's declaration winning (§3).
    pub blocks: Vec<&'a Keyframe>,
}

impl ResolvedKeyframe<'_> {
    /// The easing from this keyframe to the next: the last block's that
    /// declares one.
    pub fn easing(&self) -> Option<&TimingFunction> {
        self.blocks.iter().rev().find_map(|k| k.easing())
    }

    /// The composite operation of this keyframe: the last block's that
    /// declares one.
    pub fn composition(&self) -> Option<AnimationComposition> {
        self.blocks.iter().rev().find_map(|k| k.composition())
    }
}

impl KeyframesRule {
    /// An empty rule named `name`.
    pub fn new(name: impl Into<Arc<str>>) -> KeyframesRule {
        KeyframesRule {
            name: name.into(),
            layer: None,
            condition: None,
            keyframes: Vec::new(),
        }
    }

    /// This rule inside conditional group rule `condition`.
    pub fn in_condition(mut self, condition: Option<crate::ConditionId>) -> KeyframesRule {
        self.condition = condition;
        self
    }

    /// This rule in the cascade layer `layer` (`None`: unlayered).
    pub fn in_layer(mut self, layer: Option<crate::LayerId>) -> KeyframesRule {
        self.layer = layer;
        self
    }

    /// This rule with `keyframe` appended.
    pub fn with(mut self, keyframe: Keyframe) -> KeyframesRule {
        self.keyframes.push(keyframe);
        self
    }

    /// The keyframes by offset (CSS Animations 1 §3, "To determine the
    /// set of keyframes, all of the values in the selectors are sorted in
    /// increasing order by time. The rules within the @keyframes rule
    /// then cascade"): one per distinct offset, ascending, each with the
    /// blocks naming it in source order.
    pub fn resolve(&self) -> Vec<ResolvedKeyframe<'_>> {
        let mut out: Vec<ResolvedKeyframe<'_>> = Vec::new();
        for k in &self.keyframes {
            for s in &k.selectors {
                match out
                    .iter_mut()
                    .find(|r| r.offset == s.offset && r.range == s.range)
                {
                    Some(r) => {
                        if !r.blocks.iter().any(|b| std::ptr::eq(*b, k)) {
                            r.blocks.push(k);
                        }
                    }
                    None => out.push(ResolvedKeyframe {
                        offset: s.offset,
                        range: s.range,
                        blocks: vec![k],
                    }),
                }
            }
        }
        out.sort_by(|a, b| a.offset.total_cmp(&b.offset));
        out
    }
}

#[cfg(test)]
mod tests;
