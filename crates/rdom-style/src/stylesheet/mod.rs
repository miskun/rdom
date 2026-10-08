//! `Stylesheet` — a parsed, specificity-tagged collection of rules.
//!
//! Each author-written string selector is parsed *once* at stylesheet
//! build time via `rdom_core::selectors::parse`, yielding a `SelectorList`
//! AST. Pseudo-element suffixes (`::before` / `::after`) are stripped
//! before parsing — rdom-core's selector grammar rejects pseudo-elements
//! because they're presentational, not structural.
//!
//! A selector list with mixed pseudos (`"a, b::before"`) expands into
//! multiple `Rule` entries, one per list item, each with its own
//! specificity and pseudo-element target. This matches CSS: a list
//! is syntactic sugar for repeating the same declaration block.
//!
//! ## Rule order
//!
//! Rules are stored in a flat `Vec<Rule>`. `source_idx` is a monotonic
//! counter assigned at insertion — used by the cascade as the
//! tie-breaker after specificity. Origin + idx together give a total
//! order:
//!
//! 1. UA rules (origin = `UserAgent`) come first — baked in by
//!    `Stylesheet::new()` so authors can always override them.
//! 2. Author rules (origin = `Author`) come next, in the order `rule()`
//!    was called.
//! 3. Inline style is not a rule — it's applied separately by the
//!    cascade with `Specificity::INLINE`.
//!
//! Author rules may sit in cascade layers (`@layer`, CSS Cascade 5
//! §6.4): each sheet records the layers it declares and each rule its
//! layer; [`LayerOrder`] merges the layers of the sheets of one cascade
//! into one order (`layers.rs`).
//!
//! ## Errors
//!
//! `Stylesheet::rule()` returns `Result<Self, StyleError>`. Parse errors
//! are reported with position (byte offset into the selector string) and
//! a human-readable message. `rule_unchecked()` panics on the same errors
//! — convenient for tests and compile-time-known selectors.

use std::fmt;

use rdom_core::selectors::{ParseError, SelectorList};

use crate::{Specificity, TuiStyle};

mod conditions;
mod counter_styles;
mod imports;
mod index;
mod keyframes;
mod layers;
mod registrations;
mod scopes;
mod selector_text;
mod style_selector;
#[cfg(test)]
mod tests;
mod user_action;
#[cfg(test)]
mod user_action_tests;
mod version;

pub use conditions::{ConditionId, ConditionKind, ConditionRule};
pub use imports::Import;
pub use index::RuleIndex;
pub use layers::{Layer, LayerId, LayerOrder};
pub use scopes::{Scope, ScopeId};
pub use style_selector::{RuleContext, StyleSelector};
pub use user_action::UserActionState;

