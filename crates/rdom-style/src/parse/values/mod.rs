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
//! - `background.rs` — the `background` shorthand and its longhands.
//! - `border.rs` — `border` shorthand and per-side styles.
//! - `shadow.rs` — `box-shadow`.
//! - `content.rs` — `content` and counter operations.
//! - `grid.rs` — grid track lists (`grid-template-*`, `grid-auto-*`).
//! - `grid_areas.rs` — named grid areas (`grid-template-areas`).
//! - `grid_placement.rs` — grid placement (`grid-row` / `-column` /
//!   `-area` and their longhands, `grid-auto-flow`).
//! - `grid_shorthand.rs` — the `grid-template` and `grid` shorthands.
//! - `font.rs` — CSS Fonts 4: the font properties and the `font`
//!   shorthand.
//! - `inline.rs` — CSS Inline 3: `line-height`, `vertical-align`.
//! - `text_decoration.rs` — `text-decoration` and its longhands, the
//!   underline placement properties.
//! - `filter.rs` — Filter Effects 1 / 2: `filter`, `backdrop-filter`.
//! - `blend.rs` — Compositing and Blending: `mix-blend-mode`, `isolation`,
//!   `background-blend-mode`.
//! - `transform.rs` — CSS Transforms 1 / 2: `translate`, `transform`,
//!   `rotate`, `scale`, `transform-origin`, `transform-box`.
//! - `transition.rs` — easing, `<time>`, transition lists and shorthand.
//! - `animation.rs` — the `animation-*` lists and the `animation` shorthand.
//! - `calc.rs` — the `calc()` expression parser.
//! - `numeric.rs` — the shared `<length-percentage>` leaf and the
//!   component-value splitter.
//!
//! Every parser is re-exported here, so `parse::values::parse_*`
//! stays the single public path.

mod align;
mod animation;
mod background;
mod blend;
mod border;
mod calc;
mod calc_size;
mod color;
mod containment;
mod content;
mod cursor;
mod display;
mod easing;
mod filter;
mod flex;
mod float;
mod font;
mod grid;
mod grid_areas;
mod grid_placement;
mod grid_shorthand;
mod inline;
mod keyword;
mod length;
mod line_clamp;
mod list;
mod number;
mod numeric;
mod outline;
mod scroll;
mod scrollbar;

pub(crate) use numeric::{
    LengthPercentage, Range, components, integer, length_percentage, number, percentage,
};
pub use numeric::{MAX_ANGLE_DEGREES, parse_angle};
mod shadow;
mod spacing;
mod table;
mod text;
mod text_decoration;
mod timeline;
mod transform;
mod transition;

