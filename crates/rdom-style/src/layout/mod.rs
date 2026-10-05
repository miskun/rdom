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
//! - `sides` — `Sides`, the per-side shape
//! - `background` — the background longhands' keyword families

pub(crate) mod alignment;
mod background;
mod border;
mod box_model;
mod keywords;
mod rect;
mod sides;
mod sizing;
#[cfg(test)]
mod sizing_tests;

pub use alignment::{Align, AlignProperty, Alignment, OverflowAlign};
pub use background::{BackgroundAttachment, BackgroundRepeat, BoxShadow, RepeatStyle, VisualBox};
pub use border::{
    Border, BorderCollapse, BorderRadius, BorderSpacing, BorderStyle, BorderWeight, BorderWidth,
    CornerStyle, PaintLength,
};
pub use box_model::{Margin, MarginTrim, MarginValue, Padding, PaddingValue};
pub use keywords::{
    BoxSizing, CaretColor, CaretTextColor, Direction, Display, FlexDirection, FlexWrap, Flow,
    Overflow, PointerEvents, Position, ScrollBehavior, ScrollbarGutter, TextDecoration,
    TextDirection, UserSelect, Visibility, WhiteSpace, WritingMode, ZIndex,
};
pub use rect::LayoutRect;
pub use sides::{Corners, Sides};
pub use sizing::{
    AspectRatio, ContainIntrinsicSize, FlexBasis, GapValue, IntrinsicSize, Length, MaxSize,
    MinSize, Size, valid_flex_factor,
};
