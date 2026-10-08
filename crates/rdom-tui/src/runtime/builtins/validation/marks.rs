//! Keeps the form-state pseudo-classes current in the App's incremental
//! cascade: `:valid` / `:invalid`, a radio group's `:indeterminate`,
//! `:default`, and `:user-valid` / `:user-invalid`.
//!
//! The selector engine asks the validity hook at match time, so a query
//! is always current. The cascade, though, only re-matches the subtrees
//! the `DirtyTracker` saw mutate, and these states can change without a
//! cascade-dirtying mutation on the element: a textarea's text edit
//! (character data), `set_custom_validity` (no mutation at all), checking
//! one radio of a group (its siblings' `:indeterminate`, and their
//! `:invalid` when the group is required), any control of a form or
//! fieldset (the ancestor's `:invalid`), a submit button inserted before
//! a form's default button (the old one's `:default`), a new
//! `defaultChecked` (no mutation). Before each frame's cascade,
//! [`FormStateMarks::flush`] recomputes these states for every element
//! that has one and marks the ones that flipped since the last frame —
//! through the tracker's state path, so a `:has()` anchor reading them
//! is restyled too — and only for the states some stylesheet reads:
//! validity when a sheet uses `:valid` / `:invalid`, which the UA sheet
//! does not; radio `:indeterminate` when an author sheet uses
//! `:indeterminate` (the UA's own rule is for checkboxes, whose flag is
//! an attribute — pinned by `the_uas_indeterminate_rules_are_checkbox_only`);
//! `:default` when a sheet uses it; a control's user validity (a flag
//! the form builtins set without a mutation) with the validity it
//! qualifies, when a sheet uses `:user-valid` / `:user-invalid`.
//!
//! Cost (`P7G-IDLE-WALKS-1`): the App flushes only after code that can
//! change a state ran (an event, a timer, an injected closure,
//! `dom_mut()`), so an idle tick pays nothing. A flush is one tree
//! walk: each candidate's validity is computed once and radio groups
//! once per group ([`RadioGroups`]), and form / fieldset verdicts are
//! derived from the invalid candidates — a form is invalid when a
//! candidate it owns is, a fieldset when a descendant candidate is —
//! rather than by a walk per form or fieldset (the same verdicts
//! `Dom::constraint_validity` gives, pinned by a test). Which states the
//! sheets read is cached until the stylesheets change
//! ([`FormStateMarks::sheets_changed`]).

use std::collections::HashMap;

use rdom_core::selectors::{PseudoClass, SimpleSelector};
use rdom_core::{InputTypeState, NodeId, NodeType};

use super::states::{RadioGroups, compute_in};
use crate::TuiDom;
use crate::style::{DirtyTracker, Stylesheet};

/// One element's tracked states, as bits.
type States = u8;
/// The element has a `:valid` / `:invalid` state.
const HAS_VALIDITY: States = 1;
/// … and it is `:valid`.
const VALID: States = 1 << 1;
/// A radio whose group has no checked member (`:indeterminate`).
const INDETERMINATE: States = 1 << 2;
/// `:default` matches it.
const DEFAULT: States = 1 << 3;
/// Its user validity is set (`:user-valid` / `:user-invalid`).
const USER: States = 1 << 4;

/// Which tracked states the stylesheets read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Reads {
    validity: bool,
    indeterminate: bool,
    default: bool,
    user: bool,
}

impl Reads {
    fn any(self) -> bool {
        self.validity || self.indeterminate || self.default || self.user
    }
}

/// The tracked states of every element that has one, as of the last
/// frame's cascade.
#[derive(Debug, Default)]
pub(crate) struct FormStateMarks {
    last: HashMap<NodeId, States>,
    /// The map the next flush fills, kept to reuse its allocation.
    scratch: HashMap<NodeId, States>,
    /// `last` holds the states a cascade saw. Until then (first frame,
    /// or no tracked state read before) there is nothing to compare with.
    primed: bool,
    /// What the stylesheets read; `None` until computed, and again after
    /// [`Self::sheets_changed`].
    reads: Option<Reads>,
    radio_groups: RadioGroups,
}

