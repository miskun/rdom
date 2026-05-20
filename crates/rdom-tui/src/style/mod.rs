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
//!    4. Inline important → 5. Author important → 6. UA important.
//!
//! `!important` inverts origin priority.

pub mod cascade;
pub mod dirty_tracker;

pub use cascade::{CascadeExt, INHERITS_MASK, LAYOUT_MASK, PropMask};
pub use dirty_tracker::DirtyTracker;

// ── Data-model re-exports from rdom-style ───────────────────────────
//
// Every `rdom_tui::style::X` path that worked before the
// rdom-style extraction keeps working through these re-exports.
// Internal rdom-tui code uses `rdom_style::X` directly for clarity.

pub use rdom_style::transition;
pub use rdom_style::{
    AnimatableProperty, Color, ComputedStyle, Content, ImportantMask, Modifier,
    PseudoElementTarget, Rule, RuleOrigin, Specificity, StyleError, Stylesheet, TimingFunction,
    TransitionProperty, TransitionRule, TuiColor, TuiStyle, Value, VarMap, parse_color,
    resolve_tui_color,
};
