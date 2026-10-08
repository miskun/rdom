//! Counter state for the cascade walk (CSS Lists 3 §4), and the
//! document's quote depth (CSS Generated Content 3 §2.2) — the two
//! values generated content reads that run in tree order.
//!
//! A counter instance is created by `counter-reset` (or implicitly by
//! an increment, set or read of a counter not in scope) on an element
//! `E` and is in scope for `E`, `E`'s descendants, and `E`'s following
//! siblings with their descendants. That is exactly "until `E`'s
//! parent is left", so instances record the parent they were created
//! under and are dropped when the walk leaves that parent. A counter an
//! element instantiates replaces the same-named one its previous
//! sibling (or itself) created, rather than nesting in it (§4.5).
//!
//! A reversed counter's initial value without an integer depends on the
//! increments after it ([`reversed`]): the cascade computes it from the
//! boxes in its scope as last cascaded, and checks it once the walk is
//! done ([`CounterState::stale_reversed`]).

use std::rc::Rc;

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::style::{ComputedStyle, CounterOp, QuoteKind};

pub(super) mod reversed;
#[cfg(test)]
mod tests;

/// The implicit counter of list items (CSS Lists 3 §4.6).
const LIST_ITEM: &str = "list-item";

#[derive(Debug, Clone)]
struct Instance {
    name: String,
    value: i32,
    /// The parent of the element that created the instance; the
    /// instance dies when the walk leaves this element.
    scope_parent: Option<NodeId>,
    /// A reversed counter (§4.2): the implicit `list-item` increment
    /// counts it down.
    reversed: bool,
    /// Identity within one state, which a [`reversed`] scan traces.
    id: u32,
}

/// Which box of an element holds counter ops: the element's own, or its
/// `::before` / `::after` (CSS Pseudo-Elements 4 §4: its first and last
/// child).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OpBox {
    Element,
    Marker,
    Before,
    After,
}

/// A box that holds counter ops: an element and which of its boxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Owner {
    pub element: NodeId,
    pub slot: OpBox,
}

impl Owner {
    pub(super) fn element(element: NodeId) -> Self {
        Owner {
            element,
            slot: OpBox::Element,
        }
    }
}

/// The computed styles holding a kept element's counter ops, as last
/// cascaded: its own and its `::before` / `::after` boxes'. `Rc`
/// clones, so a replay can run while the walk mutates the tree.
#[derive(Debug, Default)]
pub(super) struct StoredOps {
    element: Option<Rc<ComputedStyle>>,
    marker: Option<Rc<ComputedStyle>>,
    before: Option<Rc<ComputedStyle>>,
    after: Option<Rc<ComputedStyle>>,
}

impl StoredOps {
    /// Everything `id` holds. An element nothing cascaded yet holds
    /// nothing — its own cascade supplies its ops.
    pub(super) fn of(dom: &Dom<TuiExt>, id: NodeId) -> Self {
        dom.node(id)
            .ext()
            .map_or_else(Self::default, |e| StoredOps {
                element: e.computed.clone(),
                marker: e.computed_marker().cloned(),
                before: e.computed_before.clone(),
                after: e.computed_after.clone(),
            })
    }

    /// `id`'s `::before` / `::after` only: the walk just recomputed the
    /// element itself (and applied its own ops) and keeps the rest.
    pub(super) fn pseudos_of(dom: &Dom<TuiExt>, id: NodeId) -> Self {
        StoredOps {
            element: None,
            ..Self::of(dom, id)
        }
    }
}

/// Does `id`'s subtree take part in counters — create, increment or
/// read one (`TuiExt::tree_has_counters`)? A node without an `Ext` (text,
/// a fragment) counts when it has children, which may.
pub(super) fn takes_part(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    match node.ext() {
        Some(ext) => ext.tree_has_counters,
        None => node.first_child().is_some(),
    }
}

/// Does `style` create, increment or set a counter — explicitly, or as
/// a list item (§4.6) — or move the quote depth (a `<quote>` item of
/// generated content)?
pub(super) fn has_ops(style: &ComputedStyle) -> bool {
    !style.counter_reset.is_empty()
        || !style.counter_increment.is_empty()
        || !style.counter_set.is_empty()
        || style.list_item
        || !style.content_quotes.is_empty()
}

/// A reversed counter whose initial value a box computed (§4.2), as the
/// walk used it.
#[derive(Debug, Clone)]
struct AutoReversed {
    owner: Owner,
    name: String,
    value: i32,
}

