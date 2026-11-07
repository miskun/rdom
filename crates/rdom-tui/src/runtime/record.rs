//! Recording an app's input (`specs/CAPTURE.md` §Record and replay): every
//! terminal event the App handles — keys, mouse, paste, resizes, focus —
//! with its time on the App's clock, and a hash of every frame it draws,
//! as a line-based log. A person reproduces a state by hand once in a real
//! terminal (`App::with_recorder`, or `RDOM_RECORD=<path>` for
//! [`App::run`](crate::App::run)); a capture session replays the log
//! headlessly ([`read`]) and reports the first frame that differs.
//!
//! The format, one record per line, fields separated by one space, text
//! percent-encoded:
//!
//! ```text
//! rdom-record 1 <width> <height>
//! <ms> key <code> <modifier bits> press|repeat|release
//! <ms> mouse <down|up|drag>-<left|right|middle>|moved|scroll-<up|down|left|right> <column> <row> <modifier bits>
//! <ms> paste <text>
//! <ms> resize <width> <height>
//! <ms> focus in|out
//! <ms> frame <hash, 16 hex digits>
//! ```
//!
//! A key code is a named key (`enter`, `tab`, `back-tab`, `esc`, `left`, …),
//! `f<n>`, or `c:<character>`. A frame's hash is [`Buffer::content_hash`]
//! of what the App drew. Injections from other threads
//! (`AppHandle::inject`) are not recorded: they are closures.

use std::io::{self, Write};
use std::time::Instant;

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton,
    MouseEvent, MouseEventKind,
};

use crate::render::Buffer;

/// The format's version, in the header.
pub const VERSION: u32 = 1;

/// One record of a log.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Record {
    /// The terminal's size when recording began.
    Header { width: u16, height: u16 },
    /// An event the App handled, `ms` after recording began.
    Event { ms: u64, event: CtEvent },
    /// A frame the App drew, by [`Buffer::content_hash`].
    Frame { ms: u64, hash: u64 },
}

/// Writes a log as the App runs.
pub struct Recorder {
    out: Box<dyn Write>,
    start: Option<Instant>,
    last_frame: Option<u64>,
    failed: Option<io::Error>,
}

impl std::fmt::Debug for Recorder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Recorder")
            .field("last_frame", &self.last_frame)
            .field("failed", &self.failed)
            .finish_non_exhaustive()
    }
}

impl Recorder {
    /// A recorder writing to `out`.
    pub fn new(out: impl Write + 'static) -> Self {
        Recorder {
            out: Box::new(out),
            start: None,
            last_frame: None,
            failed: None,
        }
    }

    /// A recorder writing to the file at `path` (created or truncated).
    pub fn to_file(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
        Ok(Recorder::new(io::BufWriter::new(std::fs::File::create(
            path,
        )?)))
    }

    /// Why recording stopped, if a write failed (the App goes on without
    /// it).
    pub fn error(&self) -> Option<&io::Error> {
        self.failed.as_ref()
    }

    fn line(&mut self, line: &str) {
        if self.failed.is_some() {
            return;
        }
        if let Err(e) = self
            .out
            .write_all(line.as_bytes())
            .and_then(|()| self.out.flush())
        {
            self.failed = Some(e);
        }
    }

    /// Begin, at `now`, for a terminal of `width` × `height`.
    pub(crate) fn begin(&mut self, now: Instant, width: u16, height: u16) {
        if self.start.is_none() {
            self.start = Some(now);
            self.line(&format!("rdom-record {VERSION} {width} {height}\n"));
        }
    }

    fn ms(&self, now: Instant) -> u64 {
        self.start
            .map_or(0, |s| now.saturating_duration_since(s).as_millis() as u64)
    }

    /// Record `event`, handled at `now`.
    pub(crate) fn event(&mut self, now: Instant, event: &CtEvent) {
        if let Some(fields) = encode_event(event) {
            let line = format!("{} {fields}\n", self.ms(now));
            self.line(&line);
        }
    }

