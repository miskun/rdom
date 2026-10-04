//! The background properties (CSS Backgrounds 3 §3): `set` and
//! `serialize` arms for the `background` shorthand and its longhands.
//! `background-color` is a color property and stays with the others in
//! `set.rs` / `serialize.rs`; the shorthand writes it from here.

use super::value_serializers::{join_csv, serialize_color, specified};
use crate::layout::{BackgroundRepeat, RepeatStyle, VisualBox};
use crate::parse::token::Token;
use crate::parse::values::{
    BackgroundLayer, INITIAL_CLIP, INITIAL_IMAGE, INITIAL_ORIGIN, INITIAL_POSITION, INITIAL_SIZE,
    parse_background, parse_background_attachment, parse_background_image,
    parse_background_position, parse_background_repeat, parse_background_size,
    parse_visual_box_list,
};
use crate::{Color, TuiColor, TuiStyle, Value};

/// Parse and write one background property. `None` when `name` is not
/// one; `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    Some(match name {
        "background" => parse_background(value).map(|b| {
            let layers = &b.layers;
            let column = |f: fn(&BackgroundLayer) -> String| layers.iter().map(f).collect();
            style.background_image = spec(column(|l| l.image.clone()));
            style.background_position = spec(column(|l| l.position.clone()));
            style.background_size = spec(column(|l| l.size.clone()));
            style.background_repeat = spec(layers.iter().map(|l| l.repeat).collect());
            style.background_attachment = spec(layers.iter().map(|l| l.attachment).collect());
            style.background_origin = spec(layers.iter().map(|l| l.origin).collect());
            style.background_clip = spec(layers.iter().map(|l| l.clip).collect());
            // An omitted color is the initial `transparent` (§3.10).
            style.bg = spec(b.color.unwrap_or(TuiColor::Literal(Color::TRANSPARENT)));
        }),
        "background-image" => parse_background_image(value).map(|v| {
            style.background_image = spec(v);
        }),
        "background-position" => parse_background_position(value).map(|v| {
            style.background_position = spec(v);
        }),
        "background-size" => parse_background_size(value).map(|v| {
            style.background_size = spec(v);
        }),
        "background-repeat" => parse_background_repeat(value).map(|v| {
            style.background_repeat = spec(v);
        }),
        "background-attachment" => parse_background_attachment(value).map(|v| {
            style.background_attachment = spec(v);
        }),
        "background-origin" => parse_visual_box_list(value).map(|v| {
            style.background_origin = spec(v);
        }),
        "background-clip" => parse_visual_box_list(value).map(|v| {
            style.background_clip = spec(v);
        }),
        _ => return None,
    })
}

fn spec<T>(v: T) -> Option<Value<T>> {
    Some(Value::Specified(v))
}

/// Serialize one background property. `None` when `name` is not one;
/// `Some(None)` when it is unset (or a shorthand its longhands cannot
/// express).
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let texts =
        |v: &Option<Value<Vec<String>>>| v.as_ref().and_then(specified).map(|list| list.join(", "));
    Some(match name {
        "background" => serialize_shorthand(style),
        "background-image" => texts(&style.background_image),
        "background-position" => texts(&style.background_position),
        "background-size" => texts(&style.background_size),
        "background-repeat" => style
            .background_repeat
            .as_ref()
            .and_then(specified)
            .map(|l| join_csv(l.iter(), |r| repeat_text(*r))),
        "background-attachment" => style
            .background_attachment
            .as_ref()
            .and_then(specified)
            .map(|l| join_csv(l.iter(), |a| a.keyword().to_string())),
        "background-origin" => boxes(&style.background_origin),
        "background-clip" => boxes(&style.background_clip),
        _ => return None,
    })
}

fn boxes(v: &Option<Value<Vec<VisualBox>>>) -> Option<String> {
    v.as_ref()
        .and_then(specified)
        .map(|l| join_csv(l.iter(), |b| b.keyword().to_string()))
}

/// `<repeat-style>` in its shortest form (CSSOM: `repeat-x`, one
/// keyword when both axes agree).
fn repeat_text(r: BackgroundRepeat) -> String {
    use RepeatStyle::{NoRepeat, Repeat};
    match (r.x, r.y) {
        (Repeat, NoRepeat) => "repeat-x".to_string(),
        (NoRepeat, Repeat) => "repeat-y".to_string(),
        (x, y) if x == y => x.keyword().to_string(),
        (x, y) => format!("{} {}", x.keyword(), y.keyword()),
    }
}

/// `background` from its longhands — every one specified, with one
/// entry per layer — each layer's non-initial sub-values in the
/// grammar's order, the color last on the final layer (CSSOM §6.7.2:
/// the shortest form that reads back the same). A layer with nothing
/// to say is `none`.
fn serialize_shorthand(style: &TuiStyle) -> Option<String> {
    let image = style.background_image.as_ref().and_then(specified)?;
    let position = style.background_position.as_ref().and_then(specified)?;
    let size = style.background_size.as_ref().and_then(specified)?;
    let repeat = style.background_repeat.as_ref().and_then(specified)?;
    let attachment = style.background_attachment.as_ref().and_then(specified)?;
    let origin = style.background_origin.as_ref().and_then(specified)?;
    let clip = style.background_clip.as_ref().and_then(specified)?;
    let color = style.bg.as_ref().and_then(specified)?;
    let n = image.len();
    if [
        position.len(),
        size.len(),
        repeat.len(),
        attachment.len(),
        origin.len(),
        clip.len(),
    ]
    .iter()
    .any(|&len| len != n)
    {
        return None;
    }
    let mut layers = Vec::with_capacity(n);
    for i in 0..n {
        let mut parts: Vec<String> = Vec::new();
        if image[i] != INITIAL_IMAGE {
            parts.push(image[i].clone());
        }
        if size[i] != INITIAL_SIZE {
            parts.push(format!("{} / {}", position[i], size[i]));
        } else if position[i] != INITIAL_POSITION {
            parts.push(position[i].clone());
        }
        if repeat[i] != BackgroundRepeat::default() {
            parts.push(repeat_text(repeat[i]));
        }
        if attachment[i] != Default::default() {
            parts.push(attachment[i].keyword().to_string());
        }
        if origin[i] == clip[i] {
            if origin[i] != INITIAL_ORIGIN || clip[i] != INITIAL_CLIP {
                parts.push(origin[i].keyword().to_string());
            }
        } else if (origin[i], clip[i]) != (INITIAL_ORIGIN, INITIAL_CLIP) {
            parts.push(format!("{} {}", origin[i].keyword(), clip[i].keyword()));
        }
        if i + 1 == n && *color != TuiColor::Literal(Color::TRANSPARENT) {
            parts.push(serialize_color(color));
        }
        layers.push(if parts.is_empty() {
            INITIAL_IMAGE.to_string()
        } else {
            parts.join(" ")
        });
    }
    Some(layers.join(", "))
}
