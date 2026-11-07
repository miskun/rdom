//! A capture session: one app, built once, driven by inputs and the
//! virtual clock, captured at any moment (`specs/CAPTURE.md` §Session
//! protocol). The protocol server ([`serve`](crate::serve)) is a thin layer
//! over this type; Rust code can use it directly.
//!
//! Every step settles the app the way the live loop does: the event is
//! handled, then the loop's iteration finishes at the current virtual time
//! (`App::advance(0)`). Time moves only by [`Session::advance`], which runs
//! what the live loop runs while no input arrives — timers, the tick
//! handler once per tick period, the `AppHandle`'s requests — so the same
//! steps give the same captures, byte for byte.
//!
//! App code runs inside `catch_unwind`. A panic while an app is open fails
//! the step with [`SessionError::AppPanicked`] and closes the app, since
//! its state after a panic can't be trusted; open it again to go on.
//!
//! Not yet deterministic (CAPTURE phase 9, `specs/TECH_DEBT.md`
//! `CAPTURE-INJECT-1`): closures another thread queues with
//! `AppHandle::inject` run at the next step after they arrive, so an app
//! whose background threads feed it gives captures that depend on their
//! timing.

mod error;
mod frame;
mod inspect;
mod log;
mod report;
mod snapshot;
mod write_back;

use std::panic::{AssertUnwindSafe, catch_unwind};

use rdom_tui::App;
use rdom_tui::render::TestBackend;
use serde::{Deserialize, Serialize};

pub use error::SessionError;
pub use log::Step;
pub use snapshot::MAX_MARKS;

pub(crate) use inspect::{identity_of, node_by_key};

use crate::app::{CaptureApp, FixtureInfo};
use crate::capture::{Capture, Layer, Meta, SCHEMA, node_key, nodes, owners};
use crate::env::{Environment, Viewport};
use crate::input::Input;
use error::panic_message;
use frame::{Frame, diff};

/// The most virtual time one [`Session::advance`] moves: a day. A longer
/// step is almost certainly a mistake, and a big enough one would
/// overflow the runtime's clock.
pub const MAX_ADVANCE_MS: u64 = 24 * 60 * 60 * 1000;

/// A session over the app `A` describes.
pub struct Session<A: CaptureApp> {
    /// The app's description, shared with the session's forks.
    def: std::rc::Rc<A>,
    open: Option<Opened>,
    /// Moments kept by name for `diff`, oldest first.
    marks: Vec<(String, crate::variants::diff::Snapshot)>,
}

/// The open app, what it was opened with, and its current frame.
struct Opened {
    app: App<TestBackend>,
    env: Environment,
    /// The environment it was opened in (a resize changes `env`).
    opened_env: Environment,
    fixture: Option<String>,
    /// The steps since it opened (`log`).
    log: Vec<Step>,
    /// Why it can no longer be rebuilt from its log, once it can't.
    unreplayable: Option<String>,
    /// What the last step emitted to the terminal.
    emitted: Vec<u8>,
    /// The CSS files its sheets were read from, each with a hash of its
    /// text at open: a rebuild reads them again, so one that changed
    /// stops a fork (`log`).
    files: Vec<(String, u64)>,
    time_ms: u64,
    frame: Frame,
    /// The inspector's edits and forced states (`inspect`).
    edits: crate::edit::Edits,
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
    /// The app asked to stop (a tick handler's `ControlFlow::Quit`, an
    /// `AppHandle::quit`). The session keeps it open; a live app would
    /// have exited.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub quit_requested: bool,
    /// How many bytes the step's frames sent the terminal: what a link
    /// carries.
    pub bytes_emitted: usize,
    /// Events of animations an inspector's change started, withheld from
    /// app code during the step (`specs/CAPTURE.md` §Events caused by the
    /// inspector).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suppressed_events: Vec<Suppressed>,
}

/// An event withheld from app code: an inspector-caused animation's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Suppressed {
    /// `transitionstart`, `animationend`, …
    pub event: String,
    /// The element it would have gone to.
    pub node: String,
    /// The property or animation name.
    pub of: String,
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

/// The owners layer of the frame `app` shows over `area`, and the elements
/// it names.
fn nodes_owners(
    app: &App<TestBackend>,
    area: rdom_tui::Rect,
) -> (
    crate::capture::Owners,
    Vec<rdom_tui::core_api::NodeId>,
    usize,
) {
    let trace = app.trace_paint(area);
    let (owners, named) = owners::owners(&trace);
    (owners, named, trace.frame_mismatch())
}

