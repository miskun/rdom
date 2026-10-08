//! The initial value of a reversed counter instantiated without one
//! (CSS Lists 3 §4.2):
//!
//! > Let num be 0. Let lastNonZeroIncrementNegated be 0. For each
//! > element or pseudo-element el that increments or sets the same
//! > counter in the same scope: let incrementNegated be el's increment
//! > negated; if it is not 0, set lastNonZeroIncrementNegated to it; if
//! > el sets the counter, add the set value to num and stop; add
//! > incrementNegated to num. Add lastNonZeroIncrementNegated to num.
//!
//! "The same counter in the same scope" depends on the boxes after the
//! one that instantiates it, which a top-down cascade has not reached.
//! So the value is computed by replaying, in a scratch [`CounterState`]
//! that traces the new counter, the counter ops of the boxes in its
//! scope as they were last cascaded ([`initial_value`]) — exact on every
//! cascade but the first after a change in the scope. Every value the
//! walk used is checked once the walk is done, against the boxes as they
//! now are ([`CounterState::stale_reversed`]); the cascade re-runs from
//! the boxes whose value moved, which then read the fresh ops. Counter
//! ops never depend on counter values, so one re-run settles it.

use rdom_core::{Dom, NodeId};

use super::{CounterState, OpBox, Owner, StoredOps, takes_part};
use crate::ext::TuiExt;
use crate::style::ComputedStyle;

/// What one box did to the traced counter.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Touch {
    target: Option<u32>,
    touched: bool,
    increment: i64,
    set: Option<i32>,
}

impl Touch {
    /// A record of what one box does to instance `target` (none: no scan
    /// is tracing).
    pub(super) fn aimed_at(target: Option<u32>) -> Self {
        Touch {
            target,
            ..Self::default()
        }
    }

    /// The box incremented instance `id` by `by`.
    pub(super) fn increment(&mut self, id: u32, by: i32) {
        if self.target == Some(id) {
            self.touched = true;
            self.increment += i64::from(by);
        }
    }

    /// The box set instance `id` to `value`.
    pub(super) fn set(&mut self, id: u32, value: i32) {
        if self.target == Some(id) {
            self.touched = true;
            self.set = Some(value);
        }
    }
}

/// A scan's record: the traced instance and, per box that touched it in
/// tree order, its increment and set.
#[derive(Debug, Default, Clone)]
pub(super) struct Trace {
    target: u32,
    boxes: Vec<(i64, Option<i32>)>,
}

impl Trace {
    pub(super) fn record(&mut self, touch: Touch) {
        if touch.touched {
            self.boxes.push((touch.increment, touch.set));
        }
    }

    /// §4.2's algorithm over the recorded boxes.
    fn initial_value(&self) -> i32 {
        let (mut num, mut last) = (0i64, 0i64);
        for &(increment, set) in &self.boxes {
            let negated = -increment;
            if negated != 0 {
                last = negated;
            }
            if let Some(set) = set {
                num += i64::from(set);
                break;
            }
            num += negated;
        }
        (num + last).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }
}

impl CounterState {
    /// The instance a scan traces, if this state is one: each box's ops
    /// are recorded against it (`enter`).
    pub(super) fn touch_target(&self) -> Option<u32> {
        self.trace.as_ref().map(|t| t.target)
    }

    /// The boxes whose reversed counter's computed initial value, as the
    /// walk used it, no longer matches the boxes in its scope as they now
    /// are — the elements to cascade again.
    pub(crate) fn stale_reversed(&self, dom: &Dom<TuiExt>) -> Vec<NodeId> {
        let mut stale = Vec::new();
        for used in &self.auto_reversed {
            let node = dom.node(used.owner.element);
            let own = node.ext().and_then(|e| match used.owner.slot {
                OpBox::Element => e.computed.clone(),
                OpBox::Marker => e.computed_marker().cloned(),
                OpBox::Before => e.computed_before.clone(),
                OpBox::After => e.computed_after.clone(),
            });
            let Some(own) = own else { continue };
            if initial_value(dom, used.owner, &own, &used.name) != used.value
                && !stale.contains(&used.owner.element)
            {
                stale.push(used.owner.element);
            }
        }
        stale
    }
}

/// The initial value of reversed counter `name` instantiated without one
/// by `owner`, whose own ops are `own` (§4.2): its scope's boxes replayed
/// as last cascaded, in tree order — `owner`'s own increments and sets,
/// then (an element) its `::before`, children and `::after` and its
/// following siblings' subtrees, or (a `::before`) its host's children
/// and `::after`.
pub(super) fn initial_value(
    dom: &Dom<TuiExt>,
    owner: Owner,
    own: &ComputedStyle,
    name: &str,
) -> i32 {
    let element = owner.element;
    let parent = match owner.slot {
        OpBox::Element => dom.node(element).parent_node().map(|p| p.id()),
        OpBox::Marker | OpBox::Before | OpBox::After => Some(element),
    };
    let mut scan = CounterState::exact();
    let target = scan.instantiate(name, 0, true, parent);
    scan.trace = Some(Box::new(Trace {
        target,
        boxes: Vec::new(),
    }));
    // The owner box after its resets: its increments and sets.
    let mut rest = own.clone();
    rest.counter_reset.clear();
    rest.content_quotes.clear();
    scan.enter(parent, owner, &rest);
    match owner.slot {
        OpBox::Element => {
            let ops = StoredOps::pseudos_of(dom, element);
            scan.replay_element(parent, element, &ops, |s| s.replay_children(dom, element));
            let mut sibling = dom.node(element).next_sibling().map(|n| n.id());
            while let Some(s) = sibling {
                if takes_part(dom, s) {
                    let ops = StoredOps::of(dom, s);
                    scan.replay_element(parent, s, &ops, |st| st.replay_children(dom, s));
                }
                sibling = dom.node(s).next_sibling().map(|n| n.id());
            }
        }
        OpBox::Before => {
            scan.replay_children(dom, element);
            let after = StoredOps::of(dom, element).after;
            if let Some(after) = after {
                scan.enter(
                    Some(element),
                    Owner {
                        element,
                        slot: OpBox::After,
                    },
                    &after,
                );
            }
        }
        // A marker holds no counter op (CSS Lists 3 §3.2).
        OpBox::Marker | OpBox::After => {}
    }
    scan.trace.as_deref().map_or(0, Trace::initial_value)
}

/// Fill in the computed initial value of every reversed counter
/// `style`'s `owner` box instantiates without one (§4.2).
pub(crate) fn resolve(dom: &Dom<TuiExt>, owner: Owner, style: &mut ComputedStyle) {
    if !style.counter_reset.iter().any(|op| op.is_auto_reversed()) {
        return;
    }
    let values: Vec<Option<i32>> = style
        .counter_reset
        .iter()
        .map(|op| {
            op.is_auto_reversed()
                .then(|| initial_value(dom, owner, style, &op.name))
        })
        .collect();
    for (op, value) in style.counter_reset.iter_mut().zip(values) {
        if let Some(value) = value {
            op.value = value;
        }
    }
}

#[cfg(test)]
impl Trace {
    pub(super) fn for_test(boxes: &[(i64, Option<i32>)]) -> Self {
        Trace {
            target: 0,
            boxes: boxes.to_vec(),
        }
    }

    pub(super) fn initial_value_for_test(&self) -> i32 {
        self.initial_value()
    }
}
