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
//! - `sizing` — `Size`, `MinSize`, `MaxSize`, `AspectRatio`, `GapValue`, `Length`
//! - `border` — border styles, widths, `border-collapse`
//! - `box_model` — padding, margin
//! - `grid` — the grid track lists (`GridTemplate`, `TrackSize`)
//! - `grid_areas` — named grid areas (`GridTemplateAreas`)
//! - `grid_placement` — grid item placement (`GridLine`, `GridAutoFlow`)
//! - `sides` — `Sides`, the per-side shape
//! - `background` — the background longhands' keyword families
//! - `text` — the CSS Text values (`white-space-collapse`, …) and `TextStyle`

pub(crate) mod alignment;
mod background;
mod border;
mod box_model;
mod float;
mod font;
mod grid;
mod grid_areas;
mod grid_placement;
mod keywords;
mod line_clamp;
mod line_height;
mod overflow;
mod rect;
mod scroll;
mod scrollbar;
mod sides;
mod sizing;
#[cfg(test)]
mod sizing_tests;
mod text;
mod text_decoration;
mod vertical_align;

pub use alignment::{Align, AlignProperty, Alignment, OverflowAlign};
pub use background::{BackgroundAttachment, BackgroundRepeat, BoxShadow, RepeatStyle, VisualBox};
pub use border::{
    Border, BorderCollapse, BorderRadius, BorderSpacing, BorderStyle, BorderWeight, BorderWidth,
    CornerStyle, PaintLength,
};
pub use box_model::{Margin, MarginTrim, MarginValue, Padding, PaddingValue};
pub use float::{Clear, Float, FloatSide};
pub use font::{
    FONT_STRETCH_KEYWORDS, Font, FontFamily, FontSize, FontSizeKeyword, FontStretch, FontStyle,
    FontVariant, FontWeight, SystemFont,
};
pub use grid::{
    GridTemplate, LineNameItem, LineNameList, RepeatCount, TrackBreadth, TrackList, TrackListItem,
    TrackRepeat, TrackSize,
};
pub use grid_areas::{GridTemplateAreas, NamedArea};
pub use grid_placement::{GridAutoFlow, GridLine};
pub use keywords::{
    BoxSizing, CaretColor, CaretTextColor, Direction, Display, FlexDirection, FlexWrap, Flow,
    PointerEvents, Position, ScrollBehavior, TextDecoration, TextDirection, UserSelect, Visibility,
    WritingMode, ZIndex,
};
pub use line_clamp::{BlockEllipsis, BoxOrient, Continue};
pub use line_height::LineHeight;
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
    AspectRatio, ContainIntrinsicSize, FlexBasis, GapValue, IntrinsicSize, Length, MaxSize,
    MinSize, Size, valid_flex_factor,
};
pub use text::{
    Hyphens, LineBreak, OverflowWrap, TabSize, TextAlign, TextAlignLast, TextCase, TextIndent,
    TextJustify, TextStyle, TextTransform, TextWrapMode, TextWrapStyle, WhiteSpace,
    WhiteSpaceCollapse, WordBreak,
};
pub use text_decoration::{
    AppliedDecorations, AppliedLine, TextDecorationLine, TextDecorationSkipInk,
    TextDecorationStyle, TextDecorationThickness, TextDecorations, TextUnderlineOffset,
    TextUnderlinePosition,
};
pub use vertical_align::VerticalAlign;
