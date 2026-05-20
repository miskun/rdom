//! Event-detail taxonomy — the typed payload types that travel
//! inside [`Event::detail`].
//!
//! ## Substrate rule
//!
//! These types live in `rdom-core` deliberately. `event.detail`
//! is the single canonical carrier for typed key / mouse data
//! after M4a step 8; that means the carrier types must live in
//! the substrate, not in `rdom-tui` next to crossterm. A
//! `crossterm::event::KeyCode → key: String` translation
//! helper at the `rdom-tui` input boundary (M4a step 7) is the
//! seam.
//!
//! ## Web fidelity
//!
//! - `MouseButton` mirrors the numeric `MouseEvent.button` values
//!   defined at <https://www.w3.org/TR/uievents/#dom-mouseevent-button>.
//! - `KeyboardModifiers` mirrors the four boolean accessors
//!   `ctrlKey` / `shiftKey` / `altKey` / `metaKey` on
//!   `KeyboardEvent` and `MouseEvent`.
//! - `InputType` mirrors a named subset of
//!   <https://w3c.github.io/input-events/#dom-inputevent-inputtype>,
//!   with an `Other(String)` escape hatch.
//! - `ToggleState` is the open/closed state shared by `<details>`
//!   and `<dialog>` `toggle` events.

/// DOM `MouseEvent.button` mapping.
///
/// DOM terminology calls button 1 (the middle/wheel button) the
/// "auxiliary button"; our [`MouseButton::Middle`] variant carries
/// that. Buttons 3+ (typically browser back/forward, then
/// vendor-defined) fall into [`MouseButton::Other`].
///
/// Spec: <https://www.w3.org/TR/uievents/#dom-mouseevent-button>.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Primary button (numeric value `0`). Usually the left button,
    /// or the un-initialized state for events that don't carry a
    /// button press.
    Left,
    /// Auxiliary button (numeric value `1`). Usually the
    /// middle / wheel button.
    Middle,
    /// Secondary button (numeric value `2`). Usually the right
    /// button.
    Right,
    /// Buttons 3 and above. Typically browser back (3) and
    /// browser forward (4); 5+ is vendor-defined.
    Other(i16),
}

/// Four-boolean modifier set, matching the
/// `KeyboardEvent.{ctrl,shift,alt,meta}Key` and
/// `MouseEvent.{ctrl,shift,alt,meta}Key` accessor shape.
///
/// `Default` is all-`false` — convenient for tests that synthesize
/// events without modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct KeyboardModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

/// UI Events `InputEvent.inputType` value.
///
/// The enumerated variants are the named values from the spec's
/// "Input Events Level 2" `inputType` attribute table that rdom
/// actually emits. Anything outside the named list — typically
/// composition events, formatting commands, or vendor extensions
/// rdom doesn't model — falls into [`InputType::Other`] carrying
/// the raw string.
///
/// Spec: <https://w3c.github.io/input-events/#dom-inputevent-inputtype>.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputType {
    InsertText,
    InsertReplacementText,
    InsertLineBreak,
    InsertParagraph,
    InsertFromPaste,
    InsertFromDrop,
    DeleteContentBackward,
    DeleteContentForward,
    DeleteByCut,
    DeleteWordBackward,
    DeleteWordForward,
    HistoryUndo,
    HistoryRedo,
    /// Catches anything not in the enumerated list.
    Other(String),
}

/// Open/closed state for `<details>` and `<dialog>` `toggle`
/// event detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToggleState {
    Open,
    Closed,
}

// ── EventDetail ─────────────────────────────────────────────────

use crate::node_id::NodeId;

