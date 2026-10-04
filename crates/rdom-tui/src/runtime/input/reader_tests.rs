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

/// A loop that comes back late — past the grace — finds a whole
/// sequence queued: it is read before the grace flushes anything, so
/// `ESC [ A` is Up, not Esc and a typed `[A` (`C4G-ESC-GRACE`; ECMA-48
/// §5.4 frames the sequence by its final byte, and the bytes are
/// already there).
#[test]
fn a_late_poll_reads_queued_bytes_before_the_grace_flushes() {
    let (mut r, mut t) = reader();
    t.write_all(b"\x1b").unwrap();
    assert!(!r.poll(Duration::ZERO).unwrap());
    std::thread::sleep(ESC_GRACE + Duration::from_millis(20));
    t.write_all(b"[A").unwrap();
    assert!(r.poll(Duration::ZERO).unwrap());
    assert_eq!(r.next(), Some(key(KeyCode::Up, KeyModifiers::NONE)));
    assert_eq!(r.next(), None);
}

/// Past the grace with nothing queued, the lone ESC is still Esc — even
/// on a zero-timeout poll.
#[test]
fn a_lone_escape_flushes_on_a_late_zero_timeout_poll() {
    let (mut r, mut t) = reader();
    t.write_all(b"\x1b").unwrap();
    assert!(!r.poll(Duration::ZERO).unwrap());
    std::thread::sleep(ESC_GRACE + Duration::from_millis(20));
    assert!(r.poll(Duration::ZERO).unwrap());
    assert_eq!(r.next(), Some(key(KeyCode::Esc, KeyModifiers::NONE)));
}

/// A read that fills the buffer may have left more behind (crossterm's
/// rule): one poll reads on, so a sequence straddling the buffer's end
/// is whole after it.
#[test]
fn a_full_read_reads_on() {
    let (mut r, mut t) = reader();
    let mut bytes = vec![b'a'; 1023];
    bytes.extend_from_slice(b"\x1b[A");
    t.write_all(&bytes).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    assert!(r.poll(Duration::ZERO).unwrap());
    let all: Vec<Input> = std::iter::from_fn(|| r.next()).collect();
    assert_eq!(all.len(), 1024);
    assert_eq!(all[1023], key(KeyCode::Up, KeyModifiers::NONE));
}
