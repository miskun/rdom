//! The property name ↔ storage-field table: which `TuiStyle` fields
//! each CSS property owns (`Field`, `fields_of`), the canonical
//! property-name list, and the table-driven operations that fold over
//! it — `property_mask` (`!important` bits), `remove`, and the
//! inherited-property set (`inherits`).

use super::css_wide::{CssWide, keyword_of};
use crate::TuiStyle;

/// Every CSS property name the dispatch table recognizes, in the
/// canonical order step 27's camelCase aliases iterate.
const PROPERTY_NAMES: &[&str] = &[
    // Color / text
    "color",
    "background-color",
    "background",
    "border-color",
    "font-weight",
    "font-style",
    "text-decoration",
    "opacity",
    // Layout — keywords
    "display",
    "flex-direction",
    "white-space",
    "user-select",
    "pointer-events",
    "caret-color",
    "caret-text-color",
    // Layout — overflow
    "overflow",
    "overflow-x",
    "overflow-y",
    "scrollbar-gutter",
    // Layout — sizing
    "width",
    "height",
    "min-width",
    "max-width",
    "min-height",
    "max-height",
    "aspect-ratio",
    "gap",
    // Flex shorthand (sets width and height in one declaration).
    "flex",
    "flex-shrink",
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
    "border-collapse",
    "content",
    // Positioning (M2)
    "position",
    "top",
    "right",
    "bottom",
    "left",
    "z-index",
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
];

/// The full list of property names supported by the dispatch
/// table. Sorted by category, not alphabetic — step 27's iteration
/// preserves this order for stable camelCase output.
pub fn property_names() -> &'static [&'static str] {
    PROPERTY_NAMES
}

macro_rules! define_fields {
    ($($variant:ident => $field:ident : $mask:ident,)+) => {
        /// One storage field of `TuiStyle`, as a row of the property →
        /// field table. `!important` routing, `removeProperty`, the
        /// CSS-wide keywords and their serialization all fold over
        /// [`fields_of`], so "which fields does this property own" is
        /// spelled once (`STYLE-PROPERTY-TABLES-1`).
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(super) enum Field {
            $($variant,)+
        }

        impl Field {
            /// Every field, for coverage tests.
            #[cfg(test)]
            pub(super) const ALL: &'static [Field] = &[$(Field::$variant,)+];

            /// The `!important` bit this field is guarded by.
            pub(super) fn mask(self) -> crate::ImportantMask {
                match self {
                    $(Field::$variant => crate::ImportantMask::$mask,)+
                }
            }

            /// Clear the field; `true` if it was set.
            fn take(self, style: &mut TuiStyle) -> bool {
                match self {
                    $(Field::$variant => style.$field.take().is_some(),)+
                }
            }

            /// Store a CSS-wide keyword (resolved for `name`).
            pub(super) fn put_css_wide(self, style: &mut TuiStyle, kw: CssWide, name: &str) {
                match self {
                    $(Field::$variant => style.$field = Some(kw.into_value(name)),)+
                }
            }

            /// The CSS-wide keyword the field holds, if any.
            pub(super) fn css_wide(self, style: &TuiStyle) -> Option<&'static str> {
                match self {
                    $(Field::$variant => keyword_of(&style.$field),)+
                }
            }
        }
    };
}

define_fields! {
    Fg => fg : FG,
    Bg => bg : BG,
    BorderFg => border_fg : BORDER_FG,
    Bold => bold : BOLD,
    Italic => italic : ITALIC,
    TextDecoration => text_decoration : TEXT_DECORATION,
    Opacity => opacity : OPACITY,
    Display => display : DISPLAY,
    Flow => flow : FLOW,
    Direction => direction : DIRECTION,
    WhiteSpace => white_space : WHITE_SPACE,
    UserSelect => user_select : USER_SELECT,
    PointerEvents => pointer_events : POINTER_EVENTS,
    CaretColor => caret_color : CARET_COLOR,
    CaretTextColor => caret_text_color : CARET_TEXT_COLOR,
    OverflowX => overflow_x : OVERFLOW_X,
    OverflowY => overflow_y : OVERFLOW_Y,
    ScrollbarGutter => scrollbar_gutter : SCROLLBAR_GUTTER,
    Width => width : WIDTH,
    Height => height : HEIGHT,
    MinWidth => min_width : MIN_WIDTH,
    MaxWidth => max_width : MAX_WIDTH,
    MinHeight => min_height : MIN_HEIGHT,
    MaxHeight => max_height : MAX_HEIGHT,
    AspectRatio => aspect_ratio : ASPECT_RATIO,
    Gap => gap : GAP,
    FlexShrink => flex_shrink : FLEX_SHRINK,
    Padding => padding : PADDING,
    Margin => margin : MARGIN,
    Border => border : BORDER,
    BorderCollapse => border_collapse : BORDER_COLLAPSE,
    Content => content : CONTENT,
    Position => position : POSITION,
    Top => top : TOP,
    Right => right : RIGHT,
    Bottom => bottom : BOTTOM,
    Left => left : LEFT,
    ZIndex => z_index : Z_INDEX,
    TransitionProperty => transition_property : TRANSITIONS,
    TransitionDuration => transition_duration : TRANSITIONS,
    TransitionTimingFunction => transition_timing_function : TRANSITIONS,
    TransitionDelay => transition_delay : TRANSITIONS,
    CounterReset => counter_reset : COUNTER_RESET,
    CounterIncrement => counter_increment : COUNTER_INCREMENT,
}

