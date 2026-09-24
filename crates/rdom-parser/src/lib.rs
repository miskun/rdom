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
//! - Text with character references: the full WHATWG named table
//!   (2 231 entries, including the legacy no-`;` names such as `&amp`
//!   and `&copy`, with HTML's attribute-value caveat) plus `&#NNN;` /
//!   `&#xHH;`; U+0000, surrogates and out-of-range values decode to
//!   U+FFFD
//! - A `<` not followed by an ASCII letter, `/`, `!`, or `?` is text
//!   (`a < b` needs no escaping)
//! - `<style>` / `<script>` bodies are raw text; `<textarea>` /
//!   `<title>` bodies are RCDATA (references decode, tags are text)
//! - Comments: `<!-- … -->` preserved as Comment nodes; `<?…>` and
//!   `<!…>` (other than DOCTYPE) are bogus comments, also Comment nodes
//! - `<!DOCTYPE …>` is consumed (no node)
//! - Case-insensitive tag names (normalized to lowercase)
//! - Attribute names preserved case
//!
//! ## Not supported (out of scope)
//!
//! - CDATA sections as CDATA (they become bogus comments), namespace
//!   prefixes
//! - Tree-construction recovery: a mismatched or missing end tag, or a
//!   stray end tag at the top level, is an error, not auto-repaired
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
mod entities;
mod error;
mod parser;

pub use dom_ext::NodeMutHtml;
pub use error::{ParseError, Result};
pub use parser::{parse, parse_into};
