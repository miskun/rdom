//! Motion (`specs/CAPTURE.md` §Motion): a `film` — the app stepped at a
//! fixed virtual-clock interval over a window, in a fork (the session is
//! left as it was), optionally after inputs at its start — and what it
//! shows: each frame's hash, cells changed and bytes sent; timelines of
//! chosen cells (glyph, colors) and elements (the cells they paint, their
//! border box, the animations running on them); a change heatmap; the
//! spinners found; the motion findings ([`audits`]: flicker, spinner
//! pace and jitter, cost on a link, motion under reduced motion); and an
//! asciicast of the bytes sent ([`cast`]).

pub mod audits;
pub mod cast;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::app::CaptureApp;
use crate::audit::Finding;
use crate::capture::identity::identities;
use crate::capture::{ColorValue, Grid, Layer, node_key, nodes};
use crate::input::Input;
use crate::session::{CaptureRequest, Session, SessionError};

/// The longest film: a minute of virtual time.
pub const MAX_FILM_MS: u64 = 60_000;
/// The most frames one film holds.
pub const MAX_FRAMES: usize = 600;

/// What to film.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct FilmRequest {
    /// How long, in milliseconds of virtual time.
    pub ms: u64,
    /// The interval between frames (default: the environment's animation
    /// frame, `1000 / animation_fps`).
    pub step_ms: Option<u64>,
    /// Inputs applied at the film's start (its first frame shows them).
    pub inputs: Vec<Input>,
    /// Cells whose glyph and colors to follow, `[x, y]`.
    pub cells: Vec<[u16; 2]>,
    /// Elements to follow, by selector (each matched element).
    pub nodes: Vec<String>,
    /// The link the bytes are sent over, for the cost findings.
    pub link: Link,
    /// Film again under `prefers-reduced-motion: reduce` and report what
    /// still moves (default on).
    pub reduced_motion: bool,
}

impl Default for FilmRequest {
    fn default() -> Self {
        FilmRequest {
            ms: 1000,
            step_ms: None,
            inputs: Vec::new(),
            cells: Vec::new(),
            nodes: Vec::new(),
            link: Link::Ssh,
            reduced_motion: true,
        }
    }
}

impl FilmRequest {
    /// `ms` of film.
    pub fn of(ms: u64) -> Self {
        FilmRequest {
            ms,
            ..FilmRequest::default()
        }
    }

    pub fn with_step(mut self, step_ms: u64) -> Self {
        self.step_ms = Some(step_ms);
        self
    }

    pub fn with_inputs(mut self, inputs: Vec<Input>) -> Self {
        self.inputs = inputs;
        self
    }

    pub fn with_cells(mut self, cells: Vec<[u16; 2]>) -> Self {
        self.cells = cells;
        self
    }

    pub fn with_nodes(mut self, selectors: Vec<String>) -> Self {
        self.nodes = selectors;
        self
    }

    pub fn with_link(mut self, link: Link) -> Self {
        self.link = link;
        self
    }

    pub fn with_reduced_motion(mut self, on: bool) -> Self {
        self.reduced_motion = on;
        self
    }
}

/// A link the frames are sent over (`specs/CAPTURE.md` §Motion,
/// bandwidth profiles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Link {
    /// A local terminal: no limit.
    Local,
    /// SSH over a fair connection: 2 Mbit/s.
    Ssh,
    /// A slow link: 256 kbit/s.
    Slow,
    /// A link of this many bits per second.
    Bps(u64),
}

impl Link {
    /// Bits per second; `None` for no limit.
    pub fn bps(self) -> Option<u64> {
        match self {
            Link::Local => None,
            Link::Ssh => Some(2_000_000),
            Link::Slow => Some(256_000),
            Link::Bps(b) => Some(b),
        }
    }
}

/// One frame of a film.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct FilmFrame {
    pub t_ms: u64,
    pub grid_hash: String,
    pub cells_changed: u32,
    /// The bytes it sent the terminal (frame 0: its repaint on a cleared
    /// terminal; the inputs' own are `Film::input_bytes`).
    pub bytes: usize,
}

/// A cell over time: its value at the start and at every change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct CellTimeline {
    pub x: u16,
    pub y: u16,
    pub changes: Vec<CellAt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct CellAt {
    pub t_ms: u64,
    pub glyph: String,
    pub fg: ColorValue,
    pub bg: ColorValue,
}

/// An element over time: at the start and at every change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct NodeTimeline {
    pub selector: String,
    /// Its identity.
    pub key: String,
    pub changes: Vec<NodeAt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct NodeAt {
    pub t_ms: u64,
    /// The smallest rectangle holding the cells it paints, `[x, y, w,
    /// h]`; `None` when it paints none.
    pub painted: Option<[u16; 4]>,
    /// How many cells it paints.
    pub cells: u32,
    pub border: Option<[i32; 4]>,
    /// The transitions and animations running on it.
    pub animations: Vec<String>,
}

