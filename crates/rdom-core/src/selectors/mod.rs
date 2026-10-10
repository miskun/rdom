//! CSS selector grammar — tokenizer + parser + AST.
//!
//! Supports a practical subset of CSS Level 3:
//!
//! - Simple selectors: `*`, `tag`, `#id`, `.class`, `[attr]`, `[attr=v]`,
//!   `[attr="v"]`, `[attr^=v]`, `[attr$=v]`, `[attr*=v]`, `[attr~=v]`, `[attr|=v]`,
//!   each with an optional case flag (`[attr=v i]`, `[attr=v s]`)
//! - Compound: `tag.foo#bar[attr=x]`
//! - Combinators: descendant (space), child `>`, adjacent `+`, general `~`
//! - Pseudo-classes: `:not(selector)`, `:where(selector-list)`,
//!   `:first-child`, `:last-child`, `:only-child`, `:empty`, `:root`,
//!   `:nth-child(An+B [of S])`, `:nth-last-child()`, `:nth-of-type()`,
//!   `:nth-last-of-type()`, `:first-of-type`, `:last-of-type`,
//!   `:only-of-type` (the An+B microsyntax of CSS Syntax 3 §6),
//!   `:link`, `:any-link`, `:visited` (never matches), `:lang()`,
//!   `:dir()`, `:has(<relative-selector-list>)`,
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
//! `input[type=checkbox]` matches `<input type="CheckBox">`. The case
//! flags of Selectors 4 §6.3 override that either way: `[a=b i]`
//! compares ASCII case-insensitively, `[a=b s]` case-sensitively
//! ([`AttrCase`]).
//!
//! `:is()` matches an element any item of its (forgiving) list matches,
//! with the specificity of the most specific item; `:where()` matches
//! the same way but contributes **zero specificity** (Selectors L4) —
//! the mechanism a component library uses to ship default styles that
//! any author rule overrides freely.
//!
//! Not supported yet (reserved for later phases):
//! - namespaces, pseudo-elements
//!   (`::before`, `::after`).

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
    /// `a || b` — b is a cell of a column a represents (Selectors 4
    /// §16.1), by HTML's table model.
    Column,
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
            SimpleSelector::Lang(_) | SimpleSelector::NthColumn(_) => abc.1 += 1,
            // Selectors 4 §15: like `:is()`, its most specific argument.
            SimpleSelector::Has(relative) => {
                let (a, b, c) = relative
                    .iter()
                    .map(|r| r.selector.specificity())
                    .max()
                    .unwrap_or((0, 0, 0));
                abc.0 += a;
                abc.1 += b;
                abc.2 += c;
            }
            // Selectors 4 §15: a pseudo-class, plus its `of S` list's
            // most specific selector.
            SimpleSelector::Nth(nth) => {
                abc.1 += 1;
                if let Some(of) = &nth.of {
                    let (a, b, c) = of.max_specificity();
                    abc.0 += a;
                    abc.1 += b;
                    abc.2 += c;
                }
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
        /// The case flag (`[a=b i]` / `[a=b s]`, Selectors 4 §6.3).
        case: AttrCase,
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
    /// `:has(<relative-selector-list>)` (Selectors 4 §4.5): matches the
    /// element (the anchor) when one of the relative selectors matches
    /// an element relative to it. Not valid inside another `:has()`.
    Has(Vec<RelativeSelector>),
    /// `:lang(<ranges>)` (Selectors 4 §7.2): the element's content
    /// language matches one of the language ranges by RFC 4647 §3.3.2
    /// extended filtering ([`Dom::language`](crate::Dom::language)).
    Lang(Vec<String>),
    /// `:nth-child()` / `:nth-last-child()` / `:nth-of-type()` /
    /// `:nth-last-of-type()` (Selectors 4 §13.3.1–§13.3.2, §13.4.1–§13.4.2).
    Nth(Box<NthSelector>),
    /// `:nth-col()` / `:nth-last-col()` (Selectors 4 §16.2–§16.3): a cell
    /// of a table column counted from the first or the last.
    NthColumn(NthColumnSelector),
}

/// An `:nth-col()` / `:nth-last-col()` pseudo-class (Selectors 4
/// §16.2–§16.3): it matches a cell belonging to a column with `a·n + b -
/// 1` columns before it (`:nth-col`) or after it (`:nth-last-col`), for
/// some integer `n >= 0`, by HTML's table model.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NthColumnSelector {
    /// Counted from the last column (`:nth-last-col`).
    pub last: bool,
    /// `A` of `An+B`.
    pub a: i32,
    /// `B` of `An+B`.
    pub b: i32,
}

