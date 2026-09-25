//! `StyleDeclaration` + `StyleDeclarationMut` — CSSOM IDL wrappers
//! around an element's inline `TuiStyle`.
//!
//! Reads/writes route through
//! [`rdom_style::property_dispatch`],
//! the single source of truth for the name→(setter, serializer)
//! mapping shared with the block parser.
//!
//! ## Attribute coherence
//!
//! Write methods on [`StyleDeclarationMut`] update **both**:
//!
//! 1. `TuiExt::inline_style` — the cascade input.
//! 2. The `style="…"` attribute — what `outerHTML` round-trips
//!    and what the future external-write observer (step 28)
//!    listens for.
//!
//! Both writes happen in one method body; from the caller's
//! perspective the two are atomic.
//!
//! ## Layout
//!
//! - `error` — [`SetPropertyError`], the `try_*` failure channel.
//! - `read` — [`StyleDeclaration`], the read-only snapshot.
//! - `write` — [`StyleDeclarationMut`], the dual-write setter handle.
//! - `serialize` — `cssText` serialization + shorthand suppression.

use rdom_core::NodeRef;

use crate::TuiExt;
use crate::node::TuiNodeExt;

mod error;
mod read;
mod serialize;
mod write;

#[cfg(test)]
mod tests;

pub use error::SetPropertyError;
pub use read::StyleDeclaration;
pub use write::StyleDeclarationMut;

/// Helper for [`crate::TuiAccessors::style`] — constructs a
/// snapshot declaration view of `node`'s inline style. Returns
/// `None` for non-element nodes (those without a `TuiExt`).
pub(crate) fn from_node_ref(node: &NodeRef<'_, TuiExt>) -> Option<StyleDeclaration> {
    Some(StyleDeclaration::new(node.tui_ext()?.inline_style.clone()))
}
