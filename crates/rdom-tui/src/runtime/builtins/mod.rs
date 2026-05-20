//! Built-in element default behaviors — one module per HTML
//! element that ships with opinionated keyboard / click / attribute
//! defaults.
//!
//! ## Landing pattern
//!
//! Each builtin is a small module that installs a root-level
//! event listener at `App::build` time. The listener walks up
//! from the event target to find the owning element (if any) and
//! runs the default action **after** author listeners have had a
//! chance to call `event.prevent_default`. This matches browser
//! behavior and keeps default actions out of the per-element
//! construction path — no setup hook, no registry, no per-node
//! listener bookkeeping.
//!
//! ## Roster
//!
//! - `a_href` — scheme-based click dispatch. External URLs shell
//!   out via [`UrlOpener`]; internal schemes are a no-op (apps
//!   route via their own `click` listener).
//!
//! [`UrlOpener`]: crate::runtime::url_opener::UrlOpener

pub mod a_href;
pub mod button;
pub mod canvas;
pub mod details;
pub mod dialog;
pub mod form;
pub mod gauge;
pub mod input;
pub mod label;
pub mod number;
pub mod range;
pub mod select;
pub mod table;
pub mod toggle;
