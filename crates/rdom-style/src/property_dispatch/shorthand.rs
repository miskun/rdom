//! Which property names are shorthands: [`is_shorthand`], read off the
//! one property → storage-field table (`fields::fields_of`) rather than
//! kept as a second list — and [`overlaps`], which declarations of two
//! names set a common field.

use super::fields::fields_of;
use super::names::all_property_names;

/// Whether `name` is a shorthand: it sets several fields, and each of
/// them is also set by a narrower property — `padding` (its four sides),
/// `border` (widths, styles, colors), `overflow` (its two axes), `inset`,
/// `flex`, `background`. A longhand that sets fields no other property
/// does (`display`, with its derived inner fields) is not one. `false`
/// for an unknown name.
pub fn is_shorthand(name: &str) -> bool {
    // An inline-axis flow-relative property (`margin-inline-start`) owns
    // both physical sides' fields until the cascade picks one by
    // `direction`: a longhand, though its fields look like a shorthand's.
    !super::is_directional(name) && shorthands().contains(&name)
}

/// Every longhand name — [`property_names`](super::property_names) less
/// the shorthands and the storage aliases — in that order, found once.
pub fn longhand_names() -> &'static [&'static str] {
    static LONGHANDS: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    LONGHANDS.get_or_init(|| {
        super::property_names()
            .iter()
            .copied()
            .filter(|n| !super::is_storage_alias(n) && !is_shorthand(n))
            .collect()
    })
}

/// The shorthand names, found once from the field table.
/// Whether declarations of `a` and `b` set a common storage field, so the
/// later of the two in one block decides it — `margin` and `margin-top`,
/// `border` and `border-left-color`. Read off the same property →
/// field table as [`is_shorthand`]. A custom property, or a name the
/// table does not know, overlaps only itself. Names are matched ASCII
/// case-insensitively, as CSS property names are.
pub fn overlaps(a: &str, b: &str) -> bool {
    if a.eq_ignore_ascii_case(b) {
        return true;
    }
    let (a, b) = (a.to_ascii_lowercase(), b.to_ascii_lowercase());
    match (fields_of(&a), fields_of(&b)) {
        (Some(fa), Some(fb)) => fa.iter().any(|f| fb.contains(f)),
        _ => false,
    }
}

fn shorthands() -> &'static Vec<&'static str> {
    static SHORTHANDS: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    SHORTHANDS.get_or_init(|| all_property_names().filter(|n| owns_narrower(n)).collect())
}

/// Whether every field `name` sets is also set by a narrower name.
fn owns_narrower(name: &str) -> bool {
    let Some(own) = fields_of(name) else {
        return false;
    };
    if own.len() < 2 {
        return false;
    }
    own.iter().all(|f| {
        all_property_names().any(|other| {
            other != name && fields_of(other).is_some_and(|o| o.len() < own.len() && o.contains(f))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{is_shorthand, overlaps};

    #[test]
    fn overlapping_declarations_set_a_common_field() {
        for (a, b) in [
            ("margin", "margin-top"),
            ("margin-top", "margin"),
            ("border", "border-left-color"),
            ("background", "background-color"),
            ("color", "color"),
            ("--accent", "--accent"),
            ("nope", "nope"),
        ] {
            assert!(overlaps(a, b), "{a} / {b}");
        }
        for (a, b) in [
            ("margin", "padding-top"),
            ("color", "background-color"),
            ("--accent", "--other"),
            ("--color", "color"),
            ("nope", "color"),
        ] {
            assert!(!overlaps(a, b), "{a} / {b}");
        }
    }

    #[test]
    fn shorthands_are_names_whose_fields_narrower_names_set() {
        for name in [
            "padding",
            "margin",
            "border",
            "border-color",
            "overflow",
            "inset",
            "flex",
            "background",
        ] {
            assert!(is_shorthand(name), "{name}");
        }
        for name in [
            "color",
            "padding-top",
            "display",
            "background-color",
            "width",
            "nope",
        ] {
            assert!(!is_shorthand(name), "{name}");
        }
    }
}
