//! Mouse-event routing — `mousedown`, `mouseup`, `mousemove`,
//! click synthesis on common ancestor, hover transitions, wheel
//! auto-scroll.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::runtime::hit_test::HitTestExt;
use crate::style::pseudo_pointer;
use crate::{TuiDom, TuiEvent};

use super::{RouteOutcome, Router};
use wheel::handle_wheel;

/// Dispatch `ev` to `target`, accumulating any listener-requested
/// repaint ([`EventCtx::request_redraw`](rdom_core::EventCtx::request_redraw))
/// into the router. `Router::route` folds the accumulated flag into the
/// `RouteOutcome` once per mouse event.
fn dispatch(router: &mut Router, dom: &mut TuiDom, target: crate::NodeId, ev: &mut TuiEvent) {
    crate::tui_event::dispatch_to_live(dom, target, ev);
    router.pending_redraw |= ev.event.redraw_requested();
}

/// Top-level entry for mouse events. Dispatches by kind.
pub(super) fn route_mouse(
    router: &mut Router,
    dom: &mut TuiDom,
    mouse: MouseEvent,
) -> RouteOutcome {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => handle_down(router, dom, mouse),
        MouseEventKind::Up(MouseButton::Left) => handle_up(router, dom, mouse),
        MouseEventKind::Down(MouseButton::Right) => handle_right_down(router, dom, mouse),
        MouseEventKind::Up(MouseButton::Right) => handle_nonleft_up(router, dom, mouse),
        MouseEventKind::Down(MouseButton::Middle) => handle_nonleft_down(router, dom, mouse),
        MouseEventKind::Up(MouseButton::Middle) => handle_nonleft_up(router, dom, mouse),
        MouseEventKind::Moved | MouseEventKind::Drag(MouseButton::Left) => {
            handle_move(router, dom, mouse)
        }
        MouseEventKind::ScrollUp
        | MouseEventKind::ScrollDown
        | MouseEventKind::ScrollLeft
        | MouseEventKind::ScrollRight => handle_wheel(router, dom, mouse),
        // Right/middle drag — same hit-test-and-dispatch as Moved,
        // but only when button is held. v1 routes both as plain
        // mousemove (no special drag semantics for non-left buttons).
        MouseEventKind::Drag(_) => handle_move(router, dom, mouse),
    }
}

/// Right-button mousedown: fire `mousedown` first (every button
/// fires mousedown per UI Events), then `contextmenu`. Cancelling
/// `mousedown` does NOT suppress `contextmenu` — the two are
/// independent dispatches per HTML.
fn handle_right_down(router: &mut Router, dom: &mut TuiDom, mouse: MouseEvent) -> RouteOutcome {
    let hit = dom.hit_test(mouse.column, mouse.row);
    // Popover light dismiss follows every button's press (HTML §6.12.2).
    crate::runtime::builtins::popover::light_dismiss::pointer_down(dom, hit);
    let Some(target) = hit else {
        return RouteOutcome::default();
    };
    let mut tui_down = TuiEvent::mousedown(mouse);
    dispatch(router, dom, target, &mut tui_down);

    let mut tui_ctx = TuiEvent::contextmenu(mouse);
    dispatch(router, dom, target, &mut tui_ctx);
    RouteOutcome::default()
}

/// Non-left mousedown (middle button): fire `mousedown`. No
/// associated default action; no click synthesis (browsers only
/// synthesize click for the left button).
fn handle_nonleft_down(router: &mut Router, dom: &mut TuiDom, mouse: MouseEvent) -> RouteOutcome {
    let hit = dom.hit_test(mouse.column, mouse.row);
    crate::runtime::builtins::popover::light_dismiss::pointer_down(dom, hit);
    let Some(target) = hit else {
        return RouteOutcome::default();
    };
    let mut tui = TuiEvent::mousedown(mouse);
    dispatch(router, dom, target, &mut tui);
    RouteOutcome::default()
}

/// Non-left mouseup (right or middle button): fire `mouseup`.
/// No click synthesis, no pointer-capture release path
/// (capture is left-button-only in v1).
fn handle_nonleft_up(router: &mut Router, dom: &mut TuiDom, mouse: MouseEvent) -> RouteOutcome {
    let hit = dom.hit_test(mouse.column, mouse.row);
    let dismissed = crate::runtime::builtins::popover::light_dismiss::pointer_up(dom, hit);
    let Some(target) = hit else {
        return RouteOutcome::redraw(dismissed);
    };
    let mut tui = TuiEvent::mouseup(mouse);
    dispatch(router, dom, target, &mut tui);
    RouteOutcome::redraw(dismissed)
}

