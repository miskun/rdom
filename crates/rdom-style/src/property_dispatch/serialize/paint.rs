//! `serialize` for the paint and text properties: `color`, `background-color`,
//! the font keywords, `text-decoration`, `opacity`, `white-space`,
//! `user-select`, `pointer-events`, `visibility`, the caret colors,
//! `color-scheme` and `content`.

use super::super::value_serializers::{serialize_color, serialize_content, specified};
use crate::layout::{CaretColor, CaretTextColor, UserSelect, WhiteSpace};
use crate::{Content, TuiStyle};

/// `name`'s serialization when it is one of this family's properties —
/// `Some(None)` when it is not set — else `None`.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let out = match name {
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
        "color-scheme" => style
            .color_scheme
            .as_ref()
            .and_then(specified)
            .map(|s| s.to_css()),
        "content" => style
            .content
            .as_ref()
            .and_then(specified)
            .and_then(|c| match c {
                Content::None => Some("none".to_string()),
                other => serialize_content(other),
            }),

        // Positioning (M2)
        _ => return None,
    };
    Some(out)
}
