//! Which changes can change a `:has()` anchor's match (Selectors 4 §4.5),
//! and how the dirty tracker finds the anchors to restyle — targeted
//! invalidation in the spirit of Blink's.
//!
//! An anchor matches `:has(…)` by what its subtree (`:has(.x)`,
//! `:has(> .x)`) or its later siblings (`:has(+ .x)`, `:has(~ .x)`) hold,
//! so a change *elsewhere* — a class added deep inside, a child appended,
//! a checkbox checked, the pointer moving — can restyle it. Two halves
//! keep that cheap:
//!
//! - **What can matter** ([`HasTriggers::of_sheets`], once per sheet
//!   set): the simple selectors inside every `:has()` argument — the
//!   attribute names they test (`class` for `.x`, `id` for `#x`, `[name]`),
//!   whether a pseudo-class reads state (`:hover`, `:focus`, `:empty`) or
//!   attributes (`:checked`, `:disabled`, …), and whether a relative
//!   selector reaches siblings (`+` / `~`). A change none of them reads
//!   costs nothing. A child-list change always counts (any element can
//!   arrive with what the arguments look for).
//! - **Which elements are anchors**: the cascade flags every element a
//!   `:has()` was evaluated for (`TuiExt::has_anchor`, from
//!   `SelectorCaches::has_anchors`). The tracker walks from the changed
//!   element up its ancestors — and, when a relative selector reaches
//!   siblings, along each one's earlier siblings — and restyles the
//!   flagged ones only.
//!
//! **Why earlier siblings are enough** (C11G-HAS-IS-SIBLING). Every
//! combinator nested in an argument leads up (to an ancestor) or back (to
//! an earlier sibling) from its subject, so the elements an anchor's match
//! reads are its subtree, its later siblings' subtrees — which this walk
//! reaches — and what up and back steps from those lead to. An up step
//! lands on an ancestor of the anchor (whose own change restyles its
//! subtree, the anchor in it) or inside the anchor's subtree. A back step
//! can land *before* the anchor: in `.a:has(+ :is(.x ~ *))` the `.x` is any
//! earlier sibling of the anchor's next sibling. Such an element matches a
//! compound left of a sibling combinator, which `SiblingTriggers` records
//! inside `:has()` arguments too (`selector_walk::arguments`), so its
//! change marks every one of its siblings — the anchor, or the anchor's
//! ancestor, among them. `:nth-child(… of S)` reads its siblings the same
//! way and is recorded the same way.
//!
//! Without a `:has()` rule the triggers are empty and the tracker does
//! no walk at all; without a flagged element (no cascade met a `:has()`)
//! neither (`doc_flags::has_has_anchors`).

use rdom_core::selectors::{Combinator, ComplexSelector, SimpleSelector};

use crate::style::Stylesheet;
use crate::style::selector_walk::{any_complex, compounds, sheet_selectors};
use crate::style::sibling_triggers::{Cause, reads_attributes};

/// What changes can reach a `:has()` anchor's match under a set of
/// stylesheets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct HasTriggers {
    /// The sheets are unknown: every change may (the tracker's default).
    all: bool,
    /// Some selector has a `:has()`.
    any: bool,
    /// A `:has()` argument has a pseudo-class.
    state: bool,
    /// One of them reads attributes.
    attribute_state: bool,
    /// Attribute names the arguments test, ASCII-lowercase.
    attributes: Vec<String>,
    /// How many earlier siblings away an anchor can be from a changed
    /// element or one of its ancestors: 0 without a sibling relation, the
    /// number of `+` steps leading a relative selector, `usize::MAX` once
    /// a `~` (or a sibling step nested in `:is()` / `:not()`) can reach
    /// any of them.
    sibling_reach: usize,
}

impl HasTriggers {
    /// Every change may reach an anchor (before the sheets are known).
    pub(crate) fn all() -> Self {
        Self {
            all: true,
            any: true,
            sibling_reach: usize::MAX,
            ..Self::default()
        }
    }

    /// No `:has()`: no change reaches one.
    #[cfg(test)]
    pub(crate) fn none() -> Self {
        Self::default()
    }

