//! Keyboard handling: the root-level `keydown` default action for a
//! focused `<select>` and the navigation it drives — Up / Down
//! stepping (selection in single-select, highlight in multi-select,
//! shift-extend), Home / End, Space toggle, Ctrl+A select-all, and
//! Enter / Escape dropdown open / close. Printable characters are
//! handed to [`super::typeahead`].

use rdom_core::NodeId;

use super::dropdown::{close, is_dropdown, is_open, open};
use super::model::{is_multi, options, selected_options};
use super::state::{
    anchor, extend_selection_to, fire_input_and_change, highlight, select_single, set_anchor,
    set_highlight, toggle_option,
};
use super::typeahead::{single_printable_char, typeahead_search};
use crate::{TuiDom, TuiEventCtx};

/// Body of the root `keydown` listener installed by [`super::install`].
pub(super) fn on_keydown(ctx: &mut TuiEventCtx<'_>) {
    if ctx.event.default_prevented() {
        return;
    }
    let Some(focused) = ctx.dom.focused() else {
        return;
    };
    if ctx.dom.node(focused).tag_name() != Some("select") {
        return;
    }
    if ctx.dom.node(focused).has_attribute("disabled") {
        return;
    }
    let Some(key) = ctx.event.detail.as_keyboard() else {
        return;
    };
    let select = focused;
    let multi = is_multi(ctx.dom, select);
    let shift = key.modifiers.shift;
    // Ctrl OR Meta (Cmd on macOS) — either triggers the Ctrl
    // path. The translator collapses SUPER+META into `meta`.
    let ctrl = key.modifiers.ctrl || key.modifiers.meta;
    let alt = key.modifiers.alt;
    let no_mods = !key.modifiers.ctrl && !shift && !alt && !key.modifiers.meta;

    match key.key.as_str() {
        "ArrowUp" | "ArrowDown" if !ctrl && !alt => {
            let dir = if key.key == "ArrowDown" { 1 } else { -1 };
            step_navigation(ctx.dom, select, dir, multi, shift);
        }
        "Home" => {
            jump_to_end(ctx.dom, select, true, multi, shift);
        }
        "End" => {
            jump_to_end(ctx.dom, select, false, multi, shift);
        }
        " " if multi && !ctrl => {
            toggle_highlighted(ctx.dom, select);
        }
        "a" | "A" if multi && ctrl => {
            select_all(ctx.dom, select);
        }
        // C.7b dropdown controls: Esc closes an open
        // dropdown; Enter on a closed dropdown opens
        // it, on an open one closes it. Listbox-mode
        // selects ignore both (no open/closed state).
        "Escape" if no_mods && is_dropdown(ctx.dom, select) && is_open(ctx.dom, select) => {
            close(ctx.dom, select);
        }
        "Enter" if no_mods && is_dropdown(ctx.dom, select) => {
            if is_open(ctx.dom, select) {
                close(ctx.dom, select);
            } else {
                open(ctx.dom, select);
            }
        }
        // C.7c type-ahead: pressing a printable character
        // on a focused `<select>` jumps the highlight to
        // the next option whose label starts with that
        // character (case-insensitive, wrapping). Wraps
        // around at the end; disabled options skipped.
        //
        // Skips: Ctrl/Super/Alt combos (they belong to
        // select-all / app shortcuts) and Space (reserved
        // for multi-toggle in C.7a).
        other if !ctrl && !alt && other != " " => {
            if let Some(c) = single_printable_char(other) {
                typeahead_search(ctx.dom, select, c, multi);
            }
        }
        _ => {}
    }
}

// ── Keyboard navigation ────────────────────────────────────────────

/// Move the navigation cursor by `dir` steps (±1). In single-
/// select the selection itself moves (deselect old + select new);
/// in multi-select only the highlight moves (Space toggles).
fn step_navigation(dom: &mut TuiDom, select: NodeId, dir: i32, multi: bool, shift: bool) {
    let all = options(dom, select);
    if all.is_empty() {
        return;
    }
    let current = highlight(dom, select).or_else(|| {
        // Initial highlight: first selected option, else first
        // non-disabled option.
        selected_options(dom, select).first().copied().or_else(|| {
            all.iter()
                .find(|&&o| !dom.node(o).has_attribute("disabled"))
                .copied()
        })
    });
    let next = match current {
        Some(c) => step_from(dom, &all, c, dir),
        None => return,
    };
    let Some(next) = next else { return };
    if multi {
        if shift {
            // Anchor at the OLD highlight (where we were before
            // this arrow press) on first shift-action — so shift
            // captures a range starting at the focus cell, not
            // just the new cell. Subsequent shifts preserve the
            // anchor.
            if anchor(dom, select).is_none()
                && let Some(old) = current
            {
                set_anchor(dom, select, Some(old));
            }
            set_highlight(dom, select, Some(next));
            extend_selection_to(dom, select, next);
        } else {
            // Non-shift motion clears the anchor so the next
            // shift-action starts from the new highlight.
            set_highlight(dom, select, Some(next));
            set_anchor(dom, select, None);
        }
    } else {
        set_highlight(dom, select, Some(next));
        select_single(dom, select, next);
    }
    fire_input_and_change(dom, select);
}

/// Walk `list` from `from` in direction `dir`, skipping disabled
/// options. Returns the next enabled option, or `None` when none
/// remain. No wrapping — stops at the ends.
fn step_from(dom: &TuiDom, list: &[NodeId], from: NodeId, dir: i32) -> Option<NodeId> {
    let start = list.iter().position(|&o| o == from)?;
    let mut i = start as i32;
    let len = list.len() as i32;
    loop {
        i += dir;
        if i < 0 || i >= len {
            return None;
        }
        let candidate = list[i as usize];
        if !dom.node(candidate).has_attribute("disabled") {
            return Some(candidate);
        }
    }
}

fn jump_to_end(dom: &mut TuiDom, select: NodeId, home: bool, multi: bool, shift: bool) {
    let all = options(dom, select);
    let target = if home {
        all.iter()
            .find(|&&o| !dom.node(o).has_attribute("disabled"))
            .copied()
    } else {
        all.iter()
            .rev()
            .find(|&&o| !dom.node(o).has_attribute("disabled"))
            .copied()
    };
    let Some(target) = target else { return };
    set_highlight(dom, select, Some(target));
    if multi {
        if shift {
            extend_selection_to(dom, select, target);
        } else {
            set_anchor(dom, select, None);
        }
    } else {
        select_single(dom, select, target);
    }
    fire_input_and_change(dom, select);
}

fn toggle_highlighted(dom: &mut TuiDom, select: NodeId) {
    let Some(h) = highlight(dom, select) else {
        return;
    };
    if dom.node(h).has_attribute("disabled") {
        return;
    }
    toggle_option(dom, select, h);
    fire_input_and_change(dom, select);
}

fn select_all(dom: &mut TuiDom, select: NodeId) {
    for opt in options(dom, select) {
        if !dom.node(opt).has_attribute("disabled") {
            let _ = dom.set_attribute(opt, "selected", "");
        }
    }
    fire_input_and_change(dom, select);
}
