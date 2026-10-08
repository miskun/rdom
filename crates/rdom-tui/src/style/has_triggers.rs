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
    /// A relative selector reaches the anchor's later siblings.
    siblings: bool,
}

impl HasTriggers {
    /// Every change may reach an anchor (before the sheets are known).
    pub(crate) fn all() -> Self {
        Self {
            all: true,
            any: true,
            siblings: true,
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
                                t.siblings |= matches!(
                                    rel.combinator,
                                    Combinator::AdjacentSibling | Combinator::GeneralSibling
                                );
                                t.add_reads(&rel.selector);
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

    /// Whether a relative selector reaches later siblings: a change must
    /// look along earlier siblings for anchors too.
    pub(crate) fn siblings(&self) -> bool {
        self.siblings
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

    /// Record what `complex` (a relative selector, or a selector nested
    /// in one) reads, every compound of it: they match elements around
    /// the anchor, any of which can change.
    fn add_reads(&mut self, complex: &ComplexSelector) {
        if complex
            .ancestors
            .iter()
            .any(|(c, _)| matches!(c, Combinator::AdjacentSibling | Combinator::GeneralSibling))
        {
            self.siblings = true;
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
                SimpleSelector::Pseudo(p) => {
                    self.state = true;
                    self.attribute_state |= reads_attributes(*p);
                }
                SimpleSelector::Not(list)
                | SimpleSelector::Is(list)
                | SimpleSelector::Where(list) => {
                    for inner in &list.0 {
                        self.add_reads(inner);
                    }
                }
                SimpleSelector::Nth(nth) => {
                    // The index reads the child list (always a trigger);
                    // `of S` reads what `S` reads, of the siblings.
                    for inner in nth.of.iter().flat_map(|of| &of.0) {
                        self.add_reads(inner);
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
        assert!(!t.siblings());
        let t = triggers("form:has(:checked) { color: red } li:has(+ li:hover) { color: red }");
        assert!(t.fires(Cause::State));
        assert!(t.fires(Cause::Attribute("checked")));
        assert!(t.siblings());
        let t = triggers(":is(.x, div:not(:has(> .y ~ .z))) { color: red }");
        assert!(t.siblings(), "a sibling step inside the argument");
        assert!(t.fires(Cause::Attribute("class")));
    }
}
