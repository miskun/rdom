//! Editable content — `<input>`, `<textarea>`, `contenteditable`,
//! caret, undo/redo, `beforeinput` / `input` events.
//!
//! Depends on `runtime` (focus, keyboard, selection, clipboard).
//!
//! ## Module layout (planned)
//!
//! - [`caret`] — blinking cursor rendering via `::caret` pseudo on a
//!   collapsed selection in a focused editable. Blink timer owned by
//!   `App`.
//! - [`mutation`] — the `beforeinput` → mutate → `input` lifecycle.
//!   Compose `InputEvent` with `input_type` + optional `data`;
//!   cancelable via `prevent_default` on `beforeinput`.
//! - [`undo`] — per-element bounded history stack (default 100
//!   entries). Coalescing of consecutive printable-char inserts.
//!   Ctrl-Z / Ctrl-Shift-Z (Cmd on macOS).
//! - [`input`] — `<input>` built-in element: single-line editor with
//!   `value`, `placeholder`, `type` (text / password / number),
//!   `maxlength`, `readonly`, `disabled`.
//! - [`textarea`] — `<textarea>` built-in: multi-line editor,
//!   `rows`/`cols`/`wrap` attributes.
//! - [`contenteditable`] — generalization: any element becomes
//!   editable via `contenteditable="true" | plaintext-only"`.

// Placeholders — Phase 14.7 fills these in.
// pub mod caret;
// pub mod mutation;
// pub mod undo;
// pub mod input;
// pub mod textarea;
// pub mod contenteditable;
