//! Editing infrastructure — caret rendering + `contenteditable`
//! behavior.
//!
//! ## Sub-modules
//!
//! - [`editor_state`] — per-editable state (undo/redo history,
//!   coalescing metadata). Filled in by B.4; B.1 lands the stub.
//! - [`caret`] — reverse `(node, byte_offset) → (cell_x, cell_y)`
//!   mapping used by paint to position the cursor.

pub mod caret;
pub mod editor_state;
pub mod movement;
pub mod perform;
pub mod undo;

pub use editor_state::{EditEntry, EditKind, EditorState};
pub use perform::{Edit, EditOutcome, insert_at_selection, perform_edit};
pub use undo::{UndoOutcome, redo as redo_last, undo as undo_last};
