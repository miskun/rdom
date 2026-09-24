//! Property dispatch table — single source for the
//! `name → (setter, serializer)` mapping that drives:
//!
//! - [`crate::declarations::apply_declaration`] (block parser)
//! - `rdom_tui::cssom::StyleDeclaration` (step 26)
//!
//! ## Why this exists
//!
//! Before M4b step 25 the dispatch table lived as a big `match`
//! inside `declarations::apply_declaration`. Step 26's
//! `StyleDeclaration::set_property` needs the same name→setter
//! routing, and duplicating ~150 lines of match across crates was
//! the M4b architect-pass risk that prompted this extraction.
//!
//! ## Surface
//!
//! - [`set`] takes a property name + value-string and writes to a
//!   `TuiStyle`. Returns [`DispatchError`] for unknown names /
//!   invalid values.
//! - [`set_from_tokens`] is the same function pre-tokenized — the
//!   block parser uses this so it doesn't re-tokenize per
//!   declaration.
//! - [`serialize`] emits the CSS string form for whichever value
//!   is currently stored under `name` on `style`. `None` means the
//!   property is unset (caller maps to `""` per CSSOM
//!   `getPropertyValue` convention).
//! - [`property_names`] is the sorted list of every property name
//!   the dispatch table knows about — drives camelCase alias
//!   generation in step 27 and `length` / `item` in step 26.
//!
//! ## Round-trip contract
//!
//! For every name in [`property_names`], the following must hold
//! for at least one canonical value `v`:
//!
//! ```text
//! let mut style = TuiStyle::new();
//! property_dispatch::set(name, v, &mut style)?;
//! let serialized = property_dispatch::serialize(name, &style).unwrap();
//! let mut roundtrip = TuiStyle::new();
//! property_dispatch::set(name, &serialized, &mut roundtrip)?;
//! assert_eq!(style.<field>, roundtrip.<field>);
//! ```
//!
//! The `round_trip_every_property` integration test in this module
//! enforces this for the full table.

use crate::layout::{
    CaretColor, CaretTextColor, Direction, Display, Length, Overflow, Position, Size, UserSelect,
    WhiteSpace, ZIndex,
};
use crate::parse::token::{Token, tokenize};
use crate::parse::values::{
    current_border, current_margin, current_padding, parse_aspect_ratio, parse_border,
    parse_border_side, parse_color, parse_content, parse_flex_shorthand, parse_inset_shorthand,
    parse_keyword, parse_length, parse_margin_longhand, parse_margin_shorthand, parse_min_size,
    parse_opacity, parse_overflow, parse_padding_shorthand, parse_padding_value, parse_position,
    parse_scrollbar_gutter, parse_size, parse_text_decoration, parse_time_list,
    parse_timing_function_list, parse_transition_property_list, parse_transition_shorthand,
    parse_unsigned, parse_z_index, unzip_transition_rules,
};
use crate::transition::{TimingFunction, TransitionProperty};
use crate::{Color, Content, TuiColor, TuiStyle, Value};

/// Reason `set` / `set_from_tokens` rejected a declaration.
///
/// The block parser maps these onto its existing `Warning`
/// variants; CSSOM call sites (step 26) typically silently
/// no-op (browser-faithful — `element.style.bogus = 'x'`
/// doesn't throw).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    /// `name` isn't in the dispatch table.
    UnknownProperty,
    /// `name` is known but `value` failed to parse.
    InvalidValue,
}

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
        enum Field {
            $($variant,)+
        }

        impl Field {
            /// Every field, for coverage tests.
            #[cfg(test)]
            const ALL: &'static [Field] = &[$(Field::$variant,)+];

            /// The `!important` bit this field is guarded by.
            fn mask(self) -> crate::ImportantMask {
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
            fn put_css_wide(self, style: &mut TuiStyle, kw: CssWide, name: &str) {
                match self {
                    $(Field::$variant => style.$field = Some(kw.into_value(name)),)+
                }
            }

            /// The CSS-wide keyword the field holds, if any.
            fn css_wide(self, style: &TuiStyle) -> Option<&'static str> {
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
}

/// The fields a property name owns — the one property → field table.
/// Shorthands own several (`overflow` → X + Y, `inset` → the four
/// sides); per-side `padding-*` / `margin-*` / `border-*` longhands
/// share the shorthand's single field. `display` owns the derived
/// `flow` too, so removing or `inherit`ing `display` cannot leave a
/// stale flow behind. `None` for unknown names.
fn fields_of(name: &str) -> Option<&'static [Field]> {
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

/// The CSS-wide keyword a field holds, if any.
fn keyword_of<T>(v: &Option<Value<T>>) -> Option<&'static str> {
    match v {
        Some(Value::Inherit) => Some("inherit"),
        Some(Value::Initial) => Some("initial"),
        _ => None,
    }
}

/// Does rdom inherit this property by default? Mirrors the cascade's
/// inherited set (`rdom-tui`'s `INHERITS_MASK`): the properties whose
/// computed value flows parent → child when the child declares nothing.
/// Decides what `unset` means (CSS Cascade 4 §7.3: `inherit` for
/// inherited properties, `initial` otherwise).
pub fn inherits(name: &str) -> bool {
    matches!(
        name,
        "color" | "font-weight" | "font-style" | "white-space" | "user-select" | "pointer-events"
    )
}

/// The CSS-wide keywords (CSS Cascade 4 §7). Valid for every property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CssWide {
    Inherit,
    Initial,
    Unset,
}

fn css_wide_keyword(value: &[Token]) -> Option<CssWide> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("inherit") => Some(CssWide::Inherit),
        [Token::Ident(s)] if s.eq_ignore_ascii_case("initial") => Some(CssWide::Initial),
        [Token::Ident(s)] if s.eq_ignore_ascii_case("unset") => Some(CssWide::Unset),
        _ => None,
    }
}

impl CssWide {
    /// Resolve `unset` for `name`, then build the field value.
    fn into_value<T>(self, name: &str) -> Value<T> {
        match self {
            CssWide::Inherit => Value::Inherit,
            CssWide::Initial => Value::Initial,
            CssWide::Unset => {
                if inherits(name) {
                    Value::Inherit
                } else {
                    Value::Initial
                }
            }
        }
    }
}

/// Set every field `name` owns to the CSS-wide keyword.
fn set_css_wide(name: &str, kw: CssWide, style: &mut TuiStyle) -> Result<(), DispatchError> {
    let fields = fields_of(name).ok_or(DispatchError::UnknownProperty)?;
    for f in fields {
        f.put_css_wide(style, kw, name);
    }
    Ok(())
}

/// If every field `name` owns holds the *same* CSS-wide keyword, its
/// spelling; otherwise `None` and the per-field serializer decides.
/// Shorthands over mixed fields (`overflow-x: inherit; overflow-y:
/// hidden`) therefore never claim `inherit` for the whole shorthand.
fn css_wide_of(name: &str, style: &TuiStyle) -> Option<&'static str> {
    let fields = fields_of(name)?;
    let first = fields.first()?.css_wide(style)?;
    fields
        .iter()
        .all(|f| f.css_wide(style) == Some(first))
        .then_some(first)
}

/// Set `name = value` on `style`. Tokenizes the value first; for
/// callers that already have tokens, prefer [`set_from_tokens`].
pub fn set(name: &str, value: &str, style: &mut TuiStyle) -> Result<(), DispatchError> {
    let tokens = tokenize(value).map_err(|_| DispatchError::InvalidValue)?;
    set_from_tokens(name, &tokens, style)
}

