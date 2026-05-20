//! Crossterm-input → `rdom_core` event-detail translation.
//!
//! Single canonical seam between the terminal input layer and
//! the substrate's typed event detail. M4a step 8 will replace
//! every `TuiEvent.key` / `TuiEvent.mouse` sibling access with
//! reads through `event.detail.as_keyboard()` / `.as_mouse()`;
//! the helpers here are what those reads ultimately resolve to.
//!
//! ## Web fidelity
//!
//! `key_code_to_string` returns DOM `KeyboardEvent.key` values
//! per UI Events §named-key-attribute-values
//! (<https://www.w3.org/TR/uievents-key/#named-key-attribute-values>).
//! Crossterm taxonomy is wider in some places (left/right modifier
//! variants, BackTab) and narrower in others (no `.code` /
//! physical-key info, no AltGr alongside modifier flags).
//! Documented divergences:
//!
//! - `KeyCode::BackTab` collapses to `"Tab"`. DOM `key` is `"Tab"`
//!   regardless of Shift; browsers signal Shift+Tab via the
//!   `shiftKey` modifier. Apps that need to distinguish Tab from
//!   Shift+Tab read `modifiers.shift`.
//! - `Left*` / `Right*` modifier variants collapse to their
//!   unsided web names (`"Shift"`, `"Control"`, …). The left/right
//!   distinction belongs on `KeyboardEvent.code`, which rdom doesn't
//!   ship (terminals don't reliably expose physical-key info).
//! - `MediaKeyCode::Reverse` and `MediaKeyCode::Rewind` both map to
//!   `"MediaRewind"`. UI Events doesn't define a reverse-playback
//!   key; "Rewind" is the closest semantic neighbor. Information
//!   loss is bounded (TUI apps very rarely distinguish these).
//! - `IsoLevel3Shift` / `IsoLevel5Shift` collapse to `"AltGraph"`.
//! - Modifier bitflags `SUPER` and `META` both contribute to
//!   `KeyboardModifiers.meta`. macOS terminals usually report
//!   Command as `SUPER`; we keep that pulled together with `META`
//!   so apps don't have to special-case the platform.

use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers as CtKeyModifiers, MediaKeyCode, ModifierKeyCode,
    MouseButton as CtMouseButton, MouseEvent, MouseEventKind,
};

use rdom_core::{KeyboardDetail, KeyboardModifiers, MouseButton, MouseDetail};

/// Translate a crossterm `KeyEvent` into a substrate
/// [`KeyboardDetail`]. The `key` field is the DOM-faithful
/// `KeyboardEvent.key` value; modifiers flatten crossterm's
/// bitflags into the four-bool DOM shape; `repeat` is set for
/// `KeyEventKind::Repeat` only.
pub fn translate_key_event(ev: KeyEvent) -> KeyboardDetail {
    KeyboardDetail {
        key: key_code_to_string(ev.code),
        modifiers: translate_modifiers(ev.modifiers),
        repeat: matches!(ev.kind, KeyEventKind::Repeat),
    }
}

/// Map a crossterm [`KeyCode`] to its DOM `KeyboardEvent.key`
/// string. **Exhaustive** — adding a variant to crossterm fails
/// the build until a mapping is chosen here.
pub fn key_code_to_string(code: KeyCode) -> String {
    match code {
        KeyCode::Backspace => "Backspace".into(),
        KeyCode::Enter => "Enter".into(),
        KeyCode::Left => "ArrowLeft".into(),
        KeyCode::Right => "ArrowRight".into(),
        KeyCode::Up => "ArrowUp".into(),
        KeyCode::Down => "ArrowDown".into(),
        KeyCode::Home => "Home".into(),
        KeyCode::End => "End".into(),
        KeyCode::PageUp => "PageUp".into(),
        KeyCode::PageDown => "PageDown".into(),
        KeyCode::Tab => "Tab".into(),
        KeyCode::BackTab => "Tab".into(),
        KeyCode::Delete => "Delete".into(),
        KeyCode::Insert => "Insert".into(),
        KeyCode::F(n) => format!("F{n}"),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Null => "Unidentified".into(),
        KeyCode::Esc => "Escape".into(),
        KeyCode::CapsLock => "CapsLock".into(),
        KeyCode::ScrollLock => "ScrollLock".into(),
        KeyCode::NumLock => "NumLock".into(),
        KeyCode::PrintScreen => "PrintScreen".into(),
        KeyCode::Pause => "Pause".into(),
        KeyCode::Menu => "ContextMenu".into(),
        KeyCode::KeypadBegin => "Clear".into(),
        KeyCode::Media(m) => media_key_to_string(m).into(),
        KeyCode::Modifier(m) => modifier_key_to_string(m).into(),
    }
}

