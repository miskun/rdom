//! `<input type="checkbox">` + `<input type="radio">` default
//! actions — click toggles the `checked` attribute, Space activates
//! a focused widget, and arrow keys navigate within a radio group.
//!
//! ## Contract (from MDN)
//!
//! - **Checkbox**: click toggles `checked` (presence-based boolean).
//!   Click is cancelable — `preventDefault` blocks the toggle. After
//!   the state changes, `input` and `change` both fire on the
//!   element (non-cancelable, in that order).
//! - **Radio**: click selects the target and unchecks every other
//!   `<input type="radio">` with the same `name` attribute (the
//!   "radio group"). Re-clicking the already-checked radio is a
//!   no-op — radios cannot be deselected by clicking. Same
//!   click→input→change event sequence.
//! - **Space on focus**: synthesizes a click on the focused widget,
//!   matching the `<button>` keyboard activation pattern (C.3).
//!   Modifier combos (Ctrl/Super/Alt) skip — they belong to
//!   clipboard / app-level shortcuts.
//! - **Arrow keys on radio**: Up/Left moves focus to the previous
//!   radio in the group; Down/Right to the next. Wraps at the ends.
//!   Home/End / Tab management is left to the existing focus path.
//!
//! ## State model
//!
//! V1 collapses HTML's `checked` attribute (initial state) and the
//! IDL `.checked` property (current state) into a single source: the
//! `checked` attribute's presence. Toggle on click, remove on
//! un-toggle. The `:checked` pseudo-class (added in C.4b in
//! `rdom-core/selectors.rs`) matches the same presence test, so the
//! cascade lights up the UA `::before` content rule for the
//! "checked" glyph automatically.
//!
//! ## Glyphs (UA stylesheet)
//!
//! Glyphs come from `::before` content rules in the UA stylesheet
//! (see `style/stylesheet.rs`):
//!
//! - `[type=checkbox]::before        { content: "[ ] " }`
//! - `[type=checkbox]:checked::before{ content: "[x] " }`
//! - `[type=radio]::before           { content: "( ) " }`
//! - `[type=radio]:checked::before   { content: "(•) " }`

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use rdom_core::{ListenerOptions, NodeId};

use crate::tui_event::TuiDispatchExt;
use crate::{TuiDom, TuiEvent};

/// Install the checkbox/radio default actions: the Dom's activation
/// hook (click → flip before dispatch, `input` + `change` or revert
/// after), plus two root-level keydown listeners — Space (synthesize
/// click) and arrows (radio navigation).
pub fn install(dom: &mut TuiDom) {
    use std::cell::RefCell;
    use std::rc::Rc;
    let root = dom.root();

    // HTML §4.10.5.1.15 activation behavior for checkbox / radio, run by
    // the Dom's activation hook (DOM §2.9 steps 5.5 and 11) rather than
    // by listeners: the state flips in the *pre-activation* step before
    // any listener sees the click, and the post step fires `input` +
    // `change` or reverts a canceled click — regardless of
    // `stopPropagation()`, which only affects listeners. The pending
    // flips are keyed by widget so nested dispatches cannot mix them up.
    let pending: Rc<RefCell<Vec<(NodeId, ToggleUndo)>>> = Rc::new(RefCell::new(Vec::new()));
    dom.set_activation_hook(Some(Box::new(move |dom, target, event, phase| {
        if event.event_type != "click" {
            return;
        }
        let Some(widget) = closest_toggle(dom, target) else {
            return;
        };
        match phase {
            rdom_core::ActivationPhase::Pre => {
                if dom.is_actually_disabled(widget) {
                    return;
                }
                let undo = pre_activate(dom, widget);
                pending.borrow_mut().push((widget, undo));
            }
            rdom_core::ActivationPhase::Post { canceled } => {
                let entry = {
                    let mut p = pending.borrow_mut();
                    p.iter()
                        .rposition(|(w, _)| *w == widget)
                        .map(|i| p.remove(i))
                };
                let Some((_, undo)) = entry else {
                    return; // disabled, or never pre-activated
                };
                if canceled {
                    revert(dom, widget, undo);
                } else if undo != ToggleUndo::Nothing {
                    fire_input_and_change(dom, widget);
                }
            }
        }
    })));

    // Space on focused checkbox/radio → synthesize click. Mirrors
    // the `<button>` activation flow from C.3 so apps that listen
    // for `click` see the same event regardless of input source.
    dom.add_event_listener(root, "keydown", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        let Some(focused) = ctx.dom.focused() else {
            return;
        };
        if !is_toggle(ctx.dom, focused) {
            return;
        }
        let Some(key) = ctx.event.detail.as_keyboard() else {
            return;
        };
        if key.modifiers.ctrl || key.modifiers.meta || key.modifiers.alt {
            return;
        }
        if key.key != " " {
            return;
        }
        let fake_mouse = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        };
        let mut click = TuiEvent::click(fake_mouse);
        click.event = click.event.clone().with_synthetic(true);
        let _ = ctx.dom.dispatch_tui_event(focused, &mut click);
    })
    .expect("toggle space listener install");

    // Arrow keys on focused radio → move focus within the
    // `name`-keyed radio group. Up/Left = previous, Down/Right =
    // next. Wraps. Per HTML, the radio group is a single tab
    // stop; Tab still moves to the next focusable area outside
    // the group (handled by the standard tabindex flow, which
    // we don't intercept here).
    dom.add_event_listener(root, "keydown", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        let Some(focused) = ctx.dom.focused() else {
            return;
        };
        if !is_radio(ctx.dom, focused) {
            return;
        }
        let Some(key) = ctx.event.detail.as_keyboard() else {
            return;
        };
        if key.modifiers.ctrl || key.modifiers.shift || key.modifiers.alt || key.modifiers.meta {
            return;
        }
        let direction = match key.key.as_str() {
            "ArrowUp" | "ArrowLeft" => -1i32,
            "ArrowDown" | "ArrowRight" => 1i32,
            _ => return,
        };
        move_focus_within_group(ctx.dom, focused, direction);
    })
    .expect("toggle arrow listener install");
}

