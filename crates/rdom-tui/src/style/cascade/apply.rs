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

pub(super) use super::colors::ElementColors;
use super::colors::apply_colors;
use super::decoration::apply_decoration;
use crate::layout::Display;
use crate::style::{ComputedStyle, ImportantMask, Modifier, TuiStyle, Value};

/// Where the CSS-wide keywords of one ladder pass take their values
/// from.
pub(super) struct Keywords<'a> {
    /// `inherit`: the parent's computed style (CSS Cascade 4 §7.2).
    pub parent: &'a ComputedStyle,
    /// The document's preferred color scheme (CSS Color Adjust 1
    /// §2.1), which the colors cascaded so far resolve under.
    pub preferred_scheme: rdom_style::color::ColorScheme,
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
pub(super) enum Resolved<'v, 'a, T> {
    Specified(&'v T),
    From(&'a ComputedStyle),
}

impl<'a> Keywords<'a> {
    pub(super) fn resolve<'v, T>(&self, value: &'v Value<T>) -> Resolved<'v, 'a, T> {
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
    working.establishes_new_bfc = matches!(working.flow, Flow::Flex | Flow::FlowRoot)
        || matches!(working.display, Display::InlineBlock)
        || !matches!(working.overflow_x, Overflow::Visible)
        || !matches!(working.overflow_y, Overflow::Visible)
        || matches!(working.position, Position::Absolute | Position::Fixed);
}

/// CSS Display 3 Appendix B: `display: contents` on a replaced element
/// or a form control — whose children are not its rendering — behaves
/// as `display: none`. rdom computes it so, so layout, paint, hit
/// testing and focus all see no box.
pub(super) fn finalize_unusual_contents(working: &mut ComputedStyle, tag: Option<&str>) {
    const NO_CONTENTS: &[&str] = &[
        "br", "wbr", "meter", "progress", "canvas", "embed", "object", "audio", "iframe", "img",
        "video", "frame", "frameset", "input", "textarea", "select",
    ];
    if working.display == Display::Contents && tag.is_some_and(|t| NO_CONTENTS.contains(&t)) {
        working.display = Display::None;
    }
}

/// Apply one `TuiStyle` to `working`, for one ladder pass. Paints +
/// layout + display + white_space all in one pass.
pub(super) fn apply_style(
    working: &mut ComputedStyle,
    colors: &mut ElementColors,
    style: &TuiStyle,
    important_pass: bool,
    kw: &Keywords<'_>,
) {
    // One declared value → one `ComputedStyle` field of the same type:
    // specified as written, `inherit` the parent's field, `initial` the
    // field of `ComputedStyle::initial()`.
    macro_rules! value {
        ($($($field:ident).+: $mask:ident),* $(,)?) => {$(
            apply_value(
                &mut working.$($field).+,
                &style.$($field).+,
                style.important.contains(ImportantMask::$mask),
                important_pass,
                kw,
                |c| &c.$($field).+,
            );
        )*};
    }

    // Paint properties (`colors.rs`, `decoration.rs`).
    apply_colors(working, colors, style, important_pass, kw);
    apply_decoration(working, style, important_pass, kw);

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
    value!(min_width: MIN_WIDTH, min_height: MIN_HEIGHT);
    // `max-*`: the declared value is the computed `Option` itself
    // (`none` is `None`).
    value!(max_width: MAX_WIDTH, max_height: MAX_HEIGHT);
    value!(box_sizing: BOX_SIZING);
    value!(
        contain_intrinsic_width: CONTAIN_INTRINSIC_WIDTH,
        contain_intrinsic_height: CONTAIN_INTRINSIC_HEIGHT,
    );
    // `aspect-ratio`: the declared value is the computed `Option` itself
    // (`auto` alone is `None`).
    value!(aspect_ratio: ASPECT_RATIO);
    // The `padding-*` / `margin-*` longhands, one declaration per side
    // (CSS Box 3 §3.2 / §4.2).
    value!(
        padding.top: PADDING_TOP,
        padding.right: PADDING_RIGHT,
        padding.bottom: PADDING_BOTTOM,
        padding.left: PADDING_LEFT,
        margin.top: MARGIN_TOP,
        margin.right: MARGIN_RIGHT,
        margin.bottom: MARGIN_BOTTOM,
        margin.left: MARGIN_LEFT,
    );
    value!(
        margin_trim: MARGIN_TRIM,
        row_gap: ROW_GAP,
        column_gap: COLUMN_GAP,
        flex_grow: FLEX_GROW,
        flex_shrink: FLEX_SHRINK,
        flex_basis: FLEX_BASIS,
        order: ORDER,
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
        direction: FLEX_DIRECTION,
        flex_reverse: FLEX_REVERSE,
        text_direction: TEXT_DIRECTION,
        writing_mode: WRITING_MODE,
        overflow_x: OVERFLOW_X,
        overflow_y: OVERFLOW_Y,
        scrollbar_gutter: SCROLLBAR_GUTTER,
        scroll_behavior: SCROLL_BEHAVIOR,
        // `display` owns both halves: `display: inherit` takes the
        // parent's outer and inner display.
        display: DISPLAY,
        flow: FLOW,
        list_item: LIST_ITEM,
        white_space: WHITE_SPACE,
        user_select: USER_SELECT,
        pointer_events: POINTER_EVENTS,
        visibility: VISIBILITY,
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
        transition_property: TRANSITION_PROPERTY,
        transition_duration: TRANSITION_DURATION,
        transition_timing_function: TRANSITION_TIMING_FUNCTION,
        transition_delay: TRANSITION_DELAY,
        counter_reset: COUNTER_RESET,
        counter_increment: COUNTER_INCREMENT,
        // Inherits; `light-dark()` picks by it (CSS Color Adjust 1 §2).
        color_scheme: COLOR_SCHEME,
        // Inherits (CSS 2.1 §17.6.1); laid out with C13-TFC.
        border_spacing: BORDER_SPACING,
    );
}

// ─── Applicators ────────────────────────────────────────────────────

/// Should this declaration actually apply during the current pass?
/// Normal pass applies normal declarations; important pass applies
/// important ones.
#[inline]
pub(super) fn matches_pass(important_prop: bool, important_pass: bool) -> bool {
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
