//! The startup query's reply scanner: OSC 11 (xterm "dynamic colors",
//! the background) and the DA1 reply that ends the exchange.

use std::time::{Duration, Instant};

use super::*;

fn scan(chunks: &[&[u8]]) -> Replies {
    let mut r = Replies::default();
    for c in chunks {
        r.feed(c);
    }
    r
}

/// An OSC 11 reply ended by BEL, then DA1: the exchange is complete.
#[test]
fn osc11_with_bel_then_da1() {
    let r = scan(&[b"\x1b]11;rgb:0000/2b2b/3636\x07\x1b[?62;22c"]);
    assert!(r.complete());
    assert_eq!(r.background(), Some(Color::Rgb(0, 0x2b, 0x36)));
}

/// ST (`ESC \`) ends the reply too; components have 1–4 hex digits,
/// scaled to a byte.
#[test]
fn osc11_component_widths_and_st() {
    let bg = |reply: &[u8]| scan(&[reply]).background();
    assert_eq!(
        bg(b"\x1b]11;rgb:fd/f6/e3\x1b\\"),
        Some(Color::Rgb(0xfd, 0xf6, 0xe3))
    );
    assert_eq!(bg(b"\x1b]11;rgb:f/0/f\x07"), Some(Color::Rgb(255, 0, 255)));
    assert_eq!(
        bg(b"\x1b]11;rgb:fff/000/800\x07"),
        Some(Color::Rgb(255, 0, 128))
    );
    assert_eq!(
        bg(b"\x1b]11;rgba:ffff/ffff/ffff/ffff\x07"),
        Some(Color::Rgb(255, 255, 255))
    );
    assert_eq!(
        bg(b"\x1b]11;#102030\x07"),
        Some(Color::Rgb(0x10, 0x20, 0x30))
    );
    assert_eq!(bg(b"\x1b]11;rgb:zz/00/00\x07"), None);
    assert_eq!(bg(b"\x1b]11;rgb:00/00\x07"), None);
}

/// A terminal that does not answer OSC 11 still answers DA1.
#[test]
fn da1_alone_completes_without_a_background() {
    let r = scan(&[b"\x1b[?1;2c"]);
    assert!(r.complete());
    assert_eq!(r.background(), None);
}

/// Replies arrive in pieces, possibly after unrelated bytes.
#[test]
fn replies_in_fragments() {
    let r = scan(&[
        b"x\x1b]1",
        b"1;rgb:ff",
        b"ff/0000/0000\x1b",
        b"\\\x1b[?6",
        b"5;1c",
    ]);
    assert!(r.complete());
    assert_eq!(r.background(), Some(Color::Rgb(255, 0, 0)));
    assert!(!scan(&[b"\x1b]11;rgb:ffff/0000/0000\x07\x1b[?65"]).complete());
}

// ── Waiting for a reply that has started (C3G-OSC-ROBUST) ───────────

const T: Duration = Duration::from_millis(200);
const GRACE: Duration = Duration::from_millis(800);

/// A reply has started once a reply introducer — OSC (`ESC ]`), a DA1
/// reply (`ESC [ ?`), or an `ESC` the next read may complete — has
/// arrived; unrelated bytes (a keystroke) start nothing.
#[test]
fn a_reply_has_started_once_an_introducer_arrives() {
    assert!(!scan(&[]).started());
    assert!(!scan(&[b"x"]).started());
    assert!(scan(&[b"\x1b]11;rgb:ff"]).started());
    assert!(scan(&[b"\x1b[?6"]).started());
    assert!(scan(&[b"x\x1b"]).started());
}

/// The wait: the base timeout while nothing has come; once a reply has
/// started and is not complete, the grace on top — a slow link (ssh)
/// delivers the reply in pieces past 200 ms, and stopping then would
/// leave its tail for the input reader to take as keystrokes; nothing
/// once complete.
#[test]
fn the_wait_extends_once_a_reply_has_started() {
    let start = Instant::now();
    let at = |ms: u64| start + Duration::from_millis(ms);
    let none = scan(&[]);
    assert_eq!(
        wait_left(at(50), start, &none, T, GRACE),
        Duration::from_millis(150)
    );
    let partial = scan(&[b"\x1b]11;rgb:ffff/"]);
    assert_eq!(
        wait_left(at(250), start, &partial, T, GRACE),
        Duration::from_millis(750)
    );
    assert_eq!(
        wait_left(at(1_100), start, &partial, T, GRACE),
        Duration::ZERO
    );
    let done = scan(&[b"\x1b]11;rgb:ffff/ffff/ffff\x07\x1b[?62c"]);
    assert_eq!(wait_left(at(10), start, &done, T, GRACE), Duration::ZERO);
}

/// A reply that has not begun by the base timeout is not waited for: a
/// terminal that answers late (or never) costs 200 ms, not the grace.
#[test]
fn a_late_reply_is_not_waited_for() {
    let start = Instant::now();
    let none = scan(&[b"q"]);
    assert_eq!(
        wait_left(start + Duration::from_millis(200), start, &none, T, GRACE),
        Duration::ZERO
    );
}