    /// What the `:has()` arguments of `sheets` read (module doc).
    pub(crate) fn of_sheets<'a>(sheets: impl IntoIterator<Item = &'a Stylesheet>) -> Self {
        let mut t = Self::default();
        for sheet in sheets {
            for complex in sheet_selectors(sheet) {
                let unknown = any_complex(complex, &mut |c| {
                    for simple in compounds(c).flat_map(|c| &c.simples) {
                        if let SimpleSelector::Has(relative) = simple {
                            t.any = true;
                            for rel in relative {
                                t.sibling_reach = t.sibling_reach.max(leading_reach(rel));
                                t.add_reads(&rel.selector, false);
                            }
                        }
                    }
                    false
                });
                if unknown {
                    return Self::all();
                }
            }
        }
        t
    }

    /// Whether the sheets have a `:has()` at all.
    pub(crate) fn any(&self) -> bool {
        self.any
    }

    /// How many earlier siblings of a changed element (or of an
    /// ancestor) can be an anchor: the walk's bound along each sibling
    /// list (`usize::MAX`: all of them).
    pub(crate) fn sibling_reach(&self) -> usize {
        self.sibling_reach
    }

    /// Whether a change of kind `cause` on an element can change some
    /// anchor's match.
    pub(crate) fn fires(&self, cause: Cause<'_>) -> bool {
        match cause {
            // An anchor's own `:has()` state is no `:has()` argument's
            // input (`:has()` does not nest) — whatever the sheets, so
            // restyling an anchor never starts another walk.
            Cause::Has => false,
            Cause::State => self.all || (self.any && self.state),
            Cause::Attribute(name) => {
                self.all
                    || (self.any
                        && (self.attribute_state
                            || self.attributes.iter().any(|a| a.eq_ignore_ascii_case(name))))
            }
        }
    }

    /// Record what `complex` (a relative selector, or — `nested` — a
    /// selector nested in one) reads, every compound of it: they match
    /// elements around the anchor, any of which can change. A sibling
    /// step nested in `:is()` / `:not()` relates elements the anchor's own
    /// relation does not bound: every earlier sibling is walked.
    fn add_reads(&mut self, complex: &ComplexSelector, nested: bool) {
        if nested
            && complex
                .ancestors
                .iter()
                .any(|(c, _)| matches!(c, Combinator::AdjacentSibling | Combinator::GeneralSibling))
        {
            self.sibling_reach = usize::MAX;
        }
        // Selectors 4 §16.1: a column combinator relates a cell to the
        // column elements of its columns, which its table's spans decide
        // (child lists are always a trigger).
        if complex
            .ancestors
            .iter()
            .any(|(c, _)| *c == Combinator::Column)
        {
            for name in ["span", "colspan", "rowspan"] {
                self.add_attribute(name);
            }
        }
        for simple in compounds(complex).flat_map(|c| &c.simples) {
            match simple {
                SimpleSelector::Universal | SimpleSelector::Type(_) => {}
                SimpleSelector::Id(_) => self.add_attribute("id"),
                SimpleSelector::Class(_) => self.add_attribute("class"),
                SimpleSelector::Attribute { name, .. } => self.add_attribute(name),
                SimpleSelector::Lang(_) => {
                    self.add_attribute("lang");
                    self.add_attribute("xml:lang");
                }
                // A cell's column reads its table's spans (and child
                // lists, always a trigger).
                SimpleSelector::NthColumn(_) => {
                    for name in ["span", "colspan", "rowspan"] {
                        self.add_attribute(name);
                    }
                }
                SimpleSelector::Pseudo(p) => {
                    self.state = true;
                    self.attribute_state |= reads_attributes(*p);
                }
                SimpleSelector::Not(list)
                | SimpleSelector::Is(list)
                | SimpleSelector::Where(list) => {
                    for inner in &list.0 {
                        self.add_reads(inner, true);
                    }
                }
                SimpleSelector::Nth(nth) => {
                    // The index reads the child list (always a trigger);
                    // `of S` reads what `S` reads, of the siblings.
                    for inner in nth.of.iter().flat_map(|of| &of.0) {
                        self.add_reads(inner, true);
                    }
                }
                // Not valid inside `:has()`, and unknown kinds: everything.
                other => {
                    debug_assert!(
                        matches!(other, SimpleSelector::Has(_)),
                        "has_triggers: unknown simple selector {other:?}"
                    );
                    self.all = true;
                }
            }
        }
    }

    fn add_attribute(&mut self, name: &str) {
        if !self.attributes.iter().any(|a| a.eq_ignore_ascii_case(name)) {
            self.attributes.push(name.to_ascii_lowercase());
        }
    }
}

