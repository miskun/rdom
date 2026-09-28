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
//!
//! Cost (`P7G-IDLE-WALKS-1`): the App flushes only after code that can
//! change validity ran (an event, a timer, an injected closure,
//! `dom_mut()`), so an idle tick pays nothing. A flush is one tree
//! walk: each candidate's validity is computed once (radio groups once
//! per group, [`RadioGroups`]), and form / fieldset verdicts are derived
//! from the invalid candidates — a form is invalid when a candidate it
//! owns is, a fieldset when a descendant candidate is — rather than by
//! a walk per form or fieldset (the same verdicts
//! `Dom::constraint_validity` gives, pinned by a test). Whether a sheet
//! uses the pseudo-classes is cached until the stylesheets change
//! ([`ValidityMarks::sheets_changed`]).

use std::collections::HashMap;

use rdom_core::selectors::{ComplexSelector, PseudoClass, SimpleSelector};
use rdom_core::{NodeId, NodeType};

use super::states::{RadioGroups, compute_in};
use crate::TuiDom;
use crate::style::{DirtyTracker, Stylesheet};

/// The `:valid` / `:invalid` state of every element that has one, as of
/// the last frame's cascade.
#[derive(Debug, Default)]
pub(crate) struct ValidityMarks {
    last: HashMap<NodeId, bool>,
    /// The map the next flush fills, kept to reuse its allocation.
    scratch: HashMap<NodeId, bool>,
    /// `last` holds the state a cascade saw. Until then (first frame,
    /// or no validity rule before) there is nothing to compare with.
    primed: bool,
    /// Whether some stylesheet uses `:valid` / `:invalid`; `None` until
    /// computed, and again after [`Self::sheets_changed`].
    uses_validity: Option<bool>,
    radio_groups: RadioGroups,
}

impl ValidityMarks {
    /// The stylesheets changed: recompute whether they use `:valid` /
    /// `:invalid` on the next flush.
    pub(crate) fn sheets_changed(&mut self) {
        self.uses_validity = None;
    }

    /// Mark every element whose `:valid` / `:invalid` state changed
    /// since the previous flush style-dirty. Run before the frame takes
    /// its dirty roots. Returns whether it walked the tree.
    pub(crate) fn flush<'s>(
        &mut self,
        dom: &mut TuiDom,
        tracker: &DirtyTracker,
        sheets: impl IntoIterator<Item = &'s Stylesheet>,
    ) -> bool {
        let uses = *self
            .uses_validity
            .get_or_insert_with(|| sheets.into_iter().any(uses_validity));
        if !uses {
            self.last.clear();
            self.primed = false;
            return false;
        }
        let mut now = std::mem::take(&mut self.scratch);
        current(dom, &mut now, &mut self.radio_groups);
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
        self.scratch = std::mem::replace(&mut self.last, now);
        self.scratch.clear();
        self.primed = true;
        true
    }
}

/// Fill `out` with `Dom::constraint_validity` of every connected element
/// that has one, in one walk (see the module doc).
fn current(dom: &TuiDom, out: &mut HashMap<NodeId, bool>, groups: &mut RadioGroups) {
    out.clear();
    groups.clear();
    let mut invalid = Vec::new();
    let mut stack = vec![dom.root()];
    while let Some(id) = stack.pop() {
        let node = dom.node(id);
        stack.extend(node.child_nodes().map(|c| c.id()));
        if node.node_type() != NodeType::Element {
            continue;
        }
        match node.tag_name() {
            Some("form" | "fieldset") => {
                out.insert(id, true);
            }
            _ if dom.will_validate(id) => {
                let valid = compute_in(dom, id, groups).valid();
                out.insert(id, valid);
                if !valid {
                    invalid.push(id);
                }
            }
            _ => {}
        }
    }
    for c in invalid {
        if let Some(form) = dom.form_owner(c)
            && let Some(v) = out.get_mut(&form)
        {
            *v = false;
        }
        let mut up = dom.node(c).parent_node().map(|p| p.id());
        while let Some(a) = up {
            let node = dom.node(a);
            if node.tag_name() == Some("fieldset")
                && let Some(v) = out.get_mut(&a)
            {
                *v = false;
            }
            up = node.parent_node().map(|p| p.id());
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn el(dom: &mut TuiDom, parent: NodeId, tag: &str, attrs: &[(&str, &str)]) -> NodeId {
        let e = dom.create_element(tag);
        for (k, v) in attrs {
            dom.set_attribute(e, k, v).unwrap();
        }
        dom.append_child(parent, e).unwrap();
        crate::runtime::builtins::input::ensure_seeded(dom, e);
        e
    }

    /// The one-walk verdicts equal `Dom::constraint_validity` for every
    /// element: forms owning controls through `form=`, nested fieldsets,
    /// radio groups split by form owner, barred controls.
    #[test]
    fn one_walk_verdicts_match_constraint_validity() {
        let mut dom: TuiDom = TuiDom::new();
        super::super::install(&mut dom);
        let root = dom.root();
        let f1 = el(&mut dom, root, "form", &[("id", "f1")]);
        let f2 = el(&mut dom, root, "form", &[("id", "f2")]);
        let outer = el(&mut dom, f1, "fieldset", &[]);
        let inner = el(&mut dom, outer, "fieldset", &[]);
        el(&mut dom, inner, "input", &[("required", "")]);
        el(
            &mut dom,
            inner,
            "input",
            &[("required", ""), ("form", "f2")],
        );
        el(&mut dom, outer, "input", &[("value", "ok")]);
        let quiet = el(&mut dom, f1, "fieldset", &[]);
        el(
            &mut dom,
            quiet,
            "input",
            &[("required", ""), ("value", "x")],
        );
        el(
            &mut dom,
            quiet,
            "input",
            &[("required", ""), ("disabled", "")],
        );
        for form in ["f1", "f2"] {
            el(
                &mut dom,
                root,
                "input",
                &[("type", "radio"), ("name", "g"), ("form", form)],
            );
        }
        el(
            &mut dom,
            f1,
            "input",
            &[("type", "radio"), ("name", "g"), ("required", "")],
        );
        el(
            &mut dom,
            f2,
            "input",
            &[("type", "radio"), ("name", "g"), ("checked", "")],
        );
        el(
            &mut dom,
            root,
            "input",
            &[("type", "email"), ("value", "nope")],
        );
        el(&mut dom, root, "select", &[("required", "")]);
        el(&mut dom, root, "textarea", &[]);

        let mut out = HashMap::new();
        current(&dom, &mut out, &mut RadioGroups::default());
        let mut all = Vec::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            stack.extend(dom.node(id).child_nodes().map(|c| c.id()));
            all.push(id);
        }
        let mut checked = 0;
        for id in all {
            assert_eq!(out.get(&id).copied(), dom.constraint_validity(id), "{id:?}");
            checked += usize::from(out.contains_key(&id));
        }
        assert!(
            checked > 10,
            "the fixture exercises the verdicts: {checked}"
        );
        assert_eq!(out.get(&f1), Some(&false));
        assert_eq!(out.get(&f2), Some(&false));
        assert_eq!(out.get(&quiet), Some(&true));
        assert_eq!(out.get(&inner), Some(&false));
    }
}
