//! CSS selector grammar — tokenizer + parser + AST.
//!
//! Supports a practical subset of CSS Level 3:
//!
//! - Simple selectors: `*`, `tag`, `#id`, `.class`, `[attr]`, `[attr=v]`,
//!   `[attr="v"]`, `[attr^=v]`, `[attr$=v]`, `[attr*=v]`, `[attr~=v]`, `[attr|=v]`
//! - Compound: `tag.foo#bar[attr=x]`
//! - Combinators: descendant (space), child `>`, adjacent `+`, general `~`
//! - Pseudo-classes: `:not(selector)`, `:where(selector-list)`,
//!   `:first-child`, `:last-child`, `:only-child`, `:empty`, `:root`,
//!   plus the interaction pseudos (`:hover`, `:focus`, `:focus-within`, …)
//!   and the form-state pseudos (`:checked`, `:disabled`, `:enabled`,
//!   `:valid`, `:invalid`, `:required`, `:optional`, …)
//! - Selector list: `a, b, c`
//! - Nested rule selectors (CSS Nesting 1): `&` and relative selectors,
//!   resolved against the parent rule's list by [`parse_nested`]
//!
//! Identifiers and attribute values decode CSS escapes (`.\31 0` is
//! class `10`, `#a\:b` is id `a:b`) through [`crate::css_syntax`], the
//! same decoder the value tokenizer in `rdom-style` uses.
//!
//! Attribute values match case-sensitively, except the attributes HTML
//! §4.16.2 lists as ASCII case-insensitive on HTML elements (`type`,
//! `method`, `enctype`, `lang`, `checked`, …), so the UA sheet's
//! `input[type=checkbox]` matches `<input type="CheckBox">`.
//!
//! `:is()` matches an element any item of its (forgiving) list matches,
//! with the specificity of the most specific item; `:where()` matches
//! the same way but contributes **zero specificity** (Selectors L4) —
//! the mechanism a component library uses to ship default styles that
//! any author rule overrides freely.
//!
//! Not supported yet (reserved for later phases):
//! - `:nth-child(an+b)`, `:has(...)`, namespaces, attribute
//!   case flags (`[attr="v" i]`), pseudo-elements (`::before`, `::after`).

use std::fmt;

// ─── AST ─────────────────────────────────────────────────────────────

/// A full selector expression — one or more comma-separated compound
/// selector chains connected by combinators.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectorList(pub Vec<ComplexSelector>);

/// A chain of compound selectors joined by combinators, read left→right.
/// `a > b c` parses as `[Compound(a), Combinator(Child), Compound(b),
/// Combinator(Descendant), Compound(c)]` — stored as a root compound plus
/// a list of `(combinator, compound)` pairs for easier right-to-left
/// matching.
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexSelector {
    /// The right-most compound (what we match against the candidate node).
    pub subject: CompoundSelector,
    /// Parents of the subject, each prefaced by the combinator connecting
    /// it to the *next* compound on the right. Ordered right-to-left
    /// for efficient matching (e.g. `a > b c` → [(Descendant, b), (Child, a)]).
    pub ancestors: Vec<(Combinator, CompoundSelector)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Combinator {
    /// `a b` — b descends from a.
    Descendant,
    /// `a > b` — b is a direct child of a.
    Child,
    /// `a + b` — b is the immediately-following sibling of a.
    AdjacentSibling,
    /// `a ~ b` — b is a following sibling of a.
    GeneralSibling,
}

/// A compound selector: one or more simple selectors that must *all* match.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompoundSelector {
    pub simples: Vec<SimpleSelector>,
}

