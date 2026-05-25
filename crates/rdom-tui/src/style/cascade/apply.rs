//! Cascade ladder + per-property applicators.
//!
//! The ladder has 6 ordered steps (UA normal → Author normal →
//! Inline normal → Inline important → Author important → UA
//! important). `!important` inverts origin priority, matching CSS.
//! Don't shortcut the ladder — the inversion is observable and tests
//! depend on it.
//!
//! Applicators (`apply_color`, `apply_size`, …) handle the three
//! `Value<T>` variants: `Specified`, `Inherit`, `Initial`. They also
//! honor the `important_pass` / `important_prop` pairing so normal
//! and important declarations apply in separate passes.

use crate::layout::{
    Border, CaretColor, CaretTextColor, Direction, Display, Overflow, Padding, Size, UserSelect,
    WhiteSpace,
};
use crate::style::{
    Color, ComputedStyle, ImportantMask, Modifier, Rule, RuleOrigin, TuiColor, TuiStyle, Value,
    resolve_tui_color,
};

/// Walk the cascade ladder once for this element. Calls `apply_style`
/// with `important_pass = false` for the normal passes and `true` for
/// the important passes; invoked in the CSS-spec origin order.
pub(super) fn apply_cascade_ladder(
    working: &mut ComputedStyle,
    sorted_by_spec: &[&Rule],
    inline: Option<&TuiStyle>,
    parent: &ComputedStyle,
) {
    // 1. UA normal.
    for rule in sorted_by_spec {
        if rule.origin == RuleOrigin::UserAgent {
            apply_style(working, &rule.style, parent, /*important_pass=*/ false);
        }
    }
    // 2. Author normal.
    for rule in sorted_by_spec {
        if rule.origin == RuleOrigin::Author {
            apply_style(working, &rule.style, parent, false);
        }
    }
    // 3. Inline normal.
    if let Some(s) = inline {
        apply_style(working, s, parent, false);
    }
    // 4. Inline important (beats normal inline, Author important beats this).
    if let Some(s) = inline {
        apply_style(working, s, parent, /*important_pass=*/ true);
    }
    // 5. Author important.
    for rule in sorted_by_spec {
        if rule.origin == RuleOrigin::Author {
            apply_style(working, &rule.style, parent, true);
        }
    }
    // 6. UA important — final word, can't be overridden. Matches the
    //    CSS rule that `!important` inverts the origin priority.
    for rule in sorted_by_spec {
        if rule.origin == RuleOrigin::UserAgent {
            apply_style(working, &rule.style, parent, true);
        }
    }

    // NOTE — CSS Overflow L3's cross-axis rule ("if one axis is
    // not visible, the visible side behaves as auto") is skipped
    // in v1. Browsers apply it because they know content size at
    // layout time and only show the auto scrollbar when needed.
    // rdom-tui v1 can't (we use `scrollbar-gutter: stable`-style
    // always-reserve), so enforcing the rule would surprise
    // authors writing `overflow-y: scroll` and getting an
    // unexpected horizontal gutter. Each axis is independent.
}

/// Compute `establishes_new_bfc` from the working style + parent
/// context. Runs after the cascade ladder so all source properties
/// are at their final values. Per CSS 2.1 §9.4.1 + Flexbox §3:
///
/// An element establishes a new block formatting context when:
/// - It's a flex container (`flow: Flex`) — flex containers form
///   independent BFCs for their items.
/// - It's an inline-block — establishes a new BFC for its content
///   (which then lays out as block).
/// - Its overflow on either axis is non-visible (Hidden/Scroll/
///   Auto) — clipping containers form independent BFCs.
/// - It's absolutely or fixed positioned — out-of-flow boxes form
///   their own BFCs.
/// - (Root element is also a BFC — handled implicitly because
///   layout starts at root regardless.)
///
/// Margin collapsing checks this predicate: parent-child margin
/// collapse happens only when the parent does NOT establish a new
/// BFC.
pub(super) fn finalize_bfc_formation(working: &mut ComputedStyle) {
    use crate::layout::{Flow, Overflow, Position};
    working.establishes_new_bfc = matches!(working.flow, Flow::Flex)
        || matches!(working.display, Display::InlineBlock)
        || !matches!(working.overflow_x, Overflow::Visible)
        || !matches!(working.overflow_y, Overflow::Visible)
        || matches!(working.position, Position::Absolute | Position::Fixed);
}

