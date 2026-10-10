//! Caps against hostile selector depth (C16G-DEPTH-CAPS): the parser
//! recurses once per selector argument, the matcher once per nested
//! list, and a nested rule's `&` copies its parent's list — `& &` doubles
//! it at every level of nesting (CSS Nesting 1 §2). Both are bounded, so
//! no selector text can overflow the stack or exhaust memory; a selector
//! past either cap is invalid.

use super::{ComplexSelector, RelativeSelector, SelectorList, SimpleSelector};

/// How deep selector arguments may nest — `:is()`, `:not()`, `:where()`,
/// `:has()` and `:nth-child(… of S)` inside one another, with the lists
/// `&` stands for counted: `:not(:is(a))` is 2. One level deeper and the
/// selector is invalid (a forgiving `:is()` / `:where()` argument is
/// dropped instead). Real selectors nest a few deep; no engine publishes
/// a cap of its own.
pub const MAX_SELECTOR_NESTING: usize = 32;

/// How many simple selectors a selector may grow to by `&` expansion
/// (CSS Nesting 1 §2: each `&` stands for the whole parent list). Past
/// it the nested rule's selector is invalid — so `& &`, which doubles the
/// selector per level, stops after about a dozen levels instead of
/// allocating 2³⁰ copies.
pub const MAX_SELECTOR_SIZE: usize = 4096;

impl SelectorList {
    /// Its simple selectors, those of its arguments included.
    pub(crate) fn size(&self) -> usize {
        size(&mut self.0.iter())
    }

    /// How deep its arguments nest: 0 for a list of plain compounds.
    pub(crate) fn nesting(&self) -> usize {
        nesting(&mut self.0.iter())
    }
}

/// The simple selectors of `items`, recursively.
fn size(items: &mut dyn Iterator<Item = &ComplexSelector>) -> usize {
    simples(items)
        .map(|s| {
            1 + match arguments(s) {
                Arguments::List(l) => size(&mut l.0.iter()),
                Arguments::Relative(r) => size(&mut r.iter().map(|r| &r.selector)),
                Arguments::None => 0,
            }
        })
        .sum()
}

/// How deep the arguments of `items` nest.
fn nesting(items: &mut dyn Iterator<Item = &ComplexSelector>) -> usize {
    simples(items)
        .filter_map(|s| match arguments(s) {
            Arguments::List(l) => Some(1 + nesting(&mut l.0.iter())),
            Arguments::Relative(r) => Some(1 + nesting(&mut r.iter().map(|r| &r.selector))),
            Arguments::None => None,
        })
        .max()
        .unwrap_or(0)
}

/// Every simple selector of every compound of `items`.
fn simples<'a, 'i>(
    items: &'i mut dyn Iterator<Item = &'a ComplexSelector>,
) -> impl Iterator<Item = &'a SimpleSelector> + 'i {
    items
        .flat_map(|c| std::iter::once(&c.subject).chain(c.ancestors.iter().map(|(_, k)| k)))
        .flat_map(|compound| &compound.simples)
}

/// The selectors a simple selector takes as its argument.
enum Arguments<'a> {
    None,
    List(&'a SelectorList),
    Relative(&'a [RelativeSelector]),
}

fn arguments(simple: &SimpleSelector) -> Arguments<'_> {
    match simple {
        SimpleSelector::Not(l) | SimpleSelector::Is(l) | SimpleSelector::Where(l) => {
            Arguments::List(l)
        }
        SimpleSelector::Nth(nth) => nth.of.as_ref().map_or(Arguments::None, Arguments::List),
        SimpleSelector::Has(relative) => Arguments::Relative(relative),
        _ => Arguments::None,
    }
}