fn media_key_to_string(m: MediaKeyCode) -> &'static str {
    match m {
        MediaKeyCode::Play => "MediaPlay",
        MediaKeyCode::Pause => "MediaPause",
        MediaKeyCode::PlayPause => "MediaPlayPause",
        MediaKeyCode::Reverse => "MediaRewind",
        MediaKeyCode::Stop => "MediaStop",
        MediaKeyCode::FastForward => "MediaFastForward",
        MediaKeyCode::Rewind => "MediaRewind",
        MediaKeyCode::TrackNext => "MediaTrackNext",
        MediaKeyCode::TrackPrevious => "MediaTrackPrevious",
        MediaKeyCode::Record => "MediaRecord",
        MediaKeyCode::LowerVolume => "AudioVolumeDown",
        MediaKeyCode::RaiseVolume => "AudioVolumeUp",
        MediaKeyCode::MuteVolume => "AudioVolumeMute",
    }
}

fn modifier_key_to_string(m: ModifierKeyCode) -> &'static str {
    match m {
        ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift => "Shift",
        ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl => "Control",
        ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt => "Alt",
        ModifierKeyCode::LeftSuper | ModifierKeyCode::RightSuper => "Super",
        ModifierKeyCode::LeftHyper | ModifierKeyCode::RightHyper => "Hyper",
        ModifierKeyCode::LeftMeta | ModifierKeyCode::RightMeta => "Meta",
        ModifierKeyCode::IsoLevel3Shift | ModifierKeyCode::IsoLevel5Shift => "AltGraph",
    }
}

/// Flatten crossterm's modifier bitflags into the four-boolean
/// DOM shape (`ctrlKey` / `shiftKey` / `altKey` / `metaKey`).
/// Crossterm's `SUPER` and `META` flags both feed `meta` —
/// macOS terminals report Command as SUPER on most terminals,
/// and the platform distinction isn't useful at the substrate
/// level.
pub fn translate_modifiers(m: CtKeyModifiers) -> KeyboardModifiers {
    KeyboardModifiers {
        ctrl: m.contains(CtKeyModifiers::CONTROL),
        shift: m.contains(CtKeyModifiers::SHIFT),
        alt: m.contains(CtKeyModifiers::ALT),
        meta: m.contains(CtKeyModifiers::SUPER) || m.contains(CtKeyModifiers::META),
    }
}

/// Translate a crossterm `MouseEvent` into a substrate
/// [`MouseDetail`]. Cell-grained coordinates flow through to
/// `client_x` / `client_y` (DOM `MouseEvent.clientX/Y`); the
/// button-that-transitioned + held-buttons bitmask come from
/// [`mouse_kind_to_button`].
pub fn translate_mouse_event(ev: MouseEvent) -> MouseDetail {
    let (button, buttons) = mouse_kind_to_button(ev.kind);
    let (delta_x, delta_y) = wheel_delta(ev.kind);
    MouseDetail {
        button,
        buttons,
        client_x: ev.column as i32,
        client_y: ev.row as i32,
        delta_x,
        delta_y,
        modifiers: translate_modifiers(ev.modifiers),
    }
}

