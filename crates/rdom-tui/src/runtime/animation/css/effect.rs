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
    AnimationComposition, ComputedStyle, KeyframesRule, TimelineRangeName, TuiStyle,
    transition::TimingFunction,
};
use rdom_style::color::ColorScheme;
use rdom_style::layout::TextDirection;

/// One keyframe of one longhand.
#[derive(Debug, Clone)]
struct Frame {
    /// The offset in `[0, 1]` — or, with `range`, a fraction of that
    /// timeline range (Scroll-driven Animations 1 §4.4).
    offset: f64,
    range: Option<TimelineRangeName>,
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
    /// Some keyframe sits on a timeline range: the offsets are placed,
    /// and the implicit keyframes added, at each sample.
    ranged: bool,
}

/// The keyframes of one CSS animation, per longhand.
#[derive(Debug, Clone, Default)]
pub(crate) struct KeyframeEffect {
    properties: Vec<PropertyFrames>,
    /// The animation's easing and composite operation: an implicit
    /// keyframe's.
    easing: TimingFunction,
    composite: AnimationComposition,
}

/// Where a keyframe on a timeline range falls in the animation's range
/// (`range`, the fraction of it) — `None` when the timeline has no such
/// range (Scroll-driven Animations 1 §4.4: the keyframe is ignored).
pub(crate) type RangePlacement<'a> = &'a dyn Fn(TimelineRangeName, f64) -> Option<f64>;

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
                range: keyframe.range,
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
                        ranged: false,
                    }),
                }
            }
        }
        // §3: "If a 0% or from keyframe is not specified, the user agent
        // constructs one using the computed values of the properties
        // being animated" — and a 100% one likewise.
        for p in &mut properties {
            p.ranged = p.frames.iter().any(|f| f.range.is_some());
            if !p.ranged {
                add_implicit(&mut p.frames, easing, composite);
            }
        }
        KeyframeEffect {
            properties,
            easing: easing.clone(),
            composite,
        }
    }

    /// The longhands it animates.
    pub(crate) fn longhands(&self) -> impl Iterator<Item = Longhand> + '_ {
        self.properties.iter().map(|p| p.longhand)
    }

    /// Write each longhand's value at `progress` into `out`, `underlying`
    /// being the value below this effect (Web Animations 1 §5.3.3).
    /// `place` positions the keyframes on timeline ranges; without it
    /// (not a progress timeline) they are ignored.
    pub(crate) fn apply(
        &self,
        progress: f64,
        scheme: ColorScheme,
        underlying: &ComputedStyle,
        out: &mut ComputedStyle,
        place: Option<RangePlacement<'_>>,
    ) {
        for p in &self.properties {
            if !p.ranged {
                sample(p.longhand, &p.frames, progress, scheme, underlying, out);
                continue;
            }
            // §4.4: each keyframe on a range at its place in the
            // animation's range — outside [0, 1] included — and the
            // implicit keyframes only where no keyframe reaches 0% / 100%.
            let mut frames: Vec<Frame> = p
                .frames
                .iter()
                .filter_map(|f| {
                    let offset = match f.range {
                        None => f.offset,
                        Some(r) => place?(r, f.offset)?,
                    };
                    Some(Frame {
                        offset,
                        ..f.clone()
                    })
                })
                .collect();
            frames.sort_by(|a, b| a.offset.total_cmp(&b.offset));
            add_implicit(&mut frames, &self.easing, self.composite);
            sample(p.longhand, &frames, progress, scheme, underlying, out);
        }
    }
}

/// §3: "If a 0% or from keyframe is not specified, the user agent
/// constructs one using the computed values of the properties being
/// animated" — and a 100% one likewise; with keyframes on timeline
/// ranges, only where none is at or before 0% (at or after 100%).
fn add_implicit(frames: &mut Vec<Frame>, easing: &TimingFunction, composite: AnimationComposition) {
    let implicit = |offset| Frame {
        offset,
        range: None,
        value: None,
        easing: easing.clone(),
        composite,
    };
    if frames.first().is_none_or(|f| f.offset > 0.0) {
        frames.insert(0, implicit(0.0));
    }
    if frames.last().is_none_or(|f| f.offset < 1.0) {
        frames.push(implicit(1.0));
    }
}

/// One longhand's value at `progress` from its keyframes, by offset (Web
/// Animations 1 §5.3.3): the interval from the last keyframe at or before
/// the progress (and before the last one) to the next, eased by its
/// start keyframe's easing.
fn sample(
    longhand: Longhand,
    frames: &[Frame],
    progress: f64,
    scheme: ColorScheme,
    underlying: &ComputedStyle,
    out: &mut ComputedStyle,
) {
    let last = frames.last().map_or(1.0, |f| f.offset);
    let start = frames
        .iter()
        .rposition(|f| f.offset <= progress && f.offset < last)
        .unwrap_or(0)
        .min(frames.len().saturating_sub(2));
    let (Some(a), Some(b)) = (frames.get(start), frames.get(start + 1)) else {
        return;
    };
    let span = b.offset - a.offset;
    let local = if span > 0.0 {
        (progress - a.offset) / span
    } else {
        1.0
    };
    let eased = f64::from(a.easing.ease(local as f32));
    let from = endpoint(longhand, a, underlying, scheme);
    let to = endpoint(longhand, b, underlying, scheme);
    longhand.interpolate(&from, &to, eased, scheme, out);
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