/// Counter instances in creation order (later = innermost).
#[derive(Debug, Default, Clone)]
pub(super) struct CounterState {
    instances: Vec<Instance>,
    /// The next [`Instance::id`].
    next_id: u32,
    /// The walk must account for every element in tree order — the
    /// sheets use counters — so a subtree it skips has its stored ops
    /// replayed ([`replay_children`](Self::replay_children)).
    exact: bool,
    /// An exact walk met a counter op that differs from the last
    /// cascade's: every counter value after it in tree order may differ,
    /// so the walk recomputes the elements after it that read one.
    changed: bool,
    /// A counter or the quote depth was read since the last
    /// [`take_read`](Self::take_read).
    read: std::cell::Cell<bool>,
    /// The quote depth (CSS Generated Content 3 §2.2): document-wide, in
    /// tree order, moved by the `<quote>` items of generated content.
    quote_depth: std::cell::Cell<u32>,
    /// The reversed counters with a computed initial value the walk
    /// used, to check once it is done.
    auto_reversed: Vec<AutoReversed>,
    /// A [`reversed`] scan: the instance it follows, and per box that
    /// touched it, the box's increment and set.
    trace: Option<Box<reversed::Trace>>,
}

impl CounterState {
    /// A state for a walk that skips subtrees yet must keep counters
    /// exact (sheets that use counters).
    pub(super) fn exact() -> Self {
        CounterState {
            exact: true,
            ..Self::default()
        }
    }

    /// Have the counter values moved since the last cascade — an op
    /// before this point in the walk differs ([`note_ops`](Self::note_ops))?
    pub(super) fn is_changed(&self) -> bool {
        self.changed
    }

    /// The values from here on may differ from the last cascade's.
    pub(super) fn mark_changed(&mut self) {
        self.changed = self.exact;
    }

