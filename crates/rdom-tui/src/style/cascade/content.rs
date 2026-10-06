//! Pseudo-element `content` property resolution.
//!
//! Distinguishes three author states:
//! - No declaration at any layer → caller falls back to legacy
//!   `before_content` / `after_content`.
//! - `content: none;` declared → caller must NOT fall back; pseudo
//!   stays empty.
//! - A `<content-list>` (CSS Generated Content 3 §2) → caller renders
//!   its text; its alt text and `<quote>` items are kept beside it.

use rdom_style::{ContentContext, QuoteKind};

use super::counters::CounterState;
use super::ladder::{Declarations, Plan, Rollback, Step};
use crate::style::{ComputedStyle, Content, ImportantMask, Value};

/// The cascade's view of a box for `content` resolution: the working
/// style's variable map and the counter state at this point of the walk.
/// (`attr()` was substituted before the value was parsed.)
pub(super) struct ElementContext<'a> {
    pub vars: &'a std::collections::HashMap<String, rdom_style::CustomValue>,
    pub counters: &'a CounterState,
    /// The box's computed `quotes` and its content language, which
    /// `quotes: auto` reads (§2.1).
    pub quotes: &'a rdom_style::Quotes,
    /// The counter styles names resolve to: the sheets' `@counter-style`
    /// rules over the predefined ones.
    pub styles: &'a rdom_style::counters::CounterStyleRegistry,
    pub lang: Option<&'a str>,
    /// The box's `direction` is `rtl` (`disclosure-closed` points along
    /// it, CSS Counter Styles 3 §6.3).
    pub rtl: bool,
    /// The box generates its content — a `::before` / `::after` — so its
    /// `<quote>` items take part in the quote depth (CSS Generated
    /// Content 3 §2.2). An element's own `content` generates nothing
    /// (no engine replaces an element's children with a `<content-list>`,
    /// DIVERGENCES §2): its quotes are empty and move no depth.
    pub generates: bool,
}

impl ContentContext for ElementContext<'_> {
    fn var(&self, name: &str) -> Option<String> {
        self.vars.get(name).map(|v| v.as_str().to_string())
    }
    fn counter(&self, name: &str) -> i32 {
        self.counters.value(name)
    }
    fn counters(&self, name: &str) -> Vec<i32> {
        self.counters.values(name)
    }
    fn format_counter(&self, value: i32, style: &rdom_style::CounterStyle) -> String {
        style.format_with(value, self.rtl, self.styles)
    }
    fn quote(&self, kind: QuoteKind) -> String {
        if !self.generates {
            return String::new();
        }
        let pair = |level| self.quotes.pair(level, self.lang);
        self.counters.quote(kind, pair).to_string()
    }
}

/// Resolve the `content` property through the cascade ladder (`plan`
/// over `decls`, the same steps every other property takes), and
/// return the declared value — `None` when no layer declares it (the
/// caller falls back), `Some(Content::None)` for `content: none`.
pub(super) fn declared_content(plan: &Plan, decls: Declarations<'_>) -> Option<Content> {
    let base = || None;
    let apply = |declared: &mut Option<Content>, i: usize, rollback: &Rollback<'_, _>| {
        declare_step(declared, &plan.steps()[i], decls, rollback);
    };
    let rollback = Rollback::new(plan.steps().len(), &base, &apply);
    let mut declared: Option<Content> = None;
    for step in plan.steps() {
        declare_step(&mut declared, step, decls, &rollback);
    }
    declared
}

/// Resolve `declared` onto `working`: its `content` text, its alt text
/// and — for a box that `generates` — its `<quote>` items, moving the
/// quote depth in `counters` as it goes. `lang` is the content language
/// (`quotes: auto` reads it).
pub(super) fn resolve_onto(
    working: &mut ComputedStyle,
    declared: &Content,
    counters: &CounterState,
    generates: bool,
    lang: Option<&str>,
    styles: &rdom_style::counters::CounterStyleRegistry,
) {
    let ctx = ElementContext {
        vars: &working.vars,
        counters,
        quotes: &working.quotes,
        styles,
        lang,
        rtl: working.text_direction == crate::layout::TextDirection::Rtl,
        generates,
    };
    let content = declared.resolve(&ctx);
    let alt = declared.resolve_alt(&ctx);
    working.content = content;
    working.content_alt = alt;
    working.content_quotes = if generates {
        declared.quotes()
    } else {
        Vec::new()
    };
}

/// One ladder step's `content` declarations onto `declared`.
fn declare_step(
    declared: &mut Option<Content>,
    step: &Step,
    decls: Declarations<'_>,
    rollback: &Rollback<'_, Option<Content>>,
) {
    for style in decls.of(step) {
        if let Some(v) = &style.content
            && style.important.contains(ImportantMask::CONTENT) == step.important
        {
            *declared = match v {
                Value::Specified(c) => Some(c.clone()),
                Value::Inherit => declared.clone(),
                Value::Initial => Some(Content::None),
                Value::Revert => rollback.state_before(step.revert_to).clone(),
                Value::RevertLayer => rollback.state_before(step.revert_layer_to).clone(),
            };
        }
    }
}
