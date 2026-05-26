//! Transition engine — observes property changes between cascade
//! passes, registers an `ActiveAnimation`, and writes interpolated
//! values into `TuiExt.presentation` each tick.
//!
//! Paint / layout / hit-test read the live values via the
//! `effective_*` helpers below — they fall back to `ComputedStyle`
//! when no animation is in flight.

use std::time::{Duration, Instant};

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Length, Size, ZIndex};
use crate::style::transition::{AnimatableProperty, TimingFunction, TransitionProperty};
use crate::style::{Color, ComputedStyle};

// ── Property identity (engine-internal) ───────────────────────────

/// Internal property identity tracked by an animation. Maps 1:1 to
/// `AnimatableProperty` plus the cascade-internal modifiers stored
/// on `ComputedStyle.modifiers`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimatedProp {
    Fg,
    Bg,
    BorderFg,
    Width,
    Height,
    Padding,
    Gap,
    Top,
    Right,
    Bottom,
    Left,
    ZIndex,
}

impl AnimatedProp {
    pub fn css_name(self) -> &'static str {
        match self {
            AnimatedProp::Fg => "color",
            AnimatedProp::Bg => "background-color",
            AnimatedProp::BorderFg => "border-color",
            AnimatedProp::Width => "width",
            AnimatedProp::Height => "height",
            AnimatedProp::Padding => "padding",
            AnimatedProp::Gap => "gap",
            AnimatedProp::Top => "top",
            AnimatedProp::Right => "right",
            AnimatedProp::Bottom => "bottom",
            AnimatedProp::Left => "left",
            AnimatedProp::ZIndex => "z-index",
        }
    }

    fn from_animatable(ap: AnimatableProperty) -> Self {
        match ap {
            AnimatableProperty::Color => AnimatedProp::Fg,
            AnimatableProperty::BackgroundColor => AnimatedProp::Bg,
            AnimatableProperty::BorderColor => AnimatedProp::BorderFg,
            AnimatableProperty::Width => AnimatedProp::Width,
            AnimatableProperty::Height => AnimatedProp::Height,
            AnimatableProperty::Padding => AnimatedProp::Padding,
            AnimatableProperty::Gap => AnimatedProp::Gap,
            AnimatableProperty::Top => AnimatedProp::Top,
            AnimatableProperty::Right => AnimatedProp::Right,
            AnimatableProperty::Bottom => AnimatedProp::Bottom,
            AnimatableProperty::Left => AnimatedProp::Left,
            AnimatableProperty::ZIndex => AnimatedProp::ZIndex,
        }
    }
}

/// Boxed value of any animatable property. The variant matches
/// the property type 1:1; mismatched lerps just snap at midpoint.
#[derive(Debug, Clone, PartialEq)]
pub enum AnimatedValue {
    Color(Color),
    Size(Size),
    Length(Length),
    U16(u16),
    Padding(crate::layout::Padding),
    ZIndex(ZIndex),
}

// ── Active animation ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ActiveAnimation {
    pub node: NodeId,
    pub property: AnimatedProp,
    pub from: AnimatedValue,
    pub to: AnimatedValue,
    /// Set when the animation was registered. The visual start is
    /// `started_at + delay`.
    pub started_at: Instant,
    pub delay: Duration,
    pub duration: Duration,
    pub timing: TimingFunction,
    /// Tracks whether `transitionstart` already fired (after delay
    /// elapses). The engine's tick advance dispatches
    /// `transitionstart` on the first tick where now ≥ started_at
    /// + delay, then sets this true.
    pub started_dispatched: bool,
}

impl ActiveAnimation {
    /// Linear progress in [0, 1]. Reaches 0 before delay elapses.
    fn progress(&self, now: Instant) -> f32 {
        let elapsed = now.saturating_duration_since(self.started_at);
        if elapsed < self.delay {
            return 0.0;
        }
        let post_delay = elapsed - self.delay;
        if self.duration.is_zero() {
            return 1.0;
        }
        let t = post_delay.as_secs_f32() / self.duration.as_secs_f32();
        t.clamp(0.0, 1.0)
    }

    /// True once `now >= started_at + delay + duration`.
    fn is_done(&self, now: Instant) -> bool {
        let total = self.delay + self.duration;
        now.saturating_duration_since(self.started_at) >= total
    }

