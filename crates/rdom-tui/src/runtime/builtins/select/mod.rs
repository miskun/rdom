//! `<select>` + `<option>` + `<optgroup>` — listbox and (in C.7b)
//! dropdown selection widget.
//!
//! ## Contract (from MDN)
//!
//! - `<select>` with `multiple` or `size > 1` renders as a listbox
//!   (always-visible row list). `<select>` with no `multiple` and
//!   `size <= 1` is a closed dropdown — C.7a ships the listbox
//!   path, C.7b layers the dropdown open/close behavior on top.
//! - `<option selected>` marks a selected option. Presence-only
//!   (any value counts), matching HTML's boolean-attribute
//!   semantics and the `:checked` pattern from C.4b.
//! - `<option disabled>` is skipped in keyboard navigation and
//!   cannot be selected by user action.
//! - `<optgroup label="Group">` renders the label as a bold,
//!   non-selectable separator line.
//! - `<select disabled>` blocks all interaction.
//!
//! ## State model
//!
//! - **Selection**: `<option selected>` attribute presence is
//!   the single source of truth (matches C.4b checkbox/radio).
//!   Form `collect()` reads the `selected`-marked options.
//! - **Highlight (multi-select only)**: `data-rdom-highlight`
//!   on the currently-focused option — moves with Up/Down,
//!   toggles selection on Space. Single-select selection
//!   follows the highlight directly (no separate tracking).
//! - **Anchor (multi-select only)**: `data-rdom-anchor` marks
//!   where shift-extend selection started. Shift+Up/Down
//!   selects the range from anchor to current highlight.
//!
//! ## Interaction summary
//!
//! | Key | Single | Multi |
//! |---|---|---|
//! | Up/Down | Move selection | Move highlight |
//! | Home/End | First / last | First / last (highlight) |
//! | Space / Enter | (already selected) | Toggle highlight |
//! | Shift+Up/Down | — | Extend selection to next |
//! | Ctrl+A | — | Select all |
//!
//! Click:
//! - Single: select that option (deselect siblings).
//! - Multi: toggle that option; Shift+click extends from anchor.
//!
//! ## v1 deliberate simplifications
//!
//! - No dropdown open/close (C.7b).
//! - No type-ahead search (C.7c).
//! - No auto-scroll to keep highlight visible when overflow —
//!   apps compose with `overflow-y: auto` via author CSS; polish
//!   item to make it automatic.

use rdom_core::{ListenerOptions, NodeId};

use crate::tui_event::TuiDispatchExt;
use crate::{TuiDom, TuiEvent};

/// Marker: the currently-focused option within a multi-select.
const HIGHLIGHT_ATTR: &str = "data-rdom-highlight";
/// Marker: anchor for shift-extend range selection (multi-select).
const ANCHOR_ATTR: &str = "data-rdom-anchor";
/// Marker: dropdown is open (options expanded below chrome). Only
/// meaningful for single-select dropdowns — listboxes (`multiple`
/// or `size`) ignore this attribute and are always "open."
const OPEN_ATTR: &str = "data-rdom-open";

