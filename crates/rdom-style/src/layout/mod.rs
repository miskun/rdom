//! Layout data model for flexbox-style TUI layout — the value types
//! the cascade computes and rdom-tui's layout pass reads.
//!
//! Cell-based dimensions throughout. [`LayoutRect`] uses signed `i32`
//! for position so elements can sit above / left of the viewport
//! (needed for scroll clipping). [`Size`] + [`Direction`] +
//! [`Overflow`] + [`Border`] + [`Padding`] cover the CSS-like sizing
//! model.
//!
//! The geometry that applies these values to rectangles (content
//! area, padding box, min / max clamping) belongs to the layout pass
//! and lives in rdom-tui (`rdom_tui::layout::compute_content_area`
//! and friends).
//!
//! - `rect` — `LayoutRect`
//! - `alignment` — the Box Alignment keywords (`Align`, `Alignment`)
//! - `keywords` — keyword-valued properties
//! - `sizing` — `Size`, `MinSize`, `MaxSize`, `FlexBasis`, `Length`
//! - `aspect_ratio` — `AspectRatio`; `gap` — `GapValue`
//! - `border` — border styles, widths, `border-collapse`
//! - `box_model` — padding, margin
//! - `grid` — the grid track lists (`GridTemplate`, `TrackSize`)
//! - `grid_areas` — named grid areas (`GridTemplateAreas`)
//! - `grid_placement` — grid item placement (`GridLine`, `GridAutoFlow`)
//! - `sides` — `Sides`, the per-side shape
//! - `background` — the background longhands' keyword families
//! - `white_space` — the CSS Text values of white space processing, wrapping
//!   and line breaking (`white-space-collapse`, …, `tab-size`)
//! - `text_align` — the CSS Text values of transform, indent and alignment
//! - `spacing` — `letter-spacing` / `word-spacing` (`Spacing`)
//! - `text` — `TextStyle`, the computed CSS Text group
//! - `filter` — the Filter Effects values and their color math
//! - `blend` — the compositing values and the blend functions
//! - `clip` — the `clip-path` values and their cell-sampled geometry
//! - `transform` — the CSS Transforms values
//! - `effects` — `EffectsStyle`, the computed transform, filter and
//!   compositing group
//! - `multicol` — the Multi-column and Fragmentation values, with
//!   `MulticolStyle` and `FragmentationStyle`

pub(crate) mod alignment;
mod aspect_ratio;
mod background;
mod blend;
mod border;
mod box_model;
mod calc_size;
mod clip;
mod containment;
mod effects;
mod filter;
mod float;
mod font;
mod gap;
mod grid;
mod grid_areas;
mod grid_placement;
mod keywords;
mod line_clamp;
mod line_height;
mod list;
mod multicol;
mod overflow;
mod rect;
mod scroll;
mod scrollbar;
mod sides;
mod sizing;
#[cfg(test)]
mod sizing_tests;
mod spacing;
mod table;
mod text;
mod text_align;
mod text_decoration;
mod transform;
mod ui;
mod vertical_align;
mod white_space;

pub use alignment::{Align, AlignProperty, Alignment, OverflowAlign};
pub use aspect_ratio::AspectRatio;
pub use background::{BackgroundAttachment, BackgroundRepeat, BoxShadow, RepeatStyle, VisualBox};
pub use blend::{BlendMode, Isolation};
pub use border::{
    Border, BorderCollapse, BorderRadius, BorderSpacing, BorderStyle, BorderWeight, BorderWidth,
    CornerStyle, PaintLength,
};
pub use box_model::{Margin, MarginTrim, MarginValue, Padding, PaddingValue};
pub use calc_size::{CalcSize, CalcSizeBasis, InterpolateSize};
pub use clip::{BasicShape, ClipPath, GeometryBox, ShapeRadius};
pub use containment::{
    Contain, ContainerName, ContainerSize, ContainerType, ContentVisibility, WillChange,
};
pub use effects::EffectsStyle;
pub use filter::{FilterFunction, FilterList};
pub use float::{Clear, Float, FloatSide};
pub use font::{
    Font, FontFamily, FontSize, FontSizeKeyword, FontStretch, FontStretchKeyword, FontStyle,
    FontVariant, FontWeight, SystemFont,
};
pub use gap::GapValue;
pub use grid::{
    GridTemplate, LineNameItem, LineNameList, RepeatCount, TrackBreadth, TrackList, TrackListItem,
    TrackRepeat, TrackSize,
};
pub use grid_areas::{GridTemplateAreas, NamedArea};
pub use grid_placement::{GridAutoFlow, GridLine};
pub use keywords::{
    BoxSizing, CaretColor, CaretTextColor, Direction, Display, FlexDirection, FlexWrap, Flow,
    Overlay, PointerEvents, Position, ScrollBehavior, TextDecoration, TextDirection, UserSelect,
    Visibility, WritingMode, ZIndex,
};
pub use line_clamp::{BlockEllipsis, BoxOrient, Continue};
pub use line_height::LineHeight;
pub use list::{ListStyleImage, ListStylePosition, ListStyleType, MarkerSide};
pub use multicol::{
    BoxDecorationBreak, BreakBetween, BreakInside, ColumnCount, ColumnFill, ColumnSpan,
    ColumnWidth, FragmentationStyle, MulticolStyle,
};
pub use overflow::{Overflow, OverflowClipMargin, TextOverflow, TextOverflowSide};
pub use rect::LayoutRect;
pub use scroll::{
    OverscrollBehavior, ScrollPadding, ScrollSnapAlign, ScrollSnapAxis, ScrollSnapStop,
    ScrollSnapStrictness, ScrollSnapType, SnapAlign,
};
pub use scrollbar::{
    NATIVE_SCROLLBAR_THUMB, NATIVE_SCROLLBAR_TRACK, ScrollbarColor, ScrollbarGutter, ScrollbarWidth,
};
pub use sides::{Corners, Sides};
pub use sizing::{
    ContainIntrinsicSize, FlexBasis, IntrinsicSize, Length, MaxSize, MinSize, Size,
    valid_flex_factor,
};
pub use spacing::Spacing;
pub use table::{CaptionSide, EmptyCells, TableLayout, TablePart, TableStyle};
pub use text::TextStyle;
pub use text_align::{
    TextAlign, TextAlignKeyword, TextAlignLast, TextCase, TextIndent, TextJustify, TextTransform,
};
pub use text_decoration::{
    AppliedDecorations, AppliedLine, TextDecorationLine, TextDecorationSkipInk,
    TextDecorationStyle, TextDecorationThickness, TextDecorations, TextUnderlineOffset,
    TextUnderlinePosition,
};
pub use transform::{
    Rotate, Scale, TransformBox, TransformFunction, TransformList, TransformOrigin, Translate,
    TranslateFunction,
};
pub use ui::{
    AccentColor, Appearance, CaretAnimation, CaretShape, Cursor, CursorImage, CursorKeyword,
    FieldSizing, OutlineColor, OutlineStyle, Resize, UiStyle,
};
pub use vertical_align::VerticalAlign;
pub use white_space::{
    Hyphens, LineBreak, OverflowWrap, TabSize, TextWrapMode, TextWrapStyle, WhiteSpace,
    WhiteSpaceCollapse, WordBreak,
};