// ── State change ───────────────────────────────────────────────────

/// What `revert` needs to undo a pre-activation flip.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ToggleUndo {
    /// Checkbox flipped; `revert` flips it back.
    Checkbox,
    /// Radio checked; the group member that was checked before, if any.
    Radio { previously_checked: Option<NodeId> },
    /// Click on an already-checked radio: no state change.
    Nothing,
}

/// Pre-activation step: flip a checkbox, or check a radio and uncheck
/// the rest of its group. Returns what it did so a canceled click can
/// undo it. No-op on an already-checked radio (HTML rule: radios can't
/// be deselected by clicking).
fn pre_activate(dom: &mut TuiDom, widget: NodeId) -> ToggleUndo {
    let was_checked = dom.node(widget).has_attribute("checked");
    note_default_checked(dom, widget);
    if is_radio(dom, widget) {
        if was_checked {
            return ToggleUndo::Nothing;
        }
        let siblings = radio_group(dom, widget);
        for &sib in &siblings {
            note_default_checked(dom, sib);
        }
        let previously_checked = siblings
            .iter()
            .copied()
            .find(|&sib| sib != widget && dom.node(sib).has_attribute("checked"));
        for sib in siblings {
            if sib != widget {
                let _ = dom.remove_attribute(sib, "checked");
            }
        }
        let _ = dom.set_attribute(widget, "checked", "");
        return ToggleUndo::Radio { previously_checked };
    }
    if was_checked {
        let _ = dom.remove_attribute(widget, "checked");
    } else {
        let _ = dom.set_attribute(widget, "checked", "");
    }
    ToggleUndo::Checkbox
}

/// Record the widget's current checkedness as its `defaultChecked`
/// unless already known (`FORM-DEFAULTS-1`); a `<form>` reset restores it.
pub(crate) fn note_default_checked(dom: &mut TuiDom, widget: NodeId) {
    let known = dom
        .node(widget)
        .ext()
        .is_some_and(|e| e.default_checked.is_some());
    if known {
        return;
    }
    let checked = dom.node(widget).has_attribute("checked");
    if let Some(ext) = dom.node_mut(widget).ext_mut() {
        ext.default_checked = Some(checked);
    }
}