/// Pre-tokenized variant of [`set`]. The block parser in
/// `rdom-css` calls this to avoid re-tokenizing each declaration's
/// value when the surrounding block was already tokenized.
pub fn set_from_tokens(
    name: &str,
    value: &[Token],
    style: &mut TuiStyle,
) -> Result<(), DispatchError> {
    if let Some(kw) = css_wide_keyword(value) {
        return set_css_wide(name, kw, style);
    }
    let outcome: Option<()> = match name {
        // Color / modifiers
        "color" => parse_color(value).map(|c| {
            style.fg = Some(Value::Specified(c));
        }),
        // `background` shorthand: only the color component exists in a
        // cell grid (no images, positions, or repeat), so a lone color
        // is `background-color` and anything else is invalid.
        "background-color" | "background" => parse_color(value).map(|c| {
            style.bg = Some(Value::Specified(c));
        }),
        "border-color" => parse_color(value).map(|c| {
            style.border_fg = Some(Value::Specified(c));
        }),
        "font-weight" => parse_keyword(value, &[("bold", true), ("normal", false)]).map(|v| {
            style.bold = Some(Value::Specified(v));
        }),
        "font-style" => parse_keyword(value, &[("italic", true), ("normal", false)]).map(|v| {
            style.italic = Some(Value::Specified(v));
        }),
        "text-decoration" => parse_text_decoration(value).map(|(under, strike)| {
            // Map the boolean pair to the `TextDecoration` enum.
            // `(false, false)` → None; `(true, _)` → Underline;
            // `(false, true)` → LineThrough. Mutually exclusive in
            // 0.1.0 (single-axis representation); future CSS-shorthand
            // `text-decoration: underline line-through` would need
            // both bits, deferred to 0.2.x.
            let td = if strike {
                crate::layout::TextDecoration::LineThrough
            } else if under {
                crate::layout::TextDecoration::Underline
            } else {
                crate::layout::TextDecoration::None
            };
            style.text_decoration = Some(Value::Specified(td));
        }),
        "opacity" => parse_opacity(value).map(|v| {
            style.opacity = Some(Value::Specified(v));
        }),

        // Layout — keywords
        //
        // `display` writes BOTH outer (`Display`) and inner (`Flow`)
        // values per CSS3 Display Module. The single-value forms map:
        //  `block`        → Block + flow:Block
        //  `flex`         → Block + flow:Flex   (most common)
        //  `inline`       → Inline + (flow N/A)
        //  `inline-block` → InlineBlock + flow:Block
        //  `inline-flex`  → Inline + flow:Flex
        //  `none`         → None
        // The Flow side overwrites any prior author `flow` write —
        // matches CSS expectation that `display: flex` makes the
        // element a flex container regardless of any other prop.
        "display" => match value {
            [Token::Ident(s)] if s.eq_ignore_ascii_case("block") => {
                style.display = Some(Value::Specified(Display::Block));
                style.flow = Some(Value::Specified(crate::layout::Flow::Block));
                Some(())
            }
            [Token::Ident(s)] if s.eq_ignore_ascii_case("flex") => {
                style.display = Some(Value::Specified(Display::Block));
                style.flow = Some(Value::Specified(crate::layout::Flow::Flex));
                Some(())
            }
            [Token::Ident(s)] if s.eq_ignore_ascii_case("inline") => {
                style.display = Some(Value::Specified(Display::Inline));
                Some(())
            }
            [Token::Ident(s)] if s.eq_ignore_ascii_case("inline-block") => {
                style.display = Some(Value::Specified(Display::InlineBlock));
                style.flow = Some(Value::Specified(crate::layout::Flow::Block));
                Some(())
            }
            [Token::Ident(s)] if s.eq_ignore_ascii_case("inline-flex") => {
                style.display = Some(Value::Specified(Display::Inline));
                style.flow = Some(Value::Specified(crate::layout::Flow::Flex));
                Some(())
            }
            [Token::Ident(s)] if s.eq_ignore_ascii_case("none") => {
                style.display = Some(Value::Specified(Display::None));
                Some(())
            }
            _ => None,
        },
        "flex-direction" => parse_keyword(
            value,
            &[("row", Direction::Row), ("column", Direction::Column)],
        )
        .map(|d| {
            style.direction = Some(Value::Specified(d));
        }),
        "white-space" => parse_keyword(
            value,
            &[
                ("normal", WhiteSpace::Normal),
                ("pre", WhiteSpace::Pre),
                ("pre-wrap", WhiteSpace::PreWrap),
                ("nowrap", WhiteSpace::NoWrap),
            ],
        )
        .map(|w| {
            style.white_space = Some(Value::Specified(w));
        }),
        "user-select" => parse_keyword(
            value,
            &[
                ("auto", UserSelect::Auto),
                ("text", UserSelect::Text),
                ("none", UserSelect::None),
                ("all", UserSelect::All),
                ("contain", UserSelect::Contain),
            ],
        )
        .map(|u| {
            style.user_select = Some(Value::Specified(u));
        }),
        "pointer-events" => parse_keyword(
            value,
            &[
                ("auto", crate::layout::PointerEvents::Auto),
                ("none", crate::layout::PointerEvents::None),
            ],
        )
        .map(|v| {
            style.pointer_events = Some(Value::Specified(v));
        }),
        // `caret-color: auto | transparent | <color>`. Auto = caret
        // bg matches the underlying cell's fg (classic swap visual).
        // Transparent suppresses the caret paint entirely. A color
        // value paints the caret cell's bg with that color.
        "caret-color" => parse_keyword(
            value,
            &[
                ("auto", CaretColor::Auto),
                ("transparent", CaretColor::Transparent),
            ],
        )
        .or_else(|| parse_color(value).map(CaretColor::Color))
        .map(|c| {
            style.caret_color = Some(Value::Specified(c));
        }),
        // rdom-extension `caret-text-color: auto | <color>`. Auto =
        // glyph color matches the underlying cell's bg (classic
        // swap visual). A color value paints the caret cell's fg.
        "caret-text-color" => parse_keyword(value, &[("auto", CaretTextColor::Auto)])
            .or_else(|| parse_color(value).map(CaretTextColor::Color))
            .map(|c| {
                style.caret_text_color = Some(Value::Specified(c));
            }),

        // Layout — overflow
        "overflow" => parse_overflow(value).map(|o| {
            style.overflow_x = Some(Value::Specified(o));
            style.overflow_y = Some(Value::Specified(o));
        }),
        "overflow-x" => parse_overflow(value).map(|o| {
            style.overflow_x = Some(Value::Specified(o));
        }),
        "overflow-y" => parse_overflow(value).map(|o| {
            style.overflow_y = Some(Value::Specified(o));
        }),
        "scrollbar-gutter" => parse_scrollbar_gutter(value).map(|g| {
            style.scrollbar_gutter = Some(Value::Specified(g));
        }),

        // Layout — sizing
        "width" => parse_size(value).map(|s| {
            style.width = Some(Value::Specified(s));
        }),
        "height" => parse_size(value).map(|s| {
            style.height = Some(Value::Specified(s));
        }),
        "min-width" => parse_min_size(value).map(|m| {
            style.min_width = Some(Value::Specified(m));
        }),
        "max-width" => parse_unsigned(value).map(|n| {
            style.max_width = Some(Value::Specified(n));
        }),
        "min-height" => parse_min_size(value).map(|m| {
            style.min_height = Some(Value::Specified(m));
        }),
        "max-height" => parse_unsigned(value).map(|n| {
            style.max_height = Some(Value::Specified(n));
        }),
        "aspect-ratio" => parse_aspect_ratio(value).map(|r| {
            style.aspect_ratio = Some(Value::Specified(r));
        }),

        // Layout — gap
        "gap" => parse_unsigned(value).map(|n| {
            style.gap = Some(Value::Specified(n));
        }),

        // Flex shorthand — sets `width` + `height` AND `flex-shrink`.
        // Per CSS spec: `flex: <n>` ≡ `<n> 1 0` (grow=n, shrink=1,
        // basis=0); `flex: none` ≡ `0 0 auto` (no grow, NO shrink,
        // basis=auto). Cross-axis `Size::Flex` reads as "stretch to
        // container" in the layout pass, matching CSS default
        // `align-items: stretch`.
        "flex" => parse_flex_shorthand(value).map(|s| {
            // `flex: none` ⇒ Size::Auto with flex_shrink=0. All
            // other shapes use the CSS-default shrink=1.
            let shrink = match &s {
                Size::Auto => 0,
                _ => 1,
            };
            style.width = Some(Value::Specified(s.clone()));
            style.height = Some(Value::Specified(s));
            style.flex_shrink = Some(Value::Specified(shrink));
        }),
        "flex-shrink" => parse_unsigned(value).map(|n| {
            style.flex_shrink = Some(Value::Specified(n));
        }),

        // Padding shorthand + longhands
        "padding" => parse_padding_shorthand(value).map(|p| {
            style.padding = Some(Value::Specified(p));
        }),
        "padding-top" => parse_padding_value(value).map(|v| {
            let mut p = current_padding(style);
            p.top = v;
            style.padding = Some(Value::Specified(p));
        }),
        "padding-right" => parse_padding_value(value).map(|v| {
            let mut p = current_padding(style);
            p.right = v;
            style.padding = Some(Value::Specified(p));
        }),
        "padding-bottom" => parse_padding_value(value).map(|v| {
            let mut p = current_padding(style);
            p.bottom = v;
            style.padding = Some(Value::Specified(p));
        }),
        "padding-left" => parse_padding_value(value).map(|v| {
            let mut p = current_padding(style);
            p.left = v;
            style.padding = Some(Value::Specified(p));
        }),

        // Margin shorthand + longhands
        "margin" => parse_margin_shorthand(value).map(|m| {
            style.margin = Some(Value::Specified(m));
        }),
        "margin-top" => parse_margin_longhand(value).map(|v| {
            let mut m = current_margin(style);
            m.top = v;
            style.margin = Some(Value::Specified(m));
        }),
        "margin-right" => parse_margin_longhand(value).map(|v| {
            let mut m = current_margin(style);
            m.right = v;
            style.margin = Some(Value::Specified(m));
        }),
        "margin-bottom" => parse_margin_longhand(value).map(|v| {
            let mut m = current_margin(style);
            m.bottom = v;
            style.margin = Some(Value::Specified(m));
        }),
        "margin-left" => parse_margin_longhand(value).map(|v| {
            let mut m = current_margin(style);
            m.left = v;
            style.margin = Some(Value::Specified(m));
        }),

        // Border shorthand + per-side longhands. Per-side
        // longhands READ the current border on `style` and
        // MERGE — so `border: solid; border-top: none` correctly
        // clears just the top side and keeps R/B/L.
        "border" => parse_border(value).map(|b| {
            style.border = Some(Value::Specified(b));
        }),
        "border-top" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.top = bs;
            style.border = Some(Value::Specified(b));
        }),
        "border-right" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.right = bs;
            style.border = Some(Value::Specified(b));
        }),
        "border-bottom" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.bottom = bs;
            style.border = Some(Value::Specified(b));
        }),
        "border-left" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.left = bs;
            style.border = Some(Value::Specified(b));
        }),
        // CSS `border-style: <style>` — sets all four sides to one
        // style. `border-style: hidden` is the conflict kill-switch
        // applied uniformly.
        "border-style" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.top = bs;
            b.right = bs;
            b.bottom = bs;
            b.left = bs;
            style.border = Some(Value::Specified(b));
        }),
        // CSS per-side `*-style` longhands. Same merge semantics as
        // `border-top` / etc.; useful when authors want to flip just
        // one side's style without touching color (when color lands).
        "border-top-style" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.top = bs;
            style.border = Some(Value::Specified(b));
        }),
        "border-right-style" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.right = bs;
            style.border = Some(Value::Specified(b));
        }),
        "border-bottom-style" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.bottom = bs;
            style.border = Some(Value::Specified(b));
        }),
        "border-left-style" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.left = bs;
            style.border = Some(Value::Specified(b));
        }),
        "border-collapse" => parse_keyword(
            value,
            &[
                ("separate", crate::layout::BorderCollapse::Separate),
                ("collapse", crate::layout::BorderCollapse::Collapse),
            ],
        )
        .map(|v| {
            style.border_collapse = Some(Value::Specified(v));
        }),

        // Pseudo-element content
        "content" => parse_content(value).map(|c| {
            style.content = Some(Value::Specified(c));
        }),

        // Positioning (M2)
        "position" => parse_position(value).map(|p| {
            style.position = Some(Value::Specified(p));
        }),
        "top" => parse_length(value).map(|l| {
            style.top = Some(Value::Specified(l));
        }),
        "right" => parse_length(value).map(|l| {
            style.right = Some(Value::Specified(l));
        }),
        "bottom" => parse_length(value).map(|l| {
            style.bottom = Some(Value::Specified(l));
        }),
        "left" => parse_length(value).map(|l| {
            style.left = Some(Value::Specified(l));
        }),
        "z-index" => parse_z_index(value).map(|z| {
            style.z_index = Some(Value::Specified(z));
        }),
        "inset" => parse_inset_shorthand(value).map(|(t, r, b, l)| {
            style.top = Some(Value::Specified(t));
            style.right = Some(Value::Specified(r));
            style.bottom = Some(Value::Specified(b));
            style.left = Some(Value::Specified(l));
        }),

        // Transitions (M3)
        "transition-property" => parse_transition_property_list(value).map(|list| {
            style.transition_property = Some(Value::Specified(list));
        }),
        "transition-duration" => parse_time_list(value).map(|list| {
            style.transition_duration = Some(Value::Specified(list));
        }),
        "transition-timing-function" => parse_timing_function_list(value).map(|list| {
            style.transition_timing_function = Some(Value::Specified(list));
        }),
        "transition-delay" => parse_time_list(value).map(|list| {
            style.transition_delay = Some(Value::Specified(list));
        }),
        "transition" => parse_transition_shorthand(value).map(|rules| {
            let (props, durs, timings, delays) = unzip_transition_rules(&rules);
            style.transition_property = Some(Value::Specified(props));
            style.transition_duration = Some(Value::Specified(durs));
            style.transition_timing_function = Some(Value::Specified(timings));
            style.transition_delay = Some(Value::Specified(delays));
        }),

        _ => return Err(DispatchError::UnknownProperty),
    };

    outcome.ok_or(DispatchError::InvalidValue).map(|_| ())
}

