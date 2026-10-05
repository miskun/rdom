//! The property name ↔ storage-field table: which `TuiStyle` fields
//! each CSS property owns (`Field`, `fields_of`), and the table-driven
//! operations that fold over it — `property_mask` (`!important` bits),
//! `remove`, and the inherited-property set (`inherits`). The names
//! themselves are `names`.

use super::css_wide::{CssWide, keyword_of};
use crate::TuiStyle;

pub(super) use super::names::all_property_names;
pub use super::names::{canonical_property_name, property_names};

/// Every field the names of [`all_property_names`] own, once each.
fn all_fields() -> &'static [Field] {
    static FIELDS: std::sync::OnceLock<Vec<Field>> = std::sync::OnceLock::new();
    FIELDS.get_or_init(|| {
        let mut out: Vec<Field> = Vec::new();
        for name in all_property_names() {
            for f in fields_of(name).unwrap_or(&[]) {
                if !out.contains(f) {
                    out.push(*f);
                }
            }
        }
        out
    })
}

macro_rules! define_fields {
    ($($variant:ident => $($field:ident).+ : $mask:ident,)+) => {
        /// One storage field of `TuiStyle`, as a row of the property →
        /// field table. `!important` routing, `removeProperty`, the
        /// CSS-wide keywords and their serialization all fold over
        /// [`fields_of`], so "which fields does this property own" is
        /// spelled once (`STYLE-PROPERTY-TABLES-1`).
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(super) enum Field {
            $($variant,)+
        }

        /// How many fields — and so `!important` bits — the table has.
        pub(crate) const IMPORTANT_BITS: usize = [$(Field::$variant,)+].len();

        /// The named constant of `!important` bit `index`, for `Debug`.
        pub(crate) fn important_bit_name(index: usize) -> &'static str {
            const NAMES: &[&str] = &[$(stringify!($mask),)+];
            NAMES[index]
        }

        // Each row's `!important` bit is the row's index: the constant
        // is named on the row, numbered by the table.
        impl crate::ImportantMask {
            $(
                #[doc = concat!("`!important` bit of `TuiStyle::", stringify!($($field).+), "`.")]
                pub const $mask: Self = Self::bit(Field::$variant as usize);
            )+
        }

        impl Field {
            /// Every field, for coverage tests.
            #[cfg(test)]
            pub(super) const ALL: &'static [Field] = &[$(Field::$variant,)+];

            /// The `!important` bit this field is guarded by — its row.
            pub(super) fn mask(self) -> crate::ImportantMask {
                crate::ImportantMask::bit(self as usize)
            }

            /// Every field.
            const EVERY: &'static [Field] = &[$(Field::$variant,)+];

            /// Whether `style` sets the field.
            fn is_set(self, style: &TuiStyle) -> bool {
                match self {
                    $(Field::$variant => style.$($field).+.is_some(),)+
                }
            }

            /// Copy the field from `from` to `to` when `from` sets it.
            fn copy(self, from: &TuiStyle, to: &mut TuiStyle) {
                match self {
                    $(Field::$variant => {
                        if let Some(v) = &from.$($field).+ {
                            to.$($field).+ = Some(v.clone());
                        }
                    })+
                }
            }

            /// Clear the field; `true` if it was set.
            fn take(self, style: &mut TuiStyle) -> bool {
                match self {
                    $(Field::$variant => style.$($field).+.take().is_some(),)+
                }
            }

            /// Store a CSS-wide keyword (resolved for `name`).
            pub(super) fn put_css_wide(self, style: &mut TuiStyle, kw: CssWide, name: &str) {
                match self {
                    $(Field::$variant => style.$($field).+ = Some(kw.into_value(name)),)+
                }
            }

            /// The CSS-wide keyword the field holds, if any.
            pub(super) fn css_wide(self, style: &TuiStyle) -> Option<&'static str> {
                match self {
                    $(Field::$variant => keyword_of(&style.$($field).+),)+
                }
            }
        }
    };
}

