//! Which state changes can reach a *sibling's* selector match
//! (`P7G-SIBLING-MARK-NARROW-1`).
//!
//! A sibling combinator lets an element's match read a previous
//! sibling's state: `a:hover + b` restyles `b` when `a`'s `:hover`
//! flips, `[x] ~ p` when `x` is set on an earlier sibling. The dirty
//! tracker then dirties the changed element's siblings too — an
//! O(children) mark, so it must happen only when some rule can read the
//! state that changed.
//!
//! Only a compound **immediately left of** a `+` / `~` matters: a state
//! change on an element matching a compound left of a descendant or
//! child combinator reaches only that element's subtree, which the
//! tracker dirties anyway (and a sibling combinator further right, as in
//! `a:hover > b + c`, relates elements inside that subtree). So for each
//! such compound, [`SiblingTriggers::of_sheets`] records what it reads:
//!
//! - any pseudo-class → a state change (hover, active, focus, text,
//!   `:empty`) on an element marks its siblings;
//! - a pseudo-class that reads attributes (`:checked`, `:disabled`,
//!   `:open`, `:required`, `:placeholder-shown`, …) → any attribute
//!   change does;
//! - `[name]`, `.class`, `#id` → a change of that attribute (`name`,
//!   `class`, `id`) does.
//!
//! An `:nth-child(… of S)` / `:nth-last-child(… of S)` anywhere reads its
//! siblings the same way (Selectors 4 §13.3.1: only the siblings matching
//! `S` count), so every compound of its `S` is recorded as a left
//! compound too. The plain structural forms (`:nth-child(2)`,
//! `:first-of-type`) change only with the child list, whose change marks
//! every child of the parent already.
//!
//! The selectors walked are every one the cascade matches with — rule
//! selectors and `@scope` starts / ends — and the `:not()` / `:is()` /
//! `:where()` arguments inside them, wherever they appear
//! (`selector_walk`); every compound inside an argument that sits left
//! of a sibling combinator counts in full. A type selector (`h1 + p`)
//! reads nothing that can change. A simple selector this module does
//! not know marks everything (conservative) and trips a `debug_assert!`.

use rdom_core::selectors::{
    Combinator, ComplexSelector, CompoundSelector, PseudoClass, SimpleSelector,
};

use crate::style::Stylesheet;
use crate::style::selector_walk::{any_complex, compounds, sheet_selectors};

/// What kinds of change can reach a sibling's match under a set of
/// stylesheets — computed once per stylesheet set, like the validity
/// marks' sheet check.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SiblingTriggers {
    /// Every change marks siblings — the sheets are unknown.
    all: bool,
    /// A left compound has a pseudo-class.
    state: bool,
    /// A left compound has a pseudo-class that reads attributes.
    attribute_state: bool,
    /// Attribute names a left compound tests, ASCII-lowercase.
    attributes: Vec<String>,
    /// A left compound has a `:has()`, whose match changes with the
    /// anchor's subtree or later siblings.
    has: bool,
}

/// The change the dirty tracker is marking for.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Cause<'a> {
    /// A pseudo-class state: hover, active, focus, focus-visible,
    /// `:empty`, `:placeholder-shown` text.
    State,
    /// The named attribute (`class` for a class-list change).
    Attribute(&'a str),
    /// A `:has()` anchor's match may have changed
    /// (`style::has_triggers`).
    Has,
}

impl SiblingTriggers {
    /// Mark siblings on every change (the tracker's default, before the
    /// sheets are known).
    pub(crate) fn all() -> Self {
        Self {
            all: true,
            ..Self::default()
        }
    }

    /// Never mark siblings (no sheet uses a sibling combinator).
    pub(crate) fn none() -> Self {
        Self::default()
    }

