//! Mouse-event routing — `mousedown`, `mouseup`, `mousemove`,
//! click synthesis on common ancestor, hover transitions, wheel
//! auto-scroll.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::layout::Overflow;
use crate::node::TuiNodeExt;
use crate::runtime::hit_test::HitTestExt;
use crate::style::ComputedStyle;
use crate::{TuiDispatchExt, TuiDom, TuiEvent};

use super::{RouteOutcome, Router};

/// Top-level entry for mouse events. Dispatches by kind.
pub(super) fn route_mouse(
    router: &mut Router,
    dom: &mut TuiDom,
    mouse: MouseEvent,
) -> RouteOutcome {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => handle_down(router, dom, mouse),
        MouseEventKind::Up(MouseButton::Left) => handle_up(router, dom, mouse),
        MouseEventKind::Down(MouseButton::Right) => handle_right_down(dom, mouse),
        MouseEventKind::Up(MouseButton::Right) => handle_nonleft_up(dom, mouse),
        MouseEventKind::Down(MouseButton::Middle) => handle_nonleft_down(dom, mouse),
        MouseEventKind::Up(MouseButton::Middle) => handle_nonleft_up(dom, mouse),
        MouseEventKind::Moved | MouseEventKind::Drag(MouseButton::Left) => {
            handle_move(router, dom, mouse)
        }
        MouseEventKind::ScrollUp
        | MouseEventKind::ScrollDown
        | MouseEventKind::ScrollLeft
        | MouseEventKind::ScrollRight => handle_wheel(dom, mouse),
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
fn handle_right_down(dom: &mut TuiDom, mouse: MouseEvent) -> RouteOutcome {
    let Some(target) = dom.hit_test(mouse.column, mouse.row) else {
        return RouteOutcome::default();
    };
    let mut tui_down = TuiEvent::mousedown(mouse);
    let _ = dom.dispatch_tui_event(target, &mut tui_down);

    let mut tui_ctx = TuiEvent::contextmenu(mouse);
    let _ = dom.dispatch_tui_event(target, &mut tui_ctx);
    RouteOutcome::default()
}

/// Non-left mousedown (middle button): fire `mousedown`. No
/// associated default action; no click synthesis (browsers only
/// synthesize click for the left button).
fn handle_nonleft_down(dom: &mut TuiDom, mouse: MouseEvent) -> RouteOutcome {
    let Some(target) = dom.hit_test(mouse.column, mouse.row) else {
        return RouteOutcome::default();
    };
    let mut tui = TuiEvent::mousedown(mouse);
    let _ = dom.dispatch_tui_event(target, &mut tui);
    RouteOutcome::default()
}

/// Non-left mouseup (right or middle button): fire `mouseup`.
/// No click synthesis, no pointer-capture release path
/// (capture is left-button-only in v1).
fn handle_nonleft_up(dom: &mut TuiDom, mouse: MouseEvent) -> RouteOutcome {
    let Some(target) = dom.hit_test(mouse.column, mouse.row) else {
        return RouteOutcome::default();
    };
    let mut tui = TuiEvent::mouseup(mouse);
    let _ = dom.dispatch_tui_event(target, &mut tui);
    RouteOutcome::default()
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

    let hit = dom.hit_test(mouse.column, mouse.row);
    router.down_target = hit;

    let Some(target) = hit else {
        return RouteOutcome::default();
    };

    let mut tui = TuiEvent::mousedown(mouse);
    let _ = dom.dispatch_tui_event(target, &mut tui);

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
        if let Some(sb_hit) = crate::runtime::scrollbar::hit(dom, &path, mouse.column, mouse.row)
            && crate::runtime::scrollbar::handle_mousedown(router, dom, sb_hit)
        {
            return RouteOutcome {
                redraw_requested: true,
                quit_requested: false,
            };
        }

        if let Some(focusable) = crate::runtime::focus::nearest_focusable_ancestor(dom, target) {
            let prev = dom.focused();
            crate::runtime::focus::focus_node(dom, Some(focusable));
            if prev != Some(focusable) {
                redraw = true;
            }
        }

        // Drag-select default action: if the click landed on
        // selectable text, set a caret selection at that position
        // and engage pointer capture so subsequent moves extend it.
        let drag_started = crate::runtime::selection::drag::begin(router, dom, mouse);
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

    RouteOutcome {
        redraw_requested: redraw,
        quit_requested: false,
    }
}

/// `mouseup` with the left button. Dispatches `mouseup` to the
/// hit target; if `down_target` is set, also dispatches a
/// synthesized `click` to the common ancestor of down+up targets
/// (matches HTML semantics).
///
/// **Pointer capture**: while `dom.pointer_capture()` is set, both
/// the `mouseup` and synthesized `click` route to the captured
/// element regardless of the cursor's actual position. The capture
/// is auto-released on `mouseup` (also browser-faithful).
fn handle_up(router: &mut Router, dom: &mut TuiDom, mouse: MouseEvent) -> RouteOutcome {
    let captured = dom.pointer_capture();
    let hit = dom.hit_test(mouse.column, mouse.row);
    let down_target = router.down_target.take();

    // Routing target for mouseup: captured element (if any) else hit.
    let up_target = captured.or(hit);

    if let Some(target) = up_target {
        let mut tui_up = TuiEvent::mouseup(mouse);
        let _ = dom.dispatch_tui_event(target, &mut tui_up);
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
        let _ = dom.dispatch_tui_event(target, &mut tui_click);

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
            let _ = dom.dispatch_tui_event(target, &mut tui_dbl);
        }
    }

    // Auto-release the pointer. Browser-faithful: capture ends on
    // the next mouseup unless the handler explicitly re-captures.
    if captured.is_some() {
        dom.release_pointer_capture();
    }

    // End any drag-selection in progress. Selection itself stays —
    // clicks / taps without drag leave a collapsed selection (caret)
    // at the click position, matching browser behavior.
    crate::runtime::selection::drag::end(router);
    crate::runtime::scrollbar::end_drag(router);

    RouteOutcome::default()
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
fn handle_move(router: &mut Router, dom: &mut TuiDom, mouse: MouseEvent) -> RouteOutcome {
    // Pointer capture path: route to captured, no hover updates.
    if let Some(captured) = dom.pointer_capture() {
        let mut tui = TuiEvent::mousemove(mouse);
        let _ = dom.dispatch_tui_event(captured, &mut tui);

        // Drag-select default action: extend selection focus to the
        // current cursor position. Runs alongside the mousemove
        // dispatch so a handler that wants custom drag behavior can
        // `prevent_default` on `selectstart` (Phase 6.5.5) — but not
        // on mousemove itself, since browsers don't wire it that way.
        let mut redraw = false;
        if let Some(anchor_flow) = router.selection_drag
            && crate::runtime::selection::drag::extend(dom, mouse, anchor_flow)
        {
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
        return RouteOutcome {
            redraw_requested: redraw,
            quit_requested: false,
        };
    }

    let hit = dom.hit_test(mouse.column, mouse.row);

    // Dispatch mousemove on the current hit.
    if let Some(target) = hit {
        let mut tui = TuiEvent::mousemove(mouse);
        let _ = dom.dispatch_tui_event(target, &mut tui);
    }

    // Hover transition?
    let changed = hit != router.hover_target;
    if !changed {
        return RouteOutcome::default();
    }

    let prev = router.hover_target;
    router.hover_target = hit;

    // mouseout on the previous hover target.
    if let Some(old) = prev {
        let mut tui_out = TuiEvent::mouseout(mouse);
        tui_out.event = tui_out.event.clone().with_synthetic(true);
        let _ = dom.dispatch_tui_event(old, &mut tui_out);
    }
    // mouseover on the new hover target.
    if let Some(new) = hit {
        let mut tui_over = TuiEvent::mouseover(mouse);
        tui_over.event = tui_over.event.clone().with_synthetic(true);
        let _ = dom.dispatch_tui_event(new, &mut tui_over);
    }
    // Update Dom-level hover state so cascade picks up :hover.
    dom.set_hovered(hit);

    RouteOutcome {
        redraw_requested: true,
        quit_requested: false,
    }
}

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
/// Clamping: saturating-sub prevents underflow below zero on the
/// low end. The upper bound is deferred until the layout pass
/// populates `ext.scroll_content_{width,height}` (currently left
/// unbounded — apps that want a cap can listen for `wheel` and
/// `prevent_default` past their limit).
fn handle_wheel(dom: &mut TuiDom, mouse: MouseEvent) -> RouteOutcome {
    let Some(target) = dom.hit_test(mouse.column, mouse.row) else {
        return RouteOutcome::default();
    };

    // Dispatch wheel first — a handler may cancel the default
    // scroll by calling `ctx.event.prevent_default()`.
    let mut tui = TuiEvent::wheel(mouse);
    let _ = dom.dispatch_tui_event(target, &mut tui);
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

    // Walk ancestors for the nearest scrollable container. First
    // match wins — no nested-scroll chaining in v1.
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
            .computed()
            .cloned()
            .unwrap_or_else(ComputedStyle::initial);
        let y_scrollable = matches!(computed.overflow_y, Overflow::Scroll | Overflow::Auto);
        let x_scrollable = matches!(computed.overflow_x, Overflow::Scroll | Overflow::Auto);
        if (wants_y && y_scrollable) || (wants_x && x_scrollable) {
            // Capture pre-mutation offsets so we can detect change
            // and dispatch a `scroll` event only when offsets
            // actually moved (matches HTML — at-the-bottom wheel
            // ticks are no-ops and don't fire scroll).
            //
            // Viewport size is the padding-box (CSS Overflow 3 §3
            // scrollport), not `content_layout` — the two diverge
            // under M5.5b border-collapse.
            let border = dom
                .node(id)
                .computed()
                .map(|c| c.border)
                .unwrap_or_default();
            let (old_x, old_y, new_x, new_y) = if let Some(ext) = dom.node_mut(id).ext_mut() {
                let pb = rdom_style::layout::compute_padding_box(ext.layout, border);
                let old_x = ext.scroll_x;
                let old_y = ext.scroll_y;
                if wants_y && y_scrollable {
                    let max_y = ext.scroll_content_height.saturating_sub(pb.height as usize);
                    apply_scroll(&mut ext.scroll_y, dy, max_y);
                }
                if wants_x && x_scrollable {
                    let max_x = ext.scroll_content_width.saturating_sub(pb.width as usize);
                    apply_scroll(&mut ext.scroll_x, dx, max_x);
                }
                (old_x, old_y, ext.scroll_x, ext.scroll_y)
            } else {
                (0, 0, 0, 0)
            };
            if old_x != new_x || old_y != new_y {
                // `scroll`: bubbles, NOT cancelable per HTML.
                let mut tui_scroll = TuiEvent::new("scroll");
                tui_scroll.event.cancelable = false;
                let _ = dom.dispatch_tui_event(id, &mut tui_scroll);
                return RouteOutcome {
                    redraw_requested: true,
                    quit_requested: false,
                };
            }
            return RouteOutcome::default();
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }

    // No scrollable ancestor — event bubbled but nothing scrolled.
    RouteOutcome::default()
}

/// Adjust a `usize` scroll offset by a signed `i32` delta, clamped
/// to `[0, max]`. `max` is the maximum legal scroll position —
/// `scroll_content_size - viewport_size`, saturating at 0 when
/// content fits. Matches the browser's wheel-scroll behavior: at
/// the bottom of the content, further wheel-down does nothing.
fn apply_scroll(offset: &mut usize, delta: i32, max: usize) {
    if delta > 0 {
        *offset = offset.saturating_add(delta as usize).min(max);
    } else if delta < 0 {
        *offset = offset.saturating_sub((-delta) as usize);
    }
}

#[cfg(test)]
mod tests;