define_fields! {
    Fg => fg : FG,
    Bg => bg : BG,
    BorderTopColor => border_color.top : BORDER_TOP_COLOR,
    BorderRightColor => border_color.right : BORDER_RIGHT_COLOR,
    BorderBottomColor => border_color.bottom : BORDER_BOTTOM_COLOR,
    BorderLeftColor => border_color.left : BORDER_LEFT_COLOR,
    BorderTopWidth => border_width.top : BORDER_TOP_WIDTH,
    BorderRightWidth => border_width.right : BORDER_RIGHT_WIDTH,
    BorderBottomWidth => border_width.bottom : BORDER_BOTTOM_WIDTH,
    BorderLeftWidth => border_width.left : BORDER_LEFT_WIDTH,
    BackgroundImage => background_image : BACKGROUND_IMAGE,
    BackgroundPosition => background_position : BACKGROUND_POSITION,
    BackgroundSize => background_size : BACKGROUND_SIZE,
    BackgroundRepeat => background_repeat : BACKGROUND_REPEAT,
    BackgroundAttachment => background_attachment : BACKGROUND_ATTACHMENT,
    BackgroundOrigin => background_origin : BACKGROUND_ORIGIN,
    BackgroundClip => background_clip : BACKGROUND_CLIP,
    Bold => bold : BOLD,
    Italic => italic : ITALIC,
    TextDecoration => text_decoration : TEXT_DECORATION,
    Opacity => opacity : OPACITY,
    Display => display : DISPLAY,
    Flow => flow : FLOW,
    ListItem => list_item : LIST_ITEM,
    Direction => direction : FLEX_DIRECTION,
    FlexReverse => flex_reverse : FLEX_REVERSE,
    FlexWrap => flex_wrap : FLEX_WRAP,
    JustifyContent => justify_content : JUSTIFY_CONTENT,
    AlignItems => align_items : ALIGN_ITEMS,
    AlignContent => align_content : ALIGN_CONTENT,
    AlignSelf => align_self : ALIGN_SELF,
    JustifyItems => justify_items : JUSTIFY_ITEMS,
    JustifySelf => justify_self : JUSTIFY_SELF,
    TextDirection => text_direction : TEXT_DIRECTION,
    WritingMode => writing_mode : WRITING_MODE,
    WhiteSpace => white_space : WHITE_SPACE,
    UserSelect => user_select : USER_SELECT,
    PointerEvents => pointer_events : POINTER_EVENTS,
    Visibility => visibility : VISIBILITY,
    CaretColor => caret_color : CARET_COLOR,
    CaretTextColor => caret_text_color : CARET_TEXT_COLOR,
    OverflowX => overflow_x : OVERFLOW_X,
    OverflowY => overflow_y : OVERFLOW_Y,
    ScrollbarGutter => scrollbar_gutter : SCROLLBAR_GUTTER,
    ScrollBehavior => scroll_behavior : SCROLL_BEHAVIOR,
    Width => width : WIDTH,
    Height => height : HEIGHT,
    MinWidth => min_width : MIN_WIDTH,
    MaxWidth => max_width : MAX_WIDTH,
    MinHeight => min_height : MIN_HEIGHT,
    MaxHeight => max_height : MAX_HEIGHT,
    AspectRatio => aspect_ratio : ASPECT_RATIO,
    BoxSizing => box_sizing : BOX_SIZING,
    ContainIntrinsicWidth => contain_intrinsic_width : CONTAIN_INTRINSIC_WIDTH,
    ContainIntrinsicHeight => contain_intrinsic_height : CONTAIN_INTRINSIC_HEIGHT,
    RowGap => row_gap : ROW_GAP,
    ColumnGap => column_gap : COLUMN_GAP,
    FlexGrow => flex_grow : FLEX_GROW,
    FlexShrink => flex_shrink : FLEX_SHRINK,
    FlexBasis => flex_basis : FLEX_BASIS,
    Order => order : ORDER,
    GridTemplateColumns => grid_template_columns : GRID_TEMPLATE_COLUMNS,
    GridTemplateRows => grid_template_rows : GRID_TEMPLATE_ROWS,
    PaddingTop => padding.top : PADDING_TOP,
    PaddingRight => padding.right : PADDING_RIGHT,
    PaddingBottom => padding.bottom : PADDING_BOTTOM,
    PaddingLeft => padding.left : PADDING_LEFT,
    MarginTop => margin.top : MARGIN_TOP,
    MarginRight => margin.right : MARGIN_RIGHT,
    MarginBottom => margin.bottom : MARGIN_BOTTOM,
    MarginLeft => margin.left : MARGIN_LEFT,
    MarginTrim => margin_trim : MARGIN_TRIM,
    BorderTopStyle => border_style.top : BORDER_TOP_STYLE,
    BorderRightStyle => border_style.right : BORDER_RIGHT_STYLE,
    BorderBottomStyle => border_style.bottom : BORDER_BOTTOM_STYLE,
    BorderLeftStyle => border_style.left : BORDER_LEFT_STYLE,
    BorderTopLeftRadius => border_radius.top_left : BORDER_TOP_LEFT_RADIUS,
    BorderTopRightRadius => border_radius.top_right : BORDER_TOP_RIGHT_RADIUS,
    BorderBottomRightRadius => border_radius.bottom_right : BORDER_BOTTOM_RIGHT_RADIUS,
    BorderBottomLeftRadius => border_radius.bottom_left : BORDER_BOTTOM_LEFT_RADIUS,
    BoxShadow => box_shadow : BOX_SHADOW,
    BorderSpacing => border_spacing : BORDER_SPACING,
    BorderCollapse => border_collapse : BORDER_COLLAPSE,
    Content => content : CONTENT,
    Position => position : POSITION,
    Top => top : TOP,
    Right => right : RIGHT,
    Bottom => bottom : BOTTOM,
    Left => left : LEFT,
    ZIndex => z_index : Z_INDEX,
    TransitionProperty => transition_property : TRANSITION_PROPERTY,
    TransitionDuration => transition_duration : TRANSITION_DURATION,
    TransitionTimingFunction => transition_timing_function : TRANSITION_TIMING_FUNCTION,
    TransitionDelay => transition_delay : TRANSITION_DELAY,
    CounterReset => counter_reset : COUNTER_RESET,
    CounterIncrement => counter_increment : COUNTER_INCREMENT,
    ColorScheme => color_scheme : COLOR_SCHEME,
}

