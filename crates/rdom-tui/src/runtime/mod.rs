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
//! - [`pointer_capture`] — drag routing.
//! - [`app`] — `App`, `AppContext`, `AppHandle`, lifecycle, panic
//!   safety, the main loop.
//! - [`abort`] — `AbortController` / `AbortSignal` for listener
//!   lifetime cancellation (lives here in rdom-tui for v1; may move
//!   to rdom-core if the primitive gets broader use).
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
pub mod router;
pub mod scrollbar;
pub mod selection;
pub mod timers;
pub mod url_opener;
// Placeholders — later Phase 14.6 sub-phases fill these in.
// pub mod pointer_capture;

pub use app::{App, AppContext, AppHandle, ControlFlow};
pub use hit_test::HitTestExt;
pub use router::{RouteOutcome, Router};
