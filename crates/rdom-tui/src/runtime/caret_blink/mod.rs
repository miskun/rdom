//! Caret blink (`P7-CARET-BLINK-1`).
//!
//! Browsers blink the text caret — Chromium and Firefox follow the OS
//! setting, about 530 ms on and 530 ms off — restart it visible on every
//! caret move and every edit, and paint no caret at all while the window
//! is inactive. No CSS property controls blinking (`caret-color` /
//! `caret-shape` do not), so the rate is an [`App`](crate::runtime::App) option:
//! [`App::with_caret_blink`](crate::runtime::App::with_caret_blink).
//! [`App::new`](crate::runtime::App::new) blinks at
//! [`DEFAULT_CARET_BLINK`]; [`App::with_backend`](crate::runtime::App::with_backend)
//! (tests, custom backends) keeps a steady caret unless enabled.
//!
//! ## Design
//!
//! rdom paints its own caret (a cell in `caret-color`, see
//! `render::paint_pass::inline_paint::caret`); the hardware cursor stays
//! hidden. The blink is a phase of that painted caret, owned by the
//! runtime so paint stays a pure function of the tree: the controller
//! computes the phase from the scheduler clock (virtual under
//! `App::advance`, so tests are deterministic) and records it in the
//! caret host's [`TuiExt::caret_blink_off`](crate::TuiExt), which the
//! caret painter reads. It is not a scheduler timer — nothing user-visible
//! is queued, and there is no deadline at all unless a caret is showing:
//! with no focused editable, a range selection, or an unfocused terminal
//! the loop is never woken for the caret.
//!
//! The phase restarts (caret visible) when the caret host or the
//! selection changes (`Dom::selection_serial`) and on every key, paste
//! or mouse-button event — the edits that leave the caret where it was
//! (forward Delete) come in through those.

use std::time::{Duration, Instant};

use rdom_core::NodeId;

use crate::TuiDom;

/// The caret blink half-period [`App::new`](crate::runtime::App::new)
/// uses: the caret is on for this long, then off for this long. The
/// common OS default (GTK `cursor-blink-time` 1200 ms full cycle, Windows
/// `GetCaretBlinkTime` 530 ms).
pub const DEFAULT_CARET_BLINK: Duration = Duration::from_millis(530);

/// The caret the controller last painted.
#[derive(Debug, Clone, Copy)]
struct Shown {
    /// The editing host whose `caret_blink_off` this controller owns.
    host: NodeId,
    /// `Dom::selection_serial` when the phase last restarted.
    serial: rdom_core::SelectionSerial,
    /// When the current on-phase began.
    epoch: Instant,
    /// Whether the caret is currently painted off.
    off: bool,
}

/// Runtime-owned blink state. One per [`App`](crate::runtime::App).
#[derive(Debug)]
pub(crate) struct CaretBlink {
    period: Option<Duration>,
    terminal_focused: bool,
    restart: bool,
    shown: Option<Shown>,
    next: Option<Instant>,
}

impl CaretBlink {
    pub(crate) fn new(period: Option<Duration>) -> Self {
        Self {
            period: period.filter(|p| !p.is_zero()),
            terminal_focused: true,
            restart: false,
            shown: None,
            next: None,
        }
    }

    pub(crate) fn set_period(&mut self, period: Option<Duration>) {
        self.period = period.filter(|p| !p.is_zero());
        self.restart = true;
    }

    /// A key, paste or mouse-button event: restart the phase visible at
    /// the next [`update`](Self::update).
    pub(crate) fn note_input(&mut self) {
        self.restart = true;
    }

    /// The terminal window gained or lost focus.
    pub(crate) fn set_terminal_focused(&mut self, focused: bool) {
        if self.terminal_focused != focused {
            self.terminal_focused = focused;
            self.restart = true;
        }
    }

    /// When the phase next flips; `None` when nothing blinks.
    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        self.next
    }

    /// Bring the phase up to `now` and record it on the caret host.
    /// Returns `true` when the painted caret changed and the frame must
    /// be redrawn.
    pub(crate) fn update(&mut self, dom: &mut TuiDom, now: Instant) -> bool {
        let host = caret_host(dom);
        let serial = dom.selection_serial();
        let prev = self.shown.take();
        let mut changed = false;

        // Release a host that no longer shows the caret.
        if let Some(p) = prev
            && Some(p.host) != host
            && p.off
        {
            changed |= set_off(dom, p.host, false);
        }

        self.next = None;
        if let Some(host) = host {
            let restart = self.restart || prev.is_none_or(|p| p.host != host || p.serial != serial);
            let epoch = match prev {
                Some(p) if !restart => p.epoch,
                _ => now,
            };
            let off = if !self.terminal_focused {
                true
            } else if let Some(period) = self.period {
                let phases = now.saturating_duration_since(epoch).as_nanos() / period.as_nanos();
                // Phase k runs from epoch + k·period; the next flip is at
                // epoch + (k+1)·period.
                self.next = u32::try_from(phases + 1)
                    .ok()
                    .and_then(|k| period.checked_mul(k))
                    .map(|d| epoch + d);
                phases % 2 == 1
            } else {
                false
            };
            changed |= set_off(dom, host, off);
            self.shown = Some(Shown {
                host,
                serial,
                epoch,
                off,
            });
        }
        self.restart = false;
        changed
    }
}

/// The editing host whose caret paint would draw: the focused element's
/// editable ancestor, with a collapsed selection. Mirrors the caret
/// painter's own conditions.
fn caret_host(dom: &TuiDom) -> Option<NodeId> {
    let sel = dom.selection()?;
    if !sel.is_collapsed() {
        return None;
    }
    let focused = dom.focused()?;
    crate::node::nearest_editable_ancestor(dom, focused)
}

/// Write `off` to `host`'s flag; `true` when it changed. A host that left
/// the arena has nothing to repaint.
fn set_off(dom: &mut TuiDom, host: NodeId, off: bool) -> bool {
    if !dom.contains(host) {
        return false;
    }
    match dom.node_mut(host).ext_mut() {
        Some(ext) if ext.caret_blink_off != off => {
            ext.caret_blink_off = off;
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests;