/// `mousedown` with the left button. Hit-tests, remembers the
/// target as `down_target`, dispatches a bubbling `mousedown`,
/// and — unless the handler called `prevent_default` —
/// focus-on-click: walk up from hit to the nearest
/// `tabindex`-carrying ancestor and focus it (firing
/// blur/focusout + focus/focusin events).
fn handle_down(router: &mut Router, dom: &mut TuiDom, mouse: MouseEvent) -> RouteOutcome {
    // Register this click for multi-click tracking before any
    // dispatch — ensures consistent counting even if a handler
    // calls prevent_default on the mousedown.
    router.register_click(&mouse);

    // A new press ends any previous selection drag before deciding what
    // this press does. The drag normally ends on mouseup, or on the next
    // button-less move; a mouseup lost outside the window on a terminal
    // that reports no button-less motion (ITERM2-MOUSE-MOTION-1) skips
    // both, and the drag would otherwise stay armed through a scrollbar
    // press, a press on nothing, or a cancelled mousedown — extending the
    // old selection on every button-held move (P6G-DRAG-RESET-1). A press
    // that starts a new drag re-arms it in `drag::begin`.
    crate::runtime::selection::drag::end(router);
    // The same lost mouseup leaves a scrollbar-thumb drag armed, with the
    // pointer captured on the scrolled element: every button-held move
    // would keep scrolling it. The press ends that drag and drops the
    // capture it took; an author's capture is left alone
    // (P6G-PRESS-RESET-2).
    crate::runtime::scrollbar::cancel_drag(router, dom);
    crate::runtime::resize::cancel(router, dom);

    let hit = dom.hit_test(mouse.column, mouse.row);
    router.down_target = hit;
    // Popover light dismiss notes where the press began (HTML §6.12.2,
    // run as the pointer event is dispatched).
    crate::runtime::builtins::popover::light_dismiss::pointer_down(dom, hit);
    crate::rdom_trace!(
        "handle_down: hit_test({}, {}) = {:?}; pre-capture={:?}",
        mouse.column,
        mouse.row,
        hit,
        dom.pointer_capture()
    );

    let Some(target) = hit else {
        // A press on nothing activates nothing; it also ends an
        // activation whose release was lost outside the window.
        let deactivated = dom.active().is_some();
        dom.set_active(None);
        let pseudo = pseudo_pointer::set_active(dom, None);
        return RouteOutcome::redraw(deactivated || pseudo);
    };

    let mut tui = TuiEvent::mousedown(mouse);
    dispatch(router, dom, target, &mut tui);

    // Default actions run only when the handler didn't cancel them.
    // Browsers fire focus + selection-begin on mousedown, both off
    // the same event — so a single prevent_default() suppresses both.
    let mut redraw = false;
    if !tui.event.default_prevented() {
        // Scrollbar click check first — if the mousedown landed on
        // a scrollbar's thumb or track, that beats focus / selection
        // defaults (matches browser behavior where clicking a
        // scrollbar doesn't focus the container or start a text
        // selection).
        let path = dom.hit_test_path(mouse.column, mouse.row);
        // A resizable box's corner (CSS UI 4 §4.2) — where a browser
        // draws its resizer, over the scrollbar's end.
        if crate::runtime::resize::begin(router, dom, &path, mouse.column, mouse.row) {
            dom.set_active(None);
            pseudo_pointer::set_active(dom, None);
            return RouteOutcome::redraw(true);
        }
        if let Some(sb_hit) = crate::runtime::scrollbar::hit(dom, &path, mouse.column, mouse.row)
            && crate::runtime::scrollbar::handle_mousedown(router, dom, sb_hit)
        {
            dom.set_active(None);
            pseudo_pointer::set_active(dom, None);
            return RouteOutcome::redraw(true);
        }
    }

    // `:active` (Selectors 4 §9.4): the pressed element — and through
    // the selector, its ancestors — is being activated until the button
    // is released. A cancelled `mousedown` still activates, as in Blink;
    // a scrollbar press (returned above) activates nothing. A listener
    // that disconnected the target leaves nothing to activate.
    if dom.active() != Some(target) && dom.node(target).is_connected() {
        dom.set_active(Some(target));
        redraw = true;
    }
    // The pressed pseudo-element, for `::before:active` (Selectors 4
    // §3.6.3) — kept only while a sheet reads it.
    let pressed = (pseudo_pointer::is_tracked(dom) && dom.node(target).is_connected())
        .then(|| crate::runtime::hit_test::pseudo_at(dom, target, mouse.column, mouse.row))
        .flatten();
    redraw |= pseudo_pointer::set_active(dom, pressed);

    if !tui.event.default_prevented() {
        if let Some(focusable) = crate::runtime::focus::nearest_focusable_ancestor(dom, target) {
            let prev = dom.focused();
            crate::runtime::focus::focus_node_by_pointer(dom, Some(focusable));
            if prev != Some(focusable) {
                redraw = true;
            }
        }

        // Drag-select default action: if the press resolves to
        // selectable text (directly, or through the empty-space snap
        // to the nearest prose), set a caret there and record the drag
        // in router state so subsequent button-held moves extend it.
        // No pointer capture — the follow-up `mouseup` / `click` keep
        // their browser targets (P6G-SELECTION-CAPTURE-1).
        let drag_started = crate::runtime::selection::drag::begin(router, dom, mouse);
        crate::rdom_trace!(
            "handle_down: selection::drag::begin returned {drag_started}; \
             post-capture={:?} selection_drag={:?}",
            dom.pointer_capture(),
            router.selection_drag.is_some()
        );
        if drag_started {
            redraw = true;

            // Multi-click promotion: two fast clicks on the same
            // word expand to a word-select, three to a line-select.
            // `register_click` was called up top — we just consume
            // its count here. Only apply when a drag actually began
            // (matches drag-begin's user-select:none gating).
            let count = router.last_click.map(|c| c.count).unwrap_or(1);
            let changed = match count {
                2 => crate::runtime::selection::multiclick::expand_to_word(dom),
                3 => crate::runtime::selection::multiclick::expand_to_line(dom),
                _ => false,
            };
            if changed {
                redraw = true;
            }
        }
    }

    RouteOutcome::redraw(redraw)
}