/// Which pseudo-element a rule targets. `None` = the host element itself.
///
/// Not `Copy`: [`Highlight`](Self::Highlight) carries its name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PseudoElementTarget {
    None,
    Before,
    After,
    /// `::backdrop` — Polish #8. The overlay behind a modal
    /// `<dialog>`. Paints across the full viewport minus the
    /// dialog rect; only evaluated for elements that `dialog::
    /// is_modal` reports true on.
    Backdrop,
    /// `::selection` — author-controlled style applied to cells
    /// that fall inside the current text selection; a highlight
    /// pseudo-element, cut to [`TuiStyle::highlight_subset`]. Lookup at
    /// paint time walks up from each selected fragment's text
    /// node to the nearest ancestor with a cascaded selection
    /// style. The UA `*::selection { bg: #394B7E; fg: white }`
    /// rule provides the default; authors override per-element.
    Selection,
    /// `::scrollbar` — the scrollbar track. Author-overridable
    /// styling for the colored gutter that backs the thumb.
    /// `content` is the cell glyph (default `" "` — a colored
    /// gutter via `bg` is the modern look; an explicit `█` would
    /// give an ANSI-style fg-colored rail). `bg` / `fg` apply
    /// to every track cell. Matches WebKit's
    /// `::-webkit-scrollbar` model.
    Scrollbar,
    /// `::scrollbar-thumb` — the "you are here" handle on top
    /// of the track. `content` is the cell glyph (default `┃`).
    /// `bg` / `fg` apply to every thumb cell. The track shows
    /// through where the thumb glyph leaves cells transparent
    /// (e.g. `┃` paints a vertical bar in the center of the
    /// cell; the bg fills around it). Matches WebKit's
    /// `::-webkit-scrollbar-thumb`.
    ScrollbarThumb,
    /// `::scrollbar-thumb:vertical` — the vertical thumb only. Layers
    /// over `::scrollbar-thumb` (a matching axis rule wins ties), so
    /// `content: "═"` can be given to the horizontal bar alone.
    /// `UA-SB-1`; mirrors WebKit's `:vertical` / `:horizontal`.
    ScrollbarThumbVertical,
    /// `::scrollbar-thumb:horizontal` — the horizontal thumb only.
    ScrollbarThumbHorizontal,
    /// `::placeholder` (CSS Pseudo-Elements 4 §4.3) — the placeholder
    /// text of an `<input>` / `<textarea>` while it is
    /// `:placeholder-shown`. rdom paints that text as the host's
    /// generated `::before` box (UA `:placeholder-shown::before
    /// { content: attr(placeholder) }`), so the backend layers these
    /// rules over the `::before` rules of a host showing its
    /// placeholder. Only the `::first-line` property subset applies
    /// ([`TuiStyle::first_line_subset`]); a rule keeps nothing else.
    Placeholder,
    /// `::first-line` (CSS Pseudo-Elements 4 §2.2), also spelled
    /// `:first-line` (Selectors 4 §15): the first formatted line of a
    /// block container. Only the properties §2.2.1 lets apply are kept
    /// ([`TuiStyle::first_line_subset`]).
    FirstLine,
    /// `::first-letter` (CSS Pseudo-Elements 4 §2.3), also spelled
    /// `:first-letter` (Selectors 4 §15): the first typographic letter
    /// unit of a block container's first formatted line. Only the
    /// properties §2.3.1 lets apply are kept
    /// ([`TuiStyle::first_letter_subset`]).
    FirstLetter,
    /// `::marker` (CSS Pseudo-Elements 4 §3.1, CSS Lists 3 §3.2) — a list
    /// item's marker box. Only the properties §3.2 lets apply to it are
    /// kept ([`TuiStyle::marker_subset`]); a rule keeps nothing else.
    Marker,
    /// `::highlight(<name>)` (CSS Custom Highlight API 1 §5.1) — the
    /// ranges of the highlight registered as `name` (case-sensitive),
    /// over the originating element's text. A highlight pseudo-element:
    /// only the properties CSS Pseudo-Elements 4 §3.2 lets apply are kept
    /// ([`TuiStyle::highlight_subset`]), as for `::selection`.
    Highlight(std::sync::Arc<str>),
    /// `::details-content` (HTML §4.11.1, CSS Pseudo-Elements 4) — the
    /// slot holding a `<details>` element's content, everything but its
    /// first `<summary>` child: what the content inherits from, and what
    /// hides it while the element is closed.
    DetailsContent,
    /// `::before::marker` (CSS Pseudo-Elements 4 §4, CSS Lists 3 §3.1) —
    /// the marker of a `::before` that is a list item (`display:
    /// list-item`). Only the `::marker` properties apply, as for
    /// [`Marker`](Self::Marker).
    BeforeMarker,
    /// `::after::marker` — the marker of a list-item `::after`, as
    /// [`BeforeMarker`](Self::BeforeMarker).
    AfterMarker,
}

impl PseudoElementTarget {
    /// The targets whose rules style a given scrollbar-thumb axis, in
    /// layering order: the axis-neutral rules first, the axis rules
    /// on top.
    pub fn thumb_targets(vertical: bool) -> [PseudoElementTarget; 2] {
        if vertical {
            [
                PseudoElementTarget::ScrollbarThumb,
                PseudoElementTarget::ScrollbarThumbVertical,
            ]
        } else {
            [
                PseudoElementTarget::ScrollbarThumb,
                PseudoElementTarget::ScrollbarThumbHorizontal,
            ]
        }
    }
}

