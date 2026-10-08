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

mod form;
#[cfg(test)]
mod tests;
mod ui;

pub use form::{FormEnctype, FormMethod, InputDetail, SubmitDetail};
pub use ui::{KeyboardDetail, MouseDetail, ToggleDetail};

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
#[non_exhaustive]
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
    /// `animationstart` / `animationiteration` / `animationend` /
    /// `animationcancel` payload — emitted by `runtime::animation`.
    Animation(Box<AnimationDetail>),
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

    /// Borrow the animation payload iff this is
    /// [`EventDetail::Animation`].
    pub fn as_animation(&self) -> Option<&AnimationDetail> {
        match self {
            EventDetail::Animation(a) => Some(a),
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
#[non_exhaustive]
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

impl TransitionDetail {
    /// A transition event payload (`TransitionEventInit`): the property
    /// in CSS kebab-case, the elapsed seconds and the pseudo-element
    /// (`"::before"`), if any.
    pub fn new(
        property_name: impl Into<String>,
        elapsed: f64,
        pseudo_element: Option<String>,
    ) -> Self {
        Self {
            property_name: property_name.into(),
            elapsed,
            pseudo_element,
        }
    }
}

/// `animationstart` / `animationiteration` / `animationend` /
/// `animationcancel` event payload (CSS Animations 1 §5.1
/// `AnimationEvent`).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct AnimationDetail {
    /// The `animation-name` of the animation, as written.
    pub animation_name: String,
    /// The animation's time when the event happened, in seconds, not
    /// counting its delay (§5.1: `elapsedTime`).
    pub elapsed: f64,
    /// The pseudo-element the animation runs on (`"::before"`), or
    /// `None` on the element itself.
    pub pseudo_element: Option<String>,
}

impl AnimationDetail {
    /// An animation event payload (`AnimationEventInit`): the name, the
    /// elapsed seconds and the pseudo-element, if any.
    pub fn new(
        animation_name: impl Into<String>,
        elapsed: f64,
        pseudo_element: Option<String>,
    ) -> Self {
        Self {
            animation_name: animation_name.into(),
            elapsed,
            pseudo_element,
        }
    }
}