impl NthColumnSelector {
    /// Whether the 1-based column `index` is `a·n + b` for some `n >= 0`.
    pub fn matches_index(&self, index: u32) -> bool {
        let (a, b, i) = (i64::from(self.a), i64::from(self.b), i64::from(index));
        if a == 0 {
            return i == b;
        }
        let steps = i - b;
        steps % a == 0 && steps / a >= 0
    }
}

/// A relative selector (Selectors 4 §3.4), the argument of `:has()`: a
/// complex selector whose leftmost compound is related to the anchor
/// element by `combinator` — `Descendant` when the text starts with no
/// combinator (`:has(img)`), else the leading `>`, `+` or `~`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RelativeSelector {
    /// How the leftmost compound relates to the anchor.
    pub combinator: Combinator,
    /// The selector, matched right to left as any complex selector.
    pub selector: ComplexSelector,
}

/// An `:nth-*()` pseudo-class (Selectors 4 §13.3–§13.4): it matches an
/// element whose 1-based index among the siblings [`NthKind`] counts is
/// `a·n + b` for some integer `n >= 0`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NthSelector {
    /// Which siblings count, from which end.
    pub kind: NthKind,
    /// `A` of `An+B` (CSS Syntax 3 §6).
    pub a: i32,
    /// `B` of `An+B`.
    pub b: i32,
    /// `of S` (`:nth-child` / `:nth-last-child` only): only the siblings
    /// matching `S` count, and the element must match `S` itself.
    pub of: Option<SelectorList>,
}

impl NthSelector {
    /// Whether the 1-based `index` is `a·n + b` for some `n >= 0`.
    pub fn matches_index(&self, index: u32) -> bool {
        let (a, b, i) = (i64::from(self.a), i64::from(self.b), i64::from(index));
        if a == 0 {
            return i == b;
        }
        let steps = i - b;
        steps % a == 0 && steps / a >= 0
    }
}

/// The siblings an [`NthSelector`] counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NthKind {
    /// `:nth-child()`: element siblings, from the first.
    Child,
    /// `:nth-last-child()`: element siblings, from the last.
    LastChild,
    /// `:nth-of-type()`: siblings of the element's type, from the first.
    OfType,
    /// `:nth-last-of-type()`: siblings of its type, from the last.
    LastOfType,
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