/// Wheel `(delta_x, delta_y)` in cell units — `0` for non-wheel
/// events, `±1` per scroll tick (DOM `WheelEvent` uses pixels;
/// terminals are cell-grained so we report ticks as units).
fn wheel_delta(kind: MouseEventKind) -> (i32, i32) {
    match kind {
        MouseEventKind::ScrollUp => (0, -1),
        MouseEventKind::ScrollDown => (0, 1),
        MouseEventKind::ScrollLeft => (-1, 0),
        MouseEventKind::ScrollRight => (1, 0),
        MouseEventKind::Down(_)
        | MouseEventKind::Up(_)
        | MouseEventKind::Drag(_)
        | MouseEventKind::Moved => (0, 0),
    }
}

/// Map a crossterm [`MouseEventKind`] to `(button, buttons-bitmask)`.
/// `button` is the button that transitioned (for press / release /
/// drag); for move and scroll events it defaults to
/// [`MouseButton::Left`] — the DOM "main button" sentinel for `0`.
/// `buttons` reflects what's currently held (bit 0 = Left, bit 1 =
/// Right, bit 2 = Middle) — crossterm doesn't track multi-button
/// state, so `Up` reports `0` (released) and other events report
/// just the bit for the transitioning button.
fn mouse_kind_to_button(kind: MouseEventKind) -> (MouseButton, u8) {
    match kind {
        MouseEventKind::Down(b) => (translate_mouse_button(b), button_bit(b)),
        MouseEventKind::Up(b) => (translate_mouse_button(b), 0),
        MouseEventKind::Drag(b) => (translate_mouse_button(b), button_bit(b)),
        MouseEventKind::Moved
        | MouseEventKind::ScrollUp
        | MouseEventKind::ScrollDown
        | MouseEventKind::ScrollLeft
        | MouseEventKind::ScrollRight => (MouseButton::Left, 0),
    }
}

fn translate_mouse_button(b: CtMouseButton) -> MouseButton {
    match b {
        CtMouseButton::Left => MouseButton::Left,
        CtMouseButton::Middle => MouseButton::Middle,
        CtMouseButton::Right => MouseButton::Right,
    }
}