pub use align::{
    align_keyword, parse_align_content, parse_align_items, parse_align_self, parse_justify_content,
    parse_justify_items, parse_justify_self, parse_place_content, parse_place_items,
    parse_place_self, serialize_alignment, serialize_place,
};
pub use animation::keyframes_name;
pub(crate) use animation::{
    AnimationPiece, parse_animation_duration_list, parse_animation_name_list,
    parse_animation_shorthand, parse_composition_list, parse_direction_list, parse_fill_mode_list,
    parse_iteration_count_list, parse_play_state_list, parse_timeline_list,
};
pub use background::{
    BackgroundLayer, BackgroundShorthand, parse_background, parse_background_attachment,
    parse_background_image, parse_background_position, parse_background_repeat,
    parse_background_size, parse_visual_box_list,
};
pub(crate) use background::{
    INITIAL_CLIP, INITIAL_IMAGE, INITIAL_ORIGIN, INITIAL_POSITION, INITIAL_SIZE,
};
pub use blend::{parse_background_blend_mode, parse_isolation, parse_mix_blend_mode};
pub use border::{
    BorderRing, BorderShorthand, parse_border, parse_border_radius, parse_border_side,
    parse_border_side_shorthand, parse_border_spacing, parse_corner_radius, parse_line_width,
    parse_sides,
};
pub(crate) use calc::parse_pixel_calc;
pub use calc::{MAX_CALC_DEPTH, MAX_CALC_NESTING, looks_like_calc, parse_calc};
pub use calc_size::{parse_calc_size, parse_interpolate_size};
pub(crate) use color::{ColorExpr, compute_function as compute_color_function};
pub use color::{MAX_COLOR_NESTING, parse_color, parse_color_at, parse_rgb_args, parse_rgba_args};
pub use containment::{
    parse_contain, parse_container, parse_container_name, parse_container_type, parse_will_change,
};
pub use content::{parse_content, parse_counter_ops, parse_quotes};
pub use cursor::{
    parse_accent_color, parse_appearance, parse_caret, parse_caret_animation, parse_caret_color,
    parse_caret_shape, parse_cursor, parse_field_sizing, parse_resize,
};
pub use display::{is_legacy_box, parse_display, serialize_display};
pub use easing::{
    parse_timing_function_at, parse_timing_function_keyword, parse_timing_function_list,
};
pub use filter::parse_filter;
pub use flex::{
    parse_flex_direction, parse_flex_flow, parse_flex_wrap, serialize_flex_direction,
    serialize_flex_flow, serialize_flex_wrap,
};
pub use float::{parse_clear, parse_float};
pub use font::{
    FontShorthand, parse_font, parse_font_family, parse_font_size, parse_font_stretch,
    parse_font_style, parse_font_variant, parse_font_weight, serialize_font_family,
    serialize_font_size, serialize_font_stretch, serialize_font_style, serialize_font_variant,
    serialize_font_weight,
};
pub(crate) use grid::is_line_name;
pub use grid::{
    parse_grid_template, parse_track_size, parse_track_sizes, serialize_grid_template,
    serialize_track_size, serialize_track_sizes,
};
pub use grid_areas::{parse_grid_template_areas, serialize_grid_template_areas};
pub use grid_placement::{
    parse_grid_area, parse_grid_auto_flow, parse_grid_line, parse_grid_line_pair,
    serialize_grid_area, serialize_grid_auto_flow, serialize_grid_line, serialize_grid_line_pair,
};
pub use grid_shorthand::{
    GridShorthand, GridTemplateShorthand, parse_grid_shorthand, parse_grid_template_shorthand,
    serialize_grid_shorthand, serialize_grid_template_shorthand,
};
pub use inline::{
    parse_line_height, parse_vertical_align, serialize_line_height, serialize_vertical_align,
};
pub use keyword::{
    parse_keyword, parse_overflow, parse_overflow_clip_margin, parse_overflow_shorthand,
    parse_position, parse_scroll_behavior, parse_text_overflow,
};
pub use length::{
    FlexShorthand, parse_contain_intrinsic, parse_flex_basis, parse_flex_factor,
    parse_flex_shorthand, parse_inset_shorthand, parse_length, parse_max_size, parse_min_size,
    parse_size,
};
pub use line_clamp::{
    parse_block_ellipsis, parse_box_orient, parse_continue, parse_line_clamp, parse_max_lines,
    parse_webkit_line_clamp,
};
pub use list::{
    parse_list_style, parse_list_style_image, parse_list_style_position, parse_list_style_type,
    parse_marker_side,
};
pub use number::{parse_aspect_ratio, parse_opacity, parse_order, parse_z_index};
pub use outline::{parse_outline, parse_outline_color, parse_outline_offset, parse_outline_style};
pub use scroll::{
    parse_overscroll_behavior, parse_overscroll_behavior_shorthand, parse_scroll_margin,
    parse_scroll_margin_shorthand, parse_scroll_padding, parse_scroll_padding_shorthand,
    parse_scroll_snap_align, parse_scroll_snap_stop, parse_scroll_snap_type,
};
pub use scrollbar::{parse_scrollbar_color, parse_scrollbar_gutter, parse_scrollbar_width};
pub use shadow::parse_box_shadow;
pub use spacing::{
    parse_gap, parse_gap_shorthand, parse_margin_longhand, parse_margin_shorthand,
    parse_margin_trim, parse_padding_shorthand, parse_padding_value,
};
pub use table::{parse_caption_side, parse_empty_cells, parse_table_layout};
pub use text::{
    parse_hyphens, parse_line_break, parse_overflow_wrap, parse_spacing, parse_tab_size,
    parse_text_align, parse_text_align_all, parse_text_align_last, parse_text_indent,
    parse_text_justify, parse_text_transform, parse_text_wrap, parse_text_wrap_mode,
    parse_text_wrap_style, parse_white_space, parse_white_space_collapse, parse_word_break,
    serialize_spacing, serialize_text_transform,
};
pub use text_decoration::{
    TextDecorationShorthand, parse_text_decoration, parse_text_decoration_line,
    parse_text_decoration_skip_ink, parse_text_decoration_style, parse_text_decoration_thickness,
    parse_text_underline_offset, parse_text_underline_position, serialize_decoration_length,
    serialize_text_decoration_thickness,
};
pub(crate) use timeline::{
    parse_animation_range, parse_range_end_list, parse_range_start_list, parse_scroll_timeline,
    parse_timeline_axis_list, parse_timeline_inset_list, parse_timeline_name_list,
    parse_timeline_scope, parse_view_timeline,
};
pub use transform::{
    parse_rotate, parse_scale, parse_transform, parse_transform_box, parse_transform_origin,
    parse_translate,
};
pub use transition::{
    TransitionShorthandRule, parse_duration_list, parse_signed_time_ms, parse_time_list,
    parse_time_ms, parse_transition_behavior_list, parse_transition_property_keyword,
    parse_transition_property_list, parse_transition_shorthand, parse_transition_shorthand_single,
    unzip_transition_rules,
};

