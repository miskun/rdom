//! Cascade ladder + per-property applicators.
//!
//! The ladder has 6 ordered steps (UA normal → Author normal →
//! Inline normal → Inline important → Author important → UA
//! important). `!important` inverts origin priority, matching CSS.
//! Don't shortcut the ladder — the inversion is observable and tests
//! depend on it.
//!
//! Applicators handle the three `Value<T>` variants: `Specified`,
//! `Inherit`, `Initial`. Most properties share one generic path
//! (`apply_value`); `initial` always reads `ComputedStyle::initial()`
//! (via `Initials`), the table every element's cascade starts from.
//! They also honor the `important_pass` / `important_prop` pairing so
//! normal and important declarations apply in separate passes.

use crate::layout::Display;
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
    // 0. Custom properties (CSS Variables 1 §2) — same ladder, folded
    //    into the element's own map before any `var()` consumer runs.
    apply_custom_properties(working, sorted_by_spec, inline);
    let initial = Initials::default();
    // 1. UA normal.
    for rule in sorted_by_spec {
        if rule.origin == RuleOrigin::UserAgent {
            apply_style(
                working,
                &rule.style,
                parent,
                /*important_pass=*/ false,
                &initial,
            );
        }
    }
    // 2. Author normal.
    for rule in sorted_by_spec {
        if rule.origin == RuleOrigin::Author {
            apply_style(working, &rule.style, parent, false, &initial);
        }
    }
    // 3. Inline normal.
    if let Some(s) = inline {
        apply_style(working, s, parent, false, &initial);
    }
    // 4. Inline important (beats normal inline, Author important beats this).
    if let Some(s) = inline {
        apply_style(working, s, parent, /*important_pass=*/ true, &initial);
    }
    // 5. Author important.
    for rule in sorted_by_spec {
        if rule.origin == RuleOrigin::Author {
            apply_style(working, &rule.style, parent, true, &initial);
        }
    }
    // 6. UA important — final word, can't be overridden. Matches the
    //    CSS rule that `!important` inverts the origin priority.
    for rule in sorted_by_spec {
        if rule.origin == RuleOrigin::UserAgent {
            apply_style(working, &rule.style, parent, true, &initial);
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

/// Fold every matched `--*` declaration into `working.vars`, in ladder
/// order (UA → author → inline, normal then important), so a later or
/// more important declaration of the same name wins. Copy-on-write:
/// elements that declare nothing keep sharing their parent's map.
fn apply_custom_properties(
    working: &mut ComputedStyle,
    sorted_by_spec: &[&Rule],
    inline: Option<&TuiStyle>,
) {
    let declares = sorted_by_spec
        .iter()
        .any(|r| !r.style.custom_properties.is_empty())
        || inline.is_some_and(|s| !s.custom_properties.is_empty());
    if !declares {
        return;
    }
    // CSS Variables 1 §2: the CSS-wide keywords apply to custom
    // properties too — `initial` is the guaranteed-invalid value (the
    // property is undefined), `inherit` / `unset` take the parent's.
    let inherited = working.vars.clone();
    let map = std::rc::Rc::make_mut(&mut working.vars);
    let mut put = |style: &TuiStyle, important_pass: bool| {
        for d in &style.custom_properties {
            if d.important != important_pass {
                continue;
            }
            match d.value.trim() {
                v if v.eq_ignore_ascii_case("initial") => {
                    map.remove(&d.name);
                }
                v if v.eq_ignore_ascii_case("inherit") || v.eq_ignore_ascii_case("unset") => {
                    match inherited.get(&d.name) {
                        Some(parent_value) => {
                            map.insert(d.name.clone(), parent_value.clone());
                        }
                        None => {
                            map.remove(&d.name);
                        }
                    }
                }
                _ => {
                    map.insert(d.name.clone(), d.value.clone());
                }
            }
        }
    };
    for origin in [RuleOrigin::UserAgent, RuleOrigin::Author] {
        for rule in sorted_by_spec.iter().filter(|r| r.origin == origin) {
            put(&rule.style, false);
        }
    }
    if let Some(s) = inline {
        put(s, false);
        put(s, true);
    }
    for origin in [RuleOrigin::Author, RuleOrigin::UserAgent] {
        for rule in sorted_by_spec.iter().filter(|r| r.origin == origin) {
            put(&rule.style, true);
        }
    }
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
    initial: &Initials,
) {
    // Clone the vars Rc once per apply; all color resolutions below
    // share the same snapshot. Rc::clone is a refcount bump — cheap.
    let vars = working.vars.clone();

    // One declared value → one `ComputedStyle` field of the same type:
    // specified as written, `inherit` the parent's field, `initial` the
    // field of `ComputedStyle::initial()`.
    macro_rules! value {
        ($($field:ident: $mask:ident),* $(,)?) => {$(
            apply_value(
                &mut working.$field,
                &style.$field,
                style.important.contains(ImportantMask::$mask),
                important_pass,
                &parent.$field,
                initial,
                |c| &c.$field,
            );
        )*};
    }
    // `min-*` / `max-*` / `aspect-ratio`: `Option` fields, `None` = unset.
    macro_rules! optional {
        ($($field:ident: $mask:ident),* $(,)?) => {$(
            apply_optional(
                &mut working.$field,
                &style.$field,
                style.important.contains(ImportantMask::$mask),
                important_pass,
                parent.$field,
                initial,
                |c| c.$field,
            );
        )*};
    }

    // Paint properties.
    apply_color(
        &mut working.fg,
        &style.fg,
        style.important.contains(ImportantMask::FG),
        important_pass,
        parent.fg,
        || initial.get().fg,
        &vars,
    );
    apply_color(
        &mut working.bg,
        &style.bg,
        style.important.contains(ImportantMask::BG),
        important_pass,
        parent.bg,
        || initial.get().bg,
        &vars,
    );
    // `border-color`'s initial value is `currentColor` (CSS Backgrounds
    // 3 §3.1): the element's `color` as cascaded so far.
    let current_color = working.fg;
    apply_color(
        &mut working.border_fg,
        &style.border_fg,
        style.important.contains(ImportantMask::BORDER_FG),
        important_pass,
        parent.border_fg,
        || current_color,
        &vars,
    );

    apply_modifier_bit(
        working,
        Modifier::BOLD,
        &style.bold,
        style.important.contains(ImportantMask::BOLD),
        important_pass,
        parent.modifiers,
        initial,
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
        parent.modifiers,
        initial,
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
        parent.modifiers,
        initial,
    );
    apply_opacity(
        working,
        &style.opacity,
        style.important.contains(ImportantMask::OPACITY),
        important_pass,
        parent.opacity,
        initial,
    );

    // Layout properties.
    value!(width: WIDTH, height: HEIGHT);
    optional!(
        min_width: MIN_WIDTH,
        max_width: MAX_WIDTH,
        min_height: MIN_HEIGHT,
        max_height: MAX_HEIGHT,
        aspect_ratio: ASPECT_RATIO,
    );
    value!(
        padding: PADDING,
        margin: MARGIN,
        gap: GAP,
        flex_shrink: FLEX_SHRINK,
        border: BORDER,
    );
    apply_border_collapse(
        &mut working.border_collapse,
        &mut working.border_collapse_declared,
        &style.border_collapse,
        style.important.contains(ImportantMask::BORDER_COLLAPSE),
        important_pass,
        parent.border_collapse,
        initial,
    );
    value!(
        direction: DIRECTION,
        overflow_x: OVERFLOW_X,
        overflow_y: OVERFLOW_Y,
        scrollbar_gutter: SCROLLBAR_GUTTER,
        // `display` owns both halves: `display: inherit` takes the
        // parent's outer and inner display.
        display: DISPLAY,
        flow: FLOW,
        white_space: WHITE_SPACE,
        user_select: USER_SELECT,
        pointer_events: POINTER_EVENTS,
        caret_color: CARET_COLOR,
        caret_text_color: CARET_TEXT_COLOR,
    );
    // Positioning (M2), transitions (M3; latest list wins) and counters
    // (CSS Lists 3 §3.1). None inherit by default.
    value!(
        position: POSITION,
        top: TOP,
        right: RIGHT,
        bottom: BOTTOM,
        left: LEFT,
        z_index: Z_INDEX,
        transition_property: TRANSITIONS,
        transition_duration: TRANSITIONS,
        transition_timing_function: TRANSITIONS,
        transition_delay: TRANSITIONS,
        counter_reset: COUNTER_RESET,
        counter_increment: COUNTER_INCREMENT,
    );
}

// ─── Applicators ────────────────────────────────────────────────────

/// Should this declaration actually apply during the current pass?
/// Normal pass applies normal declarations; important pass applies
/// important ones.
#[inline]
fn matches_pass(important_prop: bool, important_pass: bool) -> bool {
    important_prop == important_pass
}

/// The initial values `initial` resolves to: `ComputedStyle::initial()`,
/// the same table every element's cascade starts from, so the two
/// cannot drift (`P6G-APPLY-INITIALS-1`). Built on the first `initial`
/// keyword an element's cascade meets; most elements never build it.
#[derive(Default)]
pub(super) struct Initials(std::cell::OnceCell<ComputedStyle>);

impl Initials {
    fn get(&self) -> &ComputedStyle {
        self.0.get_or_init(ComputedStyle::initial)
    }
}

/// The CSS-wide keyword resolution every property shares: specified
/// as written, `inherit` the parent's computed value (inherited
/// property or not — CSS Cascade 4 §7.2), `initial` the property's
/// initial value (`field` of [`Initials`]).
fn apply_value<T: Clone>(
    target: &mut T,
    value: &Option<Value<T>>,
    important_prop: bool,
    important_pass: bool,
    inherit: &T,
    initial: &Initials,
    field: fn(&ComputedStyle) -> &T,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        *target = match v {
            Value::Specified(x) => x.clone(),
            Value::Inherit => inherit.clone(),
            Value::Initial => field(initial.get()).clone(),
        };
    }
}

/// [`apply_value`] for the `Option` fields, whose declared value is the
/// inner `T`.
fn apply_optional<T: Copy>(
    target: &mut Option<T>,
    value: &Option<Value<T>>,
    important_prop: bool,
    important_pass: bool,
    inherit: Option<T>,
    initial: &Initials,
    field: fn(&ComputedStyle) -> Option<T>,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        *target = match v {
            Value::Specified(x) => Some(*x),
            Value::Inherit => inherit,
            Value::Initial => field(initial.get()),
        };
    }
}