/// How an attribute selector compares the attribute's value
/// (Selectors 4 §6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttrCase {
    /// No flag: the document language decides — ASCII case-insensitive
    /// for the attributes HTML §4.16.2 lists (`type`, `lang`, …),
    /// case-sensitive for every other.
    #[default]
    Default,
    /// `i`: ASCII case-insensitive.
    AsciiInsensitive,
    /// `s`: case-sensitive (identical code points).
    Sensitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PseudoClass {
    FirstChild,
    LastChild,
    OnlyChild,
    /// `:first-of-type` — `:nth-of-type(1)` (Selectors 4 §13.4.3).
    FirstOfType,
    /// `:last-of-type` — `:nth-last-of-type(1)` (Selectors 4 §13.4.4).
    LastOfType,
    /// `:only-of-type` — both (Selectors 4 §13.4.5).
    OnlyOfType,
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
    /// `:indeterminate` (Selectors 4 §14.4.3, HTML §4.16.3): a checkbox
    /// whose indeterminate flag is set (reflected into the
    /// `indeterminate` attribute), a radio whose radio button group has
    /// no checked member, and a `<progress>` without a `value`
    /// ([`Dom::is_indeterminate`](crate::Dom::is_indeterminate)).
    Indeterminate,
    /// `:open` (Selectors 4, HTML §4.16.3): an element with the
    /// `open` attribute — `<details open>` and an open `<dialog>`,
    /// modal or not (`dialog::show` / `show_modal` set `open`).
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
    /// `:any-link` (Selectors 4 §8.1): an `a` or `area` element with an
    /// `href` — the source anchor of a hyperlink (HTML §4.16.3).
    AnyLink,
    /// `:link` (Selectors 4 §8.2): an `:any-link` element not visited —
    /// every one, as rdom keeps no visit history.
    Link,
    /// `:visited` (Selectors 4 §8.2): never matches — rdom keeps no
    /// visit history (DIVERGENCES §1). Parsed so `a:link, a:visited`
    /// keeps its `:link` half.
    Visited,
    /// `:dir(ltr)` / `:dir(rtl)` (Selectors 4 §7.1): the element's
    /// directionality ([`Dom::directionality`](crate::Dom::directionality),
    /// HTML §3.2.6.4) — not the CSS `direction` property. `None` for an
    /// argument other than `ltr` / `rtl`, which is valid and matches
    /// nothing.
    Dir(Option<crate::Directionality>),
    /// `:read-write` (Selectors 4 §14.3.1, HTML §4.16.3): a mutable
    /// `<input>` that `readonly` applies to, a mutable `<textarea>`, or
    /// any other element that is an editing host or editable
    /// ([`Dom::is_read_write`](crate::Dom::is_read_write)).
    ReadWrite,
    /// `:read-only`: every element that is not `:read-write`.
    ReadOnly,
    /// `:default` (Selectors 4 §14.4.2, HTML §4.16.3): a form's default
    /// button, a checkbox or radio that is checked by default, an
    /// `<option>` selected by default
    /// ([`Dom::is_default`](crate::Dom::is_default)).
    Default,
    /// `:in-range` (Selectors 4 §14.3.3): a candidate for constraint
    /// validation with range limitations and a value within them
    /// ([`Dom::range_state`](crate::Dom::range_state)).
    InRange,
    /// `:out-of-range` (Selectors 4 §14.3.4): one whose value is not.
    OutOfRange,
    /// `:user-valid` (Selectors 4 §14.4.4, HTML §4.16.3): an `<input>`,
    /// `<textarea>` or `<select>` the user has interacted with — its user
    /// validity is set — that satisfies its constraints
    /// ([`Dom::user_validity_state`](crate::Dom::user_validity_state)).
    UserValid,
    /// `:user-invalid` (Selectors 4 §14.4.5): one that does not.
    UserInvalid,
    /// `:modal` (Selectors 4 §11, HTML §4.16.3): a `<dialog>` shown
    /// modally — in the top layer as a modal dialog
    /// ([`Dom::top_layer_kind`](crate::Dom::top_layer_kind)). rdom has no
    /// fullscreen.
    Modal,
    /// `:popover-open` (Selectors 4 §11, HTML §4.16.3): an element whose
    /// popover is showing — in the top layer as a popover.
    PopoverOpen,
    /// `:scope` (Selectors 4 §14.3) — the scoping root. The query
    /// methods set it as DOM §4.2.6 does: the node a
    /// [`query_selector_in`](crate::Dom::query_selector_in) /
    /// `query_selector_all_in` searches under, the element
    /// [`matches`](crate::Dom::matches) / [`closest`](crate::Dom::closest)
    /// is called on, and the document (rdom's root) for
    /// `Dom::query_selector` / `query_selector_all`. In the cascade it is
    /// an `@scope` rule's root (CSS Cascade 6 §2.5), through
    /// [`Dom::matches_list_in_scope`](crate::Dom::matches_list_in_scope);
    /// with no scoping root — [`Dom::matches_list`](crate::Dom::matches_list)
    /// — it is `:root`. A nesting selector `&` with no parent rule is
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

mod anb;
#[cfg(test)]
mod depth_tests;
mod limits;
mod nesting;
mod parser;
mod pseudo_parser;
#[cfg(test)]
mod tests;

pub use limits::{MAX_SELECTOR_NESTING, MAX_SELECTOR_SIZE};
pub use nesting::{parse_nested, parse_scoped};
pub use parser::parse;

/// The name of every pseudo-class the selector parser makes a
/// [`PseudoClass`] of, as written after the colon — `dir` for the
/// functional `:dir()` — in no particular order. The parser reads its
/// keyword pseudo-classes from the same table, so a pseudo-class it
/// accepts is listed here: for tooling that offers or checks
/// pseudo-classes. The other functional ones (`:not()`, `:is()`,
/// `:where()`, `:has()`, `:nth-*()`, `:lang()`) are not listed.
pub fn pseudo_class_names() -> impl Iterator<Item = &'static str> {
    pseudo_parser::KEYWORD_PSEUDO_CLASSES
        .iter()
        .map(|(name, _)| *name)
        .chain(std::iter::once("dir"))
}
