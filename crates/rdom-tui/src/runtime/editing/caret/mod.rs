//! The caret's cell position, for the runtime's editing behaviors
//! (vertical caret movement, caret reveal). The math is a pure function
//! of the inline layout and lives with it in
//! [`crate::render::inline`]; this path is kept for consumers.

pub use crate::render::inline::cell_of_position;

#[cfg(test)]
mod tests;
