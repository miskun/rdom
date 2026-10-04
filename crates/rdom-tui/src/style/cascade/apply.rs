//! Per-property applicators: one declaration block onto the working
//! `ComputedStyle`, for one ladder pass (the ladder itself is
//! `ladder.rs`).
//!
//! Applicators handle the `Value<T>` variants: `Specified`, and the
//! CSS-wide keywords `inherit` / `initial` / `revert` / `revert-layer`,
//! each of which
//! takes the field from a whole computed style ([`Keywords::resolve`]):
//! the parent's, `ComputedStyle::initial()` (via `Initials`, the table
//! every element's cascade starts from), or the ladder's rollback
//! state. Most properties share one generic path (`apply_value`). They
//! also honor the `important_pass` / `important_prop` pairing so normal
//! and important declarations apply in separate passes.

use super::ladder::Declarations;
use crate::layout::Display;
use crate::style::{
    Color, ComputedStyle, ImportantMask, Modifier, TuiColor, TuiStyle, Value, resolve_tui_color,
};

/// Where the CSS-wide keywords of one ladder pass take their values
/// from.
pub(super) struct Keywords<'a> {
    /// `inherit`: the parent's computed style (CSS Cascade 4 §7.2).
    pub parent: &'a ComputedStyle,
    /// `initial`: the initial values (§7.1).
    pub initial: &'a Initials,
    /// `revert`: the cascade rolled back to the previous origin
    /// (§7.3), computed on first use.
    pub revert: &'a dyn Fn() -> &'a ComputedStyle,
    /// `revert-layer`: the cascade rolled back to the previous cascade
    /// layer (Cascade 5 §7.4), computed on first use.
    pub revert_layer: &'a dyn Fn() -> &'a ComputedStyle,
}

/// A declared value, resolved for one pass: the specified value, or
/// the computed style a CSS-wide keyword copies the field from.
enum Resolved<'v, 'a, T> {
    Specified(&'v T),
    From(&'a ComputedStyle),
}

impl<'a> Keywords<'a> {
    fn resolve<'v, T>(&self, value: &'v Value<T>) -> Resolved<'v, 'a, T> {
        match value {
            Value::Specified(x) => Resolved::Specified(x),
            Value::Inherit => Resolved::From(self.parent),
            Value::Initial => Resolved::From(self.initial.get()),
            Value::Revert => Resolved::From((self.revert)()),
            Value::RevertLayer => Resolved::From((self.revert_layer)()),
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
pub(super) fn finalize_border_fg(working: &mut ComputedStyle, decls: Declarations<'_>) {
    if !decls.all().any(|s| s.border_fg.is_some()) {
        working.border_fg = working.fg;
    }
}

/// Apply one `TuiStyle` to `working`, for one ladder pass. Paints +
/// layout + display + white_space all in one pass.
pub(super) fn apply_style(
    working: &mut ComputedStyle,
    style: &TuiStyle,
    important_pass: bool,
    kw: &Keywords<'_>,
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
                kw,
                |c| &c.$field,
            );
        )*};
    }
    // `min-*` / `max-*`: `Option` fields, `None` = unset.
    macro_rules! optional {
        ($($field:ident: $mask:ident),* $(,)?) => {$(
            apply_optional(
                &mut working.$field,
                &style.$field,
                style.important.contains(ImportantMask::$mask),
                important_pass,
                kw,
                |c| &c.$field,
            );
        )*};
    }

    // Paint properties.
    apply_color(
        &mut working.fg,
        &style.fg,
        matches_pass(style.important.contains(ImportantMask::FG), important_pass),
        kw,
        |c| c.fg,
        None,
        &vars,
    );
    apply_color(
        &mut working.bg,
        &style.bg,
        matches_pass(style.important.contains(ImportantMask::BG), important_pass),
        kw,
        |c| c.bg,
        None,
        &vars,
    );
    // `border-color`'s initial value is `currentColor` (CSS Backgrounds
    // 3 §3.1): the element's `color` as cascaded so far.
    let current_color = working.fg;
    apply_color(
        &mut working.border_fg,
        &style.border_fg,
        matches_pass(
            style.important.contains(ImportantMask::BORDER_FG),
            important_pass,
        ),
        kw,
        |c| c.border_fg,
        Some(current_color),
        &vars,
    );

    apply_modifier_bit(
        working,
        Modifier::BOLD,
        &style.bold,
        style.important.contains(ImportantMask::BOLD),
        important_pass,
        kw,
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
        kw,
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
        kw,
    );
    apply_opacity(
        working,
        &style.opacity,
        style.important.contains(ImportantMask::OPACITY),
        important_pass,
        kw,
    );

