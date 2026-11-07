//! Replaying a recording (`specs/CAPTURE.md` §Record and replay): the
//! events an app handled in a real terminal (`App::with_recorder`), applied
//! to this session at the times they happened on its virtual clock, every
//! recorded frame checked against the session's by
//! `Buffer::content_hash` — the last frame drawn at each time, the state
//! the next event met (frames a live loop drew on the way, at the same
//! time, are passed over). Replay stops at the first frame that differs,
//! and says which; the events become the session's own steps, so it can
//! be forked, varied and audited at any point.

use rdom_tui::runtime::record::{Record, encode_event, read};
use serde::Serialize;

use super::{Session, SessionError};
use crate::app::CaptureApp;
use crate::input::Input;

/// Where a session following a recording is in it.
#[derive(Debug, Clone)]
pub(crate) struct Following {
    path: std::path::PathBuf,
    /// The lines replayed (the header included).
    lines: usize,
    at_ms: u64,
    frames: usize,
}

/// What a replay did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Replayed {
    /// The events applied.
    pub events: usize,
    /// The recorded frames checked (the last drawn at each time), all
    /// matching but a divergence.
    pub frames_checked: usize,
    /// Where the replay stopped, in ms of the recording.
    pub at_ms: u64,
    /// The first recorded frame the session drew otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divergence: Option<Divergence>,
}

/// A recorded frame the replay drew otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Divergence {
    pub ms: u64,
    /// Its index among the frames checked.
    pub frame: usize,
    /// The hashes, as 16 hex digits.
    pub recorded: String,
    pub replayed: String,
}

impl<A: CaptureApp> Session<A> {
    /// Replay the recording `log` on the open app — opened, like the
    /// recorded one, at the size the log's header names — up to `until_ms`
    /// of it (or its end).
    pub fn replay(&mut self, log: &str, until_ms: Option<u64>) -> Result<Replayed, SessionError> {
        let invalid = SessionError::InvalidInput;
        let records = read(log).map_err(invalid)?;
        let Some(Record::Header { width, height }) = records.first() else {
            return Err(invalid("a recording starts with its header".into()));
        };
        let size = self.environment()?.viewport;
        if (size.width, size.height) != (*width, *height) {
            return Err(invalid(format!(
                "the recording is {width}×{height} and the session {}×{}: open it at the recording's size",
                size.width, size.height
            )));
        }
        self.play(&records[1..], 0, 0, until_ms)
    }

    /// Follow the recording a running app writes at `path`
    /// (`RDOM_RECORD=<path>`): replay what it added since the last call —
    /// the whole log, the first time — on this session, which mirrors the
    /// app from then on (read-only: nothing reaches the running app). A
    /// frame line at the end is held back until the app writes on: it may
    /// draw another at the same time.
    pub fn follow(&mut self, path: &std::path::Path) -> Result<Replayed, SessionError> {
        let invalid = SessionError::InvalidInput;
        let text = std::fs::read_to_string(path)?;
        // Whole lines only: the app may be writing one.
        let text = &text[..text.rfind('\n').map_or(0, |i| i + 1)];
        let lines: Vec<&str> = text.lines().collect();
        let Some(header) = lines.first() else {
            return Err(invalid(format!(
                "{} holds no recording yet",
                path.display()
            )));
        };
        let o = self.open.as_ref().ok_or(SessionError::NotOpen)?;
        let (from, at_ms, frames) = match &o.following {
            Some(f) if f.path == path => (f.lines, f.at_ms, f.frames),
            _ => {
                // The first time: the header, checked like a replay's.
                let records = read(header).map_err(invalid)?;
                if let Some(Record::Header { width, height }) = records.first() {
                    let size = self.environment()?.viewport;
                    if (size.width, size.height) != (*width, *height) {
                        return Err(invalid(format!(
                            "the recording is {width}×{height} and the session {}×{}: open it at the recording's size",
                            size.width, size.height
                        )));
                    }
                }
                (1, 0, 0)
            }
        };
        // Hold back trailing frame lines.
        let mut to = lines.len();
        while to > from && lines[to - 1].split(' ').nth(1) == Some("frame") {
            to -= 1;
        }
        let chunk = format!("{header}\n{}", lines[from..to].join("\n"));
        let records = read(&chunk).map_err(invalid)?;
        let out = self.play(&records[1..], at_ms, frames, None)?;
        let o = self.open.as_mut().expect("open");
        o.following = Some(Following {
            path: path.to_path_buf(),
            lines: to,
            at_ms: out.at_ms,
            frames: frames + out.frames_checked,
        });
        Ok(out)
    }

    /// Apply `body`'s records from `at_ms`, `frames` checked before.
    fn play(
        &mut self,
        body: &[Record],
        at_ms: u64,
        frames: usize,
        until_ms: Option<u64>,
    ) -> Result<Replayed, SessionError> {
        let mut out = Replayed {
            events: 0,
            frames_checked: 0,
            at_ms,
            divergence: None,
        };
        let mut frame = frames;
        for (i, record) in body.iter().enumerate() {
            let ms = match record {
                Record::Event { ms, .. } | Record::Frame { ms, .. } => *ms,
                _ => continue,
            };
            if until_ms.is_some_and(|u| ms > u) {
                break;
            }
            if ms > out.at_ms {
                self.advance(ms - out.at_ms)?;
                out.at_ms = ms;
            }
            match record {
                Record::Event { event, .. } => {
                    let input = match event {
                        crossterm::event::Event::Resize(w, h) => Input::Resize {
                            width: *w,
                            height: *h,
                        },
                        e => Input::Recorded {
                            event: encode_event(e).expect("a recorded event encodes"),
                        },
                    };
                    self.input(&[input])?;
                    out.events += 1;
                }
                Record::Frame { hash, .. } => {
                    // A frame another replaced at the same time is not a
                    // state anything met.
                    let superseded = matches!(body.get(i + 1), Some(Record::Frame { ms: next, .. }) if *next == ms);
                    if superseded {
                        continue;
                    }
                    let got = self.app()?.terminal().buffer().content_hash();
                    if got != *hash {
                        out.divergence = Some(Divergence {
                            ms,
                            frame,
                            recorded: format!("{hash:016x}"),
                            replayed: format!("{got:016x}"),
                        });
                        return Ok(out);
                    }
                    frame += 1;
                    out.frames_checked += 1;
                }
                _ => {}
            }
        }
        Ok(out)
    }
}
