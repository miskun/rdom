//! Variants of a moment (`specs/CAPTURE.md` §Variants): what changed
//! between two moments ([`diff`]), and — in forks of the session, so the
//! session itself is left as it was — what candidate edits would do, the
//! same steps across environments and profiles, and content rewritten to
//! stress the layout.

pub mod diff;
pub mod matrix;
pub mod mockup;
pub mod stress;
pub mod whatif;
