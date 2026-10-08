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
/// A shorthand is representable only when every longhand of it is
/// set (`set_property("padding-top", "5")` alone lists
/// `padding-top: 5`), as in browsers (CSSOM §6.7.2).
pub(crate) fn css_text_of(style: &TuiStyle) -> String {
    let mut out = String::new();
    for &name in property_dispatch::property_names() {
        // A block-axis flow-relative property is its physical twin's
        // storage: listed under that name.
        if property_dispatch::is_storage_alias(name) {
            continue;
        }
        // Suppress longhand emission when its shorthand fires —
        // see the function-level docstring.
        if listed_under_shorthand(name, style) {
            continue;
        }
        if listed_under_logical_shorthand(name, style) {
            continue;
        }
        if listed_under_grid_shorthand(name, style) {
            continue;
        }
        if let Some(value) = property_dispatch::serialize(name, style) {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(name);
            out.push_str(": ");
            out.push_str(&value);
            if property_dispatch::is_important(name, style) {
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

/// True when the inline-axis longhand `name` is covered by an
/// inline-axis shorthand that serializes (`margin-inline-start` under a
/// set `margin-inline`): a declaration list names the shorthand once,
/// as it does a set `padding` (CSSOM §6.7.2 prefers the shorthand).
pub(crate) fn listed_under_logical_shorthand(name: &str, style: &TuiStyle) -> bool {
    let shorthands: &[&str] = match name {
        "margin-inline-start" | "margin-inline-end" => &["margin-inline"],
        "padding-inline-start" | "padding-inline-end" => &["padding-inline"],
        "inset-inline-start" | "inset-inline-end" => &["inset-inline"],
        "border-inline-start-color" => &["border-inline-start", "border-inline-color"],
        "border-inline-start-style" => &["border-inline-start", "border-inline-style"],
        "border-inline-start-width" => &["border-inline-start", "border-inline-width"],
        "border-inline-end-color" => &["border-inline-end", "border-inline-color"],
        "border-inline-end-style" => &["border-inline-end", "border-inline-style"],
        "border-inline-end-width" => &["border-inline-end", "border-inline-width"],
        _ => return false,
    };
    shorthands
        .iter()
        .any(|s| property_dispatch::serialize(s, style).is_some())
}

/// True when `name` — a grid longhand, or a grid shorthand inside a
/// larger one — is covered by a grid shorthand that serializes: the grid
/// shorthands nest (`grid` holds `grid-template`, CSS Grid 2 §7.8;
/// `grid-area` holds `grid-row` and `grid-column`, §8.4), and a
/// declaration list names the largest that serializes, once.
fn listed_under_grid_shorthand(name: &str, style: &TuiStyle) -> bool {
    let covering: &[&str] = match name {
        "grid-template-rows" | "grid-template-columns" | "grid-template-areas" => {
            &["grid", "grid-template"]
        }
        "grid-template" | "grid-auto-rows" | "grid-auto-columns" | "grid-auto-flow" => &["grid"],
        "grid-row-start" | "grid-row-end" => &["grid-area", "grid-row"],
        "grid-column-start" | "grid-column-end" => &["grid-area", "grid-column"],
        "grid-row" | "grid-column" => &["grid-area"],
        _ => return false,
    };
    covering
        .iter()
        .any(|s| property_dispatch::serialize(s, style).is_some())
}

/// True when the longhand `name` is covered by its shorthand family's
/// shorthand, which serializes for `style` (D-M4-2): `cssText` names the
/// shorthand once.
fn listed_under_shorthand(name: &str, style: &TuiStyle) -> bool {
    shorthand_family_of(name)
        .is_some_and(|shorthand| property_dispatch::serialize(shorthand, style).is_some())
}

/// Map a longhand name to its shorthand parent name. Used by
/// [`css_text_of`] to suppress longhand emission when the
/// shorthand form represents the same state. Returns `None` for
/// non-longhand names (including the shorthand names themselves
/// and standalone properties).
fn shorthand_family_of(name: &str) -> Option<&'static str> {
    match name {
        "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => Some("padding"),
        "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => Some("margin"),
        "row-gap" | "column-gap" => Some("gap"),
        "align-content" | "justify-content" => Some("place-content"),
        "align-items" | "justify-items" => Some("place-items"),
        "align-self" | "justify-self" => Some("place-self"),
        "top" | "right" | "bottom" | "left" => Some("inset"),
        "overflow-x" | "overflow-y" => Some("overflow"),
        "transition-property"
        | "transition-duration"
        | "transition-timing-function"
        | "transition-delay" => Some("transition"),
        "animation-name"
        | "animation-duration"
        | "animation-timing-function"
        | "animation-delay"
        | "animation-iteration-count"
        | "animation-direction"
        | "animation-fill-mode"
        | "animation-play-state"
        | "animation-composition"
        | "animation-timeline"
        | "animation-range-start"
        | "animation-range-end" => Some("animation"),
        "scroll-timeline-name" | "scroll-timeline-axis" => Some("scroll-timeline"),
        "view-timeline-name" | "view-timeline-axis" | "view-timeline-inset" => {
            Some("view-timeline")
        }
        _ => None,
    }
}
