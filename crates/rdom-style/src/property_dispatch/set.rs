//! `set` / `set_from_tokens`: parse a declaration value and write the
//! owned `TuiStyle` field(s). Owns custom-property (`--*`) storage,
//! CSS-wide keyword routing, and the per-side longhand merge rules
//! (`padding-top`, `border-left-style`, …) that read the current
//! shorthand value before writing one side.

use super::DispatchError;
use super::css_wide::{css_wide_keyword, set_css_wide};
use crate::layout::{CaretColor, CaretTextColor, Direction, Display, Size, UserSelect, WhiteSpace};
use crate::parse::token::{Token, tokenize};
use crate::parse::values::{
    current_border, current_margin, current_padding, parse_aspect_ratio, parse_border,
    parse_border_side, parse_color, parse_content, parse_counter_ops, parse_flex_shorthand,
    parse_gap, parse_inset_shorthand, parse_keyword, parse_length, parse_margin_longhand,
    parse_margin_shorthand, parse_min_size, parse_opacity, parse_overflow, parse_padding_shorthand,
    parse_padding_value, parse_position, parse_scrollbar_gutter, parse_size, parse_text_decoration,
    parse_time_list, parse_timing_function_list, parse_transition_property_list,
    parse_transition_shorthand, parse_unsigned, parse_z_index, unzip_transition_rules,
};
use crate::{TuiStyle, Value};

/// Set `name = value` on `style`. Tokenizes the value first; for
/// callers that already have tokens, prefer [`set_from_tokens`].
pub fn set(name: &str, value: &str, style: &mut TuiStyle) -> Result<(), DispatchError> {
    let tokens = tokenize(value).map_err(|_| DispatchError::InvalidValue)?;
    set_from_tokens(name, &tokens, style)
}

