//! The reader over a socket standing in for the terminal: reads joined
//! across `poll`s, the escape grace, end of input.

use std::io::Write;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use super::*;

fn reader() -> (InputReader, UnixStream) {
    let (ours, theirs) = UnixStream::pair().unwrap();
    (InputReader::from_fd(ours.into()), theirs)
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> Input {
    Input::Event(Event::Key(KeyEvent::new(code, modifiers)))
}

/// Nothing sent: a zero-timeout poll returns at once, a timed one at
/// its timeout.
#[test]
fn polling_an_idle_terminal_times_out() {
    let (mut r, _t) = reader();
    assert!(!r.poll(Duration::ZERO).unwrap());
    let start = Instant::now();
    assert!(!r.poll(Duration::from_millis(30)).unwrap());
    assert!(start.elapsed() >= Duration::from_millis(30));
}

/// A sequence split across two writes is one key; the first half alone
/// is nothing yet.
#[test]
fn a_sequence_split_across_reads_is_one_key() {
    let (mut r, mut t) = reader();
    t.write_all(b"\x1b[1;").unwrap();
    assert!(!r.poll(Duration::from_millis(10)).unwrap());
    t.write_all(b"5A").unwrap();
    assert!(r.poll(Duration::from_millis(500)).unwrap());
    assert_eq!(r.next(), Some(key(KeyCode::Up, KeyModifiers::CONTROL)));
}

/// A lone ESC is Esc once the grace passes with nothing after it — even
/// when the caller polls with a zero timeout in between.
#[test]
fn a_lone_escape_becomes_esc_after_the_grace() {
    let (mut r, mut t) = reader();
    t.write_all(b"\x1b").unwrap();
    assert!(
        !r.poll(Duration::ZERO).unwrap(),
        "not yet: it may start a sequence"
    );
    let start = Instant::now();
    assert!(r.poll(Duration::from_millis(500)).unwrap());
    assert!(start.elapsed() < Duration::from_millis(400));
    assert_eq!(r.next(), Some(key(KeyCode::Esc, KeyModifiers::NONE)));
}

/// ESC then a key within the grace is Alt + the key; after it, two keys.
#[test]
fn escape_then_a_key_is_alt_within_the_grace() {
    let (mut r, mut t) = reader();
    t.write_all(b"\x1b").unwrap();
    assert!(!r.poll(Duration::ZERO).unwrap());
    t.write_all(b"x").unwrap();
    assert!(r.poll(Duration::from_millis(500)).unwrap());
    assert_eq!(r.next(), Some(key(KeyCode::Char('x'), KeyModifiers::ALT)));

    t.write_all(b"\x1b").unwrap();
    assert!(r.poll(Duration::from_millis(500)).unwrap());
    assert_eq!(r.next(), Some(key(KeyCode::Esc, KeyModifiers::NONE)));
    t.write_all(b"x").unwrap();
    assert!(r.poll(Duration::from_millis(500)).unwrap());
    assert_eq!(r.next(), Some(key(KeyCode::Char('x'), KeyModifiers::NONE)));
}

/// The terminal going away is an error, not an endless stream of
/// nothing.
#[test]
fn end_of_input_is_an_error() {
    let (mut r, t) = reader();
    drop(t);
    let err = r.poll(Duration::from_millis(500)).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
}
