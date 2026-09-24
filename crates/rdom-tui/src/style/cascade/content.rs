//! Pseudo-element `content` property resolution.
//!
//! Distinguishes three author states:
//! - No declaration at any layer → caller falls back to legacy
//!   `before_content` / `after_content`.
//! - `content: none;` declared → caller must NOT fall back; pseudo
//!   stays empty.
//! - Concrete string (literal / `var()` / `concat`) → caller renders.

use rdom_style::ContentContext;

use crate::style::{ComputedStyle, Content, ImportantMask, Rule, RuleOrigin, TuiStyle, Value};

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

/// Resolve the `content` property against a sorted rule list + inline
/// style. Walks the origin + importance ladder (UA → Author → Inline,
/// then the Important inverse), and returns:
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
    sorted_by_spec: &[&Rule],
    inline: Option<&TuiStyle>,
    attr_lookup: &dyn Fn(&str) -> Option<String>,
    counter_lookup: &dyn Fn(&str) -> i32,
) -> Option<Option<String>> {
    let mut declared: Option<Content> = None;
    let mut apply_from = |style: &TuiStyle, important_prop_match: bool| {
        if let Some(v) = &style.content {
            let is_imp = style.important.contains(ImportantMask::CONTENT);
            if is_imp == important_prop_match {
                declared = match v {
                    Value::Specified(c) => Some(c.clone()),
                    Value::Inherit => declared.clone(),
                    Value::Initial => Some(Content::None),
                };
            }
        }
    };

    // Normal pass (UA → Author → Inline).
    for r in sorted_by_spec {
        if r.origin == RuleOrigin::UserAgent {
            apply_from(&r.style, false);
        }
    }
    for r in sorted_by_spec {
        if r.origin == RuleOrigin::Author {
            apply_from(&r.style, false);
        }
    }
    if let Some(s) = inline {
        apply_from(s, false);
    }
    // Important pass (Inline → Author → UA, inverting origin priority).
    if let Some(s) = inline {
        apply_from(s, true);
    }
    for r in sorted_by_spec {
        if r.origin == RuleOrigin::Author {
            apply_from(&r.style, true);
        }
    }
    for r in sorted_by_spec {
        if r.origin == RuleOrigin::UserAgent {
            apply_from(&r.style, true);
        }
    }

    let ctx = ElementContext {
        vars: &working.vars,
        attr: attr_lookup,
        counter: counter_lookup,
    };
    declared.map(|c| c.resolve(&ctx))
}