impl ComplexSelector {
    /// The selector's specificity `(A, B, C)` per
    /// [Selectors 4 §17](https://www.w3.org/TR/selectors-4/#specificity):
    /// `A` counts ID selectors; `B` class selectors, attribute selectors
    /// and pseudo-classes; `C` type selectors (pseudo-elements live
    /// outside this AST, so the caller adds them). `*` counts nothing,
    /// `:not(X)` counts as its most specific argument and `:where(X)` as
    /// zero. The match over the selector vocabulary lives here, next to
    /// the (`#[non_exhaustive]`) AST, so a new simple selector is counted
    /// where it is defined.
    pub fn specificity(&self) -> (u16, u16, u16) {
        let mut abc = (0, 0, 0);
        add_compound_specificity(&mut abc, &self.subject);
        for (_, compound) in &self.ancestors {
            add_compound_specificity(&mut abc, compound);
        }
        abc
    }
}

impl SelectorList {
    /// The largest [`ComplexSelector::specificity`] of the list's
    /// selectors — how `:not()` and `:is()` count their argument
    /// (Selectors 4 §17). `(0, 0, 0)` for an empty list.
    pub fn max_specificity(&self) -> (u16, u16, u16) {
        self.0
            .iter()
            .map(ComplexSelector::specificity)
            .max()
            .unwrap_or((0, 0, 0))
    }
}