/// Restore a checkbox / radio to its `defaultChecked`. No-op without a
/// recorded default.
pub(crate) fn reset_to_default(dom: &mut TuiDom, widget: NodeId) {
    let Some(default) = dom.node(widget).ext().and_then(|e| e.default_checked) else {
        return;
    };
    if default {
        let _ = dom.set_attribute(widget, "checked", "");
    } else {
        let _ = dom.remove_attribute(widget, "checked");
    }
}

/// Legacy-canceled-activation behavior: put the state back the way
/// `pre_activate` found it.
fn revert(dom: &mut TuiDom, widget: NodeId, undo: ToggleUndo) {
    match undo {
        ToggleUndo::Nothing => {}
        ToggleUndo::Checkbox => {
            if dom.node(widget).has_attribute("checked") {
                let _ = dom.remove_attribute(widget, "checked");
            } else {
                let _ = dom.set_attribute(widget, "checked", "");
            }
        }
        ToggleUndo::Radio { previously_checked } => {
            let _ = dom.remove_attribute(widget, "checked");
            if let Some(prev) = previously_checked {
                let _ = dom.set_attribute(prev, "checked", "");
            }
        }
    }
}

/// `input` and `change` both fire post-mutation, in that order. Both
/// are non-cancelable per MDN — handlers can observe but not block (the
/// cancelable hook is the click event upstream).
fn fire_input_and_change(dom: &mut TuiDom, widget: NodeId) {
    let mut input_ev = TuiEvent::new("input");
    let _ = dom.dispatch_tui_event(widget, &mut input_ev);
    let mut change_ev = TuiEvent::new("change");
    let _ = dom.dispatch_tui_event(widget, &mut change_ev);
}

// ── Radio group navigation ─────────────────────────────────────────

/// Move focus to the previous (`-1`) or next (`+1`) radio in the
/// same `name`-keyed group as `focused`. Wraps. No-op when the
/// group has only the focused element.
fn move_focus_within_group(dom: &mut TuiDom, focused: NodeId, direction: i32) {
    let group = radio_group(dom, focused);
    if group.len() < 2 {
        return;
    }
    let Some(idx) = group.iter().position(|&id| id == focused) else {
        return;
    };
    let len = group.len() as i32;
    let next = ((idx as i32 + direction).rem_euclid(len)) as usize;
    crate::runtime::focus::focus_node(dom, Some(group[next]));
}

/// Collect every `<input type="radio">` with the same `name`
/// attribute as `widget`, in document order. Includes `widget`
/// itself. Radios without a `name` attribute (or with empty `name`)
/// don't form a group — return just `widget` so callers see a
/// single-element list and treat the navigation as a no-op.
pub(crate) fn radio_group(dom: &TuiDom, widget: NodeId) -> Vec<NodeId> {
    let name = dom
        .node(widget)
        .get_attribute("name")
        .map(|s| s.to_string());
    let Some(name) = name.filter(|s| !s.is_empty()) else {
        return vec![widget];
    };
    let mut out = Vec::new();
    walk_radios_with_name(dom, dom.root(), &name, &mut out);
    out
}

fn walk_radios_with_name(dom: &TuiDom, id: NodeId, name: &str, out: &mut Vec<NodeId>) {
    if is_radio(dom, id) && dom.node(id).get_attribute("name") == Some(name) {
        out.push(id);
    }
    for child in dom.node(id).child_nodes() {
        walk_radios_with_name(dom, child.id(), name, out);
    }
}

// ── Tag / type helpers ─────────────────────────────────────────────

/// Walk up from `id` (inclusive) to the nearest `<input
/// type="checkbox|radio">`. The widget itself is the most common
/// target (clicking the glyph), but a wrapping author element
/// (e.g. a styled `<span>` overlay) might be the actual target —
/// the closest walk handles both cases.
fn closest_toggle(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if is_toggle(dom, n) {
            return Some(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
}

pub(crate) fn is_toggle(dom: &TuiDom, id: NodeId) -> bool {
    matches!(
        dom.input_type_state(id),
        Some(rdom_core::InputTypeState::Checkbox | rdom_core::InputTypeState::Radio)
    )
}

fn is_radio(dom: &TuiDom, id: NodeId) -> bool {
    dom.input_type_state(id) == Some(rdom_core::InputTypeState::Radio)
}

#[cfg(test)]
mod tests;
