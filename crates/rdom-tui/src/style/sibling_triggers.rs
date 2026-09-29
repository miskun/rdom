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
//! `:not()` / `:where()` arguments are walked the same way wherever they
//! appear, and every compound inside one that sits left of a sibling
//! combinator counts in full. A type selector (`h1 + p`) reads nothing
//! that can change. A simple selector this module does not know marks
//! everything (conservative).

use rdom_core::selectors::{
    Combinator, ComplexSelector, CompoundSelector, PseudoClass, SimpleSelector,
};

use crate::style::Stylesheet;

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
}

/// The change the dirty tracker is marking for.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Cause<'a> {
    /// A pseudo-class state: hover, active, focus, focus-visible,
    /// `:empty`, `:placeholder-shown` text.
    State,
    /// The named attribute (`class` for a class-list change).
    Attribute(&'a str),
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
            for rule in sheet.rules() {
                for complex in &rule.selector.0 {
                    t.visit_complex(complex);
                }
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
            }
    }

    fn visit_complex(&mut self, complex: &ComplexSelector) {
        for (combinator, compound) in &complex.ancestors {
            if matches!(
                combinator,
                Combinator::AdjacentSibling | Combinator::GeneralSibling
            ) {
                self.add_left(compound);
            }
        }
        for compound in std::iter::once(&complex.subject)
            .chain(complex.ancestors.iter().map(|(_, compound)| compound))
        {
            for simple in &compound.simples {
                if let SimpleSelector::Not(list) | SimpleSelector::Where(list) = simple {
                    for inner in &list.0 {
                        self.visit_complex(inner);
                    }
                }
            }
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
                SimpleSelector::Pseudo(p) => {
                    self.state = true;
                    self.attribute_state |= reads_attributes(*p);
                }
                // The argument is matched against the same element: every
                // compound in it reads that element's (or its relatives')
                // state.
                SimpleSelector::Not(list) | SimpleSelector::Where(list) => {
                    for inner in &list.0 {
                        self.add_left(&inner.subject);
                        for (_, c) in &inner.ancestors {
                            self.add_left(c);
                        }
                    }
                }
                _ => self.all = true,
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
fn reads_attributes(p: PseudoClass) -> bool {
    !matches!(
        p,
        PseudoClass::FirstChild
            | PseudoClass::LastChild
            | PseudoClass::OnlyChild
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

    #[test]
    fn no_sibling_combinator_fires_nothing() {
        let t = triggers("a:hover b { color: red; } [x] > p { color: red; }");
        assert_eq!(t, SiblingTriggers::none());
    }
}