/// Whether a rule comes from the built-in defaults or from the author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RuleOrigin {
    /// Baked-in defaults like `[disabled] { dim: true; }`. Always sort
    /// first. Author rules with equal-or-greater specificity override.
    UserAgent,
    /// Author-written rule via `Stylesheet::rule()`.
    Author,
}

/// One cascade rule: a selector (AST), the style block, and its origin +
/// source position so the cascade can sort them deterministically.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Rule {
    /// Parsed selector AST. Each rule holds exactly one `ComplexSelector`
    /// inside the list — selector lists are flattened at parse time.
    pub selector: SelectorList,
    /// Pseudo-element this rule targets, if any.
    pub pseudo: PseudoElementTarget,
    /// The user-action pseudo-classes following the pseudo-element
    /// (`::before:hover`, Selectors 4 §3.6.3): the rule applies only
    /// while they all hold of the pseudo-element. Empty for most rules.
    pub pseudo_state: UserActionState,
    /// The author's declaration block.
    pub style: TuiStyle,
    /// Cached specificity for this (single-item) selector list.
    pub specificity: Specificity,
    /// UA vs Author.
    pub origin: RuleOrigin,
    /// Monotonic source order — the tiebreaker when specificity is equal.
    pub source_idx: u32,
    /// Original selector text (kept for debug / devtools / error messages).
    pub source_text: String,
    /// The cascade layer the rule sits in (`@layer`, CSS Cascade 5
    /// §6.4), as declared in its own sheet; `None` for an unlayered
    /// rule (and every UA rule).
    pub layer: Option<LayerId>,
    /// The innermost `@scope` the rule sits in (CSS Cascade 6 §2.5), as
    /// declared in its own sheet; `None` for an unscoped rule.
    pub scope: Option<ScopeId>,
    /// The rule sits in `@starting-style` (CSS Transitions 2 §3): it
    /// applies to an element's starting style only, which the cascade
    /// computes for an element with no before-change style.
    pub starting_style: bool,
    /// The innermost conditional group rule the rule sits in (`@media`,
    /// CSS Conditional 3 §2), as declared in its own sheet; `None` for an
    /// unconditional rule. The rule applies while it and every enclosing
    /// one hold.
    pub condition: Option<ConditionId>,
    /// `style`'s direction-mapped declarations replayed for each
    /// `direction`, built with the rule ([`Rule::directional_overlay`]).
    directional: Option<std::sync::Arc<[[TuiStyle; 2]; 2]>>,
}

impl Rule {
    /// The block's inline-axis flow-relative declarations (CSS Logical 1
    /// §4) — with the declarations after them, in source order — mapped
    /// for an element of `direction`, which the cascade applies after
    /// [`style`](Self::style): the normal ones, then the `!important`
    /// ones, each marking only the side it maps to (CSS Cascade 4 §6.4).
    /// Built once, when the rule is added, for a block whose kept
    /// declarations need no substitution (no `var()` or `attr()`:
    /// [`TuiStyle::needs_substitution`]), so the cascade parses nothing
    /// per element; `None` for any other block (a block with a
    /// substitution function replays per element).
    pub fn directional_overlay(
        &self,
        direction: crate::layout::TextDirection,
    ) -> Option<&[TuiStyle; 2]> {
        let [ltr, rtl] = self.directional.as_deref()?;
        Some(match direction {
            crate::layout::TextDirection::Rtl => rtl,
            _ => ltr,
        })
    }
}

/// Error produced while parsing a stylesheet rule.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct StyleError {
    /// Human-readable message.
    pub msg: String,
    /// Byte offset into the original selector string where the error
    /// was detected, if known.
    pub pos: Option<usize>,
    /// The full selector text that failed to parse.
    pub source: String,
}