/// If no declaration set `border_fg`, fall back to the working `fg`.
/// Runs after the cascade ladder so `fg` is at its final value.
pub(super) fn finalize_border_fg(
    working: &mut ComputedStyle,
    sorted_by_spec: &[&Rule],
    inline: Option<&TuiStyle>,
) {
    let declared_in_rules = sorted_by_spec.iter().any(|r| r.style.border_fg.is_some());
    let declared_inline = inline.is_some_and(|s| s.border_fg.is_some());
    if !declared_in_rules && !declared_inline {
        working.border_fg = working.fg;
    }
}

/// Apply one `TuiStyle` to `working`, for one ladder pass. Paints +
/// layout + display + white_space all in one pass.
fn apply_style(
    working: &mut ComputedStyle,
    style: &TuiStyle,
    parent: &ComputedStyle,
    important_pass: bool,
) {
    // Clone the vars Rc once per apply; all color resolutions below
    // share the same snapshot. Rc::clone is a refcount bump — cheap.
    let vars = working.vars.clone();

    // Paint properties.
    apply_color(
        &mut working.fg,
        &style.fg,
        style.important.contains(ImportantMask::FG),
        important_pass,
        parent.fg,
        ComputedStyle::initial().fg,
        &vars,
    );
    apply_color(
        &mut working.bg,
        &style.bg,
        style.important.contains(ImportantMask::BG),
        important_pass,
        parent.bg,
        ComputedStyle::initial().bg,
        &vars,
    );
    apply_color(
        &mut working.border_fg,
        &style.border_fg,
        style.important.contains(ImportantMask::BORDER_FG),
        important_pass,
        // border_fg's initial is "inherit from fg" per property catalog.
        parent.border_fg,
        working.fg,
        &vars,
    );

    apply_modifier_bit(
        working,
        Modifier::BOLD,
        &style.bold,
        style.important.contains(ImportantMask::BOLD),
        important_pass,
        parent.modifiers.contains(Modifier::BOLD),
    );
    // Pre-T8 had a `.dim(true)` modifier here; dropped in the
    // pre-publish OOTB color overhaul. SGR-2 is theme-dependent and
    // has no CSS analog — authors who want muted text reach for
    // `color: gray` or `opacity: 0.5` instead, both browser-faithful
    // and truecolor-precise.
    apply_modifier_bit(
        working,
        Modifier::ITALIC,
        &style.italic,
        style.important.contains(ImportantMask::ITALIC),
        important_pass,
        parent.modifiers.contains(Modifier::ITALIC),
    );
    // `text-decoration` writes the UNDERLINED / CROSSED_OUT bits.
    // T10 made this the sole entry point — there's no longer a
    // separate `.underline()` modifier setter that could conflict.
    // CSS-faithful: text-decoration is a single property that owns
    // both line axes.
    apply_text_decoration(
        working,
        &style.text_decoration,
        style.important.contains(ImportantMask::TEXT_DECORATION),
        important_pass,
    );
    apply_opacity(
        working,
        &style.opacity,
        style.important.contains(ImportantMask::OPACITY),
        important_pass,
    );

    // Layout properties.
    apply_size(
        &mut working.width,
        &style.width,
        style.important.contains(ImportantMask::WIDTH),
        important_pass,
        parent.width.clone(),
        Size::Auto,
    );
    apply_size(
        &mut working.height,
        &style.height,
        style.important.contains(ImportantMask::HEIGHT),
        important_pass,
        parent.height.clone(),
        Size::Auto,
    );
    apply_opt_copy(
        &mut working.min_width,
        &style.min_width,
        style.important.contains(ImportantMask::MIN_WIDTH),
        important_pass,
        parent.min_width,
    );
    apply_opt_copy(
        &mut working.max_width,
        &style.max_width,
        style.important.contains(ImportantMask::MAX_WIDTH),
        important_pass,
        parent.max_width,
    );
    apply_opt_copy(
        &mut working.min_height,
        &style.min_height,
        style.important.contains(ImportantMask::MIN_HEIGHT),
        important_pass,
        parent.min_height,
    );
    apply_opt_copy(
        &mut working.max_height,
        &style.max_height,
        style.important.contains(ImportantMask::MAX_HEIGHT),
        important_pass,
        parent.max_height,
    );
    apply_opt_copy(
        &mut working.aspect_ratio,
        &style.aspect_ratio,
        style.important.contains(ImportantMask::ASPECT_RATIO),
        important_pass,
        parent.aspect_ratio,
    );
    apply_padding(
        &mut working.padding,
        &style.padding,
        style.important.contains(ImportantMask::PADDING),
        important_pass,
        parent.padding,
    );
    apply_margin(
        &mut working.margin,
        &style.margin,
        style.important.contains(ImportantMask::MARGIN),
        important_pass,
        parent.margin,
    );
    apply_u16(
        &mut working.gap,
        &style.gap,
        style.important.contains(ImportantMask::GAP),
        important_pass,
        parent.gap,
        0,
    );
    apply_u16(
        &mut working.flex_shrink,
        &style.flex_shrink,
        style.important.contains(ImportantMask::FLEX_SHRINK),
        important_pass,
        parent.flex_shrink,
        1, // CSS default
    );
    apply_border(
        &mut working.border,
        &style.border,
        style.important.contains(ImportantMask::BORDER),
        important_pass,
        parent.border,
    );
    apply_border_collapse(
        &mut working.border_collapse,
        &style.border_collapse,
        style.important.contains(ImportantMask::BORDER_COLLAPSE),
        important_pass,
        parent.border_collapse,
    );
    apply_direction(
        &mut working.direction,
        &style.direction,
        style.important.contains(ImportantMask::DIRECTION),
        important_pass,
        parent.direction,
    );
    apply_overflow(
        &mut working.overflow_x,
        &style.overflow_x,
        style.important.contains(ImportantMask::OVERFLOW_X),
        important_pass,
        parent.overflow_x,
    );
    apply_overflow(
        &mut working.overflow_y,
        &style.overflow_y,
        style.important.contains(ImportantMask::OVERFLOW_Y),
        important_pass,
        parent.overflow_y,
    );
    apply_display(
        &mut working.display,
        &style.display,
        style.important.contains(ImportantMask::DISPLAY),
        important_pass,
        parent.display,
    );
    apply_flow(
        &mut working.flow,
        &style.flow,
        style.important.contains(ImportantMask::FLOW),
        important_pass,
    );
    apply_white_space(
        &mut working.white_space,
        &style.white_space,
        style.important.contains(ImportantMask::WHITE_SPACE),
        important_pass,
        parent.white_space,
    );
    apply_user_select(
        &mut working.user_select,
        &style.user_select,
        style.important.contains(ImportantMask::USER_SELECT),
        important_pass,
        parent.user_select,
    );
    apply_caret_color(
        &mut working.caret_color,
        &style.caret_color,
        style.important.contains(ImportantMask::CARET_COLOR),
        important_pass,
        &parent.caret_color,
    );
    apply_caret_text_color(
        &mut working.caret_text_color,
        &style.caret_text_color,
        style.important.contains(ImportantMask::CARET_TEXT_COLOR),
        important_pass,
        &parent.caret_text_color,
    );
    // ── Positioning (M2). Non-inheriting.
    apply_position(
        &mut working.position,
        &style.position,
        style.important.contains(ImportantMask::POSITION),
        important_pass,
        parent.position,
    );
    apply_length(
        &mut working.top,
        &style.top,
        style.important.contains(ImportantMask::TOP),
        important_pass,
        parent.top.clone(),
    );
    apply_length(
        &mut working.right,
        &style.right,
        style.important.contains(ImportantMask::RIGHT),
        important_pass,
        parent.right.clone(),
    );
    apply_length(
        &mut working.bottom,
        &style.bottom,
        style.important.contains(ImportantMask::BOTTOM),
        important_pass,
        parent.bottom.clone(),
    );
    apply_length(
        &mut working.left,
        &style.left,
        style.important.contains(ImportantMask::LEFT),
        important_pass,
        parent.left.clone(),
    );
    apply_z_index(
        &mut working.z_index,
        &style.z_index,
        style.important.contains(ImportantMask::Z_INDEX),
        important_pass,
        parent.z_index,
    );
    // Transitions (M3). Non-inheriting; latest wins. Vec-typed,
    // so we just clone-or-keep based on important matching.
    apply_transition_lists(working, style, important_pass);
}