fn add_compound_specificity(abc: &mut (u16, u16, u16), compound: &CompoundSelector) {
    for simple in &compound.simples {
        match simple {
            SimpleSelector::Universal | SimpleSelector::Where(_) => {}
            SimpleSelector::Type(_) => abc.2 += 1,
            SimpleSelector::Id(_) => abc.0 += 1,
            SimpleSelector::Class(_)
            | SimpleSelector::Attribute { .. }
            | SimpleSelector::Pseudo(_) => abc.1 += 1,
            SimpleSelector::Not(inner) | SimpleSelector::Is(inner) => {
                let (a, b, c) = inner.max_specificity();
                abc.0 += a;
                abc.1 += b;
                abc.2 += c;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SimpleSelector {
    /// `*`.
    Universal,
    /// `tag`.
    Type(String),
    /// `#id` → equivalent to `[id=...]` but stored separately for index support.
    Id(String),
    /// `.class`.
    Class(String),
    /// `[attr]` / `[attr op value]`.
    Attribute {
        name: String,
        op: Option<AttrOp>,
        value: Option<String>,
    },
    /// `:not(...)` — the negated selector list.
    Not(Box<SelectorList>),
    /// `:is()` semantics over a selector list: matches when any complex
    /// selector in the list matches the element, with the specificity of
    /// the list's most specific item (Selectors 4 §4.2, §17). Parsed
    /// from `:is(<forgiving-selector-list>)` — an argument that does not
    /// parse is dropped, and an empty list matches nothing — and produced
    /// by the nesting selector `&` (CSS Nesting 1 §2, [`parse_nested`]).
    Is(Box<SelectorList>),
    /// `:where(...)` — matches like `:is()` (any complex selector in the
    /// list matches the element), but contributes **zero specificity**
    /// (Selectors Level 4). Lets a library ship default styles that any
    /// real author selector overrides without a specificity fight.
    Where(Box<SelectorList>),
    /// Structural pseudo-classes.
    Pseudo(PseudoClass),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttrOp {
    /// `[attr=value]` — exact match.
    Exact,
    /// `[attr~=value]` — one of a whitespace-separated list.
    Includes,
    /// `[attr|=value]` — exact or starts with `value-`.
    DashMatch,
    /// `[attr^=value]` — prefix.
    Prefix,
    /// `[attr$=value]` — suffix.
    Suffix,
    /// `[attr*=value]` — substring.
    Substring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PseudoClass {
    FirstChild,
    LastChild,
    OnlyChild,
    Empty,
    Root,
    /// `:hover` — matches the Dom's currently-hovered node (tracked via
    /// `Dom::set_hovered`) and every ancestor of it (Selectors 4 §9.2).
    Hover,
    /// `:active` — matches the element being activated (tracked via
    /// `Dom::set_active`) and every ancestor of it (Selectors 4 §9.4).
    Active,
    /// `:focus` — matches when the node is the Dom's currently-focused
    /// node (tracked via `Dom::set_focused`).
    Focus,
    /// `:focus-within` — matches when the node OR any of its
    /// descendants is the Dom's currently-focused node. Mirrors
    /// CSS Selectors L4 §10.1.4 — lets authors style an entire
    /// container (form row, label, dialog) based on whether
    /// focus is somewhere inside it. The substrate walks from
    /// the focused node upward through parents; every ancestor
    /// in the chain matches, plus the focused node itself.
    FocusWithin,
    /// `:focus-visible` — Selectors 4 §13.2: the focused node while
    /// the UA judges its focus should be evident
    /// ([`Dom::focus_visible`](crate::Dom::focus_visible), driven by
    /// the backend's keyboard / pointer heuristics).
    FocusVisible,
    /// `:checked` — matches when the element has a `checked`
    /// attribute (any value, presence-only). The user-toggle
    /// builtins flip this attribute on click / Space, so this
    /// selector reflects current state without needing a separate
    /// IDL property.
    Checked,
    /// `:placeholder-shown` — matches form controls that have a
    /// non-empty `placeholder` attribute AND whose current text
    /// content is empty. Used by UA rules to render the
    /// placeholder via `::before { content: attr(placeholder) }`.
    PlaceholderShown,
    /// `:indeterminate` — matches elements in an indeterminate
    /// state. In this v1 that means `<progress>` without a `value`
    /// attribute; browsers also match indeterminate checkboxes
    /// and orphan radio buttons, which we defer to polish (neither
    /// has a concrete state model in rdom yet).
    Indeterminate,
    /// `:open` — matches elements with the `open` attribute
    /// present. Covers `<details open>` and `<dialog open>` /
    /// `<dialog data-rdom-open>`. Authors use it to style the
    /// expanded state of disclosure widgets via CSS without having
    /// to write attribute selectors themselves.
    Open,
    /// `:disabled` — matches elements that are *actually disabled*
    /// (HTML §4.16.3): a form control with `disabled` or inside a
    /// `<fieldset disabled>` (outside its first `<legend>`), a disabled
    /// `<fieldset>` / `<optgroup>`, a disabled `<option>`. See
    /// [`Dom::is_actually_disabled`](crate::Dom::is_actually_disabled).
    Disabled,
    /// `:enabled` — matches the elements that can be disabled
    /// (`<button>`, `<input>`, `<select>`, `<textarea>`, `<fieldset>`,
    /// `<optgroup>`, `<option>`) when they are not actually disabled.
    Enabled,
    /// `:valid` — HTML §4.16.3: a candidate for constraint validation
    /// that satisfies its constraints, a `<form>` owning no invalid
    /// candidate, a `<fieldset>` with no invalid descendant candidate.
    /// The verdict per candidate comes from the backend's validity hook
    /// ([`Dom::set_validity_hook`](crate::Dom::set_validity_hook)); without
    /// one every candidate is valid, so a backend matching this installs
    /// its hook — see
    /// [`Dom::constraint_validity`](crate::Dom::constraint_validity).
    Valid,
    /// `:invalid` — the complement of `:valid` among the same elements:
    /// an invalid candidate, a form or fieldset with one.
    Invalid,
    /// `:required` — an `<input>` (in a state `required` applies to),
    /// `<select>` or `<textarea>` with the `required` attribute.
    Required,
    /// `:optional` — an `<input>`, `<select>` or `<textarea>` that is
    /// not `:required`.
    Optional,
    /// `:scope` (Selectors 4 §14.3) — the scoping root: an `@scope`
    /// rule's root (CSS Cascade 6 §2.5) when matched through
    /// [`Dom::matches_list_in_scope`](crate::Dom::matches_list_in_scope),
    /// else `:root`. A nesting selector `&` with no parent rule is
    /// `:scope` too (CSS Nesting 1 §2).
    Scope,
}

// ─── Error ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseError {
    pub msg: String,
    pub pos: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "selector parse error at {}: {}", self.pos, self.msg)
    }
}

impl std::error::Error for ParseError {}

mod nesting;
mod parser;
#[cfg(test)]
mod tests;

pub use nesting::{parse_nested, parse_scoped};
pub use parser::parse;
