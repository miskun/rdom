//! The `content` value of `::before` / `::after` and what resolving it
//! reads from its element.

/// Content for `::before` / `::after` — literal, variable reference,
/// counter, or concatenation.
///
/// - `Content::Str("▾")` — straight string.
/// - `Content::Var("arrow")` — looked up in `ComputedStyle.vars`
///   (CSS custom properties: `content: var(--arrow)`).
/// - `Content::Concat(vec![Content::Str("▾ "), Content::Var("label")])` —
///   concatenation. Nests arbitrarily.
/// - `Content::None` — explicit "no content"; the pseudo-element does
///   not render at all (matches CSS `content: none;`).
///
/// Resolution reads the element's context through [`ContentContext`].
/// CSS `content: attr(label)` is an `attr()` substitution (CSS Values 5
/// §8.7), which the cascade makes before parsing the value: the host's
/// attribute arrives as a string, so there is no attribute variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Content {
    Str(String),
    Var(String),
    /// `counter(name[, style])` — the innermost counter of that name
    /// in scope at the pseudo-element (CSS Lists 3 §3.2).
    Counter {
        name: String,
        style: crate::counters::CounterStyle,
    },
    Concat(Vec<Content>),
    None,
}

/// What `content` resolution needs from the element it is generated
/// for: its custom properties and its counters. The
/// cascade implements this over its working state; a bare
/// `HashMap<String, CustomValue>` (a [`VarMap`](crate::VarMap)'s map) implements it as "variables only" for
/// callers without an element.
pub trait ContentContext {
    /// The value of custom property `--name` (without the dashes).
    fn var(&self, name: &str) -> Option<String>;
    /// The innermost counter `name` in scope (0 when there is none).
    fn counter(&self, name: &str) -> i32;
}

impl ContentContext for std::collections::HashMap<String, crate::CustomValue> {
    fn var(&self, name: &str) -> Option<String> {
        self.get(name).map(|v| v.as_str().to_string())
    }
    fn counter(&self, _name: &str) -> i32 {
        0
    }
}

impl Content {
    /// Resolve `Var(...)` and `Counter { .. }` against `ctx`; join
    /// `Concat` parts. Returns `None` if the result should cause the
    /// pseudo-element to be skipped entirely (`Content::None`).
    ///
    /// An unresolved var yields the empty string rather than failing.
    pub fn resolve(&self, ctx: &impl ContentContext) -> Option<String> {
        match self {
            Content::None => None,
            Content::Str(s) => Some(s.clone()),
            Content::Var(name) => Some(ctx.var(name).unwrap_or_default()),
            Content::Counter { name, style } => Some(style.format(ctx.counter(name))),
            Content::Concat(parts) => {
                let mut out = String::new();
                for p in parts {
                    if let Some(s) = p.resolve(ctx) {
                        out.push_str(&s);
                    }
                    // Content::None inside a concat contributes nothing.
                }
                Some(out)
            }
        }
    }

    /// Does this value (or any nested part) read a counter?
    pub fn uses_counters(&self) -> bool {
        match self {
            Content::Counter { .. } => true,
            Content::Concat(parts) => parts.iter().any(Content::uses_counters),
            _ => false,
        }
    }
}
