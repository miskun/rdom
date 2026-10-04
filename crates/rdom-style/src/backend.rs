//! Hooks for style-engine backends — the cascade — not for applications.
//!
//! `var()` and `attr()` substitution happens at computed-value time (CSS
//! Variables 1 §3, CSS Values 5 §8.7), so it runs inside a backend's cascade, not in the parser: the
//! declaration parser keeps a `var()` declaration as tokens
//! ([`PendingDeclaration`](crate::PendingDeclaration) on
//! [`TuiStyle::pending`](crate::TuiStyle::pending)), and the backend
//! substitutes it per element ([`TuiStyle::substituted_pending`](crate::TuiStyle::substituted_pending))
//! after resolving the element's custom properties with the functions
//! here. `rdom-tui`'s cascade is the user today; a sibling backend
//! (another renderer over the same data model, CLAUDE.md "Substrate
//! First, Backend Second") needs the same hooks, which is why they are
//! public — grouped here, rather than hidden, so their contract is
//! documented in one place.
//!
//! An application builds styles with [`TuiStyle`](crate::TuiStyle),
//! [`property_dispatch::set`](crate::property_dispatch::set) or
//! `rdom-css`, and reads computed values from the backend; it has no
//! reason to call anything in this module.

pub use crate::attr::AttrLookup;
pub use crate::property_dispatch::set::{set_parsed, set_unset};
pub use crate::var::{
    ComputedStep, MAX_SUBSTITUTED_TOKENS, contains_substitution, contains_var, lookup_in,
    resolve_custom_properties, resolve_custom_properties_on, resolve_custom_properties_with,
    substitute, substitute_with, valid_var_syntax,
};
