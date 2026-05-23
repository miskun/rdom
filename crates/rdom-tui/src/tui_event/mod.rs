//! `TuiEvent` — TUI-flavored wrapper around `rdom_core::Event`.
//!
//! After M4a step 8, `TuiEvent` is a thin wrapper around the core
//! `Event` whose only job is to give the runtime a strongly-typed
//! constructor surface (`TuiEvent::keydown(crossterm::KeyEvent)`,
//! `TuiEvent::click(crossterm::MouseEvent)`, …) and a dispatch
//! helper. All key/mouse data lives inside `event.detail` — typed
//! `EventDetail::Keyboard(Box<KeyboardDetail>)` and
//! `EventDetail::Mouse(MouseDetail)` populated by the
//! crossterm-translation seam in [`key_translate`].
//!
//! Listeners read through `ctx.event.detail.as_keyboard()` /
//! `.as_mouse()`. There is no parallel thread-local channel for
//! payload data — `runtime/current_event.rs` was retired in M4a
//! step 8 along with the `TuiEvent.key` / `TuiEvent.mouse` sibling
//! fields.
//!
//! ## Listener-ergonomics divergence
//!
//! Code that previously did
//! `match current_key()?.code { KeyCode::Enter => ... }` now does
//! `match key.key.as_str() { "Enter" => ... }`. DOM-faithful, but
//! loses compile-time exhaustiveness over named keys — string-match
//! typos won't fire at compile time. Apps that want strong typing
//! can pattern-match on a wrapper helper they own.

pub mod key_translate;

use crossterm::event::{KeyEvent, MouseEvent};
use rdom_core::{Event, EventDetail};

/// TuiEvent = thin wrapper around the core `Event`. The typed
/// payload (keyboard / mouse / input / submit / toggle / transition)
/// lives in `event.detail`; there are no parallel sibling fields.
///
/// ## Construction
///
/// ```
/// # use rdom_tui::TuiEvent;
/// # use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
/// # use rdom_core::InputType;
/// // Plain typed event (like CustomEvent).
/// let e = TuiEvent::new("close");
///
/// // Key event — translated into EventDetail::Keyboard.
/// let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
/// let e = TuiEvent::keydown(key);
/// assert_eq!(e.event.detail.as_keyboard().unwrap().key, "Enter");
///
/// // Input event with typed detail.
/// let e = TuiEvent::input(InputType::InsertText, Some("hello world".into()));
/// ```
#[derive(Debug, Clone)]
pub struct TuiEvent {
    /// The core event — routing state, flags, typed detail payload.
    pub event: Event,
}

