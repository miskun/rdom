//! `TuiColor` — a color as declared: a concrete `Color`, a system
//! color, a value that depends on the element (`currentcolor`, a color
//! function holding one or `light-dark()`), or a `var(--name)`
//! reference.
//!
//! Sits on the input side of the cascade (inside `TuiStyle`). The
//! cascade resolves every `TuiColor` into a concrete `Color` via
//! [`TuiColor::resolve`] against a [`ColorContext`] before writing
//! into `ComputedStyle.fg` / `.bg` / `.border_color`, so layout and paint
//! never see a `Var`, `CurrentColor` or `Function`.
//!
//! ## `var()` resolution
//!
//! Given `TuiColor::Var { name, fallback }`:
//!
//! 1. Look up `name` in the vars map. If found AND parses as a color,
//!    that's the result.
//! 2. Otherwise, recursively resolve the `fallback` (which may itself
//!    be a `Var { ... }` — chains are supported).
//! 3. If neither yields a concrete color, use the property's
//!    "inherit" fallback (passed in by the caller — parent's computed
//!    value for that property).
//!
//! ## Parsing
//!
//! [`TuiColor::parse`] and [`parse_color`] take the whole CSS
//! `<color>` grammar (CSS Color 4 / 5): hex (`#rgb`, `#rgba`,
//! `#rrggbb`, `#rrggbbaa`), the 148 named colors, `transparent`,
//! `currentcolor`, the system colors, `rgb()` / `rgba()`, `hsl()` /
//! `hsla()`, `hwb()`, `lab()` / `lch()` / `oklab()` / `oklch()`,
//! `color()`, `color-mix()`, relative colors (`rgb(from …)`) and
//! `light-dark()`; plus rdom's `reset` (the terminal default) and a
//! bare `0..=255` palette index. A value that does not parse is `None`,
//! and the cascade uses the fallback chain.

use std::fmt;
use std::sync::Arc;

use crate::Color;
use crate::color::SystemColor;

mod text;

pub use text::parse_color;
pub(crate) use text::parse_simple_color;

/// Input-side color on `TuiStyle`: a literal, a `var()` reference,
/// or `currentcolor`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TuiColor {
    /// A literal terminal color — `#ff0000`, `red`, `Color::Indexed(204)`.
    Literal(Color),
    /// `currentcolor` (CSS Color 4 §6.4): the element's `color` — in
    /// `color` itself, the inherited one. Resolved at computed-value
    /// time against [`ColorContext::current_color`].
    CurrentColor,
    /// A system color (CSS Color 4 §6.2): the terminal's default
    /// colors or the UA palette ([`SystemColor`]).
    System(SystemColor),
    /// A color function whose value depends on the element
    /// (`color-mix(in srgb, currentcolor, blue)`), computed at
    /// computed-value time against a [`ColorContext`].
    Function(ColorFunction),
    /// Reference to a custom property (`var(--name)` or `var(--name,
    /// fallback)`). Resolved during cascade against `ComputedStyle.vars`.
    Var {
        name: String,
        /// Nested fallback `TuiColor` if `name` is unresolved. `None`
        /// means "let the cascade use the property's own inherit /
        /// initial fallback".
        fallback: Option<Box<TuiColor>>,
    },
}

impl TuiColor {
    /// `var(--name)` with no fallback.
    pub fn var(name: impl Into<String>) -> Self {
        Self::Var {
            name: name.into(),
            fallback: None,
        }
    }

    /// `var(--name, fallback)`. The `fallback` is itself a `TuiColor`,
    /// so `.var_with("accent", TuiColor::Literal(Color::Rgb(255, 0, 0)))` works,
    /// as does chaining `.var_with("accent", TuiColor::var("fallback"))`.
    pub fn var_with(name: impl Into<String>, fallback: TuiColor) -> Self {
        Self::Var {
            name: name.into(),
            fallback: Some(Box::new(fallback)),
        }
    }

    /// Is this a `Var(...)`? Helper for tests + devtools.
    pub fn is_var(&self) -> bool {
        matches!(self, TuiColor::Var { .. })
    }

    /// Parse CSS text with the full `<color>` grammar, keeping a value
    /// that resolves at computed-value time (`currentcolor`) as such.
    pub fn parse(input: &str) -> Option<TuiColor> {
        let tokens = crate::parse::tokenize(input.trim()).ok()?;
        crate::parse::values::parse_color(&tokens)
    }

