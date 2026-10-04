//! The parser's byte-stream corpus: each sequence class against the
//! `Event`s crossterm 0.28 produces for it (its own unit tests are the
//! source for most cases), the replies and reports rdom reads that
//! crossterm does not, and the stream properties — split reads, the
//! escape prefixes, recovery after an unknown or broken sequence.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MediaKeyCode,
    ModifierKeyCode, MouseButton, MouseEvent, MouseEventKind,
};
use rdom_style::color::ColorScheme;

use super::*;
use crate::style::Color;

/// Everything `chunks` parse to, fed one read at a time.
fn inputs(chunks: &[&[u8]]) -> Vec<Input> {
    let mut p = Parser::default();
    for c in chunks {
        p.feed(c);
    }
    std::iter::from_fn(|| p.next()).collect()
}

/// The events `bytes` parse to (fed byte by byte, the worst split).
fn events(bytes: &[u8]) -> Vec<Event> {
    let chunks: Vec<&[u8]> = bytes.chunks(1).collect();
    let whole = inputs(&[bytes]);
    assert_eq!(whole, inputs(&chunks), "split reads parse alike: {bytes:?}");
    whole
        .into_iter()
        .map(|i| match i {
            Input::Event(e) => e,
            other => panic!("not an event: {other:?}"),
        })
        .collect()
}

/// The one event `bytes` parse to.
fn event(bytes: &[u8]) -> Event {
    let mut all = events(bytes);
    assert_eq!(all.len(), 1, "{bytes:?} → {all:?}");
    all.remove(0)
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, modifiers))
}

fn kind(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> Event {
    Event::Key(KeyEvent::new_with_kind(code, modifiers, kind))
}

fn state(code: KeyCode, modifiers: KeyModifiers, state: KeyEventState) -> Event {
    Event::Key(KeyEvent::new_with_kind_and_state(
        code,
        modifiers,
        KeyEventKind::Press,
        state,
    ))
}

fn mouse(kind: MouseEventKind, column: u16, row: u16, modifiers: KeyModifiers) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers,
    })
}

const NONE: KeyModifiers = KeyModifiers::NONE;
const SHIFT: KeyModifiers = KeyModifiers::SHIFT;
const ALT: KeyModifiers = KeyModifiers::ALT;
const CTRL: KeyModifiers = KeyModifiers::CONTROL;

// ── Plain keys ───────────────────────────────────────────────────────

#[test]
fn plain_keys() {
    assert_eq!(event(b"c"), key(KeyCode::Char('c'), NONE));
    assert_eq!(event(b"C"), key(KeyCode::Char('C'), SHIFT));
    assert_eq!(event(b"\t"), key(KeyCode::Tab, NONE));
    assert_eq!(event(b"\r"), key(KeyCode::Enter, NONE));
    assert_eq!(event(b"\x7f"), key(KeyCode::Backspace, NONE));
    // Raw mode: `\n` is Ctrl+J.
    assert_eq!(event(b"\n"), key(KeyCode::Char('j'), CTRL));
    assert_eq!(event(b"\x03"), key(KeyCode::Char('c'), CTRL));
    assert_eq!(event(b"\x1c"), key(KeyCode::Char('4'), CTRL));
    assert_eq!(event(b"\x00"), key(KeyCode::Char(' '), CTRL));
}

#[test]
fn utf8_characters() {
    assert_eq!(event("ñ".as_bytes()), key(KeyCode::Char('ñ'), NONE));
    assert_eq!(event("Ž".as_bytes()), key(KeyCode::Char('Ž'), SHIFT));
    assert_eq!(
        event("\u{2061}".as_bytes()),
        key(KeyCode::Char('\u{2061}'), NONE)
    );
    assert_eq!(event("𐌼".as_bytes()), key(KeyCode::Char('𐌼'), NONE));
    // Invalid sequences are dropped; what follows still parses.
    assert_eq!(events(&[0xc3, 0x28]), vec![]);
    assert_eq!(events(&[0xa0, b'a']), vec![key(KeyCode::Char('a'), NONE)]);
    assert_eq!(events(&[0xe2, 0x28, 0xa1]), vec![]);
    assert_eq!(events(&[0xf0, 0x90, 0x28, 0xbc]), vec![]);
}

