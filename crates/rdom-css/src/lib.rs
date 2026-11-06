//! # rdom-css — CSS string parser for rdom-tui
//!
//! Turns CSS source strings into [`Stylesheet`] and [`TuiStyle`]
//! values consumed by the rdom-tui cascade. Three string-CSS
//! surfaces are unified under one parser: standalone stylesheets,
//! `<style>` blocks in templates, and inline `style="…"` attributes.
//!
//! ## Quick start
//!
//! ```ignore
//! let result = rdom_css::parse("button { color: #3d90ce; }");
//! assert!(result.warnings.is_empty());
//! ```

#![forbid(unsafe_code)]

use rdom_style::{Stylesheet, TuiStyle};

mod at_rules;
mod block;
mod conditional;
mod counter_style;
mod declarations;
mod import;
mod keyframes;
mod layer;
mod located;
mod position_try;
mod property;
mod result;
mod scan;
mod scope;
mod top_level;

/// The single `name → (setter, serializer)` table both this crate
/// and `rdom-tui`'s `StyleDeclaration` (M4b step 26) consume.
/// Re-exported from `rdom-style` so the public path
/// `rdom_css::property_dispatch::*` keeps working from
/// pre-restructure consumer code.
pub use at_rules::at_rule_names;
pub use located::{from_css_at, parse_at};
pub use rdom_style::property_dispatch;
use result::warning_to_error;
pub use result::{
    CounterStyleDescriptorReason, CounterStyleRuleReason, InlineParseResult, ParseError,
    ParseErrorKind, ParseResult, PositionTryDescriptorReason, Warning, WarningKind,
};

/// Convenience: parse `source` and merge the rules + vars into a
/// fresh `Stylesheet::new()` (which carries the UA defaults).
/// Lenient — warnings are dropped silently. For warnings-aware
/// parsing call [`parse`] directly.
pub fn from_css(source: &str) -> Stylesheet {
    let parsed = parse(source);
    let mut sheet = Stylesheet::new();
    sheet.append(&parsed.stylesheet);
    sheet
}

/// Strict variant of [`from_css`]: returns the first warning as a
/// [`ParseError`] instead of dropping it.
pub fn from_css_strict(source: &str) -> Result<Stylesheet, ParseError> {
    let parsed = parse(source);
    if let Some(w) = parsed.warnings.first() {
        return Err(warning_to_error(w));
    }
    let mut sheet = Stylesheet::new();
    sheet.append(&parsed.stylesheet);
    Ok(sheet)
}

use rdom_style::parse::SourceCursor;

/// Lenient parse. Unknown properties and unparseable values become
/// [`Warning`]s; the rest of the parse continues. Mirrors browser
/// behavior — copy-pasting CSS from MDN works even if a property
/// isn't supported in this build.
///
/// An `@import` has no loader here and imports nothing
/// (`WarningKind::ImportFailed`); use [`parse_with_loader`].
pub fn parse(source: &str) -> ParseResult {
    parse_in(source, None, None)
}

/// Fetches the sheets `@import` names (CSS Cascade 5 §3). rdom has no
/// network or filesystem policy of its own: the host decides what a
/// URL means — a file under an asset directory, an embedded string, a
/// refusal. `Err(reason)` imports nothing and reports
/// `WarningKind::ImportFailed`.
///
/// ## Contract
///
/// - [`load_from`](Self::load_from) gets the URL as written in the
///   `@import` and the **base**: the resolved URL of the sheet holding
///   the `@import` — the root sheet's own URL when the parse was given
///   one ([`parse_with_loader_at`]), `None` for a root without one (a
///   `<style>` element, [`parse_with_loader`]). It returns the sheet's
///   text and its **resolved URL**, which becomes the base of the
///   sheet's own imports.
/// - The resolved URL is the sheet's identity: an `@import` whose
///   resolved URL is a sheet already being parsed (an ancestor in the
///   import chain, the root included) closes a cycle and is skipped
///   with `WarningKind::ImportCycle`. A loader canonicalises URLs —
///   `css/./a.css` and `css/a.css` — by returning the same resolved URL
///   for both.
/// - The default `load_from` calls [`load`](Self::load) with the URL as
///   written and takes that as the resolved URL: no base, identity by
///   spelling. So any `Fn(&str) -> Result<String, String>` closure is a
///   loader; annotate its parameter (`|url: &str|`), or the closure is
///   not general over the `&str` lifetime and does not implement the
///   trait.
/// - Imports nest at most [`MAX_IMPORT_DEPTH`] deep; a deeper one warns
///   (`WarningKind::ImportTooDeep`) and imports nothing.
///
/// ```
/// use rdom_css::{ImportLoader, LoadedSheet, parse_with_loader, parse_with_loader_at};
///
/// // A closure: URLs as written.
/// let loader = |url: &str| match url {
///     "theme.css" => Ok("h1 { width: 2 }".to_string()),
///     other => Err(format!("no {other}")),
/// };
/// let r = parse_with_loader("@import 'theme.css'; p { width: 1 }", &loader);
/// assert!(r.warnings.is_empty());
/// assert_eq!(r.stylesheet.rules().len(), 2);
///
/// // A loader resolving relative URLs against the importing sheet.
/// struct Assets;
/// impl ImportLoader for Assets {
///     fn load(&self, url: &str) -> Result<String, String> {
///         self.load_from(url, None).map(|sheet| sheet.text)
///     }
///     fn load_from(&self, url: &str, base: Option<&str>) -> Result<LoadedSheet, String> {
///         let dir = base.and_then(|b| b.rfind('/').map(|i| &b[..=i])).unwrap_or("");
///         let resolved = format!("{dir}{url}");
///         match resolved.as_str() {
///             "css/parts/a.css" => Ok(LoadedSheet::new(resolved, ".a { width: 1 }")),
///             _ => Err(format!("no {resolved}")),
///         }
///     }
/// }
/// let r = parse_with_loader_at("@import 'parts/a.css';", "css/main.css", &Assets);
/// assert!(r.warnings.is_empty());
/// assert_eq!(r.stylesheet.rules().len(), 1);
/// ```
pub trait ImportLoader {
    /// The text of the sheet at `url`, as written in the `@import`.
    fn load(&self, url: &str) -> Result<String, String>;

