//! Which `TuiStyle` storage fields each CSS property owns — the one
//! property → field map ([`fields_of`]) that `!important` routing,
//! `removeProperty`, the CSS-wide keywords and their serialization fold
//! over (`table`).

use super::names::all_property_names;
use super::table::Field;

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
        "overflow-clip-margin" => &[OverflowClipMargin],
        "text-overflow" => &[TextOverflow],
        "line-clamp" | "-webkit-line-clamp" => &[MaxLines, BlockEllipsis, Continue],
        "max-lines" => &[MaxLines],
        "block-ellipsis" => &[BlockEllipsis],
        "continue" => &[Continue],
        "-webkit-box-orient" => &[WebkitBoxOrient],
        "scrollbar-gutter" => &[ScrollbarGutter],
        "scrollbar-width" => &[ScrollbarWidth],
        "scrollbar-color" => &[ScrollbarColor],
        "overscroll-behavior" => &[OverscrollBehaviorX, OverscrollBehaviorY],
        "overscroll-behavior-x" => &[OverscrollBehaviorX],
        "overscroll-behavior-y" => &[OverscrollBehaviorY],
        "scroll-padding" => &[
            ScrollPaddingTop,
            ScrollPaddingRight,
            ScrollPaddingBottom,
            ScrollPaddingLeft,
        ],
        "scroll-margin" => &[
            ScrollMarginTop,
            ScrollMarginRight,
            ScrollMarginBottom,
            ScrollMarginLeft,
        ],
        "scroll-padding-top" => &[ScrollPaddingTop],
        "scroll-padding-right" => &[ScrollPaddingRight],
        "scroll-padding-bottom" => &[ScrollPaddingBottom],
        "scroll-padding-left" => &[ScrollPaddingLeft],
        "scroll-margin-top" => &[ScrollMarginTop],
        "scroll-margin-right" => &[ScrollMarginRight],
        "scroll-margin-bottom" => &[ScrollMarginBottom],
        "scroll-margin-left" => &[ScrollMarginLeft],
        "scroll-snap-type" => &[ScrollSnapType],
        "scroll-snap-align" => &[ScrollSnapAlign],
        "scroll-snap-stop" => &[ScrollSnapStop],
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
        "grid-template-areas" => &[GridTemplateAreas],
        // CSS Grid 2 §7.4 / §7.8: the explicit grid's three longhands,
        // and with them the implicit grid's three (not the gutters).
        "grid-template" => &[GridTemplateRows, GridTemplateColumns, GridTemplateAreas],
        "grid" => &[
            GridTemplateRows,
            GridTemplateColumns,
            GridTemplateAreas,
            GridAutoRows,
            GridAutoColumns,
            GridAutoFlow,
        ],
        "grid-auto-columns" => &[GridAutoColumns],
        "grid-auto-rows" => &[GridAutoRows],
        "grid-auto-flow" => &[GridAutoFlow],
        "grid-row-start" => &[GridRowStart],
        "grid-row-end" => &[GridRowEnd],
        "grid-column-start" => &[GridColumnStart],
        "grid-column-end" => &[GridColumnEnd],
        // CSS Grid 2 §8.4: each shorthand sets its longhands.
        "grid-row" => &[GridRowStart, GridRowEnd],
        "grid-column" => &[GridColumnStart, GridColumnEnd],
        "grid-area" => &[GridRowStart, GridColumnStart, GridRowEnd, GridColumnEnd],
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
        "float" => &[Float],
        "clear" => &[Clear],
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
