//! Implicit-detach event dispatch.
//!
//! When the focused or hovered element is detached from the tree
//! (directly or because an ancestor is being removed), browsers
//! dispatch a small ceremony of events:
//!
//! - **Focus loss:** `blur` (non-bubbling) + `focusout` (bubbles)
//!   on the previously-focused element.
//! - **Hover loss:** `mouseout` (bubbles) + `mouseleave` (non-
//!   bubbling) on the previously-hovered element.
//!
//! These events fire BEFORE the actual removal in the DOM event
//! model, so bubbling works through the still-intact ancestor
//! chain. `rdom-core::tree::detach_from_parent` emits a
//! `Mutation::PreDetach` record before structural unlink for
//! exactly this hook: this module installs an observer that
//! listens for `PreDetach` and dispatches the appropriate events
//! via the normal `TuiDispatchExt` pipeline, which still walks
//! the live parent chain at the moment of the observer callback.
//!
//! Installed once from `App::with_backend`. No public surface — the
//! observer is opaque; consumers register their own `blur`/
//! `focusout`/`mouseout`/`mouseleave` listeners on whichever
//! nodes they care about and get fired automatically.

use rdom_core::Mutation;

use crate::{TuiDispatchExt, TuiDom, TuiEvent};

/// Observer that translates `Mutation::PreDetach` records into
/// the implicit DOM events browsers fire when the focused or
/// hovered element is removed.
pub(crate) struct ImplicitDetachEvents;

impl rdom_core::MutationObserver<crate::TuiExt> for ImplicitDetachEvents {
    fn observe(&mut self, dom: &mut TuiDom, record: &Mutation) {
        let Mutation::PreDetach {
            detached_root: _,
            focused,
            hovered,
        } = record
        else {
            return;
        };
        // Focus loss ceremony — `blur` non-bubbling, `focusout`
        // bubbling. The order is browser-faithful: blur first,
        // then focusout. Both fire on the same target; the
        // bubbling difference is on the event itself.
        if let Some(target) = focused {
            let mut blur = TuiEvent::new("blur");
            blur.event.bubbles = false;
            blur.event = blur.event.clone().with_synthetic(true);
            let _ = dom.dispatch_tui_event(*target, &mut blur);

            let mut focusout = TuiEvent::new("focusout");
            focusout.event = focusout.event.clone().with_synthetic(true);
            let _ = dom.dispatch_tui_event(*target, &mut focusout);
        }
        // Hover loss ceremony — `mouseout` bubbling, `mouseleave`
        // non-bubbling. Same target, same event-flag difference.
        // Order: mouseout first (matches browser).
        if let Some(target) = hovered {
            let mut mouseout = TuiEvent::new("mouseout");
            mouseout.event = mouseout.event.clone().with_synthetic(true);
            let _ = dom.dispatch_tui_event(*target, &mut mouseout);

            let mut mouseleave = TuiEvent::new("mouseleave");
            mouseleave.event.bubbles = false;
            mouseleave.event = mouseleave.event.clone().with_synthetic(true);
            let _ = dom.dispatch_tui_event(*target, &mut mouseleave);
        }
    }
}

/// Install the implicit-detach observer on `dom`. Called once
/// during `App::with_backend`.
pub(crate) fn install(dom: &mut TuiDom) {
    dom.add_mutation_observer(Box::new(ImplicitDetachEvents));
}
