//! `MutationObserver` — W3C-style subscription to DOM changes.
//!
//! Every mutation entry point (`set_attribute`, `add_class`,
//! `append_child`, ...) emits a `Mutation` record to every registered
//! observer. Observers can implement anything — a cascade dirty
//! tracker (rdom-tui), a devtools inspector, an accessibility mirror,
//! reactive state bindings, undo/redo, collaborative sync.
//!
//! ## Zero cost when unused
//!
//! `Dom<()>` with no observers registered pays one `is_empty()` check
//! per mutation — no record is allocated, no closure is invoked. Only
//! when observers are registered does mutation firing become active.
//!
//! ## Re-entrancy
//!
//! Observer callbacks may READ the Dom freely but must NOT mutate the
//! tree. A runtime guard (`is_observing`) panics on re-entrant mutation
//! with a clear message. This keeps the cascade dirty-tracker's
//! invariants intact: the set of dirty roots must be computed from a
//! fixed tree state, not one that shifts under each notification.
//!
//! Installing or removing observers *is* allowed from inside a
//! callback (the web's `MutationObserver.disconnect()` inside the
//! callback). Notification snapshots the observer ids up front and
//! takes each observer out of its slot only for its own call, so a
//! removed observer — including the running one — gets nothing further
//! and an added one sees only later records.
//!
//! ## Nested mutations (fragment unwrap)
//!
//! Some public APIs (`append_child` of a `Fragment`) recursively call
//! themselves internally. Each recursive call fires its own record,
//! matching browser behavior: inserting a fragment with N children
//! produces N `ChildListChanged` records, not one. This is correct
//! but observers should handle reasonable record volume.

use crate::dom::Dom;
use crate::node_id::NodeId;

/// Which interaction state changed. Fired by `Dom::set_hovered` /
/// `Dom::set_focused` so pseudo-class matches (`:hover`, `:focus`)
/// can invalidate cleanly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InteractionKind {
    Hover,
    Focus,
}

/// One DOM mutation notification.
#[derive(Debug, Clone)]
pub enum Mutation {
    /// `set_attribute` / `remove_attribute` / `toggle_attribute`.
    /// `old == None && new.is_some()` → attribute added.
    /// `old.is_some() && new == None` → attribute removed.
    /// Both `Some` → value changed.
    AttributeChanged {
        id: NodeId,
        name: String,
        old: Option<String>,
        new: Option<String>,
    },
    /// One class was added or removed. `add_class` / `remove_class` /
    /// `toggle_class` each fire a single record with one entry in
    /// either `added` or `removed`; `replace_class` fires once with
    /// both populated.
    ClassChanged {
        id: NodeId,
        added: Vec<String>,
        removed: Vec<String>,
    },
    /// A parent's child list was mutated. Fires once per top-level
    /// operation — e.g. `append_child(parent, frag)` where frag has
    /// three children fires three records, one per unwrapped child.
    ChildListChanged {
        parent: NodeId,
        added: Vec<NodeId>,
        removed: Vec<NodeId>,
    },
    /// Text or Comment node data changed.
    CharacterDataChanged {
        id: NodeId,
        old: String,
        new: String,
    },
    /// Hovered or focused node changed. Both `prev` and `next` may be
    /// `Some` (re-pointed), either may be `None` (cleared or set-from-
    /// nothing). The cascade dirty-tracker uses this to invalidate
    /// both sides' subtrees for `:hover` / `:focus` re-matching.
    InteractionChanged {
        prev: Option<NodeId>,
        next: Option<NodeId>,
        kind: InteractionKind,
    },
    /// Document selection changed via `Dom::set_selection`. Either
    /// `prev` or `next` may be `None`; cleared selections fire
    /// `next: None`. Paint observers use this to invalidate the
    /// `::selection` overlay on the nodes whose range changed.
    SelectionChanged {
        prev: Option<crate::Selection>,
        next: Option<crate::Selection>,
    },
    /// **About to detach** — fired in `detach_from_parent` BEFORE
    /// the structural unlink, so observers can dispatch implicit
    /// `blur` / `focusout` / `mouseleave` / `mouseout` events
    /// while the tree is still intact (the focused/hovered node's
    /// parent_node() still walks the live ancestor chain).
    ///
    /// `focused` is `Some(id)` iff `dom.focused()` was inside the
    /// subtree rooted at `detached_root` at the moment of detach.
    /// `hovered` likewise for hover. If neither is set, no
    /// `PreDetach` record is emitted (the structurally-cheap
    /// short-circuit path in `purge_interaction_state_for_subtree`
    /// also gates this record's emission).
    ///
    /// The subsequent `InteractionChanged { next: None, kind }`
    /// records still fire from the purge step — observers that
    /// only care about the post-detach state can listen to those.
    /// `PreDetach` is for observers that need to dispatch real
    /// DOM events with bubbling intact.
    PreDetach {
        /// Subtree root being detached.
        detached_root: NodeId,
        /// Pre-detach focused node, if it was inside the subtree.
        focused: Option<NodeId>,
        /// Pre-detach hovered node, if it was inside the subtree.
        hovered: Option<NodeId>,
    },
}

