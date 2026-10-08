//! HTML §6.12's popover algorithms (the 2026 text): check popover
//! validity, show popover, hide popover, and the removing steps' hide.
//! The stack walks they share — hide popovers until, hide popover stack
//! until, topmost popover ancestor — are in `stack`.
//!
//! A popover's *visibility state* is its top-layer membership
//! (`TopLayerKind::Popover`). The per-element state HTML keeps (opened in
//! popover mode, popover trigger, previously focused element, popover
//! hiding) and the document's (showing popover, hiding popover nesting
//! count, hint stack parent) are document data (`state`).
//!
//! **Termination.** HTML refuses to show a popover while the document is
//! showing or hiding one (show popover step 2), and every stack walk
//! hides a slice fixed when it starts plus one event-free pass, so
//! listeners cannot make a walk loop. rdom fires `toggle` synchronously,
//! inside the algorithm (DIVERGENCES §2), so a `toggle` listener cannot
//! show a popover either.

use rdom_core::{DomError, EventDetail, NodeId, ToggleDetail, ToggleState, TopLayerKind};

use super::stack::{hide_stack_until, topmost_auto_or_hint, topmost_popover_ancestor};
use super::state::{
    Popovers, data, forget, hint_stack_parent, opened_mode, recorded_mode, showing_list,
};
use super::{PopoverState, is_showing, popover_state};
use crate::{TuiDom, TuiEvent};

/// "Check popover validity" for the `togglePopover(force)` no-op branch:
/// throws as `expectedToBeShowing` = the current state.
pub(super) fn check_attribute(dom: &TuiDom, id: NodeId) -> rdom_core::Result<()> {
    check(dom, id, is_showing(dom, id), true).map(|_| ())
}

/// HTML "check popover validity" given `expect_showing` (the expected
/// document is always this one): `Ok(false)` when the element is not in
/// the expected visibility state or — without `throw` — not a valid
/// popover; the exceptions as errors with `throw`.
fn check(dom: &TuiDom, id: NodeId, expect_showing: bool, throw: bool) -> rdom_core::Result<bool> {
    let fail = |e: DomError| if throw { Err(e) } else { Ok(false) };
    // Step 1.
    if popover_state(dom, id).is_none() {
        return fail(DomError::NotSupported(
            "popover: the element has no `popover` attribute",
        ));
    }
    // Step 2.
    if is_showing(dom, id) != expect_showing {
        return Ok(false);
    }
    // Step 3: not connected (to show), a modal dialog.
    if !expect_showing && (!dom.contains(id) || !dom.node(id).is_connected()) {
        return fail(DomError::InvalidState(
            "popover: the element is not connected",
        ));
    }
    if dom.top_layer_kind(id) == Some(TopLayerKind::ModalDialog) {
        return fail(DomError::InvalidState(
            "popover: the element is a modal dialog",
        ));
    }
    Ok(true)
}