#[test]
fn alt_keys() {
    assert_eq!(event(b"\x1bc"), key(KeyCode::Char('c'), ALT));
    assert_eq!(event(b"\x1bH"), key(KeyCode::Char('H'), ALT | SHIFT));
    assert_eq!(event(b"\x1b\x14"), key(KeyCode::Char('t'), ALT | CTRL));
    assert_eq!(event("\x1bñ".as_bytes()), key(KeyCode::Char('ñ'), ALT));
    // `ESC ESC` is two Escs, not crossterm's one: see
    // `escape_escape_is_two_escapes_or_legacy_alt`.
}

#[test]
fn ss3_keys() {
    assert_eq!(event(b"\x1bOA"), key(KeyCode::Up, NONE));
    assert_eq!(event(b"\x1bOD"), key(KeyCode::Left, NONE));
    assert_eq!(event(b"\x1bOH"), key(KeyCode::Home, NONE));
    assert_eq!(event(b"\x1bOF"), key(KeyCode::End, NONE));
    assert_eq!(event(b"\x1bOP"), key(KeyCode::F(1), NONE));
    assert_eq!(event(b"\x1bOS"), key(KeyCode::F(4), NONE));
}

// ── CSI keys ─────────────────────────────────────────────────────────

#[test]
fn csi_cursor_and_function_keys() {
    assert_eq!(event(b"\x1b[A"), key(KeyCode::Up, NONE));
    assert_eq!(event(b"\x1b[B"), key(KeyCode::Down, NONE));
    assert_eq!(event(b"\x1b[C"), key(KeyCode::Right, NONE));
    assert_eq!(event(b"\x1b[D"), key(KeyCode::Left, NONE));
    assert_eq!(event(b"\x1b[H"), key(KeyCode::Home, NONE));
    assert_eq!(event(b"\x1b[F"), key(KeyCode::End, NONE));
    assert_eq!(event(b"\x1b[Z"), key(KeyCode::BackTab, SHIFT));
    assert_eq!(event(b"\x1b[P"), key(KeyCode::F(1), NONE));
    assert_eq!(event(b"\x1b[Q"), key(KeyCode::F(2), NONE));
    assert_eq!(event(b"\x1b[S"), key(KeyCode::F(4), NONE));
    assert_eq!(event(b"\x1b[[C"), key(KeyCode::F(3), NONE));
}

#[test]
fn csi_keys_with_modifiers() {
    assert_eq!(event(b"\x1b[2D"), key(KeyCode::Left, SHIFT));
    assert_eq!(event(b"\x1b[1;5A"), key(KeyCode::Up, CTRL));
    assert_eq!(event(b"\x1b[1;3C"), key(KeyCode::Right, ALT));
    assert_eq!(event(b"\x1b[1;2P"), key(KeyCode::F(1), SHIFT));
    // Kitty event types.
    assert_eq!(
        event(b"\x1b[1;1:3B"),
        kind(KeyCode::Down, NONE, KeyEventKind::Release)
    );
    assert_eq!(
        event(b"\x1b[;1:3B"),
        kind(KeyCode::Down, NONE, KeyEventKind::Release)
    );
}

#[test]
fn csi_special_keys() {
    assert_eq!(event(b"\x1b[3~"), key(KeyCode::Delete, NONE));
    assert_eq!(event(b"\x1b[3;2~"), key(KeyCode::Delete, SHIFT));
    assert_eq!(event(b"\x1b[2~"), key(KeyCode::Insert, NONE));
    assert_eq!(event(b"\x1b[1~"), key(KeyCode::Home, NONE));
    assert_eq!(event(b"\x1b[4~"), key(KeyCode::End, NONE));
    assert_eq!(event(b"\x1b[5~"), key(KeyCode::PageUp, NONE));
    assert_eq!(event(b"\x1b[6~"), key(KeyCode::PageDown, NONE));
    assert_eq!(event(b"\x1b[15~"), key(KeyCode::F(5), NONE));
    assert_eq!(event(b"\x1b[24~"), key(KeyCode::F(12), NONE));
    assert_eq!(
        event(b"\x1b[5;1:3~"),
        kind(KeyCode::PageUp, NONE, KeyEventKind::Release)
    );
    assert_eq!(
        event(b"\x1b[6;5:3~"),
        kind(KeyCode::PageDown, CTRL, KeyEventKind::Release)
    );
}

