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
    /// The UA rules live in [`crate::ua`] — see
    /// [`crate::ua::user_agent_defaults`] for the full slice.
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
            sheet.rules.extend(rule);
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
        self.rules.extend(new_rules);
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
        self.rules.extend(new_rules);
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

// ── Pseudo-element suffix extraction ────────────────────────────────

/// Strip a trailing `::before` / `::after` if present. Returns the
/// core selector + pseudo target. Errors if multiple pseudo-element
/// suffixes are present (not allowed in a single selector).
fn extract_pseudo_suffix(selector: &str) -> Result<(&str, PseudoElementTarget), String> {
    // The rule: `::before` / `::after` must appear at the END of the
    // selector, directly attached to the last compound (no whitespace
    // between). CSS `element::before` and `element::after` are the only
    // forms we accept.
    let s = selector.trim_end();

    // Disallow multiple `::` pseudo-elements (`::before::after` is invalid).
    let pseudo_count = s.matches("::").count();
    if pseudo_count > 1 {
        return Err(format!(
            "at most one pseudo-element suffix allowed per selector, found {pseudo_count}"
        ));
    }

    if let Some(core) = s.strip_suffix("::before") {
        let core = core.trim_end();
        if core.is_empty() {
            return Err("`::before` requires a host selector".to_string());
        }
        return Ok((core, PseudoElementTarget::Before));
    }
    if let Some(core) = s.strip_suffix("::after") {
        let core = core.trim_end();
        if core.is_empty() {
            return Err("`::after` requires a host selector".to_string());
        }
        return Ok((core, PseudoElementTarget::After));
    }
    if let Some(core) = s.strip_suffix("::backdrop") {
        let core = core.trim_end();
        if core.is_empty() {
            return Err("`::backdrop` requires a host selector".to_string());
        }
        return Ok((core, PseudoElementTarget::Backdrop));
    }
    if let Some(core) = s.strip_suffix("::selection") {
        let core = core.trim_end();
        if core.is_empty() {
            return Err("`::selection` requires a host selector".to_string());
        }
        return Ok((core, PseudoElementTarget::Selection));
    }
    // Note: `::scrollbar-thumb` must be checked BEFORE `::scrollbar`
    // because the latter is a prefix of the former; otherwise
    // `::scrollbar-thumb` would split as `::scrollbar` + dangling
    // `-thumb`.
    if let Some(core) = s.strip_suffix("::scrollbar-thumb") {
        let core = core.trim_end();
        if core.is_empty() {
            return Err("`::scrollbar-thumb` requires a host selector".to_string());
        }
        return Ok((core, PseudoElementTarget::ScrollbarThumb));
    }
    if let Some(core) = s.strip_suffix("::scrollbar") {
        let core = core.trim_end();
        if core.is_empty() {
            return Err("`::scrollbar` requires a host selector".to_string());
        }
        return Ok((core, PseudoElementTarget::Scrollbar));
    }

    // A bare `::other` anywhere is rejected (unsupported pseudo-element).
    if pseudo_count == 1 {
        return Err(
            "unsupported pseudo-element; only ::before, ::after, ::backdrop, ::selection, ::scrollbar, ::scrollbar-thumb allowed"
                .to_string(),
        );
    }

    Ok((s, PseudoElementTarget::None))
}

// ── Top-level comma splitting ───────────────────────────────────────

