//! CSS selector specificity — a 4-tuple ordered lexicographically.
//!
//! CSS sorts competing rules by specificity, falling back to source
//! order on ties. The 4-tuple counts, in priority order:
//!
//! 1. `inline`  — 1 if the rule is an inline `style=` attribute, else 0.
//! 2. `id`      — number of `#id` selectors in the compound.
//! 3. `class_attr_pseudo` — number of `.class`, `[attr]`, and
//!    `:pseudo-class` selectors. `:not(sel)` counts as the specificity
//!    of its argument, so `:not(.foo)` contributes 1 here.
//! 4. `type_pseudo_el` — number of `tag` type selectors and
//!    `::pseudo-element` selectors. `*` (universal) contributes 0.
//!
//! Example: `#nav .item:not(.disabled) > a::before` has specificity
//! `(0, 1, 2, 2)` — one id (`#nav`), two class-level (`.item` + `:not(.disabled)`),
//! two type-level (`a` + `::before`).
//!
//! Reference: [Selectors Level 4 §17](https://www.w3.org/TR/selectors-4/#specificity).

use rdom_core::selectors::{CompoundSelector, PseudoClass, SelectorList, SimpleSelector};

/// 4-tuple specificity. `Ord` compares field-by-field in declaration
/// order, which is exactly the CSS rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Specificity {
    pub inline: u16,
    pub id: u16,
    pub class_attr_pseudo: u16,
    pub type_pseudo_el: u16,
}

impl Specificity {
    /// Zero specificity — the implicit default for UA rules before any
    /// explicit adjustment.
    pub const ZERO: Self = Self {
        inline: 0,
        id: 0,
        class_attr_pseudo: 0,
        type_pseudo_el: 0,
    };

    /// `(1, 0, 0, 0)` — any inline style beats any stylesheet rule of
    /// specificity `(0, *, *, *)`.
    pub const INLINE: Self = Self {
        inline: 1,
        id: 0,
        class_attr_pseudo: 0,
        type_pseudo_el: 0,
    };

    /// Compute specificity for one complex selector. Adds counts from
    /// every compound in the chain (subject + ancestors). Pseudo-element
    /// targets (`::before` / `::after`) are passed separately because
    /// they live outside the core selector AST — pass 1 to add a type-
    /// level bump, 0 otherwise.
    pub fn of_complex(
        complex: &rdom_core::selectors::ComplexSelector,
        pseudo_element_count: u16,
    ) -> Self {
        let mut s = Self::ZERO;
        s.add_compound(&complex.subject);
        for (_combinator, compound) in &complex.ancestors {
            s.add_compound(compound);
        }
        s.type_pseudo_el += pseudo_element_count;
        s
    }

    /// Compute the max specificity across a selector list. `.foo, #bar`
    /// acts like two independent rules; the "effective" specificity for
    /// a specific match is the one that matched. For stylesheet storage
    /// we keep one spec per rule so callers flatten lists beforehand.
    ///
    /// This helper exists for tests and for callers who really want the
    /// highest specificity any list item could contribute.
    pub fn max_of_list(list: &SelectorList, pseudo_element_count: u16) -> Self {
        list.0
            .iter()
            .map(|complex| Self::of_complex(complex, pseudo_element_count))
            .max()
            .unwrap_or(Self::ZERO)
    }

    fn add_compound(&mut self, compound: &CompoundSelector) {
        for simple in &compound.simples {
            self.add_simple(simple);
        }
    }

