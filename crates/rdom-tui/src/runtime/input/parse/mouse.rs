//! Mouse reports: SGR (mode 1006, `CSI < b ; x ; y M|m`), rxvt (1015,
//! `CSI b ; x ; y M`) and X10 (`CSI M b x y`, three raw bytes). rdom's
//! mouse capture asks for all three; the terminal answers in the best
//! it knows. Coordinates are 1-based on the wire, 0-based out; a zero
//! coordinate (malformed) is dropped rather than wrapped. Results match
//! crossterm 0.28's.

use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use super::Step;

fn report(kind: MouseEventKind, modifiers: KeyModifiers, column: u16, row: u16) -> Step {
    Step::event(Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers,
    }))
}

/// A 1-based coordinate parameter, 0-based.
fn coordinate(part: Option<&str>) -> Option<u16> {
    part?.parse::<u16>().ok()?.checked_sub(1)
}

/// `CSI M b x y` — each of `b`, `x`, `y` a byte offset by 32.
pub(super) fn x10(buf: &[u8]) -> Step {
    if buf.len() < 6 {
        return Step::Pending;
    }
    let decoded = (|| {
        let (kind, modifiers) = button(buf[3].checked_sub(32)?)?;
        let column = u16::from(buf[4].checked_sub(32)?).checked_sub(1)?;
        let row = u16::from(buf[5].checked_sub(32)?).checked_sub(1)?;
        Some((kind, modifiers, column, row))
    })();
    match decoded {
        Some((kind, modifiers, column, row)) => report(kind, modifiers, column, row),
        None => Step::Invalid,
    }
}

/// `CSI b ; x ; y M` — `b` offset by 32.
pub(super) fn rxvt(buf: &[u8]) -> Step {
    let decoded = (|| {
        let s = std::str::from_utf8(&buf[2..buf.len() - 1]).ok()?;
        let mut split = s.split(';');
        let cb = split.next()?.parse::<u8>().ok()?.checked_sub(32)?;
        let (kind, modifiers) = button(cb)?;
        Some((
            kind,
            modifiers,
            coordinate(split.next())?,
            coordinate(split.next())?,
        ))
    })();
    match decoded {
        Some((kind, modifiers, column, row)) => report(kind, modifiers, column, row),
        None => Step::Invalid,
    }
}

/// `CSI < b ; x ; y M|m` — `m` releases the button `b` names.
pub(super) fn sgr(buf: &[u8]) -> Step {
    let release = buf[buf.len() - 1] == b'm';
    let decoded = (|| {
        let s = std::str::from_utf8(&buf[3..buf.len() - 1]).ok()?;
        let mut split = s.split(';');
        let (kind, modifiers) = button(split.next()?.parse::<u8>().ok()?)?;
        Some((
            kind,
            modifiers,
            coordinate(split.next())?,
            coordinate(split.next())?,
        ))
    })();
    match decoded {
        Some((kind, modifiers, column, row)) => {
            let kind = match kind {
                MouseEventKind::Down(b) if release => MouseEventKind::Up(b),
                other => other,
            };
            report(kind, modifiers, column, row)
        }
        None => Step::Invalid,
    }
}

/// The button byte: button number in bits 0–1 and 6–7, Shift 4, Alt 8,
/// Ctrl 16, motion 32.
fn button(cb: u8) -> Option<(MouseEventKind, KeyModifiers)> {
    let number = (cb & 0b0000_0011) | ((cb & 0b1100_0000) >> 4);
    let dragging = cb & 0b0010_0000 != 0;
    let kind = match (number, dragging) {
        (0, false) => MouseEventKind::Down(MouseButton::Left),
        (1, false) => MouseEventKind::Down(MouseButton::Middle),
        (2, false) => MouseEventKind::Down(MouseButton::Right),
        (0, true) => MouseEventKind::Drag(MouseButton::Left),
        (1, true) => MouseEventKind::Drag(MouseButton::Middle),
        (2, true) => MouseEventKind::Drag(MouseButton::Right),
        (3, false) => MouseEventKind::Up(MouseButton::Left),
        (3, true) | (4, true) | (5, true) => MouseEventKind::Moved,
        (4, false) => MouseEventKind::ScrollUp,
        (5, false) => MouseEventKind::ScrollDown,
        (6, false) => MouseEventKind::ScrollLeft,
        (7, false) => MouseEventKind::ScrollRight,
        _ => return None,
    };
    let mut modifiers = KeyModifiers::empty();
    if cb & 0b0000_0100 != 0 {
        modifiers |= KeyModifiers::SHIFT;
    }
    if cb & 0b0000_1000 != 0 {
        modifiers |= KeyModifiers::ALT;
    }
    if cb & 0b0001_0000 != 0 {
        modifiers |= KeyModifiers::CONTROL;
    }
    Some((kind, modifiers))
}
