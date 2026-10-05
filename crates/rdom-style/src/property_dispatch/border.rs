//! The border properties (CSS Backgrounds 3 §4–§5): `set` and
//! `serialize` arms for `border`, `border-<side>`, `border-style` /
//! `-color` / `-width` and their per-side longhands, and
//! `border-radius` and its per-corner longhands, and `border-spacing`.
//! `border-collapse` stays in `set.rs` / `serialize.rs`.

use super::value_serializers::{
    all_specified, border_style_keyword, serialize_color, serialize_math, shortest_sides, specified,
};
use crate::layout::{
    BorderRadius, BorderStyle, BorderWidth, Corners, GapValue, PaintLength, Sides,
};
use crate::parse::token::Token;
use crate::parse::values::{
    parse_border, parse_border_radius, parse_border_side, parse_border_side_shorthand,
    parse_border_spacing, parse_color, parse_corner_radius, parse_line_width, parse_sides,
};
use crate::{TuiColor, TuiStyle, Value};

/// Which side a per-side property name addresses.
#[derive(Clone, Copy)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

impl Side {
    fn of<T>(self, sides: &mut Sides<T>) -> &mut T {
        match self {
            Side::Top => &mut sides.top,
            Side::Right => &mut sides.right,
            Side::Bottom => &mut sides.bottom,
            Side::Left => &mut sides.left,
        }
    }

    fn get<T>(self, sides: &Sides<T>) -> &T {
        match self {
            Side::Top => &sides.top,
            Side::Right => &sides.right,
            Side::Bottom => &sides.bottom,
            Side::Left => &sides.left,
        }
    }
}

/// The side a `border-<side>…` name addresses, and its suffix after the
/// side.
fn side_of(name: &str) -> Option<(Side, &str)> {
    let rest = name.strip_prefix("border-")?;
    [
        ("top", Side::Top),
        ("right", Side::Right),
        ("bottom", Side::Bottom),
        ("left", Side::Left),
    ]
    .into_iter()
    .find_map(|(n, side)| rest.strip_prefix(n).map(|suffix| (side, suffix)))
}

/// The corner field a `border-<corner>-radius` name addresses.
fn corner_of<'c, T>(name: &str, corners: &'c mut Corners<T>) -> Option<&'c mut T> {
    Some(match name {
        "border-top-left-radius" => &mut corners.top_left,
        "border-top-right-radius" => &mut corners.top_right,
        "border-bottom-right-radius" => &mut corners.bottom_right,
        "border-bottom-left-radius" => &mut corners.bottom_left,
        _ => return None,
    })
}

fn spec<T>(v: T) -> Option<Value<T>> {
    Some(Value::Specified(v))
}

/// Parse and write one border property. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    Some(match name {
        // §4.4: every side's style, width and color. rdom's `rounded`
        // also rounds the corners: `border: solid; border-radius: 1`.
        "border" => parse_border(value).map(|ring| {
            style.border_style = ring.styles.sides().map(spec);
            style.border_width = Sides::all(spec(ring.width));
            style.border_color = Sides::all(spec(ring.color));
            if ring.rounded {
                style.border_radius = Corners::all(spec(BorderRadius::cells(1.0)));
            }
        }),
        "border-style" => parse_sides(value, parse_border_side).map(|s| {
            style.border_style = s.map(spec);
        }),
        "border-color" => parse_sides(value, parse_color).map(|c| {
            style.border_color = c.map(spec);
        }),
        "border-width" => parse_sides(value, parse_line_width).map(|w| {
            style.border_width = w.map(spec);
        }),
        "border-radius" => parse_border_radius(value).map(|r| {
            style.border_radius = r.map(spec);
        }),
        "border-spacing" => parse_border_spacing(value).map(|s| {
            style.border_spacing = spec(s);
        }),
        _ if name.ends_with("-radius") => {
            let r = parse_corner_radius(value);
            let corner = corner_of(name, &mut style.border_radius)?;
            r.map(|r| *corner = spec(r))
        }
        _ => {
            let (side, suffix) = side_of(name)?;
            match suffix {
                // §4.4: one side's style, width and color.
                "" => parse_border_side_shorthand(value).map(|s| {
                    *side.of(&mut style.border_style) = spec(s.style);
                    *side.of(&mut style.border_width) = spec(s.width);
                    *side.of(&mut style.border_color) = spec(s.color);
                }),
                "-style" => parse_border_side(value).map(|s| {
                    *side.of(&mut style.border_style) = spec(s);
                }),
                "-color" => parse_color(value).map(|c| {
                    *side.of(&mut style.border_color) = spec(c);
                }),
                "-width" => parse_line_width(value).map(|w| {
                    *side.of(&mut style.border_width) = spec(w);
                }),
                _ => return None,
            }
        }
    })
}

