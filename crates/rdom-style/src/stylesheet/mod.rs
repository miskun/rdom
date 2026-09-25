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
//! ## Errors
//!
//! `Stylesheet::rule()` returns `Result<Self, StyleError>`. Parse errors
//! are reported with position (byte offset into the selector string) and
//! a human-readable message. `rule_unchecked()` panics on the same errors
//! — convenient for tests and compile-time-known selectors.

use std::fmt;

use rdom_core::selectors::{self, ParseError, SelectorList};

use crate::{Specificity, TuiStyle};

mod index;
mod selector_text;
#[cfg(test)]
mod tests;

pub use index::RuleIndex;
use selector_text::{extract_pseudo_suffix, split_top_level_commas};

/// Which pseudo-element a rule targets. `None` = the host element itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// that fall inside the current text selection. Lookup at
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
pub struct Rule {
    /// Parsed selector AST. Each rule holds exactly one `ComplexSelector`
    /// inside the list — selector lists are flattened at parse time.
    pub selector: SelectorList,
    /// Pseudo-element this rule targets, if any.
    pub pseudo: PseudoElementTarget,
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
}

/// Error produced while parsing a stylesheet rule.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    root_vars: std::collections::HashMap<String, String>,
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

    /// Define a root custom-property value. Used by `var(--name)`
    /// references in rules; resolved during cascade against the string
    /// color grammar (hex, named, indexed).
    pub fn define_var(mut self, name: &str, value: &str) -> Self {
        self.root_vars.insert(name.to_string(), value.to_string());
        self
    }

    /// Like `define_var()` but takes `&mut self` so callers that
    /// accumulate vars in a loop don't need to chain via the fluent
    /// builder or `mem::take` the sheet. Returns `&mut Self` for
    /// fluent chaining when desired. Parity with the
    /// `add_rule` / `rule` split on the rule side.
    pub fn define_var_mut(&mut self, name: &str, value: &str) -> &mut Self {
        self.root_vars.insert(name.to_string(), value.to_string());
        self
    }

    /// The only writer of `rules`: appends and drops the cached index.
    fn push_rules(&mut self, new_rules: impl IntoIterator<Item = Rule>) {
        self.rules.extend(new_rules);
        self.index = std::cell::OnceCell::new();
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
        self.root_vars.get(name).map(String::as_str)
    }

    pub fn vars(&self) -> &std::collections::HashMap<String, String> {
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
    /// origin. Handles top-level comma splitting and pseudo-element
    /// extraction. Does NOT append — caller extends `self.rules`.
    fn build_rule(
        &mut self,
        selector: &str,
        style: TuiStyle,
        origin: RuleOrigin,
    ) -> Result<Vec<Rule>, StyleError> {
        let items = split_top_level_commas(selector);
        if items.is_empty() {
            return Err(StyleError {
                msg: "empty selector".to_string(),
                pos: None,
                source: selector.to_string(),
            });
        }

        let mut out = Vec::with_capacity(items.len());
        for item_raw in &items {
            let trimmed = item_raw.trim();
            if trimmed.is_empty() {
                return Err(StyleError {
                    msg: "empty selector in list".to_string(),
                    pos: None,
                    source: selector.to_string(),
                });
            }
            let (core, pseudo) = extract_pseudo_suffix(trimmed).map_err(|msg| StyleError {
                msg,
                pos: None,
                source: selector.to_string(),
            })?;

            let pseudo_count = if pseudo == PseudoElementTarget::None {
                0
            } else {
                1
            };
            let parsed: SelectorList =
                selectors::parse(core).map_err(|e| StyleError::from((selector, e)))?;

            // Each parsed list should have exactly one ComplexSelector
            // because we already split on top-level commas. But be robust:
            // if rdom-core returns multiple (shouldn't), produce multiple
            // rules with the same pseudo.
            for complex in parsed.0 {
                let specificity = Specificity::of_complex(&complex, pseudo_count);
                let single_list = SelectorList(vec![complex]);
                let idx = self.next_source_idx;
                self.next_source_idx += 1;
                out.push(Rule {
                    selector: single_list,
                    pseudo,
                    style: style.clone(),
                    specificity,
                    origin,
                    source_idx: idx,
                    source_text: trimmed.to_string(),
                });
            }
        }
        Ok(out)
    }
}