impl FormStateMarks {
    /// The stylesheets changed: recompute what they read on the next
    /// flush.
    pub(crate) fn sheets_changed(&mut self) {
        self.reads = None;
    }

    /// Mark every element whose tracked state changed since the
    /// previous flush style-dirty. Run before the frame takes its dirty
    /// roots. Returns whether it walked the tree.
    pub(crate) fn flush<'s>(
        &mut self,
        dom: &mut TuiDom,
        tracker: &DirtyTracker,
        sheets: impl IntoIterator<Item = &'s Stylesheet>,
    ) -> bool {
        let reads = *self.reads.get_or_insert_with(|| reads_of(sheets));
        if !reads.any() {
            self.last.clear();
            self.primed = false;
            return false;
        }
        let mut now = std::mem::take(&mut self.scratch);
        current(dom, reads, &mut now, &mut self.radio_groups);
        if self.primed {
            let changed: Vec<NodeId> = self
                .last
                .iter()
                .filter(|(id, v)| now.get(id) != Some(v))
                .map(|(&id, _)| id)
                .chain(now.keys().filter(|id| !self.last.contains_key(id)).copied())
                .filter(|&id| dom.contains(id) && dom.node(id).is_connected())
                .collect();
            for id in changed {
                tracker.mark_state_changed(dom, id);
            }
        }
        self.scratch = std::mem::replace(&mut self.last, now);
        self.scratch.clear();
        self.primed = true;
        true
    }
}

/// What `sheets` read of the tracked states.
fn reads_of<'s>(sheets: impl IntoIterator<Item = &'s Stylesheet>) -> Reads {
    let mut reads = Reads::default();
    for sheet in sheets {
        reads.validity |= uses_validity(sheet);
        reads.indeterminate |= uses_radio_indeterminate(sheet);
        reads.default |= uses_pseudo(sheet, PseudoClass::Default);
        let user = uses_pseudo(sheet, PseudoClass::UserValid)
            || uses_pseudo(sheet, PseudoClass::UserInvalid);
        reads.user |= user;
        // `:user-valid` / `:user-invalid` read the validity too.
        reads.validity |= user;
    }
    reads
}

/// Fill `out` with the tracked states of every connected element that
/// has one, in one walk (see the module doc).
fn current(
    dom: &TuiDom,
    reads: Reads,
    out: &mut HashMap<NodeId, States>,
    groups: &mut RadioGroups,
) {
    out.clear();
    groups.clear();
    let mut invalid = Vec::new();
    // One pass's caches: each form's default button is found once.
    let mut caches = rdom_core::SelectorCaches::new();
    let mut stack = vec![dom.root()];
    while let Some(id) = stack.pop() {
        let node = dom.node(id);
        stack.extend(node.child_nodes().map(|c| c.id()));
        if node.node_type() != NodeType::Element {
            continue;
        }
        let mut states = 0;
        if reads.validity {
            match node.tag_name() {
                Some("form" | "fieldset") => states |= HAS_VALIDITY | VALID,
                _ if dom.will_validate(id) => {
                    states |= HAS_VALIDITY;
                    if compute_in(dom, id, groups).valid() {
                        states |= VALID;
                    } else {
                        invalid.push(id);
                    }
                }
                _ => {}
            }
        }
        if reads.indeterminate
            && dom.input_type_state(id) == Some(InputTypeState::Radio)
            && groups.unchecked(dom, id)
        {
            states |= INDETERMINATE;
        }
        if reads.user && crate::runtime::builtins::form_state::user_validity(dom, id) {
            states |= USER;
        }
        if reads.default
            && matches!(node.tag_name(), Some("button" | "input" | "option"))
            && dom.is_default_with(id, &mut caches)
        {
            states |= DEFAULT;
        }
        if states != 0 {
            out.insert(id, states);
        }
    }
    for c in invalid {
        if let Some(form) = dom.form_owner(c)
            && let Some(v) = out.get_mut(&form)
        {
            *v &= !VALID;
        }
        let mut up = dom.node(c).parent_node().map(|p| p.id());
        while let Some(a) = up {
            let node = dom.node(a);
            if node.tag_name() == Some("fieldset")
                && let Some(v) = out.get_mut(&a)
            {
                *v &= !VALID;
            }
            up = node.parent_node().map(|p| p.id());
        }
    }
}