    /// Eased current value. Clamped to `to` once t reaches 1.0.
    fn current(&self, now: Instant) -> AnimatedValue {
        let t = self.timing.ease(self.progress(now));
        interpolate(&self.from, &self.to, t)
    }
}

// ── Registry ──────────────────────────────────────────────────────

/// All in-flight transitions for the App. Live in
/// `App.animations`; the cascade hook adds entries, the tick loop
/// advances + retires them.
#[derive(Debug, Default)]
pub struct AnimationRegistry {
    active: Vec<ActiveAnimation>,
    /// Events the engine wants the runtime to dispatch on the
    /// next event-pump cycle. The App drains this via
    /// `take_pending_events` after every tick.
    pending_events: Vec<PendingEvent>,
}

#[derive(Debug, Clone)]
pub struct PendingEvent {
    pub node: NodeId,
    pub kind: TransitionEventKind,
    pub property: AnimatedProp,
    pub elapsed_seconds: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionEventKind {
    Start,
    End,
    Cancel,
}

impl AnimationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    pub fn len(&self) -> usize {
        self.active.len()
    }

    pub fn take_pending_events(&mut self) -> Vec<PendingEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Register a new animation, replacing any existing one for
    /// the same (node, property). The replaced animation fires
    /// `transitioncancel`. The new animation's `from` is the
    /// previous's *current interpolated* value when an animation
    /// is replaced mid-flight (CSS-faithful).
    fn register(&mut self, mut anim: ActiveAnimation, now: Instant) {
        if let Some(pos) = self
            .active
            .iter()
            .position(|a| a.node == anim.node && a.property == anim.property)
        {
            let old = self.active.swap_remove(pos);
            self.pending_events.push(PendingEvent {
                node: old.node,
                kind: TransitionEventKind::Cancel,
                property: old.property,
                elapsed_seconds: now.saturating_duration_since(old.started_at).as_secs_f32(),
            });
            // New animation's `from` becomes the interpolated
            // value of the old one (avoids a discontinuity).
            anim.from = old.current(now);
        }
        self.active.push(anim);
    }

    /// Cancel every in-flight animation on `node` (e.g. node
    /// removed from DOM, display:none cascaded onto it).
    pub fn cancel_for_node(&mut self, node: NodeId, now: Instant) {
        let mut i = 0;
        while i < self.active.len() {
            if self.active[i].node == node {
                let a = self.active.swap_remove(i);
                self.pending_events.push(PendingEvent {
                    node: a.node,
                    kind: TransitionEventKind::Cancel,
                    property: a.property,
                    elapsed_seconds: now.saturating_duration_since(a.started_at).as_secs_f32(),
                });
            } else {
                i += 1;
            }
        }
    }

    /// Advance every active animation, write interpolated values
    /// into `TuiExt.presentation`, fire transitionstart /
    /// transitionend events as appropriate. Removes finished
    /// entries. Caller pumps the resulting events afterwards.
    pub fn advance(&mut self, dom: &mut Dom<TuiExt>, now: Instant) {
        let mut i = 0;
        while i < self.active.len() {
            let anim = &mut self.active[i];

            // Fire transitionstart once delay elapses.
            let elapsed = now.saturating_duration_since(anim.started_at);
            if !anim.started_dispatched && elapsed >= anim.delay {
                anim.started_dispatched = true;
                self.pending_events.push(PendingEvent {
                    node: anim.node,
                    kind: TransitionEventKind::Start,
                    property: anim.property,
                    elapsed_seconds: 0.0,
                });
            }

            // Compute current value + write to presentation.
            let value = anim.current(now);
            write_presentation(dom, anim.node, anim.property, value);

            // Retire on completion.
            if anim.is_done(now) {
                let finished = self.active.swap_remove(i);
                // Clear the presentation slot — the committed
                // value in ComputedStyle is the truth from now on.
                clear_presentation(dom, finished.node, finished.property);
                self.pending_events.push(PendingEvent {
                    node: finished.node,
                    kind: TransitionEventKind::End,
                    property: finished.property,
                    elapsed_seconds: finished.duration.as_secs_f32(),
                });
            } else {
                i += 1;
            }
        }
    }
}

#[cfg(test)]
impl AnimationRegistry {
    /// Inject a `PendingEvent` directly into the queue, bypassing
    /// `advance`. Lets tests drive `dispatch_animation_events`
    /// without setting up a real-time-driven transition.
    pub(crate) fn queue_event_for_test(&mut self, e: PendingEvent) {
        self.pending_events.push(e);
    }
}

