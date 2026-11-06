//! How an app opts into capture sessions (`specs/CAPTURE.md` §Getting an
//! app into a session).
//!
//! A Rust app can't be loaded by an outside tool, so the app ships a small
//! binary that hands a [`CaptureApp`] to [`serve`](crate::serve):
//!
//! ```no_run
//! use rdom_capture::{CaptureApp, Environment, FixtureInfo};
//! use rdom_tui::render::TestBackend;
//! use rdom_tui::{App, Stylesheet, TuiDom};
//!
//! struct MyApp;
//!
//! impl CaptureApp for MyApp {
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
//! fn main() -> std::io::Result<()> {
//!     rdom_capture::serve(MyApp)
//! }
//! ```

use std::error::Error;

use rdom_tui::App;
use rdom_tui::render::TestBackend;
use serde::{Deserialize, Serialize};

use crate::env::{Environment, Viewport};

/// An app a capture session can open.
pub trait CaptureApp {
    /// Build the app for a session, in `fixture`'s state (`None`: the
    /// default state). Build it with [`Environment::app`], or any way that
    /// gives a `TestBackend` of the viewport's size; the session applies
    /// the environment again afterwards. No terminal is touched.
    fn build(
        &self,
        env: &Environment,
        fixture: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn Error>>;

    /// Run after the app's first frame, before the session's first
    /// capture — what a page's load scripts do once the page has painted.
    /// The session draws the frame this changes. Default: nothing.
    fn after_first_frame(&self, app: &mut App<TestBackend>, fixture: Option<&str>) {
        let _ = (app, fixture);
    }

    /// The named states the app can open in ("empty", "1k-rows",
    /// "error"). When the list is not empty, the session refuses a fixture
    /// that is not on it. Default: none.
    fn fixtures(&self) -> Vec<FixtureInfo> {
        Vec::new()
    }
}

/// A named state an app can open in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FixtureInfo {
    /// What a session passes to open it.
    pub name: String,
    /// What it shows, for a person or an agent choosing one.
    pub summary: String,
    /// The viewport it is designed for, a hint for the `env` to open it
    /// in; the session opens it in whatever environment it is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport: Option<Viewport>,
}

impl FixtureInfo {
    /// A fixture named `name`.
    pub fn new(name: impl Into<String>, summary: impl Into<String>) -> Self {
        FixtureInfo {
            name: name.into(),
            summary: summary.into(),
            viewport: None,
        }
    }

    /// This fixture with the viewport it is designed for.
    pub fn with_viewport(mut self, width: u16, height: u16) -> Self {
        self.viewport = Some(Viewport { width, height });
        self
    }
}
