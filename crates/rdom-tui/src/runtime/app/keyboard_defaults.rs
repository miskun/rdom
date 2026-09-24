//! Keyboard defaults of an [`App`]: the `keydown` / `keyup` pipeline
//! ([`App::handle_key_event`]) and the runtime-owned default actions it
//! routes a key through — clipboard copy / cut / paste, undo / redo,
//! editable-key defaults (movement, Enter, character insert), selection
//! and scroll keys, focus navigation, and the Ctrl-C exit.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::App;
use crate::render::backend::Backend;
use crate::{TuiDispatchExt, TuiDom, TuiEvent};

impl<B: Backend> App<B> {
    /// Process one crossterm key event: `keyup` on release, otherwise
    /// the clipboard intercept, the Ctrl-C exit, `keydown` dispatch to
    /// the focused element (or root), Shift+F10 `contextmenu`, and the
    /// default actions when the handler did not prevent them. Called by
    /// [`App::handle_event`] with the scheduler guard already installed.
    pub(super) fn handle_key_event(&mut self, key: KeyEvent) {
        use crossterm::event::KeyEventKind;
        // Release events fire `keyup` to the focused
        // element only — no clipboard handling, no Ctrl-C
        // exit (terminals don't reliably report the
        // release event for those anyway), no default
        // actions. Pairs with `keydown` for symmetry.
        if key.kind == KeyEventKind::Release {
            let target = self.dom.focused().unwrap_or_else(|| self.dom.root());
            let mut tui = TuiEvent::keyup(key);
            let _ = self.dom.dispatch_tui_event(target, &mut tui);
            self.needs_redraw |= tui.event.redraw_requested();
            self.needs_redraw |= !self.tracker.roots_snapshot().is_empty();
            self.needs_redraw |= self.tracker.take_paint_dirty();
            return;
        }

        // Clipboard shortcuts intercept the key before it's
        // dispatched as a normal `keydown`. A non-collapsed
        // selection turns Ctrl-C into "copy" instead of
        // "quit" — matches how terminals + browsers behave.
        if try_handle_clipboard_key(self, key) {
            return;
        }

        // Ctrl-C: universal exit. Handler listeners don't
        // even see this — the runtime owns it.
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        // Dispatch `keydown` to the focused element (or root).
        // Both `Press` and `Repeat` flow through this path;
        // browsers don't distinguish auto-repeat keydown from
        // initial keydown either (the DOM `KeyboardEvent.repeat`
        // bit is on the detail, set by `key_translate`).
        let target = self.dom.focused().unwrap_or_else(|| self.dom.root());
        let mut tui = TuiEvent::keydown(key);
        let _ = self.dom.dispatch_tui_event(target, &mut tui);

        // Shift+F10 → contextmenu on the focused element
        // (accessibility / keyboard-only equivalent of
        // right-click). Dispatched as a fresh event after
        // keydown so a handler that wants to suppress the
        // keydown path doesn't accidentally swallow the
        // contextmenu intent. Synthesized; no mouse
        // coordinates (button=Left sentinel, buttons=0).
        if key.code == KeyCode::F(10) && key.modifiers.contains(KeyModifiers::SHIFT) {
            let mut cm = TuiEvent::new("contextmenu");
            cm.event = cm.event.clone().with_synthetic(true);
            let _ = self.dom.dispatch_tui_event(target, &mut cm);
            self.needs_redraw |= cm.event.redraw_requested();
        }

        // Default actions — only run if the handler didn't
        // call prevent_default. Selection-keyboard defaults
        // (Ctrl-A, Shift+arrows) run first so a selection
        // extend doesn't get overridden by focus nav.
        if !tui.event.default_prevented() {
            if crate::runtime::selection::keyboard::try_handle_key(&mut self.dom, key)
                || try_handle_editable_key(&mut self.dom, key)
                || crate::runtime::scrollbar::handle_scroll_key(&mut self.dom, key)
            {
                self.needs_redraw = true;
            } else {
                match key.code {
                    KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        crate::runtime::focus::tabindex::focus_prev(&mut self.dom);
                        self.needs_redraw = true;
                    }
                    KeyCode::Tab => {
                        crate::runtime::focus::tabindex::focus_next(&mut self.dom);
                        self.needs_redraw = true;
                    }
                    KeyCode::BackTab => {
                        // Some terminals report Shift+Tab as BackTab.
                        crate::runtime::focus::tabindex::focus_prev(&mut self.dom);
                        self.needs_redraw = true;
                    }
                    _ => {}
                }
            }
        }