/// How many earlier siblings away from the element a relative selector's
/// compounds sit on, the anchor can be (Selectors 4 §4.5): the run of
/// sibling combinators leading it, left to right — one per `+`, all of
/// them once a `~` is in the run; 0 when it leads with `>` or a
/// descendant combinator (the anchor is then an ancestor, which the walk
/// climbs to anyway).
fn leading_reach(rel: &rdom_core::selectors::RelativeSelector) -> usize {
    // `ancestors` is right to left: the combinators after the leading
    // one, left to right, are theirs reversed.
    let mut reach = 0usize;
    for comb in
        std::iter::once(rel.combinator).chain(rel.selector.ancestors.iter().rev().map(|(c, _)| *c))
    {
        match comb {
            Combinator::AdjacentSibling => reach = reach.saturating_add(1),
            Combinator::GeneralSibling => return usize::MAX,
            _ => break,
        }
    }
    reach
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triggers(css: &str) -> HasTriggers {
        HasTriggers::of_sheets([&rdom_css::parse(css).stylesheet])
    }

    /// No `:has()` anywhere: nothing fires.
    #[test]
    fn no_has_fires_nothing() {
        let t = triggers(".a .b { color: red } p:hover + q { color: red }");
        assert_eq!(t, HasTriggers::none());
        assert!(!t.fires(Cause::State));
        assert!(!t.fires(Cause::Attribute("class")));
    }

    /// Selectors 4 §4.5: the argument's simple selectors decide which
    /// changes can reach an anchor.
    #[test]
    fn the_arguments_reads_fire() {
        let t = triggers(".card:has(.err[data-k]) { color: red }");
        assert!(t.any());
        assert!(t.fires(Cause::Attribute("class")));
        assert!(t.fires(Cause::Attribute("data-k")));
        assert!(!t.fires(Cause::Attribute("title")));
        assert!(!t.fires(Cause::State));
        assert_eq!(t.sibling_reach(), 0);
        let t = triggers("form:has(:checked) { color: red } li:has(+ li:hover) { color: red }");
        assert!(t.fires(Cause::State));
        assert!(t.fires(Cause::Attribute("checked")));
        assert_eq!(t.sibling_reach(), 1);
        let t = triggers(":is(.x, div:not(:has(> .y ~ .z))) { color: red }");
        assert_eq!(t.sibling_reach(), 0, "the anchor is the parent: no walk");
        assert_eq!(
            triggers("a:has(+ b + c d) { color: red }").sibling_reach(),
            2
        );
        assert_eq!(
            triggers("a:has(+ b ~ c) { color: red }").sibling_reach(),
            usize::MAX
        );
        assert_eq!(
            triggers("a:has(> :is(b + c)) { color: red }").sibling_reach(),
            usize::MAX,
            "a nested sibling step"
        );
        assert!(t.fires(Cause::Attribute("class")));
    }

    /// Selectors 4 §16.1: a column combinator in a `:has()` argument reads
    /// the table's spans — a `colspan` change moves a cell between columns
    /// and can flip the anchor (C13G-COLUMN-INVALIDATION).
    #[test]
    fn a_column_combinator_in_has_reads_the_spans() {
        let t = triggers("body:has(col.hl || td.x) { color: red }");
        for name in ["span", "colspan", "rowspan", "class"] {
            assert!(t.fires(Cause::Attribute(name)), "{name}");
        }
        assert!(!t.fires(Cause::Attribute("title")));
    }
}
