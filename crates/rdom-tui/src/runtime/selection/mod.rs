//! Text selection + clipboard — DOM-native node+offset model
//! matching the browser `Selection` API.
//!
//! ## Sub-modules
//!
//! - types: `Selection`, `Range`, `Position` — core data model.
//! - [`drag`] — mouse-drag selection (uses pointer capture).
//! - [`keyboard`] — Shift+arrow extend, Shift+Ctrl+arrow word, Ctrl-A.
//!   Double-click word-select; triple-click line-select.
//! - [`clipboard`] — copy / cut / paste. `arboard` integration.
//!   Serialization walks the range in document order with whitespace
//!   normalization.
//! - [`paint`] — `::selection` pseudo-element overlay on cells in the
//!   range; `::caret` for collapsed selection on focused editable.
//! - [`user_select`] — `UserSelect::{Auto, Text, None, All, Contain}`
//!   CSS property + cascade hook that shapes what the drag machinery
//!   considers selectable.

pub mod clipboard;
pub(crate) mod drag;
pub(crate) mod keyboard;
pub(crate) mod multiclick;

// Placeholders — Phase 14.6 (sub-phase 6.5) fills these in.
// pub mod types;
// pub mod paint;
// pub mod user_select;
