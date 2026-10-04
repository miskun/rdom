//! Transitions of registered custom properties (CSS Properties and
//! Values API 1 §6.2, CSS Transitions 1 §3).
//!
//! A custom property registered with a syntax that interpolates — a
//! single `<color>`, `<number>`, `<integer>`, `<length>` or
//! `<percentage>` — transitions like any animatable property when
//! `transition-property` names it (or is `all`). Its animated value is
//! not painted directly: the engine writes it into the element's
//! `PresentationStyle::custom_properties`, and the element's subtree is re-cascaded
//! ([`AnimationRegistry::take_restyle`]) so that every `var()` consumer
//! — on the element or, for an inheriting property, its descendants —
//! follows it. The cascaded value (`ComputedStyle::vars`) is already the
//! end value, which is what the next change is compared against.

use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use rdom_core::{Dom, NodeId};
use rdom_style::{PropertyRegistration, SyntaxComponent};

use super::interpolate::lerp_color;
use super::{AnimationRegistry, TransitionEventKind};
use crate::ext::{StyleSlot, TuiExt};
use crate::style::transition::{TimingFunction, TransitionProperty};
use crate::style::{Color, ComputedStyle};

/// How a registered property's values interpolate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Kind {
    Color,
    /// A number; `whole` rounds it (`<integer>`, `<length>` in cells).
    Number {
        whole: bool,
        percent: bool,
    },
}

/// A value being interpolated.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Value {
    Color(Color),
    Number(f64),
}

impl Kind {
    fn of(registration: &PropertyRegistration) -> Option<Kind> {
        Some(match registration.syntax.interpolation()? {
            SyntaxComponent::Color => Kind::Color,
            SyntaxComponent::Integer | SyntaxComponent::Length => Kind::Number {
                whole: true,
                percent: false,
            },
            SyntaxComponent::Percentage => Kind::Number {
                whole: false,
                percent: true,
            },
            _ => Kind::Number {
                whole: false,
                percent: false,
            },
        })
    }

    fn parse(self, text: &str) -> Option<Value> {
        match self {
            Kind::Color => crate::style::parse_color(text).map(Value::Color),
            Kind::Number { percent, .. } => {
                let text = text.replace(' ', "");
                let text = if percent {
                    text.strip_suffix('%')?.to_string()
                } else {
                    text
                };
                text.parse().ok().map(Value::Number)
            }
        }
    }

    fn format(self, value: Value) -> String {
        match (self, value) {
            (_, Value::Color(Color::Rgb(r, g, b))) => format!("rgb({r}, {g}, {b})"),
            (_, Value::Color(Color::Indexed(i))) => i.to_string(),
            (_, Value::Color(Color::Reset)) => "reset".to_string(),
            (Kind::Number { whole: true, .. }, Value::Number(n)) => format!("{}", n.round() as i64),
            (Kind::Number { percent, .. }, Value::Number(n)) => {
                let n = (n * 1000.0).round() / 1000.0;
                if percent {
                    format!("{n}%")
                } else {
                    format!("{n}")
                }
            }
            (Kind::Color, Value::Number(n)) => format!("{n}"),
        }
    }
}

fn lerp(from: Value, to: Value, t: f32) -> Value {
    match (from, to) {
        (Value::Color(a), Value::Color(b)) => Value::Color(lerp_color(a, b, t)),
        (Value::Number(a), Value::Number(b)) => Value::Number(a + (b - a) * f64::from(t)),
        (a, b) => {
            if t < 0.5 {
                a
            } else {
                b
            }
        }
    }
}

/// A running transition of one registered custom property.
#[derive(Debug, Clone)]
pub(super) struct CustomAnimation {
    node: NodeId,
    /// The name without dashes.
    name: String,
    kind: Kind,
    from: Value,
    to: Value,
    started_at: Instant,
    delay: Duration,
    duration: Duration,
    timing: TimingFunction,
    started_dispatched: bool,
}

