//! Keys: plain bytes (C0 controls, DEL, UTF-8), SS3 (`ESC O x`), and
//! the CSI key encodings — `CSI 1 ; mods X` cursor / function keys,
//! `CSI n ; mods ~` special keys, and `CSI code ; mods u` (fixterms and
//! the kitty keyboard protocol, whose `:event-type` and alternate-key
//! fields rdom's `REPORT_EVENT_TYPES | DISAMBIGUATE_ESCAPE_CODES` flags
//! produce). Each function reproduces crossterm 0.28's result.

use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MediaKeyCode, ModifierKeyCode,
};

use super::Step;

/// A character key: Shift when it is uppercase (crossterm's
/// `char_code_to_event`).
pub(super) fn char_key(c: char) -> KeyEvent {
    let modifiers = if c.is_uppercase() {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    KeyEvent::new(KeyCode::Char(c), modifiers)
}

/// A sequence that does not start with `ESC`: one key.
pub(super) fn plain(buf: &[u8]) -> Step {
    let ctrl = |c: u8| {
        Step::key(KeyEvent::new(
            KeyCode::Char(c as char),
            KeyModifiers::CONTROL,
        ))
    };
    match buf[0] {
        b'\r' => Step::key(KeyCode::Enter),
        b'\t' => Step::key(KeyCode::Tab),
        0x7f => Step::key(KeyCode::Backspace),
        // Raw mode: `\n` is Ctrl+J (crossterm issue #371).
        c @ 0x01..=0x1a => ctrl(c - 0x1 + b'a'),
        c @ 0x1c..=0x1f => ctrl(c - 0x1c + b'4'),
        0 => ctrl(b' '),
        _ => utf8(buf),
    }
}

/// One UTF-8 character, which may still be arriving.
fn utf8(buf: &[u8]) -> Step {
    if let Ok(s) = std::str::from_utf8(buf) {
        return match s.chars().next() {
            Some(c) => Step::key(char_key(c)),
            None => Step::Invalid,
        };
    }
    let required = match buf[0] {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        0x80..=0xbf | 0xf8..=0xff => return Step::Invalid,
    };
    if buf[1..].iter().any(|b| b & 0b1100_0000 != 0b1000_0000) {
        return Step::Invalid;
    }
    if buf.len() < required {
        Step::Pending
    } else {
        Step::Invalid
    }
}

/// `ESC O x` — SS3 cursor keys and F1–F4.
pub(super) fn ss3(buf: &[u8]) -> Step {
    let Some(&b) = buf.get(2) else {
        return Step::Pending;
    };
    match b {
        b'D' => Step::key(KeyCode::Left),
        b'C' => Step::key(KeyCode::Right),
        b'A' => Step::key(KeyCode::Up),
        b'B' => Step::key(KeyCode::Down),
        b'H' => Step::key(KeyCode::Home),
        b'F' => Step::key(KeyCode::End),
        b'P'..=b'S' => Step::key(KeyCode::F(1 + b - b'P')),
        _ => Step::Invalid,
    }
}

/// The parameters of `ESC [ params final`, as text.
fn params(buf: &[u8]) -> Option<&str> {
    std::str::from_utf8(&buf[2..buf.len() - 1]).ok()
}

/// The modifier mask and event type of a `mods[:type]` parameter.
fn modifier_and_kind<'a>(split: &mut impl Iterator<Item = &'a str>) -> Option<(u8, u8)> {
    let mut sub = split.next()?.split(':');
    let mask = sub.next()?.parse::<u8>().ok()?;
    let kind = sub.next().and_then(|k| k.parse::<u8>().ok()).unwrap_or(1);
    Some((mask, kind))
}

/// xterm's modifier parameter: 1 + Shift 1 | Alt 2 | Ctrl 4 | Super 8 |
/// Hyper 16 | Meta 32 (kitty adds Caps Lock 64, Num Lock 128).
fn modifiers(mask: u8) -> KeyModifiers {
    let m = mask.saturating_sub(1);
    let mut out = KeyModifiers::empty();
    for (bit, modifier) in [
        (1, KeyModifiers::SHIFT),
        (2, KeyModifiers::ALT),
        (4, KeyModifiers::CONTROL),
        (8, KeyModifiers::SUPER),
        (16, KeyModifiers::HYPER),
        (32, KeyModifiers::META),
    ] {
        if m & bit != 0 {
            out |= modifier;
        }
    }
    out
}

/// The lock state in a kitty modifier parameter.
fn lock_state(mask: u8) -> KeyEventState {
    let m = mask.saturating_sub(1);
    let mut state = KeyEventState::empty();
    if m & 64 != 0 {
        state |= KeyEventState::CAPS_LOCK;
    }
    if m & 128 != 0 {
        state |= KeyEventState::NUM_LOCK;
    }
    state
}

