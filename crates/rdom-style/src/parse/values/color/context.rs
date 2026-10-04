//! What a color nested in a color function resolves against.
//!
//! At parse time there is no element: a `currentcolor` inside a
//! function (`color-mix(in srgb, currentcolor, blue)`), a
//! `light-dark()`, or a system color that is the terminal's default
//! stands in with a placeholder so the rest of the function is still
//! checked, and the parse records that the value needs the element —
//! it is then kept, as written, for the cascade to compute
//! (`TuiColor::Function`). At computed-value time the context holds the
//! element's color and color scheme.

use std::cell::Cell;

use crate::color::{AbsoluteColor, ColorScheme, SystemColor, system};
use crate::parse::token::Token;
use crate::{Color, ColorContext};

/// The context of one color parse.
pub(super) struct ColorCx {
    /// The element's color and color scheme, when there is an element.
    element: Option<(Color, ColorScheme)>,
    /// Set when the parse met something only an element resolves.
    needs_element: Cell<bool>,
}

impl ColorCx {
    /// Parsing a declaration: no element.
    pub fn parse_time() -> Self {
        ColorCx {
            element: None,
            needs_element: Cell::new(false),
        }
    }

    /// Computing a kept color function for an element.
    pub fn computed(context: &ColorContext) -> Self {
        ColorCx {
            element: Some((context.current_color, context.scheme)),
            needs_element: Cell::new(false),
        }
    }

    /// The element's color scheme (`light-dark()`, the canvas model) —
    /// at parse time, a stand-in.
    pub fn scheme(&self) -> ColorScheme {
        match self.element {
            Some((_, scheme)) => scheme,
            None => {
                self.needs_element.set(true);
                ColorScheme::default()
            }
        }
    }

    /// A system color inside a function: the terminal's defaults take
    /// the canvas model's values for the element's scheme.
    pub fn system(&self, color: SystemColor) -> Option<AbsoluteColor> {
        if color.is_canvas() {
            AbsoluteColor::from_color(color.definite(self.scheme()))
        } else {
            AbsoluteColor::from_color(color.color())
        }
    }

    /// True when the parse met something only an element resolves.
    pub fn needs_element(&self) -> bool {
        self.needs_element.get()
    }

    /// `currentcolor` inside a function: the element's color — the
    /// terminal's default foreground (`reset`) as the canvas model's
    /// text for its scheme, CSS Color 4 §6.4 — or, at parse time, a
    /// stand-in.
    pub fn current_color(&self) -> Option<AbsoluteColor> {
        match self.element {
            Some((Color::Reset, scheme)) => AbsoluteColor::from_color(system::canvas(scheme).1),
            Some((color, _)) => AbsoluteColor::from_color(color),
            None => {
                self.needs_element.set(true);
                AbsoluteColor::from_color(Color::Rgb(0, 0, 0))
            }
        }
    }

    /// A literal color inside a function. `reset` names no one color
    /// (it is the terminal's default foreground or background,
    /// depending on the property), so it is invalid there.
    pub fn absolute(&self, color: Color) -> Option<AbsoluteColor> {
        AbsoluteColor::from_color(color)
    }
}

/// `tokens` (one color function) as CSS text: tokens separated by one
/// space, except after `(` and before `)` and `,`.
pub(super) fn render(tokens: &[Token]) -> String {
    let mut out = String::new();
    let mut prev: Option<&Token> = None;
    for t in tokens {
        let tight = matches!(prev, None | Some(Token::Function(_) | Token::LParen))
            || matches!(t, Token::RParen | Token::Comma);
        if !tight {
            out.push(' ');
        }
        out.push_str(&crate::parse::values::render_value(std::slice::from_ref(t)));
        prev = Some(t);
    }
    out
}
