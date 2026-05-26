//! `rdom-showcase` — the in-tree TUI app that mounts every rdom
//! primitive in one browsable binary.
//!
//! See [`specs/SHOWCASE.md`](../../specs/SHOWCASE.md) in the project
//! root for the design and milestone plan. This crate is a downstream
//! consumer of the substrate (`rdom-core` + `rdom-style` +
//! `rdom-tui`); it adds zero new types to those crates. Anything the
//! showcase needs that the substrate doesn't already provide lands as
//! a substrate addition first.
//!
//! Not published. Lives in-tree so showcase regressions block CI on
//! every `rdom-tui` change.
//!
//! ## Module layout
//!
//! - [`demo`] — the `Demo` trait, `Category` enum, `Source` struct.
//! - [`registry`] — the hardcoded `DEMOS` table.
//! - [`shell`] — `build_shell` constructs the sidebar + main view +
//!   header and returns the `NodeId` consumers mount the active
//!   demo into.

pub mod demo;
pub mod demos;
pub mod nav;
pub mod registry;
pub mod shell;
pub mod status_bar;

pub use demo::{Category, Demo, Source};
pub use nav::{
    ShowcaseState, mount_demo, wire_scroll_indicator, wire_sidebar_click, wire_sidebar_keys,
};
pub use registry::DEMOS;
pub use shell::{ShellHandles, build_shell};
pub use status_bar::{seed_default_hints, wire_focus_hints};
