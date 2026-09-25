//! HTML §4.10.7 "selectedness setting algorithm" and when it runs.
//!
//! A single-select (no `multiple`) with display size 1 always shows one
//! option: with none selected, the first option that is not disabled
//! is; with several selected, only the last stays. Browsers run it
//! when an option is inserted into or removed from the select's list of
//! options, and when the select is reset. rdom runs it:
//!
//! - at mount, over every `<select>` in the tree ([`seed_all`], from
//!   `App::build`);
//! - after a `<form>` reset (`state::reset_to_default`);
//! - after `<option>` / `<optgroup>` insertions and removals, through
//!   [`Selectedness`]: a mutation observer queues the affected select
//!   (observers may not mutate the tree) and the `App` flushes the
//!   queue before it handles the next event and before each frame's
//!   cascade — not synchronously inside the mutation, as a browser
//!   does (DIVERGENCES).
//!
//! The same queue keeps a single-select exclusive when a consumer adds
//! `selected` to one of its options (HTML: setting an option's
//! selectedness to true unsets the others'): the other options lose
//! theirs at the flush.
//!
//! Selectedness is stored in the `selected` attribute (the live state,
//! FORM-DEFAULTS-1); an option the algorithm flips has its
//! `defaultSelected` captured first, so the algorithm's pick is never
//! mistaken for the author's default.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_core::{Mutation, MutationObserver, NodeId};

use super::model::{display_size, enclosing_select, is_multi, option_disabled, options};
use crate::{TuiDom, TuiExt};

/// Run the selectedness setting algorithm on `select`. No events fire.
pub(crate) fn run(dom: &mut TuiDom, select: NodeId) {
    if is_multi(dom, select) {
        return;
    }
    let all = options(dom, select);
    let selected: Vec<NodeId> = all
        .iter()
        .copied()
        .filter(|&o| dom.node(o).has_attribute("selected"))
        .collect();
    match selected.split_last() {
        None => {
            if display_size(dom, select) != 1 {
                return;
            }
            if let Some(first) = all.into_iter().find(|&o| !option_disabled(dom, o)) {
                note_default(dom, first);
                let _ = dom.set_attribute(first, "selected", "");
            }
        }
        Some((_, earlier)) => {
            for &opt in earlier {
                note_default(dom, opt);
                let _ = dom.remove_attribute(opt, "selected");
            }
        }
    }
}

/// Run the algorithm on every `<select>` under the document root.
pub fn seed_all(dom: &mut TuiDom) {
    let mut selects = Vec::new();
    collect_selects(dom, dom.root(), &mut selects);
    for select in selects {
        run(dom, select);
    }
}

fn collect_selects(dom: &TuiDom, id: NodeId, out: &mut Vec<NodeId>) {
    for child in dom.node(id).child_nodes() {
        if child.tag_name() == Some("select") {
            out.push(child.id());
        }
        collect_selects(dom, child.id(), out);
    }
}

/// Capture `opt`'s current `selected` attribute as its `defaultSelected`
/// if nothing was captured yet.
fn note_default(dom: &mut TuiDom, opt: NodeId) {
    let selected = dom.node(opt).has_attribute("selected");
    if let Some(ext) = dom.node_mut(opt).ext_mut()
        && ext.default_selected.is_none()
    {
        ext.default_selected = Some(selected);
    }
}

/// Work queued by the observer since the last flush, in record order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Queued {
    /// The select's list of options changed: run the algorithm.
    OptionsChanged(NodeId),
    /// `option` of `select` gained the `selected` attribute: in a
    /// single-select it becomes the only selected option (HTML §4.10.7:
    /// setting one option's selectedness to true unsets the others').
    Picked { select: NodeId, option: NodeId },
}

/// The observer's queue, shared with the flush. `flushing` mutes the
/// observer while [`Selectedness::flush`] writes: those writes settle
/// the selects they touch, so queueing them would only replay stale
/// picks over later changes.
#[derive(Default)]
struct Shared {
    queue: RefCell<Vec<Queued>>,
    flushing: std::cell::Cell<bool>,
}