/// Cells that changed `changes` times over the film.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Heat {
    pub rect: [u16; 4],
    pub changes: u32,
}

/// A cell cycling through glyphs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Spinner {
    pub x: u16,
    pub y: u16,
    /// One cycle's glyphs, in order.
    pub glyphs: Vec<String>,
    /// How long one glyph shows, and one cycle takes.
    pub frame_ms: u64,
    pub period_ms: u64,
}

/// A film's answer.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct Film {
    pub step_ms: u64,
    pub frames: Vec<FilmFrame>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<CellTimeline>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<NodeTimeline>,
    /// The terminal's size during the film.
    pub size: [u16; 2],
    /// The bytes the inputs at the start sent (their frame), apart from
    /// frame 0's.
    pub input_bytes: usize,
    /// The cells that changed, most-changed first (at most 64 runs).
    pub heatmap: Vec<Heat>,
    /// How many runs the heatmap had before it was cut to 64.
    pub heat_runs: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub spinners: Vec<Spinner>,
    pub findings: Vec<Finding>,
    /// The motion checks that did not run, and why.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub not_checked: Vec<crate::audit::NotChecked>,
    /// Every frame's grid, for a filmstrip.
    #[serde(skip)]
    pub grids: Vec<Grid>,
    /// The bytes each frame sent; the first: the whole frame, repainted on
    /// a cleared terminal (what a recording starts from).
    #[serde(skip)]
    pub sent: Vec<Vec<u8>>,
}

/// Each cell of `grid`: glyph and colors as sent.
pub(crate) fn cells_of(grid: &Grid) -> Vec<Vec<(String, ColorValue, ColorValue)>> {
    grid.rows
        .iter()
        .map(|runs| {
            runs.iter()
                .flat_map(|r| {
                    let (fg, bg) = r.shown_colors();
                    r.cells().into_iter().map(move |g| (g.to_string(), fg, bg))
                })
                .collect()
        })
        .collect()
}

impl<A: CaptureApp> Session<A> {
    /// Film this session's app in a fork: `request.inputs` at the start,
    /// then a frame every step over `request.ms`.
    pub fn film(&self, request: &FilmRequest) -> Result<Film, SessionError> {
        let invalid = SessionError::InvalidInput;
        if request.ms > MAX_FILM_MS {
            return Err(invalid(format!(
                "a {} ms film: at most {MAX_FILM_MS}",
                request.ms
            )));
        }
        let step = match request.step_ms {
            Some(s) => s,
            None => 1000 / u64::from(self.environment()?.animation_fps.max(1)),
        };
        if step == 0 {
            return Err(invalid("a film's step is at least 1 ms".into()));
        }
        if request.link == Link::Bps(0) {
            return Err(invalid(
                "a link of 0 bits per second carries nothing".into(),
            ));
        }
        if request.ms / step + 1 > MAX_FRAMES as u64 {
            return Err(invalid(format!(
                "{} frames: at most {MAX_FRAMES} — a longer step or a shorter film",
                request.ms / step + 1
            )));
        }
        let mut fork = self.fork()?;
        let mut film = shoot(&mut fork, request, step)?;
        film.findings = audits::findings(&film, request);
        if request.link == Link::Local {
            film.not_checked.push(crate::audit::NotChecked {
                check: "cost".into(),
                reason: "a local link carries every frame in time".into(),
            });
        }
        if !request.reduced_motion {
            film.not_checked.push(crate::audit::NotChecked {
                check: "reduced-motion".into(),
                reason: "not asked for".into(),
            });
        } else if self.environment()?.reduced_motion {
            // The film already is the reduced-motion one.
            film.findings.extend(audits::reduced_motion(&film));
        } else {
            let mut env = self.opened_environment()?.clone();
            env.reduced_motion = true;
            let mut reduced = self.fork_in(env)?;
            let still = shoot(
                &mut reduced,
                &FilmRequest {
                    cells: Vec::new(),
                    nodes: Vec::new(),
                    ..request.clone()
                },
                step,
            )?;
            film.findings.extend(audits::reduced_motion(&still));
        }
        Ok(film)
    }
}

