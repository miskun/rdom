//! A CSS animation's keyframe effect (CSS Animations 1 §3, Web
//! Animations 1 §5.3): per animated longhand, its property-specific
//! keyframes — the keyframes that declare it, with the element's own
//! value at 0% and 100% where none does — and sampling them at a
//! progress onto a style.
//!
//! Each keyframe's values are computed once, when the effect is built
//! (`cascade::keyframe_style`: the element's cascade with the keyframe's
//! blocks in the animation origin); sampling only interpolates.

use std::rc::Rc;

use crate::style::animation::Longhand;
use crate::style::{
    AnimationComposition, ComputedStyle, KeyframesRule, TuiStyle, transition::TimingFunction,
};
use rdom_style::color::ColorScheme;
use rdom_style::layout::TextDirection;

/// One keyframe of one longhand.
#[derive(Debug, Clone)]
struct Frame {
    /// The offset in `[0, 1]`.
    offset: f64,
    /// The keyframe's computed style; `None` for an implicit keyframe —
    /// the underlying value (§3: "the computed values of the properties
    /// being animated").
    value: Option<Rc<ComputedStyle>>,
    /// The easing to the next keyframe.
    easing: TimingFunction,
    /// The keyframe's composite operation (CSS Animations 2 §3.2).
    composite: AnimationComposition,
}

/// One animated longhand and its keyframes, by offset.
#[derive(Debug, Clone)]
struct PropertyFrames {
    longhand: Longhand,
    frames: Vec<Frame>,
}

/// The keyframes of one CSS animation, per longhand.
#[derive(Debug, Clone, Default)]
pub(crate) struct KeyframeEffect {
    properties: Vec<PropertyFrames>,
}

impl KeyframeEffect {
    /// The effect of `rule` for an element whose `direction` maps the
    /// flow-relative declarations: `style_of` computes a keyframe's style
    /// from its blocks (`None`: no box, the keyframe is skipped);
    /// `easing` / `composite` are the animation's, for the keyframes that
    /// declare none.
    pub(crate) fn build(
        rule: &KeyframesRule,
        easing: &TimingFunction,
        composite: AnimationComposition,
        direction: TextDirection,
        style_of: &mut dyn FnMut(&[&TuiStyle]) -> Option<ComputedStyle>,
    ) -> KeyframeEffect {
        let mut properties: Vec<PropertyFrames> = Vec::new();
        for keyframe in rule.resolve() {
            let blocks: Vec<&TuiStyle> = keyframe.blocks.iter().map(|k| &k.style).collect();
            // §3: a keyframe animates the animatable properties it names;
            // the others (the `animation-*` longhands among them) are
            // ignored.
            let declared: Vec<Longhand> = Longhand::all()
                .filter(|l| l.is_animatable() && blocks.iter().any(|b| l.declared_in(b, direction)))
                .collect();
            if declared.is_empty() {
                continue;
            }
            let Some(style) = style_of(&blocks) else {
                continue;
            };
            let frame = Frame {
                offset: f64::from(keyframe.offset),
                value: Some(Rc::new(style)),
                easing: keyframe.easing().cloned().unwrap_or_else(|| easing.clone()),
                composite: keyframe.composition().unwrap_or(composite),
            };
            for l in declared {
                match properties.iter_mut().find(|p| p.longhand == l) {
                    Some(p) => p.frames.push(frame.clone()),
                    None => properties.push(PropertyFrames {
                        longhand: l,
                        frames: vec![frame.clone()],
                    }),
                }
            }
        }
        // §3: "If a 0% or from keyframe is not specified, the user agent
        // constructs one using the computed values of the properties
        // being animated" — and a 100% one likewise.
        let implicit = |offset| Frame {
            offset,
            value: None,
            easing: easing.clone(),
            composite,
        };
        for p in &mut properties {
            if p.frames.first().is_none_or(|f| f.offset > 0.0) {
                p.frames.insert(0, implicit(0.0));
            }
            if p.frames.last().is_none_or(|f| f.offset < 1.0) {
                p.frames.push(implicit(1.0));
            }
        }
        KeyframeEffect { properties }
    }

    /// The longhands it animates.
    pub(crate) fn longhands(&self) -> impl Iterator<Item = Longhand> + '_ {
        self.properties.iter().map(|p| p.longhand)
    }

    /// Write each longhand's value at `progress` into `out`, `underlying`
    /// being the value below this effect (Web Animations 1 §5.3.3).
    pub(crate) fn apply(
        &self,
        progress: f64,
        scheme: ColorScheme,
        underlying: &ComputedStyle,
        out: &mut ComputedStyle,
    ) {
        for p in &self.properties {
            let frames = &p.frames;
            // The interval endpoints: the last keyframe at or before the
            // progress (and before 1), and the one after it.
            let start = frames
                .iter()
                .rposition(|f| f.offset <= progress && f.offset < 1.0)
                .unwrap_or(0)
                .min(frames.len().saturating_sub(2));
            let (a, b) = (&frames[start], &frames[start + 1]);
            let span = b.offset - a.offset;
            let local = if span > 0.0 {
                (progress - a.offset) / span
            } else {
                1.0
            };
            let eased = f64::from(a.easing.ease(local as f32));
            let from = endpoint(p.longhand, a, underlying, scheme);
            let to = endpoint(p.longhand, b, underlying, scheme);
            p.longhand.interpolate(&from, &to, eased, scheme, out);
        }
    }
}

/// A keyframe's value composited onto `underlying` (Web Animations 1
/// §5.4.4): its own under `replace`, `underlying + value` under `add` /
/// `accumulate` (which agree for every type rdom adds) — the value
/// alone where the type does not add; the underlying value for an
/// implicit keyframe.
fn endpoint<'a>(
    longhand: Longhand,
    frame: &'a Frame,
    underlying: &'a ComputedStyle,
    scheme: ColorScheme,
) -> std::borrow::Cow<'a, ComputedStyle> {
    use std::borrow::Cow;
    let Some(value) = frame.value.as_deref() else {
        return Cow::Borrowed(underlying);
    };
    if frame.composite == AnimationComposition::Replace {
        return Cow::Borrowed(value);
    }
    let mut sum = value.clone();
    if longhand.add(underlying, value, scheme, &mut sum) {
        Cow::Owned(sum)
    } else {
        Cow::Borrowed(value)
    }
}
