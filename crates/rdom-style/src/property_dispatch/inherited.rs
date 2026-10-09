//! The inherited-property set ([`inherits`]): which properties rdom's
//! cascade copies from parent to child, and what `unset` means for each.

use super::names::canonical_property_name;

/// Does rdom inherit this property by default? The one declaration of
/// the inherited set: `rdom-tui`'s cascade copies exactly these from
/// parent to child (pinned by a cascade test), and this table decides
/// what `unset` means (CSS Cascade 4 §7.3: `inherit` for
/// inherited properties, `initial` otherwise).
pub fn inherits(name: &str) -> bool {
    matches!(
        &*canonical_property_name(name),
        "color"
            | "font-weight"
            | "font-style"
            | "font"
            | "font-size"
            | "font-family"
            | "font-stretch"
            | "font-width"
            | "font-variant"
            | "white-space"
            | "white-space-collapse"
            | "text-wrap-mode"
            | "word-break"
            | "overflow-wrap"
            | "word-wrap"
            | "line-break"
            | "hyphens"
            | "tab-size"
            | "text-transform"
            | "text-indent"
            | "text-align"
            | "text-align-all"
            | "text-align-last"
            | "text-justify"
            | "text-wrap"
            | "text-wrap-style"
            | "letter-spacing"
            | "word-spacing"
            | "line-height"
            | "text-underline-offset"
            | "text-underline-position"
            | "text-decoration-skip-ink"
            | "pointer-events"
            | "visibility"
            | "caret-color"
            | "quotes"
            | "list-style"
            | "list-style-type"
            | "list-style-position"
            | "list-style-image"
            | "marker-side"
            | "interpolate-size"
            | "caret-text-color"
            | "color-scheme"
            | "border-spacing"
            | "caption-side"
            | "empty-cells"
            | "direction"
            | "writing-mode"
            | "block-ellipsis"
            | "scrollbar-color"
            | "cursor"
            | "caret-shape"
            | "caret-animation"
            | "caret"
            | "accent-color"
            // CSS Fragmentation 3 §3.3.
            | "orphans"
            | "widows"
    )
}