#[test]
fn csi_u_keys() {
    assert_eq!(event(b"\x1b[97u"), key(KeyCode::Char('a'), NONE));
    assert_eq!(event(b"\x1b[97;2u"), key(KeyCode::Char('a'), SHIFT));
    assert_eq!(event(b"\x1b[97;7u"), key(KeyCode::Char('a'), ALT | CTRL));
    assert_eq!(event(b"\x1b[13u"), key(KeyCode::Enter, NONE));
    assert_eq!(event(b"\x1b[27u"), key(KeyCode::Esc, NONE));
    assert_eq!(event(b"\x1b[9;2u"), key(KeyCode::BackTab, SHIFT));
    assert_eq!(event(b"\x1b[127u"), key(KeyCode::Backspace, NONE));
    assert_eq!(event(b"\x1b[57358u"), key(KeyCode::CapsLock, NONE));
    assert_eq!(event(b"\x1b[57376u"), key(KeyCode::F(13), NONE));
    assert_eq!(
        event(b"\x1b[57428u"),
        key(KeyCode::Media(MediaKeyCode::Play), NONE)
    );
    assert_eq!(
        event(b"\x1b[57441u"),
        key(KeyCode::Modifier(ModifierKeyCode::LeftShift), SHIFT)
    );
    assert_eq!(
        event(b"\x1b[57449;3:3u"),
        kind(
            KeyCode::Modifier(ModifierKeyCode::RightAlt),
            ALT,
            KeyEventKind::Release
        )
    );
    assert_eq!(
        event(b"\x1b[57399u"),
        state(KeyCode::Char('0'), NONE, KeyEventState::KEYPAD)
    );
    assert_eq!(
        event(b"\x1b[57419u"),
        state(KeyCode::Up, NONE, KeyEventState::KEYPAD)
    );
    assert_eq!(
        event(b"\x1b[97;1:2u"),
        kind(KeyCode::Char('a'), NONE, KeyEventKind::Repeat)
    );
    assert_eq!(
        event(b"\x1b[97;5:1u"),
        kind(KeyCode::Char('a'), CTRL, KeyEventKind::Press)
    );
    assert_eq!(
        event(b"\x1b[97;9u"),
        key(KeyCode::Char('a'), KeyModifiers::SUPER)
    );
    assert_eq!(
        event(b"\x1b[97;17u"),
        key(KeyCode::Char('a'), KeyModifiers::HYPER)
    );
    assert_eq!(
        event(b"\x1b[97;33u"),
        key(KeyCode::Char('a'), KeyModifiers::META)
    );
    assert_eq!(
        event(b"\x1b[97;65u"),
        state(KeyCode::Char('a'), NONE, KeyEventState::CAPS_LOCK)
    );
    assert_eq!(
        event(b"\x1b[49;129u"),
        state(KeyCode::Char('1'), NONE, KeyEventState::NUM_LOCK)
    );
    // Alternate keys: A-S-9 is A-(.
    assert_eq!(event(b"\x1b[57:40;4u"), key(KeyCode::Char('('), ALT));
    assert_eq!(event(b"\x1b[45:95;4u"), key(KeyCode::Char('_'), ALT));
}

// ── Mouse ────────────────────────────────────────────────────────────