/// Film `s` (a fork): the inputs, then a frame each `step`.
fn shoot<A: CaptureApp>(
    s: &mut Session<A>,
    request: &FilmRequest,
    step: u64,
) -> Result<Film, SessionError> {
    let mut frames = Vec::new();
    let mut grids = Vec::new();
    let mut sent = Vec::new();
    let mut follow: Vec<(String, String, rdom_tui::core_api::NodeId)> = Vec::new();
    let first = if request.inputs.is_empty() {
        s.advance(0)?
    } else {
        s.input(&request.inputs)?
    };
    let input_bytes = first.bytes_emitted;
    // The whole frame on a cleared terminal: a recording's start, and the
    // backend's memory of colors and cursor in step with it.
    s.repaint()?;
    {
        let app = s.app()?;
        let dom = app.dom();
        let ids = identities(dom);
        for sel in &request.nodes {
            let found = dom.query_selector_all_in(dom.root(), sel).map_err(|e| {
                SessionError::InvalidInput(format!("invalid selector {sel:?}: {e:?}"))
            })?;
            for id in found {
                let key = ids.get(&id).map_or_else(|| node_key(id), |i| i.key.clone());
                follow.push((sel.clone(), key, id));
            }
        }
    }
    let mut node_lines: Vec<NodeTimeline> = follow
        .iter()
        .map(|(sel, key, _)| NodeTimeline {
            selector: sel.clone(),
            key: key.clone(),
            changes: Vec::new(),
        })
        .collect();
    let mut t = 0;
    let mut change = first;
    loop {
        let capture = s.capture(&CaptureRequest::with_layers([Layer::Grid, Layer::Owners]))?;
        let grid = capture.grid.expect("asked for");
        let owners = capture.owners.expect("asked for");
        sent.push(s.last_emitted().to_vec());
        frames.push(FilmFrame {
            t_ms: t,
            grid_hash: capture.meta.grid_hash,
            cells_changed: change.cells_changed,
            bytes: sent.last().map_or(0, Vec::len),
        });
        // Each followed element: the cells it paints, its box, its
        // animations — noted when any changed.
        let app = s.app()?;
        let dom = app.dom();
        let mut painted: HashMap<String, (Option<[u16; 4]>, u32)> = HashMap::new();
        for (y, row) in owners.rows.iter().enumerate() {
            for r in row {
                if let Some(n) = &r.node {
                    let e = painted.entry(n.clone()).or_default();
                    let (x0, y0) = (r.x, y as u16);
                    e.0 = Some(match e.0 {
                        None => [x0, y0, r.w, 1],
                        Some([x, yy, w, h]) => {
                            let (x1, y1) = ((x + w).max(x0 + r.w), (yy + h).max(y0 + 1));
                            let (x, yy) = (x.min(x0), yy.min(y0));
                            [x, yy, x1 - x, y1 - yy]
                        }
                    });
                    e.1 += u32::from(r.w);
                }
            }
        }
        for ((_, _, id), line) in follow.iter().zip(node_lines.iter_mut()) {
            let (p, n) = painted.get(&node_key(*id)).copied().unwrap_or((None, 0));
            let at = NodeAt {
                t_ms: t,
                painted: p,
                cells: n,
                border: rdom_tui::capture::box_model(dom, *id).map(|b| nodes::rect_of(b.border)),
                animations: app
                    .get_animations(*id)
                    .iter()
                    .map(crate::query::explain::animation_name)
                    .collect(),
            };
            let same = line.changes.last().is_some_and(|l| {
                (l.painted, l.cells, l.border, &l.animations)
                    == (at.painted, at.cells, at.border, &at.animations)
            });
            if !same {
                line.changes.push(at);
            }
        }
        grids.push(grid);
        if t + step > request.ms {
            break;
        }
        t += step;
        change = s.advance(step)?;
    }
    let viewport = s.environment()?.viewport;
    let mut film = Film {
        step_ms: step,
        frames,
        cells: Vec::new(),
        nodes: node_lines,
        size: [viewport.width, viewport.height],
        input_bytes,
        heatmap: Vec::new(),
        heat_runs: 0,
        spinners: Vec::new(),
        findings: Vec::new(),
        not_checked: Vec::new(),
        grids,
        sent,
    };
    let all: Vec<_> = film.grids.iter().map(cells_of).collect();
    film.cells = request
        .cells
        .iter()
        .map(|&[x, y]| {
            let mut changes: Vec<CellAt> = Vec::new();
            for (k, g) in all.iter().enumerate() {
                let Some((glyph, fg, bg)) =
                    g.get(usize::from(y)).and_then(|r| r.get(usize::from(x)))
                else {
                    continue;
                };
                if changes
                    .last()
                    .is_none_or(|c| (&c.glyph, c.fg, c.bg) != (glyph, *fg, *bg))
                {
                    changes.push(CellAt {
                        t_ms: film.frames[k].t_ms,
                        glyph: glyph.clone(),
                        fg: *fg,
                        bg: *bg,
                    });
                }
            }
            CellTimeline { x, y, changes }
        })
        .collect();
    (film.heatmap, film.heat_runs) = audits::heatmap(&all);
    film.spinners = audits::spinners(&all, step);
    Ok(film)
}
