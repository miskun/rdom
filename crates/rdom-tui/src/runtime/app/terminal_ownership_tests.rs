//! ACID-FIX-16 — an `App` writes terminal modes only to a terminal it set
//! up. `App::new` enters TUI mode on stdout (alternate screen, raw input,
//! mouse capture) and re-arms the mouse capture on every resize, as some
//! terminals and multiplexers drop it across one; an `App` built
//! `with_backend` set up no mode on stdout and must write nothing there —
//! its output is its backend's (a headless `TestBackend`, a pipe, another
//! tty). Found by acid step I19, whose resizes printed the mouse-capture
//! sequences into the test run's own output.
//!
//! Process stdout cannot be read from inside the test that writes it, so
//! the check runs the writing half in a child test process and reads the
//! child's stdout.

use crossterm::event::Event as CtEvent;

use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::Stylesheet;

/// The child half: a headless app resized twice. Ignored in a normal run;
/// [`a_headless_resize_writes_nothing_to_stdout`] runs it.
#[test]
#[ignore = "run by a_headless_resize_writes_nothing_to_stdout in a child process"]
fn headless_resize_child() {
    let terminal = Terminal::new(TestBackend::new(20, 5)).unwrap();
    let mut app = App::with_backend(TuiDom::new(), Stylesheet::new(), terminal).unwrap();
    app.advance(0).unwrap();
    app.terminal_mut().backend_mut().resize(30, 6);
    app.handle_event(CtEvent::Resize(30, 6));
    app.advance(0).unwrap();
    print!("child-done");
}

#[test]
fn a_headless_resize_writes_nothing_to_stdout() {
    let exe = std::env::current_exe().expect("the test binary");
    let out = std::process::Command::new(exe)
        .args([
            "--exact",
            "runtime::app::terminal_ownership_tests::headless_resize_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .output()
        .expect("the child test runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "the child failed: {stdout}");
    assert!(stdout.contains("child-done"), "the child ran: {stdout:?}");
    assert!(
        !stdout.contains("\x1b[?1000h") && !stdout.contains("\x1b[?1006h"),
        "a with_backend app wrote mouse-capture modes to stdout: {stdout:?}"
    );
}
