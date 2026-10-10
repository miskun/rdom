//! C16G-HARDENING (Phase 16 gate decision 4): what an `App` does with a
//! process signal — the decision table, and the whole redraw after a stop.

use super::signals::{SignalAction, action};
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::input::Signal;
use crate::{Stylesheet, TuiDom};

/// SIGTERM / SIGHUP restore the terminal and end the process as the
/// signal would; SIGTSTP leaves the terminal as it was before stopping —
/// and re-enters it after — when the App set it up, and only stops when
/// it did not; SIGCONT redraws the whole screen.
#[test]
fn each_signal_has_its_action() {
    use signal_hook::consts::{SIGHUP, SIGTERM};
    for owns in [true, false] {
        assert_eq!(
            action(Signal::Terminate(SIGTERM), owns),
            SignalAction::RestoreAndExit(SIGTERM)
        );
        assert_eq!(
            action(Signal::Terminate(SIGHUP), owns),
            SignalAction::RestoreAndExit(SIGHUP)
        );
        assert_eq!(action(Signal::Continue, owns), SignalAction::RedrawWhole);
    }
    assert_eq!(
        action(Signal::Suspend, true),
        SignalAction::SuspendAndResume
    );
    assert_eq!(action(Signal::Suspend, false), SignalAction::Stop);
}

/// After a stop the screen may hold anything (the shell's output): the
/// next frame draws every cell again, though nothing in the tree changed.
#[test]
fn a_continue_redraws_every_cell() {
    let mut dom = TuiDom::new();
    let p = dom.create_element("p");
    let t = dom.create_text_node("hello");
    dom.append_child(p, t).unwrap();
    let root = dom.root();
    dom.append_child(root, p).unwrap();
    let terminal = Terminal::new(TestBackend::new(8, 2)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.draw_if_dirty().unwrap();
    app.terminal_mut().backend_mut().clear_bytes();
    app.draw_if_dirty().unwrap();
    assert!(
        app.terminal().backend().bytes().is_empty(),
        "a clean frame draws nothing"
    );
    app.redraw_whole();
    app.draw_if_dirty().unwrap();
    let bytes = String::from_utf8_lossy(app.terminal().backend().bytes()).into_owned();
    assert!(bytes.contains("hello"), "every cell again: {bytes:?}");
}
