//! The startup query's exchange: which inputs are its replies, and how
//! long it waits. (Parsing the replies' bytes — OSC 11 forms, DA1,
//! fragments — is the input parser's corpus, `runtime::input`.)

use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEvent};

use super::*;

fn key(c: char) -> Input {
    Input::Event(Event::Key(KeyEvent::from(KeyCode::Char(c))))
}

/// The OSC 11 and DA1 replies are the query's; a key typed meanwhile is
/// handed back for the App.
#[test]
fn replies_are_taken_and_keys_handed_back() {
    let mut r = Replies::default();
    assert_eq!(r.take(key('x')), Some(key('x')));
    assert_eq!(r.take(Input::Background(Color::Rgb(0, 0x2b, 0x36))), None);
    assert!(!r.complete());
    assert_eq!(r.take(Input::DeviceAttributes), None);
    assert!(r.complete());
    assert_eq!(r.background(), Some(Color::Rgb(0, 0x2b, 0x36)));
}

/// A terminal that does not answer OSC 11 still answers DA1.
#[test]
fn da1_alone_completes_without_a_background() {
    let mut r = Replies::default();
    r.take(Input::DeviceAttributes);
    assert!(r.complete());
    assert_eq!(r.background(), None);
}

// ── Waiting for a reply that has started (C3G-OSC-ROBUST) ───────────

const T: Duration = Duration::from_millis(200);
const GRACE: Duration = Duration::from_millis(800);

/// A reply has started once one has been read or the reader is partway
/// through an escape sequence; a typed key starts nothing.
#[test]
fn a_reply_has_started_once_one_arrives() {
    let mut r = Replies::default();
    assert!(!r.started(false));
    r.take(key('x'));
    assert!(!r.started(false));
    assert!(r.started(true));
    r.take(Input::Background(Color::Rgb(1, 2, 3)));
    assert!(r.started(false));
}

/// The wait: the base timeout while nothing has come; once a reply has
/// started and is not complete, the grace on top — a slow link (ssh)
/// delivers the reply in pieces past 200 ms; nothing once complete.
#[test]
fn the_wait_extends_once_a_reply_has_started() {
    let start = Instant::now();
    let at = |ms: u64| start + Duration::from_millis(ms);
    let none = Replies::default();
    assert_eq!(
        wait_left(at(50), start, &none, false, T, GRACE),
        Duration::from_millis(150)
    );
    assert_eq!(
        wait_left(at(250), start, &none, true, T, GRACE),
        Duration::from_millis(750)
    );
    assert_eq!(
        wait_left(at(1_100), start, &none, true, T, GRACE),
        Duration::ZERO
    );
    let mut done = Replies::default();
    done.take(Input::DeviceAttributes);
    assert_eq!(
        wait_left(at(10), start, &done, false, T, GRACE),
        Duration::ZERO
    );
}

/// A reply that has not begun by the base timeout is not waited for: a
/// terminal that answers late (or never) costs 200 ms, not the grace.
#[test]
fn a_late_reply_is_not_waited_for() {
    let start = Instant::now();
    let none = Replies::default();
    assert_eq!(
        wait_left(
            start + Duration::from_millis(200),
            start,
            &none,
            false,
            T,
            GRACE
        ),
        Duration::ZERO
    );
}