    // Layout properties.
    value!(width: WIDTH, height: HEIGHT);
    optional!(
        min_width: MIN_WIDTH,
        max_width: MAX_WIDTH,
        min_height: MIN_HEIGHT,
        max_height: MAX_HEIGHT,
    );
    // `aspect-ratio`: the declared value is the computed `Option` itself
    // (`auto` alone is `None`).
    value!(aspect_ratio: ASPECT_RATIO);
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
        kw,
    );
    value!(
        direction: DIRECTION,
        overflow_x: OVERFLOW_X,
        overflow_y: OVERFLOW_Y,
        scrollbar_gutter: SCROLLBAR_GUTTER,
        scroll_behavior: SCROLL_BEHAVIOR,
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
    pub(super) fn get(&self) -> &ComputedStyle {
        self.0.get_or_init(ComputedStyle::initial)
    }
}

/// The CSS-wide keyword resolution every property shares: specified
/// as written, a keyword the `field` of its source style
/// ([`Keywords::resolve`]) — `inherit` the parent's computed value
/// (inherited property or not — CSS Cascade 4 §7.2), `initial` the
/// property's initial value, `revert` the rolled-back cascade's.
fn apply_value<T: Clone>(
    target: &mut T,
    value: &Option<Value<T>>,
    important_prop: bool,
    important_pass: bool,
    kw: &Keywords<'_>,
    field: fn(&ComputedStyle) -> &T,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        *target = match kw.resolve(v) {
            Resolved::Specified(x) => x.clone(),
            Resolved::From(source) => field(source).clone(),
        };
    }
}

/// [`apply_value`] for the `Option` fields, whose declared value is the
/// inner `T`.
fn apply_optional<T: Clone>(
    target: &mut Option<T>,
    value: &Option<Value<T>>,
    important_prop: bool,
    important_pass: bool,
    kw: &Keywords<'_>,
    field: fn(&ComputedStyle) -> &Option<T>,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        *target = match kw.resolve(v) {
            Resolved::Specified(x) => Some(x.clone()),
            Resolved::From(source) => field(source).clone(),
        };
    }
}

/// A color property (`in_pass`: the declaration's importance matches
/// the pass). `initial_override` replaces the initial-table
/// value for `initial` (`border-color`'s initial value is
/// `currentColor`, the element's `color` as cascaded so far).
fn apply_color(
    target: &mut Color,
    value: &Option<Value<TuiColor>>,
    in_pass: bool,
    kw: &Keywords<'_>,
    field: fn(&ComputedStyle) -> Color,
    initial_override: Option<Color>,
    vars: &std::collections::HashMap<String, rdom_style::CustomValue>,
) {
    if let Some(v) = value
        && in_pass
    {
        *target = match (v, initial_override) {
            (Value::Initial, Some(initial)) => initial,
            _ => match kw.resolve(v) {
                Resolved::Specified(tc) => resolve_tui_color(tc, vars, field(kw.parent)),
                Resolved::From(source) => field(source),
            },
        };
    }
}

fn apply_border_collapse(
    target: &mut crate::layout::BorderCollapse,
    declared: &mut bool,
    value: &Option<Value<crate::layout::BorderCollapse>>,
    important_prop: bool,
    important_pass: bool,
    kw: &Keywords<'_>,
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
                *target = kw.parent.border_collapse;
            }
            Value::Initial => {
                *target = kw.initial.get().border_collapse;
                // `initial` resets to the property's initial value
                // (`separate`); the author IS declaring something on
                // this element, so it's a (trivial) collapse-root.
                *declared = true;
            }
            Value::Revert | Value::RevertLayer => {
                // Whatever the rolled-back cascade had, collapse-root
                // flag included.
                let source = if matches!(v, Value::Revert) {
                    (kw.revert)()
                } else {
                    (kw.revert_layer)()
                };
                *target = source.border_collapse;
                *declared = source.border_collapse_declared;
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
    kw: &Keywords<'_>,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        working.opacity = match kw.resolve(v) {
            Resolved::Specified(v) => v.clamp(0.0, 1.0),
            Resolved::From(source) => source.opacity,
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
    kw: &Keywords<'_>,
) {
    use crate::layout::TextDecoration;
    let Some(v) = value else { return };
    if !matches_pass(important_prop, important_pass) {
        return;
    }
    let resolved = match kw.resolve(v) {
        Resolved::Specified(v) => *v,
        Resolved::From(source) => decoration_of(source.modifiers),
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
    kw: &Keywords<'_>,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        let on = match kw.resolve(v) {
            Resolved::Specified(b) => *b,
            Resolved::From(source) => source.modifiers.contains(bit),
        };
        working.modifiers.set(bit, on);
    }
}