/// Serialize the named property's current value as a CSS string.
/// Returns `None` when the property isn't currently set — callers
/// map this to `""` to match `getPropertyValue`.
///
/// Unknown property names also return `None` (rather than
/// errorring); CSSOM `getPropertyValue("bogus")` returns `""` too.
pub fn serialize(name: &str, style: &TuiStyle) -> Option<String> {
    if let Some(kw) = css_wide_of(name, style) {
        return Some(kw.to_string());
    }
    match name {
        // Color / modifiers
        "color" => style.fg.as_ref().and_then(specified).map(serialize_color),
        "background-color" | "background" => {
            style.bg.as_ref().and_then(specified).map(serialize_color)
        }
        "border-color" => style
            .border_fg
            .as_ref()
            .and_then(specified)
            .map(serialize_color),
        "font-weight" => style.bold.as_ref().and_then(specified).map(|b| {
            if *b {
                "bold".to_string()
            } else {
                "normal".to_string()
            }
        }),
        "font-style" => style.italic.as_ref().and_then(specified).map(|b| {
            if *b {
                "italic".to_string()
            } else {
                "normal".to_string()
            }
        }),
        "text-decoration" => style
            .text_decoration
            .as_ref()
            .and_then(specified)
            .map(|td| {
                match td {
                    crate::layout::TextDecoration::None => "none",
                    crate::layout::TextDecoration::Underline => "underline",
                    crate::layout::TextDecoration::LineThrough => "line-through",
                }
                .to_string()
            }),
        "opacity" => style.opacity.as_ref().and_then(specified).map(|v| {
            // Drop trailing zeros for the common cases — `1.0` →
            // `"1"`, `0.5` → `"0.5"`, `0.0` → `"0"`. Matches
            // browser CSSOM serialization for `getPropertyValue`.
            if *v == 1.0 {
                "1".to_string()
            } else if *v == 0.0 {
                "0".to_string()
            } else {
                format!("{v}")
            }
        }),

        // Layout — keywords
        "display" => style.display.as_ref().and_then(specified).map(|d| {
            match d {
                Display::Block => "block",
                Display::Inline => "inline",
                Display::InlineBlock => "inline-block",
                Display::None => "none",
            }
            .to_string()
        }),
        "flex-direction" => style.direction.as_ref().and_then(specified).map(|d| {
            match d {
                Direction::Row => "row",
                Direction::Column => "column",
            }
            .to_string()
        }),
        "white-space" => style.white_space.as_ref().and_then(specified).map(|w| {
            match w {
                WhiteSpace::Normal => "normal",
                WhiteSpace::Pre => "pre",
                WhiteSpace::PreWrap => "pre-wrap",
                WhiteSpace::NoWrap => "nowrap",
            }
            .to_string()
        }),
        "user-select" => style.user_select.as_ref().and_then(specified).map(|u| {
            match u {
                UserSelect::Auto => "auto",
                UserSelect::Text => "text",
                UserSelect::None => "none",
                UserSelect::All => "all",
                UserSelect::Contain => "contain",
            }
            .to_string()
        }),
        "pointer-events" => style
            .pointer_events
            .as_ref()
            .and_then(specified)
            .map(|p| match p {
                crate::layout::PointerEvents::Auto => "auto".to_string(),
                crate::layout::PointerEvents::None => "none".to_string(),
            }),
        "caret-color" => style
            .caret_color
            .as_ref()
            .and_then(specified)
            .map(|c| match c {
                CaretColor::Auto => "auto".to_string(),
                CaretColor::Transparent => "transparent".to_string(),
                CaretColor::Color(c) => serialize_color(c),
            }),
        "caret-text-color" => {
            style
                .caret_text_color
                .as_ref()
                .and_then(specified)
                .map(|c| match c {
                    CaretTextColor::Auto => "auto".to_string(),
                    CaretTextColor::Color(c) => serialize_color(c),
                })
        }

        // Layout — overflow. The shorthand only serializes when
        // both axes agree (matching CSS's `overflow: <single>`
        // form). Mismatched axes only expose via the longhands.
        "overflow" => match (
            style.overflow_x.as_ref().and_then(specified),
            style.overflow_y.as_ref().and_then(specified),
        ) {
            (Some(x), Some(y)) if x == y => Some(serialize_overflow(x).to_string()),
            _ => None,
        },
        "overflow-x" => style
            .overflow_x
            .as_ref()
            .and_then(specified)
            .map(|o| serialize_overflow(o).to_string()),
        "overflow-y" => style
            .overflow_y
            .as_ref()
            .and_then(specified)
            .map(|o| serialize_overflow(o).to_string()),
        "scrollbar-gutter" => style
            .scrollbar_gutter
            .as_ref()
            .and_then(specified)
            .map(|g| {
                match g {
                    crate::layout::ScrollbarGutter::Auto => "auto",
                    crate::layout::ScrollbarGutter::Stable => "stable",
                }
                .to_string()
            }),

        // Flex shorthand. Serializes only when width and height
        // agree, matching the shape `parse_flex_shorthand` outputs
        // (`flex: <grow>` sets both axes to the same value). When
        // the axes diverge, expose via the `width` / `height`
        // longhands instead.
        "flex" => match (
            style.width.as_ref().and_then(specified),
            style.height.as_ref().and_then(specified),
        ) {
            (Some(w), Some(h)) if w == h => match w {
                Size::Flex(n) => Some(n.to_string()),
                Size::Auto => Some("none".to_string()),
                _ => None,
            },
            _ => None,
        },
        "flex-shrink" => style
            .flex_shrink
            .as_ref()
            .and_then(specified)
            .map(|n| n.to_string()),

        // Layout — sizing
        "width" => style.width.as_ref().and_then(specified).map(serialize_size),
        "height" => style
            .height
            .as_ref()
            .and_then(specified)
            .map(serialize_size),
        "min-width" => style
            .min_width
            .as_ref()
            .and_then(specified)
            .map(serialize_min_size),
        "max-width" => style
            .max_width
            .as_ref()
            .and_then(specified)
            .map(|n| n.to_string()),
        "min-height" => style
            .min_height
            .as_ref()
            .and_then(specified)
            .map(serialize_min_size),
        "max-height" => style
            .max_height
            .as_ref()
            .and_then(specified)
            .map(|n| n.to_string()),
        "aspect-ratio" => style
            .aspect_ratio
            .as_ref()
            .and_then(specified)
            .map(|r| format!("{}/{}", r.numerator, r.denominator)),

        // Layout — gap
        "gap" => style
            .gap
            .as_ref()
            .and_then(specified)
            .map(|n| n.to_string()),

        // Padding — emit the 4-value shorthand always (round-trips
        // via parse_padding_shorthand). The longhands read a
        // single side from the same shorthand value.
        "padding" => style.padding.as_ref().and_then(specified).map(|p| {
            format!(
                "{} {} {} {}",
                serialize_padding_value(&p.top),
                serialize_padding_value(&p.right),
                serialize_padding_value(&p.bottom),
                serialize_padding_value(&p.left),
            )
        }),
        "padding-top" => style
            .padding
            .as_ref()
            .and_then(specified)
            .map(|p| serialize_padding_value(&p.top)),
        "padding-right" => style
            .padding
            .as_ref()
            .and_then(specified)
            .map(|p| serialize_padding_value(&p.right)),
        "padding-bottom" => style
            .padding
            .as_ref()
            .and_then(specified)
            .map(|p| serialize_padding_value(&p.bottom)),
        "padding-left" => style
            .padding
            .as_ref()
            .and_then(specified)
            .map(|p| serialize_padding_value(&p.left)),

        // Margin — same shape as padding (shorthand + 4 longhands).
        "margin" => style.margin.as_ref().and_then(specified).map(|m| {
            format!(
                "{} {} {} {}",
                serialize_margin_value(&m.top),
                serialize_margin_value(&m.right),
                serialize_margin_value(&m.bottom),
                serialize_margin_value(&m.left),
            )
        }),
        "margin-top" => style
            .margin
            .as_ref()
            .and_then(specified)
            .map(|m| serialize_margin_value(&m.top)),
        "margin-right" => style
            .margin
            .as_ref()
            .and_then(specified)
            .map(|m| serialize_margin_value(&m.right)),
        "margin-bottom" => style
            .margin
            .as_ref()
            .and_then(specified)
            .map(|m| serialize_margin_value(&m.bottom)),
        "margin-left" => style
            .margin
            .as_ref()
            .and_then(specified)
            .map(|m| serialize_margin_value(&m.left)),

        // Border shorthand. Serializes only the combinations the
        // shorthand can express (all 4 sides on/off + corner
        // style, or exactly one side). Other combinations exist
        // (e.g. top+bottom from per-side longhands) — those
        // serialize via the per-side longhands, not here.
        "border" => style.border.as_ref().and_then(specified).and_then(|b| {
            use crate::layout::CornerStyle;
            if b.is_empty() {
                return Some("none".to_string());
            }
            // If all four sides share the same style, that style
            // serializes as the shorthand. `rounded` is the
            // rdom-specific spelling for `solid` ring + rounded
            // corners; only emit it for square→rounded promotion.
            if b.top == b.right && b.right == b.bottom && b.bottom == b.left {
                let s = border_style_keyword(b.top);
                if b.corner_style == CornerStyle::Rounded
                    && b.top == crate::layout::BorderStyle::Solid
                {
                    return Some("rounded".to_string());
                }
                return Some(s.to_string());
            }
            // Single-side shorthand legacy syntax (rdom-specific).
            // Only meaningful when the chosen side is Solid; mixed
            // styles serialize via per-side longhands instead.
            use crate::layout::BorderStyle as BS;
            match (b.top, b.right, b.bottom, b.left) {
                (BS::Solid, BS::None, BS::None, BS::None) => Some("top".to_string()),
                (BS::None, BS::Solid, BS::None, BS::None) => Some("right".to_string()),
                (BS::None, BS::None, BS::Solid, BS::None) => Some("bottom".to_string()),
                (BS::None, BS::None, BS::None, BS::Solid) => Some("left".to_string()),
                _ => None,
            }
        }),
        "border-top" | "border-top-style" => style
            .border
            .as_ref()
            .and_then(specified)
            .map(|b| border_style_keyword(b.top).to_string()),
        "border-right" | "border-right-style" => style
            .border
            .as_ref()
            .and_then(specified)
            .map(|b| border_style_keyword(b.right).to_string()),
        "border-bottom" | "border-bottom-style" => style
            .border
            .as_ref()
            .and_then(specified)
            .map(|b| border_style_keyword(b.bottom).to_string()),
        "border-left" | "border-left-style" => style
            .border
            .as_ref()
            .and_then(specified)
            .map(|b| border_style_keyword(b.left).to_string()),
        "border-style" => style.border.as_ref().and_then(specified).and_then(|b| {
            // `border-style` shorthand serializes when all four sides
            // match. Otherwise consumers read the per-side longhands.
            if b.top == b.right && b.right == b.bottom && b.bottom == b.left {
                Some(border_style_keyword(b.top).to_string())
            } else {
                None
            }
        }),
        "border-collapse" => style
            .border_collapse
            .as_ref()
            .and_then(specified)
            .map(|v| match v {
                crate::layout::BorderCollapse::Separate => "separate".to_string(),
                crate::layout::BorderCollapse::Collapse => "collapse".to_string(),
            }),

        // Pseudo-element content
        "content" => style
            .content
            .as_ref()
            .and_then(specified)
            .and_then(|c| match c {
                Content::Str(s) => Some(format!("\"{s}\"")),
                Content::Attr(a) => Some(format!("attr({a})")),
                // `Var` / `Concat` / `None` aren't produced by the
                // parser today — they're internal cascade outputs.
                // No CSS round-trip; serializer returns None.
                Content::Var(_) | Content::Concat(_) | Content::None => None,
            }),

        // Positioning (M2)
        "position" => style.position.as_ref().and_then(specified).map(|p| {
            match p {
                Position::Static => "static",
                Position::Relative => "relative",
                Position::Absolute => "absolute",
                Position::Fixed => "fixed",
                Position::Sticky => "sticky",
            }
            .to_string()
        }),
        "top" => style.top.as_ref().and_then(specified).map(serialize_length),
        "right" => style
            .right
            .as_ref()
            .and_then(specified)
            .map(serialize_length),
        "bottom" => style
            .bottom
            .as_ref()
            .and_then(specified)
            .map(serialize_length),
        "left" => style
            .left
            .as_ref()
            .and_then(specified)
            .map(serialize_length),
        "z-index" => style.z_index.as_ref().and_then(specified).map(|z| match z {
            ZIndex::Auto => "auto".to_string(),
            ZIndex::Value(n) => n.to_string(),
        }),
        // `inset` shorthand emits whenever all four sides agree on
        // some Specified Length. (CSS L1 only allows agreement;
        // mismatched values need the longhands.)
        "inset" => match (
            style.top.as_ref().and_then(specified),
            style.right.as_ref().and_then(specified),
            style.bottom.as_ref().and_then(specified),
            style.left.as_ref().and_then(specified),
        ) {
            (Some(t), Some(r), Some(b), Some(l)) => Some(format!(
                "{} {} {} {}",
                serialize_length(t),
                serialize_length(r),
                serialize_length(b),
                serialize_length(l),
            )),
            _ => None,
        },

        // Transitions (M3)
        "transition-property" => style
            .transition_property
            .as_ref()
            .and_then(specified)
            .map(|list| join_csv(list.iter(), serialize_transition_property)),
        "transition-duration" => style
            .transition_duration
            .as_ref()
            .and_then(specified)
            .map(|list| join_csv(list.iter(), |ms| format!("{ms}ms"))),
        "transition-timing-function" => style
            .transition_timing_function
            .as_ref()
            .and_then(specified)
            .map(|list| join_csv(list.iter(), serialize_timing_function)),
        "transition-delay" => style
            .transition_delay
            .as_ref()
            .and_then(specified)
            .map(|list| join_csv(list.iter(), |ms| format!("{ms}ms"))),
        "transition" => serialize_transition_shorthand(style),

        _ => None,
    }
}