/// The files `app`'s sheets were read from — every named source that is a
/// readable file — with a hash of each one's text.
fn read_files(app: &App<TestBackend>) -> Vec<(String, u64)> {
    let mut out: Vec<(String, u64)> = Vec::new();
    for id in app.stylesheet_ids() {
        for name in app
            .stylesheet(id)
            .into_iter()
            .flat_map(|s| s.sources().iter().flatten())
        {
            if out.iter().any(|(n, _)| n == name) {
                continue;
            }
            if let Ok(text) = std::fs::read(name) {
                out.push((name.clone(), crate::capture::fnv1a64(&text)));
            }
        }
    }
    out
}

/// Run app code, turning a panic into [`SessionError::AppPanicked`].
fn guarded<T>(f: impl FnOnce() -> Result<T, SessionError>) -> Result<T, SessionError> {
    catch_unwind(AssertUnwindSafe(f))
        .unwrap_or_else(|p| Err(SessionError::AppPanicked(panic_message(&*p))))
}

impl<A: CaptureApp> Session<A> {
    /// A session over `def`, with no app open.
    pub fn new(def: A) -> Self {
        Session {
            def: std::rc::Rc::new(def),
            open: None,
            marks: Vec::new(),
        }
    }

    /// The app's fixtures ([`CaptureApp::fixtures`]), its code guarded.
    pub fn fixtures(&self) -> Result<Vec<FixtureInfo>, SessionError> {
        guarded(|| Ok(self.def.fixtures()))
    }

    /// Whether an app is open.
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// What the last step sent the terminal (escape sequences and text).
    pub fn last_emitted(&self) -> &[u8] {
        self.open.as_ref().map_or(&[], |o| &o.emitted)
    }

    /// The open app.
    pub fn app(&self) -> Result<&App<TestBackend>, SessionError> {
        self.open
            .as_ref()
            .map(|o| &o.app)
            .ok_or(SessionError::NotOpen)
    }

    /// The open app, mutably — for Rust callers that drive it themselves.
    /// Steps taken through it are not on the session's clock, and the
    /// next capture reads the frame of the session's last step. They are
    /// not in its log either, so the session can no longer be forked.
    pub fn app_mut(&mut self) -> Result<&mut App<TestBackend>, SessionError> {
        self.unreplayable(
            "the app was driven through Session::app_mut, outside the session's steps",
        );
        self.open
            .as_mut()
            .map(|o| &mut o.app)
            .ok_or(SessionError::NotOpen)
    }

