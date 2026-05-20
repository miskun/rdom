//! `AppHandle` — cross-thread / cross-task handle to the running
//! `App`.
//!
//! `App::run` is sync and single-threaded. Apps that need to push
//! data into the event loop from elsewhere (watch streams,
//! tokio tasks, timer threads, IPC) clone an `AppHandle` and call
//! [`request_redraw`], [`quit`], or [`inject`] from the other
//! thread. The main loop picks up the changes on the next
//! iteration.
//!
//! ## Wake semantics
//!
//! Crossterm's `event::poll` doesn't support external wake-ups, so
//! a background-thread `request_redraw` takes effect on the next
//! `tick_rate` timeout (default 50 ms). Apps that need faster
//! response can set a tighter `tick_rate`. A dedicated wake-fd
//! channel may arrive in a future phase.
//!
//! [`request_redraw`]: AppHandle::request_redraw
//! [`quit`]: AppHandle::quit
//! [`inject`]: AppHandle::inject

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::AppContext;

/// Closures injected into the loop must be `Send + 'static`:
/// they're built on one thread and run on the main thread at a
/// later point.
type Injection = Box<dyn FnOnce(&mut AppContext<'_>) + Send + 'static>;

/// Shared state behind an `AppHandle`. Stored in an `Arc` so
/// handle clones all see the same flags.
#[derive(Default)]
pub(super) struct AppShared {
    pub(super) redraw_requested: AtomicBool,
    pub(super) quit_requested: AtomicBool,
    pub(super) inject_queue: Mutex<Vec<Injection>>,
}

impl AppShared {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Drain the inject queue, returning the pending closures. The
    /// caller runs each against an `AppContext` borrowed against
    /// the DOM.
    pub(super) fn drain_injections(&self) -> Vec<Injection> {
        let mut q = self.inject_queue.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *q)
    }
}

/// Clone-able, `Send + Sync` handle to poke the running `App`
/// from a background thread / async task.
#[derive(Clone)]
pub struct AppHandle {
    pub(super) shared: Arc<AppShared>,
}

impl AppHandle {
    pub(super) fn from_shared(shared: Arc<AppShared>) -> Self {
        Self { shared }
    }

    /// Request the runtime paint on its next iteration. Thread-
    /// safe; the flag is checked via an atomic load.
    pub fn request_redraw(&self) {
        self.shared.redraw_requested.store(true, Ordering::Relaxed);
    }

    /// Ask the runtime to exit the loop. Thread-safe.
    pub fn quit(&self) {
        self.shared.quit_requested.store(true, Ordering::Relaxed);
    }

    /// Run `f` on the loop thread at the next iteration. The
    /// closure receives a fresh `AppContext` scoped to the loop's
    /// exclusive DOM borrow — so it can mutate the tree, request
    /// redraws, or call `quit()`.
    ///
    /// Useful for watch-stream bridges: the stream task collects
    /// data on its own thread, then `inject`s a DOM-mutating
    /// closure that the main loop runs synchronously.
    pub fn inject<F>(&self, f: F)
    where
        F: FnOnce(&mut AppContext<'_>) + Send + 'static,
    {
        let mut q = self
            .shared
            .inject_queue
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        q.push(Box::new(f));
    }
}

// Explicit assertion that AppHandle really is Send + Sync. If this
// ever breaks (e.g., a non-Send field creeps in), compile fails here
// instead of at a user's `spawn()` call.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<AppHandle>();
    assert_sync::<AppHandle>();
};
