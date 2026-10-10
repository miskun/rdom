//! What an [`App`] does with a process signal (Unix, C16G-HARDENING; the
//! Phase 16 gate's decision 4). The reader turns SIGTERM, SIGHUP,
//! SIGTSTP and SIGCONT into `Input::Signal`s (their default actions do
//! not happen while it listens), and `run` acts on each:
//!
//! - **SIGTERM / SIGHUP** — restore the terminal (raw mode, the
//!   alternate screen, mouse capture, the keyboard flags), then end the
//!   process as the signal would (`emulate_default_handler`), so a
//!   `kill` or a closed window leaves a usable shell and the parent sees
//!   the signal's status.
//! - **SIGTSTP** (Ctrl+Z) — when the App set the terminal up, leave TUI
//!   mode, stop, and on resume enter it again and redraw every cell; an
//!   App given a terminal it did not set up only stops.
//! - **SIGCONT** — redraw every cell: while stopped (by SIGTSTP, or by a
//!   SIGSTOP no handler sees), the screen may have been drawn over.

use super::App;
use super::redraw::Redraw;
use crate::render::backend::Backend;
use crate::runtime::input::Signal;

/// What a signal asks of an App.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(unix), allow(dead_code))]
pub(crate) enum SignalAction {
    /// Restore the terminal, then end the process as the signal does.
    RestoreAndExit(i32),
    /// Leave TUI mode, stop, enter it again and redraw whole.
    SuspendAndResume,
    /// Stop, the terminal left to whoever set it up.
    Stop,
    /// Draw every cell at the next frame.
    RedrawWhole,
}

/// The action for `signal`, in an App that set up its terminal
/// (`owns_terminal`: built by `App::new`) or not.
#[cfg_attr(not(unix), allow(dead_code))]
pub(crate) fn action(signal: Signal, owns_terminal: bool) -> SignalAction {
    match signal {
        Signal::Terminate(number) => SignalAction::RestoreAndExit(number),
        Signal::Suspend if owns_terminal => SignalAction::SuspendAndResume,
        Signal::Suspend => SignalAction::Stop,
        Signal::Continue => SignalAction::RedrawWhole,
    }
}

impl<B: Backend> App<B> {
    /// Draw every cell at the next frame, and forget the terminal's
    /// style state: whatever is on the screen now is not what the App
    /// drew.
    pub(crate) fn redraw_whole(&mut self) {
        self.terminal.queue_full_redraw();
        self.terminal.backend_mut().reset_style_cache();
        self.redraw.note(Redraw::Paint);
    }
}

#[cfg(unix)]
impl App<crate::render::CrosstermBackend<std::io::Stdout>> {
    /// Act on `signal` (module doc).
    pub(super) fn on_signal(&mut self, signal: Signal) -> std::io::Result<()> {
        use crate::render::backend_crossterm::{
            enter_theme_reports, enter_tui_mode, leave_tui_mode,
        };
        use signal_hook::low_level::emulate_default_handler;
        let mut stdout = std::io::stdout();
        match action(signal, self.guard.is_some()) {
            SignalAction::RestoreAndExit(number) => {
                // Every step is tried even after one fails; a closed
                // terminal (SIGHUP) fails them all, and the process
                // still ends.
                let _ = leave_tui_mode(&mut stdout);
                let _ = emulate_default_handler(number);
                // Not reached: the default action of both ends the process.
                std::process::exit(128 + number);
            }
            SignalAction::SuspendAndResume => {
                leave_tui_mode(&mut stdout)?;
                emulate_default_handler(signal_hook::consts::SIGTSTP)?;
                // Running again (SIGCONT).
                enter_tui_mode(&mut stdout)?;
                enter_theme_reports(&mut stdout)?;
                self.redraw_whole();
            }
            SignalAction::Stop => {
                emulate_default_handler(signal_hook::consts::SIGTSTP)?;
                self.redraw_whole();
            }
            SignalAction::RedrawWhole => self.redraw_whole(),
        }
        Ok(())
    }
}