// ── Per-value-type serializers ──────────────────────────────────────

fn specified<T>(v: &Value<T>) -> Option<&T> {
    match v {
        Value::Specified(t) => Some(t),
        _ => None,
    }
}

fn serialize_color(c: &TuiColor) -> String {
    match c {
        TuiColor::Literal(lit) => serialize_literal_color(lit),
        TuiColor::Var { name, fallback } => match fallback {
            Some(fb) => format!("var(--{name}, {})", serialize_color(fb)),
            None => format!("var(--{name})"),
        },
    }
}

/// Serialize a `Color` to a form `parse_color` will read back as
/// the same value:
/// - Reset → "Reset" (parses via `rdom_tui::style::parse_color`).
/// - Named ANSI → lowercase name.
/// - Rgb(r,g,b) → "rgb(r, g, b)".
/// - Indexed(n) → "indexed-N" — not currently round-trippable
///   through the CSS parser; emitted as a debug-friendly form.
fn serialize_literal_color(c: &Color) -> String {
    match c {
        Color::Reset => "reset".to_string(),
        Color::Indexed(n) => format!("indexed-{n}"),
        Color::Rgb(r, g, b) => {
            // Prefer the terminal-palette spellings authors write most
            // (they are also the aliases `named::name_of` would not pick
            // first), then any CSS name for the triple, then `rgb()`.
            let preferred = match (r, g, b) {
                (0, 255, 255) => Some("cyan"),
                (255, 0, 255) => Some("magenta"),
                (128, 128, 128) => Some("gray"),
                (169, 169, 169) => Some("darkgray"),
                _ => None,
            };
            preferred
                .or_else(|| crate::color::named::name_of(*c))
                .map(str::to_string)
                .unwrap_or_else(|| format!("rgb({r}, {g}, {b})"))
        }
    }
}

