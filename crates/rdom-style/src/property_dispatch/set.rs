//! `set_parsed`: parse a declaration value with its property's grammar
//! and write the owned `TuiStyle` field(s). Owns CSS-wide keyword
//! routing; a per-side longhand (`padding-top`, `border-left-style`,
//! …) writes its own side's field. Declaring on a block — `set` / `set_from_tokens`,
//! what is kept for the cascade, custom properties — is `declare`.

use super::DispatchError;
use super::css_wide::{css_wide_keyword, set_css_wide};
use super::table::canonical_property_name;
use crate::layout::{CaretColor, CaretTextColor, Sides, TextDirection, UserSelect, WhiteSpace};
use crate::parse::token::Token;
use crate::parse::values::{
    parse_aspect_ratio, parse_color, parse_content, parse_counter_ops, parse_flex_factor,
    parse_flex_shorthand, parse_gap, parse_inset_shorthand, parse_keyword, parse_length,
    parse_margin_longhand, parse_margin_shorthand, parse_max_size, parse_min_size, parse_opacity,
    parse_overflow, parse_padding_shorthand, parse_padding_value, parse_position,
    parse_scroll_behavior, parse_scrollbar_gutter, parse_size, parse_text_decoration,
    parse_time_list, parse_timing_function_list, parse_transition_property_list,
    parse_transition_shorthand, parse_z_index, unzip_transition_rules,
};
use crate::{TuiStyle, Value};

/// Set `name` to the CSS-wide `unset` — the value of a declaration
/// invalid at computed-value time (CSS Variables 1 §3.1).
pub fn set_unset(name: &str, style: &mut TuiStyle) {
    set_unset_in(name, style, TextDirection::Ltr);
}

/// [`set_unset`] for an element of `direction`: a flow-relative
/// property unsets the physical property it maps to.
pub(crate) fn set_unset_in(name: &str, style: &mut TuiStyle, direction: TextDirection) {
    let name = &*canonical_property_name(name);
    let unset = [Token::Ident("unset".to_string())];
    if super::logical::set_mapped(name, &unset, style, direction).is_some() {
        return;
    }
    // Every table name accepts a CSS-wide keyword.
    let _ = set_css_wide(name, super::css_wide::CssWide::Unset, style);
}

/// Parse `value` with `name`'s own grammar and write it — no `var()`
/// handling; the cascade calls this with substituted tokens. A
/// flow-relative inline-axis property maps as under `ltr`; the cascade
/// maps them by the element's direction instead (`SubstitutionContext::direction`).
pub fn set_parsed(name: &str, value: &[Token], style: &mut TuiStyle) -> Result<(), DispatchError> {
    set_parsed_in(name, value, style, TextDirection::Ltr)
}

/// [`set_parsed`] for an element of `direction`: a flow-relative
/// property writes the physical property it maps to (CSS Logical 1).
pub(crate) fn set_parsed_in(
    name: &str,
    value: &[Token],
    style: &mut TuiStyle,
    direction: TextDirection,
) -> Result<(), DispatchError> {
    let mapped =
        super::logical::set_mapped(&canonical_property_name(name), value, style, direction);
    if let Some(outcome) = mapped {
        return outcome;
    }
    set_physical(name, value, style)
}

