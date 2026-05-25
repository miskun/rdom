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
    Border, CaretColor, CaretTextColor, Direction, Display, Length, Overflow, Position, Size,
    UserSelect, WhiteSpace, ZIndex,
};
use crate::parse::token::{Token, tokenize};
use crate::parse::values::{
    current_margin, current_padding, parse_aspect_ratio, parse_border, parse_color, parse_content,
    parse_flex_shorthand, parse_inset_shorthand, parse_keyword, parse_length,
    parse_margin_longhand, parse_margin_shorthand, parse_min_size, parse_opacity, parse_overflow,
    parse_padding_shorthand, parse_position, parse_size, parse_text_decoration, parse_time_list,
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
    "caret-color",
    "caret-text-color",
    // Layout — overflow
    "overflow",
    "overflow-x",
    "overflow-y",
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

/// Map a property name to the [`ImportantMask`] bit(s) it owns.
/// Properties that affect multiple fields (`overflow` → X + Y,
/// `inset` → all four sides) return the OR of every bit they
/// touch. Returns `None` for unknown names.
///
/// Two consumers: the `rdom-css` block parser's `!important`
/// routing, and `rdom-tui`'s `StyleDeclaration::set_property_
/// important` / `get_property_priority`.
pub fn property_mask(name: &str) -> Option<crate::ImportantMask> {
    use crate::ImportantMask;
    Some(match name {
        "color" => ImportantMask::FG,
        "background-color" => ImportantMask::BG,
        "border-color" => ImportantMask::BORDER_FG,
        "font-weight" => ImportantMask::BOLD,
        "font-style" => ImportantMask::ITALIC,
        "text-decoration" => ImportantMask::TEXT_DECORATION,
        "opacity" => ImportantMask::OPACITY,
        "display" => ImportantMask::DISPLAY,
        "flex-direction" => ImportantMask::DIRECTION,
        "white-space" => ImportantMask::WHITE_SPACE,
        "user-select" => ImportantMask::USER_SELECT,
        "caret-color" => ImportantMask::CARET_COLOR,
        "caret-text-color" => ImportantMask::CARET_TEXT_COLOR,
        "overflow" => ImportantMask::OVERFLOW_X | ImportantMask::OVERFLOW_Y,
        "overflow-x" => ImportantMask::OVERFLOW_X,
        "overflow-y" => ImportantMask::OVERFLOW_Y,
        "width" => ImportantMask::WIDTH,
        "height" => ImportantMask::HEIGHT,
        "min-width" => ImportantMask::MIN_WIDTH,
        "max-width" => ImportantMask::MAX_WIDTH,
        "min-height" => ImportantMask::MIN_HEIGHT,
        "max-height" => ImportantMask::MAX_HEIGHT,
        "aspect-ratio" => ImportantMask::ASPECT_RATIO,
        "gap" => ImportantMask::GAP,
        "flex" => ImportantMask::WIDTH | ImportantMask::HEIGHT | ImportantMask::FLEX_SHRINK,
        "flex-shrink" => ImportantMask::FLEX_SHRINK,
        "padding" | "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => {
            ImportantMask::PADDING
        }
        "margin" | "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => {
            ImportantMask::MARGIN
        }
        "border" => ImportantMask::BORDER,
        "border-collapse" => ImportantMask::BORDER_COLLAPSE,
        "content" => ImportantMask::CONTENT,
        "transition-property"
        | "transition-duration"
        | "transition-timing-function"
        | "transition-delay"
        | "transition" => ImportantMask::TRANSITIONS,
        "position" => ImportantMask::POSITION,
        "top" => ImportantMask::TOP,
        "right" => ImportantMask::RIGHT,
        "bottom" => ImportantMask::BOTTOM,
        "left" => ImportantMask::LEFT,
        "z-index" => ImportantMask::Z_INDEX,
        "inset" => {
            ImportantMask::TOP | ImportantMask::RIGHT | ImportantMask::BOTTOM | ImportantMask::LEFT
        }
        _ => return None,
    })
}