/// The fields a property name owns — the one property → field table.
/// Shorthands own several (`overflow` → X + Y, `inset` → the four
/// sides, `margin` → its four longhands); a per-side longhand owns its
/// side's field. `display` owns the derived
/// `flow` and `list_item` too, so removing or `inherit`ing `display`
/// cannot leave a stale inner type behind. `None` for unknown names.
pub(super) fn fields_of(name: &str) -> Option<&'static [Field]> {
    use Field::*;
    Some(match name {
        "color" => &[Fg],
        "background-color" => &[Bg],
        // CSS Backgrounds 3 §3.10: the shorthand sets every longhand.
        "background" => &[
            Bg,
            BackgroundImage,
            BackgroundPosition,
            BackgroundSize,
            BackgroundRepeat,
            BackgroundAttachment,
            BackgroundOrigin,
            BackgroundClip,
        ],
        "background-image" => &[BackgroundImage],
        "background-position" => &[BackgroundPosition],
        "background-size" => &[BackgroundSize],
        "background-repeat" => &[BackgroundRepeat],
        "background-attachment" => &[BackgroundAttachment],
        "background-origin" => &[BackgroundOrigin],
        "background-clip" => &[BackgroundClip],
        "border-color" => &[
            BorderTopColor,
            BorderRightColor,
            BorderBottomColor,
            BorderLeftColor,
        ],
        "font-weight" => &[Bold],
        "font-style" => &[Italic],
        "text-decoration" => &[TextDecoration],
        "opacity" => &[Opacity],
        "display" => &[Display, Flow, ListItem],
        "flex-direction" => &[Direction, FlexReverse],
        "flex-wrap" => &[FlexWrap],
        // CSS Flexbox §5.3: the shorthand sets both longhands.
        "flex-flow" => &[Direction, FlexReverse, FlexWrap],
        "justify-content" => &[JustifyContent],
        "align-items" => &[AlignItems],
        "align-content" => &[AlignContent],
        "align-self" => &[AlignSelf],
        "justify-items" => &[JustifyItems],
        "justify-self" => &[JustifySelf],
        // CSS Box Alignment 3 §5.5 / §6.4 / §6.5: each shorthand sets its
        // two longhands.
        "place-content" => &[AlignContent, JustifyContent],
        "place-items" => &[AlignItems, JustifyItems],
        "place-self" => &[AlignSelf, JustifySelf],
        "white-space" => &[WhiteSpace],
        "user-select" => &[UserSelect],
        "pointer-events" => &[PointerEvents],
        "visibility" => &[Visibility],
        "caret-color" => &[CaretColor],
        "caret-text-color" => &[CaretTextColor],
        "overflow" => &[OverflowX, OverflowY],
        "overflow-x" => &[OverflowX],
        "overflow-y" => &[OverflowY],
        "scrollbar-gutter" => &[ScrollbarGutter],
        "scroll-behavior" => &[ScrollBehavior],
        "width" => &[Width],
        "height" => &[Height],
        "min-width" => &[MinWidth],
        "max-width" => &[MaxWidth],
        "min-height" => &[MinHeight],
        "max-height" => &[MaxHeight],
        "aspect-ratio" => &[AspectRatio],
        "box-sizing" => &[BoxSizing],
        // CSS Sizing 4 §6.1; the logical longhands are the physical ones
        // in horizontal-tb (CSS Logical 1 §4), sharing their storage.
        "contain-intrinsic-size" => &[ContainIntrinsicWidth, ContainIntrinsicHeight],
        "contain-intrinsic-width" | "contain-intrinsic-inline-size" => &[ContainIntrinsicWidth],
        "contain-intrinsic-height" | "contain-intrinsic-block-size" => &[ContainIntrinsicHeight],
        // CSS Box Alignment 3 §8.3: `gap` sets both.
        "gap" => &[RowGap, ColumnGap],
        "row-gap" => &[RowGap],
        "column-gap" => &[ColumnGap],
        // CSS Flexbox §7.2: the shorthand sets its three longhands.
        "flex" => &[FlexGrow, FlexShrink, FlexBasis],
        "flex-grow" => &[FlexGrow],
        "flex-shrink" => &[FlexShrink],
        "flex-basis" => &[FlexBasis],
        "order" => &[Order],
        "grid-template-columns" => &[GridTemplateColumns],
        "grid-template-rows" => &[GridTemplateRows],
        // CSS Box 3 §3.2 / §4.2: the shorthand sets the four longhands.
        "padding" => &[PaddingTop, PaddingRight, PaddingBottom, PaddingLeft],
        "padding-top" => &[PaddingTop],
        "padding-right" => &[PaddingRight],
        "padding-bottom" => &[PaddingBottom],
        "padding-left" => &[PaddingLeft],
        "margin" => &[MarginTop, MarginRight, MarginBottom, MarginLeft],
        "margin-top" => &[MarginTop],
        "margin-right" => &[MarginRight],
        "margin-bottom" => &[MarginBottom],
        "margin-left" => &[MarginLeft],
        "margin-trim" => &[MarginTrim],
        // CSS Backgrounds 3 §4.4: `border` sets every side's style,
        // width and color; `border-<side>` its side's.
        "border" => &[
            BorderTopStyle,
            BorderRightStyle,
            BorderBottomStyle,
            BorderLeftStyle,
            BorderTopColor,
            BorderRightColor,
            BorderBottomColor,
            BorderLeftColor,
            BorderTopWidth,
            BorderRightWidth,
            BorderBottomWidth,
            BorderLeftWidth,
        ],
        "border-top" => &[BorderTopStyle, BorderTopColor, BorderTopWidth],
        "border-right" => &[BorderRightStyle, BorderRightColor, BorderRightWidth],
        "border-bottom" => &[BorderBottomStyle, BorderBottomColor, BorderBottomWidth],
        "border-left" => &[BorderLeftStyle, BorderLeftColor, BorderLeftWidth],
        "border-style" => &[
            BorderTopStyle,
            BorderRightStyle,
            BorderBottomStyle,
            BorderLeftStyle,
        ],
        "border-width" => &[
            BorderTopWidth,
            BorderRightWidth,
            BorderBottomWidth,
            BorderLeftWidth,
        ],
        "border-top-style" => &[BorderTopStyle],
        "border-right-style" => &[BorderRightStyle],
        "border-bottom-style" => &[BorderBottomStyle],
        "border-left-style" => &[BorderLeftStyle],
        "border-top-color" => &[BorderTopColor],
        "border-right-color" => &[BorderRightColor],
        "border-bottom-color" => &[BorderBottomColor],
        "border-left-color" => &[BorderLeftColor],
        "border-top-width" => &[BorderTopWidth],
        "border-right-width" => &[BorderRightWidth],
        "border-bottom-width" => &[BorderBottomWidth],
        "border-left-width" => &[BorderLeftWidth],
        // §5.2.
        "border-radius" => &[
            BorderTopLeftRadius,
            BorderTopRightRadius,
            BorderBottomRightRadius,
            BorderBottomLeftRadius,
        ],
        "border-top-left-radius" => &[BorderTopLeftRadius],
        "border-top-right-radius" => &[BorderTopRightRadius],
        "border-bottom-right-radius" => &[BorderBottomRightRadius],
        "border-bottom-left-radius" => &[BorderBottomLeftRadius],
        "box-shadow" => &[BoxShadow],
        "border-collapse" => &[BorderCollapse],
        "border-spacing" => &[BorderSpacing],
        "content" => &[Content],
        "position" => &[Position],
        "top" => &[Top],
        "right" => &[Right],
        "bottom" => &[Bottom],
        "left" => &[Left],
        "z-index" => &[ZIndex],
        "inset" => &[Top, Right, Bottom, Left],
        "transition-property" => &[TransitionProperty],
        "transition-duration" => &[TransitionDuration],
        "transition-timing-function" => &[TransitionTimingFunction],
        "transition-delay" => &[TransitionDelay],
        "transition" => &[
            TransitionProperty,
            TransitionDuration,
            TransitionTimingFunction,
            TransitionDelay,
        ],
        "counter-reset" => &[CounterReset],
        "counter-increment" => &[CounterIncrement],
        "color-scheme" => &[ColorScheme],
        "direction" => &[TextDirection],
        "writing-mode" => &[WritingMode],
        // CSS Cascade 4 §3.2: every property in the table.
        "all" => all_fields(),
        // CSS Logical 1: the physical properties' fields.
        _ => return super::logical::fields(name),
    })
}