    /// What `sheets` can read through a sibling combinator (module doc).
    pub(crate) fn of_sheets<'a>(sheets: impl IntoIterator<Item = &'a Stylesheet>) -> Self {
        let mut t = Self::none();
        for sheet in sheets {
            for complex in sheet_selectors(sheet) {
                t.visit_complex(complex);
            }
        }
        t
    }

    /// Whether a change of kind `cause` on an element can change a
    /// sibling's match.
    pub(crate) fn fires(&self, cause: Cause<'_>) -> bool {
        self.all
            || match cause {
                Cause::State => self.state,
                Cause::Attribute(name) => {
                    self.attribute_state
                        || self.attributes.iter().any(|a| a.eq_ignore_ascii_case(name))
                }
                Cause::Has => self.has,
            }
    }

    /// Record every compound left of a `+` / `~` in `complex` and in
    /// the selectors nested in its arguments (`selector_walk`).
    fn visit_complex(&mut self, complex: &ComplexSelector) {
        let unknown = any_complex(complex, &mut |c| {
            for (combinator, compound) in &c.ancestors {
                if matches!(
                    combinator,
                    Combinator::AdjacentSibling | Combinator::GeneralSibling
                ) {
                    self.add_left(compound);
                }
            }
            // `of S` counts the siblings matching `S` (module doc).
            for simple in compounds(c).flat_map(|c| &c.simples) {
                if let SimpleSelector::Nth(nth) = simple
                    && let Some(of) = &nth.of
                {
                    for compound in of.0.iter().flat_map(compounds) {
                        self.add_left(compound);
                    }
                }
            }
            false
        });
        if unknown {
            self.all = true;
        }
    }

    /// Record what `compound`, left of a sibling combinator, reads.
    fn add_left(&mut self, compound: &CompoundSelector) {
        for simple in &compound.simples {
            match simple {
                SimpleSelector::Universal | SimpleSelector::Type(_) => {}
                SimpleSelector::Id(_) => self.add_attribute("id"),
                SimpleSelector::Class(_) => self.add_attribute("class"),
                SimpleSelector::Attribute { name, .. } => self.add_attribute(name),
                // The language is the element's own `lang` / `xml:lang`
                // or an ancestor's (whose change marks the siblings'
                // shared subtree anyway).
                SimpleSelector::Lang(_) => {
                    self.add_attribute("lang");
                    self.add_attribute("xml:lang");
                }
                SimpleSelector::Pseudo(p) => {
                    self.state = true;
                    self.attribute_state |= reads_attributes(*p);
                }
                // The argument is matched against the same element: every
                // compound in it reads that element's (or its relatives')
                // state.
                SimpleSelector::Not(list)
                | SimpleSelector::Is(list)
                | SimpleSelector::Where(list) => {
                    for inner in &list.0 {
                        for c in compounds(inner) {
                            self.add_left(c);
                        }
                    }
                }
                // Its match reads the element's subtree and later siblings:
                // the anchor is marked with `Cause::Has` when they change.
                SimpleSelector::Has(_) => self.has = true,
                // The index reads the child list (marked on its change);
                // an `of S` reads what `S` reads of the same element.
                SimpleSelector::Nth(nth) => {
                    for c in nth.of.iter().flat_map(|of| &of.0).flat_map(compounds) {
                        self.add_left(c);
                    }
                }
                // `SimpleSelector` is `#[non_exhaustive]`: an unknown
                // kind marks everything (`selector_walk`).
                other => {
                    debug_assert!(false, "sibling_triggers: unknown simple selector {other:?}");
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

/// Whether `p`'s match reads attributes (`:checked` reads `checked`,
/// `:disabled` reads `disabled`, …). The interaction and tree-structural
/// ones do not; one this list does not know is assumed to.
pub(crate) fn reads_attributes(p: PseudoClass) -> bool {
    !matches!(
        p,
        PseudoClass::FirstChild
            | PseudoClass::LastChild
            | PseudoClass::OnlyChild
            | PseudoClass::Visited
            | PseudoClass::FirstOfType
            | PseudoClass::LastOfType
            | PseudoClass::OnlyOfType
            | PseudoClass::Empty
            | PseudoClass::Root
            | PseudoClass::Hover
            | PseudoClass::Active
            | PseudoClass::Focus
            | PseudoClass::FocusWithin
            | PseudoClass::FocusVisible
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triggers(css: &str) -> SiblingTriggers {
        SiblingTriggers::of_sheets([&rdom_css::parse(css).stylesheet])
    }

    #[test]
    fn a_type_only_left_compound_reads_nothing() {
        let t = triggers("h1 + p { color: red; } h2 ~ p { color: red; }");
        assert!(!t.fires(Cause::State));
        assert!(!t.fires(Cause::Attribute("class")));
    }

    #[test]
    fn a_pseudo_class_left_compound_fires_on_state() {
        let t = triggers("a:hover + b { color: red; }");
        assert!(t.fires(Cause::State));
        assert!(!t.fires(Cause::Attribute("x")), ":hover reads no attribute");
    }

    #[test]
    fn an_attribute_left_compound_fires_for_that_attribute_only() {
        let t = triggers("[x] + b { color: red; } .on ~ i { color: red; } #k + u { color: red; }");
        assert!(!t.fires(Cause::State));
        for name in ["x", "X", "class", "id"] {
            assert!(t.fires(Cause::Attribute(name)), "{name}");
        }
        assert!(!t.fires(Cause::Attribute("y")));
    }

    #[test]
    fn an_attribute_pseudo_class_fires_on_any_attribute() {
        let t = triggers("input:checked + label { color: red; }");
        assert!(t.fires(Cause::Attribute("checked")));
    }

    #[test]
    fn a_compound_left_of_a_child_combinator_only_is_not_a_trigger() {
        let t = triggers("a:hover > b + c { color: red; }");
        assert!(
            !t.fires(Cause::State),
            "b reads nothing; a's subtree is marked anyway"
        );
    }

    #[test]
    fn sibling_combinators_inside_not_count() {
        let t = triggers("p:not(.x + *) { color: red; }");
        assert!(t.fires(Cause::Attribute("class")));
        assert!(!t.fires(Cause::State));
    }

    /// Selectors 4 §13.3.1: `of S` counts the siblings matching `S`, so a
    /// change of what `S` reads reaches the siblings; a plain index
    /// reads no attribute.
    #[test]
    fn nth_of_s_reads_what_s_reads() {
        let t = triggers("li:nth-child(odd of .x[data-k]) { color: red }");
        assert!(t.fires(Cause::Attribute("class")));
        assert!(t.fires(Cause::Attribute("data-k")));
        assert!(!t.fires(Cause::State));
        let plain = triggers("li:nth-child(odd) { color: red } p:first-of-type { color: red }");
        assert_eq!(plain, SiblingTriggers::none());
    }

    #[test]
    fn no_sibling_combinator_fires_nothing() {
        let t = triggers("a:hover b { color: red; } [x] > p { color: red; }");
        assert_eq!(t, SiblingTriggers::none());
    }
}
