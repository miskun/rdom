//! Reversing a transition (CSS Transitions 1 §3, "the reversing-adjusted
//! start value" and "the reversing shortening factor"): a running
//! transition interrupted by a change back to the value it started from
//! is replaced by one that runs back over the share of the duration it
//! had covered — a hover tooltip un-hovered half-way fades out in half
//! the time — and a chain of reversals compounds the factor.
//!
//! The registry keeps, for each transition a reversal started, its
//! reversing-adjusted start value and factor; any other transition's are
//! its start value and 1.

use std::rc::Rc;
use std::time::Instant;

use super::{ActiveAnimation, AnimationRegistry};
use crate::style::ComputedStyle;

/// A reversal-started transition's record, keyed by the transition.
#[derive(Debug, Clone)]
pub(super) struct Reversing {
    node: rdom_core::NodeId,
    slot: crate::ext::StyleSlot,
    property: super::Longhand,
    started_at: Instant,
    /// The reversing-adjusted start value: the end value of the
    /// transition it reversed.
    start: Rc<ComputedStyle>,
    /// The reversing shortening factor, in [0, 1].
    factor: f32,
}

impl Reversing {
    fn of(&self, a: &ActiveAnimation) -> bool {
        self.node == a.node
            && self.slot == a.slot
            && self.property == a.property
            && self.started_at == a.started_at
    }
}

impl AnimationRegistry {
    /// `anim` replaces the running `old` at `now`: when its end value is
    /// `old`'s reversing-adjusted start value, shorten it by the reversing
    /// shortening factor — its duration, and its delay if negative — and
    /// record what a later reversal of it reads.
    pub(super) fn reverse(
        &mut self,
        old: &ActiveAnimation,
        anim: &mut ActiveAnimation,
        now: Instant,
    ) {
        let (start, factor) = match self.reversing.iter().find(|r| r.of(old)) {
            Some(r) => (r.start.clone(), r.factor),
            None => (old.from.clone(), 1.0),
        };
        if anim.property.differs(&start, &anim.to) {
            return;
        }
        // The output of `old`'s timing function now (its before flag in
        // the delay).
        let output = old.eased(now);
        let factor = (output * factor + 1.0 - factor).abs().clamp(0.0, 1.0);
        anim.duration = anim.duration.mul_f32(factor);
        if !anim.skipped.is_zero() {
            // A negative delay is shortened too: it skips that share.
            let skipped = anim.skipped.mul_f32(factor);
            anim.started_at = (anim.started_at + anim.skipped)
                .checked_sub(skipped)
                .unwrap_or(anim.started_at);
            anim.skipped = skipped.min(anim.duration);
        }
        self.reversing.push(Reversing {
            node: anim.node,
            slot: anim.slot,
            property: anim.property,
            started_at: anim.started_at,
            start: old.to.clone(),
            factor,
        });
    }

    /// Drop the records of transitions no longer running.
    pub(super) fn prune_reversing(&mut self) {
        if self.reversing.is_empty() {
            return;
        }
        let active = &self.active;
        self.reversing.retain(|r| active.iter().any(|a| r.of(a)));
    }
}
