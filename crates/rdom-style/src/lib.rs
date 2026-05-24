//! # rdom-style — CSS data model + property dispatch for rdom
//!
//! Leaf crate that holds the **value types** consumed by both the
//! CSS string parser (`rdom-css`) and the cascade/layout/paint
//! pipeline (`rdom-tui`):
//!
//! - [`TuiStyle`] — author-input style block; what every cascade
//!   rule writes into.
//! - [`ComputedStyle`] — post-cascade concrete values.
//! - [`Stylesheet`] / [`Rule`] — the rule collection + UA defaults.
//! - [`TuiColor`], [`Color`], [`Modifier`] — color + modifier
//!   primitives.
//! - [`Value`], [`Specificity`], [`ImportantMask`] — cascade
//!   primitives.
//! - [`layout`] — `Display`, `Direction`, `WhiteSpace`, `Size`,
//!   `Padding`, `Border`, `Position`, `Length`, `ZIndex`,
//!   `Overflow`, `LayoutRect`, …
//! - [`transition`] — animation type system.
//! - [`property_dispatch`] — the single name→(setter, serializer)
//!   table both `rdom-css` (parser) and `rdom-tui`
//!   (`StyleDeclaration`) consume.
//! - [`parse`] — token-level CSS parsing primitives used by
//!   `property_dispatch` and re-exported for `rdom-css`'s block
//!   parser.
//!
//! ## Why a leaf crate
//!
//! Pre-refactor `rdom-css` depended on `rdom-tui` (for `TuiStyle`),
//! which made the parser transitively depend on cascade, layout,
//! paint, runtime, crossterm — violating CLAUDE.md's "parser must
//! not require a runtime or a backend" rule. Step 26's
//! `StyleDeclaration` would have closed a Cargo cycle. Extracting
//! the data model into this leaf inverts the dep direction so:
//!
//! ```text
//! rdom-core    (substrate)
//!     ↑
//! rdom-style   (this crate — data model + property_dispatch)
//!     ↑      ↑
//! rdom-css   rdom-tui
//! ```
//!
//! Authors keep using `rdom_tui::TuiStyle` etc.; `rdom-tui`
//! re-exports our types for backward compatibility.

#![forbid(unsafe_code)]

pub mod calc;
pub mod layout;
pub mod parse;
pub mod property_dispatch;
pub mod transition;

pub mod color;
mod computed;
mod modifier;
mod specificity;
mod stylesheet;
mod tui_color;
mod tui_style;
mod ua;
mod value;

pub use color::Color;
pub use computed::{ComputedStyle, Content, VarMap};
pub use modifier::Modifier;
pub use specificity::Specificity;
pub use stylesheet::{PseudoElementTarget, Rule, RuleOrigin, StyleError, Stylesheet};
pub use transition::{AnimatableProperty, TimingFunction, TransitionProperty, TransitionRule};
pub use tui_color::{TuiColor, parse_color, resolve_tui_color};
pub use tui_style::{ImportantMask, TuiStyle};
pub use value::Value;
