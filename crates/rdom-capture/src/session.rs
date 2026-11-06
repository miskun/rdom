//! A capture session: one app, built once, driven by inputs and the
//! virtual clock, captured at any moment (`specs/CAPTURE.md` §Session
//! protocol). The protocol server ([`serve`](crate::serve)) is a thin layer
//! over this type; Rust code can use it directly.
//!
//! Every step settles the app the same way a terminal frame does: the
//! event is handled, then the frame at the current virtual time is drawn
//! (`App::advance(0)`). Time moves only by [`Session::advance`], so the
//! same steps give the same captures, byte for byte.
//!
//! A panic in app code during a step is caught: the step fails with
//! [`SessionError::AppPanicked`] and the session closes, since the app's
//! state after a panic can't be trusted. Open it again to go on.

use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};

use rdom_tui::render::TestBackend;
use rdom_tui::{App, Buffer};
use serde::{Deserialize, Serialize};

use crate::app::CaptureApp;
use crate::capture::{Capture, Grid, Layer, Meta, SCHEMA, grid_hash};
use crate::env::Environment;
use crate::input::Input;

/// A session over the app `A` describes.
pub struct Session<A: CaptureApp> {
    def: A,
    open: Option<Opened>,
}

/// The open app and what it was opened with.
struct Opened {
    app: App<TestBackend>,
    env: Environment,
    fixture: Option<String>,
    time_ms: u64,
}

/// What a step changed on screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Change {
    /// The virtual clock after the step.
    pub time_ms: u64,
    /// How many cells show something else than before the step.
    pub cells_changed: u32,
    /// The smallest rectangle holding every changed cell, as
    /// `[x, y, width, height]`; absent when nothing changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<[u16; 4]>,
    /// The grid's hash after the step.
    pub grid_hash: String,
}

/// What a capture should include.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct CaptureRequest {
    /// The layers to include beside `meta`.
    pub layers: Vec<Layer>,
}

impl CaptureRequest {
    /// A request for `layers`.
    pub fn with_layers(layers: impl IntoIterator<Item = Layer>) -> Self {
        CaptureRequest {
            layers: layers.into_iter().collect(),
        }
    }
}

/// Why a session step failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum SessionError {
    /// No app is open: call [`Session::open`] first.
    NotOpen,
    /// The environment can't be run.
    InvalidEnvironment(String),
    /// The app has no fixture of that name.
    UnknownFixture { name: String, known: Vec<String> },
    /// The app failed to build.
    Build(String),
    /// An input can't be turned into terminal events.
    InvalidInput(String),
    /// App code panicked; the session closed.
    AppPanicked(String),
    /// Drawing a frame failed.
    Io(std::io::Error),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::NotOpen => f.write_str("no app is open; call open first"),
            SessionError::InvalidEnvironment(e) => write!(f, "invalid environment: {e}"),
            SessionError::UnknownFixture { name, known } => {
                write!(f, "no fixture {name:?}; the app has: {}", known.join(", "))
            }
            SessionError::Build(e) => write!(f, "the app failed to build: {e}"),
            SessionError::InvalidInput(e) => write!(f, "invalid input: {e}"),
            SessionError::AppPanicked(e) => {
                write!(
                    f,
                    "app code panicked ({e}); the session closed, open it again"
                )
            }
            SessionError::Io(e) => write!(f, "drawing a frame failed: {e}"),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<std::io::Error> for SessionError {
    fn from(e: std::io::Error) -> Self {
        SessionError::Io(e)
    }
}

/// The message of a caught panic.
fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_string()))
        .unwrap_or_else(|| "a panic with no message".to_string())
}

impl<A: CaptureApp> Session<A> {
    /// A session over `def`, with no app open.
    pub fn new(def: A) -> Self {
        Session { def, open: None }
    }

    /// The app description.
    pub fn app_def(&self) -> &A {
        &self.def
    }

    /// Whether an app is open.
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// The open app.
    pub fn app(&self) -> Result<&App<TestBackend>, SessionError> {
        self.open
            .as_ref()
            .map(|o| &o.app)
            .ok_or(SessionError::NotOpen)
    }

    /// The open app, mutably — for Rust callers that drive it themselves.
    /// Steps taken through it are not on the session's clock.
    pub fn app_mut(&mut self) -> Result<&mut App<TestBackend>, SessionError> {
        self.open
            .as_mut()
            .map(|o| &mut o.app)
            .ok_or(SessionError::NotOpen)
    }

    /// Build the app in `env` and `fixture` (closing any open one), draw
    /// its first frame at time 0, run [`CaptureApp::after_first_frame`] and
    /// draw again.
    pub fn open(
        &mut self,
        env: Environment,
        fixture: Option<&str>,
    ) -> Result<Change, SessionError> {
        self.open = None;
        env.validate().map_err(SessionError::InvalidEnvironment)?;
        let known: Vec<String> = self.def.fixtures().into_iter().map(|f| f.name).collect();
        if let Some(name) = fixture
            && !known.is_empty()
            && !known.iter().any(|k| k == name)
        {
            return Err(SessionError::UnknownFixture {
                name: name.to_string(),
                known,
            });
        }
        let def = &self.def;
        let built = catch_unwind(AssertUnwindSafe(|| {
            let app = def
                .build(&env, fixture)
                .map_err(|e| SessionError::Build(e.to_string()))?;
            env.check_viewport(&app).map_err(SessionError::Build)?;
            let mut app = env.enforce(app);
            app.advance(0)?;
            def.after_first_frame(&mut app, fixture);
            app.advance(0)?;
            app.terminal_mut().backend_mut().clear_bytes();
            Ok::<_, SessionError>(app)
        }))
        .map_err(|p| SessionError::AppPanicked(panic_message(&*p)))??;
        let before = Buffer::empty(built.terminal().buffer().area);
        self.open = Some(Opened {
            app: built,
            env,
            fixture: fixture.map(str::to_string),
            time_ms: 0,
        });
        Ok(self.change_since(&before))
    }

