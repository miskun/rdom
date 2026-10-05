//! `TuiAccessors` / `TuiAccessorsMut` extension traits — author-
//! facing IDL-style accessor methods on `NodeRef<'a, TuiExt>` (read
//! side) and `NodeMut<'a, TuiExt>` (write side).
//!
//! Substrate boundary: accessors that need TUI-only state —
//! runtime focus rules, builtin form helpers, computed cascade,
//! layout rects — live here rather than in `rdom-core`. Pure-DOM
//! accessors stay on the rdom-core wrappers.
//!
//! ## Wrong-tag policy
//!
//! Setters are gated by the tags that own each IDL property in HTML.
//! Calls on a non-owning tag (e.g. `set_value("foo")` on a `<div>`)
//! are silent `Ok(())` no-ops — browser-faithful, since JS assignment
//! to a missing IDL property doesn't throw. The wrong-tag case stays
//! loud on the read side: `value()` returns `None` for a `<div>`.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use rdom_tui::{TuiAccessors, TuiAccessorsMut, TuiDom};
//!
//! let mut dom: TuiDom = TuiDom::new();
//! // ... build a form ...
//! let mut el = dom.node_mut(input_id);
//! let current = el.value();      // read works on NodeMut too (delegates to as_ref)
//! el.set_value("hello")?;        // write — single-block read-then-mutate
//! ```
//!
//! `value()` is the "smart" accessor: dispatches on tag at runtime so
//! `<input>`, `<textarea>`, and `<select>` all reply through one call
//! site. The narrow, tag-prefixed variants (`input_value`,
//! `select_value`, etc.) ship in step 30.

pub mod doc;
mod helpers;
mod read_api;
mod read_mut;
mod read_ref;
mod write;
mod write_api;

#[cfg(test)]
mod tests;

pub use crate::runtime::smooth_scroll::{
    ScrollBehaviorOption, ScrollIntoViewOptions, ScrollLogicalPosition, ScrollToOptions,
};
pub use doc::TuiDocAccessors;
pub use read_api::{DomRect, TuiAccessors};
pub use write_api::TuiAccessorsMut;