/// DOM `MouseEvent.buttons` bitmask: bit 0 = Left, bit 1 = Right,
/// bit 2 = Middle.
fn button_bit(b: CtMouseButton) -> u8 {
    match b {
        CtMouseButton::Left => 1,   // bit 0
        CtMouseButton::Right => 2,  // bit 1
        CtMouseButton::Middle => 4, // bit 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode, modifiers: CtKeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    // ── Named key strings ─────────────────────────────────────────────

    #[test]
    fn navigation_keys() {
        assert_eq!(key_code_to_string(KeyCode::Enter), "Enter");
        assert_eq!(key_code_to_string(KeyCode::Esc), "Escape");
        assert_eq!(key_code_to_string(KeyCode::Tab), "Tab");
        assert_eq!(key_code_to_string(KeyCode::BackTab), "Tab");
        assert_eq!(key_code_to_string(KeyCode::Backspace), "Backspace");
        assert_eq!(key_code_to_string(KeyCode::Delete), "Delete");
        assert_eq!(key_code_to_string(KeyCode::Insert), "Insert");
    }

    #[test]
    fn arrow_keys() {
        assert_eq!(key_code_to_string(KeyCode::Up), "ArrowUp");
        assert_eq!(key_code_to_string(KeyCode::Down), "ArrowDown");
        assert_eq!(key_code_to_string(KeyCode::Left), "ArrowLeft");
        assert_eq!(key_code_to_string(KeyCode::Right), "ArrowRight");
    }

    #[test]
    fn paging_keys() {
        assert_eq!(key_code_to_string(KeyCode::Home), "Home");
        assert_eq!(key_code_to_string(KeyCode::End), "End");
        assert_eq!(key_code_to_string(KeyCode::PageUp), "PageUp");
        assert_eq!(key_code_to_string(KeyCode::PageDown), "PageDown");
    }

    #[test]
    fn lock_keys() {
        assert_eq!(key_code_to_string(KeyCode::CapsLock), "CapsLock");
        assert_eq!(key_code_to_string(KeyCode::NumLock), "NumLock");
        assert_eq!(key_code_to_string(KeyCode::ScrollLock), "ScrollLock");
    }

    #[test]
    fn screen_and_pause_keys() {
        assert_eq!(key_code_to_string(KeyCode::PrintScreen), "PrintScreen");
        assert_eq!(key_code_to_string(KeyCode::Pause), "Pause");
        assert_eq!(key_code_to_string(KeyCode::Menu), "ContextMenu");
        assert_eq!(key_code_to_string(KeyCode::KeypadBegin), "Clear");
    }

    #[test]
    fn null_is_unidentified() {
        assert_eq!(key_code_to_string(KeyCode::Null), "Unidentified");
    }

    #[test]
    fn f_keys_one_through_twelve() {
        for n in 1..=12u8 {
            assert_eq!(key_code_to_string(KeyCode::F(n)), format!("F{n}"));
        }
    }

    #[test]
    fn f_keys_high_numbers() {
        // F13–F24 also valid in DOM; crossterm reports them too.
        assert_eq!(key_code_to_string(KeyCode::F(24)), "F24");
    }

    #[test]
    fn printable_chars() {
        assert_eq!(key_code_to_string(KeyCode::Char('a')), "a");
        assert_eq!(key_code_to_string(KeyCode::Char('A')), "A");
        assert_eq!(key_code_to_string(KeyCode::Char('1')), "1");
        assert_eq!(key_code_to_string(KeyCode::Char('!')), "!");
        assert_eq!(key_code_to_string(KeyCode::Char(' ')), " ");
        assert_eq!(key_code_to_string(KeyCode::Char('é')), "é");
    }

    // ── Media keys ────────────────────────────────────────────────────

    #[test]
    fn media_keys() {
        use MediaKeyCode::*;
        assert_eq!(key_code_to_string(KeyCode::Media(Play)), "MediaPlay");
        assert_eq!(key_code_to_string(KeyCode::Media(Pause)), "MediaPause");
        assert_eq!(
            key_code_to_string(KeyCode::Media(PlayPause)),
            "MediaPlayPause"
        );
        assert_eq!(key_code_to_string(KeyCode::Media(Stop)), "MediaStop");
        assert_eq!(
            key_code_to_string(KeyCode::Media(FastForward)),
            "MediaFastForward"
        );
        assert_eq!(key_code_to_string(KeyCode::Media(Rewind)), "MediaRewind");
        // Reverse collapses to Rewind — documented divergence.
        assert_eq!(key_code_to_string(KeyCode::Media(Reverse)), "MediaRewind");
        assert_eq!(
            key_code_to_string(KeyCode::Media(TrackNext)),
            "MediaTrackNext"
        );
        assert_eq!(
            key_code_to_string(KeyCode::Media(TrackPrevious)),
            "MediaTrackPrevious"
        );
        assert_eq!(key_code_to_string(KeyCode::Media(Record)), "MediaRecord");
        assert_eq!(
            key_code_to_string(KeyCode::Media(LowerVolume)),
            "AudioVolumeDown"
        );
        assert_eq!(
            key_code_to_string(KeyCode::Media(RaiseVolume)),
            "AudioVolumeUp"
        );
        assert_eq!(
            key_code_to_string(KeyCode::Media(MuteVolume)),
            "AudioVolumeMute"
        );
    }

    // ── Modifier-only keys ────────────────────────────────────────────

    #[test]
    fn modifier_only_keys_collapse_left_and_right() {
        use ModifierKeyCode::*;
        assert_eq!(key_code_to_string(KeyCode::Modifier(LeftShift)), "Shift");
        assert_eq!(key_code_to_string(KeyCode::Modifier(RightShift)), "Shift");
        assert_eq!(
            key_code_to_string(KeyCode::Modifier(LeftControl)),
            "Control"
        );
        assert_eq!(
            key_code_to_string(KeyCode::Modifier(RightControl)),
            "Control"
        );
        assert_eq!(key_code_to_string(KeyCode::Modifier(LeftAlt)), "Alt");
        assert_eq!(key_code_to_string(KeyCode::Modifier(RightAlt)), "Alt");
        assert_eq!(key_code_to_string(KeyCode::Modifier(LeftSuper)), "Super");
        assert_eq!(key_code_to_string(KeyCode::Modifier(RightSuper)), "Super");
        assert_eq!(key_code_to_string(KeyCode::Modifier(LeftHyper)), "Hyper");
        assert_eq!(key_code_to_string(KeyCode::Modifier(RightHyper)), "Hyper");
        assert_eq!(key_code_to_string(KeyCode::Modifier(LeftMeta)), "Meta");
        assert_eq!(key_code_to_string(KeyCode::Modifier(RightMeta)), "Meta");
        assert_eq!(
            key_code_to_string(KeyCode::Modifier(IsoLevel3Shift)),
            "AltGraph"
        );
        assert_eq!(
            key_code_to_string(KeyCode::Modifier(IsoLevel5Shift)),
            "AltGraph"
        );
    }

    // ── Modifier bitflag translation ──────────────────────────────────

    #[test]
    fn modifiers_default_when_no_flags() {
        let m = translate_modifiers(CtKeyModifiers::empty());
        assert!(!m.ctrl && !m.shift && !m.alt && !m.meta);
    }

    #[test]
    fn modifiers_each_flag() {
        let m = translate_modifiers(CtKeyModifiers::CONTROL);
        assert!(m.ctrl && !m.shift && !m.alt && !m.meta);
        let m = translate_modifiers(CtKeyModifiers::SHIFT);
        assert!(!m.ctrl && m.shift && !m.alt && !m.meta);
        let m = translate_modifiers(CtKeyModifiers::ALT);
        assert!(!m.ctrl && !m.shift && m.alt && !m.meta);
    }

    #[test]
    fn modifiers_super_and_meta_both_feed_meta() {
        // macOS Command is reported as SUPER on most terminals.
        let from_super = translate_modifiers(CtKeyModifiers::SUPER);
        assert!(from_super.meta);
        let from_meta = translate_modifiers(CtKeyModifiers::META);
        assert!(from_meta.meta);
        let both = translate_modifiers(CtKeyModifiers::SUPER | CtKeyModifiers::META);
        assert!(both.meta);
    }

    #[test]
    fn modifiers_combined() {
        let m = translate_modifiers(CtKeyModifiers::CONTROL | CtKeyModifiers::SHIFT);
        assert!(m.ctrl && m.shift && !m.alt && !m.meta);
    }

    // ── Repeat flag ──────────────────────────────────────────────────

    #[test]
    fn repeat_unset_when_press_kind() {
        let d = translate_key_event(key(KeyCode::Char('a'), CtKeyModifiers::empty()));
        assert!(!d.repeat);
    }

    #[test]
    fn repeat_set_when_repeat_kind() {
        let mut k = key(KeyCode::Char('a'), CtKeyModifiers::empty());
        k.kind = KeyEventKind::Repeat;
        let d = translate_key_event(k);
        assert!(d.repeat);
    }

    #[test]
    fn repeat_unset_when_release_kind() {
        let mut k = key(KeyCode::Char('a'), CtKeyModifiers::empty());
        k.kind = KeyEventKind::Release;
        let d = translate_key_event(k);
        assert!(!d.repeat, "Release isn't a repeat");
    }

    // ── End-to-end key translation ────────────────────────────────────

    #[test]
    fn translate_enter_with_ctrl() {
        let d = translate_key_event(key(KeyCode::Enter, CtKeyModifiers::CONTROL));
        assert_eq!(d.key, "Enter");
        assert!(d.modifiers.ctrl);
        assert!(!d.modifiers.shift);
    }

    #[test]
    fn translate_backtab_reports_shifted_tab() {
        // BackTab is the wire-level encoding of Shift+Tab. Browsers
        // report this as `key = "Tab"` + `shiftKey = true`. Our
        // translation gives `key = "Tab"` but doesn't synthesize
        // the shift modifier — the wire format carries Shift in
        // the modifiers bits already.
        let d = translate_key_event(key(KeyCode::BackTab, CtKeyModifiers::SHIFT));
        assert_eq!(d.key, "Tab");
        assert!(d.modifiers.shift);
    }

    // ── MouseEvent translation ────────────────────────────────────────

    fn mouse(kind: MouseEventKind, column: u16, row: u16, m: CtKeyModifiers) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers: m,
        }
    }

    #[test]
    fn mouse_down_left_sets_bit_zero() {
        let d = translate_mouse_event(mouse(
            MouseEventKind::Down(CtMouseButton::Left),
            5,
            3,
            CtKeyModifiers::empty(),
        ));
        assert_eq!(d.button, MouseButton::Left);
        assert_eq!(d.buttons, 1);
        assert_eq!(d.client_x, 5);
        assert_eq!(d.client_y, 3);
        assert!(!d.modifiers.ctrl);
    }

    #[test]
    fn mouse_down_right_sets_bit_one() {
        let d = translate_mouse_event(mouse(
            MouseEventKind::Down(CtMouseButton::Right),
            0,
            0,
            CtKeyModifiers::empty(),
        ));
        assert_eq!(d.button, MouseButton::Right);
        assert_eq!(d.buttons, 2);
    }

    #[test]
    fn mouse_down_middle_sets_bit_two() {
        let d = translate_mouse_event(mouse(
            MouseEventKind::Down(CtMouseButton::Middle),
            0,
            0,
            CtKeyModifiers::empty(),
        ));
        assert_eq!(d.button, MouseButton::Middle);
        assert_eq!(d.buttons, 4);
    }

    #[test]
    fn mouse_up_clears_buttons() {
        let d = translate_mouse_event(mouse(
            MouseEventKind::Up(CtMouseButton::Left),
            0,
            0,
            CtKeyModifiers::empty(),
        ));
        assert_eq!(d.button, MouseButton::Left);
        assert_eq!(d.buttons, 0);
    }

    #[test]
    fn mouse_drag_carries_button_bit() {
        let d = translate_mouse_event(mouse(
            MouseEventKind::Drag(CtMouseButton::Left),
            7,
            2,
            CtKeyModifiers::empty(),
        ));
        assert_eq!(d.button, MouseButton::Left);
        assert_eq!(d.buttons, 1);
    }

    #[test]
    fn mouse_moved_defaults_to_left_no_bits() {
        let d = translate_mouse_event(mouse(MouseEventKind::Moved, 10, 4, CtKeyModifiers::empty()));
        assert_eq!(d.button, MouseButton::Left);
        assert_eq!(d.buttons, 0);
    }

    #[test]
    fn mouse_scroll_directions_default_to_left() {
        for kind in [
            MouseEventKind::ScrollUp,
            MouseEventKind::ScrollDown,
            MouseEventKind::ScrollLeft,
            MouseEventKind::ScrollRight,
        ] {
            let d = translate_mouse_event(mouse(kind, 0, 0, CtKeyModifiers::empty()));
            assert_eq!(d.button, MouseButton::Left);
            assert_eq!(d.buttons, 0);
        }
    }

    #[test]
    fn mouse_scroll_deltas_match_direction() {
        let cases = [
            (MouseEventKind::ScrollUp, (0, -1)),
            (MouseEventKind::ScrollDown, (0, 1)),
            (MouseEventKind::ScrollLeft, (-1, 0)),
            (MouseEventKind::ScrollRight, (1, 0)),
        ];
        for (kind, (dx, dy)) in cases {
            let d = translate_mouse_event(mouse(kind, 0, 0, CtKeyModifiers::empty()));
            assert_eq!(d.delta_x, dx);
            assert_eq!(d.delta_y, dy);
        }
    }

    #[test]
    fn non_wheel_events_have_zero_deltas() {
        let d = translate_mouse_event(mouse(
            MouseEventKind::Down(CtMouseButton::Left),
            0,
            0,
            CtKeyModifiers::empty(),
        ));
        assert_eq!(d.delta_x, 0);
        assert_eq!(d.delta_y, 0);
    }

    #[test]
    fn mouse_modifiers_propagate() {
        let d = translate_mouse_event(mouse(
            MouseEventKind::Down(CtMouseButton::Left),
            0,
            0,
            CtKeyModifiers::CONTROL | CtKeyModifiers::SHIFT,
        ));
        assert!(d.modifiers.ctrl);
        assert!(d.modifiers.shift);
        assert!(!d.modifiers.alt);
    }
}
