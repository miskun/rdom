//! Mouse activation: the root-level `click` default action. Resolves
//! the option / select under the target, then either picks or
//! toggles the option (shift-click extends in multi-select) or, for
//! a chrome click on a dropdown, toggles it open / closed. A pick on
//! a single-select dropdown auto-closes it.

use rdom_core::NodeId;

use super::dropdown::{close, is_dropdown, is_open, open};
use super::model::{closest_option, closest_select, enclosing_select, is_multi, option_disabled};
use super::state::{
    extend_selection_to, fire_input_and_change, select_single, set_highlight, toggle_option,
};
use crate::{TuiDom, TuiEventCtx};

/// Body of the root `click` listener installed by [`super::install`].
pub(super) fn on_click(ctx: &mut TuiEventCtx<'_>) {
    if ctx.event.default_prevented() {
        return;
    }
    let Some(target) = ctx.event.target else {
        return;
    };
    // Two click paths branch here:
    // 1. Target is an `<option>` (or its text child) inside
    //    a `<select>`: select / toggle, then auto-close the
    //    dropdown if it's a single-select (C.7b).
    // 2. Target is the `<select>` chrome itself or any
    //    descendant that isn't an option: toggle open /
    //    closed for dropdown-mode selects. Listbox-mode
    //    selects ignore this path (they're always "open").
    let option = closest_option(ctx.dom, target);
    let select = match option {
        Some(o) => enclosing_select(ctx.dom, o),
        None => closest_select(ctx.dom, target),
    };
    let Some(select) = select else {
        return;
    };
    if ctx.dom.node(select).has_attribute("disabled") {
        return;
    }

    // Shift-click extends in multi-select. After M4a step 8
    // the click event carries typed Mouse detail; synthetic
    // Space/Enter clicks from <button> C.3 don't (they're
    // EventDetail::Mouse with empty modifiers). For those, we
    // miss the Shift modifier — that's the same behavior as
    // before, where current_mouse() was None inside synthetic
    // clicks.
    let shift = ctx
        .event
        .detail
        .as_mouse()
        .map(|m| m.modifiers.shift)
        .unwrap_or(false);

    match option {
        Some(opt) if !option_disabled(ctx.dom, opt) => {
            click_option(ctx.dom, select, opt, shift);
            // Single-select dropdown auto-closes after a
            // pick — matches browser behavior. Listbox
            // (multi or `size`) stays "open" regardless;
            // its UA rule keeps it Auto-height.
            if is_dropdown(ctx.dom, select) {
                close(ctx.dom, select);
            }
        }
        Some(_) => {
            // Disabled option — no-op.
        }
        None => {
            // Chrome click on a dropdown-mode select →
            // toggle open. Listbox-mode selects don't
            // have chrome (full list is always visible).
            if is_dropdown(ctx.dom, select) {
                if is_open(ctx.dom, select) {
                    close(ctx.dom, select);
                } else {
                    open(ctx.dom, select);
                }
            }
        }
    }
}

fn click_option(dom: &mut TuiDom, select: NodeId, option: NodeId, shift: bool) {
    let multi = is_multi(dom, select);
    if multi {
        if shift {
            extend_selection_to(dom, select, option);
        } else {
            toggle_option(dom, select, option);
        }
        // Shift-click also moves the highlight, but the anchor
        // stays put (first shift-target defines the anchor).
        set_highlight(dom, select, Some(option));
    } else {
        select_single(dom, select, option);
        set_highlight(dom, select, Some(option));
    }
    fire_input_and_change(dom, select);
}