/// `mouseup` with the left button. Dispatches `mouseup` to the
/// hit target; if `down_target` is set, also dispatches a
/// synthesized `click` to the common ancestor of down+up targets
/// (matches HTML semantics).
///
/// **Pointer capture**: while `dom.pointer_capture()` is set (a
/// consumer's `set_pointer_capture`, or a scrollbar-thumb drag), both
/// the `mouseup` and synthesized `click` route to the captured
/// element regardless of the cursor's actual position. The capture
/// is auto-released on `mouseup` (also browser-faithful). A
/// text-selection drag takes no capture, so after one the `mouseup`
/// targets the hit and the `click` the common ancestor, as in a
/// browser.
fn handle_up(router: &mut Router, dom: &mut TuiDom, mouse: MouseEvent) -> RouteOutcome {
    // The release ends the activation (`:active`, Selectors 4 §9.4)
    // before any of its own events run, as in Blink:
    // `EventHandler::HandleMouseReleaseEvent` performs the `kRelease` hit
    // test, whose `Document::UpdateHoverActiveState(false, …)` clears the
    // whole `:active` chain, before it dispatches `pointerup` / `mouseup`
    // and then `click` (`P7G-ACTIVE-CLEARS-ON-RELEASE-1`).
    let deactivated = dom.active().is_some();
    dom.set_active(None);
    let deactivated = pseudo_pointer::set_active(dom, None) || deactivated;
    let captured = dom.pointer_capture();
    let hit = dom.hit_test(mouse.column, mouse.row);
    let down_target = router.down_target.take();
    crate::rdom_trace!(
        "handle_up: hit={hit:?} down_target={down_target:?} captured={captured:?} \
         selection_drag={} scrollbar_drag={}",
        router.selection_drag.is_some(),
        router.scrollbar_drag.is_some()
    );

    // Routing target for mouseup: captured element (if any) else hit.
    let up_target = captured.or(hit);
    // A press and release in the same popover stack — or both outside
    // every popover — hide the popovers above it (HTML §6.12.2).
    let dismissed = crate::runtime::builtins::popover::light_dismiss::pointer_up(dom, up_target);

    if let Some(target) = up_target {
        let mut tui_up = TuiEvent::mouseup(mouse);
        dispatch(router, dom, target, &mut tui_up);
    }

    // Click synthesis.
    //   With capture: click fires on the captured element (regardless
    //     of where the cursor is). The common-ancestor dance is moot —
    //     capture says "all follow-up events are mine."
    //   Without capture: click fires on the common ancestor of
    //     mousedown's target and mouseup's hit target (HTML spec).
    //     Requires both down_target and hit to exist.
    let click_target = if let Some(cap) = captured {
        Some(cap)
    } else if let (Some(down), Some(up)) = (down_target, hit) {
        dom.common_ancestor(down, up)
    } else {
        None
    };
    if let Some(target) = click_target {
        let mut tui_click = TuiEvent::click(mouse);
        tui_click.event = tui_click.event.clone().with_synthetic(true);
        dispatch(router, dom, target, &mut tui_click);

        // Multi-click promotion: fire `dblclick` on the second
        // click of a sequence, AFTER the regular click event.
        // `register_click` recorded the running count on the
        // matching mousedown; we consume it here. Only the 2nd
        // click promotes — a 3rd click in the same sequence is a
        // triple-click gesture (selection extends to a line); a
        // 4th wraps the counter back to 1 and starts a new pair,
        // so dblclick will fire again on a fresh second click.
        // Synthetic per UI Events §5.10.
        let count = router.last_click.map(|c| c.count).unwrap_or(0);
        if count == 2 {
            let mut tui_dbl = TuiEvent::dblclick(mouse);
            tui_dbl.event = tui_dbl.event.clone().with_synthetic(true);
            dispatch(router, dom, target, &mut tui_dbl);
        }
    }

    // Auto-release the pointer. Browser-faithful: capture ends on
    // the next mouseup unless the handler explicitly re-captures.
    if captured.is_some() {
        crate::rdom_trace!("handle_up: auto-releasing pointer_capture (was {captured:?})");
        dom.release_pointer_capture();
    }

    // End any drag-selection in progress. Selection itself stays —
    // clicks / taps without drag leave a collapsed selection (caret)
    // at the click position, matching browser behavior.
    crate::runtime::selection::drag::end(router);
    crate::runtime::scrollbar::end_drag(router, dom);
    crate::runtime::resize::end(router);
    crate::rdom_trace!(
        "handle_up: end of fn — capture={:?} hovered={:?}",
        dom.pointer_capture(),
        dom.hovered()
    );

    RouteOutcome::redraw(deactivated || dismissed)
}

