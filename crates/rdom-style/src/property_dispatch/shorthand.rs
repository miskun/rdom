//! Which property names are shorthands: [`is_shorthand`], read off the
//! one property → storage-field table (`fields::fields_of`) rather than
//! kept as a second list.

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
    use super::is_shorthand;

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
