//! The `App`'s construction-time options: consuming `with_*` builders
//! chained after [`App::new`] or [`App::with_backend`] (the table of every
//! option and its defaults is in the [`app`](super) module doc,
//! C12G-APP-CONFIG).

use std::rc::Rc;
use std::time::Duration;

use super::{App, AppContext, ControlFlow};
use crate::render::backend::Backend;
use crate::runtime::selection::clipboard::Clipboard;
use crate::runtime::url_opener::UrlOpener;

impl<B: Backend> App<B> {
    /// The frame rate while timers, `requestAnimationFrame`,
    /// transitions or animations run: `fps` frames a second, clamped to
    /// 1–120. Default 60 (a 16 ms frame budget). Pass `30` on slower
    /// terminals or to reduce CPU.
    pub fn with_animation_frame_rate(mut self, fps: u16) -> Self {
        let fps = fps.clamp(1, 120);
        self.animation_frame_ms = (1000 / fps as u32).max(1);
        self
    }

    /// Set the caret blink half-period: the caret of a focused editable
    /// is painted for `period`, then hidden for `period`, restarting
    /// visible on every caret move, edit, key or click. `None` (or a zero
    /// duration) keeps a steady caret. Browsers follow the OS setting and
    /// no CSS property controls it, so this is an App option.
    /// [`App::new`] blinks at
    /// [`DEFAULT_CARET_BLINK`](crate::runtime::caret_blink::DEFAULT_CARET_BLINK)
    /// (530 ms); [`App::with_backend`] starts steady, so paint snapshots
    /// do not depend on the clock. Blinking wakes the event loop only
    /// while an editable with a caret is focused and the terminal has
    /// focus.
    pub fn with_caret_blink(mut self, period: Option<Duration>) -> Self {
        self.prelude.caret_blink.set_period(period);
        self
    }

    /// Emit the SGR extensions `caps` lists — the underline styles and
    /// color, the overline (CSS Text Decoration 4) — overriding what the
    /// backend had: [`App::new`]'s guess from the environment
    /// ([`SgrCapabilities::from_env`](crate::SgrCapabilities::from_env)),
    /// a [`TestBackend`](crate::TestBackend)'s
    /// [`BASIC`](crate::SgrCapabilities::BASIC). Force `BASIC` for output
    /// that is logged or recorded, or give a terminal the detection does
    /// not know its extensions. The next frame is drawn whole.
    pub fn with_sgr_capabilities(mut self, caps: crate::SgrCapabilities) -> Self {
        self.terminal.backend_mut().set_sgr_capabilities(caps);
        self.terminal.queue_full_redraw();
        self
    }

    /// The SGR extensions the app's backend emits.
    pub fn sgr_capabilities(&self) -> crate::SgrCapabilities {
        self.terminal.backend().sgr_capabilities()
    }

    /// Replace the clipboard backend. Useful for tests
    /// (`MemoryClipboard`) and for apps that want custom format
    /// or transport on `copy`.
    pub fn with_clipboard(mut self, clipboard: Box<dyn Clipboard>) -> Self {
        self.clipboard = clipboard;
        self
    }

    /// Replace the URL opener backend used by the `<a href>`
    /// click default action. Useful for tests (`MemoryUrlOpener`
    /// to avoid launching browsers) and for apps that want custom
    /// external-URL handling (e.g. an in-app preview for
    /// `https://` instead of shelling out).
    ///
    /// The App's `<a href>` listener is installed by the constructor and
    /// reads the opener through a shared cell, so the swap takes effect
    /// whenever this is called.
    pub fn with_url_opener(self, opener: Rc<dyn UrlOpener>) -> Self {
        *self.url_opener.borrow_mut() = opener;
        self
    }

    /// The crossterm event-poll timeout / tick cadence. Default 50 ms.
    /// `Duration::ZERO` disables tick firing entirely (the loop blocks
    /// until a real event arrives).
    pub fn with_tick_rate(mut self, d: Duration) -> Self {
        self.tick_rate = d;
        self
    }

    /// The tick handler: a callback that fires each iteration where no
    /// crossterm event arrived within the tick rate
    /// ([`with_tick_rate`](Self::with_tick_rate)). Primarily for
    /// draining app-level channels (watch streams, timers, inter-thread
    /// signals) into DOM mutations. Replaces an earlier one.
    ///
    /// Return `ControlFlow::Quit` to exit the loop.
    pub fn with_tick_handler<F>(mut self, f: F) -> Self
    where
        F: FnMut(&mut AppContext<'_>) -> ControlFlow + 'static,
    {
        self.on_tick = Some(Box::new(f));
        self
    }
}
