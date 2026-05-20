//! # rdom-parser — HTML-ish template parser for rdom-core
//!
//! Turns a template string into a `Dom<Ext>` tree. The `parseFromString`
//! equivalent for the rdom family of crates.
//!
//! ## Quick start
//!
//! ```
//! use rdom_parser::parse;
//! use rdom_core::Dom;
//!
//! let (dom, ids): (Dom<()>, _) = parse(r#"
//!     <div class="hero" id="main">
//!         <h1>Welcome</h1>
//!         <p>Hello &amp; <strong>world</strong></p>
//!     </div>
//! "#).unwrap();
//!
//! let div = ids.iter().find(|&&id| dom.node(id).tag_name() == Some("div")).unwrap();
//! assert!(dom.node(*div).has_class("hero"));
//! ```
//!
//! ## Supported
//!
//! - Start / end / self-closing tags, case-insensitive tag names
//! - Void elements (`<br>`, `<hr>`, `<img>`, …) auto-close
//! - Attributes: `name="value"`, `name='value'`, `name=value`, `name`
//!   (boolean). `class="a b c"` populates the classList.
//! - Text with entity decoding: `&amp; &lt; &gt; &quot; &apos; &nbsp;
//!   &#NNN; &#xHH;`
//! - Comments: `<!-- … -->` preserved as Comment nodes
//! - Case-insensitive tag names (normalized to lowercase)
//! - Attribute names preserved case
//!
//! ## Not supported (out of scope)
//!
//! - `<!DOCTYPE>` — no need for a TUI DOM
//! - CDATA sections, namespace prefixes, processing instructions
//! - `<script>` / `<style>` raw-text mode (our DOM doesn't have these)
//! - Mismatched tags are errors, not auto-repaired
//!
//! ## Error reporting
//!
//! `ParseError` carries line/col/pos + optional hint:
//!
//! ```
//! # use rdom_parser::parse;
//! # use rdom_core::Dom;
//! let err = parse::<()>("<div><span></p></div>").unwrap_err();
//! assert!(err.msg.contains("mismatched"));
//! assert!(err.hint.is_some());
//! ```

mod dom_ext;
mod error;
mod parser;

pub use dom_ext::NodeMutHtml;
pub use error::{ParseError, Result};
pub use parser::{parse, parse_into};
