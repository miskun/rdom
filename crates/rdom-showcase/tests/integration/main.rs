//! Consolidated `rdom-showcase` integration tests. See
//! `crates/rdom-tui/tests/integration/main.rs` for the pattern.

#![allow(dead_code)]

pub mod common;

// Paint snapshots for every demo (the `examples/` shims run the same
// `build` + `stylesheet`, so they are covered too).
mod animations_demos_snapshot;
mod border_collapse_snapshot;
mod counter_button_snapshot;
mod dom_api_snapshot;
mod mutation_observer_snapshot;
mod parse_and_render_snapshot;
mod scrollable_list_snapshot;
mod selectable_text_snapshot;
mod sticky_snapshot;
mod tab_form_snapshot;
mod text_demos_snapshot;
mod ua_chrome_snapshot;

mod chrome_dump;
mod chrome_layout_contract;
mod details_toggle_regression;
mod keyboard_nav;
mod mutation_observer_demo;
mod resize_integration;
mod scaffold;
mod scroll_indicator;
mod scrollable_list_hover_regression;
mod scrollable_list_keyboard_scroll;
mod scrollable_list_wheel_regression;
mod sidebar_scroll_end_regression;
mod source_overflow_regression;
mod status_bar_renders;
mod subtree_swap_integration;

mod tab_form_typing_repro;

mod sticky_single_scrollbar;
