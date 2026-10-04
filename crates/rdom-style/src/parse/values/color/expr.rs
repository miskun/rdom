//! The parsed form of a color function that needs the element
//! (`TuiColor::Function`): what is known at parse time is folded into
//! absolute colors, and what depends on the element — `currentcolor`,
//! the canvas system colors, `light-dark()` — stays a node, as do the
//! `color-mix()` and relative colors above one. Computing it for an
//! element evaluates the tree; nothing is tokenized or parsed again.

use std::sync::Arc;

use super::{mix, relative};
use crate::color::{AbsoluteColor, ColorScheme, SystemColor, system};
use crate::parse::token::Token;
use crate::{Color, ColorContext};

/// A color value inside a color function.
#[derive(Debug, Clone)]
pub(crate) enum ColorExpr {
    /// Known at parse time.
    Absolute(AbsoluteColor),
    /// `currentcolor` (CSS Color 4 §6.4).
    CurrentColor,
    /// A system color that is the terminal's default (`Canvas`,
    /// `CanvasText`, …): the canvas model's color for the element's
    /// scheme.
    Canvas(SystemColor),
    /// `light-dark(light, dark)` (CSS Color 5 §5.1).
    LightDark(Box<ColorExpr>, Box<ColorExpr>),
    /// `color-mix()` with an argument that needs the element.
    Mix(Box<Mix>),
    /// A relative color whose origin needs the element.
    Relative(Box<Relative>),
}

/// `color-mix()`'s parsed arguments (CSS Color 5 §2).
#[derive(Debug, Clone)]
pub(crate) struct Mix {
    pub method: mix::Method,
    pub colors: [ColorExpr; 2],
}

/// A relative color (CSS Color 5 §4): the function, its origin, and the
/// channel arguments after the origin, bound to the origin's channels
/// when computed.
#[derive(Debug, Clone)]
pub(crate) struct Relative {
    /// The function's name, lower case.
    pub name: Box<str>,
    pub origin: ColorExpr,
    pub channels: Arc<[Token]>,
}

impl ColorExpr {
    /// The color for an element (`cx`).
    pub fn eval(&self, cx: &ColorContext) -> Option<AbsoluteColor> {
        match self {
            ColorExpr::Absolute(c) => Some(*c),
            // The terminal's default foreground is the canvas model's
            // text for the scheme.
            ColorExpr::CurrentColor => match cx.current_color {
                Color::Reset => AbsoluteColor::from_color(system::canvas(cx.scheme).1),
                c => AbsoluteColor::from_color(c),
            },
            ColorExpr::Canvas(s) => AbsoluteColor::from_color(s.definite(cx.scheme)),
            ColorExpr::LightDark(light, dark) => match cx.scheme {
                ColorScheme::Light => light.eval(cx),
                ColorScheme::Dark => dark.eval(cx),
            },
            ColorExpr::Mix(m) => Some(m.method.apply(m.colors[0].eval(cx)?, m.colors[1].eval(cx)?)),
            ColorExpr::Relative(r) => relative::apply(&r.name, r.origin.eval(cx)?, &r.channels),
        }
    }
}