    /// The sheet `url` names in an `@import` of the sheet at `base`
    /// (trait doc): its text and resolved URL. Defaults to
    /// [`load`](Self::load), with `url` as the resolved URL and `base`
    /// unused.
    fn load_from(&self, url: &str, _base: Option<&str>) -> Result<LoadedSheet, String> {
        self.load(url).map(|text| LoadedSheet::new(url, text))
    }
}

/// How deep `@import`s nest before one is refused
/// (`WarningKind::ImportTooDeep`) — a guard against a loader that
/// produces an endless chain of distinct URLs.
pub const MAX_IMPORT_DEPTH: usize = 16;

/// How deep blocks nest before one is skipped
/// (`WarningKind::BlockTooDeep`): style rules (CSS Nesting 1 §2) and the
/// at-rules that hold blocks — `@media`, `@supports`, `@container`,
/// `@layer`, `@scope`, `@starting-style` — counted together, so
/// `a { @media screen { b { … } } }` is three deep. The parser recurses
/// once per block; the cap bounds its stack against hostile depth, as
/// [`MAX_IMPORT_DEPTH`] and rdom-style's `MAX_CALC_NESTING` bound theirs
/// (C16G-DEPTH-CAPS). Real sheets nest a handful deep; no engine
/// publishes a cap of its own.
pub const MAX_BLOCK_DEPTH: usize = 32;

/// A sheet an [`ImportLoader`] loaded: its resolved URL — its identity
/// and the base of its own imports — and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LoadedSheet {
    pub url: String,
    pub text: String,
}

impl LoadedSheet {
    pub fn new(url: impl Into<String>, text: impl Into<String>) -> Self {
        LoadedSheet {
            url: url.into(),
            text: text.into(),
        }
    }
}

impl<F: Fn(&str) -> Result<String, String>> ImportLoader for F {
    fn load(&self, url: &str) -> Result<String, String> {
        self(url)
    }
}

/// [`parse`], resolving `@import` through `loader`: each imported
/// sheet's rules are parsed in at the import's position (in its
/// `layer(…)`, if any), recorded in `Stylesheet::imports`; cycles are
/// cut with `WarningKind::ImportCycle`. The sheet itself has no URL:
/// its imports get no base, and an import of it is not recognised as a
/// cycle until the second time round — use [`parse_with_loader_at`]
/// for a sheet loaded from a URL.
pub fn parse_with_loader(source: &str, loader: &dyn ImportLoader) -> ParseResult {
    parse_in(source, Some(loader), None)
}

/// [`parse_with_loader`] for a sheet whose own resolved URL is `url`:
/// its imports resolve against it, and an import of it is a cycle
/// ([`ImportLoader`] contract).
pub fn parse_with_loader_at(source: &str, url: &str, loader: &dyn ImportLoader) -> ParseResult {
    parse_in(source, Some(loader), Some(url))
}

pub(crate) fn parse_in(
    source: &str,
    loader: Option<&dyn ImportLoader>,
    url: Option<&str>,
) -> ParseResult {
    let mut cursor = SourceCursor::new(source);
    let mut sheet = Stylesheet::bare();
    let mut warnings = Vec::new();
    let mut imports = import::Imports::new(loader, url);
    top_level::parse_stylesheet(&mut cursor, &mut sheet, &mut warnings, &mut imports);
    ParseResult {
        stylesheet: sheet,
        warnings,
    }
}

/// Strict parse. Returns the first [`ParseError`] encountered, or a
/// [`Stylesheet`] containing every successfully-parsed rule.
/// Used by tests and by tooling.
pub fn parse_strict(source: &str) -> Result<Stylesheet, ParseError> {
    let result = parse(source);
    if let Some(w) = result.warnings.first() {
        return Err(warning_to_error(w));
    }
    Ok(result.stylesheet)
}

/// Lenient inline-attribute parse. Reads a declaration list with
/// no surrounding `{ … }` and returns the resulting `TuiStyle` plus
/// any warnings. Custom-property declarations (`--name: value`) land on
/// `TuiStyle::custom_properties` and are scoped to the element by the
/// cascade.
pub fn parse_inline(source: &str) -> InlineParseResult {
    let mut style = TuiStyle::new();
    let mut warnings = Vec::new();
    declarations::parse_block(source, &mut style, 1, 1, &mut warnings);
    InlineParseResult { style, warnings }
}

/// Strict inline-attribute parse. Returns the first warning as a
/// `ParseError`, or the parsed `TuiStyle`.
pub fn parse_inline_strict(source: &str) -> Result<TuiStyle, ParseError> {
    let result = parse_inline(source);
    if let Some(w) = result.warnings.first() {
        return Err(warning_to_error(w));
    }
    Ok(result.style)
}

/// The README's examples, compiled and run as doctests so they keep
/// compiling as the API moves.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