type Pending = Rc<Shared>;

/// Runs the algorithm after option insertions and removals, and keeps a
/// single-select's selection exclusive after a consumer marks an option
/// `selected`: an installed mutation observer queues the work, and
/// [`Selectedness::flush`] does it.
pub(crate) struct Selectedness {
    pending: Pending,
}

impl Selectedness {
    /// Register the observer on `dom`.
    pub(crate) fn install(dom: &mut TuiDom) -> Self {
        let pending = Pending::default();
        dom.add_mutation_observer(Box::new(OptionListObserver {
            pending: pending.clone(),
        }));
        Self { pending }
    }

    /// Do the queued work, in the order it was recorded, on selects and
    /// options still in the arena. The writes it makes are consistent
    /// by construction; what they queue in turn is a no-op next time.
    pub(crate) fn flush(&self, dom: &mut TuiDom) {
        let mut queued = std::mem::take(&mut *self.pending.queue.borrow_mut());
        if queued.is_empty() {
            return;
        }
        let _muted = Muted::new(&self.pending.flushing);
        queued.dedup();
        // Of several options marked in one batch, the last one marked
        // wins: only a select's latest `Picked` is applied.
        let mut last_pick = std::collections::HashMap::new();
        for (i, item) in queued.iter().enumerate() {
            if let Queued::Picked { select, .. } = item {
                last_pick.insert(*select, i);
            }
        }
        let is_select = |dom: &TuiDom, id: NodeId| {
            dom.contains(id) && dom.node(id).tag_name() == Some("select")
        };
        for (i, item) in queued.into_iter().enumerate() {
            match item {
                Queued::OptionsChanged(select) if is_select(dom, select) => run(dom, select),
                Queued::Picked { select, option }
                    if last_pick.get(&select) == Some(&i)
                        && is_select(dom, select)
                        && dom.contains(option) =>
                {
                    make_exclusive(dom, select, option);
                }
                _ => {}
            }
        }
    }
}

/// Mutes the observer for its lifetime — unmuted on drop, so a panic in
/// the flush does not leave the queue deaf.
struct Muted<'a>(&'a std::cell::Cell<bool>);

impl<'a> Muted<'a> {
    fn new(flag: &'a std::cell::Cell<bool>) -> Self {
        flag.set(true);
        Self(flag)
    }
}

impl Drop for Muted<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// `option` is `select`'s only selected option, when it is still one
/// of its selected options and the select is a single-select.
fn make_exclusive(dom: &mut TuiDom, select: NodeId, option: NodeId) {
    if is_multi(dom, select) || !dom.node(option).has_attribute("selected") {
        return;
    }
    let all = options(dom, select);
    if !all.contains(&option) {
        return;
    }
    for other in all {
        if other != option && dom.node(other).has_attribute("selected") {
            note_default(dom, other);
            let _ = dom.remove_attribute(other, "selected");
        }
    }
}

struct OptionListObserver {
    pending: Pending,
}

impl MutationObserver<TuiExt> for OptionListObserver {
    fn observe(&mut self, dom: &mut TuiDom, record: &Mutation) {
        if self.pending.flushing.get() {
            return;
        }
        match record {
            Mutation::ChildListChanged {
                parent,
                added,
                removed,
            } => {
                let touches_options = added
                    .iter()
                    .chain(removed)
                    .any(|&n| matches!(dom.node(n).tag_name(), Some("option" | "optgroup")));
                // The list of options holds the select's option children
                // and its optgroups' option children.
                if touches_options && let Some(select) = enclosing_select(dom, *parent) {
                    self.pending
                        .queue
                        .borrow_mut()
                        .push(Queued::OptionsChanged(select));
                }
            }
            Mutation::AttributeChanged {
                id,
                name,
                old: None,
                new: Some(_),
            } if name == "selected" && dom.node(*id).tag_name() == Some("option") => {
                if let Some(select) = enclosing_select(dom, *id) {
                    self.pending.queue.borrow_mut().push(Queued::Picked {
                        select,
                        option: *id,
                    });
                }
            }
            _ => {}
        }
    }
}