/// The fields a property name owns — the one property → field table.
/// Shorthands own several (`overflow` → X + Y, `inset` → the four
/// sides); per-side `padding-*` / `margin-*` / `border-*` longhands
/// share the shorthand's single field. `display` owns the derived
/// `flow` too, so removing or `inherit`ing `display` cannot leave a
/// stale flow behind. `None` for unknown names.
pub(super) fn fields_of(name: &str) -> Option<&'static [Field]> {
    use Field::*;
    Some(match name {
        "color" => &[Fg],
        "background-color" | "background" => &[Bg],
        "border-color" => &[BorderFg],
        "font-weight" => &[Bold],
        "font-style" => &[Italic],
        "text-decoration" => &[TextDecoration],
        "opacity" => &[Opacity],
        "display" => &[Display, Flow],
        "flex-direction" => &[Direction],
        "white-space" => &[WhiteSpace],
        "user-select" => &[UserSelect],
        "pointer-events" => &[PointerEvents],
        "caret-color" => &[CaretColor],
        "caret-text-color" => &[CaretTextColor],
        "overflow" => &[OverflowX, OverflowY],
        "overflow-x" => &[OverflowX],
        "overflow-y" => &[OverflowY],
        "scrollbar-gutter" => &[ScrollbarGutter],
        "width" => &[Width],
        "height" => &[Height],
        "min-width" => &[MinWidth],
        "max-width" => &[MaxWidth],
        "min-height" => &[MinHeight],
        "max-height" => &[MaxHeight],
        "aspect-ratio" => &[AspectRatio],
        "gap" => &[Gap],
        "flex" => &[Width, Height, FlexShrink],
        "flex-shrink" => &[FlexShrink],
        "padding" | "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => {
            &[Padding]
        }
        "margin" | "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => &[Margin],
        "border"
        | "border-top"
        | "border-right"
        | "border-bottom"
        | "border-left"
        | "border-style"
        | "border-top-style"
        | "border-right-style"
        | "border-bottom-style"
        | "border-left-style" => &[Border],
        "border-collapse" => &[BorderCollapse],
        "content" => &[Content],
        "position" => &[Position],
        "top" => &[Top],
        "right" => &[Right],
        "bottom" => &[Bottom],
        "left" => &[Left],
        "z-index" => &[ZIndex],
        "inset" => &[Top, Right, Bottom, Left],
        "transition-property" => &[TransitionProperty],
        "transition-duration" => &[TransitionDuration],
        "transition-timing-function" => &[TransitionTimingFunction],
        "transition-delay" => &[TransitionDelay],
        "transition" => &[
            TransitionProperty,
            TransitionDuration,
            TransitionTimingFunction,
            TransitionDelay,
        ],
        "counter-reset" => &[CounterReset],
        "counter-increment" => &[CounterIncrement],
        _ => return None,
    })
}

/// Map a property name to the [`ImportantMask`] bit(s) it owns: the OR
/// of every field's bit. Returns `None` for unknown names.
///
/// Two consumers: the `rdom-css` block parser's `!important`
/// routing, and `rdom-tui`'s `StyleDeclaration::set_property_
/// important` / `get_property_priority`.
pub fn property_mask(name: &str) -> Option<crate::ImportantMask> {
    let fields = fields_of(name)?;
    Some(
        fields
            .iter()
            .fold(crate::ImportantMask::empty(), |m, f| m | f.mask()),
    )
}

/// Clear the named property from `style` — reset its field(s) to
/// `None` and drop its `!important` bit. Returns `true` iff the
/// property was previously set (any of its fields was `Some`).
/// Returns `false` for unknown names.
///
/// Per-side longhands (`padding-top`, …) share the shorthand's storage,
/// so removing any of them clears the whole thing — the same way CSSOM
/// `removeProperty("padding-top")` clears the entry.
pub fn remove(name: &str, style: &mut TuiStyle) -> bool {
    if let Some(custom) = name.strip_prefix("--") {
        return style.remove_custom_property(custom);
    }
    let Some(fields) = fields_of(name) else {
        return false;
    };
    // `|` not `||`: every field must be cleared, not just the first.
    let was_set = fields.iter().fold(false, |acc, f| f.take(style) | acc);
    style.important = style
        .important
        .without(property_mask(name).unwrap_or_default());
    was_set
}

/// Does rdom inherit this property by default? The one declaration of
/// the inherited set: `rdom-tui`'s cascade copies exactly these from
/// parent to child (pinned by a cascade test), and this table decides
/// what `unset` means (CSS Cascade 4 §7.3: `inherit` for
/// inherited properties, `initial` otherwise).
pub fn inherits(name: &str) -> bool {
    matches!(
        name,
        "color"
            | "font-weight"
            | "font-style"
            | "white-space"
            | "pointer-events"
            | "caret-color"
            | "caret-text-color"
    )
}
