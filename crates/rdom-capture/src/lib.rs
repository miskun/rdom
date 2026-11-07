//! # rdom-capture — headless capture sessions for rdom-tui apps
//!
//! Open an rdom-tui app without a terminal, drive it with keys, text, the
//! mouse and a virtual clock, and capture any moment as a versioned JSON
//! document and the views derived from it — for agents and people
//! improving how an app looks and behaves (`specs/CAPTURE.md`).
//!
//! - [`Environment`] — the deterministic environment a session runs an
//!   app in: size, color depth, color scheme, SGR extensions, media
//!   preferences. Nothing is probed, so the same steps give the same
//!   captures, byte for byte.
//! - [`CaptureApp`] — how an app opts in: build it for an environment and
//!   a named fixture.
//! - [`Session`] — the app, opened once and driven by [`Input`]s and
//!   [`Session::advance`]; [`Session::capture`] returns a [`Capture`].
//! - [`serve`] — the session as a JSON-RPC server over stdio
//!   ([`protocol`]), so any tool that can write lines can drive it.
//! - [`views`] — what a capture looks like as text.
//!
//! ## Quick start
//!
//! ```
//! use rdom_capture::{CaptureApp, CaptureRequest, Environment, Layer, Session, views};
//! use rdom_tui::render::TestBackend;
//! use rdom_tui::{App, Stylesheet, TuiDom};
//!
//! struct Hello;
//!
//! impl CaptureApp for Hello {
//!     fn build(
//!         &self,
//!         env: &Environment,
//!         _fixture: Option<&str>,
//!     ) -> Result<App<TestBackend>, Box<dyn std::error::Error>> {
//!         let mut dom = TuiDom::new();
//!         let root = dom.root();
//!         let p = dom.create_element("p");
//!         let text = dom.create_text_node("hello");
//!         dom.append_child(p, text)?;
//!         dom.append_child(root, p)?;
//!         Ok(env.app(dom, Stylesheet::new())?)
//!     }
//! }
//!
//! let mut session = Session::new(Hello);
//! session.open(Environment::default(), None).unwrap();
//! let capture = session.capture(&CaptureRequest::with_layers([Layer::Grid])).unwrap();
//! let grid = capture.grid.as_ref().unwrap();
//! assert_eq!(grid.row_text(0).trim_end(), "hello");
//! assert!(views::text::text(grid, Default::default()).starts_with("hello\n"));
//! ```

#![forbid(unsafe_code)]

pub mod app;
pub mod audit;
pub mod capture;
pub mod edit;
pub mod env;
pub mod input;
pub mod profile;
pub mod protocol;
pub(crate) mod query;
pub mod raster;
pub mod session;
pub mod variants;
pub mod views;

pub use app::{CaptureApp, FixtureInfo};
pub use capture::{Attr, Capture, ColorValue, Grid, Layer, Meta, Run, SCHEMA};
pub use edit::{EditNote, EditTarget, Edited, FilePatch, Patch, Reloaded, Reset};
pub use env::{ContrastPreference, Depth, Environment, Scheme, Sgr, Viewport};
pub use input::{Button, Input, MouseAction};
pub use profile::{Profile, Rgb};
pub use protocol::serve;
pub use query::Detail;
pub use raster::{Image, Raster, RasterError};
pub use session::{
    CaptureRequest, Change, MAX_ADVANCE_MS, Session, SessionError, Step, Suppressed,
};
pub use variants::diff::{Diff, Snapshot};

#[cfg(test)]
mod tests;

/// The README's examples, compiled and run as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
