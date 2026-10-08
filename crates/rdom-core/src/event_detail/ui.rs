//! The user-interface event payloads: `toggle` / `beforetoggle`
//! ([`ToggleDetail`]), the pointer events ([`MouseDetail`]) and the
//! keyboard events ([`KeyboardDetail`]).

use super::{KeyboardModifiers, MouseButton, ToggleState};

/// `toggle` / `beforetoggle` event payload (HTML `ToggleEvent`) —
/// emitted by `<details>`, `<dialog>` and popovers when their
/// open / closed state changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ToggleDetail {
    /// State before the toggle.
    pub old_state: ToggleState,
    /// State after the toggle.
    pub new_state: ToggleState,
    /// `ToggleEvent.source`: the element that invoked the change — a
    /// popover's `popovertarget` button, or the source a
    /// `showPopover()` call named. `None` otherwise.
    pub source: Option<crate::NodeId>,
}

impl ToggleDetail {
    /// A `toggle` payload (`ToggleEventInit`), without a source.
    pub const fn new(old_state: ToggleState, new_state: ToggleState) -> Self {
        Self {
            old_state,
            new_state,
            source: None,
        }
    }

    /// The same payload with `source` as its `ToggleEvent.source`.
    pub const fn with_source(mut self, source: Option<crate::NodeId>) -> Self {
        self.source = source;
        self
    }
}

/// Pointer event payload — `click`, `mousedown`, `mouseup`,
/// `mousemove`, `wheel`, `contextmenu`.
///
/// Coordinates are in cell units (column / row) — terminals
/// don't have subpixel positioning. `client_x` / `client_y`
/// match the DOM `MouseEvent` field names regardless.
///
/// `wheel` events fold `WheelEvent` into the same struct: `delta_x`
/// / `delta_y` are populated for `wheel` (positive = right / down,
/// per DOM `WheelEvent.deltaX` / `deltaY`) and `0` for all other
/// pointer events. Keeping one struct simplifies the substrate;
/// `delta_z` and `delta_mode` are omitted because terminals don't
/// surface them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct MouseDetail {
    /// Which button transitioned (for press/release/click), or
    /// `MouseButton::Left` (DOM "main button" sentinel for `0`)
    /// when no button is meaningful (e.g., `mousemove`, `wheel`).
    pub button: MouseButton,
    /// Bitmask of buttons currently held — bit 0 = Left, bit 1 =
    /// Right, bit 2 = Middle. Matches the DOM `MouseEvent.buttons`
    /// bitfield.
    pub buttons: u8,
    /// Column in cells. DOM `MouseEvent.clientX` analog.
    pub client_x: i32,
    /// Row in cells. DOM `MouseEvent.clientY` analog.
    pub client_y: i32,
    /// Horizontal wheel delta in cell units (positive = scroll
    /// right). `0` for non-`wheel` events. DOM `WheelEvent.deltaX`.
    pub delta_x: i32,
    /// Vertical wheel delta in cell units (positive = scroll
    /// down). `0` for non-`wheel` events. DOM `WheelEvent.deltaY`.
    pub delta_y: i32,
    /// Modifiers held when the event fired.
    pub modifiers: KeyboardModifiers,
}

impl MouseDetail {
    /// A pointer payload (`MouseEventInit`) at cell (`client_x`,
    /// `client_y`): `button` transitioned, no button held, no wheel
    /// delta, no modifier. The `with_*` builders set the rest.
    pub const fn new(button: MouseButton, client_x: i32, client_y: i32) -> Self {
        Self {
            button,
            buttons: 0,
            client_x,
            client_y,
            delta_x: 0,
            delta_y: 0,
            modifiers: KeyboardModifiers {
                ctrl: false,
                shift: false,
                alt: false,
                meta: false,
            },
        }
    }

    /// Set the held-buttons bitmask (`MouseEvent.buttons`).
    pub const fn with_buttons(mut self, buttons: u8) -> Self {
        self.buttons = buttons;
        self
    }

    /// Set the wheel deltas (`WheelEvent.deltaX` / `deltaY`).
    pub const fn with_delta(mut self, delta_x: i32, delta_y: i32) -> Self {
        self.delta_x = delta_x;
        self.delta_y = delta_y;
        self
    }

    /// Set the modifiers held.
    pub const fn with_modifiers(mut self, modifiers: KeyboardModifiers) -> Self {
        self.modifiers = modifiers;
        self
    }
}

/// `keydown` / `keypress` / `keyup` payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct KeyboardDetail {
    /// DOM `KeyboardEvent.key` — the printable character or named
    /// key value (`"Enter"`, `"ArrowLeft"`, `"a"`, `"F5"`, …).
    /// Translation from `crossterm::KeyCode` lives in
    /// `rdom-tui::tui_event::key_translate` (M4a step 7).
    pub key: String,
    /// Modifiers held during the press.
    pub modifiers: KeyboardModifiers,
    /// `true` for OS-generated repeats of a held key.
    pub repeat: bool,
}

impl KeyboardDetail {
    /// A key payload (`KeyboardEventInit`) for `key` with `modifiers`,
    /// not a repeat.
    pub fn new(key: impl Into<String>, modifiers: KeyboardModifiers) -> Self {
        Self {
            key: key.into(),
            modifiers,
            repeat: false,
        }
    }

    /// Set `repeat` (an OS-generated repeat of a held key).
    pub fn with_repeat(mut self, repeat: bool) -> Self {
        self.repeat = repeat;
        self
    }
}