fn serialize_overflow(o: &Overflow) -> &'static str {
    match o {
        Overflow::Visible => "visible",
        Overflow::Hidden => "hidden",
        Overflow::Scroll => "scroll",
        Overflow::Auto => "auto",
    }
}

fn serialize_size(s: &Size) -> String {
    match s {
        Size::Auto => "auto".to_string(),
        Size::Fixed(n) => n.to_string(),
        Size::Flex(n) => format!("{n}fr"),
        Size::Percent(p) => format!("{p}%"),
        Size::Calc(expr) => format!("calc({})", serialize_calc(expr)),
    }
}

fn serialize_min_size(m: &crate::layout::MinSize) -> String {
    match m {
        crate::layout::MinSize::Auto => "auto".to_string(),
        crate::layout::MinSize::Cells(n) => n.to_string(),
    }
}

fn serialize_margin_value(v: &crate::layout::MarginValue) -> String {
    match v {
        crate::layout::MarginValue::Auto => "auto".to_string(),
        crate::layout::MarginValue::Cells(n) => n.to_string(),
        crate::layout::MarginValue::Calc(expr) => format!("calc({})", serialize_calc(expr)),
    }
}

fn border_style_keyword(s: crate::layout::BorderStyle) -> &'static str {
    use crate::layout::BorderStyle;
    match s {
        BorderStyle::None => "none",
        BorderStyle::Hidden => "hidden",
        BorderStyle::Solid => "solid",
        BorderStyle::Double => "double",
        BorderStyle::Dashed => "dashed",
        BorderStyle::Dotted => "dotted",
        BorderStyle::Ridge => "ridge",
        BorderStyle::Outset => "outset",
        BorderStyle::Groove => "groove",
        BorderStyle::Inset => "inset",
        BorderStyle::HalfBlock => "half-block",
    }
}

fn serialize_padding_value(v: &crate::layout::PaddingValue) -> String {
    match v {
        crate::layout::PaddingValue::Cells(n) => n.to_string(),
        crate::layout::PaddingValue::Calc(expr) => format!("calc({})", serialize_calc(expr)),
    }
}

fn serialize_length(l: &Length) -> String {
    match l {
        Length::Auto => "auto".to_string(),
        Length::Cells(n) => n.to_string(),
        Length::Calc(expr) => format!("calc({})", serialize_calc(expr)),
    }
}