/// Serialize one border property. `None` when `name` is not one;
/// `Some(None)` when it is unset or its longhands cannot be written as
/// it.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    Some(match name {
        "border" => serialize_border(style),
        "border-style" => all_specified(&style.border_style)
            .map(|s| shortest_sides(s.map(|s| border_style_keyword(*s).to_string()))),
        "border-color" => {
            all_specified(&style.border_color).map(|c| shortest_sides(c.map(serialize_color)))
        }
        "border-width" => {
            all_specified(&style.border_width).map(|w| shortest_sides(w.map(serialize_line_width)))
        }
        "border-radius" => serialize_border_radius(&style.border_radius),
        "border-spacing" => style.border_spacing.as_ref().and_then(specified).map(|s| {
            let gap = |g: &GapValue| match g {
                GapValue::Cells(n) => n.to_string(),
                GapValue::Calc(e) => serialize_math(e),
            };
            if s.horizontal == s.vertical {
                gap(&s.horizontal)
            } else {
                format!("{} {}", gap(&s.horizontal), gap(&s.vertical))
            }
        }),
        _ if name.ends_with("-radius") => {
            let mut radii = style.border_radius.clone();
            corner_of(name, &mut radii)?
                .as_ref()
                .and_then(specified)
                .map(serialize_corner_radius)
        }
        _ => {
            let (side, suffix) = side_of(name)?;
            let line_style = side.get(&style.border_style).as_ref().and_then(specified);
            let width = side.get(&style.border_width).as_ref().and_then(specified);
            let color = side.get(&style.border_color).as_ref().and_then(specified);
            match suffix {
                "" => line_style
                    .zip(width)
                    .zip(color)
                    .map(|((s, w), c)| line(w, border_style_keyword(*s), c)),
                "-style" => line_style.map(|s| border_style_keyword(*s).to_string()),
                "-color" => color.map(serialize_color),
                "-width" => width.map(serialize_line_width),
                _ => return None,
            }
        }
    })
}

/// `border`: every side's style, width and color agree; or rdom's
/// one-side keyword form (one side solid, the rest none, width and
/// color initial).
fn serialize_border(style: &TuiStyle) -> Option<String> {
    let styles = all_specified(&style.border_style)?.map(|s| *s);
    let width = uniform(&style.border_width)?;
    let color = uniform(&style.border_color)?;
    if styles.uniform() {
        return Some(line(width, border_style_keyword(styles.top), color));
    }
    let initial = *width == BorderWidth::Medium && *color == TuiColor::CurrentColor;
    let nones = styles
        .each()
        .iter()
        .filter(|s| ***s == BorderStyle::None)
        .count();
    if !initial || nones != 3 {
        return None;
    }
    [
        (styles.top, "top"),
        (styles.right, "right"),
        (styles.bottom, "bottom"),
        (styles.left, "left"),
    ]
    .into_iter()
    .find(|(s, _)| *s == BorderStyle::Solid)
    .map(|(_, keyword)| keyword.to_string())
}

/// `border-radius`: the horizontal radii in their shortest form, then
/// `/` and the vertical ones when they differ (CSSOM §6.7.2).
fn serialize_border_radius(radii: &Corners<Option<Value<BorderRadius>>>) -> Option<String> {
    let [tl, tr, br, bl] = radii.each().map(|r| r.as_ref().and_then(specified));
    let radii = Corners::new(tl?, tr?, br?, bl?);
    let axis = |f: fn(&BorderRadius) -> &PaintLength| {
        let [tl, tr, br, bl] = radii.each().map(|r| serialize_paint_length(f(r)));
        shortest_sides(Sides::new(tl, tr, br, bl))
    };
    let horizontal = axis(|r| &r.horizontal);
    let vertical = axis(|r| &r.vertical);
    Some(if radii.each().iter().all(|r| r.horizontal == r.vertical) {
        horizontal
    } else {
        format!("{horizontal} / {vertical}")
    })
}

/// One corner's radius: one value when both axes agree.
fn serialize_corner_radius(r: &BorderRadius) -> String {
    if r.horizontal == r.vertical {
        serialize_paint_length(&r.horizontal)
    } else {
        format!(
            "{} {}",
            serialize_paint_length(&r.horizontal),
            serialize_paint_length(&r.vertical)
        )
    }
}

/// The one specified value every side holds, if they agree.
fn uniform<T: PartialEq>(sides: &Sides<Option<Value<T>>>) -> Option<&T> {
    let top = sides.top.as_ref().and_then(specified)?;
    sides
        .each()
        .iter()
        .all(|s| s.as_ref().and_then(specified) == Some(top))
        .then_some(top)
}

/// `<line-width> || <line-style> || <color>` in its shortest form:
/// the components that are not initial, `none` when none is.
fn line(width: &BorderWidth, style: &str, color: &TuiColor) -> String {
    let mut parts = Vec::with_capacity(3);
    if *width != BorderWidth::Medium {
        parts.push(serialize_line_width(width));
    }
    if style != "none" {
        parts.push(style.to_string());
    }
    if *color != TuiColor::CurrentColor {
        parts.push(serialize_color(color));
    }
    if parts.is_empty() {
        "none".to_string()
    } else {
        parts.join(" ")
    }
}

/// `<line-width>` as CSS text: a keyword, cells (a fraction as `ch`,
/// which is one cell and keeps the fraction), pixels or a math
/// function.
pub(super) fn serialize_line_width(w: &BorderWidth) -> String {
    match w {
        BorderWidth::Thin => "thin".to_string(),
        BorderWidth::Medium => "medium".to_string(),
        BorderWidth::Thick => "thick".to_string(),
        BorderWidth::Length(l) => serialize_paint_length(l),
    }
}

/// A [`PaintLength`] as CSS text.
pub(super) fn serialize_paint_length(l: &PaintLength) -> String {
    match l {
        PaintLength::Cells(c) if c.fract() == 0.0 => format!("{c}"),
        PaintLength::Cells(c) => format!("{c}ch"),
        PaintLength::Px(p) => format!("{p}px"),
        PaintLength::Calc(e) => serialize_math(e),
    }
}
