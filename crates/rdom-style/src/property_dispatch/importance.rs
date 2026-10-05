//! `!important` per declaration: [`set_important`] records it as a
//! declaration block takes a declaration, [`is_important`] reads it
//! back (CSSOM §6.6 `getPropertyPriority`).
//!
//! A property's importance is the bit of each storage field it writes
//! ([`property_mask`]) — except an inline-axis flow-relative property,
//! whose physical side the cascade picks by the element's `direction`:
//! its importance is recorded on its kept declaration
//! ([`PendingDeclaration::important`](crate::var::PendingDeclaration)).

use super::logical::inline_axis_important;
use super::table::{canonical_property_name, property_mask};
use crate::TuiStyle;

/// Record whether the declaration of `name` that `style` took last is
/// `!important`: on its kept declaration when the block keeps one (an
/// inline-axis property, or any declaration after one), and in the
/// fields' bits. `important: false` clears them (CSSOM `setProperty`
/// without a priority). Custom properties carry their own flag.
pub fn set_important(name: &str, important: bool, style: &mut TuiStyle) {
    let name = &*canonical_property_name(name);
    if let Some(d) = style.pending.iter_mut().rev().find(|d| d.name == name) {
        d.important = important;
    }
    if let Some(mask) = property_mask(name) {
        style.important = if important {
            style.important | mask
        } else {
            style.important.without(mask)
        };
    }
}

/// Whether `name` is `!important` in `style` (CSSOM §6.6
/// `getPropertyPriority`): an inline-axis property by its last
/// declarations' own priority, any other by its fields' bits. `false`
/// for an unknown name.
pub fn is_important(name: &str, style: &TuiStyle) -> bool {
    let name = &*canonical_property_name(name);
    if let Some(important) = inline_axis_important(name, style) {
        return important;
    }
    property_mask(name).is_some_and(|mask| !mask.is_empty() && style.important.contains(mask))
}