// ── Cascade hook ──────────────────────────────────────────────────

/// Run after `Dom::cascade(&sheet)`. For each element, diff
/// `computed_prev` against `computed`; for each animatable
/// property change covered by an active transition rule, register
/// (or replace) an animation. Then snapshot `computed` →
/// `computed_prev` for next pass.
pub fn diff_and_register(dom: &mut Dom<TuiExt>, registry: &mut AnimationRegistry, now: Instant) {
    let ids = collect_element_ids(dom, dom.root());
    for id in ids {
        let (prev, curr) = match snapshot(dom, id) {
            Some(pair) => pair,
            None => continue,
        };
        if let Some(prev_style) = prev.as_ref() {
            let curr_style = curr.as_ref().unwrap();
            for prop in animatable_props_for(curr_style, prev_style) {
                let Some(rule) = lookup_rule(curr_style, prop) else {
                    continue;
                };
                // Zero-duration rule means "no transition";
                // commit the new value immediately.
                if rule.duration_ms == 0 && rule.delay_ms == 0 {
                    continue;
                }
                let from = read_value(prev_style, prop);
                let to = read_value(curr_style, prop);
                if from == to {
                    continue;
                }
                let anim = ActiveAnimation {
                    node: id,
                    property: prop,
                    from,
                    to,
                    started_at: now,
                    delay: Duration::from_millis(rule.delay_ms as u64),
                    duration: Duration::from_millis(rule.duration_ms as u64),
                    timing: rule.timing,
                    started_dispatched: false,
                };
                registry.register(anim, now);
            }
        }
        // Snapshot for next pass.
        if let Some(curr_clone) = curr {
            let mut node_mut = dom.node_mut(id);
            if let Some(ext) = node_mut.ext_mut() {
                ext.computed_prev = Some(curr_clone);
            }
        }
    }
}

fn snapshot(
    dom: &Dom<TuiExt>,
    id: NodeId,
) -> Option<(Option<ComputedStyle>, Option<ComputedStyle>)> {
    let ext = dom.node(id).ext()?;
    Some((ext.computed_prev.clone(), ext.computed.clone()))
}

fn collect_element_ids(dom: &Dom<TuiExt>, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk(dom, id, &mut out);
    out
}

fn walk(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    if dom.node(id).node_type() == NodeType::Element {
        out.push(id);
    }
    for child in dom.node(id).child_nodes() {
        walk(dom, child.id(), out);
    }
}

/// Iterate the animatable properties whose values differ between
/// `prev` and `curr`. Skip properties the cascade hasn't moved.
fn animatable_props_for(curr: &ComputedStyle, prev: &ComputedStyle) -> Vec<AnimatedProp> {
    let mut out = Vec::new();
    if curr.fg != prev.fg {
        out.push(AnimatedProp::Fg);
    }
    if curr.bg != prev.bg {
        out.push(AnimatedProp::Bg);
    }
    if curr.border_fg != prev.border_fg {
        out.push(AnimatedProp::BorderFg);
    }
    if curr.width != prev.width {
        out.push(AnimatedProp::Width);
    }
    if curr.height != prev.height {
        out.push(AnimatedProp::Height);
    }
    if curr.padding != prev.padding {
        out.push(AnimatedProp::Padding);
    }
    if curr.gap != prev.gap {
        out.push(AnimatedProp::Gap);
    }
    if curr.top != prev.top {
        out.push(AnimatedProp::Top);
    }
    if curr.right != prev.right {
        out.push(AnimatedProp::Right);
    }
    if curr.bottom != prev.bottom {
        out.push(AnimatedProp::Bottom);
    }
    if curr.left != prev.left {
        out.push(AnimatedProp::Left);
    }
    if curr.z_index != prev.z_index {
        out.push(AnimatedProp::ZIndex);
    }
    out
}

