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
//! - `declare.rs`: [`set`] / [`set_from_source`] / [`set_from_tokens`]
//!   / [`set_custom`] — declare on a block: custom properties, and the
//!   values kept for the cascade (`var()` / `attr()`, inline-axis
//!   flow-relative properties) with their text.
//! - `set.rs`: `set_parsed` — parse a declaration value and write the
//!   owned field(s), including per-side longhand merging.
//! - `importance.rs`: [`set_important`] / [`is_important`] — a
//!   declaration's `!important`.
//! - `serialize.rs`: [`serialize`] — property → CSS text, one arm
//!   per name.
//! - `background.rs` / `border.rs` / `shadow.rs` / `contain.rs`: the
//!   `set` / `serialize` arms of the `background` and `border` shorthands
//!   and their longhands, of `box-shadow`, and of `contain-intrinsic-size`
//!   and its longhands.
//! - `logical.rs`: the flow-relative properties (CSS Logical 1) — the
//!   block-axis ones mapped onto their physical twins when declared, the
//!   inline-axis ones kept for the cascade to map by `direction`.
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

mod background;
mod border;
mod contain;
mod css_wide;
mod declare;
mod fields;
mod float;
mod font;
mod grid;
mod importance;
mod inline;
mod line_clamp;
mod logical;
mod names;
mod scroll;
mod scrollbar;
mod serialize;
pub(crate) mod set;
mod shadow;
mod table;
mod text;
mod text_decoration;
mod value_serializers;

#[cfg(test)]
mod align_tests;
#[cfg(test)]
mod background_tests;
#[cfg(test)]
mod border_tests;
#[cfg(test)]
mod content_tests;
#[cfg(test)]
mod display_tests;
#[cfg(test)]
mod flex_tests;
#[cfg(test)]
mod float_tests;
#[cfg(test)]
mod font_tests;
#[cfg(test)]
mod grid_areas_tests;
#[cfg(test)]
mod grid_shorthand_tests;
#[cfg(test)]
mod grid_tests;
#[cfg(test)]
mod inline_tests;
#[cfg(test)]
mod line_clamp_tests;
#[cfg(test)]
mod list_tests;
#[cfg(test)]
mod logical_tests;
#[cfg(test)]
mod overflow_tests;
#[cfg(test)]
mod scroll_tests;
#[cfg(test)]
mod scrollbar_tests;
#[cfg(test)]
mod sizing_tests;
#[cfg(test)]
mod spacing_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod text_decoration_tests;
#[cfg(test)]
mod text_tests;
#[cfg(test)]
mod visibility_tests;
#[cfg(test)]
mod writing_tests;

pub use declare::{set, set_custom, set_custom_source, set_from_source, set_from_tokens};
pub use importance::{is_important, set_important};
pub use serialize::serialize;
// `set_parsed` / `set_unset` are backend hooks, public through
// `crate::backend`.
pub use logical::is_storage_alias;
pub(crate) use logical::{is_directional, mapped_mask};
pub(crate) use set::{set_parsed_in, set_unset_in};
pub(crate) use table::{IMPORTANT_BITS, copy_fields, important_bit_name, set_field_count};
pub use table::{canonical_property_name, inherits, property_mask, property_names, remove};
pub(crate) use value_serializers::serialize_math;

/// Reason `set` / `set_from_tokens` rejected a declaration.
///
/// The block parser maps these onto its existing `Warning`
/// variants; CSSOM call sites (step 26) typically silently
/// no-op (browser-faithful — `element.style.bogus = 'x'`
/// doesn't throw).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DispatchError {
    /// `name` isn't in the dispatch table.
    UnknownProperty,
    /// `name` is known but `value` failed to parse.
    InvalidValue,
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnknownProperty => "unknown CSS property",
            Self::InvalidValue => "invalid value for property",
        })
    }
}

impl std::error::Error for DispatchError {}