fn apply_color(
    target: &mut Color,
    value: &Option<Value<TuiColor>>,
    important_prop: bool,
    important_pass: bool,
    inherit: Color,
    initial: impl FnOnce() -> Color,
    vars: &std::collections::HashMap<String, String>,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        *target = match v {
            Value::Specified(tc) => resolve_tui_color(tc, vars, inherit),
            Value::Inherit => inherit,
            Value::Initial => initial(),
        };
    }
}

fn apply_border_collapse(
    target: &mut crate::layout::BorderCollapse,
    declared: &mut bool,
    value: &Option<Value<crate::layout::BorderCollapse>>,
    important_prop: bool,
    important_pass: bool,
    inherit: crate::layout::BorderCollapse,
    initial: &Initials,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        match v {
            Value::Specified(x) => {
                *target = *x;
                // Author wrote `border-collapse: collapse | separate;`
                // on THIS element — it becomes a collapse-root (the
                // boundary of its own group, equivalent to a CSS
                // `<table>`). See `ComputedStyle::border_collapse_declared`.
                *declared = true;
            }
            Value::Inherit => {
                // Explicit `inherit` keyword: semantically "use my
                // parent's value." Not a collapse-root declaration —
                // leave `declared` as-is (initial: false; never set
                // by inheritance).
                *target = inherit;
            }
            Value::Initial => {
                *target = initial.get().border_collapse;
                // `initial` resets to the property's initial value
                // (`separate`); the author IS declaring something on
                // this element, so it's a (trivial) collapse-root.
                *declared = true;
            }
        }
    }
}

