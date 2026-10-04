//! Pseudo-element `content` property resolution.
//!
//! Distinguishes three author states:
//! - No declaration at any layer → caller falls back to legacy
//!   `before_content` / `after_content`.
//! - `content: none;` declared → caller must NOT fall back; pseudo
//!   stays empty.
//! - Concrete string (literal / `var()` / `concat`) → caller renders.

use rdom_style::ContentContext;

use super::ladder::{Declarations, Plan, Rollback, Step};
use crate::style::{ComputedStyle, Content, ImportantMask, Value};

/// The cascade's view of an element for `content` resolution: the
/// working style's variable map, the host's attributes, the counter
/// state at this point of the walk.
pub(super) struct ElementContext<'a> {
    pub vars: &'a std::collections::HashMap<String, String>,
    pub attr: &'a dyn Fn(&str) -> Option<String>,
    pub counter: &'a dyn Fn(&str) -> i32,
}

impl ContentContext for ElementContext<'_> {
    fn var(&self, name: &str) -> Option<String> {
        self.vars.get(name).cloned()
    }
    fn attr(&self, name: &str) -> Option<String> {
        (self.attr)(name)
    }
    fn counter(&self, name: &str) -> i32 {
        (self.counter)(name)
    }
}

/// Resolve the `content` property through the cascade ladder (`plan`
/// over `decls`, the same steps every other property takes), and
/// return:
///
/// - `None` — no declaration at any layer. Caller uses fallback.
/// - `Some(None)` — declaration resolved to `Content::None`
///   (suppression). Caller must NOT fall back.
/// - `Some(Some(s))` — declaration resolved to a concrete string.
///
/// `attr_lookup` is called for every `Content::Attr(name)` reference —
/// callers pass a closure that reads from the host element's
/// attributes (`dom.node(id).get_attribute(name)`).
pub(super) fn resolve_content_on(
    working: &ComputedStyle,
    plan: &Plan,
    decls: Declarations<'_>,
    attr_lookup: &dyn Fn(&str) -> Option<String>,
    counter_lookup: &dyn Fn(&str) -> i32,
) -> Option<Option<String>> {
    let base = || None;
    let apply = |declared: &mut Option<Content>, i: usize, rollback: &Rollback<'_, _>| {
        declare_step(declared, &plan.steps()[i], decls, rollback);
    };
    let rollback = Rollback::new(plan.steps().len(), &base, &apply);
    let mut declared: Option<Content> = None;
    for step in plan.steps() {
        declare_step(&mut declared, step, decls, &rollback);
    }

    let ctx = ElementContext {
        vars: &working.vars,
        attr: attr_lookup,
        counter: counter_lookup,
    };
    declared.map(|c| c.resolve(&ctx))
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
            };
        }
    }
}
