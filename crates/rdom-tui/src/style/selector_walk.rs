//! Walks over every selector a set of sheets matches with — the inputs
//! of the invalidation checks (`uses_sibling_combinators`,
//! `SiblingTriggers`, `uses_validity`) — so each check sees the same
//! selectors the cascade does (`C1G-INVALIDATION`):
//!
//! - the rules' selectors, and the `<scope-start>` / `<scope-end>`
//!   selectors of every `@scope` (CSS Cascade 6 §2.5.1), which are not
//!   rule selectors but decide which elements a scoped rule styles;
//! - inside each, every complex selector nested in a `:not()` / `:is()`
//!   / `:where()` argument (`SimpleSelector::Is` is also the nesting
//!   selector `&`, CSS Nesting 1 §2), in an `:nth-child(… of S)`'s
//!   `S` and in a `:has()`'s relative selectors.
//!
//! `SimpleSelector` is `#[non_exhaustive]` and defined in rdom-core: a
//! variant this module does not know answers "yes" (the conservative
//! answer for every check here) and trips a `debug_assert!`, so the
//! workspace's tests catch an addition that needs walking (DESIGN,
//! non-exhaustive rule).

use rdom_core::selectors::{ComplexSelector, CompoundSelector, SelectorList, SimpleSelector};

use crate::style::Stylesheet;

/// Every top-level complex selector `sheet` matches with: its rules'
/// selectors, then each `@scope`'s start and end lists.
pub(crate) fn sheet_selectors(sheet: &Stylesheet) -> impl Iterator<Item = &ComplexSelector> {
    let rules = sheet.rules().iter().flat_map(|r| r.selector.0.iter());
    let scopes = sheet
        .scopes()
        .iter()
        .flat_map(|s| s.start.iter().chain(s.end.iter()))
        .flat_map(|list: &SelectorList| list.0.iter());
    rules.chain(scopes)
}

/// [`sheet_selectors`] without the user-agent rules — for a check whose
/// answer the UA's own rules are known not to need (each such caller
/// pins that with a test of the UA sheet).
pub(crate) fn author_selectors(sheet: &Stylesheet) -> impl Iterator<Item = &ComplexSelector> {
    let rules = sheet
        .rules()
        .iter()
        .filter(|r| r.origin != crate::style::RuleOrigin::UserAgent)
        .flat_map(|r| r.selector.0.iter());
    let scopes = sheet
        .scopes()
        .iter()
        .flat_map(|s| s.start.iter().chain(s.end.iter()))
        .flat_map(|list: &SelectorList| list.0.iter());
    rules.chain(scopes)
}

/// `complex`'s compounds: the subject, then the ancestors right to left.
pub(crate) fn compounds(complex: &ComplexSelector) -> impl Iterator<Item = &CompoundSelector> {
    std::iter::once(&complex.subject).chain(complex.ancestors.iter().map(|(_, c)| c))
}

/// The complex selectors a simple selector holds as its argument: a
/// `:not()` / `:is()` / `:where()` list, an `:nth-*()`'s `of S`, a
/// `:has()`'s relative selectors (each matched from an element related
/// to the anchor, not the anchor itself — a caller that cares asks for
/// `SimpleSelector::Has` itself). `Err(())` for a variant this module
/// does not know (module doc).
pub(crate) fn arguments(simple: &SimpleSelector) -> Result<Vec<&ComplexSelector>, ()> {
    match simple {
        SimpleSelector::Not(list) | SimpleSelector::Is(list) | SimpleSelector::Where(list) => {
            Ok(list.0.iter().collect())
        }
        SimpleSelector::Nth(nth) => Ok(nth.of.iter().flat_map(|of| &of.0).collect()),
        SimpleSelector::Has(relative) => Ok(relative.iter().map(|r| &r.selector).collect()),
        SimpleSelector::Universal
        | SimpleSelector::Type(_)
        | SimpleSelector::Id(_)
        | SimpleSelector::Class(_)
        | SimpleSelector::Attribute { .. }
        | SimpleSelector::Lang(_)
        | SimpleSelector::NthColumn(_)
        | SimpleSelector::Pseudo(_) => Ok(Vec::new()),
        other => {
            debug_assert!(false, "selector_walk: unknown simple selector {other:?}");
            Err(())
        }
    }
}

/// Whether `pred` holds for `complex` or for any complex selector nested
/// in an argument inside it, depth first. A simple selector of unknown
/// kind counts as a match (module doc).
pub(crate) fn any_complex(
    complex: &ComplexSelector,
    pred: &mut dyn FnMut(&ComplexSelector) -> bool,
) -> bool {
    if pred(complex) {
        return true;
    }
    for simple in compounds(complex).flat_map(|c| &c.simples) {
        match arguments(simple) {
            Ok(nested) => {
                if nested.into_iter().any(|inner| any_complex(inner, pred)) {
                    return true;
                }
            }
            Err(()) => return true,
        }
    }
    false
}

/// Whether `pred` holds for any simple selector of `complex`, nested
/// arguments included; an unknown kind counts as a match.
pub(crate) fn any_simple(
    complex: &ComplexSelector,
    pred: &dyn Fn(&SimpleSelector) -> bool,
) -> bool {
    any_complex(complex, &mut |c| {
        compounds(c).flat_map(|c| &c.simples).any(pred)
    })
}
