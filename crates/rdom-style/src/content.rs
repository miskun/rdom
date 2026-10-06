//! The `content` value of `::before` / `::after` and what resolving it
//! reads from its element (CSS Generated Content 3 §2).

/// One `content` value — the `<content-list>` items rdom renders, their
/// concatenation, and the alt text (CSS Generated Content 3 §2).
///
/// - `Content::Str("▾")` — straight string.
/// - `Content::Var("arrow")` — looked up in `ComputedStyle.vars`
///   (CSS custom properties: `content: var(--arrow)`).
/// - `Content::Counter` / `Content::Counters` — `counter()` /
///   `counters()` (CSS Lists 3 §4.3).
/// - `Content::Quote` — `open-quote`, `close-quote`, `no-open-quote`,
///   `no-close-quote` (§2.2).
/// - `Content::Concat(vec![Content::Str("▾ "), Content::Var("label")])` —
///   concatenation. Nests arbitrarily.
/// - `Content::WithAlt` — a list followed by `/ <alt text>`: the alt
///   text is the content's alternative for speech and other non-visual
///   media, kept and never painted.
/// - `Content::None` — explicit "no content"; the pseudo-element does
///   not render at all (matches CSS `content: none;`).
///
/// Resolution reads the element's context through [`ContentContext`].
/// CSS `content: attr(label)` is an `attr()` substitution (CSS Values 5
/// §8.7), which the cascade makes before parsing the value: the host's
/// attribute arrives as a string, so there is no attribute variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Content {
    Str(String),
    Var(String),
    /// `counter(name[, style])` — the innermost counter of that name
    /// in scope at the pseudo-element (CSS Lists 3 §4.3).
    Counter {
        name: String,
        style: crate::counters::CounterStyle,
    },
    /// `counters(name, separator[, style])` — every counter of that name
    /// in scope, outermost first, joined by `separator` (CSS Lists 3
    /// §4.3).
    Counters {
        name: String,
        separator: String,
        style: crate::counters::CounterStyle,
    },
    /// A `<quote>` item (§2.2): a quotation mark of the current nesting
    /// level, or none, moving the quote depth.
    Quote(QuoteKind),
    Concat(Vec<Content>),
    /// `<content-list> / <alt text>` (§2): `content` is painted, `alt`
    /// is its alternative text (strings, counters, `attr()`).
    WithAlt {
        content: Box<Content>,
        alt: Box<Content>,
    },
    None,
}

/// The four `<quote>` keywords (CSS Generated Content 3 §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum QuoteKind {
    /// `open-quote`: the opening mark of the current depth, then the
    /// depth rises by one.
    Open,
    /// `close-quote`: the depth falls by one, then the closing mark of
    /// the new depth; ignored at depth 0.
    Close,
    /// `no-open-quote`: the depth rises by one, no mark.
    NoOpen,
    /// `no-close-quote`: the depth falls by one, no mark; ignored at
    /// depth 0.
    NoClose,
}

impl QuoteKind {
    /// Parse a `<quote>` keyword, ASCII case-insensitively.
    pub fn parse(keyword: &str) -> Option<Self> {
        Some(match keyword.to_ascii_lowercase().as_str() {
            "open-quote" => QuoteKind::Open,
            "close-quote" => QuoteKind::Close,
            "no-open-quote" => QuoteKind::NoOpen,
            "no-close-quote" => QuoteKind::NoClose,
            _ => return None,
        })
    }

    /// The CSS keyword.
    pub fn as_str(self) -> &'static str {
        match self {
            QuoteKind::Open => "open-quote",
            QuoteKind::Close => "close-quote",
            QuoteKind::NoOpen => "no-open-quote",
            QuoteKind::NoClose => "no-close-quote",
        }
    }

    /// The quote depth after this item at depth `depth` (§2.2: a close
    /// at depth 0 is ignored).
    pub fn next_depth(self, depth: u32) -> u32 {
        match self {
            QuoteKind::Open | QuoteKind::NoOpen => depth.saturating_add(1),
            QuoteKind::Close | QuoteKind::NoClose => depth.saturating_sub(1),
        }
    }
}