    /// Build the app in `env` and `fixture`, draw its first frame at time
    /// 0, run [`CaptureApp::after_first_frame`] and draw again. The open
    /// app, if any, is replaced only when the new one opens; an invalid
    /// environment, an unknown fixture or a failed build leave it open.
    pub fn open(
        &mut self,
        env: Environment,
        fixture: Option<&str>,
    ) -> Result<Change, SessionError> {
        env.validate().map_err(SessionError::InvalidEnvironment)?;
        let known: Vec<String> = self.fixtures()?.into_iter().map(|f| f.name).collect();
        if let Some(name) = fixture
            && !known.is_empty()
            && !known.iter().any(|k| k == name)
        {
            return Err(SessionError::UnknownFixture {
                name: name.to_string(),
                known,
            });
        }
        let def = &*self.def;
        let app = guarded(|| {
            let app = def
                .build(&env, fixture)
                .map_err(|e| SessionError::Build(e.to_string()))?;
            env.check_viewport(&app).map_err(SessionError::Build)?;
            let mut app = env.enforce(app);
            app.advance(0)?;
            def.after_first_frame(&mut app, fixture);
            app.advance(0)?;
            app.terminal_mut().backend_mut().clear_bytes();
            Ok(app)
        })?;
        let before = rdom_tui::Buffer::empty(app.terminal().buffer().area);
        let frame = Frame::of(&app);
        let files = read_files(&app);
        self.open = Some(Opened {
            app,
            opened_env: env.clone(),
            env,
            fixture: fixture.map(str::to_string),
            log: Vec::new(),
            unreplayable: None,
            files,
            emitted: Vec::new(),
            time_ms: 0,
            frame,
            edits: crate::edit::Edits::default(),
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
        self.step(|o| {
            for (input, events) in steps {
                if let Input::Resize { width, height } = input {
                    o.app.terminal_mut().backend_mut().resize(*width, *height);
                    o.env.viewport = Viewport {
                        width: *width,
                        height: *height,
                    };
                }
                for event in events {
                    o.app.handle_event(event);
                    o.app.advance(0)?;
                }
            }
            Ok(())
        })?;
        self.record(Step::Input {
            inputs: inputs.to_vec(),
        });
        Ok(self.change_since(&before))
    }

    /// Move the virtual clock `ms` forward (at most [`MAX_ADVANCE_MS`]),
    /// firing what comes due, and draw the frame.
    pub fn advance(&mut self, ms: u64) -> Result<Change, SessionError> {
        if ms > MAX_ADVANCE_MS {
            return Err(SessionError::InvalidInput(format!(
                "advance by {ms} ms: at most {MAX_ADVANCE_MS} (a day) per step"
            )));
        }
        let before = self.app()?.terminal().buffer().clone();
        self.step(|o| {
            o.app.advance(ms)?;
            o.time_ms += ms;
            Ok(())
        })?;
        self.record(Step::Advance { ms });
        Ok(self.change_since(&before))
    }

    /// A capture of the current frame with the layers `request` asks for.
    pub fn capture(&self, request: &CaptureRequest) -> Result<Capture, SessionError> {
        let o = self.open.as_ref().ok_or(SessionError::NotOpen)?;
        let wants = |l: Layer| request.layers.contains(&l);
        let meta = Meta {
            rdom_capture: env!("CARGO_PKG_VERSION").to_string(),
            rdom_tui: rdom_tui::VERSION.to_string(),
            time_ms: o.time_ms,
            env: o.env.clone(),
            fixture: o.fixture.clone(),
            grid_hash: o.frame.hash.clone(),
            warnings: Vec::new(),
        };
        let dom = o.app.dom();
        let mut named = Vec::new();
        let mut warnings = Vec::new();
        let owners = if wants(Layer::Owners) {
            // A traced repaint of the frame: app code (a `<canvas>`
            // callback) may run, so it is guarded. The repaint reads the
            // app and changes nothing, so a panic there leaves it open.
            let area = o.app.terminal().buffer().area;
            let (owners, n, mismatch) = guarded(|| Ok(nodes_owners(&o.app, area)))?;
            named.extend(n);
            if mismatch > 0 {
                warnings.push(format!(
                    "owners: the traced repaint differs from the frame in {mismatch} cells \
                     (a <canvas> callback or a layout that changed since the frame); \
                     the owners layer may not describe what the grid shows there"
                ));
            }
            Some(owners)
        } else {
            None
        };
        let boxes = wants(Layer::Boxes).then(|| {
            let boxes = nodes::boxes(dom);
            named.extend(boxes.iter().map(|(id, _)| *id));
            boxes.into_iter().map(|(id, b)| (node_key(id), b)).collect()
        });
        if wants(Layer::Nodes) {
            named = nodes::elements(dom);
        }
        let nodes = (!named.is_empty()).then(|| nodes::nodes(dom, named));
        let mut meta = meta;
        meta.warnings = warnings;
        Ok(Capture {
            schema: SCHEMA.to_string(),
            meta,
            grid: wants(Layer::Grid).then(|| o.frame.grid.clone()),
            owners,
            boxes,
            nodes,
        })
    }

    /// Run `f` on the open app, closing it if app code panics. Afterwards
    /// the frame is read again and the bytes the step emitted are dropped
    /// (the wire view keeps them once it exists), so a long session
    /// doesn't grow.
    fn step(
        &mut self,
        f: impl FnOnce(&mut Opened) -> Result<(), SessionError>,
    ) -> Result<(), SessionError> {
        let o = self.open.as_mut().ok_or(SessionError::NotOpen)?;
        let result = guarded(|| f(o));
        if let Err(SessionError::AppPanicked(_)) = &result {
            self.open = None;
            return result;
        }
        let o = self
            .open
            .as_mut()
            .expect("a step that did not panic leaves the app open");
        o.emitted.clear();
        o.emitted
            .extend_from_slice(o.app.terminal().backend().bytes());
        o.app.terminal_mut().backend_mut().clear_bytes();
        o.frame = Frame::of(&o.app);
        result
    }

    /// What changed between `before` and the current frame, and the
    /// events withheld since the last step.
    fn change_since(&mut self, before: &rdom_tui::Buffer) -> Change {
        let o = self.open.as_mut().expect("a step leaves the app open");
        let suppressed_events = o
            .app
            .take_suppressed_events()
            .into_iter()
            .map(|e| Suppressed {
                event: e.event,
                node: crate::capture::node_key(e.node),
                of: e.of,
            })
            .collect();
        let (cells_changed, bounds) = diff(before, o.app.terminal().buffer());
        Change {
            time_ms: o.time_ms,
            cells_changed,
            bounds,
            grid_hash: o.frame.hash.clone(),
            quit_requested: o.app.quit_requested(),
            bytes_emitted: o.emitted.len(),
            suppressed_events,
        }
    }
}

#[cfg(test)]
mod tests;
