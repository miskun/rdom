//! Editable content — `<input>`, `<textarea>`, `contenteditable`,
//! caret, undo/redo, `beforeinput` / `input` events.
//!
//! Depends on `runtime` (focus, keyboard, selection, clipboard).
//!
//! ## Where it lives
//!
//! This module is an empty placeholder; the editing implementation
//! landed elsewhere:
//!
//! - [`crate::runtime::editing`] — the caret (`(node, byte_offset) →
//!   cell` mapping), caret movement, the `beforeinput` → mutate →
//!   `input` lifecycle ([`perform_edit`](crate::runtime::editing::perform_edit)),
//!   and the per-element undo / redo history.
//! - [`crate::runtime::builtins::input`] — the `<input>` / `<textarea>`
//!   built-ins and `contenteditable` hosts.

// Placeholders — Phase 14.7 fills these in.
// pub mod caret;
// pub mod mutation;
// pub mod undo;
// pub mod input;
// pub mod textarea;
// pub mod contenteditable;
