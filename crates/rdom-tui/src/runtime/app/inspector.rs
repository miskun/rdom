//! What an inspector does to an App (`specs/CAPTURE.md` §Edits): change a
//! sheet's rule, force an element into a user-action state, and draw the
//! frame that change causes with every animation it starts taken as the
//! inspector's ([`App::inspect`]) — whose events app code is not shown
//! ([`App::take_suppressed_events`]). App code cannot tell: no event, no
//! mutation record, and `matches()` reports the real state.

use std::io;
use std::rc::Rc;

use rdom_core::{ForcedState, NodeId};

use super::{App, StylesheetId};
use crate::render::Backend;
use crate::runtime::animation::SuppressedEvent;
use crate::style::Stylesheet;

impl<B: Backend> App<B> {
    /// Apply an inspector's change `f` and draw the frame it causes, every
    /// transition or animation that frame starts taken as the inspector's:
    /// its events are withheld from app code. Settle the app first (a
    /// capture session does: each step draws its frame), so the frame
    /// holds the inspector's change alone.
    pub fn inspect<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> io::Result<T> {
        self.animations.set_inspecting(true);
        let out = f(self);
        let drawn = self.advance(0);
        self.animations.set_inspecting(false);
        drawn.map(|()| out)
    }

    /// Force element `id` into `state` (`on`) or stop forcing it — for the
    /// cascade only (`Dom::set_forced_state`); the element restyles at the
    /// next frame. Call inside [`inspect`](Self::inspect).
    pub fn force_state(&mut self, id: NodeId, state: ForcedState, on: bool) {
        if !self.dom.contains(id) {
            return;
        }
        self.dom.set_forced_state(id, state, on);
        self.tracker.mark_state_changed(&mut self.dom, id);
    }

    /// Stop forcing every state on every element, and forget which
    /// animations an inspector started.
    pub fn clear_forced_states(&mut self) {
        let forced: Vec<NodeId> = self
            .dom
            .forced_elements()
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        self.dom.clear_forced_states();
        for id in forced {
            if self.dom.contains(id) {
                self.tracker.mark_state_changed(&mut self.dom, id);
            }
        }
        self.animations.clear_inspector_causes();
    }

    /// The events inspector-caused animations fired since the last call,
    /// which app code was not shown.
    pub fn take_suppressed_events(&mut self) -> Vec<SuppressedEvent> {
        self.animations.take_suppressed()
    }

    /// The App's sheet `id`, if it is on the stack.
    pub fn stylesheet(&self, id: StylesheetId) -> Option<&Stylesheet> {
        self.stylesheets
            .iter()
            .find(|(sid, _)| *sid == id)
            .map(|(_, s)| &**s)
    }

    /// Change the App's sheet `id` through `f` (copied first when shared);
    /// the next frame re-cascades. `None` when `id` is not on the stack.
    pub fn edit_stylesheet<T>(
        &mut self,
        id: StylesheetId,
        f: impl FnOnce(&mut Stylesheet) -> T,
    ) -> Option<T> {
        let (_, sheet) = self.stylesheets.iter_mut().find(|(sid, _)| *sid == id)?;
        let out = f(Rc::make_mut(sheet));
        self.invalidate_cascade();
        Some(out)
    }

    /// The App sheet at `index` of the sheets the cascade reads (a
    /// [`RuleRef::sheet`](crate::capture::RuleRef::sheet)): `None` for a
    /// `<style>` element's sheet (edited through its text) and the
    /// registrations sheet.
    pub fn stylesheet_at(&self, index: usize) -> Option<StylesheetId> {
        let style_elements = self.prelude.cascade_order(&[]).len().saturating_sub(1);
        let i = index.checked_sub(style_elements)?;
        self.stylesheets.get(i).map(|(id, _)| *id)
    }
}