use crate::parse::token::Token;

/// [`render_value`] for a specified value CSSOM serializes (CSSOM §6.7.2):
/// keywords, function names and units in ASCII lowercase — they are
/// ASCII case-insensitive (CSS Values 4 §2.1) — while a custom
/// property name (`--Foo`, case-sensitive), a string and a URL keep
/// their case. The kept text of a background image or position.
pub(crate) fn render_keywords_lowercase(value: &[Token]) -> String {
    let lowered: Vec<Token> = value
        .iter()
        .map(|t| match t {
            Token::Ident(s) if !s.starts_with("--") => Token::Ident(s.to_ascii_lowercase()),
            Token::Function(f) => Token::Function(f.to_ascii_lowercase()),
            Token::Dimension {
                value,
                integer,
                unit,
            } => Token::Dimension {
                value: *value,
                integer: *integer,
                unit: unit.to_ascii_lowercase(),
            },
            other => other.clone(),
        })
        .collect();
    render_value(&lowered)
}

/// Render a `&[Token]` slice back to CSS text the way CSSOM serializes a
/// component value list (CSSOM §6.7.2; CSS Syntax 3 §9): one space
/// between component values, none just inside parentheses or before a
/// comma, one after a comma, a sign on the number it signs (`-45deg`),
/// and a math function's `+` / `-` operators spaced (`calc(50% - 1px)`,
/// where the space is required). Reading it back gives the same tokens:
/// strings and identifiers — function names and dimension units
/// included — keep their escapes (`rdom_core::css_syntax`), a dimension
/// stays one token while a number and an ident stay two, an ident before
/// `(` stays apart from it. The tokenizer keeps no whitespace, so a `-`
/// (or `+`) before a number is read as its sign everywhere except after
/// an operand inside a math function, the only place CSS has a binary
/// one. Custom properties are stored this way; the kept background
/// text and the block parser's `InvalidValue` warnings use it too.
pub fn render_value(value: &[Token]) -> String {
    use rdom_core::css_syntax::{serialize_identifier, serialize_string};
    let mut out = String::new();
    // Per open parenthesis: is it a math function's (or nested in one)?
    let mut math: Vec<bool> = Vec::new();
    let mut tight = true;
    for (i, t) in value.iter().enumerate() {
        let in_math = math.last().copied().unwrap_or(false);
        if !tight && !matches!(t, Token::RParen | Token::Comma) {
            out.push(' ');
        }
        tight = false;
        match t {
            Token::Ident(s) => out.push_str(&serialize_identifier(s)),
            Token::Number(n) => out.push_str(&n.to_string()),
            Token::Float(f) => out.push_str(&f.to_string()),
            Token::Percentage(n) => {
                out.push_str(&n.to_string());
                out.push('%');
            }
            Token::Dimension { value, unit, .. } => {
                out.push_str(&value.to_string());
                out.push_str(&serialize_unit(unit));
            }
            Token::String(s) => out.push_str(&serialize_string(s)),
            Token::Url(url) => {
                out.push_str("url(");
                out.push_str(&serialize_url(url));
                out.push(')');
            }
            // Never round-trips: a declaration holding one is invalid,
            // so only a warning shows it.
            Token::BadUrl => out.push_str("url(\u{FFFD})"),
            Token::HexColor(h) => {
                out.push('#');
                out.push_str(h);
            }
            Token::Function(name) => {
                out.push_str(&serialize_identifier(name));
                out.push('(');
                math.push(in_math || calc::math_function(name).is_some());
                tight = true;
            }
            Token::LParen => {
                out.push('(');
                math.push(in_math);
                tight = true;
            }
            Token::RParen => {
                out.push(')');
                math.pop();
            }
            Token::Colon => out.push(':'),
            Token::Semicolon => out.push(';'),
            Token::Comma => out.push(','),
            Token::Bang => out.push('!'),
            Token::Delim(c @ ('-' | '+')) => {
                out.push(*c);
                let signs_next = matches!(
                    value.get(i + 1),
                    Some(
                        Token::Number(_)
                            | Token::Float(_)
                            | Token::Percentage(_)
                            | Token::Dimension { .. }
                    )
                );
                let after_operand = i > 0 && ends_operand(&value[i - 1]);
                tight = signs_next && !(in_math && after_operand);
            }
            Token::Delim(c) => out.push(*c),
        }
    }
    out
}