fn apply_transition_lists(
    working: &mut ComputedStyle,
    style: &crate::style::TuiStyle,
    important_pass: bool,
) {
    let important = style.important.contains(ImportantMask::TRANSITIONS);
    if !matches_pass(important, important_pass) {
        return;
    }
    if let Some(list) = &style.transition_property {
        working.transition_property = list.clone();
    }
    if let Some(list) = &style.transition_duration {
        working.transition_duration = list.clone();
    }
    if let Some(list) = &style.transition_timing_function {
        working.transition_timing_function = list.clone();
    }
    if let Some(list) = &style.transition_delay {
        working.transition_delay = list.clone();
    }
}

// ─── Tiny per-type applicator helpers ───────────────────────────────

/// Should this declaration actually apply during the current pass?
/// Normal pass applies normal declarations; important pass applies
/// important ones.
#[inline]
fn matches_pass(important_prop: bool, important_pass: bool) -> bool {
    important_prop == important_pass
}

macro_rules! apply_simple {
    ($working:expr, $value_opt:expr, $important_prop:expr, $important_pass:expr, $inherit:expr, $initial:expr) => {
        if let Some(v) = $value_opt {
            if matches_pass($important_prop, $important_pass) {
                // `.clone()` works for both Copy and Clone-only
                // types; Copy types still memcpy because their
                // `Clone` impl forwards to Copy.
                $working = match v {
                    Value::Specified(x) => x.clone(),
                    Value::Inherit => $inherit,
                    Value::Initial => $initial,
                };
            }
        }
    };
}

