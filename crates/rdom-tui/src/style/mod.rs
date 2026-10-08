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

pub(crate) mod accent;
pub mod cascade;
pub(crate) mod dir_auto;
pub mod dirty_tracker;
pub(crate) mod doc_flags;
pub(crate) mod has_triggers;
pub(crate) mod pseudo_pointer;
pub(crate) mod selector_walk;
pub(crate) mod sibling_triggers;
pub(crate) mod user_select;

#[cfg(test)]
mod layering_tests;

pub use cascade::CascadeExt;
pub use dirty_tracker::DirtyTracker;

// ── Data-model re-exports from rdom-style ───────────────────────────
//
// Every `rdom_tui::style::X` path that worked before the
// rdom-style extraction keeps working through these re-exports.
// Internal rdom-tui code uses `rdom_style::X` directly for clarity.

/// The animation types and interpolation of computed values
/// (`animation::Longhand`, `animation::AnimationType`) — what a running
/// transition composites onto a style.
pub use rdom_style::animation;
pub use rdom_style::color::{ColorScheme, ColorSchemeList, SystemColor};
/// The counter styles (CSS Counter Styles 3): `@counter-style` rules
/// (`counters::CounterStyleRule`, `counters::System`, …), the predefined
/// styles and the registry — rdom-style's module, so `CounterStyle::symbols`
/// takes a `System` a consumer of `rdom-tui` alone can name.
pub use rdom_style::counters;
pub use rdom_style::counters::CounterStyleName;
/// `@keyframes` rules and the `animation-*` values (CSS Animations 1 / 2).
pub use rdom_style::keyframes;
pub use rdom_style::transition;
pub use rdom_style::{
    AnimationComposition, AnimationDirection, AnimationDuration, AnimationFillMode, AnimationName,
    AnimationPlayState, AnimationTimeline, IterationCount, Keyframe, KeyframeSelector,
    KeyframesRule, RangeBoundary, ResolvedKeyframe, TimelineAxis, TimelineInset, TimelineName,
    TimelineRangeName, TimelineScope, TimelineScroller,
};
pub use rdom_style::{
    Color, ColorContext, ColorFunction, ComputedStyle, Content, ContentContext, CounterOp,
    CounterStyle, CustomDeclaration, CustomValue, FontDeclarations, ImportantMask, LayerId,
    Modifier, PropertyRegistration, PropertySyntax, PropertySyntaxError, PseudoElementTarget,
    QuoteKind, QuotePair, Quotes, RegisterPropertyError, Rule, RuleContext, RuleOrigin,
    Specificity, StyleError, StyleSelector, Stylesheet, TableDeclarations, TextDeclarations,
    TextDecorationDeclarations, TimingFunction, TransitionProperty, TransitionRule, TuiColor,
    TuiStyle, UiDeclarations, UserActionState, Value, VarMap, parse_color, resolve_tui_color,
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
