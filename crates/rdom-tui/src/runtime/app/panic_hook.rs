//! Process-global panic hook that restores the terminal before
//! the default hook prints the panic message.
//!
//! ## Why this exists
//!
//! When a panic starts unwinding, Rust's default panic hook prints
//! the message and backtrace to `stderr`. If we're still in alt-
//! screen and raw mode when that happens, the user sees nothing
//! useful — the message lands on the alt-screen buffer which gets
//! erased on exit.
//!
//! This module installs a hook that runs `leave_tui_mode` first
//! (restoring the main screen + cooked mode), then delegates to the
//! previous hook. Combined with `TerminalGuard::drop` (which runs
//! during stack unwinding), the terminal is definitively restored
//! regardless of which cleanup path wins.
//!
//! ## Install-once
//!
//! A global `AtomicBool` guards the install. Multiple `App::new`
//! calls don't stack hooks; re-running the binary picks up a fresh
//! process with a fresh hook. We never uninstall: the hook is
//! harmless when TUI mode isn't active (`leave_tui_mode` on an
//! already-cooked terminal emits idempotent sequences).

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::render::backend_crossterm::leave_tui_mode;

static INSTALLED: AtomicBool = AtomicBool::new(false);

/// Install the terminal-restoring panic hook. Idempotent — multiple
/// calls leave only the first install in place. Safe to call before,
/// during, or after any `App::run` session.
pub fn install() {
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = leave_tui_mode(&mut io::stdout());
        previous(info);
    }));
}

/// `true` when [`install`] has been called at least once this
/// process. Exposed for test assertion only.
#[cfg(test)]
pub(crate) fn is_installed() -> bool {
    INSTALLED.load(Ordering::SeqCst)
}