/// Pre-tokenized variant of [`set`]. The block parser in
/// `rdom-css` calls this to avoid re-tokenizing each declaration's
/// value when the surrounding block was already tokenized.
pub fn set_from_tokens(
    name: &str,
    value: &[Token],
    style: &mut TuiStyle,
) -> Result<(), DispatchError> {
    if let Some(custom) = name.strip_prefix("--") {
        // CSS Variables 1 §2: any `--*` name is valid and untyped;
        // the value is kept verbatim (no css-wide keyword handling
        // either — `--x: inherit` is the token `inherit`).
        if custom.is_empty() {
            return Err(DispatchError::UnknownProperty);
        }
        style.set_custom_property(custom, &crate::parse::values::render_value(value), false);
        return Ok(());
    }
    if let Some(kw) = css_wide_keyword(value) {
        return set_css_wide(name, kw, style);
    }
    let outcome: Option<()> = match name {
        // Color / modifiers
        "color" => parse_color(value).map(|c| {
            style.fg = Some(Value::Specified(c));
        }),
        // `background` shorthand: only the color component exists in a
        // cell grid (no images, positions, or repeat), so a lone color
        // is `background-color` and anything else is invalid.
        "background-color" | "background" => parse_color(value).map(|c| {
            style.bg = Some(Value::Specified(c));
        }),
        "border-color" => parse_color(value).map(|c| {
            style.border_fg = Some(Value::Specified(c));
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
        // `display` writes BOTH outer (`Display`) and inner (`Flow`)
        // values per CSS3 Display Module. The single-value forms map:
        //  `block`        → Block + flow:Block
        //  `flex`         → Block + flow:Flex   (most common)
        //  `inline`       → Inline + (flow N/A)
        //  `inline-block` → InlineBlock + flow:Block
        //  `inline-flex`  → Inline + flow:Flex
        //  `none`         → None
        // The Flow side overwrites any prior author `flow` write —
        // matches CSS expectation that `display: flex` makes the
        // element a flex container regardless of any other prop.
        "display" => match value {
            [Token::Ident(s)] if s.eq_ignore_ascii_case("block") => {
                style.display = Some(Value::Specified(Display::Block));
                style.flow = Some(Value::Specified(crate::layout::Flow::Block));
                Some(())
            }
            [Token::Ident(s)] if s.eq_ignore_ascii_case("flex") => {
                style.display = Some(Value::Specified(Display::Block));
                style.flow = Some(Value::Specified(crate::layout::Flow::Flex));
                Some(())
            }
            [Token::Ident(s)] if s.eq_ignore_ascii_case("inline") => {
                style.display = Some(Value::Specified(Display::Inline));
                Some(())
            }
            [Token::Ident(s)] if s.eq_ignore_ascii_case("inline-block") => {
                style.display = Some(Value::Specified(Display::InlineBlock));
                style.flow = Some(Value::Specified(crate::layout::Flow::Block));
                Some(())
            }
            [Token::Ident(s)] if s.eq_ignore_ascii_case("inline-flex") => {
                style.display = Some(Value::Specified(Display::Inline));
                style.flow = Some(Value::Specified(crate::layout::Flow::Flex));
                Some(())
            }
            [Token::Ident(s)] if s.eq_ignore_ascii_case("none") => {
                style.display = Some(Value::Specified(Display::None));
                Some(())
            }
            _ => None,
        },
        "flex-direction" => parse_keyword(
            value,
            &[("row", Direction::Row), ("column", Direction::Column)],
        )
        .map(|d| {
            style.direction = Some(Value::Specified(d));
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
        "max-width" => parse_unsigned(value).map(|n| {
            style.max_width = Some(Value::Specified(n));
        }),
        "min-height" => parse_min_size(value).map(|m| {
            style.min_height = Some(Value::Specified(m));
        }),
        "max-height" => parse_unsigned(value).map(|n| {
            style.max_height = Some(Value::Specified(n));
        }),
        "aspect-ratio" => parse_aspect_ratio(value).map(|r| {
            style.aspect_ratio = Some(Value::Specified(r));
        }),

        // Layout — gap
        "gap" => parse_gap(value).map(|g| {
            style.gap = Some(Value::Specified(g));
        }),

        // Flex shorthand — sets `width` + `height` AND `flex-shrink`.
        // Per CSS spec: `flex: <n>` ≡ `<n> 1 0` (grow=n, shrink=1,
        // basis=0); `flex: none` ≡ `0 0 auto` (no grow, NO shrink,
        // basis=auto). Cross-axis `Size::Flex` reads as "stretch to
        // container" in the layout pass, matching CSS default
        // `align-items: stretch`.
        "flex" => parse_flex_shorthand(value).map(|s| {
            // `flex: none` ⇒ Size::Auto with flex_shrink=0. All
            // other shapes use the CSS-default shrink=1.
            let shrink = match &s {
                Size::Auto => 0,
                _ => 1,
            };
            style.width = Some(Value::Specified(s.clone()));
            style.height = Some(Value::Specified(s));
            style.flex_shrink = Some(Value::Specified(shrink));
        }),
        "flex-shrink" => parse_unsigned(value).map(|n| {
            style.flex_shrink = Some(Value::Specified(n));
        }),

        // Padding shorthand + longhands
        "padding" => parse_padding_shorthand(value).map(|p| {
            style.padding = Some(Value::Specified(p));
        }),
        "padding-top" => parse_padding_value(value).map(|v| {
            let mut p = current_padding(style);
            p.top = v;
            style.padding = Some(Value::Specified(p));
        }),
        "padding-right" => parse_padding_value(value).map(|v| {
            let mut p = current_padding(style);
            p.right = v;
            style.padding = Some(Value::Specified(p));
        }),
        "padding-bottom" => parse_padding_value(value).map(|v| {
            let mut p = current_padding(style);
            p.bottom = v;
            style.padding = Some(Value::Specified(p));
        }),
        "padding-left" => parse_padding_value(value).map(|v| {
            let mut p = current_padding(style);
            p.left = v;
            style.padding = Some(Value::Specified(p));
        }),

        // Margin shorthand + longhands
        "margin" => parse_margin_shorthand(value).map(|m| {
            style.margin = Some(Value::Specified(m));
        }),
        "margin-top" => parse_margin_longhand(value).map(|v| {
            let mut m = current_margin(style);
            m.top = v;
            style.margin = Some(Value::Specified(m));
        }),
        "margin-right" => parse_margin_longhand(value).map(|v| {
            let mut m = current_margin(style);
            m.right = v;
            style.margin = Some(Value::Specified(m));
        }),
        "margin-bottom" => parse_margin_longhand(value).map(|v| {
            let mut m = current_margin(style);
            m.bottom = v;
            style.margin = Some(Value::Specified(m));
        }),
        "margin-left" => parse_margin_longhand(value).map(|v| {
            let mut m = current_margin(style);
            m.left = v;
            style.margin = Some(Value::Specified(m));
        }),

        // Border shorthand + per-side longhands. Per-side
        // longhands READ the current border on `style` and
        // MERGE — so `border: solid; border-top: none` correctly
        // clears just the top side and keeps R/B/L.
        "border" => parse_border(value).map(|b| {
            style.border = Some(Value::Specified(b));
        }),
        "border-top" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.top = bs;
            style.border = Some(Value::Specified(b));
        }),
        "border-right" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.right = bs;
            style.border = Some(Value::Specified(b));
        }),
        "border-bottom" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.bottom = bs;
            style.border = Some(Value::Specified(b));
        }),
        "border-left" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.left = bs;
            style.border = Some(Value::Specified(b));
        }),
        // CSS `border-style: <style>` — sets all four sides to one
        // style. `border-style: hidden` is the conflict kill-switch
        // applied uniformly.
        "border-style" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.top = bs;
            b.right = bs;
            b.bottom = bs;
            b.left = bs;
            style.border = Some(Value::Specified(b));
        }),
        // CSS per-side `*-style` longhands. Same merge semantics as
        // `border-top` / etc.; useful when authors want to flip just
        // one side's style without touching color (when color lands).
        "border-top-style" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.top = bs;
            style.border = Some(Value::Specified(b));
        }),
        "border-right-style" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.right = bs;
            style.border = Some(Value::Specified(b));
        }),
        "border-bottom-style" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.bottom = bs;
            style.border = Some(Value::Specified(b));
        }),
        "border-left-style" => parse_border_side(value).map(|bs| {
            let mut b = current_border(style);
            b.left = bs;
            style.border = Some(Value::Specified(b));
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

        _ => return Err(DispatchError::UnknownProperty),
    };

    outcome.ok_or(DispatchError::InvalidValue).map(|_| ())
}
