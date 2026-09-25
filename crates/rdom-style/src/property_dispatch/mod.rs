//! Property dispatch table — single source for the
//! `name → (setter, serializer)` mapping that drives:
//!
//! - `rdom_css`'s declaration block parser (`apply_declaration`)
//! - `rdom_tui::cssom::StyleDeclaration` (step 26)
//!
//! ## Why this exists
//!
//! Before M4b step 25 the dispatch table lived as a big `match`
//! inside `declarations::apply_declaration`. Step 26's
//! `StyleDeclaration::set_property` needs the same name→setter
//! routing, and duplicating ~150 lines of match across crates was
//! the M4b architect-pass risk that prompted this extraction.
//!
//! ## Surface
//!
//! - [`set`] takes a property name + value-string and writes to a
//!   `TuiStyle`. Returns [`DispatchError`] for unknown names /
//!   invalid values.
//! - [`set_from_tokens`] is the same function pre-tokenized — the
//!   block parser uses this so it doesn't re-tokenize per
//!   declaration.
//! - [`serialize`] emits the CSS string form for whichever value
//!   is currently stored under `name` on `style`. `None` means the
//!   property is unset (caller maps to `""` per CSSOM
//!   `getPropertyValue` convention).
//! - [`property_names`] is the sorted list of every property name
//!   the dispatch table knows about — drives camelCase alias
//!   generation in step 27 and `length` / `item` in step 26.
//!
//! ## Layout (`STYLE-DISPATCH-SPLIT-1`)
//!
//! One file per concern; this module only re-exports:
//!
//! - `table.rs`: the property name ↔ storage-field
//!   table (`Field`, `fields_of`), [`property_names`],
//!   [`property_mask`], [`remove`], [`inherits`].
//! - `css_wide.rs`: the CSS-wide keywords (`inherit` / `initial` /
//!   `unset`) — detection, storage across every owned field, and
//!   the all-fields-agree serialization rule.
//! - `set.rs`: [`set`] / [`set_from_tokens`] — parse a declaration
//!   value and write the owned field(s), including custom
//!   properties and per-side longhand merging.
//! - `serialize.rs`: [`serialize`] — property → CSS text, one arm
//!   per name.
//! - `value_serializers.rs`: the per-value-type serializers
//!   (`serialize_color`, `serialize_calc`, …) `serialize.rs` folds
//!   over.
//! - `tests.rs`: the round-trip contract and per-property tests.
//!
//! ## Round-trip contract
//!
//! For every name in [`property_names`], the following must hold
//! for at least one canonical value `v`:
//!
//! ```text
//! let mut style = TuiStyle::new();
//! property_dispatch::set(name, v, &mut style)?;
//! let serialized = property_dispatch::serialize(name, &style).unwrap();
//! let mut roundtrip = TuiStyle::new();
//! property_dispatch::set(name, &serialized, &mut roundtrip)?;
//! assert_eq!(style.<field>, roundtrip.<field>);
//! ```
//!
//! The `round_trip_every_property` integration test in this module
//! enforces this for the full table.

mod css_wide;
mod serialize;
mod set;
mod table;
mod value_serializers;

#[cfg(test)]
mod tests;

pub use serialize::serialize;
pub use set::{set, set_from_tokens};
pub use table::{inherits, property_mask, property_names, remove};

/// Reason `set` / `set_from_tokens` rejected a declaration.
///
/// The block parser maps these onto its existing `Warning`
/// variants; CSSOM call sites (step 26) typically silently
/// no-op (browser-faithful — `element.style.bogus = 'x'`
/// doesn't throw).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    /// `name` isn't in the dispatch table.
    UnknownProperty,
    /// `name` is known but `value` failed to parse.
    InvalidValue,
}