/// Typed payload carried on [`Event::detail`](crate::Event#structfield.detail).
///
/// Replaces the pre-M4 `Option<String>` detail with a closed enum
/// of typed variants. The [`EventDetail::String`] variant
/// preserves the one-shot escape hatch authors used to lean on via
/// `Event::new("custom").with_detail("payload")`.
///
/// ## Variant boxing
///
/// Variants whose largest field is a [`String`] are boxed; inline
/// for fixed-size variants. This keeps the enum under 32 bytes on
/// 64-bit (a const-assert below enforces that).
///
/// ## Accessor pattern
///
/// Listeners read typed detail through the `as_*` accessors:
///
/// ```
/// # use rdom_core::{Event, EventDetail};
/// let mut e = Event::new("custom");
/// e.detail = EventDetail::String("payload".into());
/// assert_eq!(e.detail.as_string(), Some("payload"));
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub enum EventDetail {
    /// No detail attached. Default for plain events (`click`,
    /// `focus`, `blur`, …) and the initial state on `Event::new`.
    #[default]
    None,
    /// Free-form string payload. Used for `CustomEvent`-style
    /// author-fired events; also the migration target for any
    /// pre-M4 reader that stored a string in `Option<String>`.
    String(String),
    /// `transitionstart` / `transitionend` / `transitioncancel`
    /// payload — emitted by `runtime::animation`.
    Transition(Box<TransitionDetail>),
    /// `beforeinput` / `input` event payload — emitted by
    /// `<input>` / `<textarea>` and contenteditable elements.
    Input(Box<InputDetail>),
    /// `submit` event payload — emitted by `<form>`.
    Submit(Box<SubmitDetail>),
    /// `toggle` event payload — emitted by `<details>` and
    /// `<dialog>`.
    Toggle(Box<ToggleDetail>),
    /// Pointer event payload — `click` / `mousedown` / `mouseup`
    /// / `mousemove` / `wheel` / `contextmenu`. Inline; the
    /// struct is fixed-size.
    Mouse(MouseDetail),
    /// `keydown` / `keypress` / `keyup` payload. Boxed because
    /// the inner `key: String` would otherwise push the enum
    /// past its 32-byte budget.
    Keyboard(Box<KeyboardDetail>),
}

/// Permanent regression guard for [`EventDetail`]'s size budget.
/// Failure means a redesign is needed (likely boxing the variant
/// that grew).
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<EventDetail>() <= 32);

impl EventDetail {
    /// Borrow the payload as a `&str` iff this is
    /// [`EventDetail::String`]. Returns `None` for every other
    /// variant — typed-detail readers should use the matching
    /// `as_*` accessor instead.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            EventDetail::String(s) => Some(s),
            _ => None,
        }
    }

    /// Borrow the transition payload iff this is
    /// [`EventDetail::Transition`].
    pub fn as_transition(&self) -> Option<&TransitionDetail> {
        match self {
            EventDetail::Transition(t) => Some(t),
            _ => None,
        }
    }

    /// Borrow the input payload iff this is
    /// [`EventDetail::Input`].
    pub fn as_input(&self) -> Option<&InputDetail> {
        match self {
            EventDetail::Input(i) => Some(i),
            _ => None,
        }
    }

    /// Borrow the submit payload iff this is
    /// [`EventDetail::Submit`].
    pub fn as_submit(&self) -> Option<&SubmitDetail> {
        match self {
            EventDetail::Submit(s) => Some(s),
            _ => None,
        }
    }

    /// Borrow the toggle payload iff this is
    /// [`EventDetail::Toggle`].
    pub fn as_toggle(&self) -> Option<&ToggleDetail> {
        match self {
            EventDetail::Toggle(t) => Some(t),
            _ => None,
        }
    }

    /// Borrow the mouse payload iff this is
    /// [`EventDetail::Mouse`].
    pub fn as_mouse(&self) -> Option<&MouseDetail> {
        match self {
            EventDetail::Mouse(m) => Some(m),
            _ => None,
        }
    }

    /// Borrow the keyboard payload iff this is
    /// [`EventDetail::Keyboard`].
    pub fn as_keyboard(&self) -> Option<&KeyboardDetail> {
        match self {
            EventDetail::Keyboard(k) => Some(k),
            _ => None,
        }
    }
}

/// `transitionstart` / `transitionend` / `transitioncancel` event
/// payload. CSS Transitions Level 1 §5.1.
#[derive(Debug, Clone, PartialEq)]
pub struct TransitionDetail {
    /// Animatable property whose value crossed a transition
    /// boundary, in CSS-canonical kebab-case (`"color"`,
    /// `"background-color"`, …).
    pub property_name: String,
    /// Time elapsed since the transition started, in seconds.
    /// For `transitionstart`, always 0.0.
    pub elapsed: f64,
    /// Pseudo-element associated with the transition, or
    /// `None` if the transition is on the element itself.
    pub pseudo_element: Option<String>,
}

/// `beforeinput` / `input` event payload, per UI Events / Input
/// Events Level 2.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputDetail {
    /// What kind of edit produced this event. See [`InputType`].
    pub input_type: InputType,
    /// Text being inserted, or `None` for deletion-style events.
    pub data: Option<String>,
    /// `true` if this event fires as part of an IME composition
    /// sequence. rdom doesn't model IME directly; always `false`
    /// in M4. Reserved for future polish.
    pub is_composing: bool,
}