fn apply_color(
    target: &mut Color,
    value: &Option<Value<TuiColor>>,
    important_prop: bool,
    important_pass: bool,
    inherit: Color,
    initial: Color,
    vars: &std::collections::HashMap<String, String>,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        *target = match v {
            Value::Specified(tc) => resolve_tui_color(tc, vars, inherit),
            Value::Inherit => inherit,
            Value::Initial => initial,
        };
    }
}

fn apply_size(
    target: &mut Size,
    value: &Option<Value<Size>>,
    important_prop: bool,
    important_pass: bool,
    inherit: Size,
    initial: Size,
) {
    apply_simple!(
        *target,
        value,
        important_prop,
        important_pass,
        inherit,
        initial
    );
}

fn apply_u16(
    target: &mut u16,
    value: &Option<Value<u16>>,
    important_prop: bool,
    important_pass: bool,
    inherit: u16,
    initial: u16,
) {
    apply_simple!(
        *target,
        value,
        important_prop,
        important_pass,
        inherit,
        initial
    );
}

fn apply_opt_copy<T: Copy>(
    target: &mut Option<T>,
    value: &Option<Value<T>>,
    important_prop: bool,
    important_pass: bool,
    inherit: Option<T>,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        *target = match v {
            Value::Specified(x) => Some(*x),
            Value::Inherit => inherit,
            Value::Initial => None,
        };
    }
}

fn apply_padding(
    target: &mut Padding,
    value: &Option<Value<Padding>>,
    important_prop: bool,
    important_pass: bool,
    inherit: Padding,
) {
    apply_simple!(
        *target,
        value,
        important_prop,
        important_pass,
        inherit,
        Padding::default()
    );
}