/// Whether any selector `sheet` matches with mentions `:valid` or
/// `:invalid` — in a rule's selector or an `@scope`'s start / end,
/// inside `:not()` / `:is()` / `:where()` too (`style::selector_walk`).
pub(super) fn uses_validity(sheet: &Stylesheet) -> bool {
    use crate::style::selector_walk::{any_simple, sheet_selectors};
    sheet_selectors(sheet).any(|c| {
        any_simple(c, &|s| {
            matches!(
                s,
                SimpleSelector::Pseudo(PseudoClass::Valid | PseudoClass::Invalid)
            )
        })
    })
}

/// Whether any selector `sheet` matches with mentions `pseudo`.
fn uses_pseudo(sheet: &Stylesheet, pseudo: PseudoClass) -> bool {
    use crate::style::selector_walk::{any_simple, sheet_selectors};
    sheet_selectors(sheet).any(|c| any_simple(c, &|s| *s == SimpleSelector::Pseudo(pseudo)))
}

/// Whether an author selector of `sheet` mentions `:indeterminate`,
/// which a radio's group decides. The UA's rules are left out: they use
/// it on checkboxes only, whose flag is an attribute the tracker sees.
fn uses_radio_indeterminate(sheet: &Stylesheet) -> bool {
    use crate::style::selector_walk::{any_simple, author_selectors};
    author_selectors(sheet).any(|c| {
        any_simple(c, &|s| {
            matches!(s, SimpleSelector::Pseudo(PseudoClass::Indeterminate))
        })
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
        let reads = Reads {
            validity: true,
            ..Reads::default()
        };
        current(&dom, reads, &mut out, &mut RadioGroups::default());
        let mut all = Vec::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            stack.extend(dom.node(id).child_nodes().map(|c| c.id()));
            all.push(id);
        }
        let mut checked = 0;
        for id in all {
            let verdict = out.get(&id).map(|&v| v & VALID != 0);
            assert_eq!(verdict, dom.constraint_validity(id), "{id:?}");
            checked += usize::from(out.contains_key(&id));
        }
        assert!(
            checked > 10,
            "the fixture exercises the verdicts: {checked}"
        );
        let valid = |id| out.get(&id).map(|&v| v & VALID != 0);
        assert_eq!(valid(f1), Some(false));
        assert_eq!(valid(f2), Some(false));
        assert_eq!(valid(quiet), Some(true));
        assert_eq!(valid(inner), Some(false));
    }

    /// The module doc's claim: every UA rule that reads `:indeterminate`
    /// is a checkbox rule, so radio groups need no tracking for the UA.
    #[test]
    fn the_uas_indeterminate_rules_are_checkbox_only() {
        use crate::style::RuleOrigin;
        let sheet = Stylesheet::new();
        let ua: Vec<_> = sheet
            .rules()
            .iter()
            .filter(|r| r.origin == RuleOrigin::UserAgent)
            .filter(|r| r.source_text.contains(":indeterminate"))
            .collect();
        assert!(!ua.is_empty(), "the UA has an `:indeterminate` rule");
        for rule in ua {
            assert!(
                rule.source_text
                    .starts_with("input[type=checkbox]:indeterminate"),
                "{}",
                rule.source_text
            );
        }
        assert!(!uses_radio_indeterminate(&sheet));
    }
}
