//! `serialize` for the paint and text properties: `color`, `background-color`,
//! `opacity`,
//! `user-select`, `pointer-events`, `visibility`, the caret colors,
//! `color-scheme` and `content`.

use super::super::value_serializers::{
    serialize_color, serialize_content, serialize_css_string, specified,
};
use crate::layout::{CaretColor, CaretTextColor, UserSelect};
use crate::{Content, TuiStyle};

/// `name`'s serialization when it is one of this family's properties —
/// `Some(None)` when it is not set — else `None`.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let out = match name {
        // Color / modifiers
        "color" => style.fg.as_ref().and_then(specified).map(serialize_color),
        "background-color" => style.bg.as_ref().and_then(specified).map(serialize_color),
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
        "list-style-type" => style
            .list_style_type
            .as_ref()
            .and_then(specified)
            .map(|t| t.to_css()),
        "list-style-position" => style
            .list_style_position
            .as_ref()
            .and_then(specified)
            .map(|p| list_position(*p).to_string()),
        "list-style-image" => style
            .list_style_image
            .as_ref()
            .and_then(specified)
            .map(|i| i.to_css()),
        "list-style" => {
            let position = style.list_style_position.as_ref().and_then(specified)?;
            let image = style.list_style_image.as_ref().and_then(specified)?;
            let kind = style.list_style_type.as_ref().and_then(specified)?;
            Some(list_style_shorthand(*position, image, kind))
        }
        "marker-side" => style.marker_side.as_ref().and_then(specified).map(|m| {
            match m {
                crate::layout::MarkerSide::MatchParent => "match-parent",
                _ => "match-self",
            }
            .to_string()
        }),
        "quotes" => style.quotes.as_ref().and_then(specified).map(|q| match q {
            crate::Quotes::None => "none".to_string(),
            crate::Quotes::MatchParent => "match-parent".to_string(),
            crate::Quotes::Pairs(pairs) => pairs
                .iter()
                .map(|p| {
                    format!(
                        "{} {}",
                        serialize_css_string(&p.open),
                        serialize_css_string(&p.close)
                    )
                })
                .collect::<Vec<_>>()
                .join(" "),
            _ => "auto".to_string(),
        }),
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

/// The `list-style-position` keyword.
fn list_position(p: crate::layout::ListStylePosition) -> &'static str {
    match p {
        crate::layout::ListStylePosition::Inside => "inside",
        _ => "outside",
    }
}

/// The `list-style` shorthand, shortest: the longhands that are not
/// initial, in position / image / type order; `none` when type and image
/// are both `none` and the position is initial; `disc` when all are
/// initial.
fn list_style_shorthand(
    position: crate::layout::ListStylePosition,
    image: &crate::layout::ListStyleImage,
    kind: &crate::layout::ListStyleType,
) -> String {
    use crate::layout::{ListStyleImage, ListStylePosition, ListStyleType};
    let outside = position == ListStylePosition::Outside;
    let no_image = *image == ListStyleImage::None;
    if outside && no_image && *kind == ListStyleType::None {
        return "none".to_string();
    }
    let mut parts = Vec::new();
    if !outside {
        parts.push(list_position(position).to_string());
    }
    if !no_image {
        parts.push(image.to_css());
    }
    if *kind != ListStyleType::default() {
        parts.push(kind.to_css());
    }
    if parts.is_empty() {
        "disc".to_string()
    } else {
        parts.join(" ")
    }
}
