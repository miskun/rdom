//! `serialize`: property name → CSS text for whatever `TuiStyle`
//! currently holds under that name, one arm per property, by family
//! (`box_model`, `flex`, `paint`, `position`; the backgrounds, borders,
//! shadows, containment, grid and flow-relative properties in their own
//! modules). Shorthands
//! only serialize when their longhands agree (`overflow`, `flex`,
//! `border`, `inset`, `transition`); the per-value-type helpers live
//! in `value_serializers.rs`.

mod box_model;
mod flex;
mod paint;
mod position;

use super::css_wide::css_wide_of;
use super::table::canonical_property_name;
use crate::TuiStyle;

/// Serialize the named property's current value as a CSS string.
/// Returns `None` when the property isn't currently set — callers
/// map this to `""` to match `getPropertyValue`.
///
/// Unknown property names also return `None` (rather than
/// errorring); CSSOM `getPropertyValue("bogus")` returns `""` too.
pub fn serialize(name: &str, style: &TuiStyle) -> Option<String> {
    if let Some(custom) = name.strip_prefix("--") {
        return style.custom_property_value(custom).map(str::to_string);
    }
    let name = &*canonical_property_name(name);
    // A `var()` value is kept as written until the cascade (CSS
    // Variables 1 §3).
    if let Some(d) = style.pending.iter().find(|d| {
        d.name == name && d.has_substitution && d.restriction == crate::var::Restriction::All
    }) {
        return Some(d.value_text());
    }
    // An inline-axis flow-relative property is mapped only by the
    // cascade (CSS Logical 1 §4): read from its last declarations, a
    // shorthand's component included (CSSOM §6.6).
    if let Some(out) = super::logical::serialize_inline_axis(name, style) {
        return out;
    }
    if let Some(kw) = css_wide_of(name, style) {
        return Some(kw.to_string());
    }
    if super::logical::is_directional(name) {
        return None;
    }
    if let Some(out) = super::background::serialize(name, style)
        .or_else(|| super::border::serialize(name, style))
        .or_else(|| super::shadow::serialize(name, style))
        .or_else(|| super::contain::serialize(name, style))
        .or_else(|| super::line_clamp::serialize(name, style))
        .or_else(|| super::float::serialize(name, style))
        .or_else(|| super::scrollbar::serialize(name, style))
        .or_else(|| super::scroll::serialize(name, style))
        .or_else(|| super::grid::serialize(name, style))
        .or_else(|| super::text::serialize(name, style))
        .or_else(|| super::inline::serialize(name, style))
        .or_else(|| super::text_decoration::serialize(name, style))
        .or_else(|| super::logical::serialize_block_axis(name, style))
    {
        return out;
    }
    box_model::serialize(name, style)
        .or_else(|| flex::serialize(name, style))
        .or_else(|| paint::serialize(name, style))
        .or_else(|| position::serialize(name, style))
        .flatten()
}