    /// Record the frame in `buffer`, drawn at `now`, when it differs from
    /// the last one recorded.
    pub(crate) fn frame(&mut self, now: Instant, buffer: &Buffer) {
        let hash = buffer.content_hash();
        if self.last_frame != Some(hash) {
            self.last_frame = Some(hash);
            let line = format!("{} frame {hash:016x}\n", self.ms(now));
            self.line(&line);
        }
    }
}

const KEYS: &[(KeyCode, &str)] = &[
    (KeyCode::Backspace, "backspace"),
    (KeyCode::Enter, "enter"),
    (KeyCode::Left, "left"),
    (KeyCode::Right, "right"),
    (KeyCode::Up, "up"),
    (KeyCode::Down, "down"),
    (KeyCode::Home, "home"),
    (KeyCode::End, "end"),
    (KeyCode::PageUp, "page-up"),
    (KeyCode::PageDown, "page-down"),
    (KeyCode::Tab, "tab"),
    (KeyCode::BackTab, "back-tab"),
    (KeyCode::Delete, "delete"),
    (KeyCode::Insert, "insert"),
    (KeyCode::Esc, "esc"),
    (KeyCode::Null, "null"),
    (KeyCode::CapsLock, "caps-lock"),
    (KeyCode::ScrollLock, "scroll-lock"),
    (KeyCode::NumLock, "num-lock"),
    (KeyCode::PrintScreen, "print-screen"),
    (KeyCode::Pause, "pause"),
    (KeyCode::Menu, "menu"),
    (KeyCode::KeypadBegin, "keypad-begin"),
];

fn key_name(code: KeyCode) -> Option<String> {
    match code {
        KeyCode::Char(c) => Some(format!("c:{}", escape(&c.to_string()))),
        KeyCode::F(n) => Some(format!("f{n}")),
        other => KEYS
            .iter()
            .find(|(k, _)| *k == other)
            .map(|(_, n)| (*n).to_string()),
    }
}

fn key_code(name: &str) -> Option<KeyCode> {
    if let Some(c) = name.strip_prefix("c:") {
        let s = unescape(c)?;
        let mut chars = s.chars();
        let c = chars.next()?;
        return chars.next().is_none().then_some(KeyCode::Char(c));
    }
    if let Some(n) = name.strip_prefix('f').and_then(|n| n.parse().ok()) {
        return Some(KeyCode::F(n));
    }
    KEYS.iter().find(|(_, n)| *n == name).map(|(k, _)| *k)
}

const BUTTONS: [(MouseButton, &str); 3] = [
    (MouseButton::Left, "left"),
    (MouseButton::Right, "right"),
    (MouseButton::Middle, "middle"),
];

fn button_name(b: MouseButton) -> &'static str {
    BUTTONS
        .iter()
        .find(|(x, _)| *x == b)
        .map_or("left", |(_, n)| n)
}

fn button(name: &str) -> Option<MouseButton> {
    BUTTONS.iter().find(|(_, n)| *n == name).map(|(b, _)| *b)
}

/// `event` as a record line's fields after its time (`key c:a 0 press`);
/// `None` for an event the format does not hold (a key code without a
/// name: a media or modifier key).
pub fn encode_event(event: &CtEvent) -> Option<String> {
    Some(match event {
        CtEvent::Key(k) => {
            let kind = match k.kind {
                KeyEventKind::Press => "press",
                KeyEventKind::Repeat => "repeat",
                KeyEventKind::Release => "release",
            };
            format!("key {} {} {kind}", key_name(k.code)?, k.modifiers.bits())
        }
        CtEvent::Mouse(m) => {
            let kind = match m.kind {
                MouseEventKind::Down(b) => format!("down-{}", button_name(b)),
                MouseEventKind::Up(b) => format!("up-{}", button_name(b)),
                MouseEventKind::Drag(b) => format!("drag-{}", button_name(b)),
                MouseEventKind::Moved => "moved".into(),
                MouseEventKind::ScrollUp => "scroll-up".into(),
                MouseEventKind::ScrollDown => "scroll-down".into(),
                MouseEventKind::ScrollLeft => "scroll-left".into(),
                MouseEventKind::ScrollRight => "scroll-right".into(),
            };
            format!("mouse {kind} {} {} {}", m.column, m.row, m.modifiers.bits())
        }
        CtEvent::Paste(text) => format!("paste {}", escape(text)),
        CtEvent::Resize(w, h) => format!("resize {w} {h}"),
        CtEvent::FocusGained => "focus in".into(),
        CtEvent::FocusLost => "focus out".into(),
    })
}