fn apply_margin(
    target: &mut crate::layout::Margin,
    value: &Option<Value<crate::layout::Margin>>,
    important_prop: bool,
    important_pass: bool,
    inherit: crate::layout::Margin,
) {
    apply_simple!(
        *target,
        value,
        important_prop,
        important_pass,
        inherit,
        crate::layout::Margin::default()
    );
}

fn apply_border(
    target: &mut Border,
    value: &Option<Value<Border>>,
    important_prop: bool,
    important_pass: bool,
    inherit: Border,
) {
    apply_simple!(
        *target,
        value,
        important_prop,
        important_pass,
        inherit,
        Border::None
    );
}

fn apply_border_collapse(
    target: &mut crate::layout::BorderCollapse,
    value: &Option<Value<crate::layout::BorderCollapse>>,
    important_prop: bool,
    important_pass: bool,
    inherit: crate::layout::BorderCollapse,
) {
    apply_simple!(
        *target,
        value,
        important_prop,
        important_pass,
        inherit,
        crate::layout::BorderCollapse::Separate
    );
}

fn apply_direction(
    target: &mut Direction,
    value: &Option<Value<Direction>>,
    important_prop: bool,
    important_pass: bool,
    inherit: Direction,
) {
    apply_simple!(
        *target,
        value,
        important_prop,
        important_pass,
        inherit,
        Direction::Column
    );
}

fn apply_overflow(
    target: &mut Overflow,
    value: &Option<Value<Overflow>>,
    important_prop: bool,
    important_pass: bool,
    inherit: Overflow,
) {
    apply_simple!(
        *target,
        value,
        important_prop,
        important_pass,
        inherit,
        Overflow::Visible
    );
}

fn apply_display(
    target: &mut Display,
    value: &Option<Value<Display>>,
    important_prop: bool,
    important_pass: bool,
    inherit: Display,
) {
    apply_simple!(
        *target,
        value,
        important_prop,
        important_pass,
        inherit,
        Display::Block
    );
}

/// `flow` is non-inheriting (matches `display`'s non-inheriting nature).
/// Default is `Flow::Block`. Initial-value reset means absent writes
/// fall back to Block, not to the parent's flow.
fn apply_flow(
    target: &mut crate::layout::Flow,
    value: &Option<Value<crate::layout::Flow>>,
    important_prop: bool,
    important_pass: bool,
) {
    apply_simple!(
        *target,
        value,
        important_prop,
        important_pass,
        crate::layout::Flow::Block, // non-inheriting; inherit slot = initial
        crate::layout::Flow::Block
    );
}

fn apply_white_space(
    target: &mut WhiteSpace,
    value: &Option<Value<WhiteSpace>>,
    important_prop: bool,
    important_pass: bool,
    inherit: WhiteSpace,
) {
    apply_simple!(
        *target,
        value,
        important_prop,
        important_pass,
        inherit,
        WhiteSpace::Normal
    );
}

fn apply_user_select(
    target: &mut UserSelect,
    value: &Option<Value<UserSelect>>,
    important_prop: bool,
    important_pass: bool,
    inherit: UserSelect,
) {
    apply_simple!(
        *target,
        value,
        important_prop,
        important_pass,
        inherit,
        UserSelect::Auto
    );
}

/// Apply `caret-color`. Hand-rolled (not `apply_simple!`) because
/// `CaretColor::Color(TuiColor)` is not `Copy` — TuiColor's `Var`
/// variant holds a `String`. Same shape as the macro, with `.clone()`.
fn apply_caret_color(
    target: &mut CaretColor,
    value: &Option<Value<CaretColor>>,
    important_prop: bool,
    important_pass: bool,
    inherit: &CaretColor,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        *target = match v {
            Value::Specified(x) => x.clone(),
            Value::Inherit => inherit.clone(),
            Value::Initial => CaretColor::Auto,
        };
    }
}

