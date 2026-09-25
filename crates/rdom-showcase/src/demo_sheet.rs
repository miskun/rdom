//! The mounted demo's stylesheet.
//!
//! The App's stylesheet stack holds the shell's base sheet and the
//! mounted demo's sheet, nothing else. [`DemoSheet`] owns the second
//! slot: [`ShowcaseState::attach_sheet`](crate::ShowcaseState::attach_sheet)
//! registers the first demo's sheet directly on the `App` before the
//! first frame, and every later demo switch — which happens inside the
//! sidebar's `click` listener, where no `App` is reachable — queues the
//! swap through [`AppHandle::inject`]: the closure removes the previous
//! demo's sheet and pushes the new one with the `AppContext` stylesheet
//! intents. The App drains injections at the end of the loop iteration
//! that ran the listener (and in [`App::advance`]), before it draws, so
//! no frame paints the new demo under the old demo's sheet.

use std::sync::{Arc, Mutex};

use rdom_tui::{App, AppContext, AppHandle, Backend, StylesheetId};

use crate::DEMOS;

/// The mounted demo's slot on the App's stylesheet stack.
#[derive(Clone)]
pub struct DemoSheet {
    handle: AppHandle,
    /// The id of the mounted demo's sheet. Shared with the injected
    /// closures (which run on the loop thread, hence `Arc<Mutex<_>>`
    /// for `AppHandle::inject`'s `Send` bound).
    current: Arc<Mutex<Option<StylesheetId>>>,
}

impl DemoSheet {
    /// Register `DEMOS[demo_idx]`'s sheet on `app` now.
    pub(crate) fn attach<B: Backend>(app: &mut App<B>, demo_idx: usize) -> Self {
        let id = app.push_stylesheet(DEMOS[demo_idx].stylesheet());
        Self {
            handle: app.handle(),
            current: Arc::new(Mutex::new(Some(id))),
        }
    }

    /// Replace the mounted demo's sheet with `DEMOS[demo_idx]`'s, once
    /// the current handler returns (see the module docs).
    pub(crate) fn switch_to(&self, demo_idx: usize) {
        let current = Arc::clone(&self.current);
        self.handle.inject(move |ctx| swap(ctx, &current, demo_idx));
    }
}

fn swap(ctx: &mut AppContext<'_>, current: &Mutex<Option<StylesheetId>>, demo_idx: usize) {
    // A poisoned lock only means an earlier swap panicked mid-way; the
    // id it holds is still the registered one (or `None`).
    let mut slot = current.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(previous) = slot.take() {
        ctx.remove_stylesheet(previous);
    }
    *slot = Some(ctx.push_stylesheet(DEMOS[demo_idx].stylesheet()));
}
