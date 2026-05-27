//! Lightweight file-based tracing for interactive runtime debugging.
//!
//! In TUI mode stdout/stderr are the alt-screen, so println!/eprintln!
//! either get hidden or smear into the painted UI. This module writes
//! to a file path instead, controlled by the `RDOM_TRACE` env var:
//!
//! ```bash
//! RDOM_TRACE=/tmp/rdom-trace.log cargo run -p rdom-showcase
//! tail -f /tmp/rdom-trace.log
//! ```
//!
//! The macro is a no-op when the env var is unset, so leaving
//! `trace!(...)` calls in shipped code costs only a single atomic
//! load per invocation. Open the file lazily on first hit; failures
//! to open silently disable tracing for the rest of the process so
//! the app keeps running.

use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Instant;

struct TraceState {
    started: Instant,
    writer: Mutex<BufWriter<std::fs::File>>,
}

static STATE: OnceLock<Option<TraceState>> = OnceLock::new();

fn state() -> &'static Option<TraceState> {
    STATE.get_or_init(|| {
        let path = std::env::var("RDOM_TRACE").ok()?;
        if path.is_empty() {
            return None;
        }
        // APPEND so consecutive process launches accumulate in the
        // same file — lets us compare a working session against a
        // broken one without losing the working data on relaunch.
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok()?;
        let mut writer = BufWriter::new(file);
        let _ = writeln!(
            writer,
            "\n========== process start (pid={}) ==========",
            std::process::id()
        );
        let _ = writer.flush();
        Some(TraceState {
            started: Instant::now(),
            writer: Mutex::new(writer),
        })
    })
}

/// Returns true when tracing is enabled (env var set + file open
/// succeeded). Callers use this to skip expensive `format!` work
/// when tracing is off — the [`trace!`] macro does this for you.
#[inline]
pub fn enabled() -> bool {
    state().is_some()
}

/// Write a line to the trace log, prefixed with monotonic
/// milliseconds since process start. Flushes immediately so
/// `tail -f` shows entries as they happen.
pub fn write_line(line: &str) {
    let Some(s) = state().as_ref() else {
        return;
    };
    let ms = s.started.elapsed().as_millis();
    let Ok(mut w) = s.writer.lock() else { return };
    let _ = writeln!(w, "[{ms:>8}ms] {line}");
    let _ = w.flush();
}

/// `trace!("fmt", args...)` — formats and writes a line when
/// `RDOM_TRACE` is set, no-op otherwise.
#[macro_export]
macro_rules! rdom_trace {
    ($($arg:tt)*) => {
        if $crate::runtime::trace::enabled() {
            $crate::runtime::trace::write_line(&format!($($arg)*));
        }
    };
}
