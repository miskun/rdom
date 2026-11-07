//! Replaying a recording (`specs/CAPTURE.md` §Record and replay): the
//! events an app handled (`App::with_recorder`), applied to this session
//! at the times they happened on its virtual clock, every recorded frame
//! checked against the session's by `Buffer::content_hash` — the last
//! frame drawn at each time, the state the next event met (frames drawn on
//! the way, at the same time, are passed over). The events become the
//! session's own steps, so it can be forked, varied and audited.
//!
//! A replay refuses what it cannot reproduce rather than reporting a frame
//! that merely differs: a recording of an App configured otherwise than
//! the session's (the header's `caret`, `frame`, `tick`, `scheme`), a
//! session that already took steps; and it stops, saying why, at what the
//! log marks unreplayable (a closure another thread injected, the terminal
//! reporting its colors) and at the first frame that differs.
//!
//! Limits (`TECH_DEBT` CAPTURE-REPLAY-1): the session draws a frame after
//! each event, where a live loop drains a burst of input and draws once —
//! an app whose transitions see the difference diverges; times are whole
//! milliseconds.

use rdom_tui::runtime::record::{Record, encode_event, read};
use serde::Serialize;

use super::{Session, SessionError};
use crate::app::CaptureApp;
use crate::input::Input;

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
    /// What the log could not replay, where the replay stopped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unreplayable: Option<String>,
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

/// Where a session following a recording is in it.
#[derive(Debug, Clone)]
pub(crate) struct Following {
    path: std::path::PathBuf,
    /// The header line: a file that no longer starts with it is another
    /// recording.
    header: String,
    /// The lines replayed (the header included).
    lines: usize,
    at_ms: u64,
    frames: usize,
    /// Where it stopped for good (a divergence, an unreplayable mark).
    stopped: Option<Replayed>,
}

/// The session's environment and App against a recording's header.
fn check_header<A: CaptureApp>(s: &Session<A>, record: &Record) -> Result<(), SessionError> {
    let invalid = SessionError::InvalidInput;
    let Record::Header {
        width,
        height,
        config,
    } = record
    else {
        return Err(invalid("a recording starts with its header".into()));
    };
    let size = s.environment()?.viewport;
    if (size.width, size.height) != (*width, *height) {
        return Err(invalid(format!(
            "the recording is {width}×{height} and the session {}×{}: open it at the recording's size",
            size.width, size.height
        )));
    }
    let ours = s.app()?.recording_config();
    let differ: Vec<String> = config
        .iter()
        .filter_map(|(k, v)| {
            let mine = ours.iter().find(|(o, _)| o == k)?;
            (mine.1 != *v).then(|| format!("{k} is {v} there and {} here", mine.1))
        })
        .collect();
    if !differ.is_empty() {
        return Err(invalid(format!(
            "the recorded App was configured otherwise ({}): replaying it here would differ by that, not by the app",
            differ.join("; ")
        )));
    }
    Ok(())
}

impl<A: CaptureApp> Session<A> {
    /// Replay the recording `log` on the open app — freshly opened, like
    /// the recorded one, at the size and configuration the log's header
    /// names — up to `until_ms` of it (or its end).
    pub fn replay(&mut self, log: &str, until_ms: Option<u64>) -> Result<Replayed, SessionError> {
        let records = read(log).map_err(SessionError::InvalidInput)?;
        self.fresh()?;
        check_header(self, &records[0])?;
        let (mut out, _) = self.play(&records[1..], 0, 0, until_ms)?;
        if let Some(u) = until_ms
            && out.divergence.is_none()
            && out.unreplayable.is_none()
            && u > out.at_ms
        {
            self.advance(u - out.at_ms)?;
            out.at_ms = u;
        }
        Ok(out)
    }

    /// Refuse a session that already took steps: a recording's times start
    /// at the app's first frame.
    fn fresh(&self) -> Result<(), SessionError> {
        let o = self.open.as_ref().ok_or(SessionError::NotOpen)?;
        if !o.log.is_empty() || o.time_ms > 0 {
            return Err(SessionError::InvalidInput(
                "a recording replays on a freshly opened session (this one took steps): open it again"
                    .into(),
            ));
        }
        Ok(())
    }

