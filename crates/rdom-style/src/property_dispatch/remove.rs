//! CSSOM §6.6 `removeProperty` on a declaration block ([`remove`]): the
//! property's storage fields cleared and its `!important` bit dropped,
//! and the kept declarations (`var()` substitutions, flow-relative
//! properties) that set it removed or restricted.

use super::table::{canonical_property_name, fields_of, property_mask};
use crate::TuiStyle;

/// Clear the named property from `style` — reset its field(s) to
/// `None` and drop its `!important` bit. Returns `true` iff the
/// property was previously set (any of its fields was `Some`).
/// Returns `false` for unknown names.
///
/// A shorthand removes its longhands; a longhand (`padding-top`) its
/// own side only (CSSOM §6.6 `removeProperty`).
pub fn remove(name: &str, style: &mut TuiStyle) -> bool {
    if let Some(custom) = name.strip_prefix("--") {
        return style.remove_custom_property(custom);
    }
    let name = &*canonical_property_name(name);
    let Some(fields) = fields_of(name) else {
        return false;
    };
    // An inline-axis declaration writes no field of its block: removing
    // it leaves the physical declarations (and their bits) alone.
    if super::logical::is_directional(name) {
        let removed = super::logical::remove_inline_axis(name, style);
        drop_unneeded_pending(style);
        return removed;
    }
    let removed_kept = remove_kept_longhands(name, style);
    drop_unneeded_pending(style);
    // `|` not `||`: every field must be cleared, not just the first.
    let was_set = fields
        .iter()
        .fold(removed_kept, |acc, f| f.take(style) | acc);
    style.important = style
        .important
        .without(property_mask(name).unwrap_or_default());
    was_set
}

/// CSSOM §6.6 `removeProperty` of the physical property `name` among
/// the kept declarations: one that sets only `name`'s longhands (`name`
/// itself, or a longhand of it) goes; one that sets others too (a
/// shorthand of `name`) stays, restricted to those others
/// (`Restriction::Without`), so a substitution it waits for still
/// reaches them. A flow-relative declaration is a property of its own
/// (CSS Logical 1 §4: `margin-inline-start` is no longhand of `margin`)
/// and stays. Returns whether anything was removed.
fn remove_kept_longhands(name: &str, style: &mut TuiStyle) -> bool {
    use crate::var::Restriction;
    let gone = property_mask(name).unwrap_or_default();
    let mut removed = false;
    let mut kept = Vec::with_capacity(style.pending.len());
    for mut d in std::mem::take(&mut style.pending) {
        let own = property_mask(&d.name).unwrap_or_default();
        if d.directional || !own.intersects(gone) {
            kept.push(d);
            continue;
        }
        removed = true;
        if gone.contains(own) {
            continue;
        }
        match &mut d.restriction {
            Restriction::Without(names) => names.push(name.to_string()),
            other => *other = Restriction::Without(vec![name.to_string()]),
        }
        kept.push(d);
    }
    style.pending = kept;
    removed
}

/// Kept declarations are needed while one holds a substitution function
/// or an inline-axis property (the declarations after it keep their
/// order against it); without one, the block's fields say it all.
fn drop_unneeded_pending(style: &mut TuiStyle) {
    if !style
        .pending
        .iter()
        .any(|d| d.has_substitution || d.directional)
    {
        style.pending.clear();
    }
}
