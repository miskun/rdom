//! Styling layer — cascade engine + dirty tracker on top of the
//! `rdom-style` data model.
//!
//! ## What lives here vs in `rdom-style`
//!
//! - **rdom-style** owns the *data model*: `TuiStyle`, `Value<T>`,
//!   `TuiColor`, `ComputedStyle`, `Stylesheet`, `Specificity`,
//!   `ImportantMask`, the transition types, layout enums, color +
//!   modifier.
//! - **This module** owns *the runtime cascade*: walking the tree,
//!   building each element's `ComputedStyle` from the matching
//!   rules, and the `MutationObserver` (`DirtyTracker`) that
//!   marks subtrees dirty for incremental re-cascade.
//!
//! The split was the M4b mid-stream restructure (rdom-style
//! extracted as a leaf so `rdom-css` and `rdom-tui` could both
//! depend on it without a Cargo cycle).
//!
//! ## Cascade ladder (CSS-spec faithful)
//!
//! 1. UA normal → 2. Author normal → 3. Inline normal →
//!    4. Author important → 5. Inline important → 6. UA important.
//!
//! `!important` inverts origin priority; the `style` attribute beats
//! the author's rules at both importances (Cascade 4 §6.1).

pub mod cascade;
pub mod dirty_tracker;
pub(crate) mod selector_walk;
pub(crate) mod sibling_triggers;
pub(crate) mod user_select;

pub use cascade::CascadeExt;
pub use dirty_tracker::DirtyTracker;

// ── Data-model re-exports from rdom-style ───────────────────────────
//
// Every `rdom_tui::style::X` path that worked before the
// rdom-style extraction keeps working through these re-exports.
// Internal rdom-tui code uses `rdom_style::X` directly for clarity.

pub use rdom_style::color::{ColorScheme, ColorSchemeList, SystemColor};
pub use rdom_style::transition;
pub use rdom_style::{
    AnimatableProperty, Color, ColorContext, ColorFunction, ComputedStyle, Content, ContentContext,
    CounterOp, CounterStyle, CustomDeclaration, CustomValue, ImportantMask, LayerId, Modifier,
    PropertyRegistration, PropertySyntax, PropertySyntaxError, PseudoElementTarget,
    RegisterPropertyError, Rule, RuleContext, RuleOrigin, Specificity, StyleError, StyleSelector,
    Stylesheet, TimingFunction, TransitionProperty, TransitionRule, TuiColor, TuiStyle, Value,
    VarMap, parse_color, resolve_tui_color,
};
/// The declaration-level CSS parsing primitives (`parse::tokenize`,
/// `parse::Token`, `parse::values::*`), the property dispatch table
/// (`property_dispatch::set` / `serialize`, the one `rdom-css` and the
/// CSSOM use) and the cascade's substitution hooks
/// (`backend::SubstitutionContext`, which `TuiStyle::substituted` takes):
/// rdom-style's modules, here so a consumer of `rdom-tui` alone reaches
/// every path the style types' signatures and the CHANGELOG name —
/// `rdom_tui::style::backend` is the style backend, not the terminal
/// [`Backend`](crate::Backend).
pub use rdom_style::{backend, parse, property_dispatch};
