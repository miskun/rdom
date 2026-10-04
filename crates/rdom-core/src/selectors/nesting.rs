//! Nested style rule selectors (CSS Nesting 1 §2).
//!
//! A nested rule's selector is a *relative* selector list resolved
//! against its parent rule's list:
//!
//! - `&` stands for the parent's elements, as `:is(<parent list>)`
//!   ([`SimpleSelector::Is`]) — it may sit anywhere: in a compound
//!   (`&.x`), after other compounds (`.x &`), inside a pseudo-class
//!   argument (`:not(&)`);
//! - a selector that starts with a combinator (`> p`, `+ p`, `~ p`) is
//!   anchored at `&` with it;
//! - a selector with no `&` and no leading combinator is anchored at
//!   `&` with the descendant combinator (`p` is `& p`).
//!
//! When the parent list has one item, the parent's selector is spliced
//! in place of the `:is()` — anywhere for a single-compound parent
//! (`.x &` with parent `.a` is `.x .a`), in the outermost compound
//! otherwise (`.a .b { &.c {} }` becomes `.a .b.c`). That matches the same
//! elements with the same specificity, and keeps the subject's id /
//! class / type visible to the rule index.

use super::parser::Parser;
use super::{
    Combinator, ComplexSelector, CompoundSelector, ParseError, SelectorList, SimpleSelector,
};

/// Parse `input` as the selector of a style rule nested in a rule whose
/// selector list is `parent` (CSS Nesting 1 §2). `parent` should hold
/// no pseudo-element items — `&` cannot represent pseudo-elements; an
/// empty `parent` makes `&` match nothing.
pub fn parse_nested(input: &str, parent: &SelectorList) -> Result<SelectorList, ParseError> {
    let mut p = Parser::new(input, Some(parent));
    let mut items = Vec::new();
    loop {
        p.skip_ws();
        let leading = match p.peek() {
            Some(b'>') => Some(Combinator::Child),
            Some(b'+') => Some(Combinator::AdjacentSibling),
            Some(b'~') => Some(Combinator::GeneralSibling),
            _ => None,
        };
        if leading.is_some() {
            p.bump();
            p.skip_ws();
        }
        p.nest_seen = false;
        let mut complex = p.parse_complex_selector()?;
        match (leading, p.nest_seen) {
            (Some(combinator), _) => anchor(&mut complex, combinator, parent),
            (None, false) => anchor(&mut complex, Combinator::Descendant, parent),
            (None, true) => {}
        }
        splice_outermost(&mut complex, parent);
        items.push(complex);
        p.skip_ws();
        if p.peek() == Some(b',') {
            p.bump();
            continue;
        }
        break;
    }
    p.finish()?;
    Ok(SelectorList(items))
}

/// Prefix `complex` with `& <combinator>`.
fn anchor(complex: &mut ComplexSelector, combinator: Combinator, parent: &SelectorList) {
    let nest = CompoundSelector {
        simples: vec![SimpleSelector::Is(Box::new(parent.clone()))],
    };
    complex.ancestors.push((combinator, nest));
}

/// With a one-item `parent`, replace `&` by the parent's selector
/// itself: in any compound when the parent is a single compound
/// (`:is(.a)` is `.a`), else only in the outermost compound, where
/// the parent's ancestors can extend the chain.
fn splice_outermost(complex: &mut ComplexSelector, parent: &SelectorList) {
    let [single] = parent.0.as_slice() else {
        return;
    };
    let is_nest = |s: &SimpleSelector| matches!(s, SimpleSelector::Is(list) if **list == *parent);
    let inline = |compound: &mut CompoundSelector| {
        let Some(at) = compound.simples.iter().position(is_nest) else {
            return false;
        };
        compound.simples.remove(at);
        let mut simples = single.subject.simples.clone();
        simples.append(&mut compound.simples);
        compound.simples = simples;
        true
    };
    if single.ancestors.is_empty() {
        inline(&mut complex.subject);
        for (_, compound) in &mut complex.ancestors {
            inline(compound);
        }
        return;
    }
    let outermost = match complex.ancestors.last_mut() {
        Some((_, compound)) => compound,
        None => &mut complex.subject,
    };
    if inline(outermost) {
        complex.ancestors.extend(single.ancestors.iter().cloned());
    }
}