/// Run `body`, then `cleanup` — on every path, a panic in a listener
/// included (the panic is re-raised after), so no flag outlives its run.
fn guarded<T>(
    dom: &mut TuiDom,
    body: impl FnOnce(&mut TuiDom) -> T,
    cleanup: impl FnOnce(&mut TuiDom),
) -> T {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body(dom)));
    cleanup(dom);
    match outcome {
        Ok(v) => v,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Fire a non-bubbling `beforetoggle` / `toggle` at `id`; returns whether
/// it went uncanceled.
fn fire(
    dom: &mut TuiDom,
    id: NodeId,
    ty: &str,
    open: bool,
    cancelable: bool,
    source: Option<NodeId>,
) -> bool {
    let (old, new) = if open {
        (ToggleState::Closed, ToggleState::Open)
    } else {
        (ToggleState::Open, ToggleState::Closed)
    };
    let mut ev = TuiEvent::new(ty);
    ev.event.bubbles = false;
    ev.event.cancelable = cancelable;
    ev.event.detail =
        EventDetail::Toggle(Box::new(ToggleDetail::new(old, new).with_source(source)));
    crate::tui_event::dispatch_to_live(dom, id, &mut ev) && !ev.event.default_prevented()
}

/// An exception of a step, or a plain return without `throw`.
fn refuse(throw: bool, message: &'static str) -> rdom_core::Result<()> {
    if throw {
        Err(DomError::InvalidState(message))
    } else {
        Ok(())
    }
}

/// HTML §6.12 "show popover" given `throw` and `source`.
pub(super) fn show(
    dom: &mut TuiDom,
    id: NodeId,
    throw: bool,
    source: Option<NodeId>,
) -> rdom_core::Result<()> {
    // Step 2: no show while another popover shows or hides.
    let d = data(dom);
    if d.showing_popover || d.hiding_nesting != 0 {
        return refuse(throw, "showPopover: another popover is showing or hiding");
    }
    // Steps 3–4.
    if !check(dom, id, false, throw)? {
        return Ok(());
    }
    // Steps 7–8: "showing popover", cleared by cleanupShowingSteps.
    data(dom).showing_popover = true;
    guarded(
        dom,
        |dom| show_steps(dom, id, throw, source),
        |dom| data(dom).showing_popover = false,
    )
}

/// "Show popover" steps 9–24, inside the "showing popover" flag.
fn show_steps(
    dom: &mut TuiDom,
    id: NodeId,
    throw: bool,
    source: Option<NodeId>,
) -> rdom_core::Result<()> {
    // Steps 9–11.
    if !fire(dom, id, "beforetoggle", true, true, source) || !check(dom, id, false, throw)? {
        return Ok(());
    }
    // Steps 12–14.
    let should_restore_focus;
    let original = popover_state(dom, id).expect("checked above");
    let mut effective = original;
    let mut ancestor = None;
    // Step 15.
    if original == PopoverState::Manual {
        should_restore_focus = false;
    } else {
        ancestor = topmost_popover_ancestor(dom, id, source, true);
        // 15.2: an auto popover under a hint is a hint.
        if let Some(a) = ancestor
            && opened_mode(dom, a) == Some(PopoverState::Hint)
            && effective == PopoverState::Auto
        {
            effective = PopoverState::Hint;
        }
        // 15.3–15.4 (shouldRestoreFocus is false here).
        hide_stack_until(dom, ancestor, PopoverState::Hint, false, true);
        if effective == PopoverState::Auto {
            hide_stack_until(dom, ancestor, PopoverState::Auto, false, true);
        }
        // 15.5–15.7: the hiding ran listeners.
        if popover_state(dom, id) != Some(original) {
            return refuse(
                throw,
                "showPopover: the popover attribute changed while showing",
            );
        }
        if !check(dom, id, false, throw)? {
            return Ok(());
        }
        // 15.8: only the first popover of a stack restores focus.
        should_restore_focus = topmost_auto_or_hint(dom).is_none();
    }
    // Steps 16–18.
    let originally_focused = dom.focused();
    dom.add_to_top_layer(id, TopLayerKind::Popover)?;
    let d = data(dom);
    d.previously_focused.remove(&id);
    if original != PopoverState::Manual {
        // 15.9: "opened in popover mode".
        d.opened.retain(|&(p, _)| p != id);
        d.opened.push((id, effective));
    }
    // Step 19.
    if effective == PopoverState::Hint
        && let Some(a) = ancestor
        && recorded_mode(dom, a) == Some(PopoverState::Auto)
    {
        data(dom).hint_stack_parent = Some(a);
    }
    // Step 21: "popover trigger".
    let d = data(dom);
    match source {
        Some(s) => d.trigger.insert(id, s),
        None => d.trigger.remove(&id),
    };
    // Steps 23–24.
    focusing_steps(dom, id);
    if should_restore_focus
        && popover_state(dom, id).is_some()
        && let Some(previous) = originally_focused
    {
        data(dom).previously_focused.insert(id, previous);
    }
    // Step 26, synchronously (DIVERGENCES §2).
    fire(dom, id, "toggle", true, false, source);
    Ok(())
}

/// The popover focusing steps (HTML §6.12): a `<dialog>`'s dialog
/// focusing steps; otherwise the element itself when it has
/// `autofocus`, else its autofocus delegate.
fn focusing_steps(dom: &mut TuiDom, id: NodeId) {
    if dom.node(id).tag_name() == Some("dialog") {
        crate::runtime::builtins::dialog::focusing_steps(dom, id);
    } else {
        crate::runtime::autofocus::focus_within_opened(dom, id);
    }
}

/// The arguments of one "hide popover" run.
#[derive(Debug, Clone, Copy)]
pub(super) struct Hide {
    pub(super) focus_previous: bool,
    pub(super) fire_events: bool,
    pub(super) throw: bool,
    /// The events' source (an invoker's activation passes the button).
    pub(super) source: Option<NodeId>,
    /// Skip the attribute's validity: the attribute change steps hide a
    /// popover whose attribute has already changed (Blink hides it before
    /// storing the new state; HTML's check would refuse it).
    pub(super) ignore_dom_state: bool,
}

impl Hide {
    /// `hidePopover()`: focus restored, events, exceptions.
    pub(super) const THROWING: Self = Self {
        focus_previous: true,
        fire_events: true,
        throw: true,
        source: None,
        ignore_dom_state: false,
    };

    /// A popover hidden by a stack walk.
    pub(super) const fn quiet(focus_previous: bool, fire_events: bool) -> Self {
        Self {
            focus_previous,
            fire_events,
            throw: false,
            source: None,
            ignore_dom_state: false,
        }
    }
}

/// Whether the hide algorithm may go on: the popover is showing and,
/// unless `ignore_dom_state`, still a valid popover.
fn still_valid(dom: &TuiDom, id: NodeId, how: Hide) -> rdom_core::Result<bool> {
    if how.ignore_dom_state {
        Ok(is_showing(dom, id))
    } else {
        check(dom, id, true, how.throw)
    }
}

/// HTML §6.12 "hide popover algorithm".
pub(super) fn hide(dom: &mut TuiDom, id: NodeId, how: Hide) -> rdom_core::Result<()> {
    // Steps 1–2.
    if !still_valid(dom, id, how)? {
        return Ok(());
    }
    // Steps 4–7: "popover hiding" (a nested hide fires no events) and the
    // document's nesting count.
    let d = data(dom);
    let nested = !d.hiding.insert(id);
    d.hiding_nesting += 1;
    let how = Hide {
        fire_events: how.fire_events && !nested,
        ..how
    };
    // Step 8: cleanupSteps.
    guarded(
        dom,
        |dom| hide_steps(dom, id, how),
        |dom| {
            let d = data(dom);
            if !nested {
                d.hiding.remove(&id);
            }
            d.hiding_nesting -= 1;
        },
    )
}

/// "Hide popover" steps 9–22, inside the flags.
fn hide_steps(dom: &mut TuiDom, id: NodeId, how: Hide) -> rdom_core::Result<()> {
    // Steps 9–10.
    let auto_contains = showing_list(dom, PopoverState::Auto).contains(&id);
    let hint_contains = showing_list(dom, PopoverState::Hint).contains(&id);
    // Step 11.
    if recorded_mode(dom, id).is_some() {
        let (fp, fe) = (how.focus_previous, how.fire_events);
        if hint_contains {
            hide_stack_until(dom, Some(id), PopoverState::Hint, fp, fe);
        }
        if hint_stack_parent(dom) == Some(id) {
            hide_stack_until(dom, None, PopoverState::Hint, fp, fe);
        }
        if auto_contains {
            hide_stack_until(dom, Some(id), PopoverState::Auto, fp, fe);
        }
        if !still_valid(dom, id, how)? {
            return Ok(());
        }
    }
    // Step 12.
    if how.fire_events {
        fire(dom, id, "beforetoggle", false, false, how.source);
        if !still_valid(dom, id, how)? {
            return Ok(());
        }
    }
    // Steps 12.4 / 13: out of the top layer (rdom has no overlay
    // transition to wait for), then steps 14–17.
    dom.remove_from_top_layer(id);
    forget(dom, id);
    // Step 18.
    if how.fire_events {
        fire(dom, id, "toggle", false, false, how.source);
    }
    // Steps 19–20.
    let previous = data(dom).previously_focused.remove(&id);
    if let Some(previous) = previous
        && how.focus_previous
        && dom
            .focused()
            .is_some_and(|f| super::stack::is_inclusive_ancestor(dom, id, f))
        && dom.contains(previous)
        && dom.node(previous).is_connected()
    {
        crate::runtime::focus::focus_node_with_options(
            dom,
            Some(previous),
            crate::runtime::focus::FocusOptions::new().prevent_scroll(true),
        );
    }
    Ok(())
}

/// HTML's removing steps for the popovers the tree took out of the top
/// layer since the last call (rdom-core's removing steps drop a removed
/// element from the top layer at once; the stack walk cannot run inside
/// a mutation observer): "hide popover" given the popover, no focus, no
/// events — so the popovers nested above it hide too — and the state of
/// every popover that no longer shows is dropped. The `App` runs this at
/// its next event or frame, with the attribute change steps.
pub(crate) fn flush_removed(dom: &mut TuiDom) {
    let Some(d) = dom.document_data::<Popovers>() else {
        return;
    };
    let removed: Vec<(NodeId, PopoverState)> = d
        .opened
        .iter()
        .copied()
        .filter(|&(id, _)| !is_showing(dom, id))
        .collect();
    for (id, mode) in removed {
        // Hide steps 11.1–11.3, with the removed popover still in its
        // recorded list (the endpoint of the walks).
        if mode == PopoverState::Hint {
            hide_stack_until(dom, Some(id), PopoverState::Hint, false, false);
        }
        if hint_stack_parent(dom) == Some(id) {
            hide_stack_until(dom, None, PopoverState::Hint, false, false);
        }
        if mode == PopoverState::Auto {
            hide_stack_until(dom, Some(id), PopoverState::Auto, false, false);
        }
        forget(dom, id);
        data(dom).previously_focused.remove(&id);
    }
    // A manual popover keeps a trigger and a previously focused element
    // too: drop them once it no longer shows.
    let d = data(dom);
    let gone: Vec<NodeId> = d
        .trigger
        .keys()
        .chain(d.previously_focused.keys())
        .copied()
        .collect();
    for id in gone {
        if !dom.contains(id) || !is_showing(dom, id) {
            let d = data(dom);
            d.trigger.remove(&id);
            d.previously_focused.remove(&id);
        }
    }
}