/// A token an operand ends with — after it, a math function's `-` is
/// the binary operator.
fn ends_operand(t: &Token) -> bool {
    matches!(
        t,
        Token::Number(_)
            | Token::Float(_)
            | Token::Percentage(_)
            | Token::Dimension { .. }
            | Token::Ident(_)
            | Token::RParen
    )
}

/// A `<url-token>`'s text, escaped so it reads back as one (CSS Syntax 3
/// §4.3.6): whitespace, quotes, parentheses, a backslash and the
/// non-printable code points as hex escapes.
fn serialize_url(url: &str) -> String {
    let mut out = String::with_capacity(url.len());
    for c in url.chars() {
        if c.is_whitespace()
            || matches!(c, '"' | '\'' | '(' | ')' | '\\')
            || matches!(c, '\u{0}'..='\u{1F}' | '\u{7F}')
        {
            out.push_str(&format!("\\{:x} ", u32::from(c)));
        } else {
            out.push(c);
        }
    }
    out
}

/// A dimension's unit as an identifier, with a leading `e` escaped when
/// it would read as an exponent (`1\65 3`, not `1e3`).
fn serialize_unit(unit: &str) -> String {
    let ident = rdom_core::css_syntax::serialize_identifier(unit);
    let mut chars = unit.chars();
    let exponent_like = matches!(chars.next(), Some('e' | 'E'))
        && match chars.next() {
            Some(d) if d.is_ascii_digit() => true,
            Some('+' | '-') => chars.next().is_some_and(|d| d.is_ascii_digit()),
            _ => false,
        };
    if exponent_like {
        let first = unit.chars().next().map_or(0, u32::from);
        format!("\\{first:x} {}", &ident[1..])
    } else {
        ident
    }
}