/// Map a property name to the [`ImportantMask`](crate::tui_style::ImportantMask) bit(s) it owns: the OR
/// of every field's bit. Returns `None` for unknown names.
///
/// Two consumers: the `rdom-css` block parser's `!important`
/// routing, and `rdom-tui`'s `StyleDeclaration::set_property_
/// important` / `get_property_priority`.
pub fn property_mask(name: &str) -> Option<crate::ImportantMask> {
    let fields = fields_of(&canonical_property_name(name))?;
    Some(
        fields
            .iter()
            .fold(crate::ImportantMask::empty(), |m, f| m | f.mask()),
    )
}

/// How many of the table's storage fields `style` sets.
pub(crate) fn set_field_count(style: &TuiStyle) -> usize {
    Field::EVERY.iter().filter(|f| f.is_set(style)).count()
}

/// Copy every field of `mask` that `from` sets onto `to`.
pub(crate) fn copy_fields(from: &TuiStyle, to: &mut TuiStyle, mask: crate::ImportantMask) {
    for f in Field::EVERY {
        if mask.contains(f.mask()) {
            f.copy(from, to);
        }
    }
}

/// Clear the named property from `style` — reset its field(s) to
/// `None` and drop its `!important` bit. Returns `true` iff the
/// property was previously set (any of its fields was `Some`).
/// Returns `false` for unknown names.
///
/// A shorthand removes its longhands; a longhand (`padding-top`) its
/// own side only (CSSOM §6.6 `removeProperty`).
pub fn remove(name: &str, style: &mut TuiStyle) -> bool {
    if let Some(custom) = name.strip_prefix("--") {
        return style.remove_custom_property(custom);
    }
    let name = &*canonical_property_name(name);
    let Some(fields) = fields_of(name) else {
        return false;
    };
    // An inline-axis declaration writes no field of its block: removing
    // it leaves the physical declarations (and their bits) alone.
    if super::logical::is_directional(name) {
        let removed = super::logical::remove_inline_axis(name, style);
        drop_unneeded_pending(style);
        return removed;
    }
    let removed_kept = remove_kept_longhands(name, style);
    drop_unneeded_pending(style);
    // `|` not `||`: every field must be cleared, not just the first.
    let was_set = fields
        .iter()
        .fold(removed_kept, |acc, f| f.take(style) | acc);
    style.important = style
        .important
        .without(property_mask(name).unwrap_or_default());
    was_set
}