impl fmt::Display for StyleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.pos {
            Some(p) => write!(
                f,
                "style error at byte {p} in `{}`: {}",
                self.source, self.msg
            ),
            None => write!(f, "style error in `{}`: {}", self.source, self.msg),
        }
    }
}

impl std::error::Error for StyleError {}

impl From<(&str, ParseError)> for StyleError {
    fn from((source, e): (&str, ParseError)) -> Self {
        Self {
            msg: e.msg,
            pos: Some(e.pos),
            source: source.to_string(),
        }
    }
}

/// A parsed, specificity-tagged collection of rules. Built with the
/// fluent `rule()` / `define_var()` API.
#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    rules: Vec<Rule>,
    /// Lazily built rightmost-selector index; reset whenever `rules`
    /// changes.
    index: std::cell::OnceCell<RuleIndex>,
    /// Next `source_idx` to assign.
    next_source_idx: u32,
    /// Custom-property (`--foo: bar;`) root values. `define_var` adds
    /// entries here; the cascade clones these into every
    /// `ComputedStyle.vars` via `root_vars_rc()` so `var(--foo)`
    /// references in rules resolve to concrete values.
    root_vars: std::collections::HashMap<String, crate::CustomValue>,
    /// Declared cascade layers, in order of first declaration
    /// (`layers.rs`).
    layers: Vec<Layer>,
    /// Registered custom properties (`@property`), in source order.
    registrations: Vec<crate::PropertyRegistration>,
    /// `@counter-style` definitions, in source order (`counter_styles.rs`).
    counter_styles: Vec<crate::counters::CounterStyleDefinition>,
    /// `@keyframes` rules, in source order (`keyframes.rs`).
    keyframes: Vec<crate::keyframes::KeyframesRule>,
    /// The `@import`s that loaded (`imports.rs`).
    imports: Vec<Import>,
    /// Declared `@scope` rules, in source order (`scopes.rs`).
    scopes: Vec<Scope>,
    /// Declared conditional group rules, in source order
    /// (`conditions.rs`).
    conditions: Vec<ConditionRule>,
    /// CSSOM `ownerNode`: the `<style>` element the sheet came from.
    owner_node: Option<rdom_core::NodeId>,
    /// Renewed by every mutation ([`Stylesheet::version`]).
    version: version::Version,
}

impl Stylesheet {
    /// Create a stylesheet with the baked-in UA defaults.
    ///
    /// The UA rules live in `crate::ua` — see
    /// `crate::ua::user_agent_defaults` for the full slice.
    ///
    /// Authors override any rule by writing a more-specific or
    /// `!important` rule; UA rules carry `RuleOrigin::UserAgent`
    /// and minimal specificity so any real selector beats them.
    pub fn new() -> Self {
        let mut sheet = Self::bare();
        for (selector, style) in crate::ua::user_agent_defaults() {
            let rule = sheet
                .build_rule(selector, style, RuleOrigin::UserAgent)
                .expect("UA rule must parse");
            sheet.push_rules(rule);
        }
        sheet
    }

    /// Create an empty stylesheet with NO UA defaults. Useful for tests
    /// and for callers who want full control over the rule set.
    pub fn bare() -> Self {
        Self::default()
    }

    /// Add a rule. Parses `selector` via `rdom_core::selectors`, strips
    /// any `::before` / `::after` suffix, computes specificity, assigns
    /// the next monotonic source index. Errors on malformed selectors.
    ///
    /// A selector list (`a, b.foo`) expands into one `Rule` per list item.
    pub fn rule(mut self, selector: &str, style: TuiStyle) -> Result<Self, StyleError> {
        let new_rules = self.build_rule(selector, style, RuleOrigin::Author)?;
        self.push_rules(new_rules);
        Ok(self)
    }