/// The kitty protocol's event type: 1 press, 2 repeat, 3 release.
fn kind(code: u8) -> KeyEventKind {
    match code {
        2 => KeyEventKind::Repeat,
        3 => KeyEventKind::Release,
        _ => KeyEventKind::Press,
    }
}

/// `CSI [1] ; mods[:type] X` — a cursor key, Home / End or F1–F4 with
/// modifiers (`X` the final byte).
pub(super) fn modified(buf: &[u8]) -> Step {
    let Some(s) = params(buf) else {
        return Step::Invalid;
    };
    let mut split = s.split(';');
    split.next();
    let (mods, kind) = if let Some((mask, code)) = modifier_and_kind(&mut split) {
        (modifiers(mask), kind(code))
    } else if buf.len() > 3 {
        // `CSI 5 A`: the digit before the final byte is the mask.
        match (buf[buf.len() - 2] as char).to_digit(10) {
            Some(d) => (modifiers(d as u8), KeyEventKind::Press),
            None => return Step::Invalid,
        }
    } else {
        (KeyModifiers::NONE, KeyEventKind::Press)
    };
    let code = match buf[buf.len() - 1] {
        b'A' => KeyCode::Up,
        b'B' => KeyCode::Down,
        b'C' => KeyCode::Right,
        b'D' => KeyCode::Left,
        b'F' => KeyCode::End,
        b'H' => KeyCode::Home,
        b'P' => KeyCode::F(1),
        b'Q' => KeyCode::F(2),
        b'R' => KeyCode::F(3),
        b'S' => KeyCode::F(4),
        _ => return Step::Invalid,
    };
    Step::key(KeyEvent::new_with_kind(code, mods, kind))
}

/// `CSI n [; mods[:type]] ~` — Home, Insert, Delete, End, Page Up /
/// Down, F1–F20.
pub(super) fn special(buf: &[u8]) -> Step {
    let Some(s) = params(buf) else {
        return Step::Invalid;
    };
    let mut split = s.split(';');
    let Some(first) = split.next().and_then(|n| n.parse::<u8>().ok()) else {
        return Step::Invalid;
    };
    let (mods, kind, state) = match modifier_and_kind(&mut split) {
        Some((mask, code)) => (modifiers(mask), kind(code), lock_state(mask)),
        None => (KeyModifiers::NONE, KeyEventKind::Press, KeyEventState::NONE),
    };
    let code = match first {
        1 | 7 => KeyCode::Home,
        2 => KeyCode::Insert,
        3 => KeyCode::Delete,
        4 | 8 => KeyCode::End,
        5 => KeyCode::PageUp,
        6 => KeyCode::PageDown,
        v @ 11..=15 => KeyCode::F(v - 10),
        v @ 17..=21 => KeyCode::F(v - 11),
        v @ 23..=26 => KeyCode::F(v - 12),
        v @ 28..=29 => KeyCode::F(v - 15),
        v @ 31..=34 => KeyCode::F(v - 17),
        _ => return Step::Invalid,
    };
    Step::key(KeyEvent::new_with_kind_and_state(code, mods, kind, state))
}

/// `CSI code[:shifted] [; mods[:type]] u` — fixterms / the kitty
/// keyboard protocol.
pub(super) fn csi_u(buf: &[u8]) -> Step {
    let Some(s) = params(buf) else {
        return Step::Invalid;
    };
    let mut split = s.split(';');
    let mut codepoints = split.next().unwrap_or("").split(':');
    let Some(codepoint) = codepoints.next().and_then(|c| c.parse::<u32>().ok()) else {
        return Step::Invalid;
    };
    let (mut mods, kind, lock) = match modifier_and_kind(&mut split) {
        Some((mask, code)) => (modifiers(mask), kind(code), lock_state(mask)),
        None => (KeyModifiers::NONE, KeyEventKind::Press, KeyEventState::NONE),
    };
    let (mut code, keypad) = if let Some(functional) = functional_key(codepoint) {
        functional
    } else if let Some(c) = char::from_u32(codepoint) {
        let code = match c {
            '\x1b' => KeyCode::Esc,
            '\r' => KeyCode::Enter,
            '\t' if mods.contains(KeyModifiers::SHIFT) => KeyCode::BackTab,
            '\t' => KeyCode::Tab,
            '\x7f' => KeyCode::Backspace,
            _ => KeyCode::Char(c),
        };
        (code, KeyEventState::empty())
    } else {
        return Step::Invalid;
    };
    if let KeyCode::Modifier(m) = code {
        use ModifierKeyCode as M;
        let held = match m {
            M::LeftAlt | M::RightAlt => KeyModifiers::ALT,
            M::LeftControl | M::RightControl => KeyModifiers::CONTROL,
            M::LeftShift | M::RightShift => KeyModifiers::SHIFT,
            M::LeftSuper | M::RightSuper => KeyModifiers::SUPER,
            M::LeftHyper | M::RightHyper => KeyModifiers::HYPER,
            M::LeftMeta | M::RightMeta => KeyModifiers::META,
            _ => KeyModifiers::NONE,
        };
        mods |= held;
    }
    // "Report alternate keys": with Shift held, the shifted character
    // follows the code after a `:`; it replaces the key, and Shift is
    // consumed by it.
    if mods.contains(KeyModifiers::SHIFT)
        && let Some(shifted) = codepoints
            .next()
            .and_then(|c| c.parse::<u32>().ok())
            .and_then(char::from_u32)
    {
        code = KeyCode::Char(shifted);
        mods.remove(KeyModifiers::SHIFT);
    }
    Step::key(KeyEvent::new_with_kind_and_state(
        code,
        mods,
        kind,
        keypad | lock,
    ))
}

