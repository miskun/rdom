//! Counter state for the cascade walk (CSS Lists 3 §4), and the
//! document's quote depth (CSS Generated Content 3 §2.2) — the two
//! values generated content reads that run in tree order.
//!
//! A counter instance is created by `counter-reset` (or implicitly by
//! an increment / read of a counter not in scope) on an element `E`
//! and is in scope for `E`, `E`'s descendants, and `E`'s following
//! siblings with their descendants. That is exactly "until `E`'s
//! parent is left", so instances record the parent they were created
//! under and are dropped when the walk leaves that parent.

use std::rc::Rc;

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::style::{ComputedStyle, CounterOp, QuoteKind};

#[derive(Debug, Clone)]
struct Instance {
    name: String,
    value: i32,
    /// The parent of the element that created the instance; the
    /// instance dies when the walk leaves this element.
    scope_parent: Option<NodeId>,
}

/// The computed styles holding a kept element's counter ops, as last
/// cascaded: its own and its `::before` / `::after` boxes'. `Rc`
/// clones, so a replay can run while the walk mutates the tree.
#[derive(Debug, Default)]
pub(super) struct StoredOps {
    element: Option<Rc<ComputedStyle>>,
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

/// Does `style` create or increment a counter, or move the quote depth
/// (a `<quote>` item of generated content)?
pub(super) fn has_ops(style: &ComputedStyle) -> bool {
    !style.counter_reset.is_empty()
        || !style.counter_increment.is_empty()
        || !style.content_quotes.is_empty()
}

/// Counter instances in creation order (later = innermost).
#[derive(Debug, Default, Clone)]
pub(super) struct CounterState {
    instances: Vec<Instance>,
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
        fn ops(c: Option<&ComputedStyle>) -> (&[CounterOp], &[CounterOp], &[QuoteKind]) {
            c.map_or((&[], &[], &[]), |c| {
                (&c.counter_reset, &c.counter_increment, &c.content_quotes)
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
    /// element's own, its `::before`'s, `children` (whatever accounts for
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
        if let Some(c) = &ops.element {
            self.enter(parent, &c.counter_reset, &c.counter_increment);
        }
        if let Some(c) = &ops.before {
            self.enter(Some(id), &c.counter_reset, &c.counter_increment);
            self.replay_quotes(&c.content_quotes);
        }
        children(self);
        if let Some(c) = &ops.after {
            self.enter(Some(id), &c.counter_reset, &c.counter_increment);
            self.replay_quotes(&c.content_quotes);
        }
        self.exit(id);
    }

    /// Apply an element's `counter-reset` then `counter-increment`
    /// (CSS Lists 3 §3.1.1 – §3.1.2). `parent` is the element's parent,
    /// which bounds the scope of anything created here.
    pub(super) fn enter(
        &mut self,
        parent: Option<NodeId>,
        reset: &[CounterOp],
        increment: &[CounterOp],
    ) {
        for op in reset {
            self.instances.push(Instance {
                name: op.name.clone(),
                value: op.value,
                scope_parent: parent,
            });
        }
        for op in increment {
            match self.instances.iter_mut().rev().find(|i| i.name == op.name) {
                Some(inst) => inst.value = inst.value.saturating_add(op.value),
                // Incrementing a counter not in scope implicitly resets
                // it to 0 on this element first.
                None => self.instances.push(Instance {
                    name: op.name.clone(),
                    value: op.value,
                    scope_parent: parent,
                }),
            }
        }
    }

    /// Leave `element`: instances created by its children go out of
    /// scope.
    pub(super) fn exit(&mut self, element: NodeId) {
        self.instances.retain(|i| i.scope_parent != Some(element));
    }

    /// A kept box's `<quote>` items: their moves of the quote depth.
    fn replay_quotes(&mut self, quotes: &[QuoteKind]) {
        let depth = self.quote_depth.get_mut();
        for q in quotes {
            *depth = q.next_depth(*depth);
        }
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
    /// §3.2: a missing counter reads as 0).
    pub(super) fn value(&self, name: &str) -> i32 {
        self.read.set(true);
        self.instances
            .iter()
            .rev()
            .find(|i| i.name == name)
            .map_or(0, |i| i.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op(name: &str, value: i32) -> CounterOp {
        CounterOp {
            name: name.into(),
            value,
        }
    }

    /// `ol > li` numbering with a nested list: the inner reset is
    /// scoped to the inner `<ol>`, the outer count resumes after it,
    /// and a following sibling `<ol>` starts over.
    #[test]
    fn nested_lists_scope_and_resume() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let ol = dom.create_element("ol");
        let li1 = dom.create_element("li");
        let inner = dom.create_element("ol");
        let inner_li = dom.create_element("li");
        let li2 = dom.create_element("li");
        let ol2 = dom.create_element("ol");
        dom.append_child(root, ol).unwrap();
        dom.append_child(ol, li1).unwrap();
        dom.append_child(li1, inner).unwrap();
        dom.append_child(inner, inner_li).unwrap();
        dom.append_child(ol, li2).unwrap();
        dom.append_child(root, ol2).unwrap();

        let mut st = CounterState::default();
        st.enter(Some(root), &[op("list-item", 0)], &[]); // <ol>
        st.enter(Some(ol), &[], &[op("list-item", 1)]); // <li> 1
        assert_eq!(st.value("list-item"), 1);
        st.enter(Some(li1), &[op("list-item", 0)], &[]); // inner <ol>
        st.enter(Some(inner), &[], &[op("list-item", 1)]); // inner <li>
        assert_eq!(st.value("list-item"), 1);
        st.exit(inner_li);
        st.exit(inner); // inner <ol> closes: its own instance survives until li1 exits
        assert_eq!(
            st.value("list-item"),
            1,
            "still in the inner <ol>'s scope (a following sibling would see it)"
        );
        st.exit(li1); // leaving <li> 1 drops the inner <ol>'s instance
        assert_eq!(st.value("list-item"), 1, "outer count resumes");
        st.enter(Some(ol), &[], &[op("list-item", 1)]); // <li> 2
        assert_eq!(st.value("list-item"), 2);
        st.exit(li2);
        st.exit(ol);
        // <ol> 2 is a following sibling of <ol> 1: the first instance is
        // still in scope, and the reset creates a fresh one.
        st.enter(Some(root), &[op("list-item", 0)], &[]);
        st.enter(Some(ol2), &[], &[op("list-item", 1)]);
        assert_eq!(st.value("list-item"), 1);
    }

    #[test]
    fn increment_without_reset_creates_the_counter_and_missing_reads_zero() {
        let mut st = CounterState::default();
        assert_eq!(st.value("x"), 0);
        st.enter(None, &[], &[op("x", 5)]);
        assert_eq!(st.value("x"), 5);
        st.enter(None, &[], &[op("x", -2)]);
        assert_eq!(st.value("x"), 3);
    }
}
