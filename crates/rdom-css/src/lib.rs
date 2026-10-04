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

mod block;
mod declarations;
mod import;
mod layer;
mod property;
mod scope;
mod top_level;

/// The single `name → (setter, serializer)` table both this crate
/// and `rdom-tui`'s `StyleDeclaration` (M4b step 26) consume.
/// Re-exported from `rdom-style` so the public path
/// `rdom_css::property_dispatch::*` keeps working from
/// pre-restructure consumer code.
pub use rdom_style::property_dispatch;

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

use rdom_style::parse::Cursor;

/// Lenient parse. Unknown properties and unparseable values become
/// [`Warning`]s; the rest of the parse continues. Mirrors browser
/// behavior — copy-pasting CSS from MDN works even if a property
/// isn't supported in this build.
///
/// An `@import` has no loader here and imports nothing
/// (`WarningKind::ImportFailed`); use [`parse_with_loader`].
pub fn parse(source: &str) -> ParseResult {
    parse_in(source, None)
}

/// Fetches the sheets `@import` names (CSS Cascade 5 §3). rdom has no
/// network or filesystem policy of its own: the host decides what a
/// URL means — a file under an asset directory, an embedded string, a
/// refusal. `Err(reason)` imports nothing and reports
/// `WarningKind::ImportFailed`. Closures `Fn(&str) -> Result<String,
/// String>` implement it.
pub trait ImportLoader {
    /// The text of the sheet at `url`, as written in the `@import`.
    fn load(&self, url: &str) -> Result<String, String>;
}

impl<F: Fn(&str) -> Result<String, String>> ImportLoader for F {
    fn load(&self, url: &str) -> Result<String, String> {
        self(url)
    }
}

/// [`parse`], resolving `@import` through `loader`: each imported
/// sheet's rules are parsed in at the import's position (in its
/// `layer(…)`, if any), recorded in `Stylesheet::imports`; cycles are
/// cut with `WarningKind::ImportCycle`.
pub fn parse_with_loader(source: &str, loader: &dyn ImportLoader) -> ParseResult {
    parse_in(source, Some(loader))
}

fn parse_in(source: &str, loader: Option<&dyn ImportLoader>) -> ParseResult {
    let mut cursor = Cursor::new(source);
    let mut sheet = Stylesheet::bare();
    let mut warnings = Vec::new();
    let mut imports = import::Imports {
        loader,
        stack: Vec::new(),
    };
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

fn warning_to_error(w: &Warning) -> ParseError {
    let kind = match &w.kind {
        WarningKind::UnterminatedComment => ParseErrorKind::UnterminatedComment,
        WarningKind::UnterminatedString => ParseErrorKind::UnterminatedString,
        WarningKind::InvalidSelector(s) => ParseErrorKind::InvalidSelector(s.clone()),
        WarningKind::UnknownProperty(_)
        | WarningKind::InvalidValue { .. }
        | WarningKind::MalformedDeclaration(_) => {
            ParseErrorKind::ExpectedToken("valid declaration")
        }
        WarningKind::UnsupportedAtRule(_) | WarningKind::ImportIgnored(_) => {
            ParseErrorKind::ExpectedToken("rule")
        }
        WarningKind::ImportCycle(_) | WarningKind::ImportFailed { .. } => {
            ParseErrorKind::ExpectedToken("importable sheet")
        }
        WarningKind::InvalidAtRulePrelude { .. } | WarningKind::InvalidPropertyRule { .. } => {
            ParseErrorKind::ExpectedToken("at-rule prelude")
        }
    };
    ParseError {
        kind,
        line: w.line,
        column: w.column,
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ParseResult {
    pub stylesheet: Stylesheet,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct InlineParseResult {
    pub style: TuiStyle,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    UnexpectedEof,
    UnterminatedComment,
    UnterminatedString,
    InvalidSelector(String),
    ExpectedToken(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Warning {
    pub kind: WarningKind,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum WarningKind {
    UnknownProperty(String),
    InvalidValue {
        property: String,
        value: String,
    },
    /// A declaration segment that is not `name : value` (missing colon,
    /// missing name, stray tokens). Dropped per CSS Syntax 3 §5.4.4;
    /// the payload is the segment's rendered text.
    MalformedDeclaration(String),
    UnsupportedAtRule(String),
    /// An at-rule rdom evaluates whose prelude is invalid — `@layer a
    /// b;`, `@layer a, b { … }`, a reserved layer name. The whole rule
    /// (block included) is dropped (CSS Syntax 3 §5.4.2).
    InvalidAtRulePrelude {
        name: String,
        prelude: String,
    },
    InvalidSelector(String),
    /// An `@import` (its URL) after a rule other than `@charset` and
    /// `@layer` statements, or inside a block: ignored (CSS Cascade 5 §3).
    ImportIgnored(String),
    /// An `@import` (its URL) that would import a sheet already being
    /// imported: skipped.
    ImportCycle(String),
    /// An `@import` whose sheet did not load: no loader was given, or
    /// the loader refused (`reason`). Nothing is imported.
    ImportFailed {
        url: String,
        reason: String,
    },
    /// An `@property` rule (its prelude, `name`) that registers nothing:
    /// a missing or invalid descriptor, an invalid name, or an initial
    /// value that does not match the syntax (CSS Properties and Values
    /// API 1 §3).
    InvalidPropertyRule {
        name: String,
        reason: String,
    },
    UnterminatedComment,
    UnterminatedString,
}