#[test]
fn sgr_mouse() {
    use MouseButton::*;
    use MouseEventKind::*;
    assert_eq!(event(b"\x1b[<0;20;10M"), mouse(Down(Left), 19, 9, NONE));
    assert_eq!(event(b"\x1b[<0;20;10;M"), mouse(Down(Left), 19, 9, NONE));
    assert_eq!(event(b"\x1b[<0;20;10m"), mouse(Up(Left), 19, 9, NONE));
    assert_eq!(event(b"\x1b[<1;1;1M"), mouse(Down(Middle), 0, 0, NONE));
    assert_eq!(event(b"\x1b[<2;1;1m"), mouse(Up(Right), 0, 0, NONE));
    assert_eq!(event(b"\x1b[<32;5;6M"), mouse(Drag(Left), 4, 5, NONE));
    assert_eq!(event(b"\x1b[<34;5;6M"), mouse(Drag(Right), 4, 5, NONE));
    assert_eq!(event(b"\x1b[<35;5;6M"), mouse(Moved, 4, 5, NONE));
    assert_eq!(event(b"\x1b[<64;3;4M"), mouse(ScrollUp, 2, 3, NONE));
    assert_eq!(event(b"\x1b[<65;3;4M"), mouse(ScrollDown, 2, 3, NONE));
    assert_eq!(event(b"\x1b[<66;3;4M"), mouse(ScrollLeft, 2, 3, NONE));
    assert_eq!(event(b"\x1b[<67;3;4M"), mouse(ScrollRight, 2, 3, NONE));
    assert_eq!(event(b"\x1b[<4;1;1M"), mouse(Down(Left), 0, 0, SHIFT));
    assert_eq!(event(b"\x1b[<8;1;1M"), mouse(Down(Left), 0, 0, ALT));
    assert_eq!(event(b"\x1b[<16;1;1M"), mouse(Down(Left), 0, 0, CTRL));
    assert_eq!(event(b"\x1b[<80;1;1M"), mouse(ScrollUp, 0, 0, CTRL));
    // A zero coordinate is malformed: dropped, not wrapped.
    assert_eq!(events(b"\x1b[<0;0;1M"), vec![]);
}

#[test]
fn rxvt_and_x10_mouse() {
    assert_eq!(
        event(b"\x1b[32;30;40;M"),
        mouse(MouseEventKind::Down(MouseButton::Left), 29, 39, NONE)
    );
    assert_eq!(
        event(b"\x1b[M0\x60\x70"),
        mouse(MouseEventKind::Down(MouseButton::Left), 63, 79, CTRL)
    );
    assert_eq!(events(b"\x1b[M0\x20\x70"), vec![]);
}

// ── Paste, focus ─────────────────────────────────────────────────────

#[test]
fn bracketed_paste() {
    assert_eq!(
        event(b"\x1b[200~on and on\x1b[201~"),
        Event::Paste("on and on".into())
    );
    // Escapes and controls inside a paste are text.
    assert_eq!(
        event(b"\x1b[200~o\x1b[2D\r\x1b[201~"),
        Event::Paste("o\x1b[2D\r".into())
    );
    assert_eq!(event(b"\x1b[200~\x1b[201~"), Event::Paste(String::new()));
    assert_eq!(
        inputs(&[b"\x1b[200~o\x1b[2D"]),
        vec![],
        "unfinished paste waits"
    );
}

#[test]
fn focus_events() {
    assert_eq!(event(b"\x1b[I"), Event::FocusGained);
    assert_eq!(event(b"\x1b[O"), Event::FocusLost);
}

// ── Replies and reports ──────────────────────────────────────────────

/// OSC 11 (xterm "dynamic colors"): the background, ended by BEL or ST,
/// components of 1–4 hex digits scaled to a byte.
#[test]
fn osc11_background_replies() {
    let bg = |reply: &[u8]| inputs(&[reply]);
    let one = |c| vec![Input::Background(c)];
    assert_eq!(
        bg(b"\x1b]11;rgb:0000/2b2b/3636\x07"),
        one(Color::Rgb(0, 0x2b, 0x36))
    );
    assert_eq!(
        bg(b"\x1b]11;rgb:fd/f6/e3\x1b\\"),
        one(Color::Rgb(0xfd, 0xf6, 0xe3))
    );
    assert_eq!(bg(b"\x1b]11;rgb:f/0/f\x07"), one(Color::Rgb(255, 0, 255)));
    assert_eq!(
        bg(b"\x1b]11;rgb:fff/000/800\x07"),
        one(Color::Rgb(255, 0, 128))
    );
    assert_eq!(
        bg(b"\x1b]11;rgba:ffff/ffff/ffff/ffff\x07"),
        one(Color::Rgb(255, 255, 255))
    );
    assert_eq!(
        bg(b"\x1b]11;#102030\x07"),
        one(Color::Rgb(0x10, 0x20, 0x30))
    );
    // Unparseable colors and other OSC strings are consumed, not typed.
    assert_eq!(bg(b"\x1b]11;rgb:zz/00/00\x07"), vec![]);
    assert_eq!(bg(b"\x1b]11;rgb:00/00\x07"), vec![]);
    assert_eq!(bg(b"\x1b]10;rgb:ffff/ffff/ffff\x07"), vec![]);
}

