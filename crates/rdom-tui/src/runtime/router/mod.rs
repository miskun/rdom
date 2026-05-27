//! Event router — converts crossterm events into `TuiEvent`
//! dispatches against the DOM.
//!
//! The router is the only place that knows about crossterm's
//! encoding of mouse / keyboard events. It translates each into an
//! rdom-flavored `TuiEvent`, hit-tests where applicable, and walks
//! the capture → target → bubble pipeline.
//!
//! ## Usage
//!
//! `App::run` drives the router automatically. For tests, or custom
//! event loops, construct a `Router` and feed it `crossterm::Event`
//! values:
//!
//! ```ignore
//! let mut router = Router::new();
//! let outcome = router.route(&mut dom, crossterm_event);
//! if outcome.redraw_requested {
//!     // cascade + layout + paint
//! }
//! ```
//!
//! ## Sub-modules
//!
//! - [`mouse`] — `mousedown` / `mouseup` / `mousemove` / `click`
//!   (common-ancestor synthesis), plus hover transition triggers.

pub(crate) mod mouse;

use std::time::{Duration, Instant};

use crossterm::event::{Event as CtEvent, MouseEvent};
use rdom_core::NodeId;

use crate::TuiDom;

/// Max gap between successive `mousedown`s for them to count as
/// part of the same multi-click gesture. Matches the 500 ms default
/// most desktop toolkits use; terminals don't fire their own
/// double-click events once mouse capture is on, so we detect this
/// ourselves from raw `Down` events.
const MULTI_CLICK_THRESHOLD: Duration = Duration::from_millis(500);
/// How far the cursor may drift between consecutive clicks before
/// we treat them as separate gestures. 2 cells lets small hand
/// jitter slide through while still distinguishing "double-click
/// here" from "click here, click over there".
const MULTI_CLICK_TOLERANCE: u16 = 2;

#[derive(Debug, Clone, Copy)]
pub(super) struct ClickRecord {
    pub(super) at: Instant,
    pub(super) column: u16,
    pub(super) row: u16,
    /// 1 on first click, 2 on double, 3 on triple. Resets to 1 when
    /// either `MULTI_CLICK_THRESHOLD` or `MULTI_CLICK_TOLERANCE` is
    /// exceeded, or when the counter crosses 3 (a 4th click starts
    /// over).
    pub(super) count: u8,
}

/// Router state persisted between events:
/// - `down_target`: which node received the last `mousedown`; used
///   to find the common-ancestor target for `click` synthesis on
///   matching `mouseup`.
/// - `hover_target`: which node is currently under the cursor; used
///   to fire `mouseover`/`mouseout` on transitions and to drive the
///   `:hover` pseudo-class cascade.
/// - `selection_drag`: `Some(InlineFlow)` while a mouse-drag
///   text-selection is in progress. Set by
///   `runtime::selection::drag::begin` on `mousedown`, consulted by
///   `handle_move` to route through the drag extension default
///   action, cleared on `mouseup`. The stored `InlineFlow` identifies
///   the inline-flow container holding the anchor — either a classic
///   IFC block or one of a parent's anonymous block boxes (BFC-1
///   phase 3).
///
/// All fields are internal; apps touch the router only through
/// [`Router::new`] and [`Router::route`].
#[derive(Debug, Default)]
pub struct Router {
    pub(super) down_target: Option<NodeId>,
    pub(super) hover_target: Option<NodeId>,
    pub(crate) selection_drag: Option<crate::render::inline::InlineFlow>,
    /// Last observed `mousedown` — time, position, running count.
    /// Used by `register_click` to promote successive fast clicks
    /// into double / triple gestures.
    pub(super) last_click: Option<ClickRecord>,
    /// Active scrollbar-thumb drag, if any. Set by the scrollbar
    /// mouse handler on a thumb-click; consumed by subsequent
    /// mousemove events (via pointer capture) to adjust scroll.
    /// See `runtime::scrollbar`.
    pub(crate) scrollbar_drag: Option<crate::runtime::scrollbar::ScrollbarDrag>,
}

