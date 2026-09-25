//! `serialize`: property name → CSS text for whatever `TuiStyle`
//! currently holds under that name, one arm per property. Shorthands
//! only serialize when their longhands agree (`overflow`, `flex`,
//! `border`, `inset`, `transition`); the per-value-type helpers live
//! in `value_serializers.rs`.

use super::css_wide::css_wide_of;
use super::value_serializers::{
    border_style_keyword, join_csv, serialize_calc, serialize_color, serialize_content,
    serialize_counter_ops, serialize_length, serialize_margin_value, serialize_min_size,
    serialize_overflow, serialize_padding_value, serialize_size, serialize_timing_function,
    serialize_transition_property, serialize_transition_shorthand, specified,
};
use crate::layout::{
    CaretColor, CaretTextColor, Direction, Display, Position, Size, UserSelect, WhiteSpace, ZIndex,
};
use crate::{Content, TuiStyle};

/// Serialize the named property's current value as a CSS string.
/// Returns `None` when the property isn't currently set — callers
/// map this to `""` to match `getPropertyValue`.
///
/// Unknown property names also return `None` (rather than
/// errorring); CSSOM `getPropertyValue("bogus")` returns `""` too.
pub fn serialize(name: &str, style: &TuiStyle) -> Option<String> {
    if let Some(custom) = name.strip_prefix("--") {
        return style.custom_property_value(custom).map(str::to_string);
    }
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
        "gap" => style.gap.as_ref().and_then(specified).map(|g| match g {
            crate::layout::GapValue::Cells(n) => n.to_string(),
            crate::layout::GapValue::Calc(expr) => format!("calc({})", serialize_calc(expr)),
        }),

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
                Content::None => Some("none".to_string()),
                other => serialize_content(other),
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
        "counter-reset" => style
            .counter_reset
            .as_ref()
            .and_then(specified)
            .map(|ops| serialize_counter_ops(ops)),
        "counter-increment" => style
            .counter_increment
            .as_ref()
            .and_then(specified)
            .map(|ops| serialize_counter_ops(ops)),

        _ => None,
    }
}
