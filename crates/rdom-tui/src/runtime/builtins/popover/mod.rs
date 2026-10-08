//! The `popover` attribute (HTML §6.12): popovers shown and hidden in
//! the document's top layer.
//!
//! ## Contract
//!
//! - **States** — [`popover_state`]: `auto` (or the empty string),
//!   `manual`, `hint`; an invalid value is `manual`, no attribute no
//!   popover. A popover is *showing* while it is in the top layer as
//!   `TopLayerKind::Popover` — what `:popover-open` matches.
//! - **API** — [`show_popover`] / [`show_popover_from`] (`showPopover()`,
//!   with a source), [`hide_popover`], [`toggle_popover`]: HTML §6.12.2's
//!   algorithms, errors as its exceptions (`DomError::NotSupported` for an
//!   element without the attribute, `DomError::InvalidState` for one not
//!   connected or a modal dialog).
//! - **Events** — `beforetoggle` before a change (cancelable when
//!   showing), `toggle` after, both non-bubbling with
//!   `EventDetail::Toggle` (old / new state, the invoking source);
//!   dispatched synchronously (DIVERGENCES §2).
//! - **Stacks** — showing an `auto` popover hides the auto popovers that
//!   are not its ancestors (through the DOM or through its invoker) and
//!   every `hint`; a `hint` opens above the auto stack and hides the other
//!   hints; hiding a popover hides the ones nested above it first.
//!   `manual` popovers stand alone. The stacks are the top layer's
//!   popovers by the mode each was shown in (`algorithms`).
//! - **Focus** — showing runs the popover focusing steps (the
//!   `[autofocus]` element, or a `<dialog>`'s focusing steps); hiding an
//!   auto or hint popover that holds the focus returns it to the element
//!   focused before it opened.
//! - **Invokers** — a button's `popovertarget` (`popovertargetaction`
//!   `toggle` / `show` / `hide`) acts on the popover it names when the
//!   button is activated (`invoker`).
//! - **Attribute changes** — a showing popover whose `popover` attribute
//!   changes state or goes is hidden, with its events, when the `App`
//!   next handles an event or draws a frame (`attribute`).
//! - **Light dismiss** — a press and release outside an auto / hint
//!   popover (and its invoker) hides it and those above it; Esc hides the
//!   topmost one when it is the most recent close watcher
//!   (`light_dismiss`).
//! - **Rendering** — the UA sheet hides `[popover]` until it shows and
//!   centres it in the viewport (HTML's rendering rules); the top layer
//!   paints it above the document (`paint_pass::top_layer`).
//!
//! Anchor positioning (`position-anchor`, a popover placed against its
//! invoker) is C15-ANCHOR: the invoker is recorded per popover
//! ([`invoker_of`]) for it.

mod algorithms;
pub(crate) mod attribute;
mod invoker;
pub(crate) mod light_dismiss;

pub use light_dismiss::topmost_close_watcher;

#[cfg(test)]
mod tests;

use rdom_core::{ListenerOptions, NodeId};

use crate::TuiDom;

/// The state of an element's `popover` attribute (HTML §6.12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PopoverState {
    /// `auto` or `""`: light-dismissable, one stack per document.
    Auto,
    /// `manual` (and any invalid value): shown and hidden by script or
    /// an invoker only.
    Manual,
    /// `hint`: a light-dismissable popover above the auto stack.
    Hint,
}

/// The state of `id`'s `popover` attribute: `None` without one (the
/// missing value default, "no popover"). Keywords match ASCII
/// case-insensitively; an invalid value is [`PopoverState::Manual`].
pub fn popover_state(dom: &TuiDom, id: NodeId) -> Option<PopoverState> {
    PopoverState::of(dom.node(id).get_attribute("popover"))
}

impl PopoverState {
    /// The state a `popover` attribute value gives (`None`: no
    /// attribute, no popover).
    fn of(value: Option<&str>) -> Option<Self> {
        let value = value?;
        Some(if value.is_empty() || value.eq_ignore_ascii_case("auto") {
            Self::Auto
        } else if value.eq_ignore_ascii_case("hint") {
            Self::Hint
        } else {
            Self::Manual
        })
    }
}