    /// Close the open app, if any.
    pub fn close(&mut self) {
        self.open = None;
    }

    /// Apply `inputs` in order, each followed by the frame it causes.
    /// Every input is checked before the first is applied, so an invalid
    /// one applies none.
    pub fn input(&mut self, inputs: &[Input]) -> Result<Change, SessionError> {
        let steps: Vec<_> = inputs
            .iter()
            .map(|i| i.events().map(|e| (i, e)))
            .collect::<Result<_, _>>()
            .map_err(SessionError::InvalidInput)?;
        let before = self.app()?.terminal().buffer().clone();
        self.step(|app| {
            for (input, events) in steps {
                if let Input::Resize { width, height } = input {
                    app.terminal_mut().backend_mut().resize(*width, *height);
                }
                for event in events {
                    app.handle_event(event);
                    app.advance(0)?;
                }
            }
            Ok(())
        })?;
        Ok(self.change_since(&before))
    }

    /// Move the virtual clock `ms` forward, firing what comes due, and
    /// draw the frame.
    pub fn advance(&mut self, ms: u64) -> Result<Change, SessionError> {
        let before = self.app()?.terminal().buffer().clone();
        self.step(|app| Ok(app.advance(ms)?))?;
        if let Some(o) = self.open.as_mut() {
            o.time_ms += ms;
        }
        Ok(self.change_since(&before))
    }

    /// A capture of the current frame with the layers `request` asks for.
    pub fn capture(&self, request: &CaptureRequest) -> Result<Capture, SessionError> {
        let o = self.open.as_ref().ok_or(SessionError::NotOpen)?;
        let grid = Grid::from_buffer(o.app.terminal().buffer(), o.env.color_depth.color_depth());
        let meta = Meta {
            rdom_capture: env!("CARGO_PKG_VERSION").to_string(),
            time_ms: o.time_ms,
            env: o.env.clone(),
            fixture: o.fixture.clone(),
            grid_hash: grid_hash(&grid),
        };
        Ok(Capture {
            schema: SCHEMA.to_string(),
            meta,
            grid: request.layers.contains(&Layer::Grid).then_some(grid),
        })
    }

    /// Run `f` on the open app, closing the session if app code panics.
    /// The bytes the step emitted are dropped (the wire view keeps them
    /// once it exists), so a long session doesn't grow.
    fn step(
        &mut self,
        f: impl FnOnce(&mut App<TestBackend>) -> Result<(), SessionError>,
    ) -> Result<(), SessionError> {
        let o = self.open.as_mut().ok_or(SessionError::NotOpen)?;
        let result = catch_unwind(AssertUnwindSafe(|| f(&mut o.app)));
        match result {
            Ok(r) => {
                o.app.terminal_mut().backend_mut().clear_bytes();
                r
            }
            Err(panic) => {
                self.open = None;
                Err(SessionError::AppPanicked(panic_message(&*panic)))
            }
        }
    }

    /// What changed between `before` and the current frame.
    fn change_since(&self, before: &Buffer) -> Change {
        let o = self.open.as_ref().expect("a step leaves the app open");
        let now = o.app.terminal().buffer();
        let grid = Grid::from_buffer(now, o.env.color_depth.color_depth());
        let (cells_changed, bounds) = diff(before, now);
        Change {
            time_ms: o.time_ms,
            cells_changed,
            bounds,
            grid_hash: grid_hash(&grid),
        }
    }
}

/// How many cells differ between `a` and `b`, and the rectangle holding
/// them. Buffers of different sizes differ everywhere `b` covers.
fn diff(a: &Buffer, b: &Buffer) -> (u32, Option<[u16; 4]>) {
    let area = b.area;
    let (mut n, mut min_x, mut min_y, mut max_x, mut max_y) = (0u32, u16::MAX, u16::MAX, 0, 0);
    let same_size = a.area == b.area;
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            let differs = !same_size || !same_look(a.cell(x, y), b.cell(x, y));
            if differs {
                n += 1;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    let bounds = (n > 0).then(|| [min_x, min_y, max_x - min_x + 1, max_y - min_y + 1]);
    (n, bounds)
}

/// Whether two cells show the same thing: glyph, colors, attributes,
/// underline color and link — not the renderer's diff hint.
fn same_look(a: Option<&rdom_tui::Cell>, b: Option<&rdom_tui::Cell>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            a.raw_symbol() == b.raw_symbol()
                && a.fg == b.fg
                && a.bg == b.bg
                && a.modifier == b.modifier
                && a.underline_color == b.underline_color
                && a.link == b.link
        }
        (a, b) => a.is_none() && b.is_none(),
    }
}