/// `s` with `%`, space, control characters and line breaks
/// percent-encoded (UTF-8 bytes).
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c == '%' || c == ' ' || c.is_control() {
            let mut buf = [0u8; 4];
            for b in c.encode_utf8(&mut buf).bytes() {
                out.push_str(&format!("%{b:02X}"));
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn unescape(s: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(s.len());
    let mut it = s.bytes();
    while let Some(b) = it.next() {
        if b == b'%' {
            let hex = [it.next()?, it.next()?];
            bytes.push(u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?);
        } else {
            bytes.push(b);
        }
    }
    String::from_utf8(bytes).ok()
}

/// Every record of a log, in order; `Err` names the first line that is
/// not one (1-based) and why.
pub fn read(log: &str) -> Result<Vec<Record>, String> {
    let mut out = Vec::new();
    for (i, line) in log.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let bad = |why: &str| format!("line {}: {why}: {line:?}", i + 1);
        let f: Vec<&str> = line.split(' ').collect();
        if i == 0 {
            match f.as_slice() {
                ["rdom-record", v, w, h] => {
                    if v.parse::<u32>().ok() != Some(VERSION) {
                        return Err(bad("a log of another version"));
                    }
                    let n = |s: &str| s.parse::<u16>().map_err(|_| bad("not a size"));
                    out.push(Record::Header {
                        width: n(w)?,
                        height: n(h)?,
                    });
                    continue;
                }
                _ => return Err(bad("not an rdom-record header")),
            }
        }
        let ms: u64 = f
            .first()
            .and_then(|t| t.parse().ok())
            .ok_or_else(|| bad("no time"))?;
        let rest = f[1..].join(" ");
        let record = match &f[1..] {
            ["frame", hash] => Record::Frame {
                ms,
                hash: u64::from_str_radix(hash, 16).map_err(|_| bad("not a hash"))?,
            },
            _ => Record::Event {
                ms,
                event: parse_event(&rest).map_err(|why| bad(&why))?,
            },
        };
        out.push(record);
    }
    if !matches!(out.first(), Some(Record::Header { .. })) {
        return Err("an empty log: no rdom-record header".into());
    }
    Ok(out)
}

/// One event from a record line's fields after its time (the inverse of
/// [`encode_event`]); `Err` says why it is not one.
pub fn parse_event(fields: &str) -> Result<CtEvent, String> {
    let f: Vec<&str> = fields.split(' ').collect();
    let num = |s: &str| {
        s.parse::<u16>()
            .map_err(|_| format!("{s:?} is not a number"))
    };
    let mods = |s: &str| {
        s.parse::<u8>()
            .ok()
            .and_then(KeyModifiers::from_bits)
            .ok_or_else(|| format!("{s:?} is not modifier bits"))
    };
    Ok(match f.as_slice() {
        ["key", code, m, kind] => CtEvent::Key(KeyEvent {
            code: key_code(code).ok_or_else(|| format!("no such key {code:?}"))?,
            modifiers: mods(m)?,
            kind: match *kind {
                "press" => KeyEventKind::Press,
                "repeat" => KeyEventKind::Repeat,
                "release" => KeyEventKind::Release,
                k => return Err(format!("no such key event kind {k:?}")),
            },
            state: KeyEventState::NONE,
        }),
        ["mouse", kind, col, row, m] => {
            let kind = match *kind {
                "moved" => MouseEventKind::Moved,
                "scroll-up" => MouseEventKind::ScrollUp,
                "scroll-down" => MouseEventKind::ScrollDown,
                "scroll-left" => MouseEventKind::ScrollLeft,
                "scroll-right" => MouseEventKind::ScrollRight,
                k => {
                    let no = || format!("no such mouse event {k:?}");
                    let (what, b) = k.split_once('-').ok_or_else(no)?;
                    let b = button(b).ok_or_else(no)?;
                    match what {
                        "down" => MouseEventKind::Down(b),
                        "up" => MouseEventKind::Up(b),
                        "drag" => MouseEventKind::Drag(b),
                        _ => return Err(no()),
                    }
                }
            };
            CtEvent::Mouse(MouseEvent {
                kind,
                column: num(col)?,
                row: num(row)?,
                modifiers: mods(m)?,
            })
        }
        ["paste", text] => {
            CtEvent::Paste(unescape(text).ok_or_else(|| format!("bad escape in {text:?}"))?)
        }
        ["resize", w, h] => CtEvent::Resize(num(w)?, num(h)?),
        ["focus", "in"] => CtEvent::FocusGained,
        ["focus", "out"] => CtEvent::FocusLost,
        _ => return Err("not an event".into()),
    })
}

#[cfg(test)]
mod tests {
    use crossterm::event::{
        Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
    };

    use super::{Record, encode_event, read};

    fn key(code: KeyCode, mods: KeyModifiers) -> CtEvent {
        CtEvent::Key(KeyEvent {
            code,
            modifiers: mods,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    #[test]
    fn every_event_round_trips_through_its_line() {
        let events = [
            key(KeyCode::Char('a'), KeyModifiers::NONE),
            key(KeyCode::Char(' '), KeyModifiers::CONTROL),
            key(KeyCode::Char('%'), KeyModifiers::SHIFT | KeyModifiers::ALT),
            key(KeyCode::Char('ä'), KeyModifiers::NONE),
            key(KeyCode::F(5), KeyModifiers::NONE),
            key(KeyCode::BackTab, KeyModifiers::SHIFT),
            CtEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Drag(MouseButton::Right),
                column: 7,
                row: 3,
                modifiers: KeyModifiers::NONE,
            }),
            CtEvent::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 0,
                row: 0,
                modifiers: KeyModifiers::CONTROL,
            }),
            CtEvent::Paste("two words\nand a 100% line".into()),
            CtEvent::Resize(120, 40),
            CtEvent::FocusGained,
            CtEvent::FocusLost,
        ];
        let mut log = String::from("rdom-record 1 80 24\n");
        for (i, e) in events.iter().enumerate() {
            log.push_str(&format!("{} {}\n", i * 10, encode_event(e).unwrap()));
        }
        log.push_str("120 frame 00ff00ff00ff00ff\n");
        let records = read(&log).unwrap();
        assert_eq!(
            records[0],
            Record::Header {
                width: 80,
                height: 24
            }
        );
        for (i, e) in events.iter().enumerate() {
            assert_eq!(
                records[i + 1],
                Record::Event {
                    ms: i as u64 * 10,
                    event: e.clone()
                }
            );
        }
        assert_eq!(
            records.last(),
            Some(&Record::Frame {
                ms: 120,
                hash: 0x00ff00ff00ff00ff
            })
        );
    }

    #[test]
    fn a_bad_line_is_named() {
        let err = read("rdom-record 1 80 24\n5 key nope 0 press\n").unwrap_err();
        assert!(err.starts_with("line 2: no such key"), "{err}");
        assert!(read("hello").is_err());
        assert!(
            read("rdom-record 2 80 24\n")
                .unwrap_err()
                .contains("version")
        );
    }
}