/// Whether `id`'s popover is showing — `:popover-open`.
pub fn is_showing(dom: &TuiDom, id: NodeId) -> bool {
    dom.top_layer_kind(id) == Some(rdom_core::TopLayerKind::Popover)
}

/// `showPopover()` (HTML §6.12.2 "show popover", throwing): see the
/// module contract. Showing a popover that is already showing does
/// nothing.
pub fn show_popover(dom: &mut TuiDom, id: NodeId) -> rdom_core::Result<()> {
    show_popover_from(dom, id, None)
}

/// `showPopover({ source })`: [`show_popover`] with `source` as the
/// popover's invoker — it nests the popover in the popover holding
/// `source`, and is the `toggle` events' source.
pub fn show_popover_from(
    dom: &mut TuiDom,
    id: NodeId,
    source: Option<NodeId>,
) -> rdom_core::Result<()> {
    algorithms::show(dom, id, true, source)
}

/// `hidePopover()` (HTML §6.12.2 "hide popover", throwing, focusing the
/// previous element, firing events). Hiding a hidden popover does
/// nothing.
pub fn hide_popover(dom: &mut TuiDom, id: NodeId) -> rdom_core::Result<()> {
    algorithms::hide(dom, id, algorithms::Hide::THROWING)
}

/// `togglePopover(force)`: hide a showing popover unless `force` is
/// `Some(true)`, show a hidden one unless it is `Some(false)`. Returns
/// whether the popover is showing afterwards.
pub fn toggle_popover(
    dom: &mut TuiDom,
    id: NodeId,
    force: Option<bool>,
) -> rdom_core::Result<bool> {
    if is_showing(dom, id) && force != Some(true) {
        hide_popover(dom, id)?;
    } else if !is_showing(dom, id) && force != Some(false) {
        show_popover(dom, id)?;
    } else {
        // Validity is checked either way (HTML's `togglePopover` step 1).
        algorithms::check_attribute(dom, id)?;
    }
    Ok(is_showing(dom, id))
}

/// HTML §4.11.4 `showModal()`'s popover steps: hide every auto and hint
/// popover that is not an ancestor of the dialog `id` about to be modal
/// (the topmost one holding it, hint then auto, bounds the hiding).
pub(crate) fn hide_unrelated_to(dom: &mut TuiDom, id: NodeId) {
    let hint = algorithms::showing_list(dom, PopoverState::Hint);
    let auto = algorithms::showing_list(dom, PopoverState::Auto);
    let until = algorithms::topmost_popover_ancestor(dom, id, &hint, None, false)
        .or_else(|| algorithms::topmost_popover_ancestor(dom, id, &auto, None, false));
    algorithms::hide_all_until(dom, until, false, true);
}

/// The element that invoked `id`'s showing popover — its `popovertarget`
/// button or the `showPopover()` source — while it shows.
pub fn invoker_of(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    algorithms::invoker_of(dom, id)
}

/// Install the popover default actions: a root-level `click` listener
/// running a `popovertarget` button's activation behavior, and a
/// `keydown` one sending Esc to the topmost auto / hint popover when it
/// is the most recent close watcher (`light_dismiss`). The mouse router
/// runs the pointer half of light dismiss.
pub fn install(dom: &mut TuiDom) {
    let root = dom.root();
    dom.add_event_listener(root, "keydown", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        let Some(key) = ctx.event.detail.as_keyboard() else {
            return;
        };
        let m = key.modifiers;
        if key.key != "Escape" || m.ctrl || m.shift || m.alt || m.meta {
            return;
        }
        if light_dismiss::close_request(ctx.dom) {
            ctx.event.prevent_default();
        }
    })
    .expect("popover Esc listener install");
    dom.add_event_listener(root, "click", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        let Some(target) = ctx.event.target else {
            return;
        };
        invoker::activate(ctx.dom, target);
    })
    .expect("popover invoker listener install");
}