impl TuiEvent {
    /// A plain TuiEvent — no payload on `event.detail`. Equivalent
    /// to a DOM `CustomEvent`.
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event: Event::new(event_type),
        }
    }

    /// Construct a `keydown` event from a crossterm `KeyEvent`.
    /// Bubbles by default, cancelable (so preventDefault can suppress
    /// the browser-ish default action). The key payload is translated
    /// into `EventDetail::Keyboard` via [`key_translate::translate_key_event`].
    pub fn keydown(key: KeyEvent) -> Self {
        let mut e = Self::new("keydown");
        e.event.detail = EventDetail::Keyboard(Box::new(key_translate::translate_key_event(key)));
        e
    }

    /// Construct a `keyup` event. See [`Self::keydown`] for detail shape.
    pub fn keyup(key: KeyEvent) -> Self {
        let mut e = Self::new("keyup");
        e.event.detail = EventDetail::Keyboard(Box::new(key_translate::translate_key_event(key)));
        e
    }

    /// Construct a `keypress` event (character-level input).
    /// See [`Self::keydown`] for detail shape.
    pub fn keypress(key: KeyEvent) -> Self {
        let mut e = Self::new("keypress");
        e.event.detail = EventDetail::Keyboard(Box::new(key_translate::translate_key_event(key)));
        e
    }

    /// Construct a `click` event from a crossterm `MouseEvent`. The
    /// mouse payload is translated into `EventDetail::Mouse` via
    /// [`key_translate::translate_mouse_event`].
    pub fn click(mouse: MouseEvent) -> Self {
        let mut e = Self::new("click");
        e.event.detail = EventDetail::Mouse(key_translate::translate_mouse_event(mouse));
        e
    }

    /// Construct a `mousedown` event.
    pub fn mousedown(mouse: MouseEvent) -> Self {
        let mut e = Self::new("mousedown");
        e.event.detail = EventDetail::Mouse(key_translate::translate_mouse_event(mouse));
        e
    }

    /// Construct a `mouseup` event.
    pub fn mouseup(mouse: MouseEvent) -> Self {
        let mut e = Self::new("mouseup");
        e.event.detail = EventDetail::Mouse(key_translate::translate_mouse_event(mouse));
        e
    }

    /// Construct a `mousemove` event.
    pub fn mousemove(mouse: MouseEvent) -> Self {
        let mut e = Self::new("mousemove");
        e.event.detail = EventDetail::Mouse(key_translate::translate_mouse_event(mouse));
        e
    }

    /// Construct a `mouseover` event (pointer entered the element).
    /// Bubbles; matches DOM.
    pub fn mouseover(mouse: MouseEvent) -> Self {
        let mut e = Self::new("mouseover");
        e.event.detail = EventDetail::Mouse(key_translate::translate_mouse_event(mouse));
        e
    }

    /// Construct a `mouseout` event (pointer left the element).
    /// Bubbles; matches DOM.
    pub fn mouseout(mouse: MouseEvent) -> Self {
        let mut e = Self::new("mouseout");
        e.event.detail = EventDetail::Mouse(key_translate::translate_mouse_event(mouse));
        e
    }

    /// Construct a `contextmenu` event from a crossterm
    /// `MouseEvent`. Fired on right-mouse-button down (and on
    /// `Shift+F10` / context-menu key — those paths construct
    /// via the [`Self::contextmenu_keyboard`] variant since they
    /// have no mouse coordinates). Bubbles, cancelable.
    pub fn contextmenu(mouse: MouseEvent) -> Self {
        let mut e = Self::new("contextmenu");
        e.event.detail = EventDetail::Mouse(key_translate::translate_mouse_event(mouse));
        e
    }

    /// Construct a `dblclick` event from a crossterm `MouseEvent`.
    /// Synthesized by the router on the second click in a
    /// 2-click sequence within the platform double-click window;
    /// fired in addition to the second `click`. Bubbles, cancelable.
    pub fn dblclick(mouse: MouseEvent) -> Self {
        let mut e = Self::new("dblclick");
        e.event.detail = EventDetail::Mouse(key_translate::translate_mouse_event(mouse));
        e
    }

    /// Construct a `wheel` event. The translator populates
    /// `MouseDetail.delta_x` / `delta_y` for the scroll direction
    /// (`±1` per tick); button-related fields default to the
    /// `Left`/`buttons=0` sentinels.
    pub fn wheel(mouse: MouseEvent) -> Self {
        let mut e = Self::new("wheel");
        e.event.detail = EventDetail::Mouse(key_translate::translate_mouse_event(mouse));
        e
    }

    /// Construct an `input` event with typed `EventDetail::Input`.
    /// Fires AFTER a text edit commits; non-cancelable.
    /// `input_type` says *what kind* of edit (insertion, deletion,
    /// undo, …); `data` carries the inserted text for inserts and
    /// `None` for deletions / history events, matching the DOM
    /// `InputEvent` convention.
    pub fn input(input_type: rdom_core::InputType, data: Option<String>) -> Self {
        let mut e = Self::new("input");
        e.event.detail = rdom_core::EventDetail::Input(Box::new(rdom_core::InputDetail {
            input_type,
            data,
            is_composing: false,
        }));
        e
    }

    /// Construct a `beforeinput` event. Fires BEFORE a text edit
    /// commits; cancelable — handlers that `prevent_default` stop
    /// the mutation (matches browser `InputEvent` semantics).
    /// Detail shape matches [`Self::input`].
    pub fn before_input(input_type: rdom_core::InputType, data: Option<String>) -> Self {
        let mut e = Self::new("beforeinput");
        e.event.detail = rdom_core::EventDetail::Input(Box::new(rdom_core::InputDetail {
            input_type,
            data,
            is_composing: false,
        }));
        e
    }

    /// Construct a `copy` event. `detail` carries the serialized
    /// selection text — handlers may `prevent_default` to suppress
    /// the runtime's default clipboard write (e.g. to write a
    /// custom format via their own clipboard shim).
    pub fn copy(text: impl Into<String>) -> Self {
        let mut e = Self::new("copy");
        e.event.detail = rdom_core::EventDetail::String(text.into());
        e
    }

    /// Construct a `cut` event. Same detail semantics as [`copy`].
    /// Default action copies the detail to the system clipboard;
    /// actually deleting the selected content is the handler's job
    /// (an `<input>` editor listens for `cut` and removes the
    /// range). Read-only views just get the copy behavior.
    pub fn cut(text: impl Into<String>) -> Self {
        let mut e = Self::new("cut");
        e.event.detail = rdom_core::EventDetail::String(text.into());
        e
    }

    /// Construct a `paste` event. `detail` carries the text read
    /// from the clipboard. A listener on an editable element
    /// inserts it; without a listener, the event is a no-op.
    pub fn paste(text: impl Into<String>) -> Self {
        let mut e = Self::new("paste");
        e.event.detail = rdom_core::EventDetail::String(text.into());
        e
    }

    /// Construct a `focus` event. **Does not bubble** — matches
    /// the browser DOM spec. For a bubbling variant, see
    /// [`Self::focusin`].
    pub fn focus() -> Self {
        let mut e = Self::new("focus");
        e.event = e.event.with_bubbles(false);
        e
    }

    /// Construct a `focusin` event — the bubbling counterpart to
    /// `focus`. Fires on the same transitions. Matches DOM spec.
    pub fn focusin() -> Self {
        Self::new("focusin")
    }

    /// Construct a `blur` event. **Does not bubble** — matches
    /// DOM spec. For a bubbling variant, see [`Self::focusout`].
    pub fn blur() -> Self {
        let mut e = Self::new("blur");
        e.event = e.event.with_bubbles(false);
        e
    }

    /// Construct a `focusout` event — the bubbling counterpart to
    /// `blur`. Fires on the same transitions. Matches DOM spec.
    pub fn focusout() -> Self {
        Self::new("focusout")
    }

    /// Construct a `close` event (commonly used by dialogs).
    pub fn close() -> Self {
        Self::new("close")
    }

    /// Borrow the wrapped core event.
    pub fn as_event(&self) -> &Event {
        &self.event
    }

    /// Mutably borrow the wrapped core event.
    pub fn as_event_mut(&mut self) -> &mut Event {
        &mut self.event
    }

    /// Take ownership of the wrapped core event, dropping TUI payload.
    pub fn into_event(self) -> Event {
        self.event
    }

    /// True when this event carries an `EventDetail::Keyboard` payload.
    pub fn is_key(&self) -> bool {
        matches!(self.event.detail, EventDetail::Keyboard(_))
    }

    /// True when this event carries an `EventDetail::Mouse` payload.
    pub fn is_mouse(&self) -> bool {
        matches!(self.event.detail, EventDetail::Mouse(_))
    }
}

