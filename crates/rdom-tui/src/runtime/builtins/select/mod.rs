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
//! Keeping the keyboard highlight visible inside an overflowing
//! listbox is the author's job (`overflow-y: auto`); the builtin does
//! not reveal it automatically.
//!
//! ## Module map
//!
//! - [`model`] — option list + read API (`options`,
//!   `selected_options`, `value`, `option_value`, `option_label`),
//!   `multiple` / display-size queries, and the option / select
//!   ancestor walks.
//! - [`state`] — selection writes shared by every input path
//!   (single pick, toggle, anchor range extend), the highlight /
//!   anchor markers, and `input` + `change` firing.
//! - [`click`] — the `click` default action: option pick / toggle,
//!   shift-click extend, dropdown chrome toggle and auto-close.
//! - [`keyboard`] — the `keydown` default action: arrow / Home /
//!   End stepping, Space, Ctrl+A, Enter / Escape on a dropdown.
//! - [`typeahead`] — the per-select type-ahead buffer and its
//!   prefix / cycle match.
//! - [`dropdown`] — dropdown-vs-listbox test and the open / close
//!   marker (`is_dropdown`, `is_open`, `open`, `close`).
//!
//! This file keeps [`install`] — the two root-level listeners — and
//! the public re-exports.

mod click;
mod dropdown;
mod keyboard;
mod model;
mod state;
mod typeahead;

use rdom_core::ListenerOptions;

use crate::TuiDom;

pub use dropdown::{close, is_dropdown, is_open, open};
pub use model::{option_label, option_value, options, selected_options, value};

/// Install the select default actions: two root-level listeners,
/// click (select / toggle / open-close) and keydown (arrow navigation
/// + Space + Home/End + Ctrl+A + type-ahead).
pub fn install(dom: &mut TuiDom) {
    let root = dom.root();

    // Click → select / toggle / open-chrome.
    dom.add_event_listener(root, "click", ListenerOptions::default(), click::on_click)
        .expect("select click listener install");

    // Keydown → navigation + selection.
    dom.add_event_listener(
        root,
        "keydown",
        ListenerOptions::default(),
        keyboard::on_keydown,
    )
    .expect("select keydown listener install");
}

#[cfg(test)]
mod tests;