/// What `content` resolution needs from the element it is generated
/// for: its custom properties, its counters and the quote depth. The
/// cascade implements this over its working state; a bare
/// `HashMap<String, CustomValue>` (a [`VarMap`](crate::VarMap)'s map)
/// implements it as "variables only" for callers without an element.
pub trait ContentContext {
    /// The value of custom property `--name` (without the dashes).
    fn var(&self, name: &str) -> Option<String>;
    /// The innermost counter `name` in scope (0 when there is none).
    fn counter(&self, name: &str) -> i32;
    /// Every counter `name` in scope, outermost first — `counters()`
    /// (CSS Lists 3 §4.3). By default the innermost alone.
    fn counters(&self, name: &str) -> Vec<i32> {
        vec![self.counter(name)]
    }
    /// The text of a `<quote>` item at this point of the document, moving
    /// the quote depth (§2.2). By default no mark and no depth.
    fn quote(&self, kind: QuoteKind) -> String {
        let _ = kind;
        String::new()
    }
    /// `value` in counter style `style` (CSS Counter Styles 3 §3.1). By
    /// default among the predefined styles; the cascade resolves the
    /// author's `@counter-style` rules too.
    fn format_counter(&self, value: i32, style: &crate::counters::CounterStyle) -> String {
        style.format(value)
    }
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
    /// Resolve `Var(...)`, counters and quotes against `ctx`; join
    /// `Concat` parts. Returns `None` if the result should cause the
    /// pseudo-element to be skipped entirely (`Content::None`). The alt
    /// text of a [`WithAlt`](Content::WithAlt) is not part of it
    /// ([`resolve_alt`](Self::resolve_alt)).
    ///
    /// An unresolved var yields the empty string rather than failing.
    pub fn resolve(&self, ctx: &impl ContentContext) -> Option<String> {
        match self {
            Content::None => None,
            Content::WithAlt { content, .. } => content.resolve(ctx),
            other => {
                let mut out = String::new();
                other.write(ctx, &mut out);
                Some(out)
            }
        }
    }

    /// The resolved alt text (§2), `None` without one.
    pub fn resolve_alt(&self, ctx: &impl ContentContext) -> Option<String> {
        match self {
            Content::WithAlt { alt, .. } => alt.resolve(ctx),
            _ => None,
        }
    }

    /// Append this item's text to `out`.
    fn write(&self, ctx: &impl ContentContext, out: &mut String) {
        match self {
            // `Content::None` inside a concat contributes nothing.
            Content::None => {}
            Content::Str(s) => out.push_str(s),
            Content::Var(name) => out.push_str(&ctx.var(name).unwrap_or_default()),
            Content::Counter { name, style } => {
                out.push_str(&ctx.format_counter(ctx.counter(name), style))
            }
            Content::Counters {
                name,
                separator,
                style,
            } => {
                for (i, v) in ctx.counters(name).into_iter().enumerate() {
                    if i > 0 {
                        out.push_str(separator);
                    }
                    out.push_str(&ctx.format_counter(v, style));
                }
            }
            Content::Quote(kind) => out.push_str(&ctx.quote(*kind)),
            Content::Concat(parts) => parts.iter().for_each(|p| p.write(ctx, out)),
            Content::WithAlt { content, .. } => content.write(ctx, out),
        }
    }

    /// The parts of this value, flattened: a `Concat`'s items in order,
    /// a `WithAlt`'s content then its alt text.
    fn any_part(&self, f: &impl Fn(&Content) -> bool) -> bool {
        match self {
            Content::Concat(parts) => parts.iter().any(|p| p.any_part(f)),
            Content::WithAlt { content, alt } => content.any_part(f) || alt.any_part(f),
            other => f(other),
        }
    }

    /// Does this value (or any nested part) read a counter?
    pub fn uses_counters(&self) -> bool {
        self.any_part(&|c| matches!(c, Content::Counter { .. } | Content::Counters { .. }))
    }

    /// Does this value hold a `<quote>` item, which reads and moves the
    /// document's quote depth (§2.2)?
    pub fn uses_quotes(&self) -> bool {
        self.any_part(&|c| matches!(c, Content::Quote(_)))
    }

    /// The `<quote>` items of the painted content, in order — what moves
    /// the quote depth.
    pub fn quotes(&self) -> Vec<QuoteKind> {
        let mut out = Vec::new();
        self.collect_quotes(&mut out);
        out
    }

    fn collect_quotes(&self, out: &mut Vec<QuoteKind>) {
        match self {
            Content::Quote(k) => out.push(*k),
            Content::Concat(parts) => parts.iter().for_each(|p| p.collect_quotes(out)),
            Content::WithAlt { content, .. } => content.collect_quotes(out),
            _ => {}
        }
    }
}
