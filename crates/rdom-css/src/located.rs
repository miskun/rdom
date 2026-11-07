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

/// A stylesheet from the CSS file at `path` (absolute, as [`css_file!`]
/// gives it): its text read from disk in a debug build — so an edited
/// file shows on the next load without a rebuild (CAPTURE phase 4's dev
/// loop) — else `embedded`, the text compiled in. A debug build whose file
/// is gone uses `embedded` too: the file as it was built. Each rule
/// records `path` and its line (`Rule::location`), so a tool can point
/// at, and patch, the file.
pub fn load_css_file(path: &str, embedded: &str) -> Stylesheet {
    let text = if cfg!(debug_assertions) {
        std::fs::read_to_string(path).ok()
    } else {
        None
    };
    from_css_at(text.as_deref().unwrap_or(embedded), path)
}

/// The stylesheet of a CSS file of this crate, `path` relative to its
/// `Cargo.toml`: compiled in, read from disk again in a debug build
/// ([`load_css_file`]), each rule recording the file and its line.
///
/// ```ignore
/// let sheet = rdom_css::css_file!("src/app.css");
/// ```
#[macro_export]
macro_rules! css_file {
    ($path:literal) => {
        $crate::load_css_file(
            concat!(env!("CARGO_MANIFEST_DIR"), "/", $path),
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)),
        )
    };
}
