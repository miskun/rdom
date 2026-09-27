//! Editing infrastructure — caret rendering + `contenteditable`
//! behavior.
//!
//! ## Sub-modules
//!
//! - [`editor_state`] — per-editable state: undo/redo history and
//!   undo grouping (Blink's typing-command model — a typing run, a
//!   Backspace run and a Delete run are one step each; a selection
//!   change, paste, cut, undo and redo close the group; no timer).
//! - [`caret`] — reverse `(node, byte_offset) → (cell_x, cell_y)`
//!   mapping used by paint to position the cursor.

pub mod caret;
pub mod editor_state;
pub mod movement;
pub mod perform;
pub mod undo;

pub use editor_state::{EditEntry, EditKind, EditorState};
pub use perform::{
    Edit, EditOutcome, insert_at_selection, insert_at_selection_as, perform_edit, perform_edit_as,
};
pub use undo::{UndoOutcome, redo as redo_last, undo as undo_last};
