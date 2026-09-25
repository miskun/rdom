//! Text selection + clipboard — DOM-native node+offset model
//! matching the browser `Selection` API.
//!
//! ## Sub-modules
//!
//! - types: `Selection`, `Range`, `Position` — core data model.
//! - `drag` — mouse-drag selection (router state; no pointer capture).
//! - `keyboard` — Shift+arrow extend, Shift+Ctrl+arrow word, Ctrl-A.
//!   Double-click word-select; triple-click line-select.
//! - [`clipboard`] — copy / cut / paste. `arboard` integration.
//!   Serialization walks the range in document order with whitespace
//!   normalization.
//! - The `::selection` overlay on the range's cells is painted by
//!   `crate::render::paint_pass` (`selection_overlay`), as is the caret.
//! - `user_select` — `UserSelect::{Auto, Text, None, All, Contain}`
//!   CSS property + cascade hook that shapes what the drag machinery
//!   considers selectable.

pub mod clipboard;
pub(crate) mod drag;
pub(crate) mod keyboard;
pub(crate) mod multiclick;
pub(crate) mod user_select;