/// A reply split across reads, with a keystroke before it and DA1 after.
#[test]
fn replies_in_fragments() {
    assert_eq!(
        inputs(&[
            b"x\x1b]1",
            b"1;rgb:ff",
            b"ff/0000/0000\x1b",
            b"\\\x1b[?6",
            b"5;1c",
        ]),
        vec![
            Input::Event(key(KeyCode::Char('x'), NONE)),
            Input::Background(Color::Rgb(255, 0, 0)),
            Input::DeviceAttributes,
        ]
    );
}

/// A late reply — after the startup query stopped waiting — is consumed:
/// DA1 and OSC 11 do not become keystrokes, and the keys around them
/// still arrive.
#[test]
fn late_replies_are_not_keystrokes() {
    assert_eq!(
        inputs(&[b"a\x1b]11;rgb:ffff/ffff/ffff\x1b\\b\x1b[?62;22cc"]),
        vec![
            Input::Event(key(KeyCode::Char('a'), NONE)),
            Input::Background(Color::Rgb(255, 255, 255)),
            Input::Event(key(KeyCode::Char('b'), NONE)),
            Input::DeviceAttributes,
            Input::Event(key(KeyCode::Char('c'), NONE)),
        ]
    );
}

/// DEC mode 2031: `CSI ? 997 ; 1 n` dark, `; 2 n` light — and the keys
/// after it are not held (crossterm 0.28 holds them all).
#[test]
fn mode_2031_reports() {
    assert_eq!(
        inputs(&[b"\x1b[?997;2nq"]),
        vec![
            Input::ColorScheme(ColorScheme::Light),
            Input::Event(key(KeyCode::Char('q'), NONE)),
        ]
    );
    assert_eq!(
        inputs(&[b"\x1b[?997;", b"1n"]),
        vec![Input::ColorScheme(ColorScheme::Dark)]
    );
}

/// Other replies (the kitty flags reply, a cursor position report) are
/// consumed, as crossterm keeps them out of its event stream.
#[test]
fn other_replies_are_consumed() {
    assert_eq!(inputs(&[b"\x1b[?1u\x1b[20;10R"]), vec![]);
}

// ── Stream properties ────────────────────────────────────────────────

/// Every sequence parses the same when a read ends anywhere inside it
/// (`events` checks byte-by-byte feeding against one read); here, a
/// split into two reads mid-sequence.
#[test]
fn split_reads() {
    assert_eq!(
        inputs(&[b"\x1b[1;", b"5A"]),
        vec![Input::Event(key(KeyCode::Up, CTRL))]
    );
    assert_eq!(
        inputs(&[b"\x1b[<0;2", b"0;10M"]),
        vec![Input::Event(mouse(
            MouseEventKind::Down(MouseButton::Left),
            19,
            9,
            NONE
        ))]
    );
    assert_eq!(
        inputs(&[&"é".as_bytes()[..1], &"é".as_bytes()[1..]]),
        vec![Input::Event(key(KeyCode::Char('é'), NONE))]
    );
}