/// Install the select default actions. Three root-level listeners:
/// click (select / toggle), keydown (arrow navigation + Space +
/// Home/End + Ctrl+A), and a focus hook that sets up the initial
/// highlight when a select gains focus.
pub fn install(dom: &mut TuiDom) {
    let root = dom.root();

    // Click → select / toggle / open-chrome.
    dom.add_event_listener(root, "click", ListenerOptions::default(), move |ctx| {
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
            Some(opt) if !ctx.dom.node(opt).has_attribute("disabled") => {
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
    })
    .expect("select click listener install");

    // Keydown → navigation + selection.
    dom.add_event_listener(root, "keydown", ListenerOptions::default(), move |ctx| {
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
    })
    .expect("select keydown listener install");
}

// ── Public read API ────────────────────────────────────────────────

/// Collect every currently-selected `<option>` under `select`, in
/// document order. Used by `form::collect` and by apps that want
/// to read the current selection.
pub fn selected_options(dom: &TuiDom, select: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    for opt in options(dom, select) {
        if dom.node(opt).has_attribute("selected") {
            out.push(opt);
        }
    }
    out
}

/// The current value of a `<select>`. Single-select: value of the
/// selected option (or empty if none). Multi-select: space-
/// separated values of all selected options (matches the legacy
/// rdom serialization).
pub fn value(dom: &TuiDom, select: NodeId) -> String {
    let selected = selected_options(dom, select);
    if selected.is_empty() {
        return String::new();
    }
    if is_multi(dom, select) {
        selected
            .iter()
            .map(|&id| option_value(dom, id))
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        option_value(dom, selected[0])
    }
}

/// Read an `<option>`'s submit value: the `value` attribute, or
/// (per HTML) the text content when `value` is absent.
pub fn option_value(dom: &TuiDom, option: NodeId) -> String {
    if let Some(v) = dom.node(option).get_attribute("value") {
        return v.to_string();
    }
    option_label(dom, option)
}

/// Read an `<option>`'s display label: the `label` attribute if
/// set, otherwise the text content.
pub fn option_label(dom: &TuiDom, option: NodeId) -> String {
    if let Some(v) = dom.node(option).get_attribute("label") {
        return v.to_string();
    }
    let mut text = String::new();
    for child in dom.node(option).child_nodes() {
        if child.node_type() == rdom_core::NodeType::Text
            && let Some(s) = child.node_value()
        {
            text.push_str(s);
        }
    }
    text
}

// ── Click path ─────────────────────────────────────────────────────

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

fn select_single(dom: &mut TuiDom, select: NodeId, option: NodeId) {
    let current = selected_options(dom, select);
    let already = current.len() == 1 && current[0] == option;
    if already {
        return;
    }
    for opt in current {
        let _ = dom.remove_attribute(opt, "selected");
    }
    let _ = dom.set_attribute(option, "selected", "");
}

fn toggle_option(dom: &mut TuiDom, _select: NodeId, option: NodeId) {
    if dom.node(option).has_attribute("selected") {
        let _ = dom.remove_attribute(option, "selected");
    } else {
        let _ = dom.set_attribute(option, "selected", "");
    }
}

/// Shift-click / shift-arrow: extend selection from the anchor
/// option to `target`. Sets the anchor on first shift-action if
/// absent. All options between anchor and target (inclusive)
/// become selected; others outside the range are deselected.
fn extend_selection_to(dom: &mut TuiDom, select: NodeId, target: NodeId) {
    let anchor = anchor(dom, select).unwrap_or_else(|| {
        // First shift action — anchor snaps to the current highlight
        // (or the target itself when no highlight exists yet).
        let a = highlight(dom, select).unwrap_or(target);
        set_anchor(dom, select, Some(a));
        a
    });
    let all = options(dom, select);
    let a_idx = all.iter().position(|&o| o == anchor);
    let t_idx = all.iter().position(|&o| o == target);
    let (Some(a_idx), Some(t_idx)) = (a_idx, t_idx) else {
        return;
    };
    let (lo, hi) = if a_idx <= t_idx {
        (a_idx, t_idx)
    } else {
        (t_idx, a_idx)
    };
    for (i, &opt) in all.iter().enumerate() {
        if dom.node(opt).has_attribute("disabled") {
            continue;
        }
        if i >= lo && i <= hi {
            let _ = dom.set_attribute(opt, "selected", "");
        } else {
            let _ = dom.remove_attribute(opt, "selected");
        }
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

/// Polish #6: multi-keystroke type-ahead buffer.
///
/// A single-keystroke match (C.7c original) is frustrating when
/// several options share a first letter. This upgrade accumulates
/// typed characters within an inactivity window and matches
/// whole-prefix, case-insensitive.
///
/// Reset rules:
/// - Inactivity timeout of [`TYPEAHEAD_TIMEOUT`] since the last
///   keystroke → next key starts a fresh buffer.
/// - Focus moves to a different `<select>` (or away entirely) →
///   buffer reset on next match.
/// - Buffer is shared across all selects (thread-local); since the
///   user can only interact with one focused select at a time,
///   cross-contamination isn't possible.
///
/// Stickiness: if the appended character extends the buffer to a
/// prefix no option matches, we still update the buffer (so the
/// user's next keystroke combines with it) and fall back to a
/// single-char search of the new key alone — matches browser
/// behavior where repeated typing "cycles" even past misses.
use std::cell::RefCell;
use std::time::{Duration, Instant};

const TYPEAHEAD_TIMEOUT: Duration = Duration::from_millis(500);

thread_local! {
    static TYPEAHEAD_STATE: RefCell<TypeaheadState> =
        RefCell::new(TypeaheadState::default());
}

#[derive(Default)]
struct TypeaheadState {
    buffer: String,
    last: Option<Instant>,
    last_select: Option<NodeId>,
}

/// Decode the DOM `KeyboardEvent.key` string into a single
/// printable character. Returns `None` for named keys (`"Enter"`,
/// `"ArrowUp"`, …), multi-char strings, and control characters.
/// Type-ahead, character-insertion, and printable-key heuristics
/// share this so the "is this a typed character?" check has one
/// home.
fn single_printable_char(key: &str) -> Option<char> {
    let mut chars = key.chars();
    let first = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    if first.is_control() {
        return None;
    }
    Some(first)
}

fn typeahead_search(dom: &mut TuiDom, select: NodeId, ch: char, multi: bool) {
    let all = options(dom, select);
    if all.is_empty() {
        return;
    }

    // Update the shared buffer, returning (query, cycle_mode). In
    // cycle mode we advance the start index PAST the current
    // highlight so repeated same-letter taps cycle through matches;
    // in prefix mode (multi-char buffer) we start AT the current
    // highlight so extra letters refine the match.
    let (query_lower, cycle_mode): (String, bool) = TYPEAHEAD_STATE.with(|s| {
        let mut st = s.borrow_mut();
        let now = Instant::now();
        let expired = st
            .last
            .is_none_or(|t| now.duration_since(t) > TYPEAHEAD_TIMEOUT);
        let switched = st.last_select != Some(select);
        let lc = ch.to_ascii_lowercase();
        let mut cycle = false;

        if expired || switched {
            st.buffer.clear();
            st.buffer.push(lc);
        } else if st.buffer.len() == 1 && st.buffer.starts_with(lc) {
            // Same single char repeated within the timeout →
            // cycle. Buffer stays as that one char.
            cycle = true;
        } else {
            st.buffer.push(lc);
        }
        st.last = Some(now);
        st.last_select = Some(select);
        (st.buffer.clone(), cycle)
    });

    let start_idx = highlight(dom, select)
        .and_then(|h| all.iter().position(|&o| o == h))
        .map(|i| if cycle_mode { i + 1 } else { i })
        .unwrap_or(0);
    let len = all.len();

    // Two passes: current-and-after then from-start (wrap). For a
    // single-char buffer we advance PAST the current highlight so
    // repeated taps cycle; for a multi-char buffer we start AT the
    // current highlight so additional letters refine rather than
    // skip.
    let target = (0..len).find_map(|offset| {
        let i = (start_idx + offset) % len;
        let opt = all[i];
        if dom.node(opt).has_attribute("disabled") {
            return None;
        }
        let label = option_label(dom, opt).to_ascii_lowercase();
        label.starts_with(&query_lower).then_some(opt)
    });
    let Some(target) = target else { return };

    set_highlight(dom, select, Some(target));
    if !multi {
        select_single(dom, select, target);
    } else {
        // Multi-select: clear anchor so the next shift-action
        // starts from the new highlight (matches the C.7a arrow-
        // nav pattern).
        set_anchor(dom, select, None);
    }
    fire_input_and_change(dom, select);
}

#[cfg(test)]
pub(super) fn reset_typeahead_buffer_for_tests() {
    TYPEAHEAD_STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.buffer.clear();
        st.last = None;
        st.last_select = None;
    });
}

// ── Event firing ───────────────────────────────────────────────────

/// Fire `input` then `change` on the select (matches the pattern
/// from C.4b toggle). Both non-cancelable; apps observe.
fn fire_input_and_change(dom: &mut TuiDom, select: NodeId) {
    let mut input_ev = TuiEvent::new("input");
    let _ = dom.dispatch_tui_event(select, &mut input_ev);
    let mut change_ev = TuiEvent::new("change");
    let _ = dom.dispatch_tui_event(select, &mut change_ev);
}

// ── Tree traversal helpers ─────────────────────────────────────────

/// Is this select a multi-select (either `multiple` attribute set,
/// or `size > 1` — both produce the listbox with independent
/// selection per HTML).
fn is_multi(dom: &TuiDom, select: NodeId) -> bool {
    dom.node(select).has_attribute("multiple")
}

/// Collect every `<option>` descendant of `select`, in document
/// order. Descends into `<optgroup>` children (options nest under
/// groups per HTML).
pub fn options(dom: &TuiDom, select: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk_options(dom, select, &mut out);
    out
}

fn walk_options(dom: &TuiDom, id: NodeId, out: &mut Vec<NodeId>) {
    for child in dom.node(id).child_nodes() {
        match child.tag_name() {
            Some("option") => out.push(child.id()),
            Some("optgroup") => walk_options(dom, child.id(), out),
            _ => {}
        }
    }
}

fn closest_option(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if dom.node(n).tag_name() == Some("option") {
            return Some(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
}

/// Walk up from `id` (inclusive) to the nearest `<select>`.
/// Same logic as `enclosing_select`; separate name reflects its
/// use from the click path where the target might be the select
/// chrome itself, not an option descendant.
fn closest_select(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    enclosing_select(dom, id)
}

fn enclosing_select(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if dom.node(n).tag_name() == Some("select") {
            return Some(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
}

// ── Dropdown open/close (C.7b) ─────────────────────────────────────

/// True when this select is a dropdown (single-select, no
/// `multiple`, no `size`). Listbox selects always show their
/// option list and don't have open/closed state.
pub fn is_dropdown(dom: &TuiDom, select: NodeId) -> bool {
    !dom.node(select).has_attribute("multiple") && !dom.node(select).has_attribute("size")
}

/// True when this dropdown is currently open. Always `false`
/// for listbox-mode selects.
pub fn is_open(dom: &TuiDom, select: NodeId) -> bool {
    dom.node(select).has_attribute(OPEN_ATTR)
}

/// Open a dropdown — set the open marker. No-op for listbox-
/// mode selects (their option list is always visible).
pub fn open(dom: &mut TuiDom, select: NodeId) {
    if !is_dropdown(dom, select) {
        return;
    }
    let _ = dom.set_attribute(select, OPEN_ATTR, "");
}

/// Close a dropdown — clear the open marker. No-op for listbox-
/// mode selects.
pub fn close(dom: &mut TuiDom, select: NodeId) {
    if !is_dropdown(dom, select) {
        return;
    }
    let _ = dom.remove_attribute(select, OPEN_ATTR);
}

// ── Highlight / anchor attribute helpers ───────────────────────────

fn highlight(dom: &TuiDom, select: NodeId) -> Option<NodeId> {
    options(dom, select)
        .into_iter()
        .find(|&o| dom.node(o).has_attribute(HIGHLIGHT_ATTR))
}

fn set_highlight(dom: &mut TuiDom, select: NodeId, target: Option<NodeId>) {
    for opt in options(dom, select) {
        let want = target == Some(opt);
        let has = dom.node(opt).has_attribute(HIGHLIGHT_ATTR);
        if want && !has {
            let _ = dom.set_attribute(opt, HIGHLIGHT_ATTR, "");
        } else if !want && has {
            let _ = dom.remove_attribute(opt, HIGHLIGHT_ATTR);
        }
    }
}

fn anchor(dom: &TuiDom, select: NodeId) -> Option<NodeId> {
    options(dom, select)
        .into_iter()
        .find(|&o| dom.node(o).has_attribute(ANCHOR_ATTR))
}

fn set_anchor(dom: &mut TuiDom, select: NodeId, target: Option<NodeId>) {
    for opt in options(dom, select) {
        let want = target == Some(opt);
        let has = dom.node(opt).has_attribute(ANCHOR_ATTR);
        if want && !has {
            let _ = dom.set_attribute(opt, ANCHOR_ATTR, "");
        } else if !want && has {
            let _ = dom.remove_attribute(opt, ANCHOR_ATTR);
        }
    }
}

#[cfg(test)]
mod tests;
