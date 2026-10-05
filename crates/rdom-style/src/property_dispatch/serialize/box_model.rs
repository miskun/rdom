//! `serialize` for the box model: overflow and scrolling keywords,
//! `margin-trim`, `direction` / `writing-mode`, `box-sizing`, the sizes,
//! `padding`, `margin` and `border-collapse`.

use super::super::value_serializers::{
    all_specified, serialize_margin_value, serialize_max_size, serialize_min_size,
    serialize_overflow, serialize_padding_value, serialize_size, shortest_sides, side_value,
    specified,
};
use crate::TuiStyle;

/// `name`'s serialization when it is one of this family's properties —
/// `Some(None)` when it is not set — else `None`.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let out = match name {
        // One value when both axes agree, else `<x> <y>` (CSS Overflow 3
        // §3.1).
        "overflow" => match (
            style.overflow_x.as_ref().and_then(specified),
            style.overflow_y.as_ref().and_then(specified),
        ) {
            (Some(x), Some(y)) if x == y => Some(serialize_overflow(x).to_string()),
            (Some(x), Some(y)) => Some(format!(
                "{} {}",
                serialize_overflow(x),
                serialize_overflow(y)
            )),
            _ => None,
        },
        // The shortest form: the box when not `padding-box`, then the
        // length when not 0 (`0` when both are omitted).
        "overflow-clip-margin" => {
            style
                .overflow_clip_margin
                .as_ref()
                .and_then(specified)
                .map(|m| {
                    let initial = crate::layout::OverflowClipMargin::default();
                    match (m.visual_box != initial.visual_box, m.margin) {
                        (true, 0) => m.visual_box.keyword().to_string(),
                        (true, n) => format!("{} {n}", m.visual_box.keyword()),
                        (false, n) => n.to_string(),
                    }
                })
        }
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
        "margin-trim" => style
            .margin_trim
            .as_ref()
            .and_then(specified)
            .map(super::super::value_serializers::serialize_margin_trim),
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
        _ => return None,
    };
    Some(out)
}
