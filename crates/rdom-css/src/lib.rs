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

mod declarations;
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
    for rule in parsed.stylesheet.rules() {
        let _ = sheet.add_rule(&rule.source_text, rule.style.clone());
    }
    for (k, v) in parsed.stylesheet.vars() {
        let owned = std::mem::take(&mut sheet);
        sheet = owned.define_var(k, v);
    }
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
    for rule in parsed.stylesheet.rules() {
        let _ = sheet.add_rule(&rule.source_text, rule.style.clone());
    }
    for (k, v) in parsed.stylesheet.vars() {
        let owned = std::mem::take(&mut sheet);
        sheet = owned.define_var(k, v);
    }
    Ok(sheet)
}

use rdom_style::parse::Cursor;

/// Lenient parse. Unknown properties and unparseable values become
/// [`Warning`]s; the rest of the parse continues. Mirrors browser
/// behavior — copy-pasting CSS from MDN works even if a property
/// isn't supported in this build.
pub fn parse(source: &str) -> ParseResult {
    let mut cursor = Cursor::new(source);
    let mut sheet = Stylesheet::bare();
    let mut warnings = Vec::new();
    top_level::parse_stylesheet(&mut cursor, &mut sheet, &mut warnings);
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
/// any warnings. Custom-property declarations (`--name: value`)
/// inside an inline style are dropped silently in M1 — there's no
/// scoped vars story for inline yet.
pub fn parse_inline(source: &str) -> InlineParseResult {
    let mut style = TuiStyle::new();
    let mut custom_props: Vec<declarations::CustomProperty> = Vec::new();
    let mut warnings = Vec::new();
    declarations::parse_block(source, &mut style, &mut custom_props, 1, 1, &mut warnings);
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
        WarningKind::UnknownProperty(_) | WarningKind::InvalidValue { .. } => {
            ParseErrorKind::ExpectedToken("valid declaration")
        }
        WarningKind::UnsupportedAtRule(_) => ParseErrorKind::ExpectedToken("rule"),
    };
    ParseError {
        kind,
        line: w.line,
        column: w.column,
    }
}

#[derive(Debug, Clone)]
pub struct ParseResult {
    pub stylesheet: Stylesheet,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone)]
pub struct InlineParseResult {
    pub style: TuiStyle,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorKind {
    UnexpectedEof,
    UnterminatedComment,
    UnterminatedString,
    InvalidSelector(String),
    ExpectedToken(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub kind: WarningKind,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WarningKind {
    UnknownProperty(String),
    InvalidValue { property: String, value: String },
    UnsupportedAtRule(String),
    InvalidSelector(String),
    UnterminatedComment,
    UnterminatedString,
}