/// Observer callback trait. Receives a mutable `&mut Dom<Ext>` so
/// observers can READ freely — but any attempt to mutate the tree
/// inside `observe()` panics via the `is_observing` guard. Installing
/// or removing observers from inside the callback is allowed (the web's
/// `MutationObserver.disconnect()` inside the callback): a removed
/// observer — including the one currently running — receives nothing
/// further; an added one receives only later records.
pub trait MutationObserver<Ext>: 'static {
    fn observe(&mut self, dom: &mut Dom<Ext>, record: &Mutation);
}

/// Handle returned from `add_mutation_observer`. Pass to
/// `remove_mutation_observer` to unregister.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObserverId(pub(crate) u32);

/// One registered observer. The box is `None` while that observer is
/// being invoked (taken out so the callback can receive `&mut Dom`) and
/// restored by id afterwards.
type ObserverSlot<Ext> = (ObserverId, Option<Box<dyn MutationObserver<Ext>>>);

pub(crate) struct ObserverStore<Ext> {
    next_id: u32,
    entries: Vec<ObserverSlot<Ext>>,
}

impl<Ext> Default for ObserverStore<Ext> {
    fn default() -> Self {
        Self {
            next_id: 0,
            entries: Vec::new(),
        }
    }
}

impl<Ext> std::fmt::Debug for ObserverStore<Ext> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObserverStore")
            .field("count", &self.entries.len())
            .field("next_id", &self.next_id)
            .finish()
    }
}

impl<Ext> ObserverStore<Ext> {
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ─── Dom API ────────────────────────────────────────────────────────

impl<Ext: 'static> Dom<Ext> {
    /// Register a mutation observer. Fires for every subsequent DOM
    /// mutation on this `Dom`. Returns a handle for removal. Callable
    /// from inside an `observe()` callback; the new observer first sees
    /// the *next* record.
    pub fn add_mutation_observer(
        &mut self,
        observer: Box<dyn MutationObserver<Ext>>,
    ) -> ObserverId {
        let id = ObserverId(self.observers.next_id);
        self.observers.next_id += 1;
        self.observers.entries.push((id, Some(observer)));
        id
    }

    /// Remove a previously-registered observer. Returns `true` if the
    /// observer existed and was removed. Callable from inside an
    /// `observe()` callback, including by the observer being notified
    /// (it is dropped once its call returns).
    pub fn remove_mutation_observer(&mut self, id: ObserverId) -> bool {
        let before = self.observers.entries.len();
        self.observers.entries.retain(|(oid, _)| *oid != id);
        self.observers.entries.len() < before
    }

    /// How many observers are currently registered.
    pub fn observer_count(&self) -> usize {
        self.observers.entries.len()
    }