/// The kitty protocol's functional-key code points (Private Use Area):
/// keypad keys (with the `KEYPAD` state), lock and media keys, F13–F35,
/// modifier keys.
fn functional_key(codepoint: u32) -> Option<(KeyCode, KeyEventState)> {
    let keypad = match codepoint {
        57399..=57408 => Some(KeyCode::Char(char::from(b'0' + (codepoint - 57399) as u8))),
        57409 => Some(KeyCode::Char('.')),
        57410 => Some(KeyCode::Char('/')),
        57411 => Some(KeyCode::Char('*')),
        57412 => Some(KeyCode::Char('-')),
        57413 => Some(KeyCode::Char('+')),
        57414 => Some(KeyCode::Enter),
        57415 => Some(KeyCode::Char('=')),
        57416 => Some(KeyCode::Char(',')),
        57417 => Some(KeyCode::Left),
        57418 => Some(KeyCode::Right),
        57419 => Some(KeyCode::Up),
        57420 => Some(KeyCode::Down),
        57421 => Some(KeyCode::PageUp),
        57422 => Some(KeyCode::PageDown),
        57423 => Some(KeyCode::Home),
        57424 => Some(KeyCode::End),
        57425 => Some(KeyCode::Insert),
        57426 => Some(KeyCode::Delete),
        57427 => Some(KeyCode::KeypadBegin),
        _ => None,
    };
    if let Some(code) = keypad {
        return Some((code, KeyEventState::KEYPAD));
    }
    use MediaKeyCode as Media;
    use ModifierKeyCode as Mod;
    let code = match codepoint {
        57358 => KeyCode::CapsLock,
        57359 => KeyCode::ScrollLock,
        57360 => KeyCode::NumLock,
        57361 => KeyCode::PrintScreen,
        57362 => KeyCode::Pause,
        57363 => KeyCode::Menu,
        57376..=57398 => KeyCode::F(13 + (codepoint - 57376) as u8),
        57428 => KeyCode::Media(Media::Play),
        57429 => KeyCode::Media(Media::Pause),
        57430 => KeyCode::Media(Media::PlayPause),
        57431 => KeyCode::Media(Media::Reverse),
        57432 => KeyCode::Media(Media::Stop),
        57433 => KeyCode::Media(Media::FastForward),
        57434 => KeyCode::Media(Media::Rewind),
        57435 => KeyCode::Media(Media::TrackNext),
        57436 => KeyCode::Media(Media::TrackPrevious),
        57437 => KeyCode::Media(Media::Record),
        57438 => KeyCode::Media(Media::LowerVolume),
        57439 => KeyCode::Media(Media::RaiseVolume),
        57440 => KeyCode::Media(Media::MuteVolume),
        57441 => KeyCode::Modifier(Mod::LeftShift),
        57442 => KeyCode::Modifier(Mod::LeftControl),
        57443 => KeyCode::Modifier(Mod::LeftAlt),
        57444 => KeyCode::Modifier(Mod::LeftSuper),
        57445 => KeyCode::Modifier(Mod::LeftHyper),
        57446 => KeyCode::Modifier(Mod::LeftMeta),
        57447 => KeyCode::Modifier(Mod::RightShift),
        57448 => KeyCode::Modifier(Mod::RightControl),
        57449 => KeyCode::Modifier(Mod::RightAlt),
        57450 => KeyCode::Modifier(Mod::RightSuper),
        57451 => KeyCode::Modifier(Mod::RightHyper),
        57452 => KeyCode::Modifier(Mod::RightMeta),
        57453 => KeyCode::Modifier(Mod::IsoLevel3Shift),
        57454 => KeyCode::Modifier(Mod::IsoLevel5Shift),
        _ => return None,
    };
    Some((code, KeyEventState::empty()))
}