/// A lone `ESC` waits — it may begin a sequence — until more bytes come
/// or the reader flushes it as Esc; likewise `ESC [`, `ESC O`, `ESC ]`
/// are Alt+`[`, Alt+`O`, Alt+`]` when nothing follows.
#[test]
fn escape_prefixes_wait_then_flush() {
    let mut p = Parser::default();
    p.feed(b"\x1b");
    assert!(p.awaits_prefix());
    assert_eq!(p.next(), None);
    p.flush_prefix();
    assert_eq!(p.next(), Some(Input::Event(key(KeyCode::Esc, NONE))));
    assert!(!p.awaits_prefix());

    for (b, k) in [
        (b'[', key(KeyCode::Char('['), ALT)),
        (b'O', key(KeyCode::Char('O'), ALT | SHIFT)),
        (b']', key(KeyCode::Char(']'), ALT)),
    ] {
        p.feed(&[0x1b, b]);
        assert!(p.awaits_prefix());
        p.flush_prefix();
        assert_eq!(p.next(), Some(Input::Event(k)));
    }

    // An ESC followed by a key in the next read is Alt + the key.
    p.feed(b"\x1b");
    p.feed(b"x");
    assert_eq!(p.next(), Some(Input::Event(key(KeyCode::Char('x'), ALT))));
    // `ESC ]` and a non-digit: Alt+`]`, then the key.
    p.feed(b"\x1b]x");
    assert_eq!(p.next(), Some(Input::Event(key(KeyCode::Char(']'), ALT))));
    assert_eq!(p.next(), Some(Input::Event(key(KeyCode::Char('x'), NONE))));
    // A sequence in progress is not a prefix: nothing to flush.
    p.feed(b"\x1b[1");
    assert!(!p.awaits_prefix());
    assert!(p.in_sequence());
    p.flush_prefix();
    assert_eq!(p.next(), None);
}

/// `ESC ESC` is Esc and the second `ESC` is read again (crossterm reads
/// one Esc for both): two Esc presses are two Escs, and `ESC ESC x` is
/// Esc, Alt+x. `ESC` before a CSI or SS3 key is the legacy Alt encoding
/// of that key (rxvt, Terminal.app's Option-as-Meta send `ESC ESC [ A`
/// for Alt+Up): Alt + the key (`C4G-ESC-ESC`). `ESC` before any other
/// sequence is Esc, then the sequence.
#[test]
fn escape_escape_is_two_escapes_or_legacy_alt() {
    let esc = || key(KeyCode::Esc, NONE);
    assert_eq!(
        events(b"\x1b\x1bx"),
        vec![esc(), key(KeyCode::Char('x'), ALT)]
    );
    assert_eq!(events(b"\x1b\x1b[A"), vec![key(KeyCode::Up, ALT)]);
    assert_eq!(events(b"\x1b\x1b[1;5A"), vec![key(KeyCode::Up, ALT | CTRL)]);
    assert_eq!(events(b"\x1b\x1bOP"), vec![key(KeyCode::F(1), ALT)]);
    assert_eq!(events(b"\x1b\x1b[3~"), vec![key(KeyCode::Delete, ALT)]);
    assert_eq!(
        events(b"\x1b\x1b[<0;20;10M"),
        vec![
            esc(),
            mouse(MouseEventKind::Down(MouseButton::Left), 19, 9, NONE)
        ]
    );
    // An unknown sequence after it: Esc, the sequence consumed.
    assert_eq!(
        events(b"\x1b\x1b[42Xq"),
        vec![esc(), key(KeyCode::Char('q'), NONE)]
    );

    // Nothing after: each ESC is an Esc once the reader flushes.
    let mut p = Parser::default();
    p.feed(b"\x1b\x1b");
    assert!(p.awaits_prefix());
    assert_eq!(p.next(), None);
    p.flush_prefix();
    let all: Vec<Input> = std::iter::from_fn(|| p.next()).collect();
    assert_eq!(all, vec![Input::Event(esc()), Input::Event(esc())]);
    p.feed(b"\x1b\x1b\x1b");
    p.flush_prefix();
    assert_eq!(std::iter::from_fn(|| p.next()).count(), 3);
    p.feed(b"\x1b\x1b[");
    assert!(p.awaits_prefix());
    p.flush_prefix();
    let all: Vec<Input> = std::iter::from_fn(|| p.next()).collect();
    assert_eq!(
        all,
        vec![
            Input::Event(esc()),
            Input::Event(key(KeyCode::Char('['), ALT))
        ]
    );
}