impl Router {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drive the router with a crossterm event. Routes, dispatches,
    /// updates internal state. Returns hints for the caller about
    /// whether a redraw or quit was requested.
    ///
    /// Non-mouse events (`CtEvent::Resize`, `Key`, etc.) return an
    /// empty outcome — `App` handles them directly.
    pub fn route(&mut self, dom: &mut TuiDom, event: CtEvent) -> RouteOutcome {
        if crate::runtime::trace::enabled() {
            match &event {
                CtEvent::Mouse(m) => crate::rdom_trace!(
                    "Router::route Mouse {{ kind: {:?}, col: {}, row: {}, mods: {:?} }} \
                     capture={:?} hover_target={:?} dom.hovered={:?} sel_drag={:?} scb_drag={}",
                    m.kind,
                    m.column,
                    m.row,
                    m.modifiers,
                    dom.pointer_capture(),
                    self.hover_target,
                    dom.hovered(),
                    self.selection_drag.is_some(),
                    self.scrollbar_drag.is_some(),
                ),
                CtEvent::FocusGained => {
                    crate::rdom_trace!("Router::route FocusGained")
                }
                CtEvent::FocusLost => {
                    crate::rdom_trace!("Router::route FocusLost")
                }
                CtEvent::Resize(w, h) => {
                    crate::rdom_trace!("Router::route Resize {w}x{h}")
                }
                CtEvent::Key(k) => crate::rdom_trace!("Router::route Key {k:?}"),
                CtEvent::Paste(_) => crate::rdom_trace!("Router::route Paste"),
            }
        }
        match event {
            CtEvent::Mouse(m) => mouse::route_mouse(self, dom, m),
            _ => RouteOutcome::default(),
        }
    }

    /// Reset all state. Useful between tests or after a full
    /// re-layout where hover/down targets may no longer exist.
    pub fn reset(&mut self) {
        self.down_target = None;
        self.hover_target = None;
        self.selection_drag = None;
        self.last_click = None;
        self.scrollbar_drag = None;
    }

    /// Record a `mousedown` and return the resulting click count
    /// (1 for a fresh click, 2 for double, 3 for triple, 1 again
    /// for a fourth click inside the window — which wraps the
    /// counter so apps can re-enter word/line selection fluidly).
    ///
    /// Close-enough means within [`MULTI_CLICK_THRESHOLD`] in time
    /// AND [`MULTI_CLICK_TOLERANCE`] cells in both dimensions.
    pub(super) fn register_click(&mut self, mouse: &MouseEvent) -> u8 {
        let now = Instant::now();
        let count = match self.last_click {
            Some(prev)
                if prev.count < 3
                    && now.duration_since(prev.at) <= MULTI_CLICK_THRESHOLD
                    && abs_diff_u16(mouse.column, prev.column) <= MULTI_CLICK_TOLERANCE
                    && abs_diff_u16(mouse.row, prev.row) <= MULTI_CLICK_TOLERANCE =>
            {
                prev.count + 1
            }
            _ => 1,
        };
        self.last_click = Some(ClickRecord {
            at: now,
            column: mouse.column,
            row: mouse.row,
            count,
        });
        count
    }

    /// The node currently tracked as hovered, if any. Matches
    /// `dom.hovered()` in steady state; exposed for test assertions.
    pub fn hover_target(&self) -> Option<NodeId> {
        self.hover_target
    }

    /// The node that received the most recent un-matched
    /// `mousedown`. Cleared when `mouseup` fires (whether or not
    /// click synthesis succeeded).
    pub fn down_target(&self) -> Option<NodeId> {
        self.down_target
    }
}

/// Hints returned by [`Router::route`] about what changed this
/// dispatch. The caller (usually `App`) uses them to decide
/// whether to run cascade + layout + paint and whether to exit.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RouteOutcome {
    /// Something visible changed (hover transition, focus change,
    /// scroll via wheel, etc.) — the frame should be repainted.
    pub redraw_requested: bool,
    /// A handler or default action asked the app to exit the loop.
    pub quit_requested: bool,
}

impl RouteOutcome {
    /// Merge another outcome into `self` — OR the flags. Used when
    /// a single crossterm event fans out to multiple dispatches
    /// (e.g., `mouseup` + synthesized `click`, each of which may
    /// request redraw).
    pub fn merge(&mut self, other: RouteOutcome) {
        self.redraw_requested |= other.redraw_requested;
        self.quit_requested |= other.quit_requested;
    }
}

#[inline]
fn abs_diff_u16(a: u16, b: u16) -> u16 {
    a.abs_diff(b)
}