    /// A box was recomputed with `new` where it had `old` (`None`: no
    /// box, no ops): when an exact walk sees their counter ops differ,
    /// the counter values after it [are changed](Self::is_changed).
    pub(super) fn note_ops(&mut self, old: Option<&ComputedStyle>, new: Option<&ComputedStyle>) {
        if !self.exact || self.changed {
            return;
        }
        type Ops<'a> = (
            &'a [CounterOp],
            &'a [CounterOp],
            &'a [CounterOp],
            &'a [QuoteKind],
            bool,
        );
        fn ops(c: Option<&ComputedStyle>) -> Ops<'_> {
            c.map_or((&[], &[], &[], &[], false), |c| {
                (
                    &c.counter_reset,
                    &c.counter_increment,
                    &c.counter_set,
                    &c.content_quotes,
                    c.list_item,
                )
            })
        }
        if ops(old) != ops(new) {
            self.changed = true;
        }
    }

    /// Was a counter read ([`value`](Self::value)) since the last call?
    pub(super) fn take_read(&self) -> bool {
        self.read.replace(false)
    }

    /// Account for the elements under `id` (its children's subtrees),
    /// which keep their computed styles: replay each child's stored ops
    /// ([`replay_element`](Self::replay_element)) in tree order. A no-op
    /// unless the state is [`exact`](Self::exact).
    pub(super) fn replay_children(&mut self, dom: &Dom<TuiExt>, id: NodeId) {
        if !self.exact {
            return;
        }
        for child in dom.node(id).child_nodes().map(|n| n.id()) {
            // A subtree without counters has no op to replay.
            if !takes_part(dom, child) {
                continue;
            }
            #[cfg(test)]
            super::walk::probe::visit();
            let ops = StoredOps::of(dom, child);
            self.replay_element(Some(id), child, &ops, |s| s.replay_children(dom, child));
        }
    }

    /// Replay a kept element's stored counter ops in tree order — the
    /// element's own, its `::marker`'s (quotes in its `content`), its
    /// `::before`'s, `children` (whatever accounts for
    /// its subtree), its `::after`'s — then leave it. `::before` is the
    /// element's first child and `::after` its last (CSS Pseudo-Elements
    /// 4 §4), so their instances are scoped to the element. The one
    /// replay every partial walk uses: the cascade's walk between
    /// subtree roots, a restyle's kept element and its kept children.
    pub(super) fn replay_element(
        &mut self,
        parent: Option<NodeId>,
        id: NodeId,
        ops: &StoredOps,
        children: impl FnOnce(&mut Self),
    ) {
        let owner = |slot| Owner { element: id, slot };
        if let Some(c) = &ops.element {
            self.enter(parent, owner(OpBox::Element), c);
        }
        if let Some(c) = &ops.marker {
            self.enter(Some(id), owner(OpBox::Marker), c);
        }
        if let Some(c) = &ops.before {
            self.enter(Some(id), owner(OpBox::Before), c);
        }
        children(self);
        if let Some(c) = &ops.after {
            self.enter(Some(id), owner(OpBox::After), c);
        }
        self.exit(id);
    }

    /// Apply a box's counter ops (CSS Lists 3 §4.4): instantiate its
    /// `counter-reset` counters, then increment — its `counter-increment`
    /// and, for a list item that names no `list-item` increment, the
    /// implicit one (§4.6: +1, or -1 on a reversed counter) — then set
    /// its `counter-set` ones; and move the quote depth by its `<quote>`
    /// items. `parent` is the box's parent, which bounds the scope of
    /// anything created here; `owner` the box.
    pub(super) fn enter(&mut self, parent: Option<NodeId>, owner: Owner, style: &ComputedStyle) {
        let mut touched = reversed::Touch::aimed_at(self.touch_target());
        for op in &style.counter_reset {
            if op.is_auto_reversed() && self.trace.is_none() {
                self.auto_reversed.push(AutoReversed {
                    owner,
                    name: op.name.clone(),
                    value: op.value,
                });
            }
            self.instantiate(&op.name, op.value, op.reversed, parent);
        }
        for op in &style.counter_increment {
            self.increment(&op.name, op.value, parent, &mut touched);
        }
        if style.list_item
            && !style
                .counter_increment
                .iter()
                .any(|op| op.name == LIST_ITEM)
        {
            let down = self.innermost(LIST_ITEM).is_some_and(|i| i.reversed);
            self.increment(LIST_ITEM, if down { -1 } else { 1 }, parent, &mut touched);
        }
        for op in &style.counter_set {
            match self.instances.iter_mut().rev().find(|i| i.name == op.name) {
                Some(inst) => {
                    inst.value = op.value;
                    touched.set(inst.id, op.value);
                }
                None => {
                    self.instantiate(&op.name, op.value, false, parent);
                }
            }
        }
        if let Some(trace) = self.trace.as_deref_mut() {
            trace.record(touched);
        }
        let depth = self.quote_depth.get_mut();
        for q in &style.content_quotes {
            *depth = q.next_depth(*depth);
        }
    }

    /// Instantiate counter `name` (§4.5): it replaces the innermost
    /// same-named counter when that one was created by this element or a
    /// previous sibling (same `parent`), and nests otherwise.
    fn instantiate(
        &mut self,
        name: &str,
        value: i32,
        reversed: bool,
        parent: Option<NodeId>,
    ) -> u32 {
        if let Some(at) = self.instances.iter().rposition(|i| i.name == name)
            && self.instances[at].scope_parent == parent
        {
            self.instances.remove(at);
        }
        let id = self.next_id;
        self.next_id += 1;
        self.instances.push(Instance {
            name: name.to_string(),
            value,
            scope_parent: parent,
            reversed,
            id,
        });
        id
    }

    /// Increment the innermost counter `name` by `by`; one not in scope
    /// is instantiated at 0 first (§4.4).
    fn increment(
        &mut self,
        name: &str,
        by: i32,
        parent: Option<NodeId>,
        touched: &mut reversed::Touch,
    ) {
        if self.innermost(name).is_none() {
            self.instantiate(name, 0, false, parent);
        }
        if let Some(inst) = self.instances.iter_mut().rev().find(|i| i.name == name) {
            inst.value = inst.value.saturating_add(by);
            touched.increment(inst.id, by);
        }
    }

    fn innermost(&self, name: &str) -> Option<&Instance> {
        self.instances.iter().rev().find(|i| i.name == name)
    }

    /// Leave `element`: instances created by its children go out of
    /// scope.
    pub(super) fn exit(&mut self, element: NodeId) {
        self.instances.retain(|i| i.scope_parent != Some(element));
    }

    /// The text of a `<quote>` item at this point of the walk, moving
    /// the quote depth (CSS Generated Content 3 §2.2): `open-quote` the
    /// opening mark of the current level, `close-quote` the closing mark
    /// of the level it returns to — nothing at depth 0, which it leaves —
    /// and the `no-*` forms no mark. `pair` gives a level's marks
    /// (`Quotes::pair`; `None` under `quotes: none`).
    pub(super) fn quote<'q>(
        &self,
        kind: QuoteKind,
        pair: impl Fn(u32) -> Option<(&'q str, &'q str)>,
    ) -> &'q str {
        self.read.set(true);
        let depth = self.quote_depth.get();
        self.quote_depth.set(kind.next_depth(depth));
        match kind {
            QuoteKind::Open => pair(depth).map_or("", |p| p.0),
            QuoteKind::Close if depth > 0 => pair(depth - 1).map_or("", |p| p.1),
            _ => "",
        }
    }

    /// Every instance of `name` in scope, outermost first — `counters()`
    /// (CSS Lists 3 §4.3); a counter not in scope reads as one 0.
    pub(super) fn values(&self, name: &str) -> Vec<i32> {
        self.read.set(true);
        let all: Vec<i32> = self
            .instances
            .iter()
            .filter(|i| i.name == name)
            .map(|i| i.value)
            .collect();
        if all.is_empty() { vec![0] } else { all }
    }

    /// The innermost instance of `name` in scope, or 0 (CSS Lists 3
    /// §4.3: a missing counter reads as 0).
    pub(super) fn value(&self, name: &str) -> i32 {
        self.read.set(true);
        self.innermost(name).map_or(0, |i| i.value)
    }
}