/// Render a `CalcExpr` back to its source form. Used by
/// `serialize_size` / `serialize_length` for the cssText
/// round-trip + devtools / debug output.
fn serialize_calc(expr: &crate::calc::CalcExpr) -> String {
    use crate::calc::{CalcExpr, CalcOp};
    match expr {
        CalcExpr::Number(n) => {
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        CalcExpr::Length(c) => format!("{c}"),
        CalcExpr::Percent(p) => {
            if p.fract() == 0.0 {
                format!("{}%", *p as i64)
            } else {
                format!("{p}%")
            }
        }
        CalcExpr::Binary { op, lhs, rhs } => {
            let op_str = match op {
                CalcOp::Add => "+",
                CalcOp::Sub => "-",
                CalcOp::Mul => "*",
                CalcOp::Div => "/",
            };
            // Parenthesize a sub-expression whenever re-parsing the
            // flat form would bind differently: a lower-precedence
            // child under `*` / `/`, or any binary right operand of
            // the non-associative `-` / `/`.
            let prec = |o: &CalcOp| match o {
                CalcOp::Add | CalcOp::Sub => 1,
                CalcOp::Mul | CalcOp::Div => 2,
            };
            let wrap = |child: &CalcExpr, is_rhs: bool| -> String {
                let text = serialize_calc(child);
                match child {
                    CalcExpr::Binary { op: child_op, .. } => {
                        let needs = prec(child_op) < prec(op)
                            || (is_rhs
                                && prec(child_op) == prec(op)
                                && matches!(op, CalcOp::Sub | CalcOp::Div));
                        if needs { format!("({text})") } else { text }
                    }
                    _ => text,
                }
            };
            format!("{} {} {}", wrap(lhs, false), op_str, wrap(rhs, true))
        }
    }
}

fn serialize_transition_property(p: &TransitionProperty) -> String {
    use crate::transition::AnimatableProperty;
    match p {
        TransitionProperty::All => "all".to_string(),
        TransitionProperty::None => "none".to_string(),
        TransitionProperty::Named(a) => match a {
            AnimatableProperty::Color => "color",
            AnimatableProperty::BackgroundColor => "background-color",
            AnimatableProperty::BorderColor => "border-color",
            AnimatableProperty::Width => "width",
            AnimatableProperty::Height => "height",
            AnimatableProperty::Padding => "padding",
            AnimatableProperty::Gap => "gap",
            AnimatableProperty::Top => "top",
            AnimatableProperty::Right => "right",
            AnimatableProperty::Bottom => "bottom",
            AnimatableProperty::Left => "left",
            AnimatableProperty::ZIndex => "z-index",
        }
        .to_string(),
    }
}

fn serialize_timing_function(f: &TimingFunction) -> String {
    use crate::transition::StepPosition;
    match f {
        TimingFunction::Linear => "linear".to_string(),
        TimingFunction::Ease => "ease".to_string(),
        TimingFunction::EaseIn => "ease-in".to_string(),
        TimingFunction::EaseOut => "ease-out".to_string(),
        TimingFunction::EaseInOut => "ease-in-out".to_string(),
        TimingFunction::CubicBezier { x1, y1, x2, y2 } => {
            format!("cubic-bezier({x1}, {y1}, {x2}, {y2})")
        }
        TimingFunction::Steps { count, position } => {
            let pos = match position {
                StepPosition::Start => "jump-start",
                StepPosition::End => "jump-end",
                StepPosition::JumpNone => "jump-none",
                StepPosition::JumpBoth => "jump-both",
            };
            format!("steps({count}, {pos})")
        }
    }
}

/// Serialize the `transition` shorthand from the four longhand
/// vectors. Pads shorter vectors by repeating the last element
/// (matches CSS's "repeat shorter list" rule), then emits one
/// comma-separated piece per rule.
fn serialize_transition_shorthand(style: &TuiStyle) -> Option<String> {
    let props = style.transition_property.as_ref().and_then(specified)?;
    let durs = style.transition_duration.as_ref().and_then(specified)?;
    let timings = style
        .transition_timing_function
        .as_ref()
        .and_then(specified)?;
    let delays = style.transition_delay.as_ref().and_then(specified)?;
    let n = props.len();
    if n == 0 || durs.is_empty() || timings.is_empty() || delays.is_empty() {
        return None;
    }
    let pad_dur = |i: usize| durs[i.min(durs.len() - 1)];
    let pad_timing = |i: usize| &timings[i.min(timings.len() - 1)];
    let pad_delay = |i: usize| delays[i.min(delays.len() - 1)];
    let mut parts = Vec::with_capacity(n);
    for (i, p) in props.iter().enumerate() {
        parts.push(format!(
            "{} {}ms {} {}ms",
            serialize_transition_property(p),
            pad_dur(i),
            serialize_timing_function(pad_timing(i)),
            pad_delay(i),
        ));
    }
    Some(parts.join(", "))
}

fn join_csv<I, F, T>(iter: I, f: F) -> String
where
    I: Iterator<Item = T>,
    F: Fn(T) -> String,
{
    let mut out = String::new();
    for (i, item) in iter.enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&f(item));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TuiStyle;

    /// One canonical value per property — chosen to survive the
    /// parser → serialize → parser round trip. The
    /// `round_trip_every_property` test below iterates over this.
    fn canonical_values() -> &'static [(&'static str, &'static str)] {
        &[
            ("color", "red"),
            ("background-color", "blue"),
            ("background", "green"),
            ("border-color", "rgb(10, 20, 30)"),
            ("font-weight", "bold"),
            ("font-style", "italic"),
            ("text-decoration", "underline"),
            ("opacity", "0.5"),
            ("display", "inline"),
            ("flex-direction", "column"),
            ("white-space", "pre"),
            ("user-select", "text"),
            ("pointer-events", "none"),
            ("caret-color", "transparent"),
            ("caret-text-color", "auto"),
            ("overflow", "scroll"),
            ("overflow-x", "auto"),
            ("overflow-y", "hidden"),
            ("scrollbar-gutter", "stable"),
            ("width", "40"),
            ("height", "auto"),
            ("min-width", "10"),
            ("max-width", "100"),
            ("min-height", "5"),
            ("max-height", "50"),
            ("aspect-ratio", "16/9"),
            ("gap", "2"),
            ("flex", "1"),
            ("flex-shrink", "1"),
            ("padding", "1 2 3 4"),
            ("padding-top", "5"),
            ("padding-right", "6"),
            ("padding-bottom", "7"),
            ("padding-left", "8"),
            ("margin", "1 2 3 auto"),
            ("margin-top", "1"),
            ("margin-right", "2"),
            ("margin-bottom", "3"),
            ("margin-left", "auto"),
            ("border", "solid"),
            ("border-top", "solid"),
            ("border-right", "solid"),
            ("border-bottom", "solid"),
            ("border-left", "solid"),
            ("border-style", "dashed"),
            ("border-top-style", "double"),
            ("border-right-style", "hidden"),
            ("border-bottom-style", "dotted"),
            ("border-left-style", "ridge"),
            ("border-collapse", "collapse"),
            ("content", "\"hello\""),
            ("position", "absolute"),
            ("top", "10"),
            ("right", "20"),
            ("bottom", "auto"),
            ("left", "5"),
            ("z-index", "3"),
            ("inset", "1 2 3 4"),
            ("transition-property", "color"),
            ("transition-duration", "200ms"),
            ("transition-timing-function", "ease-in-out"),
            ("transition-delay", "50ms"),
            ("transition", "width 300ms ease 0ms"),
        ]
    }

    #[test]
    fn property_names_matches_canonical_values_table() {
        // Sanity: every name in `property_names()` has a canonical
        // value, and vice versa.
        let names: Vec<&str> = property_names().to_vec();
        let canon: Vec<&str> = canonical_values().iter().map(|(n, _)| *n).collect();
        assert_eq!(
            names, canon,
            "property_names() and canonical_values() must enumerate the same set in the same order"
        );
    }

    #[test]
    fn border_collapse_parses_both_keywords() {
        use crate::layout::BorderCollapse;
        let mut style = TuiStyle::new();
        set("border-collapse", "separate", &mut style).expect("separate parses");
        assert_eq!(
            style.border_collapse,
            Some(Value::Specified(BorderCollapse::Separate))
        );
        set("border-collapse", "collapse", &mut style).expect("collapse parses");
        assert_eq!(
            style.border_collapse,
            Some(Value::Specified(BorderCollapse::Collapse))
        );
    }

    #[test]
    fn border_collapse_serializes_roundtrip() {
        let mut style = TuiStyle::new();
        set("border-collapse", "collapse", &mut style).unwrap();
        assert_eq!(
            serialize("border-collapse", &style).as_deref(),
            Some("collapse")
        );
        set("border-collapse", "separate", &mut style).unwrap();
        assert_eq!(
            serialize("border-collapse", &style).as_deref(),
            Some("separate")
        );
    }

    #[test]
    fn margin_shorthand_one_value_applies_to_all_sides() {
        use crate::layout::{Margin, MarginValue};
        let mut style = TuiStyle::new();
        set("margin", "5", &mut style).expect("1-value shorthand parses");
        assert_eq!(
            style.margin,
            Some(Value::Specified(Margin {
                top: MarginValue::Cells(5),
                right: MarginValue::Cells(5),
                bottom: MarginValue::Cells(5),
                left: MarginValue::Cells(5),
            }))
        );
    }

    #[test]
    fn margin_shorthand_two_values_split_vertical_horizontal() {
        use crate::layout::{Margin, MarginValue};
        let mut style = TuiStyle::new();
        set("margin", "1 2", &mut style).expect("2-value shorthand parses");
        assert_eq!(
            style.margin,
            Some(Value::Specified(Margin {
                top: MarginValue::Cells(1),
                right: MarginValue::Cells(2),
                bottom: MarginValue::Cells(1),
                left: MarginValue::Cells(2),
            }))
        );
    }

    #[test]
    fn margin_shorthand_three_values_top_horiz_bottom() {
        use crate::layout::{Margin, MarginValue};
        let mut style = TuiStyle::new();
        set("margin", "1 2 3", &mut style).expect("3-value shorthand parses");
        assert_eq!(
            style.margin,
            Some(Value::Specified(Margin {
                top: MarginValue::Cells(1),
                right: MarginValue::Cells(2),
                bottom: MarginValue::Cells(3),
                left: MarginValue::Cells(2),
            }))
        );
    }

    #[test]
    fn margin_shorthand_four_values_each_side() {
        use crate::layout::{Margin, MarginValue};
        let mut style = TuiStyle::new();
        set("margin", "1 2 3 4", &mut style).expect("4-value shorthand parses");
        assert_eq!(
            style.margin,
            Some(Value::Specified(Margin {
                top: MarginValue::Cells(1),
                right: MarginValue::Cells(2),
                bottom: MarginValue::Cells(3),
                left: MarginValue::Cells(4),
            }))
        );
    }

    #[test]
    fn margin_accepts_negative_values() {
        use crate::layout::Margin;
        let mut style = TuiStyle::new();
        set("margin", "-5", &mut style).expect("negative values parse");
        assert_eq!(style.margin, Some(Value::Specified(Margin::all_cells(-5))));
    }

    #[test]
    fn margin_auto_keyword_parses() {
        use crate::layout::{Margin, MarginValue};
        let mut style = TuiStyle::new();
        // `0 auto`: top/bottom = 0, left/right = auto. Classic
        // horizontal centering for block-level boxes — semantic
        // wired in M5.3b.
        set("margin", "0 auto", &mut style).expect("0 auto parses");
        assert_eq!(
            style.margin,
            Some(Value::Specified(Margin {
                top: MarginValue::Cells(0),
                right: MarginValue::Auto,
                bottom: MarginValue::Cells(0),
                left: MarginValue::Auto,
            }))
        );
    }

    #[test]
    fn margin_longhand_combines_with_previous_shorthand() {
        // Setting a longhand after a shorthand updates just that side.
        use crate::layout::{Margin, MarginValue};
        let mut style = TuiStyle::new();
        set("margin", "5", &mut style).unwrap();
        set("margin-top", "10", &mut style).unwrap();
        assert_eq!(
            style.margin,
            Some(Value::Specified(Margin {
                top: MarginValue::Cells(10),
                right: MarginValue::Cells(5),
                bottom: MarginValue::Cells(5),
                left: MarginValue::Cells(5),
            }))
        );
    }

    #[test]
    fn min_width_auto_parses_and_round_trips() {
        // M5.1.b: `min-width: auto` is the CSS keyword that opts a flex
        // item into intrinsic min-content protection. The dispatch
        // accepts it both directions of the round trip.
        let mut style = TuiStyle::new();
        set("min-width", "auto", &mut style).expect("auto parses");
        assert_eq!(serialize("min-width", &style).as_deref(), Some("auto"));

        let mut style = TuiStyle::new();
        set("min-height", "auto", &mut style).expect("auto parses");
        assert_eq!(serialize("min-height", &style).as_deref(), Some("auto"));
    }

    #[test]
    fn min_width_numeric_still_round_trips_after_auto_support() {
        // Regression: adding `auto` must not break the existing
        // numeric path that M5.1.a shipped.
        let mut style = TuiStyle::new();
        set("min-width", "10", &mut style).expect("numeric parses");
        assert_eq!(serialize("min-width", &style).as_deref(), Some("10"));
    }

    #[test]
    fn set_unknown_property_errs() {
        let mut style = TuiStyle::new();
        assert_eq!(
            set("not-a-property", "x", &mut style),
            Err(DispatchError::UnknownProperty)
        );
    }

    #[test]
    fn set_invalid_value_errs() {
        let mut style = TuiStyle::new();
        assert_eq!(
            set("color", "definitely-not-a-color", &mut style),
            Err(DispatchError::InvalidValue)
        );
    }

    #[test]
    fn serialize_unset_property_is_none() {
        let style = TuiStyle::new();
        for (name, _) in canonical_values() {
            assert!(
                serialize(name, &style).is_none(),
                "serialize({name}, unset) should be None"
            );
        }
    }

    #[test]
    fn serialize_unknown_property_is_none() {
        let style = TuiStyle::new();
        assert!(serialize("bogus", &style).is_none());
    }

    /// CALC-PADMARG-1 closing test. Percent-bearing padding/margin
    /// `calc()` declarations must survive set → serialize → set.
    /// The canonical_values table doesn't cover this directly because
    /// it pairs each property with one canonical form; calc-bearing
    /// values are an additional shape over the same property.
    #[test]
    fn padding_and_margin_calc_round_trip() {
        let cases = [
            ("padding-top", "calc(50% + 1)"),
            ("padding-left", "calc(25% - 2)"),
            ("margin-right", "calc(10% + 3)"),
            ("margin-bottom", "calc(100% / 2)"),
        ];
        for (name, value) in cases {
            let mut style_a = TuiStyle::new();
            set(name, value, &mut style_a)
                .unwrap_or_else(|e| panic!("first set({name:?}, {value:?}) errored: {e:?}"));
            let serialized = serialize(name, &style_a)
                .unwrap_or_else(|| panic!("serialize({name:?}) returned None"));
            let mut style_b = TuiStyle::new();
            set(name, &serialized, &mut style_b).unwrap_or_else(|e| {
                panic!("round-trip set({name:?}, {serialized:?}) errored: {e:?}",)
            });
            assert_eq!(
                style_a, style_b,
                "{name}: calc round-trip diverged. original={value:?}, serialized={serialized:?}"
            );
        }
    }

    // ── HARDENING-2026-09 Batch 2 ────────────────────────────────────

    /// CSS-wide keywords (CSS Cascade 4 §7): `inherit` and `initial`
    /// are valid for every property; `unset` is `inherit` for inherited
    /// properties and `initial` otherwise.
    /// `STYLE-PROPERTY-TABLES-1`: the property → field table is total
    /// over the property list, every field is reachable from some
    /// property, and every `ImportantMask` bit is owned by a field.
    #[test]
    fn field_table_covers_every_property_field_and_mask_bit() {
        for name in property_names() {
            assert!(fields_of(name).is_some(), "{name} has no fields");
            assert!(property_mask(name).is_some(), "{name} has no mask");
        }
        assert!(fields_of("bogus").is_none());
        for field in Field::ALL {
            assert!(
                property_names()
                    .iter()
                    .any(|n| fields_of(n).unwrap().contains(field)),
                "{field:?} is not owned by any property"
            );
        }
        let owned = Field::ALL
            .iter()
            .fold(crate::ImportantMask::empty(), |m, f| m | f.mask());
        assert_eq!(owned, crate::ImportantMask::all());
    }

    /// `display` writes the derived `flow` too; removing it must not
    /// leave the flow behind.
    #[test]
    fn removing_display_clears_the_derived_flow() {
        let mut style = TuiStyle::new();
        set("display", "flex", &mut style).unwrap();
        assert!(style.flow.is_some());
        assert!(remove("display", &mut style));
        assert!(style.display.is_none() && style.flow.is_none());
        assert_eq!(
            property_mask("display"),
            Some(crate::ImportantMask::DISPLAY | crate::ImportantMask::FLOW)
        );
    }

    #[test]
    fn easing_functions_serialize() {
        let mut style = TuiStyle::new();
        set(
            "transition-timing-function",
            "cubic-bezier(0.1, 0.7, 1, 0.1), steps(4, jump-none), step-end, linear",
            &mut style,
        )
        .unwrap();
        assert_eq!(
            serialize("transition-timing-function", &style).as_deref(),
            Some("cubic-bezier(0.1, 0.7, 1, 0.1), steps(4, jump-none), steps(1, jump-end), linear")
        );
    }

    #[test]
    fn css_wide_keywords_parse_for_every_property() {
        for (name, _) in canonical_values() {
            let mut style = TuiStyle::new();
            set(name, "inherit", &mut style).unwrap_or_else(|e| panic!("{name}: inherit {e:?}"));
            let mut style = TuiStyle::new();
            set(name, "initial", &mut style).unwrap_or_else(|e| panic!("{name}: initial {e:?}"));
            let mut style = TuiStyle::new();
            set(name, "unset", &mut style).unwrap_or_else(|e| panic!("{name}: unset {e:?}"));
        }
        let mut style = TuiStyle::new();
        set("color", "inherit", &mut style).unwrap();
        assert_eq!(style.fg, Some(Value::Inherit));
        set("width", "initial", &mut style).unwrap();
        assert_eq!(style.width, Some(Value::Initial));
        // `unset`: color inherits, width does not.
        set("color", "unset", &mut style).unwrap();
        assert_eq!(style.fg, Some(Value::Inherit));
        set("width", "unset", &mut style).unwrap();
        assert_eq!(style.width, Some(Value::Initial));
        // Case-insensitive.
        set("color", "INHERIT", &mut style).unwrap();
        assert_eq!(style.fg, Some(Value::Inherit));
    }

    #[test]
    fn css_wide_keywords_serialize_as_themselves() {
        let mut style = TuiStyle::new();
        set("color", "inherit", &mut style).unwrap();
        assert_eq!(serialize("color", &style).as_deref(), Some("inherit"));
        set("width", "initial", &mut style).unwrap();
        assert_eq!(serialize("width", &style).as_deref(), Some("initial"));
    }

    /// A shorthand serializes as a CSS-wide keyword only when *every*
    /// field it owns holds that same keyword; mixed fields fall through
    /// to the normal per-field serialization (or `None`).
    #[test]
    fn css_wide_serialization_requires_all_owned_fields_to_agree() {
        let mut style = TuiStyle::new();
        set("overflow-x", "inherit", &mut style).unwrap();
        set("overflow-y", "hidden", &mut style).unwrap();
        assert_ne!(serialize("overflow", &style).as_deref(), Some("inherit"));
        assert_eq!(serialize("overflow-x", &style).as_deref(), Some("inherit"));

        let mut style = TuiStyle::new();
        set("top", "inherit", &mut style).unwrap();
        set("left", "1", &mut style).unwrap();
        assert_ne!(serialize("inset", &style).as_deref(), Some("inherit"));

        let mut style = TuiStyle::new();
        set("overflow", "initial", &mut style).unwrap();
        assert_eq!(serialize("overflow", &style).as_deref(), Some("initial"));
        assert_eq!(serialize("overflow-y", &style).as_deref(), Some("initial"));
    }

    /// `aspect-ratio` was missed by the checked-`u16` sweep.
    #[test]
    fn aspect_ratio_out_of_range_is_rejected_not_wrapped() {
        assert_eq!(
            set("aspect-ratio", "70000 / 1", &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue)
        );
        set("aspect-ratio", "16 / 9", &mut TuiStyle::new()).unwrap();
    }

    /// CSS Color 4 §11.1: out-of-range opacity is valid and clamps.
    #[test]
    fn negative_opacity_clamps_to_zero() {
        let mut style = TuiStyle::new();
        set("opacity", "-0.5", &mut style).unwrap();
        assert_eq!(serialize("opacity", &style).as_deref(), Some("0"));
    }

    /// `pointer-events: auto | none` (CSS Pointer Events / SVG 1.1 §16.6
    /// subset): parses, serializes, and inherits.
    #[test]
    fn pointer_events_parses_serializes_and_inherits() {
        let mut style = TuiStyle::new();
        set("pointer-events", "none", &mut style).unwrap();
        assert_eq!(serialize("pointer-events", &style).as_deref(), Some("none"));
        set("pointer-events", "auto", &mut style).unwrap();
        assert_eq!(serialize("pointer-events", &style).as_deref(), Some("auto"));
        assert_eq!(
            set("pointer-events", "visiblePainted", &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "SVG-only values are not supported in a cell grid"
        );
        assert!(inherits("pointer-events"));
        assert!(remove("pointer-events", &mut style));
        assert_eq!(serialize("pointer-events", &style), None);
    }

    /// `background` shorthand with only a color is `background-color`.
    #[test]
    fn background_shorthand_sets_background_color() {
        let mut style = TuiStyle::new();
        set("background", "red", &mut style).unwrap();
        assert_eq!(
            serialize("background-color", &style).as_deref(),
            Some("red")
        );
        assert_eq!(serialize("background", &style).as_deref(), Some("red"));
        assert_eq!(
            property_mask("background"),
            property_mask("background-color")
        );
        // Anything beyond a color (images, positions) is unsupported.
        assert_eq!(
            set("background", "url(x.png) red", &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue)
        );
    }

    /// Named colors serialize back to a CSS name, never to a non-CSS
    /// one: `lightcoral` used to come back as `lightred`, which then
    /// failed to parse.
    #[test]
    fn named_colors_round_trip_through_css_names() {
        for name in [
            "lightcoral",
            "rebeccapurple",
            "gray",
            "red",
            "cyan",
            "aliceblue",
        ] {
            let mut a = TuiStyle::new();
            set("color", name, &mut a).unwrap();
            let out = serialize("color", &a).unwrap();
            assert!(!out.starts_with("rgb("), "{name} → {out}");
            let mut b = TuiStyle::new();
            set("color", &out, &mut b).unwrap_or_else(|e| panic!("{name} → {out}: {e:?}"));
            assert_eq!(a, b);
        }
        let mut c = TuiStyle::new();
        set("color", "rgb(1, 2, 3)", &mut c).unwrap();
        assert_eq!(serialize("color", &c).as_deref(), Some("rgb(1, 2, 3)"));
    }

    /// Serialization must re-parse to the same tree: sub-expressions
    /// keep their parentheses.
    #[test]
    fn calc_serialization_keeps_parentheses() {
        for src in [
            "calc((50% + 2) * 2)",
            "calc(100% - (2 + 3))",
            "calc(10 / (2 * 5))",
            "calc((10 - 2) - 3)",
        ] {
            let mut a = TuiStyle::new();
            set("width", src, &mut a).unwrap();
            let out = serialize("width", &a).unwrap();
            let mut b = TuiStyle::new();
            set("width", &out, &mut b).unwrap_or_else(|e| panic!("{src} → {out}: {e:?}"));
            assert_eq!(a, b, "{src} → {out}");
        }
        let mut s = TuiStyle::new();
        set("width", "calc((50% + 2) * 2)", &mut s).unwrap();
        assert_eq!(
            serialize("width", &s).as_deref(),
            Some("calc((50% + 2) * 2)")
        );
    }

    /// Division by a literal zero is invalid at parse time (CSS Values 4
    /// §10.9), not a silent 0 at layout time.
    #[test]
    fn calc_division_by_literal_zero_is_rejected() {
        assert_eq!(
            set("width", "calc(10 / 0)", &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue)
        );
        assert_eq!(
            set("width", "calc(10 / 0.0)", &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue)
        );
        set("width", "calc(10 / 2)", &mut TuiStyle::new()).unwrap();
    }

    /// `flex: <grow> <shrink> <basis>` accepts the canonical `0%` basis
    /// and fractional factors.
    #[test]
    fn flex_shorthand_accepts_percent_basis_and_fractional_factors() {
        for src in ["1 1 0%", "1 0.5 auto", "2 1 0", "1 1 10%"] {
            set("flex", src, &mut TuiStyle::new()).unwrap_or_else(|e| panic!("{src}: {e:?}"));
        }
        assert_eq!(
            set("flex", "1 1 foo", &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue)
        );
    }

    /// The headline spec test for step 25. Every property name in
    /// the dispatch table must survive a set → serialize → set
    /// round trip, with the resulting `TuiStyle` byte-equal to the
    /// first set's `TuiStyle`.
    #[test]
    fn round_trip_every_property() {
        for (name, value) in canonical_values() {
            let mut style_a = TuiStyle::new();
            set(name, value, &mut style_a).unwrap_or_else(|e| {
                panic!("first set({name:?}, {value:?}) errored: {e:?}");
            });
            let serialized = serialize(name, &style_a)
                .unwrap_or_else(|| panic!("serialize({name:?}) returned None"));
            let mut style_b = TuiStyle::new();
            set(name, &serialized, &mut style_b).unwrap_or_else(|e| {
                panic!(
                    "round-trip set({name:?}, {serialized:?}) errored: {e:?} — \
                     serializer produced unparsable form"
                );
            });
            assert_eq!(
                style_a, style_b,
                "{name}: round-trip diverged. original={value:?}, serialized={serialized:?}"
            );
        }
    }
}
