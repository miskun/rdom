//! The border properties (CSS Backgrounds 3 §4): `set` and `serialize`
//! arms for `border`, `border-<side>`, `border-style` (+ per-side),
//! and `border-color`. `border-collapse` is a table property and stays
//! in `set.rs` / `serialize.rs`.

use super::value_serializers::{border_style_keyword, serialize_color, serialize_math, specified};
use crate::layout::{BorderStyle, BorderWidth, CornerStyle, PaintLength, Sides};
use crate::parse::token::Token;
use crate::parse::values::{
    current_border, parse_border, parse_border_side, parse_border_side_shorthand, parse_color,
    parse_line_width, parse_sides,
};
use crate::{TuiColor, TuiStyle, Value};

/// True when `value` is rdom's `rounded` keyword.
fn is_rounded(value: &[Token]) -> bool {
    matches!(value, [Token::Ident(k)] if k.eq_ignore_ascii_case("rounded"))
}

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

    fn style_of(self, b: &crate::layout::Border) -> BorderStyle {
        match self {
            Side::Top => b.top,
            Side::Right => b.right,
            Side::Bottom => b.bottom,
            Side::Left => b.left,
        }
    }

    fn style(self, b: &mut crate::layout::Border) -> &mut BorderStyle {
        match self {
            Side::Top => &mut b.top,
            Side::Right => &mut b.right,
            Side::Bottom => &mut b.bottom,
            Side::Left => &mut b.left,
        }
    }
}

/// The side a `border-<side>` / `border-<side>-style` name addresses,
/// and its suffix after the side.
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

fn spec<T>(v: T) -> Option<Value<T>> {
    Some(Value::Specified(v))
}

/// Parse and write one border property. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    Some(match name {
        // §4.4: every side's style, width and color.
        "border" => parse_border(value).map(|(border, width, color)| {
            style.border = spec(border);
            style.border_width = Sides::all(spec(width));
            style.border_color = Sides::all(spec(color));
        }),
        // §4.2, the whole ring: rdom's one-value `rounded` rounds its
        // corners, anything else squares them.
        "border-style" => parse_sides(value, parse_border_side).map(|s| {
            let mut b = crate::layout::Border {
                top: s.top,
                right: s.right,
                bottom: s.bottom,
                left: s.left,
                corner_style: CornerStyle::Square,
            };
            if is_rounded(value) {
                b.corner_style = CornerStyle::Rounded;
            }
            style.border = spec(b);
        }),
        "border-color" => parse_sides(value, parse_color).map(|c| {
            style.border_color = c.map(spec);
        }),
        "border-width" => parse_sides(value, parse_line_width).map(|w| {
            style.border_width = w.map(spec);
        }),
        _ => {
            let (side, suffix) = side_of(name)?;
            match suffix {
                // §4.4: one side's style, width and color.
                "" => parse_border_side_shorthand(value).map(|s| {
                    let mut b = current_border(style);
                    *side.style(&mut b) = s.style;
                    style.border = spec(b);
                    *side.of(&mut style.border_width) = spec(s.width);
                    *side.of(&mut style.border_color) = spec(s.color);
                }),
                "-style" => parse_border_side(value).map(|s| {
                    let mut b = current_border(style);
                    *side.style(&mut b) = s;
                    style.border = spec(b);
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
    let border = style.border.as_ref().and_then(specified);
    Some(match name {
        "border" => border.and_then(|b| {
            let width = uniform(&style.border_width)?;
            let color = uniform(&style.border_color)?;
            let styles = Sides::new(b.top, b.right, b.bottom, b.left);
            if b.corner_style == CornerStyle::Rounded {
                // rdom's `rounded`: a solid ring with rounded corners.
                return (styles.uniform() && b.top == BorderStyle::Solid)
                    .then(|| line(width, "rounded", color));
            }
            if styles.uniform() {
                return Some(line(width, border_style_keyword(b.top), color));
            }
            // rdom's one-side keywords: that side solid, the rest none,
            // width and color initial.
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
                (b.top, "top"),
                (b.right, "right"),
                (b.bottom, "bottom"),
                (b.left, "left"),
            ]
            .into_iter()
            .find(|(s, _)| *s == BorderStyle::Solid)
            .map(|(_, keyword)| keyword.to_string())
        }),
        "border-style" => border.and_then(|b| {
            let styles = Sides::new(b.top, b.right, b.bottom, b.left);
            match b.corner_style {
                CornerStyle::Rounded => {
                    (styles.uniform() && b.top == BorderStyle::Solid).then(|| "rounded".to_string())
                }
                CornerStyle::Square => Some(shortest_sides(
                    styles.map(|s| border_style_keyword(s).to_string()),
                )),
            }
        }),
        "border-color" => {
            all_specified(&style.border_color).map(|c| shortest_sides(c.map(serialize_color)))
        }
        "border-width" => {
            all_specified(&style.border_width).map(|w| shortest_sides(w.map(serialize_line_width)))
        }
        _ => {
            let (side, suffix) = side_of(name)?;
            let side_style = border.map(|b| side.style_of(b));
            let width = side.get(&style.border_width).as_ref().and_then(specified);
            let color = side.get(&style.border_color).as_ref().and_then(specified);
            match suffix {
                "" => side_style
                    .zip(width)
                    .zip(color)
                    .map(|((s, w), c)| line(w, border_style_keyword(s), c)),
                "-style" => side_style.map(|s| border_style_keyword(s).to_string()),
                "-color" => color.map(serialize_color),
                "-width" => width.map(serialize_line_width),
                _ => return None,
            }
        }
    })
}

/// Every side's specified value, if each side has one.
fn all_specified<T>(sides: &Sides<Option<Value<T>>>) -> Option<Sides<&T>> {
    let [t, r, b, l] = sides.each().map(|s| s.as_ref().and_then(specified));
    Some(Sides::new(t?, r?, b?, l?))
}

/// One to four side values in the shortest form that expands back to
/// the same sides (CSSOM §6.7.2; CSS Backgrounds 3 §4.1).
fn shortest_sides(s: Sides<String>) -> String {
    let Sides {
        top,
        right,
        bottom,
        left,
    } = s;
    if left != right {
        format!("{top} {right} {bottom} {left}")
    } else if top != bottom {
        format!("{top} {right} {bottom}")
    } else if top != right {
        format!("{top} {right}")
    } else {
        top
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
