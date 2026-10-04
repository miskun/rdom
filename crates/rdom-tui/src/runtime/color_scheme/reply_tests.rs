//! The startup query's reply scanner: OSC 11 (xterm "dynamic colors",
//! the background) and the DA1 reply that ends the exchange.

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
