//! Per-property value parsers — the leaves of the CSS dispatch
//! table.
//!
//! Each `parse_*` function takes a `&[Token]` (whitespace already
//! eaten by the tokenizer) and returns `Option<T>` where `T` is the
//! value type being parsed. `None` means "invalid input"; the caller
//! turns that into a warning / `DispatchError::InvalidValue`.
//!
//! Lives in `rdom-style` so the block parser (in `rdom-css`) and
//! `property_dispatch::set_from_tokens` (in this crate) can both
//! consume the same per-property parsing.
//!
//! One file per value family:
//! - `color.rs` — colors, `rgb()` / `rgba()` / `var()`.
//! - `keyword.rs` — the keyword-table matcher and single-keyword enums.
//! - `number.rs` — `opacity`, unsigned counts, `z-index`, `aspect-ratio`.
//! - `length.rs` — sizes, `flex`, `min-*`, signed lengths, `inset`.
//! - `spacing.rs` — `gap`, `padding`, `margin`.
//! - `border.rs` — `border` shorthand and per-side styles.
//! - `content.rs` — `content` and counter operations.
//! - `transition.rs` — easing, `<time>`, transition lists and shorthand.
//! - `calc.rs` — the `calc()` expression parser.
//!
//! Every parser is re-exported here, so `parse::values::parse_*`
//! stays the single public path.

mod border;
mod calc;
mod color;
mod content;
mod keyword;
mod length;
mod number;
mod spacing;
mod transition;

pub use border::{current_border, parse_border, parse_border_side};
pub use calc::{looks_like_calc, parse_calc};
pub use color::{parse_color, parse_color_at, parse_rgb_args, parse_rgba_args, parse_var_args};
pub use content::{parse_content, parse_counter_ops};
pub use keyword::{
    parse_keyword, parse_overflow, parse_position, parse_scrollbar_gutter, parse_text_decoration,
};
pub use length::{
    parse_flex_shorthand, parse_inset_shorthand, parse_length, parse_min_size, parse_size,
};
pub use number::{parse_aspect_ratio, parse_opacity, parse_unsigned, parse_z_index};
pub use spacing::{
    current_margin, current_padding, parse_gap, parse_margin_longhand, parse_margin_shorthand,
    parse_padding_shorthand, parse_padding_value,
};
pub use transition::{
    TransitionShorthandRule, parse_animatable_property, parse_time_list, parse_time_ms,
    parse_timing_function_at, parse_timing_function_keyword, parse_timing_function_list,
    parse_transition_property_keyword, parse_transition_property_list, parse_transition_shorthand,
    parse_transition_shorthand_single, unzip_transition_rules,
};

use crate::parse::token::Token;

/// Render a `&[Token]` slice back to its source-like string form.
/// Used by the block parser's `InvalidValue` warning path.
pub fn render_value(value: &[Token]) -> String {
    let mut out = String::new();
    for (i, t) in value.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        match t {
            Token::Ident(s) => out.push_str(s),
            Token::Number(n) => out.push_str(&n.to_string()),
            Token::Float(f) => out.push_str(&f.to_string()),
            Token::Percentage(n) => {
                out.push_str(&n.to_string());
                out.push('%');
            }
            Token::String(s) => {
                out.push('"');
                out.push_str(s);
                out.push('"');
            }
            Token::HexColor(h) => {
                out.push('#');
                out.push_str(h);
            }
            Token::Function(name) => {
                out.push_str(name);
                out.push('(');
            }
            Token::Colon => out.push(':'),
            Token::Semicolon => out.push(';'),
            Token::Comma => out.push(','),
            Token::Bang => out.push('!'),
            Token::LParen => out.push('('),
            Token::RParen => out.push(')'),
            Token::Delim(c) => out.push(*c),
        }
    }
    out
}