/// `mousemove` (or drag with left button held). Hit-tests; if
/// the hit differs from `hover_target`, fires `mouseout` on the
/// old target, `mouseover` on the new, and updates
/// `dom.set_hovered` so the `:hover` pseudo-class cascade picks
/// up the change on the next cascade pass.
///
/// **Pointer capture**: while `dom.pointer_capture()` is set, the
/// `mousemove` routes to the captured element regardless of the
/// cursor's position. Hover transitions are also suppressed —
/// `:hover` stays on whatever it was; the browser treats the
/// captured element as the implicit hover target. This matches
/// what drag-interaction apps (slider scrubbing, resize handles,
/// rubber-band selection) need: the captured handler sees every
/// move, and the rest of the UI doesn't flicker its hover state.
///
/// **Text-selection drag**: not a capture. The `mousemove` targets
/// the hit and hover updates as usual; the selection extends to the
/// pointer on every button-held move (`router.selection_drag`).
fn handle_move(router: &mut Router, dom: &mut TuiDom, mouse: MouseEvent) -> RouteOutcome {
    // Stale pointer-capture release: a `Moved` event (button NOT
    // held) arriving while `pointer_capture` is set means the
    // mouseup that would have released it never reached us — the
    // user released the button outside the terminal window, or the
    // terminal lost focus mid-drag and the mouseup was delivered to
    // a different window. crossterm doesn't surface focus events
    // unless `EnableFocusChange` is opted in, so we can't watch for
    // FocusLost to clean up; instead we treat the next button-less
    // motion as evidence the drag is over.
    //
    // Browser-faithful: the W3C Pointer Events spec fires
    // `pointercancel` when the implementation determines a pointer
    // is unlikely to produce any more events (window blur during
    // drag is the canonical case). `pointercancel` releases capture
    // and ends any in-progress drag — exactly what we do here, just
    // detected via the absence of a button on the follow-up motion.
    //
    // Without this fix, hover updates were dead until the user
    // clicked an element to force the mouseup auto-release path
    // (`handle_up` → `release_pointer_capture`).
    // The same evidence ends a text-selection drag, which lives in
    // router state rather than in capture.
    let drag_in_progress = dom.pointer_capture().is_some() || router.selection_drag.is_some();
    if drag_in_progress && matches!(mouse.kind, MouseEventKind::Moved) {
        crate::rdom_trace!(
            "handle_move: stale drag detected (Moved with capture={:?}, selection_drag={}) — ending",
            dom.pointer_capture(),
            router.selection_drag.is_some()
        );
        dom.release_pointer_capture();
        crate::runtime::selection::drag::end(router);
        crate::runtime::scrollbar::end_drag(router, dom);
        crate::runtime::resize::end(router);
    }

    // Pointer capture path: route to captured, no hover updates.
    if let Some(captured) = dom.pointer_capture() {
        crate::rdom_trace!("handle_move: capture branch — routing to {captured:?}, hover skipped");
        let mut tui = TuiEvent::mousemove(mouse);
        dispatch(router, dom, captured, &mut tui);

        // Drag-select default action, for a consumer that captured the
        // pointer on mousedown without cancelling the selection default:
        // the selection still follows the pointer.
        let mut redraw = false;
        if router.selection_drag.is_some() && crate::runtime::selection::drag::extend(dom, mouse) {
            redraw = true;
        }
        // Scrollbar drag default action: if a thumb drag is active,
        // translate the cursor delta along the track into a scroll
        // offset change on the captured scrollbar owner. Runs during
        // the pointer-capture path because mousedown on a thumb
        // engages capture on the scrollbar element.
        if router.scrollbar_drag.is_some()
            && crate::runtime::scrollbar::extend_drag(router, dom, mouse.column, mouse.row)
        {
            redraw = true;
        }
        // Corner drag of a resizable box (CSS UI 4 §4.2).
        if router.resize_drag.is_some()
            && crate::runtime::resize::extend(router, dom, mouse.column, mouse.row)
        {
            redraw = true;
        }
        return RouteOutcome::redraw(redraw);
    }

    let hit = dom.hit_test(mouse.column, mouse.row);
    if crate::runtime::trace::enabled() {
        // Resolve tag + class for human-readable trace.
        let info = hit
            .map(|id| {
                let n = dom.node(id);
                let tag = n.tag_name().unwrap_or("?");
                let cls = n.get_attribute("class").unwrap_or("");
                format!("{id:?} <{tag} class=\"{cls}\">")
            })
            .unwrap_or_else(|| "None".to_string());
        crate::rdom_trace!(
            "handle_move: hit_test({}, {}) = {info}  (prev hover_target={:?})",
            mouse.column,
            mouse.row,
            router.hover_target
        );
    }

    // Dispatch mousemove on the current hit.
    if let Some(target) = hit {
        let mut tui = TuiEvent::mousemove(mouse);
        dispatch(router, dom, target, &mut tui);
    }

    // Drag-select default action: extend the selection focus to the
    // pointer, whatever element is under it. The drag is router state,
    // not pointer capture — the `mousemove` above targeted the hit, as
    // a browser's does during a text-selection drag.
    let extended =
        router.selection_drag.is_some() && crate::runtime::selection::drag::extend(dom, mouse);

    // The pseudo-element under the pointer, for `::before:hover`
    // (Selectors 4 §3.6.3) — kept only while a sheet reads it.
    let pseudo_moved = pseudo_pointer::is_tracked(dom) && {
        let over =
            hit.and_then(|h| crate::runtime::hit_test::pseudo_at(dom, h, mouse.column, mouse.row));
        pseudo_pointer::set_hovered(dom, over)
    };

    // Hover transition?
    let changed = hit != router.hover_target;
    if !changed {
        crate::rdom_trace!(
            "handle_move: no hover change ({:?} == hover_target); extended={extended}",
            hit
        );
        return RouteOutcome::redraw(extended || pseudo_moved);
    }

    let prev = router.hover_target;
    router.hover_target = hit;
    crate::rdom_trace!(
        "handle_move: hover transition {prev:?} -> {hit:?}; calling set_hovered + redraw"
    );

    // mouseout on the previous hover target.
    if let Some(old) = prev {
        let mut tui_out = TuiEvent::mouseout(mouse);
        tui_out.event = tui_out.event.clone().with_synthetic(true);
        dispatch(router, dom, old, &mut tui_out);
    }
    // mouseover on the new hover target.
    if let Some(new) = hit {
        let mut tui_over = TuiEvent::mouseover(mouse);
        tui_over.event = tui_over.event.clone().with_synthetic(true);
        dispatch(router, dom, new, &mut tui_over);
    }
    // Update Dom-level hover state so cascade picks up :hover.
    dom.set_hovered(hit);

    RouteOutcome::redraw(true)
}

mod wheel;

#[cfg(test)]
mod overscroll_tests;
#[cfg(test)]
mod tests;