    /// Follow the recording a running app writes at `path`: replay what it
    /// added since the last call — the whole log, the first time — on this
    /// session, which mirrors the app from then on (read-only: nothing
    /// reaches the running app). A frame line at the end is held back until
    /// the app writes on (it may draw another at the same time); a log that
    /// was replaced (a new run) is refused, and a divergence or an
    /// unreplayable mark stops the following for good.
    pub fn follow(&mut self, path: &std::path::Path) -> Result<Replayed, SessionError> {
        let invalid = SessionError::InvalidInput;
        let canonical =
            std::fs::canonicalize(path).map_err(|e| invalid(format!("{}: {e}", path.display())))?;
        let text = std::fs::read(&canonical)
            .map_err(|e| invalid(format!("reading {}: {e}", path.display())))?;
        // Whole lines only (the app may be writing one), as text — a line
        // cut in a UTF-8 sequence waits for the rest.
        let whole = text.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
        let text = String::from_utf8_lossy(&text[..whole]);
        let lines: Vec<&str> = text.lines().collect();
        let Some(header) = lines.first() else {
            return Err(invalid(format!(
                "{} holds no recording yet",
                path.display()
            )));
        };
        let o = self.open.as_ref().ok_or(SessionError::NotOpen)?;
        let (from, at_ms, frames) = match &o.following {
            Some(f) if f.path == canonical => {
                if let Some(stopped) = &f.stopped {
                    return Ok(stopped.clone());
                }
                if *header != f.header || lines.len() < f.lines {
                    return Err(invalid(format!(
                        "{} is another recording now (a new run, or cut short): open the session again to follow it",
                        path.display()
                    )));
                }
                (f.lines, f.at_ms, f.frames)
            }
            Some(f) => {
                return Err(invalid(format!(
                    "the session follows {} already: open it again to follow another recording",
                    f.path.display()
                )));
            }
            None => {
                self.fresh()?;
                let records = read(header).map_err(invalid)?;
                check_header(self, &records[0])?;
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
        let (out, used) = self.play(&records[1..], at_ms, frames, None)?;
        let stopped = (out.divergence.is_some() || out.unreplayable.is_some()).then(|| out.clone());
        let o = self.open.as_mut().expect("open");
        o.following = Some(Following {
            path: canonical,
            header: (*header).to_string(),
            lines: from + used,
            at_ms: out.at_ms,
            frames: frames + out.frames_checked,
            stopped,
        });
        Ok(out)
    }

    /// Apply `body`'s records from `at_ms`, `frames` checked before; the
    /// answer and how many records it used. Every event is checked before
    /// the first is applied.
    fn play(
        &mut self,
        body: &[Record],
        at_ms: u64,
        frames: usize,
        until_ms: Option<u64>,
    ) -> Result<(Replayed, usize), SessionError> {
        let inputs: Vec<Option<Input>> = body
            .iter()
            .map(|r| match r {
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
                    input
                        .events()
                        .map(|_| Some(input))
                        .map_err(SessionError::InvalidInput)
                }
                _ => Ok(None),
            })
            .collect::<Result<_, _>>()?;
        let mut out = Replayed {
            events: 0,
            frames_checked: 0,
            at_ms,
            divergence: None,
            unreplayable: None,
        };
        let mut frame = frames;
        for (i, record) in body.iter().enumerate() {
            let ms = match record {
                Record::Event { ms, .. }
                | Record::Frame { ms, .. }
                | Record::Unreplayable { ms, .. } => *ms,
                _ => continue,
            };
            if until_ms.is_some_and(|u| ms > u) {
                return Ok((out, i));
            }
            if ms > out.at_ms {
                self.advance(ms - out.at_ms)?;
                out.at_ms = ms;
            }
            match record {
                Record::Event { .. } => {
                    let input = inputs[i].clone().expect("an event's input");
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
                        return Ok((out, i + 1));
                    }
                    frame += 1;
                    out.frames_checked += 1;
                }
                Record::Unreplayable { what, .. } => {
                    out.unreplayable = Some(what.clone());
                    return Ok((out, i + 1));
                }
                _ => {}
            }
        }
        Ok((out, body.len()))
    }
}
