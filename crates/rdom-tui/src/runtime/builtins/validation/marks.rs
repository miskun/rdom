//! Keeps `:valid` / `:invalid` current in the App's incremental cascade.
//!
//! The selector engine asks the validity hook at match time, so a query
//! is always current. The cascade, though, only re-matches the subtrees
//! the `DirtyTracker` saw mutate, and validity can change without a
//! cascade-dirtying mutation: a textarea's text edit (character data),
//! `set_custom_validity` (no mutation at all), checking one radio of a
//! required group (its siblings), any control of a form or fieldset
//! (the ancestor's `:invalid`). Before each frame's cascade,
//! [`ValidityMarks::flush`] recomputes the validity of every candidate,
//! form and fieldset and marks the ones that flipped since the last
//! frame dirty — only when some stylesheet uses `:valid` / `:invalid`,
//! which the UA sheet does not.

use std::collections::HashMap;

use rdom_core::selectors::{ComplexSelector, PseudoClass, SimpleSelector};
use rdom_core::{NodeId, NodeType};

use crate::TuiDom;
use crate::style::{DirtyTracker, Stylesheet};

/// The `:valid` / `:invalid` state of every element that has one, as of
/// the last frame's cascade.
#[derive(Debug, Default)]
pub(crate) struct ValidityMarks {
    last: HashMap<NodeId, bool>,
    /// `last` holds the state a cascade saw. Until then (first frame,
    /// or no validity rule before) there is nothing to compare with.
    primed: bool,
}

impl ValidityMarks {
    /// Mark every element whose `:valid` / `:invalid` state changed
    /// since the previous flush style-dirty. Run before the frame takes
    /// its dirty roots.
    pub(crate) fn flush<'s>(
        &mut self,
        dom: &mut TuiDom,
        tracker: &DirtyTracker,
        sheets: impl IntoIterator<Item = &'s Stylesheet>,
    ) {
        if !sheets.into_iter().any(uses_validity) {
            self.last.clear();
            self.primed = false;
            return;
        }
        let now = current(dom);
        if self.primed {
            let changed: Vec<NodeId> = now
                .iter()
                .filter(|(id, v)| self.last.get(id) != Some(v))
                .map(|(&id, _)| id)
                .collect();
            for id in changed {
                tracker.mark_dirty(dom, id);
            }
        }
        self.last = now;
        self.primed = true;
    }
}

/// `Dom::constraint_validity` of every element that has one.
fn current(dom: &TuiDom) -> HashMap<NodeId, bool> {
    let mut out = HashMap::new();
    let mut stack = vec![dom.root()];
    while let Some(id) = stack.pop() {
        let node = dom.node(id);
        if node.node_type() == NodeType::Element
            && let Some(valid) = dom.constraint_validity(id)
        {
            out.insert(id, valid);
        }
        stack.extend(node.child_nodes().map(|c| c.id()));
    }
    out
}

/// Whether any rule of `sheet` mentions `:valid` or `:invalid`,
/// anywhere in its selector (inside `:not()` / `:where()` too).
pub(super) fn uses_validity(sheet: &Stylesheet) -> bool {
    sheet
        .rules()
        .iter()
        .any(|r| r.selector.0.iter().any(complex_uses_validity))
}

fn complex_uses_validity(c: &ComplexSelector) -> bool {
    std::iter::once(&c.subject)
        .chain(c.ancestors.iter().map(|(_, compound)| compound))
        .flat_map(|compound| &compound.simples)
        .any(|s| match s {
            SimpleSelector::Pseudo(p) => matches!(p, PseudoClass::Valid | PseudoClass::Invalid),
            SimpleSelector::Not(list) | SimpleSelector::Where(list) => {
                list.0.iter().any(complex_uses_validity)
            }
            _ => false,
        })
}
