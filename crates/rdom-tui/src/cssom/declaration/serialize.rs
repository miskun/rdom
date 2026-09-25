//! `cssText` serialization: the canonical-order declaration list
//! shared by [`super::StyleDeclaration::css_text`] and every
//! attribute write in [`super::StyleDeclarationMut`], including the
//! shorthand-suppresses-longhands rule (D-M4-2).

use rdom_style::TuiStyle;
use rdom_style::property_dispatch;

/// Serialize every set property on `style` as a CSSOM-style
/// declaration list: `"name: value [!important]; …"`. Iteration
/// follows [`property_dispatch::property_names`] order.
///
/// **Shorthand/longhand rule (D-M4-2):** when a shorthand family
/// has a representable shorthand form (i.e. `serialize("padding",
/// …)` returns `Some`), the four longhands in the family are
/// suppressed in cssText — the shorthand declaration round-trips
/// the same state through `set_css_text`, and emitting both
/// produces a duplicate-laden, lossy round-trip. The longhands
/// remain available via `get_property_value("padding-top")`; the
/// suppression is purely about cssText shape.
///
/// Per-family deviation note: `padding` storage is consolidated
/// (one `Padding` struct, not four `Option<u16>`), so
/// `set_property("padding-top", "5")` ends up representable as
/// `padding: 5 0 0 0` and emits the shorthand form. Browsers
/// preserve "only padding-top was set" via per-side independent
/// storage; rdom v1 normalizes to the shorthand. Round-trip stays
/// lossless.
pub(super) fn css_text_of(style: &TuiStyle) -> String {
    let mut out = String::new();
    for &name in property_dispatch::property_names() {
        // Suppress longhand emission when its shorthand fires —
        // see the function-level docstring.
        if let Some(shorthand) = shorthand_family_of(name)
            && property_dispatch::serialize(shorthand, style).is_some()
        {
            continue;
        }
        if let Some(value) = property_dispatch::serialize(name, style) {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(name);
            out.push_str(": ");
            out.push_str(&value);
            if let Some(mask) = property_dispatch::property_mask(name)
                && style.important.contains(mask)
            {
                out.push_str(" !important");
            }
            out.push(';');
        }
    }
    for d in &style.custom_properties {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str("--");
        out.push_str(&d.name);
        out.push_str(": ");
        out.push_str(&d.value);
        if d.important {
            out.push_str(" !important");
        }
        out.push(';');
    }
    out
}

/// Map a longhand name to its shorthand parent name. Used by
/// [`css_text_of`] to suppress longhand emission when the
/// shorthand form represents the same state. Returns `None` for
/// non-longhand names (including the shorthand names themselves
/// and standalone properties).
fn shorthand_family_of(name: &str) -> Option<&'static str> {
    match name {
        "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => Some("padding"),
        "top" | "right" | "bottom" | "left" => Some("inset"),
        "overflow-x" | "overflow-y" => Some("overflow"),
        "transition-property"
        | "transition-duration"
        | "transition-timing-function"
        | "transition-delay" => Some("transition"),
        _ => None,
    }
}
