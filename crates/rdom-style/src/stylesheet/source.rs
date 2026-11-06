//! Where a rule was written (CAPTURE phase 3, `specs/CAPTURE.md`
//! §Derivation chain): the CSS text's file or URL with the line and
//! column of the rule's selector, or the Rust call that built it — so a
//! tool explaining a style can point at the line to edit.
//!
//! A sheet keeps the names of the CSS texts its rules came from
//! ([`Stylesheet::source_name`]): the root text's own (`rdom_css::
//! parse_at`, an `@import` loader's resolved URL), or none (a `<style>`
//! element, a string parsed without one). A rule refers to its text by
//! index ([`SourceFile::Css`]), which [`Stylesheet::append`] maps.

use super::Stylesheet;

/// Where a rule was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct SourceLocation {
    /// The text or code it is in.
    pub file: SourceFile,
    /// Its line, from 1.
    pub line: u32,
    /// Its column, from 1, in characters.
    pub column: u32,
}

/// The text or code a rule is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SourceFile {
    /// CSS text: index `n` of its sheet's sources
    /// ([`Stylesheet::source_name`]).
    Css(u32),
    /// Rust code calling [`Stylesheet::rule`] or [`Stylesheet::add_rule`]
    /// (`#[track_caller]`): the file, as the compiler names it.
    Rust(&'static str),
}

impl SourceLocation {
    /// Line `line`, column `column` of CSS text `file` of its sheet.
    pub fn css(file: u32, line: u32, column: u32) -> Self {
        SourceLocation {
            file: SourceFile::Css(file),
            line,
            column,
        }
    }

    /// The Rust call at `location`.
    pub fn rust(location: &'static std::panic::Location<'static>) -> Self {
        SourceLocation {
            file: SourceFile::Rust(location.file()),
            line: location.line(),
            column: location.column(),
        }
    }

    /// This location at another line and column of the same file.
    pub fn moved_to(self, line: u32, column: u32) -> Self {
        SourceLocation {
            line,
            column,
            ..self
        }
    }
}

impl Stylesheet {
    /// Record a CSS text the sheet's rules come from — its file or URL, or
    /// `None` for unnamed text — and return its index for
    /// [`SourceFile::Css`].
    pub fn add_source(&mut self, name: Option<String>) -> u32 {
        self.sources.push(name);
        u32::try_from(self.sources.len() - 1).expect("fewer than 2³² sources")
    }

    /// The name of CSS text `index` (`None`: unnamed, or no such text).
    pub fn source_name(&self, index: u32) -> Option<&str> {
        self.sources.get(index as usize)?.as_deref()
    }

    /// Every CSS text's name, by index (`None`: unnamed).
    pub fn sources(&self) -> &[Option<String>] {
        &self.sources
    }
}