/// An unknown control sequence is consumed whole and does not stall the
/// input after it; a control byte inside a sequence ends it and is read
/// as a key.
#[test]
fn unknown_or_broken_sequences_do_not_stall() {
    assert_eq!(
        events(b"\x1b[?2026$yq\x1b[42Xw\x1b[<5zv"),
        vec![
            key(KeyCode::Char('q'), NONE),
            key(KeyCode::Char('w'), NONE),
            key(KeyCode::Char('v'), NONE),
        ]
    );
    assert_eq!(events(b"\x1b[12\r"), vec![key(KeyCode::Enter, NONE)]);
    // An ESC inside a sequence starts the next one.
    assert_eq!(events(b"\x1b[1\x1b[A"), vec![key(KeyCode::Up, NONE)]);
    // An ESC inside an OSC string that does not start ST ends it.
    assert_eq!(
        events(b"\x1b]11;rgb\x1bq"),
        vec![key(KeyCode::Char('q'), ALT)]
    );
    // CAN cancels an OSC string.
    assert_eq!(
        events(b"\x1b]11;rgb\x18z"),
        vec![key(KeyCode::Char('z'), NONE)]
    );
}

/// An OSC string past the 4 KiB cap is discarded to its end — BEL or ST
/// — not read again as keys (`C4G-OSC-DISCARD`); a byte outside
/// ECMA-48's command-string range (§5.6: 0x08–0x0D, 0x20–0x7E) aborts
/// it, discarding or not, and is read again.
#[test]
fn an_overlong_osc_string_is_discarded_to_its_end() {
    let long = |end: &[u8]| {
        let mut b = b"\x1b]11;".to_vec();
        b.extend(std::iter::repeat_n(b'x', 5000));
        b.extend_from_slice(end);
        b.push(b'q');
        b
    };
    let q = vec![key(KeyCode::Char('q'), NONE)];
    assert_eq!(events(&long(b"\x07")), q);
    assert_eq!(events(&long(b"\x1b\\")), q);
    assert_eq!(events(&long(b"\x18")), q);
    // An ESC that does not start ST ends the string and is read again.
    assert_eq!(
        events(&long(b"\x1bz")),
        vec![key(KeyCode::Char('z'), ALT), key(KeyCode::Char('q'), NONE)]
    );
    // DEL aborts the discard and is a key (Backspace).
    assert_eq!(
        events(&long(b"\x7f")),
        vec![key(KeyCode::Backspace, NONE), key(KeyCode::Char('q'), NONE)]
    );
}

/// Alt+`]` and a digit typed fast start an OSC string: what is typed
/// after — CR included, which is in the string range — is part of it,
/// until a byte outside the range (Backspace, Ctrl+C) or an `ESC` (an
/// arrow key) ends it; that byte is read again (ECMA-48 §5.6).
#[test]
fn typed_text_after_alt_bracket_and_a_digit_ends_at_a_control() {
    assert_eq!(
        events(b"\x1b]1hi\r\x7fa"),
        vec![key(KeyCode::Backspace, NONE), key(KeyCode::Char('a'), NONE)]
    );
    assert_eq!(
        events(b"\x1b]1hi\r\x03"),
        vec![key(KeyCode::Char('c'), CTRL)]
    );
    assert_eq!(events(b"\x1b]1hi\r\x1b[A"), vec![key(KeyCode::Up, NONE)]);
}

/// The reader's other inputs and the startup query's held keys keep
/// their order.
#[test]
fn delivered_and_unread_inputs_keep_order() {
    let mut p = Parser::default();
    p.feed(b"c");
    p.deliver(Input::Event(Event::Resize(80, 24)));
    p.unread(vec![
        Input::Event(key(KeyCode::Char('a'), NONE)),
        Input::Event(key(KeyCode::Char('b'), NONE)),
    ]);
    let all: Vec<Input> = std::iter::from_fn(|| p.next()).collect();
    assert_eq!(
        all,
        vec![
            Input::Event(key(KeyCode::Char('a'), NONE)),
            Input::Event(key(KeyCode::Char('b'), NONE)),
            Input::Event(key(KeyCode::Char('c'), NONE)),
            Input::Event(Event::Resize(80, 24)),
        ]
    );
}