impl CustomAnimation {
    fn current(&self, now: Instant) -> Value {
        let elapsed = now.saturating_duration_since(self.started_at);
        let t = if elapsed < self.delay {
            0.0
        } else if self.duration.is_zero() {
            1.0
        } else {
            ((elapsed - self.delay).as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
        };
        lerp(self.from, self.to, self.timing.ease(t))
    }

    fn is_done(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.started_at) >= self.delay + self.duration
    }
}

/// A transition event of a custom property, for the App to dispatch.
#[derive(Debug, Clone)]
pub struct PendingCustomEvent {
    pub node: NodeId,
    pub kind: TransitionEventKind,
    /// `--name`.
    pub property: String,
    pub elapsed_seconds: f32,
}

impl AnimationRegistry {
    /// Follow the registered custom properties the cascade reads — the
    /// one registry an `App` builds per stylesheet change; the ones
    /// whose syntax interpolates can transition. The App calls this
    /// before [`diff_and_register`](super::diff_and_register).
    pub(crate) fn set_registered_properties(
        &mut self,
        registry: Rc<crate::style::cascade::PropertyRegistry>,
    ) {
        self.registered = registry;
    }

    /// The elements whose animated custom properties moved since the
    /// last call: their subtrees must be re-cascaded before layout.
    pub fn take_restyle(&mut self) -> Vec<NodeId> {
        let mut nodes = std::mem::take(&mut self.restyle);
        nodes.sort_unstable();
        nodes.dedup();
        nodes
    }

    /// The custom-property transition events queued since the last
    /// call.
    pub fn take_pending_custom_events(&mut self) -> Vec<PendingCustomEvent> {
        std::mem::take(&mut self.custom_events)
    }

    /// Register the transitions `id`'s registered custom properties
    /// start between `prev` and `curr` (the cascaded values).
    pub(super) fn diff_custom(
        &mut self,
        dom: &mut Dom<TuiExt>,
        id: NodeId,
        prev: &ComputedStyle,
        curr: &ComputedStyle,
        now: Instant,
    ) {
        if self.registered.is_empty() || Rc::ptr_eq(&prev.vars, &curr.vars) {
            return;
        }
        let changed: Vec<(String, Kind)> = self
            .registered
            .iter()
            .filter(|(name, _)| prev.vars.get(*name) != curr.vars.get(*name))
            .filter_map(|(name, reg)| Some((name.to_string(), Kind::of(reg)?)))
            .collect();
        for (name, kind) in changed {
            let Some(rule) = rule_for(curr, &name) else {
                continue;
            };
            if rule.0 == 0 && rule.2 == 0 {
                continue;
            }
            let running = self
                .custom
                .iter()
                .position(|a| a.node == id && a.name == name);
            let from = match running {
                Some(i) => Some(self.custom[i].current(now)),
                None => prev.vars.get(&name).and_then(|v| kind.parse(v)),
            };
            let to = curr.vars.get(&name).and_then(|v| kind.parse(v));
            if let Some(i) = running {
                let old = self.custom.swap_remove(i);
                self.custom_events.push(PendingCustomEvent {
                    node: id,
                    kind: TransitionEventKind::Cancel,
                    property: format!("--{name}"),
                    elapsed_seconds: now.saturating_duration_since(old.started_at).as_secs_f32(),
                });
                self.restyle.push(id);
            }
            let (Some(from), Some(to)) = (from, to) else {
                write(dom, id, &name, None);
                continue;
            };
            if from == to {
                write(dom, id, &name, None);
                continue;
            }
            self.custom.push(CustomAnimation {
                node: id,
                name,
                kind,
                from,
                to,
                started_at: now,
                delay: Duration::from_millis(u64::from(rule.2)),
                duration: Duration::from_millis(u64::from(rule.0)),
                timing: rule.1,
                started_dispatched: false,
            });
        }
    }

