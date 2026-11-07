//! What a session can do to an app: keys, text, paste, the mouse and a
//! resize, as JSON a person or an agent can write by hand.
//!
//! ```json
//! {"type": "key", "key": "ctrl+s"}
//! {"type": "text", "text": "hello"}
//! {"type": "paste", "text": "a\nb"}
//! {"type": "mouse", "action": "click", "x": 12, "y": 4}
//! {"type": "mouse", "action": "wheel-down", "x": 12, "y": 4}
//! {"type": "resize", "width": 80, "height": 24}
//! ```
//!
//! A key is `[modifier+]…name`: modifiers `ctrl`, `alt`, `shift`, `meta`
//! (any case); names `enter`, `tab`, `esc` (`escape`), `backspace`,
//! `delete` (`del`), `insert`, `up`, `down`, `left`, `right`, `home`,
//! `end`, `pageup`, `pagedown`, `space`, `f1`–`f24` (any case), or a
//! single character, taken as written (`a`, `A`, `+`). `shift+tab` is the
//! back-tab terminals send.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use serde::{Deserialize, Serialize};

/// One thing a session does to an app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub enum Input {
    /// One key press.
    Key { key: String },
    /// One key press per character, as a terminal sends typing: a newline
    /// is Enter, a tab is Tab, an uppercase letter carries Shift.
    Text { text: String },
    /// A bracketed paste.
    Paste { text: String },
    /// A pointer action at a cell.
    Mouse {
        action: MouseAction,
        x: u16,
        y: u16,
        #[serde(default)]
        button: Button,
    },
    /// The terminal changes size.
    Resize { width: u16, height: u16 },
    /// One terminal event exactly as a recording holds it
    /// (`rdom_tui::runtime::record`): `key c:a 0 press`,
    /// `mouse scroll-left 3 4 0`, `focus out` — what the other inputs do
    /// not say (a key's release, a mouse modifier). A replay's inputs.
    Recorded { event: String },
}

/// A pointer action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum MouseAction {
    /// Move without a button held.
    Move,
    /// Press the button.
    Down,
    /// Release the button.
    Up,
    /// Move there, press and release.
    Click,
    /// Move there with the button held.
    Drag,
    /// One wheel tick up.
    WheelUp,
    /// One wheel tick down.
    WheelDown,
}

/// A mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Button {
    #[default]
    Left,
    Middle,
    Right,
}

impl Button {
    fn crossterm(self) -> MouseButton {
        match self {
            Button::Left => MouseButton::Left,
            Button::Middle => MouseButton::Middle,
            Button::Right => MouseButton::Right,
        }
    }
}

/// The key `spec` names, as the event a terminal sends for it.
pub fn parse_key(spec: &str) -> Result<KeyEvent, String> {
    if spec.is_empty() {
        return Err("empty key".to_string());
    }
    // The last `+` separates the name, unless the name is `+` itself.
    let (mods, name) = match spec.strip_suffix('+') {
        Some(rest) if rest.is_empty() || rest.ends_with('+') => {
            (rest.strip_suffix('+').unwrap_or(rest), "+")
        }
        _ => spec.rsplit_once('+').unwrap_or(("", spec)),
    };
    let mut modifiers = KeyModifiers::empty();
    for m in mods.split('+').filter(|m| !m.is_empty()) {
        modifiers |= match m.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => KeyModifiers::CONTROL,
            "alt" | "option" => KeyModifiers::ALT,
            "shift" => KeyModifiers::SHIFT,
            "meta" | "super" | "cmd" => KeyModifiers::SUPER,
            other => return Err(format!("unknown modifier {other:?} in key {spec:?}")),
        };
    }
    let code = key_code(name).ok_or_else(|| format!("unknown key {name:?} in {spec:?}"))?;
    let code = match code {
        KeyCode::Tab if modifiers.contains(KeyModifiers::SHIFT) => KeyCode::BackTab,
        c => c,
    };
    Ok(KeyEvent::new(code, modifiers))
}

/// The key a terminal sends for typing `c`: Enter for a newline, Tab for a
/// tab, and a character key — with Shift when it is uppercase, as rdom's
/// input parser (and crossterm's) reads it. Other control characters
/// have no key of their own.
fn char_key(c: char) -> Result<Event, String> {
    let key = match c {
        '\n' | '\r' => KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        '\t' => KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()),
        c if c.is_control() => {
            return Err(format!(
                "text holds the control character {c:?}; send it as a key instead"
            ));
        }
        c if c.is_uppercase() => KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT),
        c => KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()),
    };
    Ok(Event::Key(key))
}

/// The key code `name` names.
fn key_code(name: &str) -> Option<KeyCode> {
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Some(KeyCode::Char(c));
    }
    let lower = name.to_ascii_lowercase();
    Some(match lower.as_str() {
        "enter" | "return" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "esc" | "escape" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "insert" | "ins" => KeyCode::Insert,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "space" => KeyCode::Char(' '),
        f => {
            let n: u8 = f.strip_prefix('f')?.parse().ok()?;
            if !(1..=24).contains(&n) {
                return None;
            }
            KeyCode::F(n)
        }
    })
}

impl Input {
    /// The terminal events this input is, in order: a click is a move, a
    /// press and a release; text is a key per character. A resize is the
    /// event alone — the session resizes the backend first.
    pub fn events(&self) -> Result<Vec<Event>, String> {
        Ok(match self {
            Input::Key { key } => vec![Event::Key(parse_key(key)?)],
            Input::Text { text } => text.chars().map(char_key).collect::<Result<_, _>>()?,
            Input::Paste { text } => vec![Event::Paste(text.clone())],
            Input::Mouse {
                action,
                x,
                y,
                button,
            } => {
                let at = |kind| {
                    Event::Mouse(MouseEvent {
                        kind,
                        column: *x,
                        row: *y,
                        modifiers: KeyModifiers::empty(),
                    })
                };
                let b = button.crossterm();
                match action {
                    MouseAction::Move => vec![at(MouseEventKind::Moved)],
                    MouseAction::Down => vec![at(MouseEventKind::Down(b))],
                    MouseAction::Up => vec![at(MouseEventKind::Up(b))],
                    MouseAction::Click => vec![
                        at(MouseEventKind::Moved),
                        at(MouseEventKind::Down(b)),
                        at(MouseEventKind::Up(b)),
                    ],
                    MouseAction::Drag => vec![at(MouseEventKind::Drag(b))],
                    MouseAction::WheelUp => vec![at(MouseEventKind::ScrollUp)],
                    MouseAction::WheelDown => vec![at(MouseEventKind::ScrollDown)],
                }
            }
            Input::Recorded { event } => vec![
                rdom_tui::runtime::record::parse_event(event)
                    .map_err(|why| format!("recorded event {event:?}: {why}"))?,
            ],
            Input::Resize { width, height } => {
                if *width == 0 || *height == 0 {
                    return Err(format!("resize to {width}×{height}: at least 1×1"));
                }
                vec![Event::Resize(*width, *height)]
            }
        })
    }
}
