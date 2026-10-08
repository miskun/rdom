//! rdom-tui runtime — the event loop, hit testing, mouse/keyboard
//! routing, focus management, text selection, clipboard.
//!
//! ## Module layout
//!
//! - [`hit_test`] — `HitTestExt`: map (x, y) → `Option<NodeId>` +
//!   `hit_test_path(x, y) → Vec<NodeId>`. Block + IFC fragment +
//!   overflow clip + paint-order stacking.
//! - [`router`] — crossterm event → synthesized `TuiEvent` → dispatch.
//!   Sub-modules: `mouse`, `hover`, `wheel`, `key`.
//! - [`focus`] — tabindex, focus navigation, modal focus trap.
//! - [`pointer_shape`] — the terminal pointer's shape, from `cursor`
//!   (OSC 22).
//! - [`style_flush`] — bring an element's style up to date between
//!   frames (what `focus()` does first).
//! - [`selection`] — text selection, clipboard, `::selection`,
//!   `user-select`.
//! - Pointer capture (drag routing) is DOM state in `rdom-core`
//!   (`Dom::set_pointer_capture`); the router honors it.
//! - [`app`] — `App`, `AppContext`, `AppHandle`, lifecycle, panic
//!   safety, the main loop.
//! - `color_scheme` — the terminal's color scheme, asked at startup
//!   (OSC 11).
//! - `input` — the terminal input reader and escape-sequence parser
//!   (Unix; crossterm's reader elsewhere).
//! - `AbortController` / `AbortSignal` (listener lifetime
//!   cancellation) live in `rdom-core` (`rdom_core::AbortSignal`).
//!
//! Every sub-module is independently testable and usable. `App`
//! composes them; apps can alternatively drive `Router::route`
//! directly for tests or custom loops.

pub mod animation;
pub mod app;
pub mod autofocus;
pub mod builtins;
pub mod caret_blink;
pub(crate) mod color_scheme;
pub mod editing;
pub mod focus;
pub mod hit_test;
pub(crate) mod implicit_events;
#[cfg(test)]
mod inert_tests;
pub(crate) mod input;
pub mod media_query;
pub mod pointer_shape;
pub(crate) mod resize;
pub mod router;
pub(crate) mod scroll_snap;
pub mod scrollbar;
pub mod selection;
pub mod smooth_scroll;
pub(crate) mod state_writes;
pub mod style_flush;
pub mod timers;
pub(crate) mod top_layer;
pub mod trace;
pub mod url_opener;
// Placeholders — later Phase 14.6 sub-phases fill these in.
// pub mod pointer_capture;

pub use app::{App, AppContext, AppHandle, ControlFlow, StylesheetId};
pub use hit_test::HitTestExt;
pub use media_query::{MediaListenerId, MediaQueryList, MediaQueryListEvent};
pub use router::{RouteOutcome, Router};