    /// Advance the custom-property transitions: write their values into
    /// `PresentationStyle::custom_properties`, retire the finished ones, queue their
    /// events, and note the elements to re-cascade.
    pub(super) fn advance_custom(&mut self, dom: &mut Dom<TuiExt>, now: Instant) {
        let mut i = 0;
        while i < self.custom.len() {
            let anim = &mut self.custom[i];
            let property = format!("--{}", anim.name);
            if !anim.started_dispatched
                && now.saturating_duration_since(anim.started_at) >= anim.delay
            {
                anim.started_dispatched = true;
                self.custom_events.push(PendingCustomEvent {
                    node: anim.node,
                    kind: TransitionEventKind::Start,
                    property: property.clone(),
                    elapsed_seconds: 0.0,
                });
            }
            let (node, name) = (anim.node, anim.name.clone());
            self.restyle.push(node);
            if anim.is_done(now) {
                let done = self.custom.swap_remove(i);
                write(dom, node, &name, None);
                self.custom_events.push(PendingCustomEvent {
                    node,
                    kind: TransitionEventKind::End,
                    property,
                    elapsed_seconds: done.duration.as_secs_f32(),
                });
            } else {
                let value = anim.kind.format(anim.current(now));
                write(dom, node, &name, Some(value));
                i += 1;
            }
        }
    }

    /// Cancel the custom-property transitions of `node`.
    pub(super) fn cancel_custom_for_node(&mut self, node: NodeId, now: Instant) {
        let mut i = 0;
        while i < self.custom.len() {
            if self.custom[i].node == node {
                let a = self.custom.swap_remove(i);
                self.custom_events.push(PendingCustomEvent {
                    node,
                    kind: TransitionEventKind::Cancel,
                    property: format!("--{}", a.name),
                    elapsed_seconds: now.saturating_duration_since(a.started_at).as_secs_f32(),
                });
            } else {
                i += 1;
            }
        }
    }
}

/// `(duration ms, timing, delay ms)` of the `transition-*` entry
/// covering `--name` (`all` or the name itself), cycling the lists.
fn rule_for(style: &ComputedStyle, name: &str) -> Option<(u32, TimingFunction, u32)> {
    let idx = style.transition_property.iter().position(|p| match p {
        TransitionProperty::All => true,
        TransitionProperty::Discrete(n) => n.strip_prefix("--") == Some(name),
        _ => false,
    })?;
    let pick = |len: usize| idx % len.max(1);
    let duration = style
        .transition_duration
        .get(pick(style.transition_duration.len()))
        .copied()
        .unwrap_or(0);
    let timing = style
        .transition_timing_function
        .get(pick(style.transition_timing_function.len()))
        .copied()
        .unwrap_or(TimingFunction::Ease);
    let delay = style
        .transition_delay
        .get(pick(style.transition_delay.len()))
        .copied()
        .unwrap_or(0);
    Some((duration, timing, delay))
}

/// Set (or, with `None`, clear) `node`'s animated value of `name`.
fn write(dom: &mut Dom<TuiExt>, node: NodeId, name: &str, value: Option<String>) {
    if !dom.contains(node) {
        return;
    }
    let mut node_mut = dom.node_mut(node);
    let Some(ext) = node_mut.ext_mut() else {
        return;
    };
    if value.is_none() && ext.presentation_for(StyleSlot::Host).is_none() {
        return;
    }
    let presentation = ext.presentation_for_mut(StyleSlot::Host);
    let map = presentation
        .custom_properties
        .get_or_insert_with(HashMap::new);
    match value {
        Some(v) => {
            map.insert(name.to_string(), v);
        }
        None => {
            map.remove(name);
        }
    }
    if map.is_empty() {
        presentation.custom_properties = None;
    }
    ext.release_empty_presentation(StyleSlot::Host);
}

#[cfg(test)]
impl AnimationRegistry {
    /// Queue a custom-property event directly (App event tests).
    pub(crate) fn queue_custom_event_for_test(&mut self, e: PendingCustomEvent) {
        self.custom_events.push(e);
    }
}
