//! `serialize`: property name → CSS text for whatever `TuiStyle`
//! currently holds under that name, one arm per property. Shorthands
//! only serialize when their longhands agree (`overflow`, `flex`,
//! `border`, `inset`, `transition`); the per-value-type helpers live
//! in `value_serializers.rs`.

use super::css_wide::css_wide_of;
use super::table::canonical_property_name;
use super::value_serializers::{
    all_specified, join_csv, serialize_color, serialize_content, serialize_counter_ops,
    serialize_flex_basis, serialize_gap, serialize_length, serialize_margin_value,
    serialize_max_size, serialize_min_size, serialize_overflow, serialize_padding_value,
    serialize_size, serialize_timing_function, serialize_transition_property,
    serialize_transition_shorthand, shortest_sides, side_value, specified,
};
use crate::layout::{CaretColor, CaretTextColor, Position, UserSelect, WhiteSpace, ZIndex};
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
    let name = &*canonical_property_name(name);
    // A `var()` value is kept as written until the cascade (CSS
    // Variables 1 §3).
    if let Some(d) = style
        .pending
        .iter()
        .find(|d| d.name == name && d.has_substitution)
    {
        return Some(d.value_text());
    }
    // An inline-axis flow-relative property is mapped only by the
    // cascade (CSS Logical 1 §4): read from its last declarations, a
    // shorthand's component included (CSSOM §6.6).
    if let Some(out) = super::logical::serialize_inline_axis(name, style) {
        return out;
    }
    if let Some(kw) = css_wide_of(name, style) {
        return Some(kw.to_string());
    }
    if super::logical::is_directional(name) {
        return None;
    }
    if let Some(out) = super::background::serialize(name, style)
        .or_else(|| super::border::serialize(name, style))
        .or_else(|| super::shadow::serialize(name, style))
        .or_else(|| super::contain::serialize(name, style))
        .or_else(|| super::logical::serialize_block_axis(name, style))
    {
        return out;
    }
    match name {
        // Color / modifiers
        "color" => style.fg.as_ref().and_then(specified).map(serialize_color),
        "background-color" => style.bg.as_ref().and_then(specified).map(serialize_color),
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
            let flow = style.flow.as_ref().and_then(specified).copied();
            let list_item = style.list_item.as_ref().and_then(specified).copied();
            crate::parse::values::serialize_display(
                *d,
                flow.unwrap_or_default(),
                list_item.unwrap_or(false),
            )
        }),
        "flex-direction" => style.direction.as_ref().and_then(specified).map(|d| {
            let reverse = style.flex_reverse.as_ref().and_then(specified) == Some(&true);
            crate::parse::values::serialize_flex_direction(*d, reverse).to_string()
        }),
        "flex-wrap" => style
            .flex_wrap
            .as_ref()
            .and_then(specified)
            .map(|w| crate::parse::values::serialize_flex_wrap(*w).to_string()),
        "justify-content" => style
            .justify_content
            .as_ref()
            .and_then(specified)
            .map(|a| crate::parse::values::serialize_alignment(*a)),
        "align-items" => style
            .align_items
            .as_ref()
            .and_then(specified)
            .map(|a| crate::parse::values::serialize_alignment(*a)),
        "align-self" => style
            .align_self
            .as_ref()
            .and_then(specified)
            .map(|a| crate::parse::values::serialize_alignment(*a)),
        // The shorthand serializes only when both longhands are set
        // (CSSOM §6.7.2).
        "flex-flow" => {
            let d = style.direction.as_ref().and_then(specified)?;
            let w = style.flex_wrap.as_ref().and_then(specified)?;
            let reverse = style.flex_reverse.as_ref().and_then(specified) == Some(&true);
            Some(crate::parse::values::serialize_flex_flow(*d, reverse, *w))
        }
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
        "visibility" => style.visibility.as_ref().and_then(specified).map(|v| {
            match v {
                crate::layout::Visibility::Visible => "visible",
                crate::layout::Visibility::Hidden => "hidden",
                crate::layout::Visibility::Collapse => "collapse",
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
        "color-scheme" => style
            .color_scheme
            .as_ref()
            .and_then(specified)
            .map(|s| s.to_css()),
        "margin-trim" => style
            .margin_trim
            .as_ref()
            .and_then(specified)
            .map(super::value_serializers::serialize_margin_trim),
        "direction" => style.text_direction.as_ref().and_then(specified).map(|d| {
            match d {
                crate::layout::TextDirection::Ltr => "ltr",
                crate::layout::TextDirection::Rtl => "rtl",
            }
            .to_string()
        }),
        "writing-mode" => style.writing_mode.as_ref().and_then(specified).map(|m| {
            use crate::layout::WritingMode;
            match m {
                WritingMode::HorizontalTb => "horizontal-tb",
                WritingMode::VerticalRl => "vertical-rl",
                WritingMode::VerticalLr => "vertical-lr",
                WritingMode::SidewaysRl => "sideways-rl",
                WritingMode::SidewaysLr => "sideways-lr",
            }
            .to_string()
        }),
        "box-sizing" => style.box_sizing.as_ref().and_then(specified).map(|b| {
            match b {
                crate::layout::BoxSizing::ContentBox => "content-box",
                crate::layout::BoxSizing::BorderBox => "border-box",
            }
            .to_string()
        }),
        "scroll-behavior" => style.scroll_behavior.as_ref().and_then(specified).map(|b| {
            match b {
                crate::layout::ScrollBehavior::Auto => "auto",
                crate::layout::ScrollBehavior::Smooth => "smooth",
            }
            .to_string()
        }),

        // Flex shorthand: `<grow> <shrink> <basis>` when its three
        // longhands are declared (CSS Flexbox §7.2).
        "flex" => match (
            style.flex_grow.as_ref().and_then(specified),
            style.flex_shrink.as_ref().and_then(specified),
            style.flex_basis.as_ref().and_then(specified),
        ) {
            (Some(grow), Some(shrink), Some(basis)) => {
                Some(format!("{grow} {shrink} {}", serialize_flex_basis(basis)))
            }
            _ => None,
        },
        "flex-grow" => style
            .flex_grow
            .as_ref()
            .and_then(specified)
            .map(|n| n.to_string()),
        "flex-basis" => style
            .flex_basis
            .as_ref()
            .and_then(specified)
            .map(serialize_flex_basis),
        "flex-shrink" => style
            .flex_shrink
            .as_ref()
            .and_then(specified)
            .map(|n| n.to_string()),
        "order" => style
            .order
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
            .map(serialize_max_size),
        "min-height" => style
            .min_height
            .as_ref()
            .and_then(specified)
            .map(serialize_min_size),
        "max-height" => style
            .max_height
            .as_ref()
            .and_then(specified)
            .map(serialize_max_size),
        "aspect-ratio" => style
            .aspect_ratio
            .as_ref()
            .and_then(specified)
            .map(|r| match r {
                None => "auto".to_string(),
                Some(r) => format!(
                    "{}{} / {}",
                    if r.auto() { "auto " } else { "" },
                    r.numerator(),
                    r.denominator()
                ),
            }),

        // Layout — gap
        // CSS Box Alignment 3 §8.3: one value when the two agree.
        "gap" => match (
            style.row_gap.as_ref().and_then(specified),
            style.column_gap.as_ref().and_then(specified),
        ) {
            (Some(row), Some(column)) if row == column => Some(serialize_gap(row)),
            (Some(row), Some(column)) => {
                Some(format!("{} {}", serialize_gap(row), serialize_gap(column)))
            }
            _ => None,
        },
        "row-gap" => style
            .row_gap
            .as_ref()
            .and_then(specified)
            .map(serialize_gap),
        "column-gap" => style
            .column_gap
            .as_ref()
            .and_then(specified)
            .map(serialize_gap),

        // CSS Box 3 §3.2 / §4.2: a shorthand when every side is set, in
        // the shortest form (CSSOM §6.7.2); a longhand its side.
        "padding" => {
            all_specified(&style.padding).map(|p| shortest_sides(p.map(serialize_padding_value)))
        }
        "padding-top" => side_value(&style.padding.top, serialize_padding_value),
        "padding-right" => side_value(&style.padding.right, serialize_padding_value),
        "padding-bottom" => side_value(&style.padding.bottom, serialize_padding_value),
        "padding-left" => side_value(&style.padding.left, serialize_padding_value),
        "margin" => {
            all_specified(&style.margin).map(|m| shortest_sides(m.map(serialize_margin_value)))
        }
        "margin-top" => side_value(&style.margin.top, serialize_margin_value),
        "margin-right" => side_value(&style.margin.right, serialize_margin_value),
        "margin-bottom" => side_value(&style.margin.bottom, serialize_margin_value),
        "margin-left" => side_value(&style.margin.left, serialize_margin_value),

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