/// [`set_parsed`] for every name but the flow-relative ones.
fn set_physical(name: &str, value: &[Token], style: &mut TuiStyle) -> Result<(), DispatchError> {
    if let Some(custom) = name.strip_prefix("--") {
        // CSS Variables 1 §2: any `--*` name is valid and untyped;
        // the value is kept verbatim (no css-wide keyword handling
        // either — `--x: inherit` is the token `inherit`).
        if custom.is_empty() {
            return Err(DispatchError::UnknownProperty);
        }
        return super::declare::set_custom(custom, value, false, style);
    }
    let name = &*canonical_property_name(name);
    if let Some(kw) = css_wide_keyword(value) {
        return set_css_wide(name, kw, style);
    }
    if let Some(outcome) = super::background::set(name, value, style)
        .or_else(|| super::border::set(name, value, style))
        .or_else(|| super::shadow::set(name, value, style))
        .or_else(|| super::contain::set(name, value, style))
    {
        return outcome.ok_or(DispatchError::InvalidValue);
    }
    let outcome: Option<()> = match name {
        // Color / modifiers
        "color" => parse_color(value).map(|c| {
            style.fg = Some(Value::Specified(c));
        }),
        "background-color" => parse_color(value).map(|c| {
            style.bg = Some(Value::Specified(c));
        }),
        "font-weight" => parse_keyword(value, &[("bold", true), ("normal", false)]).map(|v| {
            style.bold = Some(Value::Specified(v));
        }),
        "font-style" => parse_keyword(value, &[("italic", true), ("normal", false)]).map(|v| {
            style.italic = Some(Value::Specified(v));
        }),
        "text-decoration" => parse_text_decoration(value).map(|(under, strike)| {
            // Map the boolean pair to the `TextDecoration` enum.
            // `(false, false)` → None; `(true, _)` → Underline;
            // `(false, true)` → LineThrough. Mutually exclusive in
            // 0.1.0 (single-axis representation); future CSS-shorthand
            // `text-decoration: underline line-through` would need
            // both bits, deferred to 0.2.x.
            let td = if strike {
                crate::layout::TextDecoration::LineThrough
            } else if under {
                crate::layout::TextDecoration::Underline
            } else {
                crate::layout::TextDecoration::None
            };
            style.text_decoration = Some(Value::Specified(td));
        }),
        "opacity" => parse_opacity(value).map(|v| {
            style.opacity = Some(Value::Specified(v));
        }),

        // Layout — keywords
        //
        // `display` writes all three of its fields — the outer
        // (`Display`) and inner (`Flow`) types and the `list-item` flag
        // (CSS Display 3 §2; the mapping table is on `Flow`) — so a
        // later `display` leaves no earlier inner type behind.
        "display" => {
            crate::parse::values::parse_display(value).map(|(display, flow, list_item)| {
                style.display = Some(Value::Specified(display));
                style.flow = Some(Value::Specified(flow));
                style.list_item = Some(Value::Specified(list_item));
            })
        }
        // CSS Flexbox §5.1: the axis, and whether its start and end swap.
        "flex-direction" => {
            crate::parse::values::parse_flex_direction(value).map(|(d, reverse)| {
                style.direction = Some(Value::Specified(d));
                style.flex_reverse = Some(Value::Specified(reverse));
            })
        }
        // CSS Flexbox §5.2.
        "flex-wrap" => crate::parse::values::parse_flex_wrap(value).map(|w| {
            style.flex_wrap = Some(Value::Specified(w));
        }),
        // CSS Box Alignment 3 §5.2.
        "justify-content" => crate::parse::values::parse_justify_content(value).map(|a| {
            style.justify_content = Some(Value::Specified(a));
        }),
        // CSS Box Alignment 3 §5.1.
        "align-content" => crate::parse::values::parse_align_content(value).map(|a| {
            style.align_content = Some(Value::Specified(a));
        }),
        // CSS Box Alignment 3 §6.3 / §6.1.
        "align-items" => crate::parse::values::parse_align_items(value).map(|a| {
            style.align_items = Some(Value::Specified(a));
        }),
        "align-self" => crate::parse::values::parse_align_self(value).map(|a| {
            style.align_self = Some(Value::Specified(a));
        }),
        // CSS Flexbox §5.3: the shorthand writes all three fields, an
        // omitted component as its initial value.
        "flex-flow" => crate::parse::values::parse_flex_flow(value).map(|((d, reverse), w)| {
            style.direction = Some(Value::Specified(d));
            style.flex_reverse = Some(Value::Specified(reverse));
            style.flex_wrap = Some(Value::Specified(w));
        }),
        "white-space" => parse_keyword(
            value,
            &[
                ("normal", WhiteSpace::Normal),
                ("pre", WhiteSpace::Pre),
                ("pre-wrap", WhiteSpace::PreWrap),
                ("nowrap", WhiteSpace::NoWrap),
            ],
        )
        .map(|w| {
            style.white_space = Some(Value::Specified(w));
        }),
        "user-select" => parse_keyword(
            value,
            &[
                ("auto", UserSelect::Auto),
                ("text", UserSelect::Text),
                ("none", UserSelect::None),
                ("all", UserSelect::All),
                ("contain", UserSelect::Contain),
            ],
        )
        .map(|u| {
            style.user_select = Some(Value::Specified(u));
        }),
        "pointer-events" => parse_keyword(
            value,
            &[
                ("auto", crate::layout::PointerEvents::Auto),
                ("none", crate::layout::PointerEvents::None),
            ],
        )
        .map(|v| {
            style.pointer_events = Some(Value::Specified(v));
        }),
        // CSS Display 3 §4.
        "visibility" => parse_keyword(
            value,
            &[
                ("visible", crate::layout::Visibility::Visible),
                ("hidden", crate::layout::Visibility::Hidden),
                ("collapse", crate::layout::Visibility::Collapse),
            ],
        )
        .map(|v| {
            style.visibility = Some(Value::Specified(v));
        }),
        // `caret-color: auto | transparent | <color>`. Auto = caret
        // bg matches the underlying cell's fg (classic swap visual).
        // Transparent suppresses the caret paint entirely. A color
        // value paints the caret cell's bg with that color.
        "caret-color" => parse_keyword(
            value,
            &[
                ("auto", CaretColor::Auto),
                ("transparent", CaretColor::Transparent),
            ],
        )
        .or_else(|| parse_color(value).map(CaretColor::Color))
        .map(|c| {
            style.caret_color = Some(Value::Specified(c));
        }),
        // rdom-extension `caret-text-color: auto | <color>`. Auto =
        // glyph color matches the underlying cell's bg (classic
        // swap visual). A color value paints the caret cell's fg.
        "caret-text-color" => parse_keyword(value, &[("auto", CaretTextColor::Auto)])
            .or_else(|| parse_color(value).map(CaretTextColor::Color))
            .map(|c| {
                style.caret_text_color = Some(Value::Specified(c));
            }),

        // Layout — overflow
        "overflow" => parse_overflow(value).map(|o| {
            style.overflow_x = Some(Value::Specified(o));
            style.overflow_y = Some(Value::Specified(o));
        }),
        "overflow-x" => parse_overflow(value).map(|o| {
            style.overflow_x = Some(Value::Specified(o));
        }),
        "overflow-y" => parse_overflow(value).map(|o| {
            style.overflow_y = Some(Value::Specified(o));
        }),
        "scrollbar-gutter" => parse_scrollbar_gutter(value).map(|g| {
            style.scrollbar_gutter = Some(Value::Specified(g));
        }),
        "scroll-behavior" => parse_scroll_behavior(value).map(|b| {
            style.scroll_behavior = Some(Value::Specified(b));
        }),
        "direction" => parse_keyword(
            value,
            &[
                ("ltr", crate::layout::TextDirection::Ltr),
                ("rtl", crate::layout::TextDirection::Rtl),
            ],
        )
        .map(|d| {
            style.text_direction = Some(Value::Specified(d));
        }),
        "writing-mode" => parse_keyword(
            value,
            &[
                ("horizontal-tb", crate::layout::WritingMode::HorizontalTb),
                ("vertical-rl", crate::layout::WritingMode::VerticalRl),
                ("vertical-lr", crate::layout::WritingMode::VerticalLr),
                ("sideways-rl", crate::layout::WritingMode::SidewaysRl),
                ("sideways-lr", crate::layout::WritingMode::SidewaysLr),
            ],
        )
        .map(|m| {
            style.writing_mode = Some(Value::Specified(m));
        }),
        "color-scheme" => crate::color::ColorSchemeList::parse(value).map(|s| {
            style.color_scheme = Some(Value::Specified(s));
        }),

        // Layout — sizing
        "width" => parse_size(value).map(|s| {
            style.width = Some(Value::Specified(s));
        }),
        "height" => parse_size(value).map(|s| {
            style.height = Some(Value::Specified(s));
        }),
        "min-width" => parse_min_size(value).map(|m| {
            style.min_width = Some(Value::Specified(m));
        }),
        "max-width" => parse_max_size(value).map(|m| {
            style.max_width = Some(Value::Specified(m));
        }),
        "min-height" => parse_min_size(value).map(|m| {
            style.min_height = Some(Value::Specified(m));
        }),
        "max-height" => parse_max_size(value).map(|m| {
            style.max_height = Some(Value::Specified(m));
        }),
        "aspect-ratio" => parse_aspect_ratio(value).map(|r| {
            style.aspect_ratio = Some(Value::Specified(r));
        }),
        "box-sizing" => parse_keyword(
            value,
            &[
                ("content-box", crate::layout::BoxSizing::ContentBox),
                ("border-box", crate::layout::BoxSizing::BorderBox),
            ],
        )
        .map(|b| {
            style.box_sizing = Some(Value::Specified(b));
        }),

        // Layout — gap
        "gap" => crate::parse::values::parse_gap_shorthand(value).map(|(row, column)| {
            style.row_gap = Some(Value::Specified(row));
            style.column_gap = Some(Value::Specified(column));
        }),
        "row-gap" => parse_gap(value).map(|g| {
            style.row_gap = Some(Value::Specified(g));
        }),
        "column-gap" => parse_gap(value).map(|g| {
            style.column_gap = Some(Value::Specified(g));
        }),

        // Flex shorthand (CSS Flexbox §7.2): its three longhands.
        "flex" => parse_flex_shorthand(value).map(|f| {
            style.flex_grow = Some(Value::Specified(f.grow));
            style.flex_shrink = Some(Value::Specified(f.shrink));
            style.flex_basis = Some(Value::Specified(f.basis));
        }),
        "flex-grow" => parse_flex_factor(value).map(|n| {
            style.flex_grow = Some(Value::Specified(n));
        }),
        "flex-shrink" => parse_flex_factor(value).map(|n| {
            style.flex_shrink = Some(Value::Specified(n));
        }),
        "flex-basis" => crate::parse::values::parse_flex_basis(value).map(|b| {
            style.flex_basis = Some(Value::Specified(b));
        }),
        "order" => crate::parse::values::parse_order(value).map(|n| {
            style.order = Some(Value::Specified(n));
        }),

        // Padding shorthand + longhands
        "padding" => parse_padding_shorthand(value).map(|p| {
            style.padding = Sides::from(p).map(|v| Some(Value::Specified(v)));
        }),
        "padding-top" => parse_padding_value(value).map(|v| {
            style.padding.top = Some(Value::Specified(v));
        }),
        "padding-right" => parse_padding_value(value).map(|v| {
            style.padding.right = Some(Value::Specified(v));
        }),
        "padding-bottom" => parse_padding_value(value).map(|v| {
            style.padding.bottom = Some(Value::Specified(v));
        }),
        "padding-left" => parse_padding_value(value).map(|v| {
            style.padding.left = Some(Value::Specified(v));
        }),

        // Margin shorthand + longhands
        "margin" => parse_margin_shorthand(value).map(|m| {
            style.margin = Sides::from(m).map(|v| Some(Value::Specified(v)));
        }),
        "margin-top" => parse_margin_longhand(value).map(|v| {
            style.margin.top = Some(Value::Specified(v));
        }),
        "margin-right" => parse_margin_longhand(value).map(|v| {
            style.margin.right = Some(Value::Specified(v));
        }),
        "margin-bottom" => parse_margin_longhand(value).map(|v| {
            style.margin.bottom = Some(Value::Specified(v));
        }),
        "margin-left" => parse_margin_longhand(value).map(|v| {
            style.margin.left = Some(Value::Specified(v));
        }),

        "margin-trim" => crate::parse::values::parse_margin_trim(value).map(|t| {
            style.margin_trim = Some(Value::Specified(t));
        }),

        "border-collapse" => parse_keyword(
            value,
            &[
                ("separate", crate::layout::BorderCollapse::Separate),
                ("collapse", crate::layout::BorderCollapse::Collapse),
            ],
        )
        .map(|v| {
            style.border_collapse = Some(Value::Specified(v));
        }),

        // Pseudo-element content
        "content" => parse_content(value).map(|c| {
            style.content = Some(Value::Specified(c));
        }),

        // Positioning (M2)
        "position" => parse_position(value).map(|p| {
            style.position = Some(Value::Specified(p));
        }),
        "top" => parse_length(value).map(|l| {
            style.top = Some(Value::Specified(l));
        }),
        "right" => parse_length(value).map(|l| {
            style.right = Some(Value::Specified(l));
        }),
        "bottom" => parse_length(value).map(|l| {
            style.bottom = Some(Value::Specified(l));
        }),
        "left" => parse_length(value).map(|l| {
            style.left = Some(Value::Specified(l));
        }),
        "z-index" => parse_z_index(value).map(|z| {
            style.z_index = Some(Value::Specified(z));
        }),
        "inset" => parse_inset_shorthand(value).map(|(t, r, b, l)| {
            style.top = Some(Value::Specified(t));
            style.right = Some(Value::Specified(r));
            style.bottom = Some(Value::Specified(b));
            style.left = Some(Value::Specified(l));
        }),

        // Transitions (M3)
        "transition-property" => parse_transition_property_list(value).map(|list| {
            style.transition_property = Some(Value::Specified(list));
        }),
        "transition-duration" => parse_time_list(value).map(|list| {
            style.transition_duration = Some(Value::Specified(list));
        }),
        "transition-timing-function" => parse_timing_function_list(value).map(|list| {
            style.transition_timing_function = Some(Value::Specified(list));
        }),
        "transition-delay" => parse_time_list(value).map(|list| {
            style.transition_delay = Some(Value::Specified(list));
        }),
        "transition" => parse_transition_shorthand(value).map(|rules| {
            let (props, durs, timings, delays) = unzip_transition_rules(&rules);
            style.transition_property = Some(Value::Specified(props));
            style.transition_duration = Some(Value::Specified(durs));
            style.transition_timing_function = Some(Value::Specified(timings));
            style.transition_delay = Some(Value::Specified(delays));
        }),

        "counter-reset" => parse_counter_ops(value, 0).map(|ops| {
            style.counter_reset = Some(Value::Specified(ops));
        }),
        "counter-increment" => parse_counter_ops(value, 1).map(|ops| {
            style.counter_increment = Some(Value::Specified(ops));
        }),

        // `all` takes a CSS-wide keyword only (handled above).
        "all" => None,

        _ => return Err(DispatchError::UnknownProperty),
    };

    outcome.ok_or(DispatchError::InvalidValue).map(|_| ())
}
