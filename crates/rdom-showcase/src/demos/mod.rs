//! Demo implementations. Each demo is a module that defines a
//! single unit struct implementing [`crate::Demo`]. The registry
//! ([`crate::registry::DEMOS`]) holds `'static` references to
//! const instances of those structs.
//!
//! Adding a demo:
//! 1. Add a new module file under this directory.
//! 2. Define a unit struct + impl `Demo` for it.
//! 3. Register the struct in [`crate::registry::DEMOS`].

pub mod app_shell;
pub mod border_collapse;
pub mod counter_button;
pub mod dom_api;
pub mod flex_row;
pub mod headings;
pub mod hello;
pub mod hover;
pub mod inline_formatting;
pub mod interval_counter;
pub mod mutation_observer;
pub mod parse_and_render;
pub mod raf_progress;
pub mod scrollable_list;
pub mod selectable_text;
pub mod sticky;
pub mod tab_form;
pub mod transition_box;
pub mod ua_chrome;