/// `submit` event payload, per HTML §form-submission.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubmitDetail {
    /// The element that triggered submission (the `<button>` /
    /// `<input type="submit">`), or `None` for programmatic
    /// `form.requestSubmit()` calls.
    pub submitter: Option<NodeId>,
}

/// `toggle` event payload — emitted by `<details>` and
/// `<dialog>` when their open/closed state changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToggleDetail {
    /// State before the toggle.
    pub old_state: ToggleState,
    /// State after the toggle.
    pub new_state: ToggleState,
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

/// `keydown` / `keypress` / `keyup` payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- MouseButton ---

    #[test]
    fn mouse_button_left_middle_right_are_unit_variants() {
        // Pattern-match each named variant to assert it exists
        // and is a unit variant.
        assert!(matches!(MouseButton::Left, MouseButton::Left));
        assert!(matches!(MouseButton::Middle, MouseButton::Middle));
        assert!(matches!(MouseButton::Right, MouseButton::Right));
    }

    #[test]
    fn mouse_button_other_carries_i16() {
        // 3 is browser-back per the DOM table; we don't bake that
        // mapping into the type — `Other` is just the catch-all.
        let back = MouseButton::Other(3);
        match back {
            MouseButton::Other(n) => assert_eq!(n, 3),
            _ => panic!("Other(3) didn't match Other"),
        }
    }

    #[test]
    fn mouse_button_is_copy_and_eq() {
        let b = MouseButton::Left;
        let c = b; // Copy.
        assert_eq!(b, c);
        assert_ne!(MouseButton::Left, MouseButton::Right);
        assert_ne!(MouseButton::Other(3), MouseButton::Other(4));
    }

    // --- KeyboardModifiers ---

    #[test]
    fn keyboard_modifiers_default_is_all_false() {
        let m = KeyboardModifiers::default();
        assert!(!m.ctrl);
        assert!(!m.shift);
        assert!(!m.alt);
        assert!(!m.meta);
    }

    #[test]
    fn keyboard_modifiers_field_struct_round_trips() {
        let m = KeyboardModifiers {
            ctrl: true,
            shift: false,
            alt: true,
            meta: false,
        };
        assert!(m.ctrl);
        assert!(!m.shift);
        assert!(m.alt);
        assert!(!m.meta);
    }

    #[test]
    fn keyboard_modifiers_is_copy_and_eq() {
        let a = KeyboardModifiers {
            ctrl: true,
            ..Default::default()
        };
        let b = a; // Copy.
        assert_eq!(a, b);

        let c = KeyboardModifiers {
            shift: true,
            ..Default::default()
        };
        assert_ne!(a, c);
    }

    // --- InputType ---

    #[test]
    fn input_type_named_variants_exist() {
        // Round-trip every named variant through equality. This
        // also serves as a compile-time inventory of the shipped
        // set — adding a new variant requires updating this list.
        let named = [
            InputType::InsertText,
            InputType::InsertReplacementText,
            InputType::InsertLineBreak,
            InputType::InsertParagraph,
            InputType::InsertFromPaste,
            InputType::InsertFromDrop,
            InputType::DeleteContentBackward,
            InputType::DeleteContentForward,
            InputType::DeleteByCut,
            InputType::DeleteWordBackward,
            InputType::DeleteWordForward,
            InputType::HistoryUndo,
            InputType::HistoryRedo,
        ];
        // Each variant is distinct from its neighbors.
        for (i, a) in named.iter().enumerate() {
            for (j, b) in named.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn input_type_other_carries_string() {
        let it = InputType::Other("formatBold".into());
        match &it {
            InputType::Other(s) => assert_eq!(s, "formatBold"),
            _ => panic!("Other didn't match Other"),
        }
    }

    #[test]
    fn input_type_other_differs_from_named_with_same_label() {
        // `Other("insertText")` is not equal to the named
        // `InsertText` variant — the named set is closed.
        assert_ne!(InputType::Other("insertText".into()), InputType::InsertText);
    }

    // --- ToggleState ---

    #[test]
    fn toggle_state_variants_exist_and_differ() {
        assert!(matches!(ToggleState::Open, ToggleState::Open));
        assert!(matches!(ToggleState::Closed, ToggleState::Closed));
        assert_ne!(ToggleState::Open, ToggleState::Closed);
    }

    #[test]
    fn toggle_state_is_copy() {
        let s = ToggleState::Open;
        let t = s; // Copy.
        assert_eq!(s, t);
    }

    // --- EventDetail ---

    #[test]
    fn event_detail_default_is_none() {
        let d: EventDetail = Default::default();
        assert!(matches!(d, EventDetail::None));
    }

    #[test]
    fn event_detail_string_round_trip_via_as_string() {
        // The canonical step-2 failing test: an `EventDetail::String`
        // payload round-trips through `as_string()`. This is the
        // migration target for every pre-M4 reader that used
        // `event.detail.as_deref()`.
        let d = EventDetail::String("payload".into());
        assert_eq!(d.as_string(), Some("payload"));
    }

    #[test]
    fn event_detail_as_string_returns_none_for_other_variants() {
        assert_eq!(EventDetail::None.as_string(), None);
        assert_eq!(
            EventDetail::Mouse(MouseDetail {
                button: MouseButton::Left,
                buttons: 0,
                client_x: 0,
                client_y: 0,
                delta_x: 0,
                delta_y: 0,
                modifiers: KeyboardModifiers::default(),
            })
            .as_string(),
            None
        );
    }

    #[test]
    fn event_detail_as_transition_round_trips() {
        let d = EventDetail::Transition(Box::new(TransitionDetail {
            property_name: "color".into(),
            elapsed: 0.25,
            pseudo_element: None,
        }));
        let t = d.as_transition().expect("variant matches");
        assert_eq!(t.property_name, "color");
        assert!((t.elapsed - 0.25).abs() < f64::EPSILON);
        assert!(t.pseudo_element.is_none());
        assert_eq!(d.as_string(), None);
    }

    #[test]
    fn event_detail_as_input_round_trips() {
        let d = EventDetail::Input(Box::new(InputDetail {
            input_type: InputType::InsertText,
            data: Some("a".into()),
            is_composing: false,
        }));
        let i = d.as_input().expect("variant matches");
        assert_eq!(i.input_type, InputType::InsertText);
        assert_eq!(i.data.as_deref(), Some("a"));
        assert!(!i.is_composing);
    }

    #[test]
    fn event_detail_as_submit_round_trips_with_none_submitter() {
        let d = EventDetail::Submit(Box::new(SubmitDetail { submitter: None }));
        let s = d.as_submit().expect("variant matches");
        assert!(s.submitter.is_none());
    }

    #[test]
    fn event_detail_as_toggle_round_trips() {
        let d = EventDetail::Toggle(Box::new(ToggleDetail {
            old_state: ToggleState::Closed,
            new_state: ToggleState::Open,
        }));
        let t = d.as_toggle().expect("variant matches");
        assert_eq!(t.old_state, ToggleState::Closed);
        assert_eq!(t.new_state, ToggleState::Open);
    }

    #[test]
    fn event_detail_as_mouse_round_trips() {
        let d = EventDetail::Mouse(MouseDetail {
            button: MouseButton::Right,
            buttons: 0b010,
            client_x: 12,
            client_y: 7,
            delta_x: 0,
            delta_y: -1,
            modifiers: KeyboardModifiers {
                ctrl: true,
                ..Default::default()
            },
        });
        let m = d.as_mouse().expect("variant matches");
        assert_eq!(m.button, MouseButton::Right);
        assert_eq!(m.buttons, 0b010);
        assert_eq!(m.client_x, 12);
        assert_eq!(m.client_y, 7);
        assert_eq!(m.delta_y, -1);
        assert!(m.modifiers.ctrl);
        assert!(!m.modifiers.shift);
    }

    #[test]
    fn event_detail_as_keyboard_round_trips() {
        let d = EventDetail::Keyboard(Box::new(KeyboardDetail {
            key: "Enter".into(),
            modifiers: KeyboardModifiers::default(),
            repeat: false,
        }));
        let k = d.as_keyboard().expect("variant matches");
        assert_eq!(k.key, "Enter");
        assert!(!k.repeat);
    }

    #[test]
    fn event_detail_accessor_cross_check() {
        // A non-matching `as_*` accessor returns `None`, not panic
        // or wrong-variant data.
        let s = EventDetail::String("hello".into());
        assert!(s.as_transition().is_none());
        assert!(s.as_input().is_none());
        assert!(s.as_submit().is_none());
        assert!(s.as_toggle().is_none());
        assert!(s.as_mouse().is_none());
        assert!(s.as_keyboard().is_none());
    }
}