/// Look up the transition rule for `prop` inside `style`'s four
/// transition longhand lists, applying CSS L1's cycling rule when
/// list lengths differ. Returns `None` when no rule applies (no
/// transition-property entry covers this property, or
/// transition-property is `None`).
fn lookup_rule(style: &ComputedStyle, prop: AnimatedProp) -> Option<MatchedRule> {
    let props = &style.transition_property;
    if props.is_empty() {
        return None;
    }
    // Find the index of the entry covering `prop`.
    let idx = props.iter().position(|p| match p {
        TransitionProperty::All => true,
        TransitionProperty::None => false,
        TransitionProperty::Named(ap) => AnimatedProp::from_animatable(*ap) == prop,
    })?;
    // None entries disable transitions for the matched property.
    if matches!(props[idx], TransitionProperty::None) {
        return None;
    }
    let durations = &style.transition_duration;
    let timings = &style.transition_timing_function;
    let delays = &style.transition_delay;
    let duration_ms = cycle(durations, idx).copied().unwrap_or(0);
    let timing = cycle(timings, idx).copied().unwrap_or(TimingFunction::Ease);
    let delay_ms = cycle(delays, idx).copied().unwrap_or(0);
    Some(MatchedRule {
        duration_ms,
        timing,
        delay_ms,
    })
}

#[derive(Debug, Clone, Copy)]
struct MatchedRule {
    duration_ms: u32,
    timing: TimingFunction,
    delay_ms: u32,
}

fn cycle<T>(list: &[T], idx: usize) -> Option<&T> {
    if list.is_empty() {
        None
    } else {
        Some(&list[idx % list.len()])
    }
}

fn read_value(style: &ComputedStyle, prop: AnimatedProp) -> AnimatedValue {
    match prop {
        AnimatedProp::Fg => AnimatedValue::Color(style.fg),
        AnimatedProp::Bg => AnimatedValue::Color(style.bg),
        AnimatedProp::BorderFg => AnimatedValue::Color(style.border_fg),
        AnimatedProp::Width => AnimatedValue::Size(style.width.clone()),
        AnimatedProp::Height => AnimatedValue::Size(style.height.clone()),
        AnimatedProp::Padding => AnimatedValue::Padding(style.padding.clone()),
        AnimatedProp::Gap => AnimatedValue::U16(style.gap),
        AnimatedProp::Top => AnimatedValue::Length(style.top.clone()),
        AnimatedProp::Right => AnimatedValue::Length(style.right.clone()),
        AnimatedProp::Bottom => AnimatedValue::Length(style.bottom.clone()),
        AnimatedProp::Left => AnimatedValue::Length(style.left.clone()),
        AnimatedProp::ZIndex => AnimatedValue::ZIndex(style.z_index),
    }
}

fn write_presentation(
    dom: &mut Dom<TuiExt>,
    node: NodeId,
    prop: AnimatedProp,
    value: AnimatedValue,
) {
    let mut node_mut = dom.node_mut(node);
    let Some(ext) = node_mut.ext_mut() else {
        return;
    };
    match (prop, value) {
        (AnimatedProp::Fg, AnimatedValue::Color(c)) => ext.presentation.fg = Some(c),
        (AnimatedProp::Bg, AnimatedValue::Color(c)) => ext.presentation.bg = Some(c),
        (AnimatedProp::BorderFg, AnimatedValue::Color(c)) => ext.presentation.border_fg = Some(c),
        (AnimatedProp::Width, AnimatedValue::Size(s)) => ext.presentation.width = Some(s),
        (AnimatedProp::Height, AnimatedValue::Size(s)) => ext.presentation.height = Some(s),
        (AnimatedProp::Padding, AnimatedValue::Padding(p)) => ext.presentation.padding = Some(p),
        (AnimatedProp::Gap, AnimatedValue::U16(g)) => ext.presentation.gap = Some(g),
        (AnimatedProp::Top, AnimatedValue::Length(l)) => ext.presentation.top = Some(l),
        (AnimatedProp::Right, AnimatedValue::Length(l)) => ext.presentation.right = Some(l),
        (AnimatedProp::Bottom, AnimatedValue::Length(l)) => ext.presentation.bottom = Some(l),
        (AnimatedProp::Left, AnimatedValue::Length(l)) => ext.presentation.left = Some(l),
        (AnimatedProp::ZIndex, AnimatedValue::ZIndex(z)) => ext.presentation.z_index = Some(z),
        _ => {}
    }
}

