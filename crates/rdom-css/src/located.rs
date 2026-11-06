//! Parsing CSS text read from a file or URL, so each rule records where
//! it is written (`rdom_style::SourceLocation`).

use rdom_style::Stylesheet;

use crate::ParseResult;

/// [`from_css`](crate::from_css) for CSS text read from `url` — a file path, a URL — so
/// each rule records where it is written: the sheet's source
/// ([`Stylesheet::source_name`]) and the rule's line and column
/// (`Rule::location`).
pub fn from_css_at(source: &str, url: &str) -> Stylesheet {
    let parsed = parse_at(source, url);
    let mut sheet = Stylesheet::new();
    sheet.append(&parsed.stylesheet);
    sheet
}

/// [`parse`](crate::parse) for CSS text read from `url`: each rule records the URL and
/// its line and column ([`from_css_at`]).
pub fn parse_at(source: &str, url: &str) -> ParseResult {
    crate::parse_in(source, None, Some(url))
}
