//! The property name ↔ storage-field table: the `TuiStyle` storage
//! fields (`Field`), and the table-driven operations that fold over the
//! fields each CSS property owns (`fields::fields_of`) — `property_mask`
//! (`!important` bits), the copy and coverage folds. `removeProperty` is
//! `remove`, the inherited-property set `inherited`; the names themselves
//! are `names`.

use super::css_wide::{CssWide, keyword_of};
use crate::TuiStyle;

pub(super) use super::fields::fields_of;
pub use super::inherited::inherits;
pub(super) use super::names::all_property_names;
pub use super::names::{canonical_property_name, property_names};
pub use super::remove::remove;

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
            pub(super) fn take(self, style: &mut TuiStyle) -> bool {
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
    FontWeight => font.weight : FONT_WEIGHT,
    FontStyle => font.style : FONT_STYLE,
    FontSize => font.size : FONT_SIZE,
    FontFamily => font.family : FONT_FAMILY,
    FontStretch => font.stretch : FONT_STRETCH,
    FontVariant => font.variant : FONT_VARIANT,
    TextDecorationLine => text_decoration.line : TEXT_DECORATION_LINE,
    TextDecorationStyle => text_decoration.style : TEXT_DECORATION_STYLE,
    TextDecorationColor => text_decoration.color : TEXT_DECORATION_COLOR,
    TextDecorationThickness => text_decoration.thickness : TEXT_DECORATION_THICKNESS,
    Opacity => opacity : OPACITY,
    Display => display : DISPLAY,
    Flow => flow : FLOW,
    ListItem => list_item : LIST_ITEM,
    WebkitBox => webkit_box : WEBKIT_BOX,
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
    WhiteSpaceCollapse => text.white_space_collapse : WHITE_SPACE_COLLAPSE,
    TextWrapMode => text.text_wrap_mode : TEXT_WRAP_MODE,
    WordBreak => text.word_break : WORD_BREAK,
    OverflowWrap => text.overflow_wrap : OVERFLOW_WRAP,
    LineBreak => text.line_break : LINE_BREAK,
    Hyphens => text.hyphens : HYPHENS,
    TabSize => text.tab_size : TAB_SIZE,
    TextTransform => text.text_transform : TEXT_TRANSFORM,
    TextIndent => text.text_indent : TEXT_INDENT,
    TextAlignAll => text.text_align_all : TEXT_ALIGN_ALL,
    TextAlignLast => text.text_align_last : TEXT_ALIGN_LAST,
    TextJustify => text.text_justify : TEXT_JUSTIFY,
    TextWrapStyle => text.text_wrap_style : TEXT_WRAP_STYLE,
    LetterSpacing => text.letter_spacing : LETTER_SPACING,
    WordSpacing => text.word_spacing : WORD_SPACING,
    LineHeight => text.line_height : LINE_HEIGHT,
    VerticalAlign => vertical_align : VERTICAL_ALIGN,
    TextUnderlineOffset => text.text_underline_offset : TEXT_UNDERLINE_OFFSET,
    TextUnderlinePosition => text.text_underline_position : TEXT_UNDERLINE_POSITION,
    TextDecorationSkipInk => text.text_decoration_skip_ink : TEXT_DECORATION_SKIP_INK,
    UserSelect => user_select : USER_SELECT,
    PointerEvents => pointer_events : POINTER_EVENTS,
    Visibility => visibility : VISIBILITY,
    CaretColor => caret_color : CARET_COLOR,
    CaretTextColor => caret_text_color : CARET_TEXT_COLOR,
    OverflowX => overflow_x : OVERFLOW_X,
    OverflowY => overflow_y : OVERFLOW_Y,
    OverflowClipMargin => overflow_clip_margin : OVERFLOW_CLIP_MARGIN,
    TextOverflow => text_overflow : TEXT_OVERFLOW,
    MaxLines => max_lines : MAX_LINES,
    BlockEllipsis => block_ellipsis : BLOCK_ELLIPSIS,
    Continue => continue_ : CONTINUE,
    WebkitBoxOrient => webkit_box_orient : WEBKIT_BOX_ORIENT,
    ScrollbarGutter => scrollbar_gutter : SCROLLBAR_GUTTER,
    ScrollbarWidth => scrollbar_width : SCROLLBAR_WIDTH,
    ScrollbarColor => scrollbar_color : SCROLLBAR_COLOR,
    OutlineStyle => ui.outline_style : OUTLINE_STYLE,
    OutlineWidth => ui.outline_width : OUTLINE_WIDTH,
    OutlineColor => ui.outline_color : OUTLINE_COLOR,
    OutlineOffset => ui.outline_offset : OUTLINE_OFFSET,
    Cursor => ui.cursor : CURSOR,
    CaretShape => ui.caret_shape : CARET_SHAPE,
    CaretAnimation => ui.caret_animation : CARET_ANIMATION,
    AccentColor => ui.accent_color : ACCENT_COLOR,
    Appearance => ui.appearance : APPEARANCE,
    FormFieldSizing => ui.field_sizing : FIELD_SIZING,
    Resize => ui.resize : RESIZE,
    TableLayout => table.table_layout : TABLE_LAYOUT,
    CaptionSide => table.caption_side : CAPTION_SIDE,
    EmptyCells => table.empty_cells : EMPTY_CELLS,
    OverscrollBehaviorX => overscroll_behavior_x : OVERSCROLL_BEHAVIOR_X,
    OverscrollBehaviorY => overscroll_behavior_y : OVERSCROLL_BEHAVIOR_Y,
    ScrollPaddingTop => scroll_padding.top : SCROLL_PADDING_TOP,
    ScrollPaddingRight => scroll_padding.right : SCROLL_PADDING_RIGHT,
    ScrollPaddingBottom => scroll_padding.bottom : SCROLL_PADDING_BOTTOM,
    ScrollPaddingLeft => scroll_padding.left : SCROLL_PADDING_LEFT,
    ScrollMarginTop => scroll_margin.top : SCROLL_MARGIN_TOP,
    ScrollMarginRight => scroll_margin.right : SCROLL_MARGIN_RIGHT,
    ScrollMarginBottom => scroll_margin.bottom : SCROLL_MARGIN_BOTTOM,
    ScrollMarginLeft => scroll_margin.left : SCROLL_MARGIN_LEFT,
    ScrollSnapType => scroll_snap_type : SCROLL_SNAP_TYPE,
    ScrollSnapAlign => scroll_snap_align : SCROLL_SNAP_ALIGN,
    ScrollSnapStop => scroll_snap_stop : SCROLL_SNAP_STOP,
    ScrollBehavior => scroll_behavior : SCROLL_BEHAVIOR,
    Width => width : WIDTH,
    Height => height : HEIGHT,
    MinWidth => min_width : MIN_WIDTH,
    MaxWidth => max_width : MAX_WIDTH,
    MinHeight => min_height : MIN_HEIGHT,
    MaxHeight => max_height : MAX_HEIGHT,
    AspectRatio => aspect_ratio : ASPECT_RATIO,
    BoxSizing => box_sizing : BOX_SIZING,
    InterpolateSize => interpolate_size : INTERPOLATE_SIZE,
    Contain => contain : CONTAIN,
    ContentVisibility => content_visibility : CONTENT_VISIBILITY,
    WillChange => will_change : WILL_CHANGE,
    Translate => effects.translate : TRANSLATE,
    Rotate => effects.rotate : ROTATE,
    Scale => effects.scale : SCALE,
    Transform => effects.transform : TRANSFORM,
    TransformOrigin => effects.transform_origin : TRANSFORM_ORIGIN,
    TransformBox => effects.transform_box : TRANSFORM_BOX,
    Filter => effects.filter : FILTER,
    BackdropFilter => effects.backdrop_filter : BACKDROP_FILTER,
    MixBlendMode => effects.mix_blend_mode : MIX_BLEND_MODE,
    Isolation => effects.isolation : ISOLATION,
    BackgroundBlendMode => effects.background_blend_mode : BACKGROUND_BLEND_MODE,
    ClipPath => effects.clip_path : CLIP_PATH,
    Clip => effects.clip : CLIP,
    ColumnCount => multicol.column_count : COLUMN_COUNT,
    ColumnWidth => multicol.column_width : COLUMN_WIDTH,
    ColumnRuleStyle => multicol.column_rule_style : COLUMN_RULE_STYLE,
    ColumnRuleWidth => multicol.column_rule_width : COLUMN_RULE_WIDTH,
    ColumnRuleColor => multicol.column_rule_color : COLUMN_RULE_COLOR,
    ColumnSpan => multicol.column_span : COLUMN_SPAN,
    ColumnFill => multicol.column_fill : COLUMN_FILL,
    BreakBefore => fragmentation.break_before : BREAK_BEFORE,
    BreakAfter => fragmentation.break_after : BREAK_AFTER,
    BreakInside => fragmentation.break_inside : BREAK_INSIDE,
    Orphans => fragmentation.orphans : ORPHANS,
    Widows => fragmentation.widows : WIDOWS,
    BoxDecorationBreak => fragmentation.box_decoration_break : BOX_DECORATION_BREAK,
    AnchorName => anchor.anchor_name : ANCHOR_NAME,
    AnchorScope => anchor.anchor_scope : ANCHOR_SCOPE,
    PositionAnchor => anchor.position_anchor : POSITION_ANCHOR,
    PositionArea => anchor.position_area : POSITION_AREA,
    PositionTryFallbacks => anchor.position_try_fallbacks : POSITION_TRY_FALLBACKS,
    PositionTryOrder => anchor.position_try_order : POSITION_TRY_ORDER,
    PositionVisibility => anchor.position_visibility : POSITION_VISIBILITY,
    MaskImage => masks.mask_image : MASK_IMAGE,
    MaskMode => masks.mask_mode : MASK_MODE,
    MaskRepeat => masks.mask_repeat : MASK_REPEAT,
    MaskPosition => masks.mask_position : MASK_POSITION,
    MaskClip => masks.mask_clip : MASK_CLIP,
    MaskOrigin => masks.mask_origin : MASK_ORIGIN,
    MaskSize => masks.mask_size : MASK_SIZE,
    MaskComposite => masks.mask_composite : MASK_COMPOSITE,
    MaskType => masks.mask_type : MASK_TYPE,
    MaskBorderSource => masks.mask_border_source : MASK_BORDER_SOURCE,
    MaskBorderSlice => masks.mask_border_slice : MASK_BORDER_SLICE,
    MaskBorderWidth => masks.mask_border_width : MASK_BORDER_WIDTH,
    MaskBorderOutset => masks.mask_border_outset : MASK_BORDER_OUTSET,
    MaskBorderRepeat => masks.mask_border_repeat : MASK_BORDER_REPEAT,
    MaskBorderMode => masks.mask_border_mode : MASK_BORDER_MODE,
    ContainerType => container_type : CONTAINER_TYPE,
    ContainerName => container_name : CONTAINER_NAME,
    ContainIntrinsicWidth => contain_intrinsic_width : CONTAIN_INTRINSIC_WIDTH,
    ContainIntrinsicHeight => contain_intrinsic_height : CONTAIN_INTRINSIC_HEIGHT,
    RowGap => row_gap : ROW_GAP,
    ColumnGap => column_gap : COLUMN_GAP,
    FlexGrow => flex_grow : FLEX_GROW,
    FlexShrink => flex_shrink : FLEX_SHRINK,
    FlexBasis => flex_basis : FLEX_BASIS,
    Order => order : ORDER,
    GridTemplateColumns => grid.grid_template_columns : GRID_TEMPLATE_COLUMNS,
    GridTemplateRows => grid.grid_template_rows : GRID_TEMPLATE_ROWS,
    GridTemplateAreas => grid.grid_template_areas : GRID_TEMPLATE_AREAS,
    GridAutoColumns => grid.grid_auto_columns : GRID_AUTO_COLUMNS,
    GridAutoRows => grid.grid_auto_rows : GRID_AUTO_ROWS,
    GridAutoFlow => grid.grid_auto_flow : GRID_AUTO_FLOW,
    GridRowStart => grid.grid_row_start : GRID_ROW_START,
    GridRowEnd => grid.grid_row_end : GRID_ROW_END,
    GridColumnStart => grid.grid_column_start : GRID_COLUMN_START,
    GridColumnEnd => grid.grid_column_end : GRID_COLUMN_END,
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
    Quotes => quotes : QUOTES,
    ListStyleType => list_style_type : LIST_STYLE_TYPE,
    ListStylePosition => list_style_position : LIST_STYLE_POSITION,
    ListStyleImage => list_style_image : LIST_STYLE_IMAGE,
    MarkerSide => marker_side : MARKER_SIDE,
    Position => position : POSITION,
    Top => top : TOP,
    Right => right : RIGHT,
    Bottom => bottom : BOTTOM,
    Left => left : LEFT,
    ZIndex => z_index : Z_INDEX,
    Overlay => overlay : OVERLAY,
    Float => float : FLOAT,
    Clear => clear : CLEAR,
    TransitionProperty => motion.transition_property : TRANSITION_PROPERTY,
    TransitionDuration => motion.transition_duration : TRANSITION_DURATION,
    TransitionTimingFunction => motion.transition_timing_function : TRANSITION_TIMING_FUNCTION,
    TransitionDelay => motion.transition_delay : TRANSITION_DELAY,
    TransitionBehavior => motion.transition_behavior : TRANSITION_BEHAVIOR,
    AnimationName => motion.animation_name : ANIMATION_NAME,
    AnimationDuration => motion.animation_duration : ANIMATION_DURATION,
    AnimationTimingFunction => motion.animation_timing_function : ANIMATION_TIMING_FUNCTION,
    AnimationDelay => motion.animation_delay : ANIMATION_DELAY,
    AnimationIterationCount => motion.animation_iteration_count : ANIMATION_ITERATION_COUNT,
    AnimationDirection => motion.animation_direction : ANIMATION_DIRECTION,
    AnimationFillMode => motion.animation_fill_mode : ANIMATION_FILL_MODE,
    AnimationPlayState => motion.animation_play_state : ANIMATION_PLAY_STATE,
    AnimationComposition => motion.animation_composition : ANIMATION_COMPOSITION,
    AnimationTimeline => motion.animation_timeline : ANIMATION_TIMELINE,
    ScrollTimelineName => motion.scroll_timeline_name : SCROLL_TIMELINE_NAME,
    ScrollTimelineAxis => motion.scroll_timeline_axis : SCROLL_TIMELINE_AXIS,
    ViewTimelineName => motion.view_timeline_name : VIEW_TIMELINE_NAME,
    ViewTimelineAxis => motion.view_timeline_axis : VIEW_TIMELINE_AXIS,
    ViewTimelineInset => motion.view_timeline_inset : VIEW_TIMELINE_INSET,
    TimelineScope => motion.timeline_scope : TIMELINE_SCOPE,
    AnimationRangeStart => motion.animation_range_start : ANIMATION_RANGE_START,
    AnimationRangeEnd => motion.animation_range_end : ANIMATION_RANGE_END,
    CounterReset => counter_reset : COUNTER_RESET,
    CounterIncrement => counter_increment : COUNTER_INCREMENT,
    CounterSet => counter_set : COUNTER_SET,
    ColorScheme => color_scheme : COLOR_SCHEME,
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

/// Whether the property `outer` owns every storage field of `inner` (and
/// `inner` owns one): `inner` is `outer` itself, one of its longhands, or
/// a companion it writes. What expands a shorthand named by
/// `transition-property` into the longhands it animates.
pub(crate) fn covers(outer: &str, inner: &str) -> bool {
    match (fields_of(outer), fields_of(inner)) {
        (Some(o), Some(i)) => !i.is_empty() && i.iter().all(|f| o.contains(f)),
        _ => false,
    }
}

/// Whether `style` sets a storage field of the property `name` — a
/// block declaring it (or a shorthand covering it).
pub(crate) fn sets_any_field(style: &TuiStyle, name: &str) -> bool {
    fields_of(&canonical_property_name(name)).is_some_and(|f| f.iter().any(|f| f.is_set(style)))
}