/// Dispatch a `TuiEvent` through `rdom-core`'s capture → target →
/// bubble walk. Thin alias over `Dom::dispatch_event` — the trait
/// exists to give the runtime a single entrypoint, not to do any
/// extra work. Typed payload lives on `event.detail`; listeners
/// read it via `ctx.event.detail.as_keyboard()` / `as_mouse()` /
/// `as_input()` / etc.
pub trait TuiDispatchExt {
    /// Run a `TuiEvent` through rdom-core's capture → target →
    /// bubble walk.
    fn dispatch_tui_event(
        &mut self,
        target: rdom_core::NodeId,
        tui: &mut TuiEvent,
    ) -> rdom_core::Result<()>;
}

impl TuiDispatchExt for rdom_core::Dom<crate::TuiExt> {
    fn dispatch_tui_event(
        &mut self,
        target: rdom_core::NodeId,
        tui: &mut TuiEvent,
    ) -> rdom_core::Result<()> {
        self.dispatch_event(target, &mut tui.event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{
        KeyCode, KeyEventKind, KeyEventState, KeyModifiers, MouseButton, MouseEventKind,
    };

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    fn mouse_click(col: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: col,
            row,
            modifiers: KeyModifiers::empty(),
        }
    }

    // ── Construction ─────────────────────────────────────────────────

    #[test]
    fn new_plain_event() {
        let e = TuiEvent::new("close");
        assert_eq!(e.event.event_type, "close");
        assert!(!e.is_key());
        assert!(!e.is_mouse());
    }

    #[test]
    fn keydown_carries_typed_keyboard_detail() {
        let k = key(KeyCode::Enter, KeyModifiers::NONE);
        let e = TuiEvent::keydown(k);
        assert_eq!(e.event.event_type, "keydown");
        assert!(e.is_key());
        let kb = e
            .event
            .detail
            .as_keyboard()
            .expect("keydown must carry EventDetail::Keyboard");
        assert_eq!(kb.key, "Enter");
        assert!(!kb.repeat);
        assert!(!kb.modifiers.ctrl && !kb.modifiers.shift);
    }

    #[test]
    fn keyup_keypress() {
        let k = key(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(TuiEvent::keyup(k).event.event_type, "keyup");
        assert_eq!(TuiEvent::keypress(k).event.event_type, "keypress");
    }

    #[test]
    fn click_carries_typed_mouse_detail() {
        let m = mouse_click(5, 2);
        let e = TuiEvent::click(m);
        assert_eq!(e.event.event_type, "click");
        assert!(e.is_mouse());
        let md = e
            .event
            .detail
            .as_mouse()
            .expect("click must carry EventDetail::Mouse");
        assert_eq!(md.button, rdom_core::MouseButton::Left);
        assert_eq!(md.client_x, 5);
        assert_eq!(md.client_y, 2);
    }

    #[test]
    fn mouse_event_variants() {
        let m = mouse_click(0, 0);
        assert_eq!(TuiEvent::mousedown(m).event.event_type, "mousedown");
        assert_eq!(TuiEvent::mouseup(m).event.event_type, "mouseup");
        assert_eq!(TuiEvent::mousemove(m).event.event_type, "mousemove");
        assert_eq!(TuiEvent::wheel(m).event.event_type, "wheel");
    }

    #[test]
    fn input_carries_typed_detail() {
        let e = TuiEvent::input(rdom_core::InputType::InsertText, Some("hello world".into()));
        assert_eq!(e.event.event_type, "input");
        let i = e.event.detail.as_input().expect("typed Input detail");
        assert_eq!(i.input_type, rdom_core::InputType::InsertText);
        assert_eq!(i.data.as_deref(), Some("hello world"));
        assert!(!i.is_composing);
    }

    #[test]
    fn focus_blur_close_builtins() {
        assert_eq!(TuiEvent::focus().event.event_type, "focus");
        assert_eq!(TuiEvent::blur().event.event_type, "blur");
        assert_eq!(TuiEvent::close().event.event_type, "close");
    }

    // ── Integration with core Event ──────────────────────────────────

    #[test]
    fn as_event_mut_allows_stop_propagation() {
        let mut e = TuiEvent::new("click");
        assert!(!e.event.is_propagation_stopped());
        e.as_event_mut().stop_propagation();
        assert!(e.event.is_propagation_stopped());
    }

    #[test]
    fn into_event_drops_tui_payload() {
        let k = key(KeyCode::Enter, KeyModifiers::empty());
        let e = TuiEvent::keydown(k);
        let core = e.into_event();
        assert_eq!(core.event_type, "keydown");
    }

    #[test]
    fn detail_propagates_through_as_event() {
        let e = TuiEvent::input(rdom_core::InputType::InsertText, Some("typed".into()));
        assert_eq!(
            e.as_event()
                .detail
                .as_input()
                .and_then(|i| i.data.as_deref()),
            Some("typed")
        );
    }

    // ── Combined with dispatch (core side) ──────────────────────────

    #[test]
    fn core_dispatch_works_on_tui_event_inner() {
        use crate::prelude::*;
        use std::cell::Cell;
        use std::rc::Rc;

        let mut dom: TuiDom = TuiDom::new();
        let btn = dom.create_element("button");
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        dom.add_event_listener(
            btn,
            "click",
            ListenerOptions::default(),
            move |_ctx: &mut TuiEventCtx<'_>| {
                f.set(true);
            },
        )
        .unwrap();

        let mut tui = TuiEvent::click(mouse_click(3, 4));
        dom.dispatch_event(btn, tui.as_event_mut()).unwrap();
        assert!(fired.get());
    }
}