/// Split on commas that are NOT inside `(...)` or `[...]`. Returns
/// borrowed slices into `input`. Empty input yields empty vec.
///
/// `:not(a, b)` has one internal comma; we preserve it as part of the
/// single selector item because it sits inside parens.
fn split_top_level_commas(input: &str) -> Vec<&str> {
    if input.trim().is_empty() {
        return Vec::new();
    }
    let bytes = input.as_bytes();
    let mut depth_paren: i32 = 0;
    let mut depth_bracket: i32 = 0;
    let mut start = 0;
    let mut out = Vec::new();

    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'(' => depth_paren += 1,
            b')' => depth_paren = (depth_paren - 1).max(0),
            b'[' => depth_bracket += 1,
            b']' => depth_bracket = (depth_bracket - 1).max(0),
            b',' if depth_paren == 0 && depth_bracket == 0 => {
                out.push(&input[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&input[start..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, TuiStyle};

    // ── split_top_level_commas ───────────────────────────────────────

    #[test]
    fn split_empty() {
        assert!(split_top_level_commas("").is_empty());
        assert!(split_top_level_commas("   ").is_empty());
    }

    #[test]
    fn split_no_commas() {
        assert_eq!(split_top_level_commas("div.foo"), vec!["div.foo"]);
    }

    #[test]
    fn split_simple_list() {
        assert_eq!(split_top_level_commas("a, b, c"), vec!["a", " b", " c"]);
    }

    #[test]
    fn split_preserves_not_parens() {
        // :not(a, b) has an internal comma that must not split.
        assert_eq!(
            split_top_level_commas("div:not(a, b), span"),
            vec!["div:not(a, b)", " span"]
        );
    }

    #[test]
    fn split_preserves_attribute_brackets() {
        // Attribute values with quoted commas (rare but valid).
        assert_eq!(
            split_top_level_commas(r#"[data-x="a,b"], .foo"#),
            vec![r#"[data-x="a,b"]"#, " .foo"]
        );
    }

    // ── extract_pseudo_suffix ────────────────────────────────────────

    #[test]
    fn extract_no_pseudo() {
        assert_eq!(
            extract_pseudo_suffix("div.foo").unwrap(),
            ("div.foo", PseudoElementTarget::None)
        );
    }

    #[test]
    fn extract_before() {
        assert_eq!(
            extract_pseudo_suffix("tree-item::before").unwrap(),
            ("tree-item", PseudoElementTarget::Before)
        );
    }

    #[test]
    fn extract_after() {
        assert_eq!(
            extract_pseudo_suffix("dialog .close::after").unwrap(),
            ("dialog .close", PseudoElementTarget::After)
        );
    }

    #[test]
    fn extract_tolerates_trailing_whitespace() {
        assert_eq!(
            extract_pseudo_suffix("h1::before   ").unwrap(),
            ("h1", PseudoElementTarget::Before)
        );
    }

    #[test]
    fn extract_rejects_bare_pseudo() {
        assert!(extract_pseudo_suffix("::before").is_err());
        assert!(extract_pseudo_suffix("::after").is_err());
    }

    #[test]
    fn extract_rejects_unsupported_pseudo_element() {
        assert!(extract_pseudo_suffix("p::first-line").is_err());
    }

    #[test]
    fn extract_selection() {
        assert_eq!(
            extract_pseudo_suffix("p::selection").unwrap(),
            ("p", PseudoElementTarget::Selection)
        );
        assert_eq!(
            extract_pseudo_suffix("article .body::selection").unwrap(),
            ("article .body", PseudoElementTarget::Selection)
        );
    }

    #[test]
    fn extract_rejects_bare_selection() {
        assert!(extract_pseudo_suffix("::selection").is_err());
    }

    #[test]
    fn extract_rejects_multiple_pseudo_suffixes() {
        assert!(extract_pseudo_suffix("p::before::after").is_err());
    }

    #[test]
    fn extract_scrollbar() {
        // `::scrollbar-thumb` must split before `::scrollbar` — if
        // the parser stripped `::scrollbar` first the leftover
        // would be `-thumb` (invalid). Order matters.
        assert_eq!(
            extract_pseudo_suffix("*::scrollbar-thumb").unwrap(),
            ("*", PseudoElementTarget::ScrollbarThumb)
        );
        assert_eq!(
            extract_pseudo_suffix("*::scrollbar").unwrap(),
            ("*", PseudoElementTarget::Scrollbar)
        );
        assert_eq!(
            extract_pseudo_suffix(".sidebar::scrollbar-thumb").unwrap(),
            (".sidebar", PseudoElementTarget::ScrollbarThumb)
        );
    }

    #[test]
    fn extract_rejects_bare_scrollbar() {
        assert!(extract_pseudo_suffix("::scrollbar").is_err());
        assert!(extract_pseudo_suffix("::scrollbar-thumb").is_err());
    }

    // ── Stylesheet builder ───────────────────────────────────────────

    #[test]
    fn bare_has_no_rules() {
        assert_eq!(Stylesheet::bare().rules().len(), 0);
    }

    #[test]
    fn rule_adds_author_rule() {
        let s = Stylesheet::bare()
            .rule("div.hero", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
            .unwrap();
        assert_eq!(s.rules().len(), 1);
        assert_eq!(s.rules()[0].origin, RuleOrigin::Author);
        assert_eq!(s.rules()[0].source_text, "div.hero");
    }

    #[test]
    fn rule_computes_specificity() {
        let s = Stylesheet::bare()
            .rule("#main", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
            .unwrap()
            .rule("div", TuiStyle::new().fg(Color::Rgb(0, 0, 255)))
            .unwrap();
        assert!(s.rules()[0].specificity > s.rules()[1].specificity);
    }

    #[test]
    fn rule_assigns_monotonic_source_idx() {
        let s = Stylesheet::bare()
            .rule("a", TuiStyle::new())
            .unwrap()
            .rule("b", TuiStyle::new())
            .unwrap()
            .rule("c", TuiStyle::new())
            .unwrap();
        let idxs: Vec<u32> = s.rules().iter().map(|r| r.source_idx).collect();
        assert_eq!(idxs, vec![0, 1, 2]);
    }

    #[test]
    fn rule_expands_selector_list() {
        let s = Stylesheet::bare()
            .rule("a, b, c.foo", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
            .unwrap();
        assert_eq!(s.rules().len(), 3);
        assert_eq!(s.rules()[0].source_text, "a");
        assert_eq!(s.rules()[1].source_text, "b");
        assert_eq!(s.rules()[2].source_text, "c.foo");
        // Each one parses as a single complex selector, so specificities differ.
        // a + b are equal (type 1 each); c.foo is higher.
        assert_eq!(s.rules()[0].specificity, s.rules()[1].specificity);
        assert!(s.rules()[2].specificity > s.rules()[0].specificity);
    }

    #[test]
    fn rule_handles_pseudo_element() {
        let s = Stylesheet::bare()
            .rule(
                "tree-item::before",
                TuiStyle::new().fg(Color::Rgb(255, 0, 0)),
            )
            .unwrap();
        assert_eq!(s.rules()[0].pseudo, PseudoElementTarget::Before);
        // source_text preserves the full selector for debug/devtools —
        // the pseudo suffix has already been handled by `pseudo`.
        assert_eq!(s.rules()[0].source_text, "tree-item::before");
    }

    #[test]
    fn rule_handles_mixed_pseudo_list() {
        let s = Stylesheet::bare()
            .rule(
                "a, b::before, c::after",
                TuiStyle::new().fg(Color::Rgb(255, 0, 0)),
            )
            .unwrap();
        assert_eq!(s.rules().len(), 3);
        assert_eq!(s.rules()[0].pseudo, PseudoElementTarget::None);
        assert_eq!(s.rules()[1].pseudo, PseudoElementTarget::Before);
        assert_eq!(s.rules()[2].pseudo, PseudoElementTarget::After);
    }

    #[test]
    fn rule_pseudo_specificity_adds_type_bump() {
        let s = Stylesheet::bare()
            .rule("a", TuiStyle::new())
            .unwrap()
            .rule("a::before", TuiStyle::new())
            .unwrap();
        // Both share the `a` type count; `a::before` has one more.
        assert!(s.rules()[1].specificity > s.rules()[0].specificity);
    }

    #[test]
    fn rule_rejects_invalid_selector() {
        let err = Stylesheet::bare()
            .rule("div..foo", TuiStyle::new())
            .unwrap_err();
        assert!(err.msg.contains("empty class selector") || err.msg.contains("class"));
        assert_eq!(err.source, "div..foo");
    }

    #[test]
    fn rule_rejects_empty_selector_string() {
        assert!(Stylesheet::bare().rule("", TuiStyle::new()).is_err());
        assert!(Stylesheet::bare().rule("   ", TuiStyle::new()).is_err());
    }

    #[test]
    fn rule_rejects_empty_item_in_list() {
        assert!(Stylesheet::bare().rule("a, , b", TuiStyle::new()).is_err());
    }

    #[test]
    fn rule_unchecked_panics_on_error() {
        let result = std::panic::catch_unwind(|| {
            Stylesheet::bare().rule_unchecked("div..foo", TuiStyle::new())
        });
        assert!(result.is_err());
    }

    #[test]
    fn rule_unchecked_succeeds_on_valid() {
        let s = Stylesheet::bare().rule_unchecked("div", TuiStyle::new());
        assert_eq!(s.rules().len(), 1);
    }

    #[test]
    fn define_var_stores_value() {
        let s = Stylesheet::bare()
            .define_var("accent", "#ff0000")
            .define_var("muted", "#888");
        assert_eq!(s.var("accent"), Some("#ff0000"));
        assert_eq!(s.var("muted"), Some("#888"));
        assert!(s.var("nope").is_none());
    }

    #[test]
    fn define_var_overwrites() {
        let s = Stylesheet::bare().define_var("x", "1").define_var("x", "2");
        assert_eq!(s.var("x"), Some("2"));
    }

    #[test]
    fn define_var_mut_accumulates_in_a_loop() {
        // The borrow-by-value fluent `define_var` forces consumers to
        // either chain inline or `mem::take` the sheet to pass it
        // through. `define_var_mut` takes `&mut self` and returns
        // `&mut Self` so accumulation in a loop Just Works.
        let mut s = Stylesheet::bare();
        for (name, value) in [("a", "#111"), ("b", "#222"), ("c", "#333")] {
            s.define_var_mut(name, value);
        }
        assert_eq!(s.var("a"), Some("#111"));
        assert_eq!(s.var("b"), Some("#222"));
        assert_eq!(s.var("c"), Some("#333"));
    }

    #[test]
    fn define_var_mut_chains_via_returned_ref() {
        let mut s = Stylesheet::bare();
        s.define_var_mut("a", "1")
            .define_var_mut("b", "2")
            .define_var_mut("c", "3");
        assert_eq!(s.var("a"), Some("1"));
        assert_eq!(s.var("b"), Some("2"));
        assert_eq!(s.var("c"), Some("3"));
    }

    #[test]
    fn define_var_mut_overwrites_like_define_var() {
        let mut s = Stylesheet::bare();
        s.define_var_mut("x", "1");
        s.define_var_mut("x", "2");
        assert_eq!(s.var("x"), Some("2"));
    }

    // ── Cross-builder scenarios ──────────────────────────────────────

    #[test]
    fn ua_rule_comes_before_author_rule_in_source_order() {
        let s = Stylesheet::new()
            .rule(".my-button", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
            .unwrap();
        // All UA rules precede any author rule. Phase E extended the
        // UA default set, so the author rule lands at the tail.
        let last = s.rules().last().unwrap();
        assert_eq!(last.origin, RuleOrigin::Author);
        for r in s.rules().iter().take(s.rules().len() - 1) {
            assert_eq!(r.origin, RuleOrigin::UserAgent);
        }
    }

    #[test]
    fn complex_selector_preserved_in_rule() {
        let s = Stylesheet::bare()
            .rule("div.a > span", TuiStyle::new())
            .unwrap();
        let complex = &s.rules()[0].selector.0[0];
        // Subject is span; one ancestor (div.a with Child combinator)
        assert_eq!(complex.ancestors.len(), 1);
    }

    #[test]
    fn not_with_internal_comma_not_split() {
        let s = Stylesheet::bare()
            .rule(":not(a, b)", TuiStyle::new())
            .unwrap();
        // One rule (not two).
        assert_eq!(s.rules().len(), 1);
    }

    #[test]
    fn source_idx_continues_across_multiple_rule_calls_with_lists() {
        let s = Stylesheet::bare()
            .rule("a, b", TuiStyle::new()) // idxs 0, 1
            .unwrap()
            .rule("c, d, e", TuiStyle::new()) // idxs 2, 3, 4
            .unwrap();
        let idxs: Vec<u32> = s.rules().iter().map(|r| r.source_idx).collect();
        assert_eq!(idxs, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn ua_rule_specificity_is_tiny() {
        let s = Stylesheet::new();
        let ua = &s.rules()[0];
        // [disabled] is 1 class-level, 0 ids, 0 types.
        assert_eq!(ua.specificity.id, 0);
        assert_eq!(ua.specificity.class_attr_pseudo, 1);
        assert_eq!(ua.specificity.type_pseudo_el, 0);
    }

    #[test]
    fn default_is_empty() {
        // Default::default is the `bare` constructor, not `new`.
        assert_eq!(Stylesheet::default().rules().len(), 0);
    }

    #[test]
    fn error_display_shows_position() {
        let err = Stylesheet::bare()
            .rule("div..foo", TuiStyle::new())
            .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("div..foo"));
    }
}