    /// Like `rule()` but panics on parse error. Convenient for tests and
    /// for selectors known at compile time.
    pub fn rule_unchecked(self, selector: &str, style: TuiStyle) -> Self {
        self.rule(selector, style).unwrap_or_else(|e| {
            panic!("rule_unchecked failed: {e}");
        })
    }

    /// Add a rule, mutating the stylesheet in place. Same semantics as
    /// `rule()` (selector parsing, list expansion, specificity, source
    /// indexing) but takes `&mut self` so callers that aggregate rules
    /// in a loop don't need to chain via the fluent builder. On parse
    /// error the stylesheet is left untouched.
    ///
    /// Used by `rdom-css` and any other accumulator-style consumer.
    pub fn add_rule(&mut self, selector: &str, style: TuiStyle) -> Result<(), StyleError> {
        let new_rules = self.build_rule(selector, style, RuleOrigin::Author)?;
        self.push_rules(new_rules);
        Ok(())
    }

    /// Define a root custom-property value: `value` is the property's
    /// token text, as `--name: value` would hold it. The cascade seeds
    /// every element's custom properties with it (the document root's
    /// value, inherited), so a `var(--name)` in any property substitutes
    /// it and parses the result with that property's grammar.
    pub fn define_var(mut self, name: &str, value: impl Into<crate::CustomValue>) -> Self {
        self.touch();
        self.root_vars.insert(name.to_string(), value.into());
        self
    }

    /// Like `define_var()` but takes `&mut self` so callers that
    /// accumulate vars in a loop don't need to chain via the fluent
    /// builder or `mem::take` the sheet. Returns `&mut Self` for
    /// fluent chaining when desired. Parity with the
    /// `add_rule` / `rule` split on the rule side.
    pub fn define_var_mut(
        &mut self,
        name: &str,
        value: impl Into<crate::CustomValue>,
    ) -> &mut Self {
        self.touch();
        self.root_vars.insert(name.to_string(), value.into());
        self
    }

    /// The only writer of `rules`: appends and drops the cached index.
    fn push_rules(&mut self, new_rules: impl IntoIterator<Item = Rule>) {
        self.touch();
        self.rules.extend(new_rules);
        self.index = std::cell::OnceCell::new();
    }

    /// Every mutation renews the version.
    fn touch(&mut self) {
        self.version = version::Version::next();
    }

    /// This sheet's content version: unique in the process and renewed by
    /// every mutation — a clone gets its own — so an unchanged version
    /// is the same sheet with the same content. A backend hook: the
    /// cascade keys what it builds from a sheet set by it.
    pub fn version(&self) -> u64 {
        self.version.get()
    }

    /// The rightmost-selector index for this sheet (built on first use).
    /// A backend hook: the cascade asks it for candidate rules; authors
    /// never need it.
    pub fn rule_index(&self) -> &RuleIndex {
        self.index.get_or_init(|| RuleIndex::build(&self.rules))
    }

    /// All rules in source order.
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn var(&self, name: &str) -> Option<&str> {
        self.root_vars.get(name).map(crate::CustomValue::as_str)
    }

    pub fn vars(&self) -> &std::collections::HashMap<String, crate::CustomValue> {
        &self.root_vars
    }

    /// Clone the root vars into a new `Rc<HashMap>` for sharing with
    /// every `ComputedStyle` during cascade. Each call allocates a
    /// fresh `Rc` — the cascade does it once per pass, then Rc::clone
    /// propagates it cheaply into each `ComputedStyle.vars`.
    pub fn root_vars_rc(&self) -> crate::VarMap {
        std::rc::Rc::new(self.root_vars.clone())
    }

    /// Build one or more `Rule`s from a raw selector string + style +
    /// origin: one per item of the selector list ([`StyleSelector`]).
    /// Does NOT append — the caller pushes them.
    fn build_rule(
        &mut self,
        selector: &str,
        style: TuiStyle,
        origin: RuleOrigin,
    ) -> Result<Vec<Rule>, StyleError> {
        let parsed = StyleSelector::parse(selector)?;
        Ok(self.rules_for(&parsed, style, origin))
    }
}
