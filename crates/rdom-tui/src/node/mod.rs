//! Extension traits on `NodeRef` / `NodeMut` for presentation-data access.
//!
//! rdom-core's `NodeMut` exposes DOM methods (`set_id`, `add_class`,
//! `append_child`, …). The methods here live in `rdom-tui` and deal with
//! the `TuiExt` fields — sizing, padding, style, scroll, etc. Keeps the
//! builder-chain pattern ergonomic without cluttering `rdom-core` with
//! TUI-specific concepts.
//!
//! ## Why a local trait?
//!
//! rdom-core's `NodeMut<'a, TuiExt>` has no presentation methods. Adding
//! them there would mean polluting the core with TUI-specific API. So
//! we define `TuiNodeExt` / `TuiNodeMutExt` as local extension traits
//! that `TuiNodeRef` / `TuiNodeMut` opt into.
//!
//! A caller does `use rdom_tui::*;` and gets the methods for free:
//!
//! ```
//! # use rdom_tui::*;
//! let mut dom: TuiDom = TuiDom::new();
//! let div = dom.create_element("div");
//! dom.node_mut(div)
//!     .set_width(Size::Fixed(40))
//!     .set_padding(Padding::all(1));
//! ```

//! - `read` — `TuiNodeExt`, the getters.
//! - `write` — `TuiNodeMutExt`, the setters.
//! - `tree` — tree queries (editable scopes, text descendants, child
//!   text, rendered-ness).

mod read;
mod tree;
mod write;

pub use read::TuiNodeExt;
pub(crate) use tree::{
    child_text, in_skipped_contents, install_text_content, is_available, is_rendered, is_text_input,
};
pub use tree::{
    first_text_descendant, is_descendant_or_self, last_text_descendant, nearest_editable_ancestor,
    text_len,
};
pub use write::TuiNodeMutExt;

#[cfg(test)]
mod tests;
