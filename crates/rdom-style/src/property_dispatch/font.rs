//! The CSS Fonts 4 properties: `font-weight`, `font-style`, `font-size`,
//! `font-family`, `font-stretch` (and its new name `font-width`),
//! `font-variant` and the `font` shorthand — their `set` and `serialize`
//! arms.

use super::value_serializers::specified;
use crate::layout::{FontFamily, FontSize, FontStretch, FontStyle, FontVariant, FontWeight};
use crate::parse::token::Token;
use crate::parse::values::{
    parse_font, parse_font_family, parse_font_size, parse_font_stretch, parse_font_style,
    parse_font_variant, parse_font_weight, serialize_font_family, serialize_font_size,
    serialize_font_stretch, serialize_font_style, serialize_font_variant, serialize_font_weight,
    serialize_line_height,
};
use crate::{TuiStyle, Value};

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    let f = &mut style.font;
    Some(match name {
        "font-weight" => parse_font_weight(value).map(|w| {
            f.weight = Some(Value::Specified(w));
        }),
        "font-style" => parse_font_style(value).map(|s| {
            f.style = Some(Value::Specified(s));
        }),
        "font-size" => parse_font_size(value).map(|s| {
            f.size = Some(Value::Specified(s));
        }),
        "font-family" => parse_font_family(value).map(|fam| {
            f.family = Some(Value::Specified(fam));
        }),
        "font-stretch" | "font-width" => parse_font_stretch(value).map(|s| {
            f.stretch = Some(Value::Specified(s));
        }),
        "font-variant" => parse_font_variant(value).map(|v| {
            f.variant = Some(Value::Specified(v));
        }),
        "font" => parse_font(value).map(|s| {
            f.style = Some(Value::Specified(s.style));
            f.variant = Some(Value::Specified(s.variant));
            f.weight = Some(Value::Specified(s.weight));
            f.stretch = Some(Value::Specified(s.stretch));
            f.size = Some(Value::Specified(s.size));
            f.family = Some(Value::Specified(s.family));
            style.text.line_height = Some(Value::Specified(s.line_height));
        }),
        _ => return None,
    })
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let f = &style.font;
    Some(match name {
        "font-weight" => f
            .weight
            .as_ref()
            .and_then(specified)
            .map(|w| serialize_font_weight(*w)),
        "font-style" => f
            .style
            .as_ref()
            .and_then(specified)
            .map(|s| serialize_font_style(*s)),
        "font-size" => f.size.as_ref().and_then(specified).map(serialize_font_size),
        "font-family" => f
            .family
            .as_ref()
            .and_then(specified)
            .and_then(serialize_font_family),
        "font-stretch" | "font-width" => f
            .stretch
            .as_ref()
            .and_then(specified)
            .map(|s| serialize_font_stretch(*s)),
        "font-variant" => f
            .variant
            .as_ref()
            .and_then(specified)
            .map(|v| serialize_font_variant(*v).to_string()),
        "font" => serialize_shorthand(style),
        _ => return None,
    })
}

/// The `font` shorthand's serialization (CSSOM §6.7.2): when all its
/// longhands are set and the shorthand can express them — the prefix's
/// non-initial values, the size, `/ line-height` unless `normal`, the
/// family — or the system font keyword that set them.
fn serialize_shorthand(style: &TuiStyle) -> Option<String> {
    let f = &style.font;
    let weight = *f.weight.as_ref().and_then(specified)?;
    let font_style = *f.style.as_ref().and_then(specified)?;
    let size = f.size.as_ref().and_then(specified)?;
    let family = f.family.as_ref().and_then(specified)?;
    let stretch = *f.stretch.as_ref().and_then(specified)?;
    let variant = *f.variant.as_ref().and_then(specified)?;
    let line_height = style.text.line_height.as_ref().and_then(specified)?;
    let initial_rest = font_style == FontStyle::Normal
        && variant == FontVariant::Normal
        && weight == FontWeight::Normal
        && stretch == FontStretch::Normal
        && *size == FontSize::Medium
        && *line_height == crate::layout::LineHeight::Normal;
    if let FontFamily::System(system) = family {
        return initial_rest.then(|| system.keyword().to_string());
    }
    if matches!(weight, FontWeight::Bolder | FontWeight::Lighter)
        || matches!(stretch, FontStretch::Percent(_))
    {
        return None;
    }
    let mut words: Vec<String> = Vec::new();
    if font_style != FontStyle::Normal {
        words.push(serialize_font_style(font_style));
    }
    if variant != FontVariant::Normal {
        words.push(serialize_font_variant(variant).to_string());
    }
    if weight != FontWeight::Normal {
        words.push(serialize_font_weight(weight));
    }
    if stretch != FontStretch::Normal {
        words.push(serialize_font_stretch(stretch));
    }
    let mut size_part = serialize_font_size(size);
    if *line_height != crate::layout::LineHeight::Normal {
        size_part.push_str(" / ");
        size_part.push_str(&serialize_line_height(line_height));
    }
    words.push(size_part);
    words.push(serialize_font_family(family)?);
    Some(words.join(" "))
}