        // Listener-requested repaint (state outside the DOM the
        // tracker can't see — e.g. a canvas reading app state).
        self.needs_redraw |= tui.event.redraw_requested();
        self.needs_redraw |= !self.tracker.roots_snapshot().is_empty();
        // Text-only mutations from event handlers don't dirty
        // the cascade (selectors don't match text content) but
        // they DO change painted output. Without this OR, a
        // handler that calls `set_node_value` is invisible
        // until the next event ticks the cascade.
        self.needs_redraw |= self.tracker.take_paint_dirty();
    }
}

/// Try to consume `key` as a clipboard shortcut (Ctrl-C / Ctrl-X /
/// Ctrl-V, or their Cmd-variants on macOS). Returns `true` when
/// the key was claimed — caller skips the rest of its keydown
/// pipeline (including the "Ctrl-C = quit" fallback).
///
/// Rules:
/// - **`copy` / `cut`**: only fire when the selection is
///   non-collapsed. Otherwise the key falls through (so Ctrl-C
///   without a selection still quits).
/// - **`paste`**: always fires when a focused element exists (or
///   the root as fallback). Clipboard may return `None` — the
///   event still fires so apps can trigger paste-empty UX.
fn try_handle_clipboard_key<B: Backend>(app: &mut App<B>, key: crossterm::event::KeyEvent) -> bool {
    let ctrl_or_super = key.modifiers.contains(KeyModifiers::CONTROL)
        || key.modifiers.contains(KeyModifiers::SUPER);
    if !ctrl_or_super {
        return false;
    }

    match key.code {
        KeyCode::Char('c') | KeyCode::Char('C') => do_copy(app),
        KeyCode::Char('x') | KeyCode::Char('X') => do_cut(app),
        KeyCode::Char('v') | KeyCode::Char('V') => do_paste(app),
        _ => false,
    }
}

fn do_copy<B: Backend>(app: &mut App<B>) -> bool {
    let Some((text, _range)) =
        crate::runtime::selection::clipboard::current_selection_text(&app.dom)
    else {
        return false;
    };
    let target = crate::runtime::selection::clipboard::copy_target(&app.dom)
        .unwrap_or_else(|| app.dom.root());
    let mut tui = TuiEvent::copy(text.clone());
    let _ = app.dom.dispatch_tui_event(target, &mut tui);
    if !tui.event.default_prevented() {
        app.clipboard.write_text(text);
    }
    true
}

fn do_cut<B: Backend>(app: &mut App<B>) -> bool {
    let Some((text, _range)) =
        crate::runtime::selection::clipboard::current_selection_text(&app.dom)
    else {
        return false;
    };
    let target = crate::runtime::selection::clipboard::copy_target(&app.dom)
        .unwrap_or_else(|| app.dom.root());
    let mut tui = TuiEvent::cut(text.clone());
    let _ = app.dom.dispatch_tui_event(target, &mut tui);
    if !tui.event.default_prevented() {
        app.clipboard.write_text(text);
        // If the cut target is an editable, delete the selected
        // range. Routes through `insert_at_selection(dom, "")` so
        // the delete fires `beforeinput`/`input` events and lands an
        // undo entry. Non-editable cut is copy-only (matches what
        // selection-in-prose "Cmd-X" usually does — copies but
        // doesn't delete, since prose isn't editable).
        if crate::node::nearest_editable_ancestor(&app.dom, target).is_some() {
            let _ = crate::runtime::editing::insert_at_selection(&mut app.dom, "");
        }
    }
    true
}

