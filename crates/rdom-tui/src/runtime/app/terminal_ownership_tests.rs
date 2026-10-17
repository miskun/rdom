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

/// The child half of [`a_rearming_app_rearms_on_resize`]: a headless app
/// that opted in with `with_mouse_rearm(true)`, resized once.
#[test]
#[ignore = "run by a_rearming_app_rearms_on_resize in a child process"]
fn rearming_resize_child() {
    let terminal = Terminal::new(TestBackend::new(20, 5)).unwrap();
    let mut app = App::with_backend(TuiDom::new(), Stylesheet::new(), terminal)
        .unwrap()
        .with_mouse_rearm(true);
    app.advance(0).unwrap();
    app.terminal_mut().backend_mut().resize(30, 6);
    app.handle_event(CtEvent::Resize(30, 6));
    app.advance(0).unwrap();
    print!("child-done");
}

/// The Phase 16 API gate's N5: an app that set up its own crossterm
/// terminal on stdout and passes it `with_backend` re-arms the mouse
/// capture on a resize by opting in (C16G-MOUSE-REARM) — the
/// `sc-resize-rearm` workaround was a `resize` listener writing to stdout.
#[test]
fn a_rearming_app_rearms_on_resize() {
    let exe = std::env::current_exe().expect("the test binary");
    let out = std::process::Command::new(exe)
        .args([
            "--exact",
            "runtime::app::terminal_ownership_tests::rearming_resize_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .output()
        .expect("the child test runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "the child failed: {stdout}");
    assert!(stdout.contains("\x1b[?1000h"), "re-armed: {stdout:?}");
}

/// The child half of [`the_panic_hook_writes_only_while_a_terminal_is_held`]:
/// the hook installed, then a panic with no `TerminalGuard` armed (an app
/// that has ended, or one that never entered TUI mode on stdout), or —
/// with `RDOM_TEST_ARMED` set — with one armed.
#[test]
#[ignore = "run by the_panic_hook_writes_only_while_a_terminal_is_held in a child process"]
fn panicking_child() {
    crate::runtime::app::panic_hook::install();
    let _guard = std::env::var_os("RDOM_TEST_ARMED").map(|_| crate::render::TerminalGuard::new());
    print!("child-start;");
    panic!("the child panics");
}

/// The Phase 16 gate's architect N10: the panic hook is process-global
/// and stays installed, so it must restore the terminal only while a
/// `TerminalGuard` holds it — a panic after the app has ended (or in a
/// program that never entered TUI mode on stdout) writes no mode
/// sequences into its stdout (C16G-HYGIENE).
#[test]
fn the_panic_hook_writes_only_while_a_terminal_is_held() {
    let run = |armed: bool| {
        let exe = std::env::current_exe().expect("the test binary");
        let mut cmd = std::process::Command::new(exe);
        cmd.args([
            "--exact",
            "runtime::app::terminal_ownership_tests::panicking_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ]);
        if armed {
            cmd.env("RDOM_TEST_ARMED", "1");
        } else {
            cmd.env_remove("RDOM_TEST_ARMED");
        }
        let out = cmd.output().expect("the child test runs");
        assert!(!out.status.success(), "the child panicked");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let idle = run(false);
    assert!(idle.contains("child-start;"), "the child ran: {idle:?}");
    assert!(
        !idle.contains("\x1b[?1000l"),
        "no guard, no restore: {idle:?}"
    );
    let held = run(true);
    assert!(
        held.contains("\x1b[?1000l"),
        "a held terminal is restored: {held:?}"
    );
}
