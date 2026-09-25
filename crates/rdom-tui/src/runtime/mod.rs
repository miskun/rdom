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
//! - [`selection`] — text selection, clipboard, `::selection`,
//!   `user-select`.
//! - Pointer capture (drag routing) is DOM state in `rdom-core`
//!   (`Dom::set_pointer_capture`); the router honors it.
//! - [`app`] — `App`, `AppContext`, `AppHandle`, lifecycle, panic
//!   safety, the main loop.
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
pub mod editing;
pub mod focus;
pub mod hit_test;
pub(crate) mod implicit_events;
pub mod router;
pub mod scrollbar;
pub mod selection;
pub mod timers;
pub mod trace;
pub mod url_opener;
// Placeholders — later Phase 14.6 sub-phases fill these in.
// pub mod pointer_capture;

pub use app::{App, AppContext, AppHandle, ControlFlow, StylesheetId};
pub use hit_test::HitTestExt;
pub use router::{RouteOutcome, Router};
