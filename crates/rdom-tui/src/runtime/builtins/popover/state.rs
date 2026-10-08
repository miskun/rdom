//! The popover state HTML keeps per element (opened in popover mode,
//! popover trigger, previously focused element, popover hiding) and per
//! document (showing popover, hiding popover nesting count, hint stack
//! parent), as document data, with the readers the algorithms share.

use std::collections::{HashMap, HashSet};

use rdom_core::NodeId;

use super::{PopoverState, is_showing};
use crate::TuiDom;

/// The popover state of one document.
#[derive(Debug, Default)]
pub(crate) struct Popovers {
    /// The showing auto and hint popovers in the order they were shown
    /// (their top-layer order), each with its "opened in popover mode" —
    /// kept past a removal from the top layer by the tree's removing
    /// steps, so `algorithms::flush_removed` knows where it stood.
    pub(super) opened: Vec<(NodeId, PopoverState)>,
    /// "Popover trigger" of each showing popover that has one.
    pub(super) trigger: HashMap<NodeId, NodeId>,
    /// "Previously focused element", restored on hide.
    pub(super) previously_focused: HashMap<NodeId, NodeId>,
    /// "Popover hiding".
    pub(super) hiding: HashSet<NodeId>,
    /// The document's "hiding popover nesting count".
    pub(super) hiding_nesting: u32,
    /// The document's "showing popover".
    pub(super) showing_popover: bool,
    /// The document's "hint stack parent".
    pub(super) hint_stack_parent: Option<NodeId>,
}

pub(super) fn data(dom: &mut TuiDom) -> &mut Popovers {
    if dom.document_data::<Popovers>().is_none() {
        dom.set_document_data(Popovers::default());
    }
    dom.document_data_mut::<Popovers>()
        .expect("the popover data was just set")
}

/// `id`'s "opened in popover mode", as recorded (showing or just
/// removed).
pub(super) fn recorded_mode(dom: &TuiDom, id: NodeId) -> Option<PopoverState> {
    let d = dom.document_data::<Popovers>()?;
    d.opened.iter().find(|(p, _)| *p == id).map(|&(_, m)| m)
}

/// The mode a showing popover was opened in: `auto` or `hint`, `None`
/// for a manual popover or one not showing.
pub(super) fn opened_mode(dom: &TuiDom, id: NodeId) -> Option<PopoverState> {
    recorded_mode(dom, id).filter(|_| is_showing(dom, id))
}

/// The "popover trigger" of a showing popover.
pub(super) fn trigger_of(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    if !is_showing(dom, id) {
        return None;
    }
    dom.document_data::<Popovers>()?.trigger.get(&id).copied()
}

/// The document's "hint stack parent".
pub(super) fn hint_stack_parent(dom: &TuiDom) -> Option<NodeId> {
    dom.document_data::<Popovers>()?.hint_stack_parent
}

/// The document's showing auto (or hint) popover list, bottom to top.
pub(super) fn showing_list(dom: &TuiDom, mode: PopoverState) -> Vec<NodeId> {
    recorded_list(dom, mode)
        .into_iter()
        .filter(|&id| is_showing(dom, id))
        .collect()
}

/// The recorded `mode` list, a popover the removing steps took out of the
/// top layer included.
pub(super) fn recorded_list(dom: &TuiDom, mode: PopoverState) -> Vec<NodeId> {
    dom.document_data::<Popovers>().map_or_else(Vec::new, |d| {
        d.opened
            .iter()
            .filter(|&&(_, m)| m == mode)
            .map(|&(id, _)| id)
            .collect()
    })
}

/// "Hide popover" steps 14–17 for `id`: no trigger, no opened mode, and
/// the hint stack parent cleared when it was `id` or no hint shows.
pub(super) fn forget(dom: &mut TuiDom, id: NodeId) {
    let d = data(dom);
    d.trigger.remove(&id);
    d.opened.retain(|&(p, _)| p != id);
    let no_hints = !d.opened.iter().any(|&(_, m)| m == PopoverState::Hint);
    if d.hint_stack_parent == Some(id) || no_hints {
        d.hint_stack_parent = None;
    }
}

/// The recorded list `mode` — the showing popovers, plus one the removing
/// steps took out — for the stack walks.
pub(super) fn walk_list(dom: &TuiDom, mode: PopoverState) -> Vec<NodeId> {
    recorded_list(dom, mode)
}

/// How many popovers the document keeps per-popover state for (tests:
/// the state must not outlive the popover).
#[cfg(test)]
pub(super) fn tracked(dom: &TuiDom) -> usize {
    let Some(d) = dom.document_data::<Popovers>() else {
        return 0;
    };
    let mut ids: HashSet<NodeId> = d.opened.iter().map(|&(id, _)| id).collect();
    ids.extend(d.trigger.keys().copied());
    ids.extend(d.previously_focused.keys().copied());
    ids.len()
}
