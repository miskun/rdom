//! The property names the dispatch table recognizes: the canonical
//! list ([`property_names`], the flow-relative names of `logical`
//! appended), the ASCII case folding every entry point applies
//! ([`canonical_property_name`]), and the names the `all` shorthand
//! covers ([`all_property_names`]). Which fields each name owns is
//! `table`.

/// Every CSS property name the dispatch table recognizes, in the
/// canonical order step 27's camelCase aliases iterate.
const PROPERTY_NAMES: &[&str] = &[
    // Color / text
    "color",
    "background-color",
    "background",
    "background-image",
    "background-position",
    "background-size",
    "background-repeat",
    "background-attachment",
    "background-origin",
    "background-clip",
    "font-weight",
    "font-style",
    "text-decoration",
    "opacity",
    // Layout — keywords
    "display",
    "flex-direction",
    "flex-wrap",
    "flex-flow",
    "justify-content",
    "align-content",
    "align-items",
    "align-self",
    "justify-items",
    "justify-self",
    "place-content",
    "place-items",
    "place-self",
    "white-space",
    "user-select",
    "pointer-events",
    "visibility",
    "caret-color",
    "caret-text-color",
    // Layout — overflow
    "overflow",
    "overflow-x",
    "overflow-y",
    "overflow-clip-margin",
    "text-overflow",
    "line-clamp",
    "max-lines",
    "block-ellipsis",
    "continue",
    "-webkit-line-clamp",
    "-webkit-box-orient",
    "scrollbar-gutter",
    "scrollbar-width",
    "scrollbar-color",
    "overscroll-behavior",
    "overscroll-behavior-x",
    "overscroll-behavior-y",
    "scroll-padding",
    "scroll-padding-top",
    "scroll-padding-right",
    "scroll-padding-bottom",
    "scroll-padding-left",
    "scroll-margin",
    "scroll-margin-top",
    "scroll-margin-right",
    "scroll-margin-bottom",
    "scroll-margin-left",
    "scroll-snap-type",
    "scroll-snap-align",
    "scroll-snap-stop",
    "scroll-behavior",
    // Layout — sizing
    "width",
    "height",
    "min-width",
    "max-width",
    "min-height",
    "max-height",
    "aspect-ratio",
    "box-sizing",
    "contain-intrinsic-size",
    "contain-intrinsic-width",
    "contain-intrinsic-height",
    "contain-intrinsic-inline-size",
    "contain-intrinsic-block-size",
    "gap",
    "row-gap",
    "column-gap",
    // Flex shorthand (sets width and height in one declaration).
    "flex",
    "flex-grow",
    "flex-shrink",
    "flex-basis",
    "order",
    // Grid (CSS Grid 2)
    "grid",
    "grid-template",
    "grid-template-columns",
    "grid-template-rows",
    "grid-template-areas",
    "grid-auto-columns",
    "grid-auto-rows",
    "grid-auto-flow",
    "grid-row-start",
    "grid-row-end",
    "grid-column-start",
    "grid-column-end",
    "grid-row",
    "grid-column",
    "grid-area",
    // Padding (shorthand + longhands)
    "padding",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    // Margin (shorthand + longhands)
    "margin",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "margin-trim",
    // Box decoration
    "border",
    "border-top",
    "border-right",
    "border-bottom",
    "border-left",
    "border-style",
    "border-top-style",
    "border-right-style",
    "border-bottom-style",
    "border-left-style",
    "border-color",
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
    "border-width",
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
    "border-radius",
    "border-top-left-radius",
    "border-top-right-radius",
    "border-bottom-right-radius",
    "border-bottom-left-radius",
    "box-shadow",
    "border-collapse",
    "border-spacing",
    "content",
    // Positioning (M2)
    "position",
    "top",
    "right",
    "bottom",
    "left",
    "z-index",
    "float",
    "clear",
    "inset",
    // Transitions (M3)
    "transition-property",
    "transition-duration",
    "transition-timing-function",
    "transition-delay",
    "transition",
    // Counters (CSS Lists 3)
    "counter-reset",
    "counter-increment",
    // Color adjustment (CSS Color Adjust 1)
    "color-scheme",
    // Text (CSS Text 3 / 4)
    "white-space-collapse",
    "text-wrap-mode",
    "word-break",
    "overflow-wrap",
    "word-wrap",
    "line-break",
    "hyphens",
    "tab-size",
    "text-transform",
    "text-indent",
    "text-align",
    "text-align-all",
    "text-align-last",
    "text-justify",
    "text-wrap",
    "text-wrap-style",
    // Writing modes (CSS Writing Modes 4)
    "direction",
    "writing-mode",
];

/// `name` as the table spells it. CSS property names are ASCII
/// case-insensitive (CSS Syntax 3 §5.4.4 matches a declaration's name
/// case-insensitively; CSSOM `setProperty` lowercases it), except
/// custom properties (`--*`), whose names are case-sensitive (CSS
/// Variables 1 §2). Borrows unless there is an uppercase letter to
/// fold. Every public dispatch entry point folds through this, so the
/// block parser and CSSOM agree.
pub fn canonical_property_name(name: &str) -> std::borrow::Cow<'_, str> {
    if name.starts_with("--") || !name.bytes().any(|b| b.is_ascii_uppercase()) {
        std::borrow::Cow::Borrowed(name)
    } else {
        std::borrow::Cow::Owned(name.to_ascii_lowercase())
    }
}

/// The properties the `all` shorthand leaves alone besides custom
/// properties (CSS Cascade 4 §3.2). Neither is in the table yet; listed
/// so they stay excluded when they land.
const ALL_EXCLUDES: &[&str] = &["direction", "unicode-bidi"];

/// The property names `all` sets: the whole table minus
/// [`ALL_EXCLUDES`], so a property added to the table is covered
/// without touching `all`.
pub(super) fn all_property_names() -> impl Iterator<Item = &'static str> {
    property_names()
        .iter()
        .copied()
        .filter(|n| !ALL_EXCLUDES.contains(n))
}

/// The full list of property names supported by the dispatch
/// table. Sorted by category, not alphabetic — step 27's iteration
/// preserves this order for stable camelCase output. The flow-relative
/// properties (CSS Logical 1, `logical.rs`) come last.
pub fn property_names() -> &'static [&'static str] {
    static NAMES: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    NAMES.get_or_init(|| {
        PROPERTY_NAMES
            .iter()
            .chain(super::logical::NAMES)
            .copied()
            .collect()
    })
}