    fn add_simple(&mut self, simple: &SimpleSelector) {
        match simple {
            SimpleSelector::Universal => {}
            SimpleSelector::Type(_) => self.type_pseudo_el += 1,
            SimpleSelector::Id(_) => self.id += 1,
            SimpleSelector::Class(_) | SimpleSelector::Attribute { .. } => {
                self.class_attr_pseudo += 1
            }
            SimpleSelector::Pseudo(pc) => match pc {
                // Structural + interaction pseudos count as class-level.
                PseudoClass::FirstChild
                | PseudoClass::LastChild
                | PseudoClass::OnlyChild
                | PseudoClass::Empty
                | PseudoClass::Root
                | PseudoClass::Hover
                | PseudoClass::Focus
                | PseudoClass::FocusWithin
                | PseudoClass::Checked
                | PseudoClass::PlaceholderShown
                | PseudoClass::Indeterminate
                | PseudoClass::Open => self.class_attr_pseudo += 1,
            },
            // `:not(X)` contributes the specificity of X (max across its list).
            SimpleSelector::Not(inner) => {
                let inner_spec = Self::max_of_list(inner, 0);
                self.id += inner_spec.id;
                self.class_attr_pseudo += inner_spec.class_attr_pseudo;
                self.type_pseudo_el += inner_spec.type_pseudo_el;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdom_core::selectors::parse;

    fn spec(sel: &str) -> Specificity {
        let list = parse(sel).unwrap();
        // Most stylesheets have one complex selector per rule; max gives
        // us the right answer for test selectors that happen to be lists.
        Specificity::max_of_list(&list, 0)
    }

    #[test]
    fn zero_is_lowest() {
        assert!(Specificity::ZERO < spec("a"));
    }

    #[test]
    fn inline_wins_over_id() {
        assert!(Specificity::INLINE > spec("#main"));
    }

    #[test]
    fn id_beats_class() {
        assert!(spec("#main") > spec(".main"));
    }

    #[test]
    fn class_beats_type() {
        assert!(spec(".foo") > spec("div"));
    }

    #[test]
    fn multiple_same_level_accumulate() {
        assert!(spec(".a.b") > spec(".a"));
        assert!(spec(".a.b.c") > spec(".a.b"));
    }

    #[test]
    fn id_count_stacks() {
        assert!(spec("#a #b") > spec("#a"));
    }

    #[test]
    fn attribute_selectors_count_like_classes() {
        assert_eq!(spec("[href]"), spec(".foo"));
        assert_eq!(spec("[href=x]"), spec(".foo"));
    }

    #[test]
    fn universal_contributes_nothing() {
        assert_eq!(spec("*"), Specificity::ZERO);
    }

    #[test]
    fn type_selectors_count_1_each() {
        let s = spec("div span a");
        assert_eq!(s.type_pseudo_el, 3);
        assert_eq!(s.class_attr_pseudo, 0);
        assert_eq!(s.id, 0);
    }

    #[test]
    fn structural_pseudo_class_counts_as_class() {
        assert_eq!(spec(":first-child"), spec(".foo"));
        assert_eq!(spec(":empty"), spec(".foo"));
    }

    #[test]
    fn not_pseudo_inherits_inner_specificity() {
        // :not(.foo) counts as 1 class.
        assert_eq!(spec(":not(.foo)"), spec(".foo"));
        // :not(#bar) counts as 1 id.
        assert_eq!(spec(":not(#bar)"), spec("#bar"));
        // Nested :not() stacks.
        assert_eq!(spec(":not(.a.b)"), spec(".a.b"));
    }

    #[test]
    fn complex_mixed_selector() {
        // #nav .item:not(.disabled) > a
        //   id=1, class_attr=2 (.item + .disabled in :not()),
        //   type=2 (a + no universal, no div)
        //   wait: #nav is an id, .item is a class, :not(.disabled) adds 1 class, > a adds 1 type.
        //   No 'div' here.
        let s = spec("#nav .item:not(.disabled) > a");
        assert_eq!(s.id, 1);
        assert_eq!(s.class_attr_pseudo, 2);
        assert_eq!(s.type_pseudo_el, 1);
    }

    #[test]
    fn pseudo_element_bump_applies_via_caller() {
        let list = parse("h1").unwrap();
        let s = Specificity::max_of_list(&list, 1); // e.g. ::before
        assert_eq!(s.type_pseudo_el, 2); // h1 (1) + pseudo-element (1)
    }

    #[test]
    fn ordering_is_lexicographic_tuple() {
        // (0, 0, 0, 2) < (0, 0, 1, 0) — class beats 2 types
        assert!(spec("div span") < spec(".foo"));
        // (0, 1, 0, 0) < (1, 0, 0, 0) — inline beats any id
        assert!(spec("#x") < Specificity::INLINE);
    }

    #[test]
    fn selector_list_max_wins() {
        // `a, #main` — max is #main (0,1,0,0)
        let list = parse("a, #main").unwrap();
        let s = Specificity::max_of_list(&list, 0);
        assert_eq!(s.id, 1);
        assert_eq!(s.type_pseudo_el, 0);
    }

    #[test]
    fn default_is_zero() {
        assert_eq!(Specificity::default(), Specificity::ZERO);
    }
}