/// Clear the named property from `style` — reset its field(s) to
/// `None` and drop its `!important` bit. Returns `true` iff the
/// property was previously set (any of its fields was `Some`).
/// Returns `false` for unknown names.
///
/// `StyleDeclarationMut::remove_property` consumes this to
/// implement CSSOM `removeProperty()` semantics.
pub fn remove(name: &str, style: &mut TuiStyle) -> bool {
    let was_set = match name {
        "color" => style.fg.take().is_some(),
        "background-color" => style.bg.take().is_some(),
        "border-color" => style.border_fg.take().is_some(),
        "font-weight" => style.bold.take().is_some(),
        "font-style" => style.italic.take().is_some(),
        "text-decoration" => style.text_decoration.take().is_some(),
        "opacity" => style.opacity.take().is_some(),
        "display" => style.display.take().is_some(),
        "flex-direction" => style.direction.take().is_some(),
        "white-space" => style.white_space.take().is_some(),
        "user-select" => style.user_select.take().is_some(),
        "caret-color" => style.caret_color.take().is_some(),
        "caret-text-color" => style.caret_text_color.take().is_some(),
        "overflow" => style.overflow_x.take().is_some() | style.overflow_y.take().is_some(),
        "overflow-x" => style.overflow_x.take().is_some(),
        "overflow-y" => style.overflow_y.take().is_some(),
        "width" => style.width.take().is_some(),
        "height" => style.height.take().is_some(),
        "min-width" => style.min_width.take().is_some(),
        "max-width" => style.max_width.take().is_some(),
        "min-height" => style.min_height.take().is_some(),
        "max-height" => style.max_height.take().is_some(),
        "aspect-ratio" => style.aspect_ratio.take().is_some(),
        "gap" => style.gap.take().is_some(),
        "flex" => {
            style.width.take().is_some()
                | style.height.take().is_some()
                | style.flex_shrink.take().is_some()
        }
        "flex-shrink" => style.flex_shrink.take().is_some(),
        "padding" | "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => {
            // Per-side longhands don't have separate storage — the
            // shorthand owns all four cells. Removing any longhand
            // clears the whole thing (matches CSS the same way
            // `removeProperty("padding-top")` clears the entry).
            style.padding.take().is_some()
        }
        "margin" | "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => {
            // Same shape as padding — longhands share the shorthand
            // storage. Clearing any longhand removes the whole margin.
            style.margin.take().is_some()
        }
        "border" => style.border.take().is_some(),
        "border-collapse" => style.border_collapse.take().is_some(),
        "content" => style.content.take().is_some(),
        "position" => style.position.take().is_some(),
        "top" => style.top.take().is_some(),
        "right" => style.right.take().is_some(),
        "bottom" => style.bottom.take().is_some(),
        "left" => style.left.take().is_some(),
        "z-index" => style.z_index.take().is_some(),
        "inset" => {
            style.top.take().is_some()
                | style.right.take().is_some()
                | style.bottom.take().is_some()
                | style.left.take().is_some()
        }
        "transition-property" => style.transition_property.take().is_some(),
        "transition-duration" => style.transition_duration.take().is_some(),
        "transition-timing-function" => style.transition_timing_function.take().is_some(),
        "transition-delay" => style.transition_delay.take().is_some(),
        "transition" => {
            style.transition_property.take().is_some()
                | style.transition_duration.take().is_some()
                | style.transition_timing_function.take().is_some()
                | style.transition_delay.take().is_some()
        }
        _ => return false,
    };
    // Also clear the !important bit for this property.
    if let Some(mask) = property_mask(name) {
        style.important = style.important.without(mask);
    }
    was_set
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
    let outcome: Option<()> = match name {
        // Color / modifiers
        "color" => parse_color(value).map(|c| {
            style.fg = Some(Value::Specified(c));
        }),
        "background-color" => parse_color(value).map(|c| {
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
        "padding-top" => parse_unsigned(value).map(|n| {
            let mut p = current_padding(style);
            p.top = n;
            style.padding = Some(Value::Specified(p));
        }),
        "padding-right" => parse_unsigned(value).map(|n| {
            let mut p = current_padding(style);
            p.right = n;
            style.padding = Some(Value::Specified(p));
        }),
        "padding-bottom" => parse_unsigned(value).map(|n| {
            let mut p = current_padding(style);
            p.bottom = n;
            style.padding = Some(Value::Specified(p));
        }),
        "padding-left" => parse_unsigned(value).map(|n| {
            let mut p = current_padding(style);
            p.left = n;
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

        // Border keyword
        "border" => parse_border(value).map(|b| {
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
            style.transition_property = Some(list);
        }),
        "transition-duration" => parse_time_list(value).map(|list| {
            style.transition_duration = Some(list);
        }),
        "transition-timing-function" => parse_timing_function_list(value).map(|list| {
            style.transition_timing_function = Some(list);
        }),
        "transition-delay" => parse_time_list(value).map(|list| {
            style.transition_delay = Some(list);
        }),
        "transition" => parse_transition_shorthand(value).map(|rules| {
            let (props, durs, timings, delays) = unzip_transition_rules(&rules);
            style.transition_property = Some(props);
            style.transition_duration = Some(durs);
            style.transition_timing_function = Some(timings);
            style.transition_delay = Some(delays);
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
    match name {
        // Color / modifiers
        "color" => style.fg.as_ref().and_then(specified).map(serialize_color),
        "background-color" => style.bg.as_ref().and_then(specified).map(serialize_color),
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
        "padding" => style
            .padding
            .as_ref()
            .and_then(specified)
            .map(|p| format!("{} {} {} {}", p.top, p.right, p.bottom, p.left)),
        "padding-top" => style
            .padding
            .as_ref()
            .and_then(specified)
            .map(|p| p.top.to_string()),
        "padding-right" => style
            .padding
            .as_ref()
            .and_then(specified)
            .map(|p| p.right.to_string()),
        "padding-bottom" => style
            .padding
            .as_ref()
            .and_then(specified)
            .map(|p| p.bottom.to_string()),
        "padding-left" => style
            .padding
            .as_ref()
            .and_then(specified)
            .map(|p| p.left.to_string()),

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

        // Border keyword. `Single` parses as both "solid" and
        // "single"; serialize as the CSS-faithful "solid".
        "border" => style.border.as_ref().and_then(specified).map(|b| {
            match b {
                Border::None => "none",
                Border::Single => "solid",
                Border::Rounded => "rounded",
                Border::Top => "top",
                Border::Bottom => "bottom",
                Border::Left => "left",
                Border::Right => "right",
            }
            .to_string()
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
            .map(|list| join_csv(list.iter(), serialize_transition_property)),
        "transition-duration" => style
            .transition_duration
            .as_ref()
            .map(|list| join_csv(list.iter(), |ms| format!("{ms}ms"))),
        "transition-timing-function" => style
            .transition_timing_function
            .as_ref()
            .map(|list| join_csv(list.iter(), |f| serialize_timing_function(f).to_string())),
        "transition-delay" => style
            .transition_delay
            .as_ref()
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
        Color::Rgb(0, 0, 0) => "black".to_string(),
        Color::Rgb(255, 0, 0) => "red".to_string(),
        Color::Rgb(0, 128, 0) => "green".to_string(),
        Color::Rgb(255, 255, 0) => "yellow".to_string(),
        Color::Rgb(0, 0, 255) => "blue".to_string(),
        Color::Rgb(255, 0, 255) => "magenta".to_string(),
        Color::Rgb(0, 255, 255) => "cyan".to_string(),
        Color::Rgb(128, 128, 128) => "gray".to_string(),
        Color::Rgb(169, 169, 169) => "darkgray".to_string(),
        Color::Rgb(240, 128, 128) => "lightred".to_string(),
        Color::Rgb(144, 238, 144) => "lightgreen".to_string(),
        Color::Rgb(255, 255, 224) => "lightyellow".to_string(),
        Color::Rgb(173, 216, 230) => "lightblue".to_string(),
        Color::Rgb(255, 128, 255) => "lightmagenta".to_string(),
        Color::Rgb(224, 255, 255) => "lightcyan".to_string(),
        Color::Rgb(255, 255, 255) => "white".to_string(),
        Color::Indexed(n) => format!("indexed-{n}"),
        Color::Rgb(r, g, b) => format!("rgb({r}, {g}, {b})"),
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
            format!("{} {} {}", serialize_calc(lhs), op_str, serialize_calc(rhs))
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

fn serialize_timing_function(f: &TimingFunction) -> &'static str {
    match f {
        TimingFunction::Linear => "linear",
        TimingFunction::Ease => "ease",
        TimingFunction::EaseIn => "ease-in",
        TimingFunction::EaseOut => "ease-out",
        TimingFunction::EaseInOut => "ease-in-out",
    }
}

/// Serialize the `transition` shorthand from the four longhand
/// vectors. Pads shorter vectors by repeating the last element
/// (matches CSS's "repeat shorter list" rule), then emits one
/// comma-separated piece per rule.
fn serialize_transition_shorthand(style: &TuiStyle) -> Option<String> {
    let props = style.transition_property.as_ref()?;
    let durs = style.transition_duration.as_ref()?;
    let timings = style.transition_timing_function.as_ref()?;
    let delays = style.transition_delay.as_ref()?;
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
            ("border-color", "rgb(10, 20, 30)"),
            ("font-weight", "bold"),
            ("font-style", "italic"),
            ("text-decoration", "underline"),
            ("opacity", "0.5"),
            ("display", "inline"),
            ("flex-direction", "column"),
            ("white-space", "pre"),
            ("user-select", "text"),
            ("caret-color", "transparent"),
            ("caret-text-color", "auto"),
            ("overflow", "scroll"),
            ("overflow-x", "auto"),
            ("overflow-y", "hidden"),
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