fn clear_presentation(dom: &mut Dom<TuiExt>, node: NodeId, prop: AnimatedProp) {
    let mut node_mut = dom.node_mut(node);
    let Some(ext) = node_mut.ext_mut() else {
        return;
    };
    match prop {
        AnimatedProp::Fg => ext.presentation.fg = None,
        AnimatedProp::Bg => ext.presentation.bg = None,
        AnimatedProp::BorderFg => ext.presentation.border_fg = None,
        AnimatedProp::Width => ext.presentation.width = None,
        AnimatedProp::Height => ext.presentation.height = None,
        AnimatedProp::Padding => ext.presentation.padding = None,
        AnimatedProp::Gap => ext.presentation.gap = None,
        AnimatedProp::Top => ext.presentation.top = None,
        AnimatedProp::Right => ext.presentation.right = None,
        AnimatedProp::Bottom => ext.presentation.bottom = None,
        AnimatedProp::Left => ext.presentation.left = None,
        AnimatedProp::ZIndex => ext.presentation.z_index = None,
    }
}

// ── Interpolation primitives ──────────────────────────────────────

fn interpolate(from: &AnimatedValue, to: &AnimatedValue, t: f32) -> AnimatedValue {
    match (from, to) {
        (AnimatedValue::Color(a), AnimatedValue::Color(b)) => {
            AnimatedValue::Color(lerp_color(*a, *b, t))
        }
        (AnimatedValue::Size(a), AnimatedValue::Size(b)) => AnimatedValue::Size(lerp_size(a, b, t)),
        (AnimatedValue::Length(a), AnimatedValue::Length(b)) => {
            AnimatedValue::Length(lerp_length(a, b, t))
        }
        (AnimatedValue::U16(a), AnimatedValue::U16(b)) => AnimatedValue::U16(lerp_u16(*a, *b, t)),
        (AnimatedValue::Padding(a), AnimatedValue::Padding(b)) => {
            AnimatedValue::Padding(crate::layout::Padding {
                top: lerp_padding_value(&a.top, &b.top, t),
                right: lerp_padding_value(&a.right, &b.right, t),
                bottom: lerp_padding_value(&a.bottom, &b.bottom, t),
                left: lerp_padding_value(&a.left, &b.left, t),
            })
        }
        (AnimatedValue::ZIndex(a), AnimatedValue::ZIndex(b)) => {
            AnimatedValue::ZIndex(lerp_zindex(*a, *b, t))
        }
        // Type mismatch — snap at midpoint.
        _ => {
            if t < 0.5 {
                from.clone()
            } else {
                to.clone()
            }
        }
    }
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let (ar, ag, ab) = color_to_rgb_approx(a);
    let (br, bg, bb) = color_to_rgb_approx(b);
    Color::Rgb(lerp_u8(ar, br, t), lerp_u8(ag, bg, t), lerp_u8(ab, bb, t))
}

#[inline]
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let v = a as f32 + (b as f32 - a as f32) * t;
    v.round().clamp(0.0, 255.0) as u8
}

#[inline]
fn lerp_u16(a: u16, b: u16, t: f32) -> u16 {
    let v = a as f32 + (b as f32 - a as f32) * t;
    v.round().clamp(0.0, u16::MAX as f32) as u16
}