fn apply_caret_text_color(
    target: &mut CaretTextColor,
    value: &Option<Value<CaretTextColor>>,
    important_prop: bool,
    important_pass: bool,
    inherit: &CaretTextColor,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        *target = match v {
            Value::Specified(x) => x.clone(),
            Value::Inherit => inherit.clone(),
            Value::Initial => CaretTextColor::Auto,
        };
    }
}

/// Apply CSS `opacity` to the working `ComputedStyle`. Does NOT
/// inherit per CSS spec — `Value::Inherit` resolves to the
/// initial value `1.0`. Clamped to `[0.0, 1.0]` defensively at
/// cascade time even though `.opacity(f)` setter already clamps.
fn apply_opacity(
    working: &mut ComputedStyle,
    value: &Option<Value<f32>>,
    important_prop: bool,
    important_pass: bool,
) {
    if important_prop != important_pass {
        return;
    }
    let resolved = match value {
        Some(Value::Specified(v)) => v.clamp(0.0, 1.0),
        Some(Value::Inherit) | Some(Value::Initial) => 1.0,
        None => return,
    };
    working.opacity = resolved;
}

/// Apply CSS `text-decoration` to the working `ComputedStyle`. Maps
/// the enum value onto the `UNDERLINED` / `CROSSED_OUT` modifier
/// bits. `text-decoration: none` clears both. CSS-spec: the property
/// does NOT inherit (each element sets its own decoration), so the
/// `Value::Inherit` arm uses the property's initial value (`None`)
/// rather than reading from the parent.
fn apply_text_decoration(
    working: &mut ComputedStyle,
    value: &Option<Value<crate::layout::TextDecoration>>,
    important_prop: bool,
    important_pass: bool,
) {
    use crate::layout::TextDecoration;
    if important_prop != important_pass {
        return;
    }
    let resolved = match value {
        Some(Value::Specified(v)) => *v,
        Some(Value::Inherit) => TextDecoration::None,
        Some(Value::Initial) => TextDecoration::None,
        None => return,
    };
    // Wipe both decoration bits, then set the one this property
    // selected (if any).
    working
        .modifiers
        .remove(Modifier::UNDERLINED | Modifier::CROSSED_OUT);
    match resolved {
        TextDecoration::None => {}
        TextDecoration::Underline => working.modifiers.insert(Modifier::UNDERLINED),
        TextDecoration::LineThrough => working.modifiers.insert(Modifier::CROSSED_OUT),
    }
}

fn apply_position(
    target: &mut crate::layout::Position,
    value: &Option<Value<crate::layout::Position>>,
    important_prop: bool,
    important_pass: bool,
    inherit: crate::layout::Position,
) {
    apply_simple!(
        *target,
        value,
        important_prop,
        important_pass,
        inherit,
        crate::layout::Position::Static
    );
}

fn apply_length(
    target: &mut crate::layout::Length,
    value: &Option<Value<crate::layout::Length>>,
    important_prop: bool,
    important_pass: bool,
    inherit: crate::layout::Length,
) {
    apply_simple!(
        *target,
        value,
        important_prop,
        important_pass,
        inherit,
        crate::layout::Length::Auto
    );
}

fn apply_z_index(
    target: &mut crate::layout::ZIndex,
    value: &Option<Value<crate::layout::ZIndex>>,
    important_prop: bool,
    important_pass: bool,
    inherit: crate::layout::ZIndex,
) {
    apply_simple!(
        *target,
        value,
        important_prop,
        important_pass,
        inherit,
        crate::layout::ZIndex::Auto
    );
}

fn apply_modifier_bit(
    working: &mut ComputedStyle,
    bit: Modifier,
    value: &Option<Value<bool>>,
    important_prop: bool,
    important_pass: bool,
    inherit: bool,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        let on = match v {
            Value::Specified(b) => *b,
            Value::Inherit => inherit,
            Value::Initial => false,
        };
        if on {
            working.modifiers |= bit;
        } else {
            working.modifiers.remove(bit);
        }
    }
}