/// Undo / redo keydown default. Intercepts Ctrl-Z (or Cmd-Z) as
/// undo and Ctrl-Y / Ctrl-Shift-Z (or Cmd-Shift-Z) as redo. Gates
/// on focused editable; routes through `runtime::editing::undo_last`
/// / `redo_last`.
///
/// Runs before the movement / character handler so a bare 'z' still
/// types as text in an editable.
fn try_handle_history_key(dom: &mut TuiDom, key: crossterm::event::KeyEvent) -> bool {
    let ctrl_or_super = key.modifiers.contains(KeyModifiers::CONTROL)
        || key.modifiers.contains(KeyModifiers::SUPER);
    if !ctrl_or_super {
        return false;
    }
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    let is_undo = matches!(key.code, KeyCode::Char('z') | KeyCode::Char('Z')) && !shift;
    let is_redo = matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y'))
        || (matches!(key.code, KeyCode::Char('z') | KeyCode::Char('Z')) && shift);

    if is_undo {
        matches!(
            crate::runtime::editing::undo_last(dom),
            crate::runtime::editing::UndoOutcome::Applied
        )
    } else if is_redo {
        matches!(
            crate::runtime::editing::redo_last(dom),
            crate::runtime::editing::UndoOutcome::Applied
        )
    } else {
        false
    }
}

fn do_paste<B: Backend>(app: &mut App<B>) -> bool {
    let text = app.clipboard.read_text().unwrap_or_default();
    let target = app.dom.focused().unwrap_or_else(|| app.dom.root());
    let mut tui = TuiEvent::paste(text.clone());
    let _ = app.dom.dispatch_tui_event(target, &mut tui);
    // If the paste target is an editable and the event wasn't
    // prevented, insert the clipboard text at the current selection
    // (or replace the range if one's active). Non-editable paste is
    // a no-op at the framework level; apps can still intercept the
    // event to do something custom (e.g. open a pasted URL).
    if !tui.event.default_prevented()
        && crate::node::nearest_editable_ancestor(&app.dom, target).is_some()
    {
        let _ = crate::runtime::editing::insert_at_selection(&mut app.dom, &text);
    }
    true
}

/// Editable-keydown default action. Returns `true` when the key
/// was consumed (caller skips remaining defaults like Tab nav).
///
/// Runs in order:
/// 1. **Movement / deletion** — bare arrows, Ctrl+arrows, Home/End,
///    Ctrl+Home/End, Backspace, Delete. Routed through
///    `runtime::editing::movement`. Shift+arrow is already handled
///    upstream by `selection::keyboard`.
/// 2. **Printable character insert** — falls through when movement
///    didn't match. Plain chars (no Ctrl/Super) insert at the
///    current selection via `insert_at_selection`. Control
///    combinations belong to clipboard / selection paths which
///    already ran upstream.
fn try_handle_editable_key(dom: &mut TuiDom, key: crossterm::event::KeyEvent) -> bool {
    // Focused element must have an editable ancestor.
    let Some(focused) = dom.focused() else {
        return false;
    };
    if crate::node::nearest_editable_ancestor(dom, focused).is_none() {
        return false;
    };

    // Undo / redo. Ctrl-Z / Cmd-Z undoes; Ctrl-Y or Cmd-Shift-Z /
    // Ctrl-Shift-Z redoes. Must run before movement/character so a
    // bare 'z' in an editable doesn't short-circuit into character
    // insertion instead.
    if try_handle_history_key(dom, key) {
        return true;
    }

    // Movement / deletion keys.
    if crate::runtime::editing::movement::try_handle_movement_key(dom, key) {
        return true;
    }

    // Enter handling. `<input>` is single-line: bare Enter is
    // consumed but inserts nothing (form-submit handled separately).
    // Other editables (`<textarea>`, contenteditable) insert a
    // literal `\n` — `white-space: pre` on the textarea turns it
    // into a visible hard break.
    if matches!(key.code, KeyCode::Enter)
        && !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::SUPER | KeyModifiers::ALT)
    {
        let editable = crate::node::nearest_editable_ancestor(dom, focused);
        let is_input = editable
            .map(|id| dom.node(id).tag_name() == Some("input"))
            .unwrap_or(false);
        if is_input {
            return true;
        }
        let outcome = crate::runtime::editing::insert_at_selection(dom, "\n");
        return matches!(
            outcome,
            crate::runtime::editing::EditOutcome::Applied
                | crate::runtime::editing::EditOutcome::Prevented
        );
    }

    // Printable character insert. Skip modifier combos that belong
    // to other default actions upstream.
    if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::SUPER)
    {
        return false;
    }
    let ch = match key.code {
        KeyCode::Char(c) if !c.is_control() => c,
        _ => return false,
    };
    let outcome = crate::runtime::editing::insert_at_selection(dom, &ch.to_string());
    matches!(
        outcome,
        crate::runtime::editing::EditOutcome::Applied
            | crate::runtime::editing::EditOutcome::Prevented
    )
}