/// Interpolate two `PaddingValue` sides. `Cells ↔ Cells` lerps the
/// cell count linearly. Anything involving `Calc` (whose value
/// depends on a containing-block width unknown at animation time)
/// snaps at midpoint — matches the policy already used for
/// `Size::Flex|Auto|Calc` lerps below.
fn lerp_padding_value(
    a: &crate::layout::PaddingValue,
    b: &crate::layout::PaddingValue,
    t: f32,
) -> crate::layout::PaddingValue {
    use crate::layout::PaddingValue;
    match (a, b) {
        (PaddingValue::Cells(x), PaddingValue::Cells(y)) => {
            PaddingValue::Cells(lerp_u16(*x, *y, t))
        }
        _ => {
            if t < 0.5 {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

#[inline]
fn lerp_i16(a: i16, b: i16, t: f32) -> i16 {
    let v = a as f32 + (b as f32 - a as f32) * t;
    v.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

fn lerp_size(a: &Size, b: &Size, t: f32) -> Size {
    match (a, b) {
        (Size::Fixed(x), Size::Fixed(y)) => Size::Fixed(lerp_u16(*x, *y, t)),
        // Fixed ↔ Flex / Auto / Calc don't lerp meaningfully; snap.
        // Calc-bearing transitions snap because we don't have layout
        // context at interpolation time to resolve the percent
        // basis. Documented divergence; pay down by snapshotting
        // computed-pixel values at transition start.
        _ => {
            if t < 0.5 {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

fn lerp_length(a: &Length, b: &Length, t: f32) -> Length {
    match (a, b) {
        (Length::Cells(x), Length::Cells(y)) => Length::Cells(lerp_i16(*x, *y, t)),
        _ => {
            if t < 0.5 {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

fn lerp_zindex(a: ZIndex, b: ZIndex, t: f32) -> ZIndex {
    match (a, b) {
        (ZIndex::Value(x), ZIndex::Value(y)) => ZIndex::Value(lerp_i16(x, y, t)),
        _ => {
            if t < 0.5 {
                a
            } else {
                b
            }
        }
    }
}

/// Map a `Color` to an approximate sRGB triple for interpolation.
/// Named colors use the canonical ANSI-16 palette values.
/// `Reset` and `Indexed(_)` fall back to mid-gray since we don't
/// know the terminal's actual palette — interpolation through
/// these is best-effort and apps that want exact lerps should
/// use `Color::Rgb` endpoints.
/// Resolve a `Color` to its (r, g, b) for interpolation. Now
/// trivial in the truecolor-only world: `Rgb` returns its
/// channels, `Indexed` falls back to a neutral midgray placeholder
/// (a future commit could read the xterm-256 RGB table), and
/// `Reset` returns a neutral light-gray since the actual terminal
/// default is unknowable from inside the engine.
fn color_to_rgb_approx(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Reset => (192, 192, 192),
        Color::Indexed(_) => (128, 128, 128),
        Color::Rgb(r, g, b) => (r, g, b),
    }
}

// ── Effective-value helpers (read by paint/layout/hit-test) ───────

pub fn effective_fg(ext: &TuiExt) -> Color {
    ext.presentation
        .fg
        .or(ext.computed.as_ref().map(|c| c.fg))
        .unwrap_or(Color::Reset)
}

pub fn effective_bg(ext: &TuiExt) -> Color {
    ext.presentation
        .bg
        .or(ext.computed.as_ref().map(|c| c.bg))
        .unwrap_or(Color::Reset)
}

pub fn effective_border_fg(ext: &TuiExt) -> Color {
    ext.presentation
        .border_fg
        .or(ext.computed.as_ref().map(|c| c.border_fg))
        .unwrap_or(Color::Reset)
}

pub fn effective_padding(ext: &TuiExt) -> crate::layout::Padding {
    ext.presentation
        .padding
        .clone()
        .or_else(|| ext.computed.as_ref().map(|c| c.padding.clone()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Stylesheet;
    use crate::style::transition::AnimatableProperty;
    use crate::{CascadeExt, TuiDom, TuiStyle};

    fn epoch() -> Instant {
        Instant::now()
    }

    // ── §15.10 — change without transition rule applies instantly

    #[test]
    fn property_change_without_transition_rule_applies_instantly() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();

        let s1 =
            Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
        dom.cascade(&s1);
        let mut reg = AnimationRegistry::new();
        let now = epoch();
        diff_and_register(&mut dom, &mut reg, now);

        // Switch to blue.
        let s2 =
            Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(0, 0, 255)));
        dom.cascade(&s2);
        diff_and_register(&mut dom, &mut reg, now);

        // No transition rule → no animation registered.
        assert_eq!(reg.len(), 0);
        // Computed value committed immediately.
        assert_eq!(
            dom.node(div).ext().unwrap().computed.as_ref().unwrap().fg,
            Color::Rgb(0, 0, 255)
        );
    }

    // ── §15.11 — transition: color 100ms interpolates ────────────

    #[test]
    fn transition_color_interpolates_at_midpoint() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();

        // Round 1: red, with the transition rule attached.
        let s1 = Stylesheet::bare().rule_unchecked(
            "div",
            TuiStyle::new()
                .fg(Color::Rgb(255, 0, 0))
                .transition_property(vec![TransitionProperty::Named(AnimatableProperty::Color)])
                .transition_duration(vec![100])
                .transition_timing_function(vec![TimingFunction::Linear])
                .transition_delay(vec![0]),
        );
        dom.cascade(&s1);
        let mut reg = AnimationRegistry::new();
        let start = epoch();
        diff_and_register(&mut dom, &mut reg, start);

        // Round 2: blue (with the same transition rule).
        let s2 = Stylesheet::bare().rule_unchecked(
            "div",
            TuiStyle::new()
                .fg(Color::Rgb(0, 0, 255))
                .transition_property(vec![TransitionProperty::Named(AnimatableProperty::Color)])
                .transition_duration(vec![100])
                .transition_timing_function(vec![TimingFunction::Linear])
                .transition_delay(vec![0]),
        );
        dom.cascade(&s2);
        diff_and_register(&mut dom, &mut reg, start);

        // One animation registered for fg.
        assert_eq!(reg.len(), 1);

        // Advance 50ms — linear midpoint of red (255,0,0) →
        // blue (0,0,255) = (128, 0, 128) (within ±2 due to rounding).
        let mid = start + Duration::from_millis(50);
        reg.advance(&mut dom, mid);
        let pres_fg = dom.node(div).ext().unwrap().presentation.fg.unwrap();
        match pres_fg {
            Color::Rgb(r, g, b) => {
                assert!((r as i16 - 128).abs() <= 2, "r = {r}");
                assert_eq!(g, 0);
                assert!((b as i16 - 128).abs() <= 2, "b = {b}");
            }
            other => panic!("expected Rgb, got {other:?}"),
        }

        // Advance to end — animation retires, presentation cleared.
        let end = start + Duration::from_millis(120);
        reg.advance(&mut dom, end);
        assert!(reg.is_empty());
        assert!(dom.node(div).ext().unwrap().presentation.fg.is_none());
    }

    #[test]
    fn transitionend_event_queued_at_completion() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();

        let make_sheet = |c: Color| {
            Stylesheet::bare().rule_unchecked(
                "div",
                TuiStyle::new()
                    .fg(c)
                    .transition_property(vec![TransitionProperty::Named(AnimatableProperty::Color)])
                    .transition_duration(vec![100])
                    .transition_timing_function(vec![TimingFunction::Linear])
                    .transition_delay(vec![0]),
            )
        };
        dom.cascade(&make_sheet(Color::Rgb(255, 0, 0)));
        let mut reg = AnimationRegistry::new();
        let start = epoch();
        diff_and_register(&mut dom, &mut reg, start);
        dom.cascade(&make_sheet(Color::Rgb(0, 0, 255)));
        diff_and_register(&mut dom, &mut reg, start);

        // Drain initial events (transitionstart fires on first
        // tick where now ≥ started_at + delay).
        reg.advance(&mut dom, start + Duration::from_millis(0));
        let initial = reg.take_pending_events();
        assert!(initial.iter().any(|e| e.kind == TransitionEventKind::Start));

        // Advance to completion.
        reg.advance(&mut dom, start + Duration::from_millis(150));
        let ending = reg.take_pending_events();
        assert!(
            ending
                .iter()
                .any(|e| e.kind == TransitionEventKind::End && e.property == AnimatedProp::Fg),
            "expected transitionend; got {:?}",
            ending
        );
    }

    #[test]
    fn re_setting_property_mid_flight_fires_cancel_and_restarts_from_current() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();

        let make_sheet = |c: Color| {
            Stylesheet::bare().rule_unchecked(
                "div",
                TuiStyle::new()
                    .fg(c)
                    .transition_property(vec![TransitionProperty::Named(AnimatableProperty::Color)])
                    .transition_duration(vec![100])
                    .transition_timing_function(vec![TimingFunction::Linear])
                    .transition_delay(vec![0]),
            )
        };
        dom.cascade(&make_sheet(Color::Rgb(255, 0, 0)));
        let mut reg = AnimationRegistry::new();
        let t0 = epoch();
        diff_and_register(&mut dom, &mut reg, t0);
        dom.cascade(&make_sheet(Color::Rgb(0, 0, 255))); // → blue
        diff_and_register(&mut dom, &mut reg, t0);

        // Halfway through the red→blue transit, retarget to green.
        let mid = t0 + Duration::from_millis(50);
        reg.advance(&mut dom, mid);
        let _ = reg.take_pending_events();
        dom.cascade(&make_sheet(Color::Rgb(0, 255, 0)));
        diff_and_register(&mut dom, &mut reg, mid);

        let after_retarget = reg.take_pending_events();
        // The retarget should have fired transitioncancel for fg.
        assert!(
            after_retarget
                .iter()
                .any(|e| e.kind == TransitionEventKind::Cancel && e.property == AnimatedProp::Fg)
        );
        // And there's exactly one animation now (the new red-ish→green).
        assert_eq!(reg.len(), 1);
    }
}