/// Apply CSS `opacity` to the working `ComputedStyle`. Does not
/// inherit by default, but an explicit `inherit` takes the parent's
/// computed opacity. Clamped to `[0.0, 1.0]` defensively at cascade
/// time even though the `.opacity(f)` setter already clamps.
fn apply_opacity(
    working: &mut ComputedStyle,
    value: &Option<Value<f32>>,
    important_prop: bool,
    important_pass: bool,
    inherit: f32,
    initial: &Initials,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        working.opacity = match v {
            Value::Specified(v) => v.clamp(0.0, 1.0),
            Value::Inherit => inherit,
            Value::Initial => initial.get().opacity,
        };
    }
}

/// The `text-decoration` a set of modifier bits spells.
fn decoration_of(modifiers: Modifier) -> crate::layout::TextDecoration {
    use crate::layout::TextDecoration;
    if modifiers.contains(Modifier::UNDERLINED) {
        TextDecoration::Underline
    } else if modifiers.contains(Modifier::CROSSED_OUT) {
        TextDecoration::LineThrough
    } else {
        TextDecoration::None
    }
}

/// Apply CSS `text-decoration` to the working `ComputedStyle`. Maps
/// the enum value onto the `UNDERLINED` / `CROSSED_OUT` modifier
/// bits. `text-decoration: none` clears both. CSS-spec: the property
/// does NOT inherit by default (each element sets its own decoration),
/// but an explicit `text-decoration: inherit` copies the parent's
/// decoration bits; `initial` is `none`.
fn apply_text_decoration(
    working: &mut ComputedStyle,
    value: &Option<Value<crate::layout::TextDecoration>>,
    important_prop: bool,
    important_pass: bool,
    parent_modifiers: Modifier,
    initial: &Initials,
) {
    use crate::layout::TextDecoration;
    let Some(v) = value else { return };
    if !matches_pass(important_prop, important_pass) {
        return;
    }
    let resolved = match v {
        Value::Specified(v) => *v,
        Value::Inherit => decoration_of(parent_modifiers),
        Value::Initial => decoration_of(initial.get().modifiers),
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

/// `font-weight: bold` / `font-style: italic` — one modifier bit each.
fn apply_modifier_bit(
    working: &mut ComputedStyle,
    bit: Modifier,
    value: &Option<Value<bool>>,
    important_prop: bool,
    important_pass: bool,
    parent_modifiers: Modifier,
    initial: &Initials,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        let on = match v {
            Value::Specified(b) => *b,
            Value::Inherit => parent_modifiers.contains(bit),
            Value::Initial => initial.get().modifiers.contains(bit),
        };
        working.modifiers.set(bit, on);
    }
}
