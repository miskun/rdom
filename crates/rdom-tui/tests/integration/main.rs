//! Consolidated `rdom-tui` integration tests.
//!
//! All `tests/*.rs` files previously compiled as separate binaries
//! (one per file × ~5MB debug binary × statically-linked crate);
//! consolidating them into a single test target collapses 32 link
//! steps into 1, cutting workspace test build time by ~15s.
//!
//! Pattern: each former `tests/<name>.rs` is now
//! `tests/integration/<name>.rs` declared here as a module. The
//! shared `common` module hosts the headless render helpers and is
//! `pub mod`-exposed so module siblings can `use crate::common::...`.
//! Demo paint snapshots live in `rdom-showcase`'s suite, next to the
//! demos they pin; this crate does not depend on its consumer.
//!
//! Adding a new integration test: drop the file alongside this
//! `main.rs` and add a `mod <filename_without_rs>;` line below.

#![allow(dead_code)] // Some test helpers are only used by a
// subset of modules; the allow keeps things tidy without forcing
// per-file pragmas.

pub mod common;

mod append_text_reflows_layout;
mod bfc_cascade;
mod border_model_contract;
mod button_flex_repro;
mod calc_layout;
mod cssom_cascade;
mod event_request_redraw;
mod flex_blockifies_inline_children;
mod flex_shorthand;
mod flex_shrink;
mod flex_two_slot_layout;
mod implicit_detach_events;
mod inline_flow;
mod input_render_integration;
mod m5_abortsignal;
mod nested_collapse_content_inset;
mod nested_collapse_root_opacity;
mod node_setters_drive_layout;
mod padding_box_paint_clip;
mod percent_units;
mod seed_inline_styles;
mod select_render_integration;
mod style_tags;
mod subtree_replacement_contract;
mod tab_form_integration;
mod textarea_integration;
mod transparent_collapse_propagation;
mod ua_focus_overridable;