    /// Emit a mutation record to every observer registered at the time
    /// of the call. Panics if called while already inside `observe()`
    /// (re-entrant mutation). Fast-path noop when no observers AND we're
    /// not already observing — the `is_observing` check comes first so
    /// re-entrancy is detected even while an observer is taken out of
    /// its slot for its own call.
    pub(crate) fn fire_mutation(&mut self, record: Mutation) {
        if self.is_observing {
            panic!(
                "rdom-core: mutation attempted inside MutationObserver callback: {:?}. \
                 Observers must not mutate the tree during `observe()`. \
                 Schedule the mutation for after dispatch returns.",
                record
            );
        }
        if self.observers.is_empty() {
            return;
        }

        // Snapshot the ids to notify: observers added during this
        // notification are not in the snapshot (they see later records);
        // observers removed by an earlier callback are skipped when the
        // re-locate fails. Each observer is taken out of its slot for
        // the duration of its own call so it can receive `&mut Dom`,
        // then restored by id — the same shape as `dispatch::fire_at`.
        let ids: Vec<ObserverId> = self.observers.entries.iter().map(|(id, _)| *id).collect();
        self.is_observing = true;
        for id in ids {
            let Some(pos) = self
                .observers
                .entries
                .iter()
                .position(|(oid, _)| *oid == id)
            else {
                continue; // removed by a previous observer in this round
            };
            let Some(mut obs) = self.observers.entries[pos].1.take() else {
                continue;
            };
            // A panicking observer must not poison the Dom: hosts that
            // catch the panic (rdom-tui does, to restore the terminal)
            // need the re-entrancy flag cleared and the observer put
            // back. Restore first, then re-raise the original payload.
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                obs.observe(self, &record);
            }));
            if let Some(pos) = self
                .observers
                .entries
                .iter()
                .position(|(oid, _)| *oid == id)
            {
                self.observers.entries[pos].1 = Some(obs);
            }
            // Else: the observer removed itself (or was removed) during
            // its own callback; dropping `obs` completes the removal.
            if let Err(payload) = outcome {
                self.is_observing = false;
                std::panic::resume_unwind(payload);
            }
        }
        self.is_observing = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dom;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Collect all records into a shared Vec for assertions.
    struct Collector {
        records: Rc<RefCell<Vec<Mutation>>>,
    }
    impl MutationObserver<()> for Collector {
        fn observe(&mut self, _dom: &mut Dom<()>, record: &Mutation) {
            self.records.borrow_mut().push(record.clone());
        }
    }

    fn install_collector(dom: &mut Dom<()>) -> (ObserverId, Rc<RefCell<Vec<Mutation>>>) {
        let records = Rc::new(RefCell::new(Vec::new()));
        let obs = Box::new(Collector {
            records: records.clone(),
        });
        let id = dom.add_mutation_observer(obs);
        (id, records)
    }

    #[test]
    fn add_and_remove_observer() {
        let mut dom: Dom = Dom::new();
        let (id, _) = install_collector(&mut dom);
        assert_eq!(dom.observer_count(), 1);
        assert!(dom.remove_mutation_observer(id));
        assert_eq!(dom.observer_count(), 0);
        // Removing same id twice → false.
        assert!(!dom.remove_mutation_observer(id));
    }

    /// A panicking observer must not poison the `Dom`: once the host
    /// catches the panic (rdom-tui does, to restore the terminal), the
    /// re-entrancy flag is clear again and every observer — including
    /// the one that panicked — is still installed.
    #[test]
    fn panicking_observer_leaves_dom_usable_and_observers_installed() {
        struct Bomb;
        impl MutationObserver<()> for Bomb {
            fn observe(&mut self, _dom: &mut Dom<()>, _record: &Mutation) {
                panic!("observer bomb");
            }
        }
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let (_, before) = install_collector(&mut dom);
        let bomb_id = dom.add_mutation_observer(Box::new(Bomb));
        let (_, after) = install_collector(&mut dom);
        assert_eq!(dom.observer_count(), 3);

        let el = dom.create_element("div");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            dom.append_child(root, el).unwrap();
        }));
        assert!(result.is_err(), "the bomb must actually fire");

        // Dom is not stuck in "observing" state and still has all three.
        assert_eq!(dom.observer_count(), 3);
        assert!(dom.remove_mutation_observer(bomb_id));
        let n_before = before.borrow().len();
        let n_after = after.borrow().len();
        dom.set_attribute(el, "id", "x").unwrap();
        assert_eq!(before.borrow().len(), n_before + 1);
        assert_eq!(after.borrow().len(), n_after + 1);
    }

    /// `MutationObserver.disconnect()` inside the callback is legal on
    /// the web. The in-flight observer removes itself: the call reports
    /// success, later mutations don't reach it, the others are unaffected.
    #[test]
    fn observer_can_remove_itself_during_callback() {
        struct SelfRemover {
            me: Rc<std::cell::Cell<Option<ObserverId>>>,
            seen: Rc<std::cell::Cell<u32>>,
            removed: Rc<std::cell::Cell<Option<bool>>>,
        }
        impl MutationObserver<()> for SelfRemover {
            fn observe(&mut self, dom: &mut Dom<()>, _record: &Mutation) {
                self.seen.set(self.seen.get() + 1);
                let id = self.me.get().expect("id stored before first mutation");
                self.removed.set(Some(dom.remove_mutation_observer(id)));
            }
        }
        let mut dom: Dom = Dom::new();
        let (_, other) = install_collector(&mut dom);
        let me = Rc::new(std::cell::Cell::new(None));
        let seen = Rc::new(std::cell::Cell::new(0));
        let removed = Rc::new(std::cell::Cell::new(None));
        let id = dom.add_mutation_observer(Box::new(SelfRemover {
            me: me.clone(),
            seen: seen.clone(),
            removed: removed.clone(),
        }));
        me.set(Some(id));
        assert_eq!(dom.observer_count(), 2);

        let el = dom.create_element("div");
        dom.set_attribute(el, "id", "a").unwrap();
        assert_eq!(seen.get(), 1);
        assert_eq!(
            removed.get(),
            Some(true),
            "removal inside observe() succeeds"
        );
        assert_eq!(dom.observer_count(), 1);

        let n = other.borrow().len();
        dom.set_attribute(el, "id", "b").unwrap();
        assert_eq!(seen.get(), 1, "the removed observer gets nothing more");
        assert_eq!(
            other.borrow().len(),
            n + 1,
            "the other observer still fires"
        );
    }

    /// An observer installed from inside a callback receives only records
    /// for *later* mutations, in registration order after the existing ones.
    #[test]
    fn observer_added_during_callback_sees_only_later_records() {
        struct Installer {
            installed: Rc<std::cell::Cell<bool>>,
            records: Rc<RefCell<Vec<Mutation>>>,
        }
        impl MutationObserver<()> for Installer {
            fn observe(&mut self, dom: &mut Dom<()>, _record: &Mutation) {
                if !self.installed.replace(true) {
                    dom.add_mutation_observer(Box::new(Collector {
                        records: self.records.clone(),
                    }));
                }
            }
        }
        let mut dom: Dom = Dom::new();
        let late = Rc::new(RefCell::new(Vec::new()));
        dom.add_mutation_observer(Box::new(Installer {
            installed: Rc::new(std::cell::Cell::new(false)),
            records: late.clone(),
        }));
        let el = dom.create_element("div");
        dom.set_attribute(el, "id", "first").unwrap();
        assert_eq!(dom.observer_count(), 2);
        assert!(late.borrow().is_empty(), "not the record that installed it");
        dom.set_attribute(el, "id", "second").unwrap();
        assert_eq!(late.borrow().len(), 1);
    }

    /// Removing an observer that has not fired yet in the current round
    /// takes effect immediately: it is skipped for this record too.
    #[test]
    fn observer_can_remove_a_later_observer_before_it_fires() {
        struct Remover {
            victim: Rc<std::cell::Cell<Option<ObserverId>>>,
        }
        impl MutationObserver<()> for Remover {
            fn observe(&mut self, dom: &mut Dom<()>, _record: &Mutation) {
                if let Some(v) = self.victim.take() {
                    assert!(dom.remove_mutation_observer(v));
                }
            }
        }
        let mut dom: Dom = Dom::new();
        let victim = Rc::new(std::cell::Cell::new(None));
        dom.add_mutation_observer(Box::new(Remover {
            victim: victim.clone(),
        }));
        let (victim_id, victim_records) = install_collector(&mut dom);
        victim.set(Some(victim_id));

        let el = dom.create_element("div");
        dom.set_attribute(el, "id", "x").unwrap();
        assert!(
            victim_records.borrow().is_empty(),
            "removed before its turn"
        );
        assert_eq!(dom.observer_count(), 1);
    }

    #[test]
    fn no_observers_means_no_fires() {
        // Without observers, no allocations happen inside fire_mutation.
        // We can't directly test that, but we can verify mutations still
        // work (they do — it's a no-op fast path).
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let _ = dom.set_attribute(el, "id", "x");
        // No panic, no observer count increment.
        assert_eq!(dom.observer_count(), 0);
    }

    #[test]
    fn attribute_changed_fires_with_old_new() {
        let mut dom: Dom = Dom::new();
        let (_, records) = install_collector(&mut dom);
        let el = dom.create_element("div");
        dom.set_attribute(el, "role", "banner").unwrap();
        dom.set_attribute(el, "role", "navigation").unwrap();
        dom.remove_attribute(el, "role").unwrap();

        let recs = records.borrow();
        // 3 records: add (None → banner), change (banner → navigation),
        // remove (navigation → None).
        let matches: Vec<_> = recs
            .iter()
            .filter_map(|r| match r {
                Mutation::AttributeChanged { old, new, .. } => Some((old.clone(), new.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0], (None, Some("banner".into())));
        assert_eq!(
            matches[1],
            (Some("banner".into()), Some("navigation".into()))
        );
        assert_eq!(matches[2], (Some("navigation".into()), None));
    }

    #[test]
    fn class_changed_fires_add_remove_toggle_replace() {
        let mut dom: Dom = Dom::new();
        let (_, records) = install_collector(&mut dom);
        let el = dom.create_element("div");
        dom.add_class(el, "active").unwrap();
        dom.remove_class(el, "active").unwrap();
        dom.toggle_class(el, "on").unwrap(); // add
        dom.toggle_class(el, "on").unwrap(); // remove
        dom.add_class(el, "old").unwrap();
        dom.replace_class(el, "old", "new").unwrap();

        let cls_recs: Vec<_> = records
            .borrow()
            .iter()
            .filter_map(|r| match r {
                Mutation::ClassChanged { added, removed, .. } => {
                    Some((added.clone(), removed.clone()))
                }
                _ => None,
            })
            .collect();
        // 5 records (not 6, because remove_class of something not present
        // does not fire).
        assert_eq!(cls_recs.len(), 6);
        assert_eq!(cls_recs[0], (vec!["active".to_string()], vec![]));
        assert_eq!(cls_recs[1], (vec![], vec!["active".to_string()]));
        assert_eq!(cls_recs[2], (vec!["on".to_string()], vec![]));
        assert_eq!(cls_recs[3], (vec![], vec!["on".to_string()]));
        assert_eq!(cls_recs[4], (vec!["old".to_string()], vec![]));
        assert_eq!(
            cls_recs[5],
            (vec!["new".to_string()], vec!["old".to_string()])
        );
    }

    #[test]
    fn child_list_changed_on_append() {
        let mut dom: Dom = Dom::new();
        let (_, records) = install_collector(&mut dom);
        let parent = dom.create_element("div");
        let child = dom.create_element("span");
        dom.append_child(parent, child).unwrap();

        let tree_recs: Vec<_> = records
            .borrow()
            .iter()
            .filter_map(|r| match r {
                Mutation::ChildListChanged {
                    parent,
                    added,
                    removed,
                } => Some((*parent, added.clone(), removed.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(tree_recs.len(), 1);
        assert_eq!(tree_recs[0].0, parent);
        assert_eq!(tree_recs[0].1, vec![child]);
        assert_eq!(tree_recs[0].2, Vec::<NodeId>::new());
    }

    #[test]
    fn child_list_changed_on_remove() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("div");
        let child = dom.create_element("span");
        dom.append_child(parent, child).unwrap();
        // Install AFTER the append so the remove is the only recorded op.
        let (_, records) = install_collector(&mut dom);
        dom.remove_child(parent, child).unwrap();

        let rec = records
            .borrow()
            .iter()
            .find(|r| matches!(r, Mutation::ChildListChanged { .. }))
            .cloned()
            .unwrap();
        match rec {
            Mutation::ChildListChanged { added, removed, .. } => {
                assert!(added.is_empty());
                assert_eq!(removed, vec![child]);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn interaction_changed_fires_on_set_hovered() {
        let mut dom: Dom = Dom::new();
        let (_, records) = install_collector(&mut dom);
        let el = dom.create_element("div");
        dom.set_hovered(Some(el));
        dom.set_hovered(None);

        let interactions: Vec<_> = records
            .borrow()
            .iter()
            .filter_map(|r| match r {
                Mutation::InteractionChanged { prev, next, kind } => Some((*prev, *next, *kind)),
                _ => None,
            })
            .collect();
        assert_eq!(interactions.len(), 2);
        assert_eq!(interactions[0], (None, Some(el), InteractionKind::Hover));
        assert_eq!(interactions[1], (Some(el), None, InteractionKind::Hover));
    }

    #[test]
    fn interaction_changed_fires_on_set_focused() {
        let mut dom: Dom = Dom::new();
        let (_, records) = install_collector(&mut dom);
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        dom.set_focused(Some(a));
        dom.set_focused(Some(b));

        let interactions: Vec<_> = records
            .borrow()
            .iter()
            .filter_map(|r| match r {
                Mutation::InteractionChanged {
                    kind: InteractionKind::Focus,
                    prev,
                    next,
                } => Some((*prev, *next)),
                _ => None,
            })
            .collect();
        assert_eq!(interactions.len(), 2);
        assert_eq!(interactions[0], (None, Some(a)));
        assert_eq!(interactions[1], (Some(a), Some(b)));
    }

    #[test]
    fn set_hovered_to_same_does_not_fire() {
        // Self-transition: no change, no record.
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.set_hovered(Some(el));
        let (_, records) = install_collector(&mut dom);
        dom.set_hovered(Some(el));
        assert!(
            records
                .borrow()
                .iter()
                .all(|r| !matches!(r, Mutation::InteractionChanged { .. }))
        );
    }

    #[test]
    fn re_entrant_mutation_panics() {
        // An observer that tries to mutate the tree during its callback
        // should trigger the is_observing guard and panic. We mutate an
        // Element (not the Fragment root) so the mutation actually fires.
        struct EvilObserver {
            target: NodeId,
        }
        impl MutationObserver<()> for EvilObserver {
            fn observe(&mut self, dom: &mut Dom<()>, _record: &Mutation) {
                // This should panic: re-entering set_attribute during observe.
                let _ = dom.set_attribute(self.target, "evil", "1");
            }
        }
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.add_mutation_observer(Box::new(EvilObserver { target: el }));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Fire a mutation — the observer tries to mutate, panics.
            let _ = dom.set_attribute(el, "x", "1");
        }));
        assert!(result.is_err(), "expected panic on re-entrant mutation");
    }

    #[test]
    fn multiple_observers_all_fire() {
        let mut dom: Dom = Dom::new();
        let (_, r1) = install_collector(&mut dom);
        let (_, r2) = install_collector(&mut dom);
        let el = dom.create_element("div");
        dom.set_attribute(el, "id", "x").unwrap();
        assert!(!r1.borrow().is_empty());
        assert!(!r2.borrow().is_empty());
    }

    #[test]
    fn unregistered_observer_does_not_fire() {
        let mut dom: Dom = Dom::new();
        let (id, records) = install_collector(&mut dom);
        dom.remove_mutation_observer(id);
        let el = dom.create_element("div");
        dom.set_attribute(el, "id", "x").unwrap();
        assert!(records.borrow().is_empty());
    }

    #[test]
    fn toggle_attribute_fires_twice() {
        let mut dom: Dom = Dom::new();
        let (_, records) = install_collector(&mut dom);
        let el = dom.create_element("input");
        dom.toggle_attribute(el, "disabled").unwrap(); // add
        dom.toggle_attribute(el, "disabled").unwrap(); // remove

        let attr_count = records
            .borrow()
            .iter()
            .filter(|r| matches!(r, Mutation::AttributeChanged { .. }))
            .count();
        assert_eq!(attr_count, 2);
    }
}