    /// This color with its `var()` references looked up in `vars`
    /// (a reference that finds no color takes its fallback, in turn);
    /// `None` when a chain ends without one.
    pub fn substitute_vars(
        &self,
        vars: &std::collections::HashMap<String, crate::CustomValue>,
    ) -> Option<TuiColor> {
        match self {
            TuiColor::Var { name, fallback } => {
                let found = vars.get(name).and_then(|v| match v.tokens() {
                    Some(tokens) => crate::parse::values::parse_color(tokens),
                    None => TuiColor::parse(v.as_str()),
                });
                match found {
                    Some(c) => Some(c),
                    None => fallback.as_ref()?.substitute_vars(vars),
                }
            }
            other => Some(other.clone()),
        }
    }

    /// True when the value depends on the element it applies to
    /// (`currentcolor`), so it is resolved once the element's `color`
    /// is known. A `var()` is not looked through: call
    /// [`Self::substitute_vars`] first.
    pub fn depends_on_element(&self) -> bool {
        matches!(self, TuiColor::CurrentColor | TuiColor::Function(_))
    }

    /// The computed color: `var()` references looked up in `vars`
    /// ([`Self::substitute_vars`]), `currentcolor` taken from `cx`.
    /// `None` when a `var()` chain finds no color.
    pub fn resolve(
        &self,
        vars: &std::collections::HashMap<String, crate::CustomValue>,
        cx: &ColorContext,
    ) -> Option<Color> {
        match self.substitute_vars(vars)? {
            TuiColor::Literal(c) => Some(c),
            TuiColor::CurrentColor => Some(cx.current_color),
            TuiColor::System(s) => Some(s.color()),
            TuiColor::Function(f) => f.compute(cx),
            // `substitute_vars` leaves no reference.
            TuiColor::Var { .. } => None,
        }
    }
}

/// A color function kept for computed-value time
/// ([`TuiColor::Function`]): parsed once, when its value is, into a form
/// in which only what depends on the element (`currentcolor`,
/// `light-dark()`, the terminal's default colors) is left to compute;
/// with its CSS text for serialization. Cheap to clone (two reference
/// counts; atomic, so a `TuiStyle` holding one stays `Send + Sync`).
/// Equality and hashing are by text — the parsed form is a function of
/// it.
#[derive(Clone)]
pub struct ColorFunction {
    text: Arc<str>,
    expr: Arc<crate::parse::values::ColorExpr>,
}

impl ColorFunction {
    /// Wrap a color function the parser accepted: its text and parsed
    /// form.
    pub(crate) fn new(text: String, expr: crate::parse::values::ColorExpr) -> Self {
        ColorFunction {
            text: Arc::from(text),
            expr: Arc::new(expr),
        }
    }

    /// The function as CSS text.
    pub fn css_text(&self) -> &str {
        &self.text
    }

    /// The color against `cx`. `None` when a color it names has no
    /// value there (a `currentcolor` the parser could not foresee
    /// failing, which a value from the parser does not produce).
    pub fn compute(&self, cx: &ColorContext) -> Option<Color> {
        crate::parse::values::compute_color_function(&self.expr, cx)
    }
}

impl fmt::Debug for ColorFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ColorFunction").field(&self.text).finish()
    }
}

impl PartialEq for ColorFunction {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl Eq for ColorFunction {}

impl std::hash::Hash for ColorFunction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.text.hash(state);
    }
}

/// What a color value resolves against at computed-value time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ColorContext {
    /// The element's `color` — what `currentcolor` is (CSS Color 4
    /// §6.4). For the `color` property itself, the parent's.
    pub current_color: Color,
    /// The element's used color scheme (CSS Color Adjust 1 §2.1):
    /// what `light-dark()` picks by, and what the terminal's default
    /// colors count as inside a color function.
    pub scheme: crate::color::ColorScheme,
}

impl ColorContext {
    /// A context whose `currentcolor` is `current_color`, under the
    /// default (dark) color scheme.
    pub fn new(current_color: Color) -> Self {
        Self {
            current_color,
            scheme: crate::color::ColorScheme::default(),
        }
    }

    /// This context under `scheme`.
    pub fn with_scheme(mut self, scheme: crate::color::ColorScheme) -> Self {
        self.scheme = scheme;
        self
    }
}

impl From<Color> for TuiColor {
    fn from(c: Color) -> Self {
        Self::Literal(c)
    }
}

/// Resolve `color` against `vars` and `cx` ([`TuiColor::resolve`]).
/// `inherit_fallback` is the property's fallback value (typically the
/// parent's computed color) used when every var lookup and explicit
/// fallback fails.
///
/// The resolution is pure — no mutation of the vars map.
pub fn resolve_tui_color(
    color: &TuiColor,
    vars: &std::collections::HashMap<String, crate::CustomValue>,
    inherit_fallback: Color,
    cx: &ColorContext,
) -> Color {
    color.resolve(vars, cx).unwrap_or(inherit_fallback)
}

#[cfg(test)]
mod tests;