/// CSSOM §6.6 `removeProperty` of the physical property `name` among
/// the kept declarations: one that sets only `name`'s longhands (`name`
/// itself, or a longhand of it) goes; one that sets others too (a
/// shorthand of `name`) stays, restricted to those others
/// (`Restriction::Without`), so a substitution it waits for still
/// reaches them. A flow-relative declaration is a property of its own
/// (CSS Logical 1 §4: `margin-inline-start` is no longhand of `margin`)
/// and stays. Returns whether anything was removed.
fn remove_kept_longhands(name: &str, style: &mut TuiStyle) -> bool {
    use crate::var::Restriction;
    let gone = property_mask(name).unwrap_or_default();
    let mut removed = false;
    let mut kept = Vec::with_capacity(style.pending.len());
    for mut d in std::mem::take(&mut style.pending) {
        let own = property_mask(&d.name).unwrap_or_default();
        if d.directional || !own.intersects(gone) {
            kept.push(d);
            continue;
        }
        removed = true;
        if gone.contains(own) {
            continue;
        }
        match &mut d.restriction {
            Restriction::Without(names) => names.push(name.to_string()),
            other => *other = Restriction::Without(vec![name.to_string()]),
        }
        kept.push(d);
    }
    style.pending = kept;
    removed
}

/// Kept declarations are needed while one holds a substitution function
/// or an inline-axis property (the declarations after it keep their
/// order against it); without one, the block's fields say it all.
fn drop_unneeded_pending(style: &mut TuiStyle) {
    if !style
        .pending
        .iter()
        .any(|d| d.has_substitution || d.directional)
    {
        style.pending.clear();
    }
}

/// Does rdom inherit this property by default? The one declaration of
/// the inherited set: `rdom-tui`'s cascade copies exactly these from
/// parent to child (pinned by a cascade test), and this table decides
/// what `unset` means (CSS Cascade 4 §7.3: `inherit` for
/// inherited properties, `initial` otherwise).
pub fn inherits(name: &str) -> bool {
    matches!(
        &*canonical_property_name(name),
        "color"
            | "font-weight"
            | "font-style"
            | "white-space"
            | "pointer-events"
            | "visibility"
            | "caret-color"
            | "caret-text-color"
            | "color-scheme"
            | "border-spacing"
            | "direction"
            | "writing-mode"
    )
}
