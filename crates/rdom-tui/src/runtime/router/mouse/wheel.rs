//! The wheel: a cancelable `wheel` event on the hit target, then the
//! scroll of the nearest scroll container that can move on the wheel's
//! axis, chaining outward (CSS Overscroll Behavior 1 §3) and snapping
//! (CSS Scroll Snap 1 §6.2).

use crossterm::event::{MouseEvent, MouseEventKind};

use super::dispatch;
use crate::layout::Overflow;
use crate::node::TuiNodeExt;
use crate::runtime::hit_test::HitTestExt;
use crate::style::ComputedStyle;
use crate::{TuiDom, TuiEvent};

use super::super::{RouteOutcome, Router};

/// Wheel (scroll) event. Dispatches a cancelable `wheel` event on
/// the hit target, then — unless `prevent_default` was called —
/// walks ancestors for the nearest `overflow: Scroll | Auto`
/// container and adjusts its scroll offset.
///
/// Scroll amount: **1 cell per wheel tick**. Some terminals send
/// a wheel tick per line, some per "notch"; terminal emulator
/// conventions vary. 1 cell is the most predictable default;
/// higher rates can come from `App::wheel_scroll_lines(n)` in a
/// later iteration.
///
/// Clamping: to the scroll range (`layout_pass::scroll_bounds`).
pub(super) fn handle_wheel(
    router: &mut Router,
    dom: &mut TuiDom,
    mouse: MouseEvent,
) -> RouteOutcome {
    let Some(target) = dom.hit_test(mouse.column, mouse.row) else {
        return RouteOutcome::default();
    };

    // Dispatch wheel first — a handler may cancel the default
    // scroll by calling `ctx.event.prevent_default()`.
    let mut tui = TuiEvent::wheel(mouse);
    dispatch(router, dom, target, &mut tui);
    if tui.event.default_prevented() {
        return RouteOutcome::default();
    }

    // Translate kind → (dx, dy) in cells. Positive dy = scroll
    // forward (user sees content that was below the fold).
    let (dx, dy): (i32, i32) = match mouse.kind {
        MouseEventKind::ScrollUp => (0, -1),
        MouseEventKind::ScrollDown => (0, 1),
        MouseEventKind::ScrollLeft => (-1, 0),
        MouseEventKind::ScrollRight => (1, 0),
        // Unreachable given route_mouse's match; match-exhaustive
        // compiler insists.
        _ => return RouteOutcome::default(),
    };

    // Walk ancestors for the nearest scrollable container that can
    // still move in the wheel's direction. One already at its rail
    // end is skipped and the tick chains to the next scrollable
    // ancestor (CSS Overscroll Behavior 1 §3, `auto`) — or stops there
    // (`contain` / `none`).
    // Which axis is this wheel event moving? crossterm emits
    // wheel events with a single axis set (either (0, ±1) or
    // (±1, 0)), so a scrollable ancestor must match the
    // relevant axis's overflow.
    let wants_y = dy != 0;
    let wants_x = dx != 0;

    let mut cur = Some(target);
    while let Some(id) = cur {
        let computed = dom
            .node(id)
            .computed_rc()
            .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
        if !computed.is_scroll_container() {
            cur = dom.node(id).parent_node().map(|p| p.id());
            continue;
        }
        let y_scrollable = matches!(computed.overflow_y, Overflow::Scroll | Overflow::Auto);
        let x_scrollable = matches!(computed.overflow_x, Overflow::Scroll | Overflow::Auto);
        // A box the user can scroll on the wheel's axis (`hidden` scrolls
        // only from code) takes the tick unless it is at its boundary in
        // the tick's direction.
        if (wants_y && y_scrollable) || (wants_x && x_scrollable) {
            // The legal range is `layout_pass::scroll_bounds` (the
            // scrollport against the scrollable overflow) — an `rtl` box's
            // `scrollLeft` and a `column-reverse` box's `scrollTop` run
            // negative, so the wheel reaches that overflow (CSSOM View §4).
            let Some(bounds) = crate::runtime::scrollbar::scroll_bounds(dom, id) else {
                cur = dom.node(id).parent_node().map(|p| p.id());
                continue;
            };
            let (old_x, old_y) = dom
                .node(id)
                .ext()
                .map_or((0, 0), |e| (e.scroll_x, e.scroll_y));
            let mut to = (old_x, old_y);
            if wants_y && y_scrollable {
                to.1 = (old_y + dy).clamp(bounds.min_y, bounds.max_y);
            }
            if wants_x && x_scrollable {
                to.0 = (old_x + dx).clamp(bounds.min_x, bounds.max_x);
            }
            // At the boundary in the tick's direction: nothing to take.
            let at_boundary = to == (old_x, old_y);
            // A snap container rests at the snap position in the wheel's
            // direction (CSS Scroll Snap 1 §6.2).
            let (new_x, new_y) = crate::runtime::scroll_snap::snap(
                dom,
                id,
                to,
                crate::runtime::scroll_snap::Motion::By {
                    from: (old_x, old_y),
                },
            );
            if (new_x, new_y) != (old_x, old_y) {
                // A user scroll is instant whatever `scroll-behavior`
                // says, and aborts this box's smooth scroll in flight
                // (CSSOM View "perform a scroll", step 1). The offsets
                // are the snap's: the funnel fires `scroll` — only when
                // they moved (HTML: a tick at the end fires none).
                crate::runtime::smooth_scroll::abort(dom, id);
                crate::runtime::scrollbar::write_offsets(
                    dom,
                    id,
                    new_x,
                    new_y,
                    crate::runtime::scrollbar::WriteKind::Snap,
                );
                return RouteOutcome::redraw(true);
            }
            // Not at its boundary, a mandatory snap held it where it is:
            // the box took the tick, and nothing chains (CSS Overscroll
            // Behavior 1 §3 chains from a box at its boundary).
            if !at_boundary {
                return RouteOutcome::default();
            }
        }
        // At its boundary, or not user-scrollable on the axis: chain to
        // the next scroll container — unless this one's
        // `overscroll-behavior` on the wheel's axis is `contain` or
        // `none` (§3: "no scroll chaining occurs to neighboring scrolling
        // areas"), which applies to every scroll container "regardless of
        // whether those elements currently have overflowing content or
        // are user scrollable" — an `overflow: hidden` backdrop too, as
        // Chromium does since 144.
        let behavior = if wants_y {
            computed.overscroll_behavior_y
        } else {
            computed.overscroll_behavior_x
        };
        if !behavior.chains() {
            return RouteOutcome::default();
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }

    // No scrollable ancestor — event bubbled but nothing scrolled.
    RouteOutcome::default()
}
