//! Terminal rendering primitives — `Color`, `Modifier`, `Style`, `Rect`.
//!
//! These are the low-level types the paint pipeline deals in. Distinct
//! from the cascade types (`TuiColor`, `TuiStyle`, `ComputedStyle`)
//! which represent the style system's *inputs* and *computed results*.
//!
//! ## Module layout
//!
//! - [`color`] — `Color` enum (Reset, ANSI-16 named, Indexed, Rgb)
//! - [`modifier`] — `Modifier` bitflags (9 SGR effects)
//! - [`style`] — `Style` with `add_modifier`/`sub_modifier` + `patch()`
//! - [`rect`] — `Rect` (unsigned grid rectangle with saturating math)
//!
//! Phase 13 adds `Cell` + `Buffer` on top of these; Phase 14 adds
//! `Backend` and `Terminal` for actual ANSI emission.

pub mod backend;
pub mod backend_crossterm;
pub mod buffer;
pub mod cell;
pub(crate) mod compose;
pub mod inline;
pub mod layout_pass;
pub mod paint_pass;
pub mod rect;
pub mod render_context;
pub mod sgr;
pub mod style;
pub mod terminal;
pub mod virtual_screen;

pub use backend::{Backend, TestBackend};
pub use backend_crossterm::{CrosstermBackend, enter_tui_mode, leave_tui_mode};
pub use buffer::Buffer;
pub use cell::{Cell, CellDiff};
// `Color` + `Modifier` live in rdom-style as of the M4b mid-stream
// restructure; re-exported here so existing `rdom_tui::render::Color`
// callers keep working.
pub use inline::{InlineFragment, InlineLayout, LineBox, compute_inline_layout};
pub use layout_pass::LayoutExt;
pub use paint_pass::PaintExt;
pub use rdom_style::{Color, Modifier};
pub use rect::Rect;
pub use render_context::RenderContext;
pub use sgr::{SgrState, emit_cup, emit_reset, emit_sgr_transition};
pub use style::Style;
pub use terminal::{CompletedFrame, Terminal, TerminalGuard};
pub use virtual_screen::VirtualScreen;
